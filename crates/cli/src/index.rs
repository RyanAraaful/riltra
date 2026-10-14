// SPDX-License-Identifier: Apache-2.0
//! M3 atomic indexing: full rebuilds from internal parser facts into a staging
//! database, promoted atomically; plus freshness evaluation with reasons.
//! Queries never trust a timestamp alone — every verdict names its evidence.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::git;
use crate::scan::{self, PlanAction};
use crate::store;
use crate::syntax::{grammar_version, language_name, parse_source};

pub const DEFAULT_SEARCH_LIMIT: usize = 50;
pub const MAX_SEARCH_LIMIT: usize = 1000;

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub struct IndexSummary {
    pub snapshot_id: i64,
    pub status_at_write: String,
    pub pre_fingerprint: String,
    pub post_fingerprint: String,
    pub files_discovered: usize,
    pub processed: usize,
    pub definitions: usize,
    pub imports: usize,
    pub calls: usize,
    pub errors: u32,
    pub skipped_ignored: usize,
    pub skipped_large: usize,
    pub skipped_binary: usize,
    pub skipped_symlink: usize,
    pub skipped_escape: usize,
    pub skipped_unreadable: usize,
    pub unsupported_total: usize,
    pub mode: String,
    pub reused: usize,
    pub reparsed: usize,
    pub added: usize,
    pub removed: usize,
    pub fallback_reason: String,
    pub timings_ms: PhaseMs,
}

/// Milliseconds per build phase, for auditable before/after comparison.
/// No telemetry: one small struct per `index` run, reported in its JSON.
pub struct PhaseMs {
    pub diff: u64,
    pub ingest: u64,
    pub verify: u64,
    pub promote: u64,
}

pub struct Freshness {
    pub status: String,
    pub reasons: Vec<String>,
    pub snapshot: Option<store::Snapshot>,
}

/// Hash of everything that influences scan output except file contents:
/// ignore rules, size policy, parser and grammar versions.
pub fn config_fingerprint(root: &Path) -> String {
    let (rules, info) = crate::ignore::load_rules(root);
    let material = format!(
        "rules={}|file={}|maxbytes={}|parser={}|grammars={}",
        rules.join("\n"),
        info.file_present,
        scan::MAX_FILE_BYTES,
        store::PARSER_VERSION,
        store::grammar_versions()
    );
    store::content_digest(material.as_bytes())
}

/// Point-in-time input description: git state plus config. Compared before
/// and after a build, and against the stored snapshot at query time.
pub fn input_fingerprint(root: &Path) -> String {
    let g = git::fingerprint(root);
    format!(
        "present={}|valid={}|head={}|branch={}|dirty={}|untracked={}|cfg={}",
        g.present,
        g.valid,
        g.head.as_deref().unwrap_or("-"),
        g.branch.as_deref().unwrap_or("-"),
        g.dirty,
        g.untracked,
        config_fingerprint(root)
    )
}

/// Staging setup shared by full and incremental builds: everything before
/// transaction creation. The 3-line transaction-open + snapshot insert stays
/// inline at each caller (a returned `Transaction` cannot borrow a returned
/// `Connection`).
struct StagingMeta {
    canon_root: PathBuf,
    git_state: git::GitInfo,
    cfg: String,
    pre: String,
}

fn prepare_staging(
    home: &Path,
    name: &str,
    root: &Path,
) -> Result<(rusqlite::Connection, PathBuf, PathBuf, StagingMeta), String> {
    let db = store::db_path(home, name);
    let staging = store::staging_path(home, name);
    if let Some(parent) = db.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| format!("cannot create index dir: {e}"))?;
        }
    }
    if staging.exists() && std::fs::remove_file(&staging).is_err() {
        return Err("stale staging file is locked by another build".to_string());
    }
    let canon_root = root
        .canonicalize()
        .map_err(|e| format!("cannot resolve root: {e}"))?;
    let git_state = git::fingerprint(&canon_root);
    let cfg = config_fingerprint(&canon_root);
    let pre = input_fingerprint(&canon_root);
    let conn = store::open_live(&staging)?;
    Ok((
        conn,
        db,
        staging,
        StagingMeta {
            canon_root,
            git_state,
            cfg,
            pre,
        },
    ))
}

/// Shared build epilogue, first half: source verification, freshness
/// verdict, snapshot finish, commit. Both pipelines end here, so neither
/// can drift apart.
fn finish_snapshot_tx(
    tx: rusqlite::Transaction<'_>,
    snapshot_id: i64,
    summary: &mut IndexSummary,
    meta: &StagingMeta,
) -> Result<(), String> {
    let t_verify = Instant::now();
    let post = input_fingerprint(&meta.canon_root);
    // Content evidence at write time: re-verify every stored digest against
    // current source before claiming fresh. A mid-build content flip that git
    // cannot see still fails here. `fresh` requires all three agreements.
    let content_verified =
        verify_snapshot_content(&tx, snapshot_id, &meta.canon_root).unwrap_or(false);
    let status = if summary.pre_fingerprint == post && content_verified {
        "fresh"
    } else {
        "stale"
    };
    store::finish_snapshot(&tx, snapshot_id, &post, status)?;
    tx.commit().map_err(|e| e.to_string())?;
    summary.timings_ms.verify = t_verify.elapsed().as_millis() as u64;
    summary.status_at_write = status.to_string();
    summary.post_fingerprint = post;
    Ok(())
}

/// Shared build epilogue, second half: atomic promotion plus staging
/// cleanup. Split from the first half so callers drop the staging
/// connection first — on Windows an open handle blocks file removal, which
/// left corrupt staging leftovers behind (regression-tested).
fn promote_live(db: &Path, staging: &Path) -> Result<u64, String> {
    let t_promote = Instant::now();
    let live = match store::open_live(db) {
        Ok(live) => live,
        Err(_) if !store::is_readable_db(db) => {
            // Corrupt live database with a complete staging build ready:
            // everything is re-derivable from source, so replace wholesale.
            std::fs::remove_file(db)
                .map_err(|e| format!("live index is corrupt and cannot be replaced: {e}"))?;
            store::open_live(db)?
        }
        Err(e) => return Err(e),
    };
    store::promote_staging(&live, staging)?;
    drop(live);
    let _ = std::fs::remove_file(staging);
    Ok(t_promote.elapsed().as_millis() as u64)
}

/// One fresh snapshot row from build metadata. Both pipelines insert the
/// same shape; only the surrounding flow differs.
fn new_snapshot_row(name: &str, meta: &StagingMeta) -> store::Snapshot {
    store::Snapshot {
        id: 0,
        repo: name.to_string(),
        root: meta.canon_root.to_string_lossy().to_string(),
        created_unix: now_unix(),
        head: meta.git_state.head.clone(),
        branch: meta.git_state.branch.clone(),
        dirty: meta.git_state.dirty,
        untracked: meta.git_state.untracked,
        parser_version: store::PARSER_VERSION,
        grammar_versions: store::grammar_versions(),
        config_fingerprint: meta.cfg.clone(),
        pre_fingerprint: meta.pre.clone(),
        post_fingerprint: String::new(),
        status_at_write: "stale".to_string(),
        nonce: store::new_nonce(),
    }
}

