// SPDX-License-Identifier: Apache-2.0
//! G-BACK: consistent-snapshot backup, structural verification, and offline
//! restore of one knowledge store. Snapshots come from SQLite's `VACUUM INTO`
//! (a consistent copy; a plain file copy of a live WAL store is not one) and
//! are described by a manifest bound to the registered repository name and
//! canonical root. Verification is structural only (`PRAGMA integrity_check`,
//! version stamps, required tables, manifest counts): no digest, no
//! authenticity claim. Restore stages and verifies the snapshot next to the
//! target, fails closed if the target's companions exist (their committed
//! transactions cannot be proven safe to discard), and then replaces the
//! target with a single rename, so a failed restore never deletes the
//! original.

use std::io::Read;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};

use crate::json::escape;
use crate::knowledge_store;
use crate::registry::RepoEntry;

pub const BACKUP_FORMAT: &str = "riltra-knowledge-backup";
pub const BACKUP_VERSION: u64 = 1;

/// A manifest is a fixed-shape record; anything larger is not one of ours.
const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;

/// The two files of one backup: `<base>.db` and `<base>.manifest.json`.
pub struct BackupPaths {
    pub db: PathBuf,
    pub manifest: PathBuf,
}

fn with_suffix(base: &Path, suffix: &str) -> PathBuf {
    let mut os = base.as_os_str().to_os_string();
    os.push(suffix);
    PathBuf::from(os)
}

pub fn backup_paths(base: &Path) -> BackupPaths {
    BackupPaths {
        db: with_suffix(base, ".db"),
        manifest: with_suffix(base, ".manifest.json"),
    }
}

/// One backup's manifest: binds the snapshot to the registered repository
/// name and canonical root and records the schema version and entity counts
/// read from the snapshot itself.
#[derive(Debug, Clone, PartialEq)]
pub struct BackupManifest {
    pub repo: String,
    pub canonical_root: String,
    pub schema_version: u32,
    pub records: u64,
    pub relations: u64,
}

impl BackupManifest {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"format\":\"{}\",\"version\":{},\"repo\":\"{}\",\"canonical_root\":\"{}\",\
             \"schema_version\":{},\"records\":{},\"relations\":{}}}\n",
            BACKUP_FORMAT,
            BACKUP_VERSION,
            escape(&self.repo),
            escape(&self.canonical_root),
            self.schema_version,
            self.records,
            self.relations
        )
    }
}

/// Strict manifest parse: exact key set, no duplicate keys, exact types
/// (house strict-JSON rules).
pub fn parse_manifest(bytes: &[u8]) -> Result<BackupManifest, String> {
    let wrap = |e: String| format!("corrupt backup manifest: {e}");
    let fields = crate::strict_object(
        bytes,
        &[
            "format",
            "version",
            "repo",
            "canonical_root",
            "schema_version",
            "records",
            "relations",
        ],
    )
    .map_err(wrap)?;
    let format = crate::field_str(&fields, "format").map_err(wrap)?;
    if format != BACKUP_FORMAT {
        return Err(format!(
            "corrupt backup manifest: format '{format}' is not '{BACKUP_FORMAT}'"
        ));
    }
    let version = crate::field_u64(&fields, "version").map_err(wrap)?;
    if version != BACKUP_VERSION {
        return Err(format!("unsupported backup manifest version {version}"));
    }
    Ok(BackupManifest {
        repo: crate::field_str(&fields, "repo").map_err(wrap)?,
        canonical_root: crate::field_str(&fields, "canonical_root").map_err(wrap)?,
        schema_version: u32::try_from(crate::field_u64(&fields, "schema_version").map_err(wrap)?)
            .map_err(|_| {
            "corrupt backup manifest: schema version out of range".to_string()
        })?,
        records: crate::field_u64(&fields, "records").map_err(wrap)?,
        relations: crate::field_u64(&fields, "relations").map_err(wrap)?,
    })
}

