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

use rusqlite::{Connection, OpenFlags};

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
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
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
        Err(e) => Err(format!(
            "cannot inspect knowledge directory '{}': {e}",
            kdir.display()
        )),
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
const REQUIRED_TABLES: [&str; 7] = [
    "meta",
    "records",
    "heads",
    "evidence",
    "relations",
    "retractions",
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
        // The entry was retargeted to another tree before the first
        // knowledge store was created: the recorded string no longer matches
        // the live canonical root, so creation must fail.
        let (home, repo) = setup("retarget");
        let other_root = tmpdir("m7-retarget-root");
        let moved = RepoEntry {
            name: repo.name.clone(),
            root: other_root.to_string_lossy().into_owned(),
            added_unix: 1,
        };
        let err = open_knowledge_at(&home, &moved, true).unwrap_err();
        assert!(err.contains("retarget"), "unexpected: {err}");
        assert!(
            !knowledge_db_path(&home, "retarget").exists(),
            "refused creation must leave no store file behind"
        );
        // The normal compatible path still works: the recorded canonical
        // root creates fine, as does the no-index case (other tests).
        open_knowledge_at(&home, &repo, true)
            .unwrap()
            .close()
            .unwrap();
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
        let mut raw = std::env::temp_dir().into_os_string();
        raw.push(format!("/riltra-m7-nonutf-{}-", std::process::id()));
        raw.extend([0x66u8, 0x6f, 0x80, 0x6f]);
        let home = PathBuf::from(raw);
        let _ = std::fs::remove_dir_all(&home);
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
        let ok = std::process::Command::new("cmd")
            .args([
                "/C",
                "mklink",
                "/J",
                &link.to_string_lossy(),
                &anchor.to_string_lossy(),
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
}