/// Full rebuild into staging, then atomic promotion. Facts stream from the
/// internal parser straight into one transaction — the whole repository is
/// never buffered, and readers never see a half-written snapshot.
/// Production routes through `build`; this stays as the M3-compatible entry
/// point and the test oracle for parity.
#[allow(dead_code)]
pub fn build_full(home: &Path, name: &str, root: &Path) -> Result<IndexSummary, String> {
    build_full_inner(home, name, root, "full", "none")
}

/// Shared full-rebuild body; `build` sets mode/reason for cold, forced, and
/// fallback invocations.
fn build_full_inner(
    home: &Path,
    name: &str,
    root: &Path,
    mode: &str,
    fallback_reason: &str,
) -> Result<IndexSummary, String> {
    let (mut conn, db, staging, meta) = prepare_staging(home, name, root)?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let snapshot_id = store::insert_snapshot(&tx, &new_snapshot_row(name, &meta))?;
    let t_diff = Instant::now();
    let (plan, _) = scan::plan_walk(&meta.canon_root)?;
    let diff_ms = t_diff.elapsed().as_millis() as u64;
    let mut summary = IndexSummary {
        snapshot_id,
        status_at_write: "stale".to_string(),
        pre_fingerprint: meta.pre.clone(),
        post_fingerprint: String::new(),
        files_discovered: 0,
        processed: 0,
        definitions: 0,
        imports: 0,
        calls: 0,
        errors: 0,
        skipped_ignored: 0,
        skipped_large: 0,
        skipped_binary: 0,
        skipped_symlink: 0,
        skipped_escape: 0,
        skipped_unreadable: 0,
        unsupported_total: 0,
        mode: mode.to_string(),
        reused: 0,
        reparsed: 0,
        added: 0,
        removed: 0,
        fallback_reason: fallback_reason.to_string(),
        timings_ms: PhaseMs {
            diff: diff_ms,
            ingest: 0,
            verify: 0,
            promote: 0,
        },
    };
    let t_ingest = Instant::now();
    for item in &plan {
        ingest_file(&tx, snapshot_id, item, &mut summary)?;
    }
    summary.timings_ms.ingest = t_ingest.elapsed().as_millis() as u64;
    summary.reparsed = summary.processed;
    finish_snapshot_tx(tx, snapshot_id, &mut summary, &meta)?;
    drop(conn);
    summary.timings_ms.promote = promote_live(&db, &staging)?;
    Ok(summary)
}

/// One file through the shared ingest path: skip counting, content gates,
/// parse, fact inserts. Used by full builds for every file and by
/// incremental builds for added/modified files only. Returns whether the
/// file was actually parsed (gate-skipped files cost a hash, not a parse).
fn ingest_file(
    conn: &rusqlite::Connection,
    snapshot_id: i64,
    item: &scan::PlannedFile,
    summary: &mut IndexSummary,
) -> Result<bool, String> {
    summary.files_discovered += 1;
    if let PlanAction::Skip(reason) = &item.action {
        match *reason {
            "ignored" => summary.skipped_ignored += 1,
            "symlink-not-followed" => summary.skipped_symlink += 1,
            _ => summary.skipped_unreadable += 1,
        }
        return Ok(false);
    }
    // Content gates before the extension verdict, mirroring scan render:
    // a huge or binary `.xyz` counts as skipped, not unsupported.
    let bytes = match std::fs::read(&item.full) {
        Ok(b) => b,
        Err(_) => {
            summary.skipped_unreadable += 1;
            return Ok(false);
        }
    };
    if bytes.len() as u64 > scan::MAX_FILE_BYTES {
        summary.skipped_large += 1;
        return Ok(false);
    }
    if bytes.contains(&0) || std::str::from_utf8(&bytes).is_err() {
        summary.skipped_binary += 1;
        return Ok(false);
    }
    match &item.action {
        PlanAction::Parse(lang) => {
            let facts = parse_source(*lang, &bytes);
            let file_id = store::insert_file(
                conn,
                snapshot_id,
                &store::FileRow {
                    path: item.rel.clone(),
                    language: language_name(*lang).to_string(),
                    bytes: bytes.len() as u64,
                    digest: store::content_digest(&bytes),
                    errors: facts.errors,
                },
            )?;
            let grammar = grammar_version(*lang).to_string();
            let language = language_name(*lang).to_string();
            for definition in &facts.definitions {
                store::insert_symbol(
                    conn,
                    snapshot_id,
                    &store::SymbolRow {
                        file_id,
                        kind: definition.kind.to_string(),
                        name: definition.name.clone(),
                        start_line: definition.span.start_line,
                        start_col: definition.span.start_col,
                        end_line: definition.span.end_line,
                        end_col: definition.span.end_col,
                        byte_start: definition.byte_start,
                        byte_end: definition.byte_end,
                        provider: "syntax".to_string(),
                        grammar: grammar.clone(),
                        language: language.clone(),
                    },
                )?;
            }
            for import in &facts.imports {
                store::insert_import(
                    conn,
                    snapshot_id,
                    &store::ImportRow {
                        file_id,
                        text: import.text.clone(),
                        start_line: import.span.start_line,
                        start_col: import.span.start_col,
                        end_line: import.span.end_line,
                        end_col: import.span.end_col,
                        byte_start: import.byte_start,
                        byte_end: import.byte_end,
                        provider: "syntax".to_string(),
                        grammar: grammar.clone(),
                        language: language.clone(),
                    },
                )?;
            }
            for call in &facts.calls {
                store::insert_call(
                    conn,
                    snapshot_id,
                    &store::CallRow {
                        file_id,
                        name: call.name.clone(),
                        start_line: call.span.start_line,
                        start_col: call.span.start_col,
                        end_line: call.span.end_line,
                        end_col: call.span.end_col,
                        byte_start: call.byte_start,
                        byte_end: call.byte_end,
                        unresolved: 1,
                        provider: "syntax".to_string(),
                        grammar: grammar.clone(),
                        language: language.clone(),
                    },
                )?;
            }
            summary.processed += 1;
            summary.definitions += facts.definitions.len();
            summary.imports += facts.imports.len();
            summary.calls += facts.calls.len();
            summary.errors += facts.errors;
            return Ok(true);
        }
        PlanAction::Unsupported(_) => summary.unsupported_total += 1,
        PlanAction::Skip(_) => {} // unreachable here; listed for exhaustiveness
    }
    Ok(false)
}

/// Per-file classification of a fresh walk against a stored snapshot:
/// owned paths only, no lifetime coupling to the plan.
struct FileDiff {
    /// (rel path, old file rowid) with byte-identical current content.
    unchanged: Vec<(String, i64)>,
    /// Rel paths to run through `ingest_file` (modified, added, and
    /// gate-skipped files alike — ingest counts skips exactly like full).
    /// Asserted by unit tests; production routes the single ordered walk by
    /// plan-minus-unchanged instead of a second set.
    #[allow(dead_code)]
    ingest_rels: Vec<String>,
    /// Stored paths with no current `Parse` item (deleted, now-ignored,
    /// now-unsupported): facts dropped, nothing ingested.
    removed: Vec<String>,
    /// Stored paths whose current item fails the content gates (now too
    /// large, binary, or unreadable): routed through ingest for skip
    /// counting, old facts dropped.
    dropped: Vec<String>,
}