/// Bounded read of a manifest file.
fn read_manifest(path: &Path) -> Result<Vec<u8>, String> {
    let mut file = std::fs::File::open(path)
        .map_err(|e| format!("backup manifest missing: '{}': {e}", path.display()))?;
    let mut bytes = Vec::new();
    (&mut file)
        .take(MAX_MANIFEST_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("cannot read backup manifest '{}': {e}", path.display()))?;
    if bytes.len() as u64 > MAX_MANIFEST_BYTES {
        return Err(format!(
            "corrupt backup manifest: larger than {MAX_MANIFEST_BYTES} bytes"
        ));
    }
    Ok(bytes)
}

/// Snapshot the live store through `conn` into `<base>.db` and write
/// `<base>.manifest.json`, returning the manifest and the snapshot's
/// generation. The manifest is derived from the snapshot, not from the live
/// store, so it can never describe a different generation. Either output
/// path already existing refuses the backup; any failure after the snapshot
/// exists removes the partial outputs.
pub fn backup_to(
    conn: &Connection,
    repo: &RepoEntry,
    croot: &Path,
    base: &Path,
) -> Result<(BackupManifest, u64), String> {
    let paths = backup_paths(base);
    for p in [&paths.db, &paths.manifest] {
        if std::fs::symlink_metadata(p).is_ok() {
            return Err(format!(
                "backup output path already exists: '{}'",
                p.display()
            ));
        }
    }
    let dest = paths
        .db
        .to_str()
        .ok_or_else(|| format!("backup path is not valid UTF-8: '{}'", paths.db.display()))?;
    conn.execute("VACUUM INTO ?1", [dest]).map_err(|e| {
        format!(
            "cannot create backup snapshot '{}': {e}",
            paths.db.display()
        )
    })?;
    let outcome: Result<(BackupManifest, u64), String> = (|| {
        let snap = Connection::open_with_flags(&paths.db, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| format!("cannot open backup snapshot '{}': {e}", paths.db.display()))?;
        let check = knowledge_store::verify_store(&snap, repo, croot)?;
        let manifest = BackupManifest {
            repo: repo.name.clone(),
            canonical_root: repo.root.clone(),
            schema_version: check.schema_version,
            records: check.records,
            relations: check.relations,
        };
        std::fs::write(&paths.manifest, manifest.to_json()).map_err(|e| {
            format!(
                "cannot write backup manifest '{}': {e}",
                paths.manifest.display()
            )
        })?;
        let generation = knowledge_store::read_generation(&snap)?;
        Ok((manifest, generation))
    })();
    if outcome.is_err() {
        let _ = std::fs::remove_file(&paths.db);
        let _ = std::fs::remove_file(&paths.manifest);
    }
    outcome
}

/// Outcome of a restore for the CLI envelope.
#[derive(Debug, PartialEq)]
pub struct RestoreReport {
    pub schema_version: u32,
    pub records: u64,
    pub relations: u64,
    pub generation: u64,
    pub overwrote: bool,
}

