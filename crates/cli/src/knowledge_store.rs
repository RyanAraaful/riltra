// SPDX-License-Identifier: Apache-2.0
//! M7 knowledge store: separate authoritative SQLite database per registered
//! repository (`$RILTRA_HOME/knowledge/<name>.db`, schema v1). Task 1 covers
//! open, schema creation, metadata binding and location-security checks only;
//! record writes arrive in later tasks. Never touches the M6 code index.

// Task 1 leaves this module unwired (CLI dispatch is Task 7); Tasks 2-7
// consume these items. Revisit this allow when the CLI is wired.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior};

use crate::knowledge::{NewRecord, RecordVersion, UpdateRecord};
use crate::paths;
use crate::registry::RepoEntry;

/// Independent knowledge schema version (`PRAGMA user_version`).
pub const KNOWLEDGE_SCHEMA_VERSION: u32 = 1;

/// Per-repository knowledge database file (never inside repositories).
pub fn knowledge_db_path(home: &Path, repo: &str) -> PathBuf {
    home.join("knowledge").join(format!("{repo}.db"))
}

/// Open the authoritative knowledge store. `create=false` never creates a
/// database or companion files and uses open flags without CREATE.
pub fn open_knowledge(repo: &RepoEntry, create: bool) -> Result<Connection, String> {
    open_knowledge_at(&paths::config_dir(), repo, create)
}

/// Testable form of [`open_knowledge`] with an explicit home directory.
pub fn open_knowledge_at(
    home: &Path,
    repo: &RepoEntry,
    create: bool,
) -> Result<Connection, String> {
    open_inner(home, repo, create, false)
}

/// Test-only init-failure injector: fails schema initialization after the DDL
/// batch to prove the whole init rolls back atomically.
#[cfg(test)]
pub(crate) fn open_knowledge_at_inject_init_failure(
    home: &Path,
    repo: &RepoEntry,
) -> Result<Connection, String> {
    open_inner(home, repo, true, true)
}

fn open_inner(
    home: &Path,
    repo: &RepoEntry,
    create: bool,
    inject_init_failure: bool,
) -> Result<Connection, String> {
    if !paths::validate_name(&repo.name) {
        return Err(format!("invalid repository name: '{}'", repo.name));
    }
    let chome = canonical_home(home, create)?;
    let croot = paths::canonical_root(Path::new(&repo.root))
        .map_err(|e| format!("cannot resolve repository root '{}': {e}", repo.root))?;
    let cdir = ensure_knowledge_dir(&chome, create)?;
    let db = cdir.join(format!("{}.db", repo.name));
    let companions = companion_paths(&db);
    // Pre-open scan: reject hostile links before SQLite touches the target.
    for p in std::iter::once(&db).chain(companions.iter()) {
        reject_link(p)?;
    }
    let existed = db.is_file();
    if !existed && !create {
        return Err(format!("no knowledge store for repository '{}'", repo.name));
    }
    if !existed && create {
        // Refuse binding a new store to a moved or retargeted root before
        // SQLite creates anything, so a refusal leaves no file behind.
        check_fresh_root_binding(repo, &croot)?;
    }
    if !existed && create && db.exists() {
        // Present but not a file (directory, fifo, …). Links were rejected
        // above, so this is never a dangling link.
        return Err(format!(
            "knowledge store path is not a file for repository '{}'",
            repo.name
        ));
    }
    let mut conn = if create {
        Connection::open(&db)
    } else {
        Connection::open_with_flags(&db, OpenFlags::SQLITE_OPEN_READ_ONLY)
    }
    .map_err(|e| format!("cannot open knowledge store for '{}': {e}", repo.name))?;
    // Post-open re-check: SQLite follows links, so canonical comparison after
    // open is the normative guard for DB and companions.
    let cdb = db
        .canonicalize()
        .map_err(|e| format!("knowledge store path escapes for '{}': {e}", repo.name))?;
    if cdb != cdir.join(format!("{}.db", repo.name)) {
        drop(conn);
        return Err(format!(
            "knowledge store path escapes via link for repository '{}'",
            repo.name
        ));
    }
    for p in companions.iter() {
        reject_link(p)?;
    }
    conn.busy_timeout(Duration::from_millis(3000))
        .map_err(|e| format!("cannot set knowledge store timeout: {e}"))?;
    conn.query_row("SELECT count(*) FROM sqlite_master", [], |row| {
        row.get::<_, i64>(0)
    })
    .map_err(|e| format!("not a readable knowledge database for '{}': {e}", repo.name))?;
    let version: u32 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|e| format!("cannot read knowledge schema version: {e}"))?;
    if version > KNOWLEDGE_SCHEMA_VERSION {
        return Err(format!(
            "forward knowledge schema version {version} for '{}'",
            repo.name
        ));
    }
    if !existed {
        init_schema(&mut conn, repo, &croot, inject_init_failure)?;
    } else {
        if version != KNOWLEDGE_SCHEMA_VERSION {
            return Err(format!(
                "corrupt knowledge store for '{}': unexpected schema version {version}",
                repo.name
            ));
        }
        check_schema_complete(&conn, repo)?;
        check_meta(&conn, repo, &croot)?;
    }
    // Best-effort like the code index: exotic filesystems fall back to
    // rollback mode (still correct, less concurrent). Read-only opens ignore
    // the result as well.
    //
    // NOTE (Task 2): WAL mode activates here, after the scans above. Later
    // SQLite accesses can create or touch `-wal`/`-shm` files, so every write
    // transaction must re-validate companions before committing; the checks in
    // this function cover the open instant only. No TOCTOU protection beyond
    // SQLite transactions and bounded writer waits is claimed.
    let _ = conn.execute_batch("PRAGMA journal_mode=WAL;");
    Ok(conn)
}

fn companion_paths(db: &Path) -> [PathBuf; 3] {
    fn with_suffix(db: &Path, suffix: &str) -> PathBuf {
        let mut os = db.as_os_str().to_os_string();
        os.push(suffix);
        PathBuf::from(os)
    }
    [
        with_suffix(db, "-wal"),
        with_suffix(db, "-shm"),
        with_suffix(db, "-journal"),
    ]
}

/// Any symlink at a store-controlled path is an escape, including dangling
/// links (`symlink_metadata` succeeds on those). A missing path is absent;
/// any other I/O error fails closed instead of being treated as absent.
fn reject_link(p: &Path) -> Result<(), String> {
    match std::fs::symlink_metadata(p) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!(
            "cannot inspect knowledge store path '{}': {e}",
            p.display()
        )),
        Ok(md) if md.file_type().is_symlink() => Err(format!(
            "knowledge store path escapes via link: '{}'",
            p.display()
        )),
        Ok(_) => Ok(()),
    }
}

fn canonical_home(home: &Path, create: bool) -> Result<PathBuf, String> {
    match home.canonicalize() {
        Ok(c) => Ok(c),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && create => {
            std::fs::create_dir_all(home)
                .map_err(|e| format!("cannot create home '{}': {e}", home.display()))?;
            home.canonicalize()
                .map_err(|e| format!("cannot resolve home '{}': {e}", home.display()))
        }
        Err(e) => Err(format!("cannot resolve home '{}': {e}", home.display())),
    }
}

/// Validate the knowledge directory before SQLite open: a symlink is an
/// escape/retarget, a non-directory is an error, a missing directory is
/// created only when `create` is set.
fn ensure_knowledge_dir(chome: &Path, create: bool) -> Result<PathBuf, String> {
    let kdir = chome.join("knowledge");
    match std::fs::symlink_metadata(&kdir) {
        Ok(md) if md.file_type().is_symlink() => Err(format!(
            "knowledge directory escapes via link: '{}'",
            kdir.display()
        )),
        Ok(md) if md.is_dir() => {
            let c = kdir.canonicalize().map_err(|e| {
                format!(
                    "cannot resolve knowledge directory '{}': {e}",
                    kdir.display()
                )
            })?;
            if c != chome.join("knowledge") {
                return Err(format!(
                    "knowledge directory retargeted: '{}'",
                    kdir.display()
                ));
            }
            Ok(c)
        }
        Ok(_) => Err(format!(
            "knowledge path is not a directory: '{}'",
            kdir.display()
        )),
        Err(_) => {
            // Missing and unusable paths (e.g. ENOTDIR under a file on Unix)
            // share one outcome: without `create` the store is absent; with
            // `create` the attempt fails here with a uniform message on every
            // platform instead of leaking OS-specific error kinds.
            if !create {
                return Err("no knowledge store: knowledge directory is absent".to_string());
            }
            std::fs::create_dir_all(&kdir).map_err(|e| {
                format!(
                    "cannot create knowledge directory '{}': {e}",
                    kdir.display()
                )
            })?;
            let c = kdir.canonicalize().map_err(|e| {
                format!(
                    "cannot resolve knowledge directory '{}': {e}",
                    kdir.display()
                )
            })?;
            if c != chome.join("knowledge") {
                return Err(format!(
                    "knowledge directory retargeted: '{}'",
                    kdir.display()
                ));
            }
            Ok(c)
        }
    }
}

const SCHEMA_SQL: &str = "
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE records (
  id TEXT NOT NULL, revision INTEGER NOT NULL,
  kind TEXT NOT NULL, title TEXT NOT NULL, body TEXT NOT NULL,
  status TEXT NOT NULL, author TEXT NOT NULL, created_unix INTEGER NOT NULL,
  PRIMARY KEY (id, revision)
);
CREATE TABLE heads (id TEXT PRIMARY KEY, current_revision INTEGER NOT NULL);
CREATE TABLE evidence (
  id TEXT NOT NULL, revision INTEGER NOT NULL, seq INTEGER NOT NULL,
  path TEXT NOT NULL, byte_start INTEGER, byte_end INTEGER,
  digest TEXT NOT NULL, git_head TEXT, git_branch TEXT,
  PRIMARY KEY (id, revision, seq)
);
CREATE TABLE relations (
  relation_id TEXT PRIMARY KEY,
  src_id TEXT NOT NULL, src_revision INTEGER NOT NULL,
  dst_id TEXT NOT NULL, dst_revision INTEGER NOT NULL,
  kind TEXT NOT NULL, author TEXT NOT NULL,
  rationale TEXT NOT NULL, created_unix INTEGER NOT NULL
);
CREATE TABLE retractions (
  relation_id TEXT PRIMARY KEY,
  author TEXT NOT NULL, reason TEXT NOT NULL, created_unix INTEGER NOT NULL
);
CREATE TABLE measurements (
  id TEXT NOT NULL, revision INTEGER NOT NULL,
  method TEXT NOT NULL, environment TEXT NOT NULL, sample_count INTEGER NOT NULL,
  PRIMARY KEY (id, revision)
);
CREATE TABLE metric_values (
  id TEXT NOT NULL, revision INTEGER NOT NULL, name TEXT NOT NULL,
  value REAL NOT NULL, unit TEXT NOT NULL,
  PRIMARY KEY (id, revision, name)
);
CREATE VIRTUAL TABLE knowledge_fts USING fts5(id UNINDEXED, title, body);
-- FTS holds exactly the current head revision per record id. Only head moves
-- (inserts and updates of `heads.current_revision`) maintain the index, in
-- the same transaction as the move, so rolling back a head move rolls back
-- the index with it. Inserting a revision without moving the head never
-- touches the index, and immutable historical rows are never removed by it.
CREATE TRIGGER heads_fts_sync_insert AFTER INSERT ON heads BEGIN
  DELETE FROM knowledge_fts WHERE id = new.id;
  INSERT INTO knowledge_fts(id, title, body)
  SELECT id, title, body FROM records
  WHERE id = new.id AND revision = new.current_revision;