/// Compare a fresh walk against stored (file id, digest) rows. Gate logic
/// mirrors `ingest_file` (as `content_diff` already does): only Parse items
/// passing every gate with a matching digest count as unchanged.
fn diff_files(
    stored: &BTreeMap<String, (i64, String)>,
    plan: &[scan::PlannedFile],
    _canon_root: &Path,
) -> FileDiff {
    let mut unchanged = Vec::new();
    let mut ingest_rels = Vec::new();
    let mut removed = Vec::new();
    let mut dropped = Vec::new();
    let mut seen_parse = BTreeSet::new();
    for item in plan {
        if !matches!(item.action, scan::PlanAction::Parse(_)) {
            continue;
        }
        seen_parse.insert(item.rel.clone());
        let bytes = match std::fs::read(&item.full) {
            Ok(b) => b,
            Err(_) => {
                ingest_rels.push(item.rel.clone());
                if stored.contains_key(&item.rel) {
                    dropped.push(item.rel.clone());
                }
                continue;
            }
        };
        if bytes.len() as u64 > scan::MAX_FILE_BYTES
            || bytes.contains(&0)
            || std::str::from_utf8(&bytes).is_err()
        {
            ingest_rels.push(item.rel.clone());
            if stored.contains_key(&item.rel) {
                dropped.push(item.rel.clone());
            }
            continue;
        }
        match stored.get(&item.rel) {
            Some((fid, digest)) if *digest == store::content_digest(&bytes) => {
                unchanged.push((item.rel.clone(), *fid));
            }
            _ => ingest_rels.push(item.rel.clone()),
        }
    }
    for path in stored.keys() {
        if !seen_parse.contains(path) {
            removed.push(path.clone());
        }
    }
    FileDiff {
        unchanged,
        ingest_rels,
        removed,
        dropped,
    }
}

/// Incremental rebuild: copy digest-verified files set-based from the old
/// snapshot, parse only the rest. `stored` and `old` must come from one
/// live read transaction (see `build`); the copy itself runs inside the
/// staging transaction over the ATTACHed live database, so all source reads
/// share that transaction's consistent view — one revision, never a mix.
/// A source row vanishing under us (concurrent replacement) copies zero
/// rows, and the file falls through to ingest from current source.
fn build_incremental(
    home: &Path,
    name: &str,
    root: &Path,
    old: &store::Snapshot,
    stored: &BTreeMap<String, (i64, String)>,
    live_db: &Path,
) -> Result<IndexSummary, String> {
    let (mut conn, db, staging, meta) = prepare_staging(home, name, root)?;
    let t_diff = Instant::now();
    let (plan, _) = scan::plan_walk(&meta.canon_root)?;
    let diff = diff_files(stored, &plan, &meta.canon_root);
    let diff_ms = t_diff.elapsed().as_millis() as u64;
    // ATTACH on the staging connection before its write transaction starts:
    // attaching inside an active write transaction reports the source as
    // locked. Reads still share the transaction's consistent view below.
    if store::attach_src(&conn, live_db).is_err() {
        // Live database unreadable mid-refresh with staging prepared: drop
        // everything and take the explicit full-rebuild recovery.
        drop(conn);
        let _ = std::fs::remove_file(&staging);
        return build_full_inner(home, name, root, "fallback-full", "db-unreadable");
    }
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let snapshot_id = store::insert_snapshot(&tx, &new_snapshot_row(name, &meta))?;
    let unchanged_ids: BTreeMap<&str, i64> = diff
        .unchanged
        .iter()
        .map(|(p, id)| (p.as_str(), *id))
        .collect();
    let mut summary = IndexSummary {
        snapshot_id,
        status_at_write: "stale".to_string(),
        pre_fingerprint: meta.pre.clone(),
        post_fingerprint: String::new(),
        files_discovered: 0,
        processed: 0,
        definitions: 0,
        imports: 0,
        calls: 0,
        errors: 0,
        skipped_ignored: 0,
        skipped_large: 0,
        skipped_binary: 0,
        skipped_symlink: 0,
        skipped_escape: 0,
        skipped_unreadable: 0,
        unsupported_total: 0,
        mode: "incremental".to_string(),
        reused: 0,
        reparsed: 0,
        added: 0,
        removed: diff.removed.len() + diff.dropped.len(),
        fallback_reason: "none".to_string(),
        timings_ms: PhaseMs {
            diff: diff_ms,
            ingest: 0,
            verify: 0,
            promote: 0,
        },
    };
    let t_ingest = Instant::now();
    for item in &plan {
        if matches!(item.action, scan::PlanAction::Parse(_)) {
            if let Some(old_fid) = unchanged_ids.get(item.rel.as_str()) {
                // `reparsed` counts files actually parsed; gate-skipped files
                // cost a hash, not a parse, so they never inflate it.
                summary.files_discovered += 1;
                match store::copy_file_facts(&tx, snapshot_id, old.id, *old_fid)? {
                    Some(copied) => {
                        summary.reused += 1;
                        summary.processed += 1;
                        summary.definitions += copied.symbols as usize;
                        summary.imports += copied.imports as usize;
                        summary.calls += copied.calls as usize;
                        summary.errors += copied.errors;
                    }
                    None => {
                        // Source row vanished under us: ingest from current
                        // source (ingest counts the file; undo our increment).
                        if ingest_file(&tx, snapshot_id, item, &mut summary)? {
                            summary.reparsed += 1;
                        }
                        summary.files_discovered -= 1;
                    }
                }
                continue;
            }
        }
        let was_stored = stored.contains_key(&item.rel);
        if ingest_file(&tx, snapshot_id, item, &mut summary)? {
            summary.reparsed += 1;
            if !was_stored {
                summary.added += 1;
            }
        }
    }
    summary.timings_ms.ingest = t_ingest.elapsed().as_millis() as u64;
    // Detach after commit: detaching inside the write transaction reports
    // the source locked, and the connection is dropped right below anyway.
    finish_snapshot_tx(tx, snapshot_id, &mut summary, &meta)?;
    store::detach_src(&conn)?;
    drop(conn);
    summary.timings_ms.promote = promote_live(&db, &staging)?;
    Ok(summary)
}

/// Full rebuild with an explicit mode and fallback reason. `build_full`
/// stays the plain M3 entry point (`"full"` / `"none"`).
fn fallback_full(
    home: &Path,
    name: &str,
    root: &Path,
    reason: &str,
) -> Result<IndexSummary, String> {
    build_full_inner(home, name, root, "fallback-full", reason)
}

