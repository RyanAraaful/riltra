// SPDX-License-Identifier: Apache-2.0
//! M3 durable store: SQLite canonical storage with versioned schema,
//! FTS5 lexical search, and snapshot-scoped facts. Symbol rowids are
//! per-snapshot only — never permanent cross-revision identities.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};

/// Current schema version (`PRAGMA user_version`).
pub const SCHEMA_VERSION: u32 = 2;
/// Parser/extraction version. Bump whenever `syntax.rs` extraction changes;
/// freshness treats a mismatch as stale (never silently current).
pub const PARSER_VERSION: u32 = 1;

/// Grammar + runtime versions pinned in `Cargo.lock`. A test pins these
/// against the lockfile so they cannot drift silently.
pub fn grammar_versions() -> String {
    "tree-sitter:0.25.10,python:0.25.0,javascript:0.25.0,typescript:0.23.2,tsx:0.23.2,rust:0.24.2,c:0.24.2,cpp:0.23.4"
        .to_string()
}

/// FNV-1a 64-bit hex. Non-crypto change-detection digest only.
/// shortcut: upgrade to SHA-256 if digests ever back security decisions.
pub fn content_digest(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

/// Per-repo database file under the local Riltra home (never inside repos).
pub fn db_path(home: &Path, repo: &str) -> PathBuf {
    home.join("index").join(format!("{repo}.db"))
}

/// Staging file used during rebuilds; atomically promoted on success.
pub fn staging_path(home: &Path, repo: &str) -> PathBuf {
    let mut path = db_path(home, repo);
    path.set_extension("db.staging");
    path
}

fn open_inner(path: Option<&Path>) -> Result<Connection, String> {
    let conn = match path {
        Some(p) => Connection::open(p).map_err(|e| format!("cannot open database: {e}"))?,
        None => Connection::open_in_memory().map_err(|e| format!("cannot open memory db: {e}"))?,
    };
    // Read probe: rejects garbage files here, not at first query.
    conn.query_row("SELECT count(*) FROM sqlite_master", [], |row| {
        row.get::<_, i64>(0)
    })
    .map_err(|e| format!("not a readable database: {e}"))?;
    // Every open ends at the current schema: empty files are built here, so
    // no caller can ever observe missing tables. Forward versions still fail
    // loudly inside `migrate`.
    migrate(&conn)?;
    // WAL lets readers hold a stable snapshot while a rebuild commits.
    // Best-effort: exotic filesystems fall back to rollback mode (still
    // correct, less concurrent).
    let _ = conn.execute_batch("PRAGMA journal_mode=WAL;");
    Ok(conn)
}

pub fn open_live(path: &Path) -> Result<Connection, String> {
    open_inner(Some(path))
}

/// Read probe with a busy timeout: distinguishes corrupt/unreadable files
/// from transiently locked ones. Used before destructive recovery.
pub fn is_readable_db(path: &Path) -> bool {
    // Never create files from a read probe.
    if !path.is_file() {
        return false;
    }
    let Ok(conn) = Connection::open(path) else {
        return false;
    };
    let _ = conn.execute_batch("PRAGMA busy_timeout = 3000;");
    conn.query_row("SELECT count(*) FROM sqlite_master", [], |row| {
        row.get::<_, i64>(0)
    })
    .is_ok()
}

#[cfg(test)]
pub fn open_memory() -> Result<Connection, String> {
    open_inner(None)
}

const SCHEMA_SQL: &str = "
CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS snapshots (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  repo TEXT NOT NULL, root TEXT NOT NULL, created_unix INTEGER NOT NULL,
  head TEXT, branch TEXT, dirty INTEGER NOT NULL, untracked INTEGER NOT NULL,
  parser_version INTEGER NOT NULL, grammar_versions TEXT NOT NULL,
  config_fingerprint TEXT NOT NULL, pre_fingerprint TEXT NOT NULL,
  post_fingerprint TEXT NOT NULL, status_at_write TEXT NOT NULL,
  nonce TEXT NOT NULL DEFAULT ''
);
CREATE TABLE IF NOT EXISTS files (
  id INTEGER PRIMARY KEY AUTOINCREMENT, snapshot_id INTEGER NOT NULL,
  path TEXT NOT NULL, language TEXT NOT NULL, bytes INTEGER NOT NULL,
  digest TEXT NOT NULL, errors INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_files_snapshot ON files(snapshot_id);
CREATE TABLE IF NOT EXISTS symbols (
  id INTEGER PRIMARY KEY AUTOINCREMENT, snapshot_id INTEGER NOT NULL,
  file_id INTEGER NOT NULL, kind TEXT NOT NULL, name TEXT,
  start_line INTEGER NOT NULL, start_col INTEGER NOT NULL,
  end_line INTEGER NOT NULL, end_col INTEGER NOT NULL,
  byte_start INTEGER NOT NULL, byte_end INTEGER NOT NULL,
  provider TEXT NOT NULL, grammar TEXT NOT NULL, language TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_symbols_snapshot ON symbols(snapshot_id);
CREATE TABLE IF NOT EXISTS imports (
  id INTEGER PRIMARY KEY AUTOINCREMENT, snapshot_id INTEGER NOT NULL,
  file_id INTEGER NOT NULL, text TEXT, start_line INTEGER NOT NULL,
  start_col INTEGER NOT NULL, end_line INTEGER NOT NULL, end_col INTEGER NOT NULL,
  byte_start INTEGER NOT NULL, byte_end INTEGER NOT NULL,
  provider TEXT NOT NULL, grammar TEXT NOT NULL, language TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS calls (
  id INTEGER PRIMARY KEY AUTOINCREMENT, snapshot_id INTEGER NOT NULL,
  file_id INTEGER NOT NULL, name TEXT, start_line INTEGER NOT NULL,
  start_col INTEGER NOT NULL, end_line INTEGER NOT NULL, end_col INTEGER NOT NULL,
  byte_start INTEGER NOT NULL, byte_end INTEGER NOT NULL,
  unresolved INTEGER NOT NULL, provider TEXT NOT NULL, grammar TEXT NOT NULL,
  language TEXT NOT NULL
);
CREATE VIRTUAL TABLE IF NOT EXISTS symbols_fts USING fts5(
  name, content='symbols', content_rowid='id', tokenize='unicode61'
);
CREATE TRIGGER IF NOT EXISTS symbols_ai AFTER INSERT ON symbols BEGIN
  INSERT INTO symbols_fts(rowid, name) VALUES (new.id, new.name);
END;
CREATE TRIGGER IF NOT EXISTS symbols_ad AFTER DELETE ON symbols BEGIN
  INSERT INTO symbols_fts(symbols_fts, rowid, name) VALUES ('delete', old.id, old.name);
END;
";

/// Bring any database to the current schema. Empty files are built at
/// current; v1 files gain the `nonce` column with per-row backfill; newer
/// versions are rejected, never guessed.
pub fn migrate(conn: &Connection) -> Result<(), String> {
    let version: u32 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    if version > SCHEMA_VERSION {
        return Err(format!(
            "database schema v{version} is newer than supported v{SCHEMA_VERSION}"
        ));
    }
    conn.execute_batch(SCHEMA_SQL)
        .map_err(|e| format!("schema migration failed: {e}"))?;
    if version == 1 {
        conn.execute_batch(
            "ALTER TABLE snapshots ADD COLUMN nonce TEXT NOT NULL DEFAULT '';
             UPDATE snapshots SET nonce = 'v1-legacy-' || id WHERE nonce = '';",
        )
        .map_err(|e| format!("schema migration v1->v2 failed: {e}"))?;
    }
    conn.execute("PRAGMA user_version = 2", [])
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// True when the bundled SQLite was compiled with FTS5. Pinned by test on
/// every platform; lexical search depends on it.
pub fn fts_available(conn: &Connection) -> bool {
    conn.query_row(
        "SELECT sqlite_compileoption_used('ENABLE_FTS5')",
        [],
        |row| row.get::<_, i64>(0),
    )
    .map(|v| v == 1)
    .unwrap_or(false)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub id: i64,
    pub repo: String,
    pub root: String,
    pub created_unix: u64,
    pub head: Option<String>,
    pub branch: Option<String>,
    pub dirty: bool,
    pub untracked: u32,
    pub parser_version: u32,
    pub grammar_versions: String,
    pub config_fingerprint: String,
    pub pre_fingerprint: String,
    pub post_fingerprint: String,
    pub status_at_write: String,
    /// Unique revision identity: never reused across rebuilds.
    pub nonce: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileRow {
    pub path: String,
    pub language: String,
    pub bytes: u64,
    pub digest: String,
    pub errors: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SymbolRow {
    pub file_id: i64,
    pub kind: String,
    pub name: Option<String>,
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
    pub byte_start: usize,
    pub byte_end: usize,
    pub provider: String,
    pub grammar: String,
    pub language: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportRow {
    pub file_id: i64,
    pub text: Option<String>,
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
    pub byte_start: usize,
    pub byte_end: usize,
    pub provider: String,
    pub grammar: String,
    pub language: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CallRow {
    pub file_id: i64,
    pub name: Option<String>,
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
    pub byte_start: usize,
    pub byte_end: usize,
    pub unresolved: u8,
    pub provider: String,
    pub grammar: String,
    pub language: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub snapshot_id: i64,
    pub path: String,
    pub kind: String,
    pub name: Option<String>,
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
    pub byte_start: usize,
    pub byte_end: usize,
    pub provider: String,
    pub grammar: String,
    pub language: String,
}

pub fn insert_snapshot(conn: &Connection, snapshot: &Snapshot) -> Result<i64, String> {
    conn.execute(
        "INSERT INTO snapshots (repo, root, created_unix, head, branch, dirty, untracked, parser_version, grammar_versions, config_fingerprint, pre_fingerprint, post_fingerprint, status_at_write, nonce) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
        params![
            snapshot.repo,
            snapshot.root,
            snapshot.created_unix as i64,
            snapshot.head,
            snapshot.branch,
            i64::from(snapshot.dirty),
            snapshot.untracked as i64,
            snapshot.parser_version as i64,
            snapshot.grammar_versions,
            snapshot.config_fingerprint,
            snapshot.pre_fingerprint,
            snapshot.post_fingerprint,
            snapshot.status_at_write,
            snapshot.nonce,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

/// Fresh revision identities: process id, nanosecond clock, and a counter.
/// Uniqueness is what lets tests and envelopes distinguish revisions.
pub fn new_nonce() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!(
        "{}-{}-{}",
        std::process::id(),
        nanos,
        COUNTER.fetch_add(1, Ordering::SeqCst)
    )
}

/// Record the post-build fingerprint and verdict on a snapshot row.
pub fn finish_snapshot(conn: &Connection, id: i64, post: &str, status: &str) -> Result<(), String> {
    let changed = conn
        .execute(
            "UPDATE snapshots SET post_fingerprint = ?1, status_at_write = ?2 WHERE id = ?3",
            params![post, status, id],
        )
        .map_err(|e| e.to_string())?;
    if changed != 1 {
        return Err("snapshot row missing for finish".to_string());
    }
    Ok(())
}

/// Copy one staging snapshot into the live database inside a single
/// transaction: readers see the old snapshot or the new one, never a mix.
/// FTS rows follow automatically through the insert/delete triggers. On any
/// failure the live snapshot is rolled back intact.
pub fn promote_staging(live: &Connection, staging: &Path) -> Result<(), String> {
    let escaped = staging.to_string_lossy().replace('\'', "''");
    live.execute_batch(&format!("ATTACH DATABASE '{escaped}' AS src"))
        .map_err(|e| format!("cannot attach staging database: {e}"))?;
    let copied = live.execute_batch(
        "BEGIN IMMEDIATE;
         DELETE FROM symbols;
         DELETE FROM imports;
         DELETE FROM calls;
         DELETE FROM files;
         DELETE FROM snapshots;
         INSERT INTO snapshots SELECT * FROM src.snapshots;
         INSERT INTO files SELECT * FROM src.files;
         INSERT INTO symbols SELECT * FROM src.symbols;
         INSERT INTO imports SELECT * FROM src.imports;
         INSERT INTO calls SELECT * FROM src.calls;
         COMMIT;",
    );
    if let Err(e) = copied {
        let _ = live.execute_batch("ROLLBACK; DETACH DATABASE src;");
        return Err(format!("promote failed, live snapshot intact: {e}"));
    }
    live.execute_batch("DETACH DATABASE src")
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Attach another database file as `src` for set-based copies. Same
/// single-quote escaping as `promote_staging`.
pub fn attach_src(conn: &Connection, path: &Path) -> Result<(), String> {
    let escaped = path.to_string_lossy().replace('\'', "''");
    conn.execute_batch(&format!("ATTACH DATABASE '{escaped}' AS src"))
        .map_err(|e| format!("cannot attach source database: {e}"))?;
    Ok(())
}

pub fn detach_src(conn: &Connection) -> Result<(), String> {
    conn.execute_batch("DETACH DATABASE src")
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Totals over bulk-copied facts, so callers keep definition/import/call/
/// error counters without extra aggregation queries.
pub struct CopiedTotals {
    pub symbols: u64,
    pub imports: u64,
    pub calls: u64,
    pub errors: u32,
}

/// Copy one files row from the attached `src` database into this one.
/// Returns the new rowid, or `None` when the source row is absent (a
/// concurrent rebuild replaced the revision under us) — callers then ingest
/// the file from current source instead of reusing.
pub fn copy_file_row(
    staging: &Connection,
    new_snapshot_id: i64,
    old_snapshot_id: i64,
    old_file_id: i64,
) -> Result<Option<i64>, String> {
    let present: Option<i64> = staging
        .query_row(
            "SELECT 1 FROM src.files WHERE snapshot_id = ?1 AND id = ?2",
            params![old_snapshot_id, old_file_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if present.is_none() {
        return Ok(None);
    }
    staging
        .execute(
            "INSERT INTO files (snapshot_id, path, language, bytes, digest, errors)
             SELECT ?1, path, language, bytes, digest, errors FROM src.files
             WHERE snapshot_id = ?2 AND id = ?3",
            params![new_snapshot_id, old_snapshot_id, old_file_id],
        )
        .map_err(|e| e.to_string())?;
    if staging.changes() != 1 {
        return Ok(None);
    }
    Ok(Some(staging.last_insert_rowid()))
}

/// Session-local old→new file id map for bulk fact copies. A `TEMP` table:
/// invisible to other connections, gone when this one closes, no schema
/// migration. Per-file child `SELECT`s cannot use the `file_id` column
/// efficiently (no index by design), so the map turns thousands of filtered
/// copies into three joined bulk statements.
pub fn create_file_map(staging: &Connection) -> Result<(), String> {
    staging
        .execute_batch(
            "CREATE TEMP TABLE fmap (old_id INTEGER PRIMARY KEY, new_id INTEGER NOT NULL)",
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn record_file_map(staging: &Connection, old_id: i64, new_id: i64) -> Result<(), String> {
    staging
        .execute(
            "INSERT INTO fmap (old_id, new_id) VALUES (?1, ?2)",
            params![old_id, new_id],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Bulk-copy every mapped file's symbols/imports/calls in three joined
/// statements. Column lists mirror the matching `insert_*` functions.
/// Rejected alternative: per-file filtered child copies — one full snapshot
/// scan per file without a `file_id` index (quadratic at MMSP scale).
pub fn copy_mapped_facts(
    staging: &Connection,
    new_snapshot_id: i64,
    old_snapshot_id: i64,
) -> Result<CopiedTotals, String> {
    let mut counts = [0u64; 3];
    for (i, (table, extra)) in [
        ("symbols", "kind, name, start_line, start_col, end_line, end_col, byte_start, byte_end, provider, grammar, language"),
        ("imports", "text, start_line, start_col, end_line, end_col, byte_start, byte_end, provider, grammar, language"),
        ("calls", "name, start_line, start_col, end_line, end_col, byte_start, byte_end, unresolved, provider, grammar, language"),
    ]
    .iter()
    .enumerate()
    {
        staging
            .execute(
                &format!(
                    "INSERT INTO {table} (snapshot_id, file_id, {extra})
                     SELECT ?1, m.new_id, {extra} FROM src.{table} AS s
                     JOIN fmap AS m ON m.old_id = s.file_id
                     WHERE s.snapshot_id = ?2"
                ),
                params![new_snapshot_id, old_snapshot_id],
            )
            .map_err(|e| e.to_string())?;
        counts[i] = staging.changes();
    }
    let errors: i64 = staging
        .query_row(
            "SELECT COALESCE(SUM(f.errors), 0) FROM src.files AS f
             JOIN fmap AS m ON m.old_id = f.id
             WHERE f.snapshot_id = ?1",
            params![old_snapshot_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(CopiedTotals {
        symbols: counts[0],
        imports: counts[1],
        calls: counts[2],
        errors: errors as u32,
    })
}

pub fn insert_file(conn: &Connection, snapshot_id: i64, file: &FileRow) -> Result<i64, String> {
    conn.execute(
        "INSERT INTO files (snapshot_id, path, language, bytes, digest, errors) VALUES (?1,?2,?3,?4,?5,?6)",
        params![
            snapshot_id,
            file.path,
            file.language,
            file.bytes as i64,
            file.digest,
            file.errors as i64,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

pub fn insert_symbol(
    conn: &Connection,
    snapshot_id: i64,
    symbol: &SymbolRow,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO symbols (snapshot_id, file_id, kind, name, start_line, start_col, end_line, end_col, byte_start, byte_end, provider, grammar, language) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
        params![
            snapshot_id,
            symbol.file_id,
            symbol.kind,
            symbol.name,
            symbol.start_line as i64,
            symbol.start_col as i64,
            symbol.end_line as i64,
            symbol.end_col as i64,
            symbol.byte_start as i64,
            symbol.byte_end as i64,
            symbol.provider,
            symbol.grammar,
            symbol.language,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn insert_import(
    conn: &Connection,
    snapshot_id: i64,
    import: &ImportRow,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO imports (snapshot_id, file_id, text, start_line, start_col, end_line, end_col, byte_start, byte_end, provider, grammar, language) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        params![
            snapshot_id,
            import.file_id,
            import.text,
            import.start_line as i64,
            import.start_col as i64,
            import.end_line as i64,
            import.end_col as i64,
            import.byte_start as i64,
            import.byte_end as i64,
            import.provider,
            import.grammar,
            import.language,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn insert_call(conn: &Connection, snapshot_id: i64, call: &CallRow) -> Result<(), String> {
    conn.execute(
        "INSERT INTO calls (snapshot_id, file_id, name, start_line, start_col, end_line, end_col, byte_start, byte_end, unresolved, provider, grammar, language) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
        params![
            snapshot_id,
            call.file_id,
            call.name,
            call.start_line as i64,
            call.start_col as i64,
            call.end_line as i64,
            call.end_col as i64,
            call.byte_start as i64,
            call.byte_end as i64,
            i64::from(call.unresolved),
            call.provider,
            call.grammar,
            call.language,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn snapshot_from_row(row: &rusqlite::Row) -> rusqlite::Result<Snapshot> {
    Ok(Snapshot {
        id: row.get(0)?,
        repo: row.get(1)?,
        root: row.get(2)?,
        created_unix: row.get::<_, i64>(3)? as u64,
        head: row.get(4)?,
        branch: row.get(5)?,
        dirty: row.get::<_, i64>(6)? != 0,
        untracked: row.get::<_, i64>(7)? as u32,
        parser_version: row.get::<_, i64>(8)? as u32,
        grammar_versions: row.get(9)?,
        config_fingerprint: row.get(10)?,
        pre_fingerprint: row.get(11)?,
        post_fingerprint: row.get(12)?,
        status_at_write: row.get(13)?,
        nonce: row.get(14)?,
    })
}

const SNAPSHOT_COLS: &str = "id, repo, root, created_unix, head, branch, dirty, untracked, parser_version, grammar_versions, config_fingerprint, pre_fingerprint, post_fingerprint, status_at_write, nonce";

pub fn latest_snapshot(conn: &Connection) -> Result<Option<Snapshot>, String> {
    let mut statement = conn
        .prepare(&format!(
            "SELECT {SNAPSHOT_COLS} FROM snapshots ORDER BY id DESC LIMIT 1"
        ))
        .map_err(|e| e.to_string())?;
    let mut rows = statement.query([]).map_err(|e| e.to_string())?;
    match rows.next().map_err(|e| e.to_string())? {
        Some(row) => Ok(Some(snapshot_from_row(row).map_err(|e| e.to_string())?)),
        None => Ok(None),
    }
}

pub fn files_of(conn: &Connection, snapshot_id: i64) -> Result<Vec<FileRow>, String> {
    let mut statement = conn
        .prepare("SELECT path, language, bytes, digest, errors FROM files WHERE snapshot_id = ?1 ORDER BY path")
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([snapshot_id], |row| {
            Ok(FileRow {
                path: row.get(0)?,
                language: row.get(1)?,
                bytes: row.get::<_, i64>(2)? as u64,
                digest: row.get(3)?,
                errors: row.get::<_, i64>(4)? as u32,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

/// Path → (file rowid, content digest) for one snapshot. The id map lets
/// incremental reuse locate old facts without touching `FileRow`.
pub fn file_id_map(
    conn: &Connection,
    snapshot_id: i64,
) -> Result<BTreeMap<String, (i64, String)>, String> {
    let mut statement = conn
        .prepare("SELECT id, path, digest FROM files WHERE snapshot_id = ?1 ORDER BY path")
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([snapshot_id], |row| {
            Ok((
                row.get::<_, String>(1)?,
                (row.get::<_, i64>(0)?, row.get::<_, String>(2)?),
            ))
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<BTreeMap<_, _>, _>>()
        .map_err(|e| e.to_string())
}

fn hit_from_row(snapshot_id: i64, row: &rusqlite::Row) -> rusqlite::Result<Hit> {
    Ok(Hit {
        snapshot_id,
        path: row.get(0)?,
        kind: row.get(1)?,
        name: row.get(2)?,
        start_line: row.get::<_, i64>(3)? as u32,
        start_col: row.get::<_, i64>(4)? as u32,
        end_line: row.get::<_, i64>(5)? as u32,
        end_col: row.get::<_, i64>(6)? as u32,
        byte_start: row.get::<_, i64>(7)? as usize,
        byte_end: row.get::<_, i64>(8)? as usize,
        provider: row.get(9)?,
        grammar: row.get(10)?,
        language: row.get(11)?,
    })
}

fn like_pattern(term: &str) -> String {
    let mut pattern = String::with_capacity(term.len() + 2);
    pattern.push('%');
    for c in term.chars() {
        if matches!(c, '%' | '_' | '\\') {
            pattern.push('\\');
        }
        pattern.push(c);
    }
    pattern.push('%');
    pattern
}

/// Search definitions by name: FTS5 prefix matches first, then LIKE substring
/// matches, both ordered by (path, byte_start). Returns the total count plus
/// the requested page. Calls and imports are never symbol hits.
pub fn search_symbols(
    conn: &Connection,
    snapshot_id: i64,
    term: &str,
    limit: usize,
    offset: usize,
) -> Result<(usize, Vec<Hit>), String> {
    if term.is_empty() {
        return Ok((0, Vec::new()));
    }
    let mut ordered_ids: Vec<i64> = Vec::new();
    // FTS5 prefix search on the whole phrase. Any FTS error (e.g. a term made
    // only of separators) falls back to LIKE alone — never a failure.
    let phrase = format!("\"{}\"*", term.replace('"', "\"\""));
    let mut fts_ids: Vec<i64> = Vec::new();
    if let Ok(mut statement) = conn.prepare(
        "SELECT s.id FROM symbols s JOIN files f ON f.id = s.file_id JOIN symbols_fts ON symbols_fts.rowid = s.id WHERE s.snapshot_id = ?1 AND symbols_fts MATCH ?2 ORDER BY f.path, s.byte_start",
    ) {
        if let Ok(mut rows) = statement.query(params![snapshot_id, phrase]) {
            while let Ok(Some(row)) = rows.next() {
                if let Ok(id) = row.get::<_, i64>(0) {
                    fts_ids.push(id);
                }
            }
        }
    }
    // LIKE pass in (path, byte_start) order.
    let like_pattern_value = like_pattern(term);
    let mut like_ids: Vec<i64> = Vec::new();
    {
        let mut statement = conn
            .prepare(
                "SELECT s.id FROM symbols s JOIN files f ON f.id = s.file_id WHERE s.snapshot_id = ?1 AND s.name LIKE ?2 ESCAPE '\\' ORDER BY f.path, s.byte_start",
            )
            .map_err(|e| e.to_string())?;
        let mut rows = statement
            .query(params![snapshot_id, like_pattern_value])
            .map_err(|e| e.to_string())?;
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            like_ids.push(row.get(0).map_err(|e| e.to_string())?);
        }
    }
    for id in fts_ids.into_iter().chain(like_ids) {
        if !ordered_ids.contains(&id) {
            ordered_ids.push(id);
        }
    }
    let total = ordered_ids.len();
    let page: Vec<i64> = ordered_ids.into_iter().skip(offset).take(limit).collect();
    let mut hits = Vec::with_capacity(page.len());
    for id in page {
        let hit = conn
            .query_row(
                "SELECT f.path, s.kind, s.name, s.start_line, s.start_col, s.end_line, s.end_col, s.byte_start, s.byte_end, s.provider, s.grammar, s.language FROM symbols s JOIN files f ON f.id = s.file_id WHERE s.id = ?1",
                [id],
                |row| hit_from_row(snapshot_id, row),
            )
            .map_err(|e| e.to_string())?;
        hits.push(hit);
    }
    Ok((total, hits))
}

#[cfg(test)]
mod tests {
    use super::{
        content_digest, db_path, files_of, finish_snapshot, fts_available, grammar_versions,
        insert_call, insert_file, insert_import, insert_snapshot, insert_symbol, is_readable_db,
        latest_snapshot, migrate, open_live, open_memory, promote_staging, search_symbols,
        staging_path, FileRow, Snapshot, SymbolRow, SCHEMA_VERSION,
    };
    use crate::testutil::tmpdir;
    use std::path::Path;

    fn sample_snapshot() -> Snapshot {
        Snapshot {
            id: 0,
            repo: "demo".to_string(),
            root: "C:\\demo".to_string(),
            created_unix: 1000,
            head: Some("abc".to_string()),
            branch: Some("main".to_string()),
            dirty: false,
            untracked: 0,
            parser_version: super::PARSER_VERSION,
            grammar_versions: grammar_versions(),
            config_fingerprint: "cfg1".to_string(),
            pre_fingerprint: "pre1".to_string(),
            post_fingerprint: "pre1".to_string(),
            status_at_write: "fresh".to_string(),
            nonce: "test-nonce-1".to_string(),
        }
    }

    fn sample_symbol(file_id: i64, name: &str) -> SymbolRow {
        SymbolRow {
            file_id,
            kind: "function".to_string(),
            name: Some(name.to_string()),
            start_line: 1,
            start_col: 1,
            end_line: 2,
            end_col: 3,
            byte_start: 0,
            byte_end: 20,
            provider: "syntax".to_string(),
            grammar: "python:0.25.0".to_string(),
            language: "python".to_string(),
        }
    }

    #[test]
    fn fts5_is_available_in_bundled_sqlite() {
        let conn = open_memory().unwrap();
        assert!(fts_available(&conn), "bundled SQLite must enable FTS5");
    }

    #[test]
    fn schema_creates_all_tables_at_v1() {
        let conn = open_memory().unwrap();
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        for expected in [
            "meta",
            "snapshots",
            "files",
            "symbols",
            "imports",
            "calls",
            "symbols_fts",
        ] {
            assert!(
                tables.iter().any(|t| t == expected),
                "missing {expected}: {tables:?}"
            );
        }
        let version: u32 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
    }

    #[test]
    fn migrate_builds_v1_from_empty_and_rejects_future() {
        let dir = tmpdir("store-migrate");
        let path = dir.join("m.db");
        std::fs::write(&path, b"").unwrap();
        let conn = open_live(&path).unwrap();
        migrate(&conn).unwrap();
        assert!(fts_available(&conn));
        conn.execute("PRAGMA user_version = 99", []).unwrap();
        assert!(migrate(&conn).is_err(), "future schema must be rejected");
    }

    #[test]
    fn corrupt_file_fails_to_open() {
        let dir = tmpdir("store-corrupt");
        let path = dir.join("c.db");
        std::fs::write(&path, b"not a database at all").unwrap();
        assert!(open_live(&path).is_err());
        assert!(!is_readable_db(&path));
    }

    #[test]
    fn readable_check_passes_for_healthy_db() {
        let dir = tmpdir("store-readable");
        let path = dir.join("ok.db");
        {
            let conn = open_live(&path).unwrap();
            migrate(&conn).unwrap();
        }
        assert!(is_readable_db(&path));
        assert!(!is_readable_db(&dir.join("missing.db")));
    }

    #[test]
    fn wal_mode_is_enabled() {
        let dir = tmpdir("store-wal");
        let path = dir.join("w.db");
        let conn = open_live(&path).unwrap();
        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mode.to_lowercase(), "wal", "concurrent readers need WAL");
    }

    #[test]
    fn migrate_upgrades_v1_snapshots_with_nonce_backfill() {
        let dir = tmpdir("store-v1");
        let path = dir.join("old.db");
        {
            // Raw connection: open_live would auto-migrate, defeating the
            // v1 simulation.
            let conn = rusqlite::Connection::open(&path).unwrap();
            // Simulate a v1 database: v1-shaped table, version marker 1.
            conn.execute_batch(
                "CREATE TABLE snapshots (id INTEGER PRIMARY KEY AUTOINCREMENT,
                 repo TEXT NOT NULL, root TEXT NOT NULL, created_unix INTEGER NOT NULL,
                 head TEXT, branch TEXT, dirty INTEGER NOT NULL, untracked INTEGER NOT NULL,
                 parser_version INTEGER NOT NULL, grammar_versions TEXT NOT NULL,
                 config_fingerprint TEXT NOT NULL, pre_fingerprint TEXT NOT NULL,
                 post_fingerprint TEXT NOT NULL, status_at_write TEXT NOT NULL);
                 INSERT INTO snapshots (repo, root, created_unix, head, branch, dirty, untracked, parser_version, grammar_versions, config_fingerprint, pre_fingerprint, post_fingerprint, status_at_write)
                 VALUES ('r', 'C:\\r', 1, 'abc', 'main', 0, 0, 1, 'g', 'c', 'p', 'p', 'fresh');
                 PRAGMA user_version = 1;",
            )
            .unwrap();
        }
        let conn = open_live(&path).unwrap();
        migrate(&conn).unwrap();
        let version: u32 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, super::SCHEMA_VERSION);
        let snap = latest_snapshot(&conn).unwrap().unwrap();
        assert!(!snap.nonce.is_empty(), "migrated rows need identities");
        assert_eq!(
            snap.head.as_deref(),
            Some("abc"),
            "data must survive migration"
        );
    }

    #[test]
    fn read_transaction_sees_stable_snapshot_during_writer_commit() {
        // Deterministic concurrency proof without threads: reader opens a
        // read transaction, writer commits a new snapshot row, reader must
        // still see the old one. Fails on autocommit reads (sees new row)
        // and deadlocks-or-errors without WAL.
        let dir = tmpdir("store-isolation");
        let path = dir.join("iso.db");
        let writer = open_live(&path).unwrap();
        let mut first = sample_snapshot();
        first.created_unix = 1;
        insert_snapshot(&writer, &first).unwrap();
        let mut reader = open_live(&path).unwrap();
        let tx = reader.transaction().unwrap();
        let before = latest_snapshot(&tx).unwrap().unwrap();
        let mut second = sample_snapshot();
        second.created_unix = 2;
        insert_snapshot(&writer, &second).unwrap();
        let during = latest_snapshot(&tx).unwrap().unwrap();
        assert_eq!(before.created_unix, 1);
        assert_eq!(
            during.created_unix, 1,
            "read txn must not see the concurrent commit"
        );
        tx.commit().unwrap();
        let after = latest_snapshot(&reader).unwrap().unwrap();
        assert_eq!(after.created_unix, 2);
    }

    #[test]
    fn content_digest_is_deterministic_fnv() {
        assert_eq!(content_digest(b""), "cbf29ce484222325");
        assert_eq!(content_digest(b"a"), content_digest(b"a"));
        assert_ne!(content_digest(b"a"), content_digest(b"b"));
    }

    #[test]
    fn insert_and_search_roundtrip_with_pagination() {
        let conn = open_memory().unwrap();
        let snap = insert_snapshot(&conn, &sample_snapshot()).unwrap();
        let f1 = insert_file(
            &conn,
            snap,
            &FileRow {
                path: "b.py".to_string(),
                language: "python".to_string(),
                bytes: 10,
                digest: "d1".to_string(),
                errors: 0,
            },
        )
        .unwrap();
        let f2 = insert_file(
            &conn,
            snap,
            &FileRow {
                path: "a.py".to_string(),
                language: "python".to_string(),
                bytes: 10,
                digest: "d2".to_string(),
                errors: 0,
            },
        )
        .unwrap();
        insert_symbol(&conn, snap, &sample_symbol(f1, "PatchMatch::Run")).unwrap();
        insert_symbol(&conn, snap, &sample_symbol(f2, "Batch_atch")).unwrap();
        insert_symbol(&conn, snap, &sample_symbol(f2, "üñî_atch")).unwrap();
        insert_symbol(&conn, snap, &sample_symbol(f2, "unrelated")).unwrap();

        let (total, hits) = search_symbols(&conn, snap, "atch", 10, 0).unwrap();
        assert_eq!(total, 3, "substring must match all three names: {hits:?}");
        assert!(
            hits.iter().any(|h| h.name.as_deref() == Some("üñî_atch")),
            "unicode names must survive storage: {hits:?}"
        );
    }

    #[test]
    fn search_orders_by_path_and_paginates() {
        let conn = open_memory().unwrap();
        let snap = insert_snapshot(&conn, &sample_snapshot()).unwrap();
        for (path, name) in [
            ("b.py", "zeta_one"),
            ("a.py", "alpha_one"),
            ("c.py", "mid_one"),
        ] {
            let fid = insert_file(
                &conn,
                snap,
                &FileRow {
                    path: path.to_string(),
                    language: "python".to_string(),
                    bytes: 10,
                    digest: "d".to_string(),
                    errors: 0,
                },
            )
            .unwrap();
            insert_symbol(&conn, snap, &sample_symbol(fid, name)).unwrap();
        }
        let (total, page1) = search_symbols(&conn, snap, "one", 2, 0).unwrap();
        assert_eq!(total, 3);
        assert_eq!(page1.len(), 2);
        assert_eq!(page1[0].path, "a.py");
        assert_eq!(page1[1].path, "b.py");
        let (_, page2) = search_symbols(&conn, snap, "one", 2, 2).unwrap();
        assert_eq!(page2.len(), 1);
        assert_eq!(page2[0].path, "c.py");
    }

    #[test]
    fn latest_snapshot_returns_newest_or_none() {
        let conn = open_memory().unwrap();
        assert!(latest_snapshot(&conn).unwrap().is_none());
        let mut first = sample_snapshot();
        first.created_unix = 1;
        insert_snapshot(&conn, &first).unwrap();
        let mut second = sample_snapshot();
        second.created_unix = 2;
        let id2 = insert_snapshot(&conn, &second).unwrap();
        assert_eq!(latest_snapshot(&conn).unwrap().unwrap().id, id2);
    }

    #[test]
    fn files_list_is_ordered_by_path() {
        let conn = open_memory().unwrap();
        let snap = insert_snapshot(&conn, &sample_snapshot()).unwrap();
        for path in ["z.py", "a.py", "m.py"] {
            insert_file(
                &conn,
                snap,
                &FileRow {
                    path: path.to_string(),
                    language: "python".to_string(),
                    bytes: 1,
                    digest: "d".to_string(),
                    errors: 0,
                },
            )
            .unwrap();
        }
        let files = files_of(&conn, snap).unwrap();
        let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, vec!["a.py", "m.py", "z.py"]);
    }

    #[test]
    fn imports_and_calls_roundtrip() {
        use super::{CallRow, ImportRow};
        let conn = open_memory().unwrap();
        let snap = insert_snapshot(&conn, &sample_snapshot()).unwrap();
        let fid = insert_file(
            &conn,
            snap,
            &FileRow {
                path: "a.py".to_string(),
                language: "python".to_string(),
                bytes: 30,
                digest: "d".to_string(),
                errors: 0,
            },
        )
        .unwrap();
        insert_import(
            &conn,
            snap,
            &ImportRow {
                file_id: fid,
                text: Some("import os".to_string()),
                start_line: 1,
                start_col: 1,
                end_line: 1,
                end_col: 10,
                byte_start: 0,
                byte_end: 9,
                provider: "syntax".to_string(),
                grammar: "python:0.25.0".to_string(),
                language: "python".to_string(),
            },
        )
        .unwrap();
        insert_call(
            &conn,
            snap,
            &CallRow {
                file_id: fid,
                name: Some("w.process".to_string()),
                start_line: 2,
                start_col: 5,
                end_line: 2,
                end_col: 20,
                byte_start: 15,
                byte_end: 30,
                unresolved: 1,
                provider: "syntax".to_string(),
                grammar: "python:0.25.0".to_string(),
                language: "python".to_string(),
            },
        )
        .unwrap();
        let (total, hits) = search_symbols(&conn, snap, "w.process", 10, 0).unwrap();
        assert_eq!(total, 0, "calls are not symbol definitions: {hits:?}");
    }

    #[test]
    fn file_map_bulk_copy_roundtrip() {
        use super::{
            attach_src, copy_file_row, copy_mapped_facts, create_file_map, detach_src, file_id_map,
            record_file_map, CallRow, ImportRow,
        };
        let dir = tmpdir("store-bulk");
        let live_path = dir.join("live.db");
        let old_snap;
        {
            let live = open_live(&live_path).unwrap();
            old_snap = insert_snapshot(&live, &sample_snapshot()).unwrap();
            for (path, errors) in [("a.py", 2), ("b.py", 0)] {
                let fid = insert_file(
                    &live,
                    old_snap,
                    &FileRow {
                        path: path.to_string(),
                        language: "python".to_string(),
                        bytes: 30,
                        digest: "d1".to_string(),
                        errors,
                    },
                )
                .unwrap();
                insert_symbol(&live, old_snap, &sample_symbol(fid, &format!("f_{path}"))).unwrap();
            }
            insert_import(
                &live,
                old_snap,
                &ImportRow {
                    file_id: 1,
                    text: Some("import os".to_string()),
                    start_line: 1,
                    start_col: 1,
                    end_line: 1,
                    end_col: 10,
                    byte_start: 0,
                    byte_end: 9,
                    provider: "syntax".to_string(),
                    grammar: "python:0.25.0".to_string(),
                    language: "python".to_string(),
                },
            )
            .unwrap();
            insert_call(
                &live,
                old_snap,
                &CallRow {
                    file_id: 2,
                    name: Some("g()".to_string()),
                    start_line: 2,
                    start_col: 5,
                    end_line: 2,
                    end_col: 20,
                    byte_start: 15,
                    byte_end: 30,
                    unresolved: 1,
                    provider: "syntax".to_string(),
                    grammar: "python:0.25.0".to_string(),
                    language: "python".to_string(),
                },
            )
            .unwrap();
            let map = file_id_map(&live, old_snap).unwrap();
            assert_eq!(map.len(), 2);
        }
        let staging = open_live(&dir.join("staging.db")).unwrap();
        attach_src(&staging, &live_path).unwrap();
        create_file_map(&staging).unwrap();
        let new_snap = insert_snapshot(&staging, &sample_snapshot()).unwrap();
        let live = open_live(&live_path).unwrap();
        let map = file_id_map(&live, old_snap).unwrap();
        for (old_fid, _) in map.values() {
            let new_fid = copy_file_row(&staging, new_snap, old_snap, *old_fid)
                .unwrap()
                .expect("source row present");
            record_file_map(&staging, *old_fid, new_fid).unwrap();
        }
        // Vanished source rows copy nothing instead of corrupting.
        assert!(copy_file_row(&staging, new_snap, old_snap, 9999)
            .unwrap()
            .is_none());
        let totals = copy_mapped_facts(&staging, new_snap, old_snap).unwrap();
        assert_eq!(totals.symbols, 2);
        assert_eq!(totals.imports, 1);
        assert_eq!(totals.calls, 1);
        assert_eq!(totals.errors, 2);
        detach_src(&staging).unwrap();
        let files = files_of(&staging, new_snap).unwrap();
        assert_eq!(files.len(), 2);
        let (total, hits) = search_symbols(&staging, new_snap, "f_a", 10, 0).unwrap();
        assert_eq!(total, 1, "copied symbols must be searchable: {hits:?}");
    }

    #[test]
    fn grammar_versions_match_cargo_lock() {
        // Workspace lockfile lives two levels above this crate.
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let lock = std::fs::read_to_string(manifest.join("../../Cargo.lock")).unwrap();
        for (name, version) in pinned_grammars() {
            assert!(
                lock.contains(&format!("name = \"{name}\"\nversion = \"{version}\"")),
                "Cargo.lock must pin {name} {version}"
            );
        }
        let versions = grammar_versions();
        for (name, version) in pinned_grammars() {
            assert!(
                versions.contains(version),
                "grammar_versions must list {name} {version}"
            );
        }
    }

    fn pinned_grammars() -> Vec<(&'static str, &'static str)> {
        vec![
            ("tree-sitter", "0.25.10"),
            ("tree-sitter-python", "0.25.0"),
            ("tree-sitter-javascript", "0.25.0"),
            ("tree-sitter-typescript", "0.23.2"),
            ("tree-sitter-rust", "0.24.2"),
            ("tree-sitter-c", "0.24.2"),
            ("tree-sitter-cpp", "0.23.4"),
        ]
    }

    #[test]
    fn finish_snapshot_records_post_and_status() {
        let conn = open_memory().unwrap();
        let id = insert_snapshot(&conn, &sample_snapshot()).unwrap();
        finish_snapshot(&conn, id, "post", "fresh").unwrap();
        let snap = latest_snapshot(&conn).unwrap().unwrap();
        assert_eq!(snap.post_fingerprint, "post");
        assert_eq!(snap.status_at_write, "fresh");
        assert!(finish_snapshot(&conn, 9999, "x", "y").is_err());
    }

    #[test]
    fn promote_staging_copies_snapshot_atomically() {
        let dir = tmpdir("store-promote");
        let staging = dir.join("s.db");
        let live_path = dir.join("live.db");
        let staging_conn = open_live(&staging).unwrap();
        let id = insert_snapshot(&staging_conn, &sample_snapshot()).unwrap();
        finish_snapshot(&staging_conn, id, "post", "fresh").unwrap();
        let live = open_live(&live_path).unwrap();
        promote_staging(&live, &staging).unwrap();
        let snap = latest_snapshot(&live).unwrap().unwrap();
        assert_eq!(snap.post_fingerprint, "post");
        assert!(
            staging.exists(),
            "staging file itself is left for the caller to clean"
        );
    }

    #[test]
    fn db_and_staging_paths_live_under_home_index() {
        let home = tmpdir("store-paths");
        let db = db_path(&home, "myrepo");
        let staging = staging_path(&home, "myrepo");
        assert_eq!(db.file_name().unwrap().to_str().unwrap(), "myrepo.db");
        assert!(staging.to_string_lossy().ends_with("myrepo.db.staging"));
        assert_eq!(db.parent().unwrap(), staging.parent().unwrap());
    }
}