END;
CREATE TRIGGER heads_fts_sync_update AFTER UPDATE OF current_revision ON heads BEGIN
  DELETE FROM knowledge_fts WHERE id = new.id;
  INSERT INTO knowledge_fts(id, title, body)
  SELECT id, title, body FROM records
  WHERE id = new.id AND revision = new.current_revision;
END;
";

/// Build schema v1 on a file that did not exist before open, inside one
/// SQLite transaction: DDL, metadata binding, generation init and the version
/// stamp commit together or roll back together. Existing files are never
/// initialized here: garbage or version-0 files fail closed.
fn init_schema(
    conn: &mut Connection,
    repo: &RepoEntry,
    croot: &Path,
    inject_init_failure: bool,
) -> Result<(), String> {
    let tx = conn
        .transaction()
        .map_err(|e| format!("cannot begin knowledge schema transaction: {e}"))?;
    tx.execute_batch(SCHEMA_SQL)
        .map_err(|e| format!("cannot create knowledge schema: {e}"))?;
    if inject_init_failure {
        // Test hook: drop the transaction without committing to prove no
        // partial schema or metadata survives initialization failure.
        return Err("injected init failure".to_string());
    }
    tx.execute(
        "INSERT INTO meta (key, value) VALUES ('repo_name', ?1)",
        [repo.name.as_str()],
    )
    .map_err(|e| format!("cannot bind knowledge metadata: {e}"))?;
    tx.execute(
        "INSERT INTO meta (key, value) VALUES ('canonical_root', ?1)",
        [croot.to_string_lossy().as_ref()],
    )
    .map_err(|e| format!("cannot bind knowledge metadata: {e}"))?;
    tx.execute(
        "INSERT INTO meta (key, value) VALUES ('knowledge_schema_version', '1')",
        [],
    )
    .map_err(|e| format!("cannot bind knowledge metadata: {e}"))?;
    tx.execute(
        "INSERT INTO meta (key, value) VALUES ('knowledge_generation', '0')",
        [],
    )
    .map_err(|e| format!("cannot bind knowledge metadata: {e}"))?;
    tx.execute_batch("PRAGMA user_version = 1")
        .map_err(|e| format!("cannot stamp knowledge schema version: {e}"))?;
    tx.commit()
        .map_err(|e| format!("cannot commit knowledge schema: {e}"))?;
    Ok(())
}

/// Objects that must exist for a schema-v1 store to be accepted. A matching
/// `user_version` and readable `meta` alone are not enough: an existing
/// database missing any required object fails explicitly here.
const REQUIRED_TABLES: [&str; 9] = [
    "meta",
    "records",
    "heads",
    "evidence",
    "relations",
    "retractions",
    "measurements",
    "metric_values",
    "knowledge_fts",
];
const REQUIRED_TRIGGERS: [&str; 2] = ["heads_fts_sync_insert", "heads_fts_sync_update"];

fn check_schema_complete(conn: &Connection, repo: &RepoEntry) -> Result<(), String> {
    for table in REQUIRED_TABLES {
        let n: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [table],
                |row| row.get(0),
            )
            .map_err(|_| "corrupt knowledge store: schema unreadable".to_string())?;
        if n != 1 {
            return Err(format!(
                "incomplete knowledge schema for '{}': missing table '{table}'",
                repo.name
            ));
        }
    }
    for trigger in REQUIRED_TRIGGERS {
        let n: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'trigger' AND name = ?1",
                [trigger],
                |row| row.get(0),
            )
            .map_err(|_| "corrupt knowledge store: schema unreadable".to_string())?;
        if n != 1 {
            return Err(format!(
                "incomplete knowledge schema for '{}': missing trigger '{trigger}'",
                repo.name
            ));
        }
    }
    Ok(())
}

fn meta_value(conn: &Connection, key: &str) -> Result<String, String> {
    conn.query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
        row.get(0)
    })
    .map_err(|_| "corrupt knowledge store: metadata unreadable".to_string())
}

/// Fresh-create guard: registration records the canonical root string, and
/// every read command re-resolves the entry and compares (see `scan`/`status`
/// handling). A live canonical form that differs means the tree moved or the
/// entry was retargeted before the first knowledge store was created, so the
/// new store is refused instead of silently binding another tree.
fn check_fresh_root_binding(repo: &RepoEntry, croot: &Path) -> Result<(), String> {
    if croot.to_string_lossy().as_ref() != repo.root {
        return Err(format!(
            "knowledge store creation refused for '{}': repository root retargeted",
            repo.name
        ));
    }
    Ok(())
}

/// The store belongs to exactly one registered repository: name, canonical
/// root and schema marker must all match on every open. The stored root must
/// also still resolve to itself, so a moved or deleted root fails instead of
/// being accepted on a stale string comparison.
fn check_meta(conn: &Connection, repo: &RepoEntry, croot: &Path) -> Result<(), String> {
    let name = meta_value(conn, "repo_name")?;
    let root = meta_value(conn, "canonical_root")?;
    let marker = meta_value(conn, "knowledge_schema_version")?;
    let root_alive = paths::canonical_root(Path::new(&root))
        .map(|live| live.to_string_lossy().into_owned() == root)
        .unwrap_or(false);
    if name != repo.name
        || root != croot.to_string_lossy().as_ref()
        || marker != KNOWLEDGE_SCHEMA_VERSION.to_string()
        || !root_alive
    {
        return Err(format!(
            "knowledge metadata mismatch for repository '{}'",
            repo.name
        ));
    }
    Ok(())
}

/// One page of immutable revision history with its generation guard.
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryPage {
    pub revisions: Vec<RecordVersion>,
    pub total: usize,
    pub truncated: bool,
    pub generation: u64,
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Current monotonic knowledge generation (bumped by every mutation).
pub fn read_generation(conn: &Connection) -> Result<u64, String> {
    let raw: String = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'knowledge_generation'",
            [],
            |row| row.get(0),
        )
        .map_err(|_| "corrupt knowledge store: generation unreadable".to_string())?;
    raw.parse::<u64>()
        .map_err(|_| "corrupt knowledge store: generation unreadable".to_string())
}

fn bump_generation(tx: &Transaction) -> Result<u64, String> {
    let current = read_generation(tx)?;
    let next = current
        .checked_add(1)
        .ok_or_else(|| "knowledge generation overflow".to_string())?;
    tx.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'knowledge_generation'",
        [next.to_string()],
    )
    .map_err(|e| format!("cannot bump knowledge generation: {e}"))?;
    Ok(next)
}

/// Re-check the database and companion paths immediately before a write
/// transaction starts: a link planted between open and write refuses the
/// mutation with nothing committed. This covers the check instant only;
/// races between this scan and SQLite's own access are not claimed away
/// (TOCTOU), only narrowed to that window on top of SQLite's locking.
fn revalidate_open_paths(conn: &Connection) -> Result<(), String> {
    if let Some(path) = conn.path() {
        let db = Path::new(path);
        reject_link(db)?;
        for companion in companion_paths(db) {
            reject_link(&companion)?;
        }
    }
    Ok(())
}

fn record_from_row(row: &rusqlite::Row) -> rusqlite::Result<RecordVersion> {
    let revision_i64: i64 = row.get(1)?;
    let revision = u64::try_from(revision_i64).map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            1,
            rusqlite::types::Type::Integer,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("negative revision {revision_i64}"),
            )),
        )
    })?;
    Ok(RecordVersion {
        id: row.get(0)?,
        revision,
        kind: row.get(2)?,
        title: row.get(3)?,
        body: row.get(4)?,
        status: row.get(5)?,
        author: row.get(6)?,
        created_unix: row.get(7)?,
        measurement: None,
    })
}

const RECORD_COLS: &str = "id, revision, kind, title, body, status, author, created_unix";

/// Create revision 1 of a new record. Fails on duplicates, invalid input, or
/// path revalidation; failures commit nothing and bump no generation.
/// Benchmark records require measurement: use [`create_validated`].
pub fn create_record(conn: &mut Connection, input: &NewRecord) -> Result<(u64, u64), String> {
    create_inner(conn, input, None, false)
}

/// Create revision 1 from strictly validated JSON input, persisting benchmark
/// measurement atomically with the revision when present.
pub fn create_validated(
    conn: &mut Connection,
    input: &crate::knowledge::ValidatedNew,
) -> Result<(u64, u64), String> {
    create_inner(conn, &input.record, input.measurement.as_ref(), false)
}

/// Test-only fault injector: fails after the records insert to prove the
/// whole creation rolls back.
#[cfg(test)]
pub(crate) fn create_validated_with_fault(
    conn: &mut Connection,
    input: &crate::knowledge::ValidatedNew,
) -> Result<(u64, u64), String> {
    create_inner(conn, &input.record, input.measurement.as_ref(), true)
}

fn write_measurement(
    tx: &Transaction,
    id: &str,
    revision: i64,
    measurement: &crate::knowledge::BenchmarkData,
) -> Result<(), String> {
    tx.execute(
        "INSERT INTO measurements (id, revision, method, environment, sample_count)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            id,
            revision,
            measurement.method,
            measurement.environment,
            i64::try_from(measurement.sample_count)
                .map_err(|_| "sample count out of range".to_string())?,
        ],
    )
    .map_err(|e| format!("cannot store measurement: {e}"))?;
    for metric in &measurement.metrics {
        tx.execute(
            "INSERT INTO metric_values (id, revision, name, value, unit)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, revision, metric.name, metric.value, metric.unit],
        )
        .map_err(|e| format!("cannot store metric: {e}"))?;
    }
    Ok(())
}