/// Dispatcher: cold start and `--full` go straight to full rebuilds;
/// otherwise reuse is attempted only when the previous snapshot is provably
/// compatible (root, parser, grammars, config, HEAD, branch). Dirty
/// worktrees and untracked-count changes with the same HEAD/branch stay
/// incremental — per-file digests decide. A stale previous snapshot still
/// allows reuse: every reused file is independently digest-verified.
pub fn build(home: &Path, name: &str, root: &Path, full: bool) -> Result<IndexSummary, String> {
    if full {
        return build_full_inner(home, name, root, "full", "forced-full");
    }
    let db = store::db_path(home, name);
    if !db.exists() {
        return build_full_inner(home, name, root, "full", "cold-start");
    }
    // Classification inputs (snapshot row + id map) come from one live read
    // transaction: mutually consistent by construction.
    let mut live = match store::open_live(&db) {
        Ok(conn) => conn,
        Err(_) => return fallback_full(home, name, root, "db-unreadable"),
    };
    let (old, stored) = {
        let read_tx = live.transaction().map_err(|e| e.to_string())?;
        let old = match store::latest_snapshot(&read_tx)? {
            Some(s) => s,
            None => return fallback_full(home, name, root, "no-snapshot"),
        };
        let stored = store::file_id_map(&read_tx, old.id)?;
        (old, stored)
    };
    // Classification inputs are owned from here on; drop the live handle so
    // the staging transaction below is the only reader of the live file.
    drop(live);
    let canon_root = root
        .canonicalize()
        .map_err(|e| format!("cannot resolve root: {e}"))?;
    if canon_root.to_string_lossy() != old.root {
        return fallback_full(home, name, root, "root-moved");
    }
    if old.parser_version != store::PARSER_VERSION {
        return fallback_full(home, name, root, "parser-changed");
    }
    if old.grammar_versions != store::grammar_versions() {
        return fallback_full(home, name, root, "grammar-changed");
    }
    if old.config_fingerprint != config_fingerprint(&canon_root) {
        return fallback_full(home, name, root, "config-changed");
    }
    let live_git = git::fingerprint(&canon_root);
    if live_git.head != old.head {
        return fallback_full(home, name, root, "head-changed");
    }
    if live_git.branch != old.branch {
        return fallback_full(home, name, root, "branch-changed");
    }
    build_incremental(home, name, root, &old, &stored, &db)
}

/// Re-verify every stored file digest of a snapshot against current source.
/// Uses exactly the comparison the query-time evaluator runs, so build
/// admission and verification can never disagree: legitimately skipped files
/// (too large, binary, unreadable-and-absent) confirm nothing but incriminate
/// nothing, while added, removed, changed, or newly ineligible files fail.
/// There is no vacuous `true` for empty snapshots: a source appearing
/// afterwards reports `file-added`.
pub fn verify_snapshot_content(
    conn: &rusqlite::Connection,
    snapshot_id: i64,
    canon_root: &Path,
) -> Result<bool, String> {
    Ok(content_diff(conn, snapshot_id, canon_root)?.is_empty())
}

/// Compare stored file digests against a fresh content walk. Only parsed
/// files are stored, so only they are compared; ignore changes surface via
/// the config fingerprint instead.
fn content_diff(
    conn: &rusqlite::Connection,
    snapshot_id: i64,
    canon_root: &Path,
) -> Result<Vec<String>, String> {
    let stored: BTreeMap<String, String> = store::files_of(conn, snapshot_id)?
        .into_iter()
        .map(|f| (f.path, f.digest))
        .collect();
    let (plan, _) = scan::plan_walk(canon_root)?;
    let mut reasons = Vec::new();
    let mut seen = BTreeMap::new();
    for item in &plan {
        if !matches!(item.action, scan::PlanAction::Parse(_)) {
            continue;
        }
        // Mirror the build's skip decisions exactly: files the build
        // legitimately skipped (too large, binary, unreadable-and-absent)
        // must not incriminate a snapshot as changed.
        if std::fs::metadata(&item.full)
            .map(|m| m.len() > scan::MAX_FILE_BYTES)
            .unwrap_or(false)
        {
            if stored.contains_key(&item.rel) && !reasons.iter().any(|r| r == "content-changed") {
                reasons.push("content-changed".to_string());
            }
            continue;
        }
        let bytes = match std::fs::read(&item.full) {
            Ok(b) => b,
            Err(_) => {
                if stored.contains_key(&item.rel) {
                    if !item.full.exists() {
                        if !reasons.iter().any(|r| r == "file-removed") {
                            reasons.push("file-removed".to_string());
                        }
                    } else if !reasons.iter().any(|r| r == "content-changed") {
                        reasons.push("content-changed".to_string());
                    }
                }
                continue;
            }
        };
        if bytes.len() as u64 > scan::MAX_FILE_BYTES {
            if stored.contains_key(&item.rel) && !reasons.iter().any(|r| r == "content-changed") {
                reasons.push("content-changed".to_string());
            }
            continue;
        }
        if bytes.contains(&0) || std::str::from_utf8(&bytes).is_err() {
            if stored.contains_key(&item.rel) && !reasons.iter().any(|r| r == "content-changed") {
                reasons.push("content-changed".to_string());
            }
            continue;
        }
        seen.insert(item.rel.clone(), store::content_digest(&bytes));
    }
    for (path, digest) in &seen {
        match stored.get(path) {
            None => {
                if !reasons.iter().any(|r| r == "file-added") {
                    reasons.push("file-added".to_string());
                }
            }
            Some(old) if old != digest && !reasons.iter().any(|r| r == "content-changed") => {
                reasons.push("content-changed".to_string());
            }
            _ => {}
        }
    }
    for path in stored.keys() {
        if !seen.contains_key(path) && !reasons.iter().any(|r| r == "file-removed") {
            reasons.push("file-removed".to_string());
        }
    }
    Ok(reasons)
}

/// Evaluate freshness against live repository state. Every verdict carries
/// reasons; `fresh` is only returned when nothing contradicts it.
pub fn evaluate(home: &Path, name: &str, root: &Path) -> Freshness {
    let db = store::db_path(home, name);
    if !db.exists() {
        return Freshness {
            status: "unknown".to_string(),
            reasons: vec!["no-index".to_string()],
            snapshot: None,
        };
    }
    let mut conn = match store::open_live(&db) {
        Ok(c) => c,
        Err(_) => {
            return Freshness {
                status: "unknown".to_string(),
                reasons: vec!["db-unreadable".to_string()],
                snapshot: None,
            };
        }
    };
    // One read transaction across snapshot read and verdict inputs: the
    // verdict always describes exactly the revision it names.
    let tx = match conn.transaction() {
        Ok(tx) => tx,
        Err(_) => {
            return Freshness {
                status: "unknown".to_string(),
                reasons: vec!["db-unreadable".to_string()],
                snapshot: None,
            };
        }
    };
    let snapshot = match store::latest_snapshot(&tx) {
        Ok(Some(s)) => s,
        _ => {
            return Freshness {
                status: "unknown".to_string(),
                reasons: vec!["no-snapshot".to_string()],
                snapshot: None,
            };
        }
    };
    let canon_root = match root.canonicalize() {
        Ok(c) => c,
        Err(_) => {
            return Freshness {
                status: "unknown".to_string(),
                reasons: vec!["root-unreadable".to_string()],
                snapshot: Some(snapshot),
            };
        }
    };
    let (status, reasons) = evaluate_snapshot(&tx, name, &canon_root, &snapshot);
    let _ = tx.commit();
    Freshness {
        status,
        reasons,
        snapshot: Some(snapshot),
    }
}