/// Restore `<base>.db` over the store of `repo`. Preconditions, all checked
/// before anything changes: manifest and snapshot present, manifest bound to
/// this repository name and canonical root, manifest schema version accepted,
/// and the target either absent or `overwrite` set. The snapshot is then
/// staged next to the target and fully verified (integrity, version stamps,
/// tables, binding, manifest counts) before anything changes. Restore fails
/// closed if the target's WAL/SHM/journal companions exist: a stale companion
/// may hold committed transactions of the store being replaced, and their
/// safety cannot be proven, so the restore refuses without modifying the
/// store or its companions. Otherwise a single rename replaces the target, so
/// every failure path leaves the original untouched.
pub fn restore_from(
    home: &Path,
    repo: &RepoEntry,
    croot: &Path,
    base: &Path,
    overwrite: bool,
) -> Result<RestoreReport, String> {
    let paths = backup_paths(base);
    let manifest = parse_manifest(&read_manifest(&paths.manifest)?)?;
    if manifest.repo != repo.name || manifest.canonical_root != croot.to_string_lossy().as_ref() {
        return Err(format!(
            "backup binding mismatch: manifest names '{}' at '{}', not '{}' at '{}'",
            manifest.repo,
            manifest.canonical_root,
            repo.name,
            croot.display()
        ));
    }
    if manifest.schema_version > knowledge_store::KNOWLEDGE_SCHEMA_VERSION {
        return Err(format!(
            "unsupported backup schema version {}",
            manifest.schema_version
        ));
    }
    let md = std::fs::metadata(&paths.db)
        .map_err(|e| format!("backup file missing: '{}': {e}", paths.db.display()))?;
    if !md.is_file() {
        return Err(format!(
            "backup file is not a regular file: '{}'",
            paths.db.display()
        ));
    }
    let (db, companions) = knowledge_store::restore_target_paths(home, repo)?;
    let overwrote = match std::fs::symlink_metadata(&db) {
        Ok(_) => {
            if !overwrite {
                return Err(format!(
                    "knowledge store already exists: '{}' (pass --overwrite to replace it)",
                    db.display()
                ));
            }
            true
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(e) => {
            return Err(format!(
                "cannot inspect knowledge store '{}': {e}",
                db.display()
            ))
        }
    };
    let staged = with_suffix(&db, &format!(".restore-tmp-{}", std::process::id()));
    // A stale stage from an earlier crash is ours to replace; removing a link
    // removes the link itself, never its target.
    let _ = std::fs::remove_file(&staged);
    std::fs::copy(&paths.db, &staged)
        .map_err(|e| format!("cannot stage backup '{}': {e}", paths.db.display()))?;
    let verified: Result<(knowledge_store::StoreCheck, u64), String> = (|| {
        let snap = Connection::open_with_flags(&staged, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| format!("corrupt backup: cannot open snapshot: {e}"))?;
        let check = knowledge_store::verify_store(&snap, repo, croot)
            .map_err(|e| format!("corrupt backup: {e}"))?;
        if check.schema_version != manifest.schema_version {
            return Err(format!(
                "corrupt backup: manifest declares schema version {}, snapshot has {}",
                manifest.schema_version, check.schema_version
            ));
        }
        if check.records != manifest.records || check.relations != manifest.relations {
            return Err(format!(
                "corrupt backup: manifest declares {} records and {} relations, snapshot has {} and {}",
                manifest.records, manifest.relations, check.records, check.relations
            ));
        }
        let generation =
            knowledge_store::read_generation(&snap).map_err(|e| format!("corrupt backup: {e}"))?;
        Ok((check, generation))
    })();
    let (check, generation) = match verified {
        Ok(v) => v,
        Err(e) => {
            let _ = std::fs::remove_file(&staged);
            return Err(e);
        }
    };
    // Fail-closed: refuse if any companion of the target store exists. A
    // stale WAL/SHM/journal may hold committed transactions of the store
    // being replaced; their safety cannot be proven, so the restore must
    // not proceed. Nothing is moved, deleted, or modified.
    for p in companions.iter() {
        if std::fs::symlink_metadata(p).is_ok() {
            let _ = std::fs::remove_file(&staged);
            return Err(format!(
                "knowledge store has live companion '{}': its committed \
                 transactions cannot be proven safe to discard; refusing \
                 to restore without modifying the store or companion",
                p.display()
            ));
        }
    }
    if let Err(e) = std::fs::rename(&staged, &db) {
        let _ = std::fs::remove_file(&staged);
        return Err(format!(
            "cannot replace knowledge store '{}': {e}",
            db.display()
        ));
    }
    Ok(RestoreReport {
        schema_version: check.schema_version,
        records: check.records,
        relations: check.relations,
        generation,
        overwrote,
    })
}