fn create_inner(
    conn: &mut Connection,
    input: &NewRecord,
    measurement: Option<&crate::knowledge::BenchmarkData>,
    inject_fault: bool,
) -> Result<(u64, u64), String> {
    crate::knowledge::validate_new(input)?;
    if input.kind == "benchmark" && measurement.is_none() {
        return Err("benchmark records require measurement".to_string());
    }
    revalidate_open_paths(conn)?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("cannot begin knowledge write: {e}"))?;
    let exists: i64 = tx
        .query_row(
            "SELECT count(*) FROM heads WHERE id = ?1",
            [&input.id],
            |row| row.get(0),
        )
        .map_err(|e| format!("cannot check record existence: {e}"))?;
    if exists != 0 {
        return Err(format!("duplicate record id: '{}'", input.id));
    }
    tx.execute(
        "INSERT INTO records (id, revision, kind, title, body, status, author, created_unix)
         VALUES (?1, 1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            input.id,
            input.kind,
            input.title,
            input.body,
            input.status,
            input.author,
            now_unix()
        ],
    )
    .map_err(|e| format!("cannot insert record revision: {e}"))?;
    if inject_fault {
        // Test hook: drop the transaction here to prove partial writes roll
        // back fully (no head, FTS, measurement or generation survives).
        return Err("injected write failure".to_string());
    }
    tx.execute(
        "INSERT INTO heads (id, current_revision) VALUES (?1, 1)",
        [&input.id],
    )
    .map_err(|e| format!("cannot advance record head: {e}"))?;
    if let Some(data) = measurement {
        write_measurement(&tx, &input.id, 1, data)?;
    }
    let generation = bump_generation(&tx)?;
    tx.commit()
        .map_err(|e| format!("cannot commit record creation: {e}"))?;
    Ok((1, generation))
}

/// Append a new immutable revision. The expected head is verified inside the
/// write transaction, so concurrent writers serialize: exactly one wins and
/// the loser fails with a conflict, committing nothing.
pub fn update_record(
    conn: &mut Connection,
    id: &str,
    expected_revision: u64,
    input: &UpdateRecord,
) -> Result<(u64, u64), String> {
    update_inner(conn, id, expected_revision, input, None, false)
}

/// Update from strictly validated JSON input. A supplied measurement replaces
/// the claim on the new revision; when absent on a benchmark revision, the
/// previous revision's measurement is carried forward unchanged so reads stay
/// complete. Evidence refs are validated at parse time; capture belongs to
/// Task 4.
pub fn update_validated(
    conn: &mut Connection,
    id: &str,
    expected_revision: u64,
    input: &crate::knowledge::ValidatedUpdate,
) -> Result<(u64, u64), String> {
    update_inner(
        conn,
        id,
        expected_revision,
        &input.record,
        input.measurement.as_ref(),
        false,
    )
}

/// Test-only fault injector: fails after the records insert to prove the
/// whole update rolls back.
#[cfg(test)]
pub(crate) fn update_validated_with_fault(
    conn: &mut Connection,
    id: &str,
    expected_revision: u64,
    input: &crate::knowledge::ValidatedUpdate,
) -> Result<(u64, u64), String> {
    update_inner(
        conn,
        id,
        expected_revision,
        &input.record,
        input.measurement.as_ref(),
        true,
    )
}

/// Carry a benchmark claim forward to a new revision unchanged.
fn carry_measurement(tx: &Transaction, id: &str, from_rev: i64, to_rev: i64) -> Result<(), String> {
    let copied = tx
        .execute(
            "INSERT INTO measurements (id, revision, method, environment, sample_count)
             SELECT ?1, ?2, method, environment, sample_count FROM measurements
             WHERE id = ?1 AND revision = ?3",
            rusqlite::params![id, to_rev, from_rev],
        )
        .map_err(|e| format!("cannot carry measurement: {e}"))?;
    if copied != 1 {
        return Err(format!("benchmark revision without measurement: '{id}'"));
    }
    tx.execute(
        "INSERT INTO metric_values (id, revision, name, value, unit)
         SELECT ?1, ?2, name, value, unit FROM metric_values
         WHERE id = ?1 AND revision = ?3",
        rusqlite::params![id, to_rev, from_rev],
    )
    .map_err(|e| format!("cannot carry metrics: {e}"))?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn update_inner(
    conn: &mut Connection,
    id: &str,
    expected_revision: u64,
    input: &UpdateRecord,
    measurement: Option<&crate::knowledge::BenchmarkData>,
    inject_fault: bool,
) -> Result<(u64, u64), String> {
    crate::knowledge::validate_update(input)?;
    revalidate_open_paths(conn)?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("cannot begin knowledge write: {e}"))?;
    let current: Option<i64> = tx
        .query_row(
            "SELECT current_revision FROM heads WHERE id = ?1",
            [id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("cannot read record head: {e}"))?;
    let current = match current {
        Some(c) => c,
        None => return Err(format!("unknown record id: '{id}'")),
    };
    // An unrepresentable expectation can never match; a corrupt negative head
    // fails here too, before any mutation is attempted.
    let expected_matches = i64::try_from(expected_revision)
        .map(|expected| expected == current)
        .unwrap_or(false);
    if !expected_matches {
        return Err(format!(
            "revision conflict for '{id}': expected {expected_revision} but head is {current}"
        ));
    }
    let kind: String = tx
        .query_row(
            "SELECT kind FROM records WHERE id = ?1 AND revision = ?2",
            rusqlite::params![id, current],
            |row| row.get(0),
        )
        .map_err(|e| format!("cannot read record kind: {e}"))?;
    // The immutable kind decides measurement eligibility inside the
    // transaction, before any mutation: only benchmark revisions may carry
    // measurement, whether supplied or carried forward.
    if kind != "benchmark" && measurement.is_some() {
        return Err("measurement is only valid for benchmark records".to_string());
    }
    let next_i64 = current
        .checked_add(1)
        .ok_or_else(|| "knowledge revision counter overflow".to_string())?;
    let next =
        u64::try_from(next_i64).map_err(|_| "knowledge revision counter overflow".to_string())?;
    tx.execute(
        "INSERT INTO records (id, revision, kind, title, body, status, author, created_unix)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            id,
            next_i64,
            kind,
            input.title,
            input.body,
            input.status,
            input.author,
            now_unix()
        ],
    )
    .map_err(|e| format!("cannot insert record revision: {e}"))?;
    if inject_fault {
        // Test hook: drop the transaction here to prove partial writes roll
        // back fully (no head, FTS, measurement or generation survives).
        return Err("injected write failure".to_string());
    }
    tx.execute(
        "UPDATE heads SET current_revision = ?1 WHERE id = ?2",
        rusqlite::params![next_i64, id],
    )
    .map_err(|e| format!("cannot advance record head: {e}"))?;
    match measurement {
        Some(data) => write_measurement(&tx, id, next_i64, data)?,
        None if kind == "benchmark" => carry_measurement(&tx, id, current, next_i64)?,
        None => {}
    }
    let generation = bump_generation(&tx)?;
    tx.commit()
        .map_err(|e| format!("cannot commit record update: {e}"))?;
    Ok((next, generation))
}