/// Compare one snapshot row against live state on an already-open
/// connection. Callers hold a read transaction across this plus their fact
/// reads so verdict and results always describe one revision.
pub fn evaluate_snapshot(
    conn: &rusqlite::Connection,
    name: &str,
    canon_root: &Path,
    snapshot: &store::Snapshot,
) -> (String, Vec<String>) {
    let _ = name;
    let mut reasons = Vec::new();
    if canon_root.to_string_lossy() != snapshot.root {
        reasons.push("root-moved".to_string());
    }
    let live = git::fingerprint(canon_root);
    if live.dirty || snapshot.dirty {
        reasons.push("dirty".to_string());
    }
    if snapshot.pre_fingerprint != snapshot.post_fingerprint {
        reasons.push("changed-during-index".to_string());
    }
    if live.head != snapshot.head {
        reasons.push("head-changed".to_string());
    }
    if live.branch != snapshot.branch {
        reasons.push("branch-changed".to_string());
    }
    if snapshot.parser_version != store::PARSER_VERSION {
        reasons.push("parser-changed".to_string());
    }
    if snapshot.grammar_versions != store::grammar_versions() {
        reasons.push("grammar-changed".to_string());
    }
    if snapshot.config_fingerprint != config_fingerprint(canon_root) {
        reasons.push("config-changed".to_string());
    }
    match content_diff(conn, snapshot.id, canon_root) {
        Ok(mut diff) => reasons.append(&mut diff),
        Err(_) => reasons.push("content-unverifiable".to_string()),
    }
    let status = if reasons.is_empty() { "fresh" } else { "stale" };
    (status.to_string(), reasons)
}

#[cfg(test)]
mod tests {
    use super::{build, build_full, config_fingerprint, diff_files, evaluate, input_fingerprint};
    use crate::store;
    use crate::testutil::{git_repo, tmpdir};

    fn plain_repo(tag: &str) -> std::path::PathBuf {
        let dir = tmpdir(tag);
        std::fs::write(dir.join("a.py"), b"def f():\n    return g()\n").unwrap();
        dir
    }

    #[test]
    fn skipped_files_do_not_self_invalidate_freshness() {
        // Large, binary, and unsupported files are legitimately absent from
        // the snapshot; evaluate must mirror the build's skip decisions
        // instead of crying file-added (MMSP glew.h regression).
        let home = tmpdir("idx-skips-home");
        let repo = tmpdir("idx-skips");
        std::fs::write(repo.join("a.py"), b"def f():\n    pass\n").unwrap();
        std::fs::write(repo.join("big.py"), vec![b'x'; 1024 * 1024 + 1]).unwrap();
        std::fs::write(repo.join("bin.py"), b"\x00\x01").unwrap();
        std::fs::write(repo.join("note.xyz"), b"plain\n").unwrap();
        let summary = build_full(&home, "r", &repo).unwrap();
        assert_eq!(summary.processed, 1);
        let fresh = evaluate(&home, "r", &repo);
        assert_eq!(
            fresh.status, "fresh",
            "skips must not self-invalidate: {:?}",
            fresh.reasons
        );
    }

    #[test]
    fn noop_refresh_reparses_zero_files() {
        let home = tmpdir("idx-noop-home");
        let repo = plain_repo("idx-noop");
        let first = build_full(&home, "r", &repo).unwrap();
        let second = build(&home, "r", &repo, false).unwrap();
        assert_eq!(second.mode, "incremental");
        assert_eq!(second.reparsed, 0);
        assert_eq!(second.added, 0);
        assert_eq!(second.removed, 0);
        assert_eq!(second.reused, first.processed);
        assert_eq!(second.processed, first.processed);
        assert_eq!(second.status_at_write, "fresh");
        assert_eq!(second.fallback_reason, "none");
    }

    #[test]
    fn one_file_delta_reparses_only_that_file() {
        let home = tmpdir("idx-delta-home");
        let repo = tmpdir("idx-delta");
        std::fs::write(repo.join("a.py"), b"def f():\n    pass\n").unwrap();
        std::fs::write(repo.join("b.py"), b"def g():\n    pass\n").unwrap();
        build_full(&home, "r", &repo).unwrap();
        std::fs::write(repo.join("b.py"), b"def g():\n    return 1\n").unwrap();
        let second = build(&home, "r", &repo, false).unwrap();
        assert_eq!(second.mode, "incremental");
        assert_eq!(second.reparsed, 1);
        assert_eq!(second.reused, 1);
        assert_eq!(second.status_at_write, "fresh");
        let db = store::db_path(&home, "r");
        let conn = store::open_live(&db).unwrap();
        let snap = store::latest_snapshot(&conn).unwrap().unwrap();
        let files = store::files_of(&conn, snap.id).unwrap();
        assert_eq!(files.len(), 2);
        let (total, _) = store::search_symbols(&conn, snap.id, "Worker", 10, 0).unwrap();
        assert_eq!(total, 0);
    }

