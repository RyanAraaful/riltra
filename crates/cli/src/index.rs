// SPDX-License-Identifier: Apache-2.0
//! M3 atomic indexing: full rebuilds from internal parser facts into a staging
//! database, promoted atomically; plus freshness evaluation with reasons.
//! Queries never trust a timestamp alone — every verdict names its evidence.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

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

/// Full rebuild into staging, then atomic promotion. Facts stream from the
/// internal parser straight into one transaction — the whole repository is
/// never buffered, and readers never see a half-written snapshot.
pub fn build_full(home: &Path, name: &str, root: &Path) -> Result<IndexSummary, String> {
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
    let mut conn = store::open_live(&staging)?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let snapshot_id = store::insert_snapshot(
        &tx,
        &store::Snapshot {
            id: 0,
            repo: name.to_string(),
            root: canon_root.to_string_lossy().to_string(),
            created_unix: now_unix(),
            head: git_state.head.clone(),
            branch: git_state.branch.clone(),
            dirty: git_state.dirty,
            untracked: git_state.untracked,
            parser_version: store::PARSER_VERSION,
            grammar_versions: store::grammar_versions(),
            config_fingerprint: cfg,
            pre_fingerprint: pre.clone(),
            post_fingerprint: String::new(),
            status_at_write: "stale".to_string(),
        },
    )?;
    let (plan, _) = scan::plan_walk(&canon_root)?;
    let mut summary = IndexSummary {
        snapshot_id,
        status_at_write: "stale".to_string(),
        pre_fingerprint: pre,
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
    };
    for item in &plan {
        summary.files_discovered += 1;
        if let PlanAction::Skip(reason) = &item.action {
            match *reason {
                "ignored" => summary.skipped_ignored += 1,
                "symlink-not-followed" => summary.skipped_symlink += 1,
                _ => summary.skipped_unreadable += 1,
            }
            continue;
        }
        // Content gates before the extension verdict, mirroring scan render:
        // a huge or binary `.xyz` counts as skipped, not unsupported.
        let bytes = match std::fs::read(&item.full) {
            Ok(b) => b,
            Err(_) => {
                summary.skipped_unreadable += 1;
                continue;
            }
        };
        if bytes.len() as u64 > scan::MAX_FILE_BYTES {
            summary.skipped_large += 1;
            continue;
        }
        if bytes.contains(&0) || std::str::from_utf8(&bytes).is_err() {
            summary.skipped_binary += 1;
            continue;
        }
        match &item.action {
            PlanAction::Parse(lang) => {
                let facts = parse_source(*lang, &bytes);
                let file_id = store::insert_file(
                    &tx,
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
                        &tx,
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
                        &tx,
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
                        &tx,
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
            }
            PlanAction::Unsupported(_) => summary.unsupported_total += 1,
            PlanAction::Skip(_) => {} // returned early above; listed for exhaustiveness
        }
    }
    let post = input_fingerprint(&canon_root);
    let status = if summary.pre_fingerprint == post {
        "fresh"
    } else {
        "stale"
    };
    store::finish_snapshot(&tx, snapshot_id, &post, status)?;
    tx.commit().map_err(|e| e.to_string())?;
    drop(conn);
    let live = match store::open_live(&db) {
        Ok(live) => live,
        Err(_) if !store::is_readable_db(&db) => {
            // Corrupt live database with a complete staging build ready:
            // everything is re-derivable from source, so replace wholesale.
            std::fs::remove_file(&db)
                .map_err(|e| format!("live index is corrupt and cannot be replaced: {e}"))?;
            store::open_live(&db)?
        }
        Err(e) => return Err(e),
    };
    store::promote_staging(&live, &staging)?;
    drop(live);
    let _ = std::fs::remove_file(&staging);
    summary.status_at_write = status.to_string();
    summary.post_fingerprint = post;
    Ok(summary)
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
    let conn = match store::open_live(&db) {
        Ok(c) => c,
        Err(_) => {
            return Freshness {
                status: "unknown".to_string(),
                reasons: vec!["db-unreadable".to_string()],
                snapshot: None,
            };
        }
    };
    let snapshot = match store::latest_snapshot(&conn) {
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
    let mut reasons = Vec::new();
    if canon_root.to_string_lossy() != snapshot.root {
        reasons.push("root-moved".to_string());
    }
    let live = git::fingerprint(&canon_root);
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
    if snapshot.config_fingerprint != config_fingerprint(&canon_root) {
        reasons.push("config-changed".to_string());
    }
    match content_diff(&conn, snapshot.id, &canon_root) {
        Ok(mut diff) => reasons.append(&mut diff),
        Err(_) => reasons.push("content-unverifiable".to_string()),
    }
    let status = if reasons.is_empty() { "fresh" } else { "stale" };
    Freshness {
        status: status.to_string(),
        reasons,
        snapshot: Some(snapshot),
    }
}

#[cfg(test)]
mod tests {
    use super::{build_full, config_fingerprint, evaluate, input_fingerprint};
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
        }
    }
}