/// Measurement attached to one revision, if any. Metrics order by insertion.
fn read_measurement(
    conn: &Connection,
    id: &str,
    revision: i64,
) -> Result<Option<crate::knowledge::BenchmarkData>, String> {
    let row: Option<(String, String, i64)> = conn
        .query_row(
            "SELECT method, environment, sample_count FROM measurements
             WHERE id = ?1 AND revision = ?2",
            rusqlite::params![id, revision],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()
        .map_err(|e| format!("cannot read measurement: {e}"))?;
    let Some((method, environment, sample)) = row else {
        return Ok(None);
    };
    let sample_count = u64::try_from(sample)
        .map_err(|_| "corrupt knowledge store: sample count out of range".to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT name, value, unit FROM metric_values
             WHERE id = ?1 AND revision = ?2 ORDER BY rowid",
        )
        .map_err(|e| format!("cannot read measurement: {e}"))?;
    let metrics = stmt
        .query_map(rusqlite::params![id, revision], |r| {
            Ok(crate::knowledge::MetricData {
                name: r.get(0)?,
                value: r.get(1)?,
                unit: r.get(2)?,
            })
        })
        .map_err(|e| format!("cannot read measurement: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("cannot read measurement: {e}"))?;
    Ok(Some(crate::knowledge::BenchmarkData {
        method,
        environment,
        sample_count,
        metrics,
    }))
}

/// Read one revision (`None` = current head). Never creates anything.
pub fn show_record(
    conn: &Connection,
    id: &str,
    revision: Option<u64>,
) -> Result<RecordVersion, String> {
    let rev = match revision {
        Some(r) => r,
        None => {
            let current: i64 = conn
                .query_row(
                    "SELECT current_revision FROM heads WHERE id = ?1",
                    [id],
                    |row| row.get(0),
                )
                .map_err(|_| format!("unknown record id: '{id}'"))?;
            u64::try_from(current)
                .map_err(|_| "corrupt knowledge store: stored revision out of range".to_string())?
        }
    };
    let rev_i64 = i64::try_from(rev).map_err(|_| format!("record revision out of range: {rev}"))?;
    let mut record = conn
        .query_row(
            &format!("SELECT {RECORD_COLS} FROM records WHERE id = ?1 AND revision = ?2"),
            rusqlite::params![id, rev_i64],
            record_from_row,
        )
        .map_err(|e| match e {
            rusqlite::Error::FromSqlConversionFailure(..) => {
                "corrupt knowledge store: stored revision out of range".to_string()
            }
            _ => format!("unknown record revision: '{id}' rev {rev}"),
        })?;
    record.measurement = read_measurement(conn, id, rev_i64)?;
    Ok(record)
}

/// Deterministic history by revision with a generation guard: a supplied
/// generation that no longer matches fails instead of serving shifted pages.
///
/// Generation, total and rows are read inside one SQLite read transaction, so
/// a concurrent commit landing mid-read can neither mix generations with rows
/// nor shift pages; the whole page reflects a single snapshot.
pub fn history(
    conn: &Connection,
    id: &str,
    limit: usize,
    offset: usize,
    generation: Option<u64>,
) -> Result<HistoryPage, String> {
    history_inner(conn, id, limit, offset, generation, None)
}

/// Test-only probe invoked after generation validation but before row reads,
/// so a test can commit from another connection exactly on that boundary.
#[cfg(test)]
pub(crate) fn history_with_probe(
    conn: &Connection,
    id: &str,
    limit: usize,
    offset: usize,
    generation: Option<u64>,
    probe: Option<&dyn Fn()>,
) -> Result<HistoryPage, String> {
    history_inner(conn, id, limit, offset, generation, probe)
}

fn history_inner(
    conn: &Connection,
    id: &str,
    limit: usize,
    offset: usize,
    generation: Option<u64>,
    probe: Option<&dyn Fn()>,
) -> Result<HistoryPage, String> {
    let limit_i64 =
        i64::try_from(limit).map_err(|_| format!("history limit out of range: {limit}"))?;
    let offset_i64 =
        i64::try_from(offset).map_err(|_| format!("history offset out of range: {offset}"))?;
    conn.execute_batch("BEGIN")
        .map_err(|e| format!("cannot begin knowledge read: {e}"))?;
    let page = (|| {
        let current = read_generation(conn)?;
        if let Some(expected) = generation {
            if expected != current {
                return Err(format!(
                    "knowledge generation conflict: expected {expected} but store is at {current}"
                ));
            }
        }
        if let Some(run) = probe {
            run();
        }
        let total: i64 = conn
            .query_row("SELECT count(*) FROM records WHERE id = ?1", [id], |row| {
                row.get(0)
            })
            .map_err(|e| format!("cannot read record history: {e}"))?;
        if total == 0 {
            return Err(format!("unknown record id: '{id}'"));
        }
        let total_usize = usize::try_from(total)
            .map_err(|_| "corrupt knowledge store: history total out of range".to_string())?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {RECORD_COLS} FROM records WHERE id = ?1
                 ORDER BY revision ASC LIMIT ?2 OFFSET ?3"
            ))
            .map_err(|e| format!("cannot read record history: {e}"))?;
        let mut revisions = stmt
            .query_map(
                rusqlite::params![id, limit_i64, offset_i64],
                record_from_row,
            )
            .map_err(|e| map_row_error(e, "cannot read record history"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| map_row_error(e, "cannot read record history"))?;
        for record in &mut revisions {
            let rev_i64 = i64::try_from(record.revision)
                .map_err(|_| "corrupt knowledge store: stored revision out of range".to_string())?;
            record.measurement = read_measurement(conn, id, rev_i64)?;
        }
        let end = offset.saturating_add(revisions.len());
        Ok(HistoryPage {
            truncated: end < total_usize,
            total: total_usize,
            generation: current,
            revisions,
        })
    })();
    match page {
        Ok(built) => {
            conn.execute_batch("COMMIT")
                .map_err(|e| format!("cannot commit knowledge read: {e}"))?;
            Ok(built)
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(e)
        }
    }
}

/// A stored revision outside `u64` range surfaces as store corruption, never
/// as a panic and never disguised as a missing row.
fn map_row_error(e: rusqlite::Error, otherwise: &str) -> String {
    match e {
        rusqlite::Error::FromSqlConversionFailure(..) => {
            "corrupt knowledge store: stored revision out of range".to_string()
        }
        _ => otherwise.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::tmpdir;

    fn setup(name: &str) -> (PathBuf, RepoEntry) {
        let home = tmpdir("m7-home");
        let root = tmpdir("m7-root");
        // Fixtures mirror production registry entries, which record the
        // canonical root string at registration time.
        let canonical = crate::paths::canonical_root(&root)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let repo = RepoEntry {
            name: name.to_string(),
            root: canonical,
            added_unix: 1,
        };
        (home, repo)
    }

    fn companion(db: &Path, suffix: &str) -> PathBuf {
        PathBuf::from(format!("{}{suffix}", db.display()))
    }

    #[test]
    fn knowledge_store_open_db_path_layout() {
        let home = PathBuf::from("/tmp/h");
        assert_eq!(
            knowledge_db_path(&home, "myrepo"),
            PathBuf::from("/tmp/h/knowledge/myrepo.db")
        );
    }

    #[test]
    fn knowledge_store_open_missing_store_read_creates_nothing() {
        let (home, repo) = setup("missing");
        let db = knowledge_db_path(&home, "missing");
        let err = open_knowledge_at(&home, &repo, false).unwrap_err();
        assert!(err.contains("no knowledge store"), "unexpected: {err}");
        assert!(!db.exists(), "read must not create the database");
        for s in ["-wal", "-shm", "-journal"] {
            assert!(
                !companion(&db, s).exists(),
                "read must not create companion {s}"
            );
        }
    }

    #[test]
    fn knowledge_store_open_readonly_never_creates_and_reads() {
        let (home, repo) = setup("ro");
        open_knowledge_at(&home, &repo, true).unwrap();
        let conn = open_knowledge_at(&home, &repo, false).unwrap();
        assert!(conn
            .execute_batch("CREATE TABLE probe_x (a INTEGER)")
            .is_err());
        conn.close().unwrap();
    }

    #[test]
    fn knowledge_store_open_fresh_create_builds_schema_v1() {
        let (home, repo) = setup("fresh");
        let conn = open_knowledge_at(&home, &repo, true).unwrap();
        let version: u32 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, KNOWLEDGE_SCHEMA_VERSION);
        for table in [
            "meta",
            "records",
            "heads",
            "evidence",
            "relations",
            "retractions",
            "knowledge_fts",
        ] {
            let n: i64 = conn
                .query_row(
                    "SELECT count(*) FROM sqlite_master WHERE name = ?1",
                    [table],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(n, 1, "schema v1 must contain {table}");
        }
        let name: String = conn
            .query_row("SELECT value FROM meta WHERE key = 'repo_name'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(name, "fresh");
        conn.close().unwrap();
        // The M6 code index is never touched by knowledge operations.
        assert!(!home.join("index").join("fresh.db").exists());
    }

    #[test]
    fn knowledge_store_open_corrupt_file_fails_closed() {
        let (home, repo) = setup("corrupt");
        let db = knowledge_db_path(&home, "corrupt");
        std::fs::create_dir_all(db.parent().unwrap()).unwrap();
        std::fs::write(&db, b"not-a-database").unwrap();
        let err = open_knowledge_at(&home, &repo, true).unwrap_err();
        assert!(
            err.contains("readable") || err.contains("corrupt"),
            "unexpected: {err}"
        );
        assert_eq!(std::fs::read(&db).unwrap(), b"not-a-database");
    }

    #[test]
    fn knowledge_store_open_forward_version_fails() {
        let (home, repo) = setup("fwd");
        open_knowledge_at(&home, &repo, true)
            .unwrap()
            .close()
            .unwrap();
        let raw = rusqlite::Connection::open(knowledge_db_path(&home, "fwd")).unwrap();
        raw.execute_batch("PRAGMA user_version = 99").unwrap();
        raw.close().unwrap();
        let err = open_knowledge_at(&home, &repo, false).unwrap_err();
        assert!(err.contains("forward"), "unexpected: {err}");
    }

    #[test]
    fn knowledge_store_open_meta_mismatch_fails() {
        for (key, bad) in [("repo_name", "other"), ("knowledge_schema_version", "2")] {
            let (home, repo) = setup("mismatch");
            open_knowledge_at(&home, &repo, true)
                .unwrap()
                .close()
                .unwrap();
            let raw = rusqlite::Connection::open(knowledge_db_path(&home, "mismatch")).unwrap();
            raw.execute("UPDATE meta SET value = ?1 WHERE key = ?2", [bad, key])
                .unwrap();
            raw.close().unwrap();
            let err = open_knowledge_at(&home, &repo, false).unwrap_err();
            assert!(err.contains("mismatch"), "key {key}: unexpected: {err}");
        }
        // Wrong canonical root: point the entry at another existing dir.
        let (home, repo) = setup("mismatch-root");
        open_knowledge_at(&home, &repo, true)
            .unwrap()
            .close()
            .unwrap();
        let other_root = tmpdir("m7-other-root");
        let moved = RepoEntry {
            name: repo.name.clone(),
            root: other_root.to_string_lossy().into_owned(),
            added_unix: 1,
        };
        let err = open_knowledge_at(&home, &moved, false).unwrap_err();
        assert!(err.contains("mismatch"), "root: unexpected: {err}");
    }

    #[test]
    fn knowledge_store_open_fts_follows_head() {
        let (home, repo) = setup("ftshead");
        let conn = open_knowledge_at(&home, &repo, true).unwrap();
        for (rev, title) in [
            (1, "zqxalpha first decision"),
            (2, "zqxbeta revised decision"),
        ] {
            conn.execute(
                "INSERT INTO records (id, revision, kind, title, body, status, author, created_unix)
                 VALUES ('r1', ?1, 'decision', ?2, 'body', 'active', 't', 1)",
                rusqlite::params![rev, title],
            )
            .unwrap();
        }
        // No head yet: inserting revisions alone must not index anything.
        let unheaded: i64 = conn
            .query_row("SELECT count(*) FROM knowledge_fts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(unheaded, 0, "revisions without a head must not be indexed");
        // Head at rev1: FTS is rev1 only, even though rev2 exists.
        conn.execute(
            "INSERT INTO heads (id, current_revision) VALUES ('r1', 1)",
            [],
        )
        .unwrap();
        let stale: i64 = conn
            .query_row(
                "SELECT count(*) FROM knowledge_fts WHERE knowledge_fts MATCH 'zqxalpha'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stale, 1, "head revision must be indexed");
        let future: i64 = conn
            .query_row(
                "SELECT count(*) FROM knowledge_fts WHERE knowledge_fts MATCH 'zqxbeta'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(future, 0, "non-head revision must not be indexed");
        // Moving the head moves the index.
        conn.execute("UPDATE heads SET current_revision = 2 WHERE id = 'r1'", [])
            .unwrap();
        let stale_after: i64 = conn
            .query_row(
                "SELECT count(*) FROM knowledge_fts WHERE knowledge_fts MATCH 'zqxalpha'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stale_after, 0, "old head must leave the index");
        let head_after: i64 = conn
            .query_row(
                "SELECT count(*) FROM knowledge_fts WHERE knowledge_fts MATCH 'zqxbeta'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(head_after, 1, "new head must be indexed");
        // Immutable history is preserved regardless of the index.
        let rows: i64 = conn
            .query_row("SELECT count(*) FROM records WHERE id = 'r1'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(rows, 2, "historical revisions are never removed");
        // Rolling back a head move rolls back the index with it.
        conn.execute_batch("BEGIN IMMEDIATE").unwrap();
        conn.execute("UPDATE heads SET current_revision = 1 WHERE id = 'r1'", [])
            .unwrap();
        let in_txn: i64 = conn
            .query_row(
                "SELECT count(*) FROM knowledge_fts WHERE knowledge_fts MATCH 'zqxalpha'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(in_txn, 1, "index follows the head inside the transaction");
        conn.execute_batch("ROLLBACK").unwrap();
        let head_rev: i64 = conn
            .query_row(
                "SELECT current_revision FROM heads WHERE id = 'r1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(head_rev, 2);
        let rolled_back: i64 = conn
            .query_row(
                "SELECT count(*) FROM knowledge_fts WHERE knowledge_fts MATCH 'zqxbeta'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            rolled_back, 1,
            "rolled-back head move must not leak index changes"
        );
        conn.close().unwrap();
    }

    #[test]
    fn knowledge_store_open_init_failure_rolls_back() {
        let (home, repo) = setup("initfault");
        let err = open_knowledge_at_inject_init_failure(&home, &repo).unwrap_err();
        assert!(err.contains("injected"), "unexpected: {err}");
        // No partial schema or metadata survived the failed initialization.
        let raw = rusqlite::Connection::open(knowledge_db_path(&home, "initfault")).unwrap();
        let objects: i64 = raw
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type IN ('table', 'trigger')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(objects, 0, "failed init must leave no tables or triggers");
        let version: u32 = raw
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, 0, "failed init must not stamp a version");
        raw.close().unwrap();
        // The leftover file is not repaired or deleted automatically: the next
        // open fails explicitly.
        let err = open_knowledge_at(&home, &repo, true).unwrap_err();
        assert!(err.contains("corrupt"), "unexpected: {err}");
        assert!(knowledge_db_path(&home, "initfault").exists());
    }

    #[test]
    fn knowledge_store_open_incomplete_schema_rejected() {
        for drop_sql in ["DROP TABLE heads", "DROP TABLE knowledge_fts"] {
            let (home, repo) = setup("incomplete");
            open_knowledge_at(&home, &repo, true)
                .unwrap()
                .close()
                .unwrap();
            let raw = rusqlite::Connection::open(knowledge_db_path(&home, "incomplete")).unwrap();
            raw.execute_batch(drop_sql).unwrap();
            raw.close().unwrap();
            let err = open_knowledge_at(&home, &repo, false).unwrap_err();
            assert!(err.contains("incomplete"), "{drop_sql}: unexpected: {err}");
        }
    }

    #[test]
    fn knowledge_store_open_moved_root_mismatch() {
        let (home, repo) = setup("movedroot");
        open_knowledge_at(&home, &repo, true)
            .unwrap()
            .close()
            .unwrap();
        std::fs::remove_dir_all(Path::new(&repo.root)).unwrap();
        let err = open_knowledge_at(&home, &repo, false).unwrap_err();
        assert!(
            err.contains("resolve") || err.contains("mismatch"),
            "unexpected: {err}"
        );
    }

    #[test]
    fn knowledge_store_open_fresh_create_refuses_retarget() {
        // The recorded path is rebound to another tree before the first
        // knowledge store is created: the live canonical root no longer
        // matches the recorded string, so creation must fail. Rebinding the
        // path itself (rather than swapping the entry string) detects the
        // retarget on every platform, including filesystems where a raw path
        // already equals its canonical form.
        let (home, repo) = setup("retarget");
        let other = tmpdir("m7-retarget-target");
        if !repoint_dir_to(Path::new(&repo.root), &other) {
            eprintln!("NOT TESTED: cannot rebind directories in this environment");
            return;
        }
        let err = open_knowledge_at(&home, &repo, true).unwrap_err();
        assert!(err.contains("retarget"), "unexpected: {err}");
        assert!(
            !knowledge_db_path(&home, "retarget").exists(),
            "refused creation must leave no store file behind"
        );
    }

    #[test]
    fn knowledge_store_open_fresh_create_compatible_root() {
        // The normal compatible path still works: the recorded canonical
        // root creates fine, as does the no-index case (other tests).
        let (home, repo) = setup("retarget-ok");
        open_knowledge_at(&home, &repo, true)
            .unwrap()
            .close()
            .unwrap();
    }

    /// Replace the directory at `link` with a link to `target`, so the
    /// recorded path resolves to another tree. Returns false when the
    /// environment cannot create the link (caller reports NOT TESTED).
    fn repoint_dir_to(link: &Path, target: &Path) -> bool {
        if std::fs::remove_dir_all(link).is_err() {
            return false;
        }
        #[cfg(unix)]
        {
            if std::os::unix::fs::symlink(target, link).is_err() {
                return false;
            }
        }
        #[cfg(windows)]
        {
            if plant_link(link, target).is_none() {
                return false;
            }
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = (link, target);
            return false;
        }
        // The link primitive reported success, but some environments leave a
        // reparse point the OS itself cannot resolve (observed as Access
        // denied on canonicalize). Only a resolvable link exercises the
        // retarget path; otherwise report NOT TESTED instead of failing an
        // environment limitation as a code defect.
        if link.canonicalize().is_err() {
            let _ = std::fs::remove_file(link);
            let _ = std::fs::remove_dir(link);
            return false;
        }
        true
    }

    #[test]
    fn knowledge_store_open_db_path_is_directory() {
        let (home, repo) = setup("isdir");
        let db = knowledge_db_path(&home, "isdir");
        std::fs::create_dir_all(&db).unwrap();
        let err = open_knowledge_at(&home, &repo, true).unwrap_err();
        assert!(err.contains("not a file"), "unexpected: {err}");
    }

    #[test]
    fn knowledge_store_open_unusable_home_fails() {
        let (home, repo) = setup("badhome");
        let file = home.join("file-home");
        std::fs::write(&file, b"x").unwrap();
        let err = open_knowledge_at(&file, &repo, true).unwrap_err();
        assert!(
            err.contains("cannot create") || err.contains("cannot resolve"),
            "unexpected: {err}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn knowledge_store_open_non_utf8_home_supported() {
        use std::os::unix::ffi::OsStringExt;
        let mut bytes = std::env::temp_dir().into_os_string().into_vec();
        bytes.extend_from_slice(format!("/riltra-m7-nonutf-{}-", std::process::id()).as_bytes());
        bytes.extend_from_slice(&[0x66u8, 0x6f, 0x80, 0x6f]);
        let home = PathBuf::from(std::ffi::OsString::from_vec(bytes));
        let _ = std::fs::remove_dir_all(&home);
        // Some filesystems (e.g. APFS) cannot represent non-UTF-8 names at
        // all: probe creation first and report NOT TESTED instead of failing
        // an environment limitation as a code defect.
        if std::fs::create_dir_all(&home).is_err() {
            eprintln!("NOT TESTED: filesystem rejects non-UTF-8 names");
            return;
        }
        let root = tmpdir("m7-root");
        let repo = RepoEntry {
            name: "nonutf".to_string(),
            root: crate::paths::canonical_root(&root)
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            added_unix: 1,
        };
        let conn = open_knowledge_at(&home, &repo, true).unwrap();
        conn.close().unwrap();
        assert!(knowledge_db_path(&home, "nonutf").is_file());
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn knowledge_store_open_hostile_db_symlink_rejected() {
        let (home, repo) = setup("dblink");
        let db = knowledge_db_path(&home, "dblink");
        std::fs::create_dir_all(db.parent().unwrap()).unwrap();
        let outside = tmpdir("m7-outside");
        let target = outside.join("evil.db");
        std::fs::write(&target, b"SENTINEL").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &db).unwrap();
        #[cfg(windows)]
        match plant_link(&db, &target) {
            Some(mech) => eprintln!("hostile db link planted via {mech}"),
            None => {
                eprintln!("NOT TESTED: cannot plant links in this environment");
                return;
            }
        }
        let err = open_knowledge_at(&home, &repo, true).unwrap_err();
        assert!(
            err.contains("link") || err.contains("escape"),
            "unexpected: {err}"
        );
        assert_eq!(std::fs::read(&target).unwrap(), b"SENTINEL");
    }

    #[test]
    fn knowledge_store_open_hostile_companion_symlink_rejected() {
        let (home, repo) = setup("complink");
        open_knowledge_at(&home, &repo, true)
            .unwrap()
            .close()
            .unwrap();
        let db = knowledge_db_path(&home, "complink");
        let wal = companion(&db, "-wal");
        let _ = std::fs::remove_file(&wal);
        let outside = tmpdir("m7-outside-wal");
        let target = outside.join("evil-wal");
        std::fs::write(&target, b"SENTINEL").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &wal).unwrap();
        #[cfg(windows)]
        match plant_link(&wal, &target) {
            Some(mech) => eprintln!("hostile companion link planted via {mech}"),
            None => {
                eprintln!("NOT TESTED: cannot plant links in this environment");
                return;
            }
        }
        let err = open_knowledge_at(&home, &repo, false).unwrap_err();
        assert!(
            err.contains("link") || err.contains("escape"),
            "unexpected: {err}"
        );
        assert_eq!(std::fs::read(&target).unwrap(), b"SENTINEL");
    }

    #[test]
    fn knowledge_store_open_dangling_companion_symlink_rejected() {
        let (home, repo) = setup("dangling");
        open_knowledge_at(&home, &repo, true)
            .unwrap()
            .close()
            .unwrap();
        let db = knowledge_db_path(&home, "dangling");
        let shm = companion(&db, "-shm");
        let _ = std::fs::remove_file(&shm);
        let ghost = tmpdir("m7-ghost").join("nope-shm");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&ghost, &shm).unwrap();
        #[cfg(windows)]
        match plant_link(&shm, &ghost) {
            Some(mech) => eprintln!("dangling companion link planted via {mech}"),
            None => {
                eprintln!("NOT TESTED: cannot plant links in this environment");
                return;
            }
        }
        let err = open_knowledge_at(&home, &repo, false).unwrap_err();
        assert!(
            err.contains("link") || err.contains("escape") || err.contains("dangling"),
            "unexpected: {err}"
        );
    }

    #[test]
    fn knowledge_store_open_post_open_recheck_rejects_link() {
        // The DB path itself is a link: the post-open canonical re-check is
        // the normative guard because SQLite follows links on open.
        let (home, repo) = setup("recheck");
        let db = knowledge_db_path(&home, "recheck");
        std::fs::create_dir_all(db.parent().unwrap()).unwrap();
        let outside = tmpdir("m7-outside-recheck");
        let target = outside.join("real.db");
        std::fs::write(&target, b"SENTINEL").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &db).unwrap();
        #[cfg(windows)]
        match plant_link(&db, &target) {
            Some(mech) => eprintln!("recheck link planted via {mech}"),
            None => {
                eprintln!("NOT TESTED: cannot plant links in this environment");
                return;
            }
        }
        let err = open_knowledge_at(&home, &repo, true).unwrap_err();
        assert!(
            err.contains("link") || err.contains("escape"),
            "unexpected: {err}"
        );
    }

    #[cfg(windows)]
    #[test]
    fn knowledge_store_open_windows_junction_knowledge_dir_rejected() {
        let (home, repo) = setup("junction");
        let kdir = home.join("knowledge");
        std::fs::create_dir_all(&home).unwrap();
        let outside = tmpdir("m7-outside-junction");
        let status = std::process::Command::new("cmd")
            .args([
                "/C",
                "mklink",
                "/J",
                kdir.to_str().unwrap(),
                outside.to_str().unwrap(),
            ])
            .output();
        let created = status.map(|o| o.status.success()).unwrap_or(false) && is_junction(&kdir);
        if !created {
            eprintln!("NOT TESTED: cannot create junctions in this environment");
            return;
        }
        let err = open_knowledge_at(&home, &repo, true).unwrap_err();
        assert!(
            err.contains("link") || err.contains("escape") || err.contains("retarget"),
            "unexpected: {err}"
        );
    }

    #[cfg(windows)]
    fn is_junction(p: &Path) -> bool {
        std::fs::symlink_metadata(p)
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false)
            || p.canonicalize().map(|c| c != *p).unwrap_or(false)
    }

    /// Plant a hostile link for rejection tests. Returns the mechanism used,
    /// or `None` when the environment allows neither (caller reports
    /// NOT TESTED). The junction fallback names a directory junction like a
    /// file; rejection runs before any type check, so it exercises the same
    /// pre/post-open scan path as a file symlink.
    #[cfg(windows)]
    fn plant_link(link: &Path, target: &Path) -> Option<&'static str> {
        if std::os::windows::fs::symlink_file(target, link).is_ok() {
            return Some("symlink");
        }
        let anchor = if target.is_dir() {
            target.to_path_buf()
        } else {
            match target.parent() {
                Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
                _ => return None,
            }
        };
        // mklink cannot consume verbatim (`\\?\`) paths: it reports success
        // yet leaves a junction Windows cannot resolve (later surfacing as
        // Access denied). Strip the prefix for the link tool only.
        let ok = std::process::Command::new("cmd")
            .args([
                "/C",
                "mklink",
                "/J",
                &for_mklink(link),
                &for_mklink(&anchor),
            ])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if ok && std::fs::symlink_metadata(link).is_ok() {
            Some("junction-fallback")
        } else {
            None
        }
    }

    #[cfg(windows)]
    fn for_mklink(path: &Path) -> String {
        let text = path.to_string_lossy();
        if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
            return format!(r"\\{rest}");
        }
        if let Some(rest) = text.strip_prefix(r"\\?\") {
            return rest.to_string();
        }
        text.into_owned()
    }

    #[cfg(not(windows))]
    #[test]
    fn knowledge_store_open_unix_dir_symlink_rejected() {
        let (home, repo) = setup("dirlink");
        let kdir = home.join("knowledge");
        let outside = tmpdir("m7-outside-dir");
        std::os::unix::fs::symlink(&outside, &kdir).unwrap();
        let err = open_knowledge_at(&home, &repo, true).unwrap_err();
        assert!(
            err.contains("link") || err.contains("escape") || err.contains("retarget"),
            "unexpected: {err}"
        );
    }

    use crate::knowledge::{validate_create_json, validate_update_json, NewRecord, UpdateRecord};

    fn new_record(id: &str, title: &str) -> NewRecord {
        NewRecord {
            id: id.to_string(),
            kind: "decision".to_string(),
            title: title.to_string(),
            body: format!("body of {title}"),
            status: "active".to_string(),
            author: "tester".to_string(),
        }
    }

    fn update_to(title: &str, status: &str) -> UpdateRecord {
        UpdateRecord {
            title: title.to_string(),
            body: format!("body of {title}"),
            status: status.to_string(),
            author: "tester".to_string(),
        }
    }

    fn open_rw(home: &Path, repo: &RepoEntry) -> rusqlite::Connection {
        open_knowledge_at(home, repo, true).unwrap()
    }

    #[test]
    fn knowledge_records_create_show_update_history() {
        let (home, repo) = setup("revs");
        let mut conn = open_rw(&home, &repo);
        let (rev, gen) = create_record(&mut conn, &new_record("d1", "first zqxtitle")).unwrap();
        assert_eq!((rev, gen), (1, 1));
        let shown = show_record(&conn, "d1", None).unwrap();
        assert_eq!(shown.revision, 1);
        assert_eq!(shown.title, "first zqxtitle");
        let (rev2, gen2) =
            update_record(&mut conn, "d1", 1, &update_to("second zqxtitle", "active")).unwrap();
        assert_eq!((rev2, gen2), (2, 2));
        let old = show_record(&conn, "d1", Some(1)).unwrap();
        assert_eq!(old.title, "first zqxtitle");
        let page = history(&conn, "d1", 50, 0, None).unwrap();
        assert_eq!(page.total, 2);
        assert!(!page.truncated);
        assert_eq!(page.generation, 2);
        assert_eq!(
            page.revisions
                .iter()
                .map(|r| r.revision)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
        conn.close().unwrap();
    }

    #[test]
    fn knowledge_records_stale_expected_revision_conflict() {
        let (home, repo) = setup("stale");
        let mut conn = open_rw(&home, &repo);
        update_record(&mut conn, "missing-never", 1, &update_to("x", "active")).unwrap_err();
        create_record(&mut conn, &new_record("d1", "stable zqxone")).unwrap();
        update_record(&mut conn, "d1", 1, &update_to("moved zqxtwo", "active")).unwrap();
        let gen_before = read_generation(&conn).unwrap();
        let err =
            update_record(&mut conn, "d1", 1, &update_to("loser zqxthree", "active")).unwrap_err();
        assert!(err.contains("conflict"), "unexpected: {err}");
        // No write and no generation bump happened.
        assert_eq!(show_record(&conn, "d1", None).unwrap().revision, 2);
        assert_eq!(read_generation(&conn).unwrap(), gen_before);
        let page = history(&conn, "d1", 50, 0, None).unwrap();
        assert_eq!(page.total, 2);
        let hits: i64 = conn
            .query_row(
                "SELECT count(*) FROM knowledge_fts WHERE knowledge_fts MATCH 'zqxthree'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(hits, 0, "failed update must not leak into the index");
        conn.close().unwrap();
    }

    #[test]
    fn knowledge_records_duplicate_id_fails() {
        let (home, repo) = setup("dup");
        let mut conn = open_rw(&home, &repo);
        create_record(&mut conn, &new_record("d1", "t1")).unwrap();
        let err = create_record(&mut conn, &new_record("d1", "t2")).unwrap_err();
        assert!(err.contains("duplicate"), "unexpected: {err}");
        // Same id with a different kind is still a duplicate, not a rename.
        let mut other_kind = new_record("d1", "t3");
        other_kind.kind = "constraint".to_string();
        let err = create_record(&mut conn, &other_kind).unwrap_err();
        assert!(err.contains("duplicate"), "unexpected: {err}");
        assert_eq!(history(&conn, "d1", 50, 0, None).unwrap().total, 1);
        conn.close().unwrap();
    }

    #[test]
    fn knowledge_records_id_and_kind_immutable() {
        let (home, repo) = setup("immutable");
        let mut conn = open_rw(&home, &repo);
        create_record(&mut conn, &new_record("d1", "t1")).unwrap();
        update_record(&mut conn, "d1", 1, &update_to("t2", "active")).unwrap();
        let shown = show_record(&conn, "d1", None).unwrap();
        assert_eq!(shown.id, "d1");
        assert_eq!(shown.kind, "decision");
        conn.close().unwrap();
    }

    #[test]
    fn knowledge_records_invalid_kind_and_status_rejected() {
        let (home, repo) = setup("invalid");
        let mut conn = open_rw(&home, &repo);
        let mut bad_kind = new_record("d1", "t");
        bad_kind.kind = "note".to_string();
        assert!(create_record(&mut conn, &bad_kind).is_err());
        for status in ["superseded", "retracted", "bogus"] {
            let mut bad_status = new_record("d1", "t");
            bad_status.status = status.to_string();
            let err = create_record(&mut conn, &bad_status).unwrap_err();
            assert!(err.contains("status"), "{status}: unexpected: {err}");
        }
        // Nothing was written by the rejected inputs.
        assert!(show_record(&conn, "d1", None).is_err());
        assert_eq!(read_generation(&conn).unwrap(), 0);
        conn.close().unwrap();
    }

    #[test]
    fn knowledge_records_archive_restore_new_revisions() {
        let (home, repo) = setup("lifecycle");
        let mut conn = open_rw(&home, &repo);
        create_record(&mut conn, &new_record("d1", "t1")).unwrap();
        let (rev2, _) = update_record(&mut conn, "d1", 1, &update_to("t1", "archived")).unwrap();
        assert_eq!(rev2, 2);
        assert_eq!(show_record(&conn, "d1", None).unwrap().status, "archived");
        let (rev3, _) = update_record(&mut conn, "d1", 2, &update_to("t1", "active")).unwrap();
        assert_eq!(rev3, 3);
        assert_eq!(show_record(&conn, "d1", None).unwrap().status, "active");
        assert_eq!(history(&conn, "d1", 50, 0, None).unwrap().total, 3);
        conn.close().unwrap();
    }

    #[test]
    fn knowledge_records_concurrent_writers_one_winner() {
        let (home, repo) = setup("race");
        let mut first = open_rw(&home, &repo);
        create_record(&mut first, &new_record("d1", "base zqxbase")).unwrap();
        let gen_before = read_generation(&first).unwrap();
        drop(first);
        // Two read-write connections observe the same head revision.
        let mut a = open_knowledge_at(&home, &repo, true).unwrap();
        let mut b = open_knowledge_at(&home, &repo, true).unwrap();
        assert_eq!(show_record(&a, "d1", None).unwrap().revision, 1);
        assert_eq!(show_record(&b, "d1", None).unwrap().revision, 1);
        let ra = update_record(&mut a, "d1", 1, &update_to("winner zqxwin", "active"));
        let rb = update_record(&mut b, "d1", 1, &update_to("loser zqxlose", "active"));
        // Exactly one writer wins; the loser reports a conflict.
        match (ra, rb) {
            (Ok(_), Err(e)) | (Err(e), Ok(_)) => {
                assert!(e.contains("conflict"), "loser must conflict: {e}")
            }
            (Ok(_), Ok(_)) => panic!("both writers won: lost update"),
            (Err(a), Err(b)) => panic!("both writers lost: {a} vs {b}"),
        }
        drop(a);
        drop(b);
        let check = open_knowledge_at(&home, &repo, false).unwrap();
        let head = show_record(&check, "d1", None).unwrap();
        assert_eq!(
            head.revision, 2,
            "no lost update: head advanced exactly once"
        );
        assert_eq!(history(&check, "d1", 50, 0, None).unwrap().total, 2);
        assert_eq!(
            read_generation(&check).unwrap(),
            gen_before + 1,
            "loser caused no generation bump"
        );
        check.close().unwrap();
    }

    #[test]
    fn knowledge_records_history_generation_guard_and_pagination() {
        let (home, repo) = setup("paging");
        let mut conn = open_rw(&home, &repo);
        create_record(&mut conn, &new_record("d1", "t1")).unwrap();
        update_record(&mut conn, "d1", 1, &update_to("t2", "active")).unwrap();
        update_record(&mut conn, "d1", 2, &update_to("t3", "active")).unwrap();
        let gen = read_generation(&conn).unwrap();
        let err = history(&conn, "d1", 50, 0, Some(gen + 1)).unwrap_err();
        assert!(err.contains("generation"), "unexpected: {err}");
        let p1 = history(&conn, "d1", 2, 0, Some(gen)).unwrap();
        assert_eq!(p1.total, 3);
        assert!(p1.truncated);
        assert_eq!(
            p1.revisions.iter().map(|r| r.revision).collect::<Vec<_>>(),
            vec![1, 2]
        );
        let p2 = history(&conn, "d1", 2, 2, Some(gen)).unwrap();
        assert!(!p2.truncated);
        assert_eq!(
            p2.revisions.iter().map(|r| r.revision).collect::<Vec<_>>(),
            vec![3]
        );
        let empty = history(&conn, "d1", 2, 3, Some(gen)).unwrap();
        assert_eq!(empty.total, 3);
        assert!(empty.revisions.is_empty());
        conn.close().unwrap();
    }

    #[test]
    fn knowledge_records_missing_id_errors() {
        let (home, repo) = setup("noid");
        let mut conn = open_rw(&home, &repo);
        assert!(show_record(&conn, "nope", None).is_err());
        assert!(show_record(&conn, "nope", Some(1)).is_err());
        assert!(history(&conn, "nope", 50, 0, None).is_err());
        assert!(update_record(&mut conn, "nope", 1, &update_to("t", "active")).is_err());
        conn.close().unwrap();
    }

    #[test]
    fn knowledge_records_write_revalidates_companions() {
        let (home, repo) = setup("revalidate");
        let mut conn = open_rw(&home, &repo);
        create_record(&mut conn, &new_record("d1", "t1")).unwrap();
        let gen_before = read_generation(&conn).unwrap();
        conn.close().unwrap();
        let db = knowledge_db_path(&home, "revalidate");
        // Open the writer on clean paths, then clear any companions SQLite
        // maintains itself: companion lifecycle across open is
        // platform-dependent, so planting must start from a verified-absent
        // state with no SQLite call between clearing and planting.
        // The -journal path is used because SQLite never holds it open while
        // this connection is idle (WAL mode never uses it; a rollback write
        // never starts since revalidation runs first).
        let mut conn = open_knowledge_at(&home, &repo, true).unwrap();
        for suffix in ["-wal", "-shm", "-journal"] {
            let _ = std::fs::remove_file(companion(&db, suffix));
        }
        let journal = companion(&db, "-journal");
        assert!(
            !journal.exists(),
            "companion must be absent before planting"
        );
        let outside = tmpdir("m7-outside-reval");
        let target = outside.join("evil-wal");
        std::fs::write(&target, b"SENTINEL").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &journal).unwrap();
        #[cfg(windows)]
        match plant_link(&journal, &target) {
            Some(mech) => eprintln!("revalidation link planted via {mech}"),
            None => {
                eprintln!("NOT TESTED: cannot plant links in this environment");
                return;
            }
        }
        let err = update_record(&mut conn, "d1", 1, &update_to("t2", "active")).unwrap_err();
        assert!(
            err.contains("link") || err.contains("escape"),
            "unexpected: {err}"
        );
        // Drop the poisoned connection without insisting on a clean close,
        // remove the planted link, and verify nothing changed through a fresh
        // connection: reads through the hostile companion are not trusted.
        // Junctions plant as directories on Windows, so removal falls back
        // from file to directory removal.
        let _ = conn.close();
        std::fs::remove_file(&journal)
            .or_else(|_| std::fs::remove_dir(&journal))
            .unwrap();
        let check = open_knowledge_at(&home, &repo, false).unwrap();
        assert_eq!(show_record(&check, "d1", None).unwrap().revision, 1);
        assert_eq!(read_generation(&check).unwrap(), gen_before);
        let hits: i64 = check
            .query_row(
                "SELECT count(*) FROM knowledge_fts WHERE knowledge_fts MATCH 't2'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(hits, 0, "refused update must not leak into the index");
        check.close().unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"SENTINEL");
    }

    #[test]
    fn knowledge_history_reads_single_snapshot() {
        use std::sync::{Arc, Barrier};
        let (home, repo) = setup("snapshot");
        let mut conn = open_rw(&home, &repo);
        create_record(&mut conn, &new_record("d1", "snap zqxone")).unwrap();
        conn.close().unwrap();
        // Writer commits exactly on the boundary between the reader's
        // generation validation and its row reads. No sleeps: barriers order
        // the two threads deterministically.
        // Both connections are opened sequentially up front: the barrier
        // orders the commit boundary deterministically, and no concurrent
        // SQLite open ever races WAL initialization.
        let barrier = Arc::new(Barrier::new(2));
        let mut writer_conn = open_knowledge_at(&home, &repo, true).unwrap();
        let parked = barrier.clone();
        let released = barrier.clone();
        let writer = std::thread::spawn(move || {
            parked.wait();
            update_record(
                &mut writer_conn,
                "d1",
                1,
                &update_to("writer zqxwtwo", "active"),
            )
            .unwrap();
            writer_conn.close().unwrap();
            released.wait();
        });
        let conn = open_knowledge_at(&home, &repo, false).unwrap();
        let page = history_with_probe(
            &conn,
            "d1",
            50,
            0,
            Some(1),
            Some(&|| {
                barrier.wait();
                barrier.wait();
            }),
        )
        .unwrap();
        writer.join().unwrap();
        // Generation, total and rows all reflect the pre-commit snapshot.
        assert_eq!(page.generation, 1);
        assert_eq!(page.total, 1);
        assert_eq!(
            page.revisions
                .iter()
                .map(|r| r.revision)
                .collect::<Vec<_>>(),
            vec![1]
        );
        // The concurrent commit itself is intact afterwards.
        assert_eq!(show_record(&conn, "d1", None).unwrap().revision, 2);
        conn.close().unwrap();
    }

    #[test]
    fn knowledge_integers_checked_no_panic_no_mutation() {
        let (home, repo) = setup("intbounds");
        let mut conn = open_rw(&home, &repo);
        create_record(&mut conn, &new_record("d1", "t1")).unwrap();
        // Unrepresentable revision request fails explicitly.
        let err = show_record(&conn, "d1", Some(u64::MAX)).unwrap_err();
        assert!(err.contains("range"), "unexpected: {err}");
        // Pagination beyond i64 fails explicitly.
        let err = history(&conn, "d1", usize::MAX, 0, None).unwrap_err();
        assert!(err.contains("limit"), "unexpected: {err}");
        let err = history(&conn, "d1", 50, usize::MAX, None).unwrap_err();
        assert!(err.contains("offset"), "unexpected: {err}");
        // A corrupt negative stored revision fails explicitly on reads.
        conn.execute("UPDATE heads SET current_revision = -1 WHERE id = 'd1'", [])
            .unwrap();
        let err = show_record(&conn, "d1", None).unwrap_err();
        assert!(err.contains("corrupt"), "unexpected: {err}");
        conn.execute(
            "UPDATE records SET revision = -1 WHERE id = 'd1' AND revision = 1",
            [],
        )
        .unwrap();
        let err = history(&conn, "d1", 50, 0, None).unwrap_err();
        assert!(err.contains("corrupt"), "unexpected: {err}");
        conn.close().unwrap();
        // Generation overflow fails explicitly with no mutation.
        let (home, repo) = setup("genoverflow");
        let mut conn = open_rw(&home, &repo);
        create_record(&mut conn, &new_record("d1", "t1")).unwrap();
        conn.execute(
            "UPDATE meta SET value = '18446744073709551615' WHERE key = 'knowledge_generation'",
            [],
        )
        .unwrap();
        let err = update_record(&mut conn, "d1", 1, &update_to("t2", "active")).unwrap_err();
        assert!(err.contains("overflow"), "unexpected: {err}");
        assert_eq!(show_record(&conn, "d1", None).unwrap().revision, 1);
        assert_eq!(history(&conn, "d1", 50, 0, None).unwrap().total, 1);
        conn.close().unwrap();
    }

    #[test]
    fn knowledge_records_parallel_writers_overlap() {
        use std::sync::{Arc, Barrier};
        let (home, repo) = setup("parrace");
        let mut first = open_rw(&home, &repo);
        create_record(&mut first, &new_record("d1", "base zqxpbase")).unwrap();
        let gen_before = read_generation(&first).unwrap();
        drop(first);
        // Both writers read head rev 1 first then rendezvous, so the two
        // expected-1 updates genuinely overlap instead of merely following
        // each other. Connections open sequentially up front so no concurrent
        // SQLite open ever races WAL initialization; only the updates overlap.
        let barrier = Arc::new(Barrier::new(3));
        let mut writers = Vec::new();
        for _ in ["zqxpwina", "zqxpwinb"] {
            writers.push(open_knowledge_at(&home, &repo, true).unwrap());
        }
        let mut handles = Vec::new();
        for (term, mut w) in ["zqxpwina", "zqxpwinb"].into_iter().zip(writers) {
            let b = barrier.clone();
            handles.push(std::thread::spawn(move || {
                assert_eq!(show_record(&w, "d1", None).unwrap().revision, 1);
                b.wait();
                let out = update_record(&mut w, "d1", 1, &update_to(term, "active"));
                w.close().unwrap();
                (term, out.map(|_| term))
            }));
        }
        barrier.wait();
        let mut winners = Vec::new();
        for h in handles {
            let (term, res) = h.join().unwrap();
            match res {
                Ok(_) => winners.push(term),
                Err(e) => assert!(e.contains("conflict"), "loser must conflict: {e}"),
            }
        }
        assert_eq!(winners.len(), 1, "exactly one overlapping writer wins");
        let check = open_knowledge_at(&home, &repo, false).unwrap();
        assert_eq!(show_record(&check, "d1", None).unwrap().revision, 2);
        assert_eq!(history(&check, "d1", 50, 0, None).unwrap().total, 2);
        assert_eq!(read_generation(&check).unwrap(), gen_before + 1);
        // FTS reflects the winner only.
        let winner_hits: i64 = check
            .query_row(
                "SELECT count(*) FROM knowledge_fts WHERE knowledge_fts MATCH 'zqxpwina' OR knowledge_fts MATCH 'zqxpwinb'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(winner_hits, 1, "index must reflect exactly the winner");
        check.close().unwrap();
    }

    #[test]
    fn knowledge_records_invalid_json_fails_atomically() {
        let (home, repo) = setup("jsonatomic");
        let conn = open_rw(&home, &repo);
        let bad_title = "t".repeat(513);
        let bad_payloads = [
            // Duplicate top-level key.
            r#"{"id":"d1","kind":"decision","title":"t","title":"t2","body":"b","status":"active","author":"r","evidence":[]}"#.to_string(),
            // Unknown field.
            r#"{"id":"d1","kind":"decision","title":"t","body":"b","status":"active","author":"r","evidence":[],"zzz":1}"#.to_string(),
            // Missing author.
            r#"{"id":"d1","kind":"decision","title":"t","body":"b","status":"active","evidence":[]}"#.to_string(),
            // Oversize title (513 bytes).
            format!(
                "{{\"id\":\"d1\",\"kind\":\"decision\",\"title\":\"{bad_title}\",\"body\":\"b\",\"status\":\"active\",\"author\":\"r\",\"evidence\":[]}}"
            ),
            // Benchmark without measurement.
            r#"{"id":"b1","kind":"benchmark","title":"t","body":"b","status":"active","author":"r","evidence":[]}"#.to_string(),
        ];
        for raw in &bad_payloads {
            assert!(
                validate_create_json(raw.as_bytes()).is_err(),
                "must fail: {raw}"
            );
        }
        // Nothing was written by any rejected input.
        assert!(show_record(&conn, "d1", None).is_err());
        assert!(show_record(&conn, "b1", None).is_err());
        assert_eq!(read_generation(&conn).unwrap(), 0);
        conn.close().unwrap();
    }

    #[test]
    fn knowledge_records_benchmark_roundtrip() {
        let (home, repo) = setup("benchrt");
        let mut conn = open_rw(&home, &repo);
        let raw = r#"{"id":"b1","kind":"benchmark","title":"speed zqxb","body":"faster","status":"active","author":"r","evidence":[],
            "measurement":{"method":"criterion","environment":"lab-pc","sample_count":100,
            "metrics":{"latency":{"value":1.25,"unit":"ms"},"throughput":{"value":800,"unit":"ops"}}}}"#;
        let input = validate_create_json(raw.as_bytes()).unwrap();
        let (rev, _) = create_validated(&mut conn, &input).unwrap();
        assert_eq!(rev, 1);
        let shown = show_record(&conn, "b1", None).unwrap();
        let measurement = shown
            .measurement
            .expect("benchmark show must carry measurement");
        assert_eq!(measurement.method, "criterion");
        assert_eq!(measurement.sample_count, 100);
        assert_eq!(measurement.metrics.len(), 2);
        assert!(measurement
            .metrics
            .iter()
            .any(|m| m.name == "latency" && m.unit == "ms"));
        // Update with a new measurement replaces it on the new revision only.
        let raw2 = r#"{"title":"speed zqxb2","body":"faster","status":"active","author":"r","evidence":[],
            "measurement":{"method":"criterion","environment":"lab-pc","sample_count":200,
            "metrics":{"latency":{"value":1.0,"unit":"ms"}}}}"#;
        let update = validate_update_json(raw2.as_bytes()).unwrap();
        update_validated(&mut conn, "b1", 1, &update).unwrap();
        let shown2 = show_record(&conn, "b1", None).unwrap();
        assert_eq!(shown2.revision, 2);
        assert_eq!(shown2.measurement.as_ref().unwrap().sample_count, 200);
        let shown1 = show_record(&conn, "b1", Some(1)).unwrap();
        assert_eq!(shown1.measurement.as_ref().unwrap().sample_count, 100);
        conn.close().unwrap();
    }

    #[test]
    fn knowledge_records_benchmark_update_carries_forward() {
        let (home, repo) = setup("benchcarry");
        let mut conn = open_rw(&home, &repo);
        let raw = r#"{"id":"b1","kind":"benchmark","title":"t","body":"b","status":"active","author":"r","evidence":[],
            "measurement":{"method":"m","environment":"e","sample_count":10,
            "metrics":{"m":{"value":1.0,"unit":"ms"}}}}"#;
        let input = validate_create_json(raw.as_bytes()).unwrap();
        create_validated(&mut conn, &input).unwrap();
        // Title-only update without measurement keeps the claim intact.
        let raw2 = r#"{"title":"t2","body":"b","status":"active","author":"r","evidence":[]}"#;
        let update = validate_update_json(raw2.as_bytes()).unwrap();
        update_validated(&mut conn, "b1", 1, &update).unwrap();
        let shown = show_record(&conn, "b1", None).unwrap();
        assert_eq!(shown.measurement.as_ref().unwrap().sample_count, 10);
        conn.close().unwrap();
    }

    #[test]
    fn knowledge_records_measurement_rejected_on_non_benchmark_update() {
        for kind in ["decision", "constraint"] {
            let (home, repo) = setup(&format!("nomeasure-{kind}"));
            let mut conn = open_rw(&home, &repo);
            let raw = format!(
                "{{\"id\":\"d1\",\"kind\":\"{kind}\",\"title\":\"t\",\"body\":\"b\",\"status\":\"active\",\"author\":\"r\",\"evidence\":[]}}"
            );
            let input = validate_create_json(raw.as_bytes()).unwrap();
            create_validated(&mut conn, &input).unwrap();
            let gen_before = read_generation(&conn).unwrap();
            // A fully valid measurement is still refused on non-benchmarks.
            let raw2 = r#"{"title":"t2","body":"b","status":"active","author":"r","evidence":[],
                "measurement":{"method":"m","environment":"e","sample_count":10,
                "metrics":{"m":{"value":1.0,"unit":"ms"}}}}"#;
            let update = validate_update_json(raw2.as_bytes()).unwrap();
            let err = update_validated(&mut conn, "d1", 1, &update).unwrap_err();
            assert!(err.contains("measurement"), "{kind}: unexpected: {err}");
            // No head, revision, FTS or generation change.
            assert_eq!(show_record(&conn, "d1", None).unwrap().revision, 1);
            assert_eq!(history(&conn, "d1", 50, 0, None).unwrap().total, 1);
            assert_eq!(read_generation(&conn).unwrap(), gen_before);
            let hits: i64 = conn
                .query_row(
                    "SELECT count(*) FROM knowledge_fts WHERE knowledge_fts MATCH 't2'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(
                hits, 0,
                "{kind}: refused update must not leak into the index"
            );
            conn.close().unwrap();
        }
    }

    #[test]
    fn knowledge_records_mid_write_fault_rolls_back() {
        // Create path: fault after the records insert.
        let (home, repo) = setup("faultcreate");
        let mut conn = open_rw(&home, &repo);
        let raw = r#"{"id":"d1","kind":"decision","title":"fault zqff","body":"b","status":"active","author":"r","evidence":[]}"#;
        let input = validate_create_json(raw.as_bytes()).unwrap();
        let err = create_validated_with_fault(&mut conn, &input).unwrap_err();
        assert!(err.contains("injected"), "unexpected: {err}");
        assert!(show_record(&conn, "d1", None).is_err());
        assert!(history(&conn, "d1", 50, 0, None).is_err());
        assert_eq!(read_generation(&conn).unwrap(), 0);
        let hits: i64 = conn
            .query_row(
                "SELECT count(*) FROM knowledge_fts WHERE knowledge_fts MATCH 'zqff'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(hits, 0);
        conn.close().unwrap();
        // Update path: fault after the records insert; head and index intact.
        let (home, repo) = setup("faultupdate");
        let mut conn = open_rw(&home, &repo);
        let raw = r#"{"id":"d1","kind":"decision","title":"base zqfb","body":"b","status":"active","author":"r","evidence":[]}"#;
        let input = validate_create_json(raw.as_bytes()).unwrap();
        create_validated(&mut conn, &input).unwrap();
        let gen_before = read_generation(&conn).unwrap();
        let raw2 =
            r#"{"title":"new zqfn","body":"b","status":"active","author":"r","evidence":[]}"#;
        let update = validate_update_json(raw2.as_bytes()).unwrap();
        let err = update_validated_with_fault(&mut conn, "d1", 1, &update).unwrap_err();
        assert!(err.contains("injected"), "unexpected: {err}");
        assert_eq!(show_record(&conn, "d1", None).unwrap().revision, 1);
        assert_eq!(read_generation(&conn).unwrap(), gen_before);
        let hits: i64 = conn
            .query_row(
                "SELECT count(*) FROM knowledge_fts WHERE knowledge_fts MATCH 'zqfn'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(hits, 0);
        conn.close().unwrap();
    }
}