    #[test]
    fn add_remove_rename_yield_exact_facts() {
        let home = tmpdir("idx-arr-home");
        let repo = tmpdir("idx-arr");
        std::fs::write(repo.join("a.py"), b"def alpha():\n    pass\n").unwrap();
        std::fs::write(repo.join("b.py"), b"def beta():\n    pass\n").unwrap();
        build_full(&home, "r", &repo).unwrap();
        std::fs::remove_file(repo.join("a.py")).unwrap();
        std::fs::rename(repo.join("b.py"), repo.join("c.py")).unwrap();
        std::fs::write(repo.join("d.py"), b"def delta():\n    pass\n").unwrap();
        let second = build(&home, "r", &repo, false).unwrap();
        assert_eq!(second.removed, 2, "a.py deleted, b.py renamed away");
        assert_eq!(second.status_at_write, "fresh");
        let db = store::db_path(&home, "r");
        let conn = store::open_live(&db).unwrap();
        let snap = store::latest_snapshot(&conn).unwrap().unwrap();
        let files = store::files_of(&conn, snap.id).unwrap();
        let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, vec!["c.py", "d.py"]);
        let (total, _) = store::search_symbols(&conn, snap.id, "alpha", 10, 0).unwrap();
        assert_eq!(total, 0, "deleted facts must not survive");
        let (total, _) = store::search_symbols(&conn, snap.id, "beta", 10, 0).unwrap();
        assert_eq!(total, 1, "renamed facts move with the path");
    }

    #[test]
    fn repeated_changes_do_not_duplicate_facts() {
        let home = tmpdir("idx-dup-home");
        let repo = plain_repo("idx-dup");
        build_full(&home, "r", &repo).unwrap();
        for i in 0..3 {
            std::fs::write(repo.join("a.py"), format!("def f{i}():\n    pass\n")).unwrap();
            build(&home, "r", &repo, false).unwrap();
        }
        let db = store::db_path(&home, "r");
        let conn = store::open_live(&db).unwrap();
        let snap = store::latest_snapshot(&conn).unwrap().unwrap();
        assert_eq!(store::files_of(&conn, snap.id).unwrap().len(), 1);
        let (total, hits) = store::search_symbols(&conn, snap.id, "f", 10, 0).unwrap();
        assert_eq!(total, 1, "exactly one current definition: {hits:?}");
        assert_eq!(hits[0].name.as_deref(), Some("f2"));
    }

    #[test]
    fn empty_repo_stays_fresh_and_empty_file_is_eligible() {
        let home = tmpdir("idx-empty-home");
        let repo = tmpdir("idx-empty");
        let first = build_full(&home, "r", &repo).unwrap();
        assert_eq!(first.status_at_write, "fresh");
        assert_eq!(first.processed, 0);
        std::fs::write(repo.join("a.py"), b"").unwrap();
        let second = build(&home, "r", &repo, false).unwrap();
        assert_eq!(second.processed, 1, "zero-byte files are eligible");
        assert_eq!(second.status_at_write, "fresh");
    }

    #[test]
    fn incremental_matches_independent_full_rebuild() {
        // Guard test (Task 5): PASS-first-run expected. Any FAIL stops the
        // line — core parity broken, never loosen normalization to hide it.
        use std::collections::BTreeMap;
        type Span9 = (u32, u32, u32, u32, usize, usize);
        type SymRow = (
            String,
            String,
            Option<String>,
            Span9,
            String,
            String,
            String,
        );
        type ImpRow = (String, Option<String>, Span9, String, String, String);
        type CallRow = (String, Option<String>, Span9, i64, String, String, String);
        #[derive(Debug, PartialEq, PartialOrd, Ord, Eq)]
        struct Facts {
            files: Vec<(String, String, u64, String, u32)>,
            symbols: Vec<SymRow>,
            imports: Vec<ImpRow>,
            calls: Vec<CallRow>,
        }
        fn dump(conn: &rusqlite::Connection, snap_id: i64) -> Facts {
            let files = store::files_of(conn, snap_id).unwrap();
            let mut id_map = BTreeMap::new();
            let map = store::file_id_map(conn, snap_id).unwrap();
            for (path, (id, _)) in &map {
                id_map.insert(*id, path.clone());
            }
            let path_of = |id: i64| id_map.get(&id).cloned().unwrap_or_default();
            let mut symbols = Vec::new();
            {
                let mut st = conn
                    .prepare("SELECT file_id, kind, name, start_line, start_col, end_line, end_col, byte_start, byte_end, provider, grammar, language FROM symbols WHERE snapshot_id = ?1")
                    .unwrap();
                let rows = st
                    .query_map([snap_id], |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, Option<String>>(2)?,
                            row.get::<_, i64>(3)? as u32,
                            row.get::<_, i64>(4)? as u32,
                            row.get::<_, i64>(5)? as u32,
                            row.get::<_, i64>(6)? as u32,
                            row.get::<_, i64>(7)? as usize,
                            row.get::<_, i64>(8)? as usize,
                            row.get::<_, String>(9)?,
                            row.get::<_, String>(10)?,
                            row.get::<_, String>(11)?,
                        ))
                    })
                    .unwrap();
                for r in rows {
                    let r = r.unwrap();
                    symbols.push((
                        path_of(r.0),
                        r.1,
                        r.2,
                        (r.3, r.4, r.5, r.6, r.7, r.8),
                        r.9,
                        r.10,
                        r.11,
                    ));
                }
            }
            let mut imports = Vec::new();
            {
                let mut st = conn
                    .prepare("SELECT file_id, text, start_line, start_col, end_line, end_col, byte_start, byte_end, provider, grammar, language FROM imports WHERE snapshot_id = ?1")
                    .unwrap();
                let rows = st
                    .query_map([snap_id], |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, Option<String>>(1)?,
                            row.get::<_, i64>(2)? as u32,
                            row.get::<_, i64>(3)? as u32,
                            row.get::<_, i64>(4)? as u32,
                            row.get::<_, i64>(5)? as u32,
                            row.get::<_, i64>(6)? as usize,
                            row.get::<_, i64>(7)? as usize,
                            row.get::<_, String>(8)?,
                            row.get::<_, String>(9)?,
                            row.get::<_, String>(10)?,
                        ))
                    })
                    .unwrap();
                for r in rows {
                    let r = r.unwrap();
                    imports.push((
                        path_of(r.0),
                        r.1,
                        (r.2, r.3, r.4, r.5, r.6, r.7),
                        r.8,
                        r.9,
                        r.10,
                    ));
                }
            }
            let mut calls = Vec::new();
            {
                let mut st = conn
                    .prepare("SELECT file_id, name, start_line, start_col, end_line, end_col, byte_start, byte_end, unresolved, provider, grammar, language FROM calls WHERE snapshot_id = ?1")
                    .unwrap();
                let rows = st
                    .query_map([snap_id], |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, Option<String>>(1)?,
                            row.get::<_, i64>(2)? as u32,
                            row.get::<_, i64>(3)? as u32,
                            row.get::<_, i64>(4)? as u32,
                            row.get::<_, i64>(5)? as u32,
                            row.get::<_, i64>(6)? as usize,
                            row.get::<_, i64>(7)? as usize,
                            row.get::<_, i64>(8)?,
                            row.get::<_, String>(9)?,
                            row.get::<_, String>(10)?,
                            row.get::<_, String>(11)?,
                        ))
                    })
                    .unwrap();
                for r in rows {
                    let r = r.unwrap();
                    calls.push((
                        path_of(r.0),
                        r.1,
                        (r.2, r.3, r.4, r.5, r.6, r.7),
                        r.8,
                        r.9,
                        r.10,
                        r.11,
                    ));
                }
            }
            let mut file_vec: Vec<(String, String, u64, String, u32)> = files
                .into_iter()
                .map(|f| (f.path, f.language, f.bytes, f.digest, f.errors))
                .collect();
            file_vec.sort();
            symbols.sort();
            imports.sort();
            calls.sort();
            Facts {
                files: file_vec,
                symbols,
                imports,
                calls,
            }
        }
        let home_b = tmpdir("idx-par-b-home");
        let home_c = tmpdir("idx-par-c-home");
        let repo = tmpdir("idx-par");
        std::fs::write(repo.join("a.py"), b"def alpha():\n    pass\n").unwrap();
        std::fs::write(
            repo.join("b.py"),
            b"import os\ndef beta():\n    return os.name\n",
        )
        .unwrap();
        std::fs::write(repo.join("c.js"), b"function gamma() { return 1; }\n").unwrap();
        std::fs::write(repo.join("bin.py"), b"\x00\x01").unwrap();
        std::fs::write(repo.join("note.xyz"), b"plain\n").unwrap();
        build_full(&home_b, "r", &repo).unwrap();
        // Stage 1: modify. Stage 2: add + delete. Stage 3: rename.
        std::fs::write(
            repo.join("b.py"),
            b"import sys\ndef beta():\n    return sys.argv\n",
        )
        .unwrap();
        build(&home_b, "r", &repo, false).unwrap();
        std::fs::write(repo.join("d.py"), b"def delta():\n    pass\n").unwrap();
        std::fs::remove_file(repo.join("c.js")).unwrap();
        build(&home_b, "r", &repo, false).unwrap();
        std::fs::rename(repo.join("a.py"), repo.join("z.py")).unwrap();
        let inc = build(&home_b, "r", &repo, false).unwrap();
        assert_eq!(inc.status_at_write, "fresh");
        build_full(&home_c, "r", &repo).unwrap();
        let read = |home: &std::path::Path| {
            let db = store::db_path(home, "r");
            let conn = store::open_live(&db).unwrap();
            let snap = store::latest_snapshot(&conn).unwrap().unwrap();
            dump(&conn, snap.id)
        };
        assert_eq!(read(&home_b), read(&home_c), "incremental must equal full");
    }

    #[test]
    fn reuse_window_mutation_is_caught() {
        use super::verify_snapshot_content;
        let home = tmpdir("idx-rwindow-home");
        let repo = plain_repo("idx-rwindow");
        build_full(&home, "r", &repo).unwrap();
        let db = store::db_path(&home, "r");
        let conn = store::open_live(&db).unwrap();
        let snap = store::latest_snapshot(&conn).unwrap().unwrap();
        let canon = repo.canonicalize().unwrap();
        let stored = store::file_id_map(&conn, snap.id).unwrap();
        let (plan, _) = crate::scan::plan_walk(&canon).unwrap();
        let diff = diff_files(&stored, &plan, &canon);
        assert_eq!(diff.unchanged.len(), 1);
        assert!(
            diff.ingest_rels.is_empty(),
            "clean tree diffs to all-unchanged"
        );
        // Mutate inside the window between the reuse read and final verification.
        std::fs::write(repo.join("a.py"), b"def f():\n    return h()\n").unwrap();
        assert!(!verify_snapshot_content(&conn, snap.id, &canon).unwrap());
        let (plan2, _) = crate::scan::plan_walk(&canon).unwrap();
        let diff2 = diff_files(&stored, &plan2, &canon);
        assert_eq!(diff2.ingest_rels, vec!["a.py".to_string()]);
    }

    #[test]
    fn skipped_files_do_not_break_write_time_fresh() {
        use super::verify_snapshot_content;
        // One indexed file plus supported-extension files skipped since the
        // start (too large / binary): with no changes and all indexed files
        // matching, the build must still claim fresh.
        let home = tmpdir("idx-wverify-home");
        let repo = tmpdir("idx-wverify");
        std::fs::write(repo.join("a.py"), b"def f():\n    pass\n").unwrap();
        std::fs::write(repo.join("big.py"), vec![b'x'; 1024 * 1024 + 1]).unwrap();
        std::fs::write(repo.join("bin.py"), b"\x00\x01").unwrap();
        let summary = build_full(&home, "r", &repo).unwrap();
        assert_eq!(summary.processed, 1);
        assert_eq!(
            summary.status_at_write, "fresh",
            "longstanding skips must not break write-time verification"
        );
        let db = store::db_path(&home, "r");
        let conn = store::open_live(&db).unwrap();
        let snap = store::latest_snapshot(&conn).unwrap().unwrap();
        let canon = repo.canonicalize().unwrap();
        assert!(verify_snapshot_content(&conn, snap.id, &canon).unwrap());
        // A previously indexed file turning ineligible must still be caught.
        std::fs::write(repo.join("a.py"), vec![b'y'; 1024 * 1024 + 1]).unwrap();
        assert!(
            !verify_snapshot_content(&conn, snap.id, &canon).unwrap(),
            "indexed-then-ineligible must fail verification"
        );
    }

    #[test]
    fn empty_snapshot_does_not_verify_new_sources() {
        use super::verify_snapshot_content;
        // A snapshot that stored nothing must not verify new sources as
        // matching: the empty check is not evidence of agreement.
        let home = tmpdir("idx-vempty-home");
        let repo = tmpdir("idx-vempty");
        std::fs::write(repo.join("note.xyz"), b"plain\n").unwrap();
        build_full(&home, "r", &repo).unwrap();
        let db = store::db_path(&home, "r");
        let conn = store::open_live(&db).unwrap();
        let snap = store::latest_snapshot(&conn).unwrap().unwrap();
        let canon = repo.canonicalize().unwrap();
        assert!(verify_snapshot_content(&conn, snap.id, &canon).unwrap());
        std::fs::write(repo.join("a.py"), b"def f():\n    pass\n").unwrap();
        assert!(
            !verify_snapshot_content(&conn, snap.id, &canon).unwrap(),
            "a source appearing after an empty snapshot is file-added, not verified"
        );
    }

    #[test]
    fn verify_snapshot_content_detects_drift() {
        use super::verify_snapshot_content;
        let home = tmpdir("idx-verify-home");
        let repo = plain_repo("idx-verify");
        build_full(&home, "r", &repo).unwrap();
        let db = store::db_path(&home, "r");
        let conn = store::open_live(&db).unwrap();
        let snap = store::latest_snapshot(&conn).unwrap().unwrap();
        let canon = repo.canonicalize().unwrap();
        assert!(verify_snapshot_content(&conn, snap.id, &canon).unwrap());
        std::fs::write(repo.join("a.py"), b"def f():\n    return changed()\n").unwrap();
        assert!(!verify_snapshot_content(&conn, snap.id, &canon).unwrap());
    }

    #[test]
    fn snapshot_nonces_are_unique_across_rebuilds() {
        let home = tmpdir("idx-nonce-home");
        let repo = plain_repo("idx-nonce");
        build_full(&home, "r", &repo).unwrap();
        let db = store::db_path(&home, "r");
        let first = store::latest_snapshot(&store::open_live(&db).unwrap())
            .unwrap()
            .unwrap()
            .nonce;
        assert!(!first.is_empty());
        std::fs::write(repo.join("b.py"), b"def g():\n    pass\n").unwrap();
        build_full(&home, "r", &repo).unwrap();
        let second = store::latest_snapshot(&store::open_live(&db).unwrap())
            .unwrap()
            .unwrap()
            .nonce;
        assert_ne!(
            first, second,
            "rebuilds must mint fresh revision identities"
        );
    }

    #[test]
    fn fresh_after_clean_build() {
        let home = tmpdir("idx-fresh-home");
        let repo = plain_repo("idx-fresh");
        let summary = build_full(&home, "r", &repo).unwrap();
        assert_eq!(summary.status_at_write, "fresh");
        assert_eq!(summary.processed, 1);
        assert_eq!(summary.definitions, 1);
        let fresh = evaluate(&home, "r", &repo);
        assert_eq!(fresh.status, "fresh");
        assert!(fresh.reasons.is_empty());
        assert!(fresh.snapshot.is_some());
    }

    #[test]
    fn dirty_worktree_without_head_change_is_stale() {
        let Some(repo) = git_repo("idx-dirty", "main") else {
            return;
        };
        let home = tmpdir("idx-dirty-home");
        build_full(&home, "r", &repo).unwrap();
        assert_eq!(evaluate(&home, "r", &repo).status, "fresh");
        std::fs::write(repo.join("a.txt"), "changed\n").unwrap();
        let fresh = evaluate(&home, "r", &repo);
        assert_eq!(fresh.status, "stale");
        assert!(fresh
            .reasons
            .iter()
            .any(|r| r == "dirty" || r == "content-changed"));
    }

    #[test]
    fn rename_delete_and_new_files_are_stale() {
        let home = tmpdir("idx-rename-home");
        let repo = plain_repo("idx-rename");
        build_full(&home, "r", &repo).unwrap();
        std::fs::rename(repo.join("a.py"), repo.join("b.py")).unwrap();
        let fresh = evaluate(&home, "r", &repo);
        assert_eq!(fresh.status, "stale");
        assert!(fresh
            .reasons
            .iter()
            .any(|r| r == "file-added" || r == "file-removed"));
        std::fs::remove_file(repo.join("b.py")).unwrap();
        std::fs::write(repo.join("c.py"), b"x = 1\n").unwrap();
        let fresh = evaluate(&home, "r", &repo);
        assert_eq!(fresh.status, "stale");
        build_full(&home, "r", &repo).unwrap();
        assert_eq!(evaluate(&home, "r", &repo).status, "fresh");
    }

    #[test]
    fn new_commit_and_ignore_change_are_stale() {
        let Some(repo) = git_repo("idx-head", "main") else {
            return;
        };
        let home = tmpdir("idx-head-home");
        build_full(&home, "r", &repo).unwrap();
        std::fs::write(repo.join("b.txt"), "new\n").unwrap();
        let st = std::process::Command::new("git")
            .args(["add", "b.txt"])
            .current_dir(&repo)
            .env("GIT_TERMINAL_PROMPT", "0")
            .status()
            .unwrap();
        assert!(st.success());
        let st = std::process::Command::new("git")
            .args(["commit", "-m", "second"])
            .current_dir(&repo)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@t")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@t")
            .status()
            .unwrap();
        assert!(st.success());
        let fresh = evaluate(&home, "r", &repo);
        assert_eq!(fresh.status, "stale");
        assert!(fresh.reasons.iter().any(|r| r == "head-changed"));
        std::fs::write(repo.join(".riltraignore"), "b.txt\n").unwrap();
        let fresh = evaluate(&home, "r", &repo);
        assert_eq!(fresh.status, "stale");
        assert!(fresh.reasons.iter().any(|r| r == "config-changed"));
    }

    #[test]
    fn parser_version_mismatch_is_stale() {
        let home = tmpdir("idx-parser-home");
        let repo = plain_repo("idx-parser");
        // Fabricate a snapshot recorded by a different parser build.
        let db = store::db_path(&home, "r");
        std::fs::create_dir_all(db.parent().unwrap()).unwrap();
        let conn = store::open_live(&db).unwrap();
        let mut snap = fabricated_snapshot("r", &repo);
        snap.parser_version = super::super::store::PARSER_VERSION + 1;
        store::insert_snapshot(&conn, &snap).unwrap();
        drop(conn);
        let fresh = evaluate(&home, "r", &repo);
        assert_eq!(fresh.status, "stale");
        assert!(fresh.reasons.iter().any(|r| r == "parser-changed"));
    }

    #[test]
    fn changed_during_index_is_recorded_stale() {
        let home = tmpdir("idx-mixed-home");
        let repo = plain_repo("idx-mixed");
        let db = store::db_path(&home, "r");
        std::fs::create_dir_all(db.parent().unwrap()).unwrap();
        let conn = store::open_live(&db).unwrap();
        let mut snap = fabricated_snapshot("r", &repo);
        snap.pre_fingerprint = "before".to_string();
        snap.post_fingerprint = "after".to_string();
        snap.status_at_write = "stale".to_string();
        store::insert_snapshot(&conn, &snap).unwrap();
        drop(conn);
        let fresh = evaluate(&home, "r", &repo);
        assert_eq!(fresh.status, "stale");
        assert!(fresh.reasons.iter().any(|r| r == "changed-during-index"));
    }

    #[test]
    fn missing_index_and_root_are_unknown_never_fresh() {
        let home = tmpdir("idx-unknown-home");
        let repo = plain_repo("idx-unknown");
        let fresh = evaluate(&home, "nope", &repo);
        assert_eq!(fresh.status, "unknown");
        let fresh = evaluate(&home, "r", &repo);
        assert_eq!(fresh.status, "unknown");
        let gone = repo.join("missing");
        let fresh = evaluate(&home, "r", &gone);
        assert_eq!(fresh.status, "unknown");
        assert_ne!(fresh.status, "fresh");
    }

    #[test]
    fn corrupt_staging_leftover_does_not_block_rebuild() {
        let home = tmpdir("idx-staging-home");
        let repo = plain_repo("idx-staging");
        build_full(&home, "r", &repo).unwrap();
        let staging = store::staging_path(&home, "r");
        std::fs::write(&staging, b"corrupt leftover").unwrap();
        let summary = build_full(&home, "r", &repo).unwrap();
        assert_eq!(summary.processed, 1);
        assert!(!staging.exists(), "staging must be consumed by promote");
        assert_eq!(evaluate(&home, "r", &repo).status, "fresh");
    }

    #[test]
    fn fingerprints_cover_git_and_config_not_content() {
        // Pre/post fingerprints guard global inputs (git state, config).
        // File contents are guarded per-file by digests at evaluate time.
        let repo = plain_repo("idx-fp");
        let before = input_fingerprint(&repo);
        std::fs::write(repo.join("a.py"), b"def f():\n    return h()\n").unwrap();
        assert_eq!(
            input_fingerprint(&repo),
            before,
            "content alone is a per-file digest concern"
        );
        let cfg_before = config_fingerprint(&repo);
        std::fs::write(repo.join(".riltraignore"), "x\n").unwrap();
        assert_ne!(config_fingerprint(&repo), cfg_before);
        assert_ne!(input_fingerprint(&repo), before);
    }

    fn fabricated_snapshot(repo: &str, root: &std::path::Path) -> store::Snapshot {
        let canon = root.canonicalize().unwrap().to_string_lossy().to_string();
        store::Snapshot {
            id: 0,
            repo: repo.to_string(),
            root: canon,
            created_unix: 1,
            head: None,
            branch: None,
            dirty: false,
            untracked: 0,
            parser_version: store::PARSER_VERSION,
            grammar_versions: store::grammar_versions(),
            config_fingerprint: config_fingerprint(root),
            pre_fingerprint: "same".to_string(),
            post_fingerprint: "same".to_string(),
            status_at_write: "fresh".to_string(),
            nonce: store::new_nonce(),
        }
    }
}
