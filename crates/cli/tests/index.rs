// SPDX-License-Identifier: Apache-2.0
//! Tier A index tests: atomic rebuilds, freshness verdicts, corruption,
//! concurrency, and isolation — all through the real binary with isolated
//! `RILTRA_HOME` directories, so every spawn is a true reopen.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn home(tag: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let mut p = std::env::temp_dir();
    p.push(format!("riltra-idx-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn run(home: &Path, args: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_riltra"))
        .args(args)
        .env("RILTRA_HOME", home)
        .output()
        .expect("spawn riltra");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn mkrepo(home: &Path, name: &str) -> PathBuf {
    let d = home.join(name);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn git(dir: &Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .args(args)
            .current_dir(dir)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@t")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@t")
            .status()
            .expect("run git")
            .success(),
        "git {args:?} failed"
    );
}

fn git_repo(home: &Path, name: &str, branch: &str) -> Option<PathBuf> {
    if !git_available() {
        return None;
    }
    let dir = mkrepo(home, name);
    git(&dir, &["init", "-b", branch, "."]);
    git(&dir, &["config", "user.email", "t@t"]);
    git(&dir, &["config", "user.name", "t"]);
    std::fs::write(dir.join("a.txt"), "hello\n").unwrap();
    git(&dir, &["add", "a.txt"]);
    git(&dir, &["commit", "-m", "init"]);
    Some(dir)
}

fn fixture_source(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("m2")
        .join(rel)
}

fn copy_fixture(dir: &Path, rel: &str) {
    let dest = dir.join(rel);
    std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
    std::fs::copy(fixture_source(rel), &dest).unwrap();
}

fn init_and_index(home: &Path, dir: &Path, name: &str) -> String {
    let (code, _, err) = run(
        home,
        &["init", "--repo", dir.to_str().unwrap(), "--name", name],
    );
    assert_eq!(code, 0, "init failed: {err}");
    let (code, out, err) = run(home, &["index", "--repo", name]);
    assert_eq!(code, 0, "index failed: {err}");
    out
}

/// Envelope `"snapshot":{"id":N,...}` (first occurrence is the envelope-level
/// one); per-hit entries stay `"snapshot":N`.
fn envelope_snapshot(out: &str) -> String {
    let key = "\"snapshot\":{\"id\":";
    let pos = out.find(key).expect("envelope must carry snapshot");
    let rest = &out[pos + key.len()..];
    rest.chars().take_while(|c| c.is_ascii_digit()).collect()
}

/// Every per-hit `"snapshot":N` in the output must equal the envelope id.
/// Requires at least one hit: vacuous consistency proves nothing.
fn snapshot_ids_consistent(out: &str) -> bool {
    let id = envelope_snapshot(out);
    let mut rest = out;
    let mut found_hit = false;
    while let Some(pos) = rest.find("\"snapshot\":") {
        rest = &rest[pos + "\"snapshot\":".len()..];
        // Skip the envelope object form `"snapshot":{...}`.
        if rest.starts_with('{') {
            rest = &rest[1..];
            continue;
        }
        let value: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if value.is_empty() || value != id {
            return false;
        }
        found_hit = true;
    }
    found_hit
}

#[test]
fn search_response_carries_one_revision_identity() {
    let h = home("nonce");
    let repo = mkrepo(&h, "work");
    copy_fixture(&repo, "py/hello.py");
    init_and_index(&h, &repo, "r");
    let (_, first, _) = run(&h, &["search", "--repo", "r", "--symbol", "Worker"]);
    assert!(first.contains("\"nonce\":\""), "got: {first}");
    assert!(
        !first.contains("\"nonce\":\"\""),
        "nonce must not be empty: {first}"
    );
    assert!(snapshot_ids_consistent(&first));
    std::fs::write(repo.join("b.py"), b"def g():\n    pass\n").unwrap();
    init_and_index(&h, &repo, "r");
    let (_, second, _) = run(&h, &["search", "--repo", "r", "--symbol", "Worker"]);
    let nonce_of = |out: &str| -> String {
        let key = "\"nonce\":\"";
        let pos = out.find(key).expect("nonce in envelope");
        let rest = &out[pos + key.len()..];
        rest[..rest.find('"').unwrap()].to_string()
    };
    assert_ne!(
        nonce_of(&first),
        nonce_of(&second),
        "revisions must be distinguishable"
    );
    assert!(snapshot_ids_consistent(&second));
}

#[test]
fn build_reopen_search_is_consistent() {
    let h = home("reopen");
    let repo = mkrepo(&h, "work");
    copy_fixture(&repo, "py/hello.py");
    init_and_index(&h, &repo, "r");
    let (_, first, _) = run(&h, &["search", "--repo", "r", "--symbol", "Worker"]);
    let (_, second, _) = run(&h, &["search", "--repo", "r", "--symbol", "Worker"]);
    assert_eq!(first, second, "reopen must serve identical results");
    assert!(first.contains("\"freshness\":\"fresh\""));
    assert!(first.contains("\"name\":\"Worker\""));
    assert!(snapshot_ids_consistent(&first));
}

#[test]
fn dirty_without_head_change_is_stale_but_served() {
    let h = home("dirty");
    let Some(repo) = git_repo(&h, "code", "main") else {
        return;
    };
    copy_fixture(&repo, "py/hello.py");
    init_and_index(&h, &repo, "r");
    std::fs::write(repo.join("a.txt"), "changed\n").unwrap();
    let (code, out, _) = run(&h, &["index", "status", "--repo", "r"]);
    assert_eq!(code, 0);
    assert!(out.contains("\"freshness\":\"stale\""), "got: {out}");
    assert!(
        out.contains("dirty") || out.contains("content-changed"),
        "got: {out}"
    );
    let (code, out, _) = run(&h, &["search", "--repo", "r", "--symbol", "Worker"]);
    assert_eq!(code, 0, "stale snapshots stay queryable with their flag");
    assert!(out.contains("\"freshness\":\"stale\""));
    assert!(out.contains("\"name\":\"Worker\""));
}

#[test]
fn rename_delete_and_new_files_are_stale_then_fresh() {
    let h = home("rename");
    let repo = mkrepo(&h, "work");
    std::fs::write(repo.join("a.py"), b"def alpha_fn():\n    pass\n").unwrap();
    init_and_index(&h, &repo, "r");
    std::fs::rename(repo.join("a.py"), repo.join("b.py")).unwrap();
    let (_, out, _) = run(&h, &["index", "status", "--repo", "r"]);
    assert!(out.contains("\"freshness\":\"stale\""), "got: {out}");
    assert!(
        out.contains("file-added") || out.contains("file-removed"),
        "got: {out}"
    );
    std::fs::remove_file(repo.join("b.py")).unwrap();
    std::fs::write(repo.join("c.py"), b"def gamma_fn():\n    pass\n").unwrap();
    let (_, out, _) = run(&h, &["index", "status", "--repo", "r"]);
    assert!(out.contains("\"freshness\":\"stale\""), "got: {out}");
    init_and_index(&h, &repo, "r");
    let (_, out, _) = run(&h, &["index", "status", "--repo", "r"]);
    assert!(out.contains("\"freshness\":\"fresh\""), "got: {out}");
    let (_, out, _) = run(&h, &["search", "--repo", "r", "--symbol", "alpha_fn"]);
    assert!(out.contains("\"total\":0"), "got: {out}");
    let (_, out, _) = run(&h, &["search", "--repo", "r", "--symbol", "gamma_fn"]);
    assert!(out.contains("\"name\":\"gamma_fn\""), "got: {out}");
}

#[test]
fn branch_head_and_ignore_changes_are_stale() {
    let h = home("branch");
    let Some(repo) = git_repo(&h, "code", "main") else {
        return;
    };
    init_and_index(&h, &repo, "r");
    git(&repo, &["commit", "--allow-empty", "-m", "second"]);
    let (_, out, _) = run(&h, &["index", "status", "--repo", "r"]);
    assert!(out.contains("head-changed"), "got: {out}");
    git(&repo, &["checkout", "-b", "feature"]);
    let (_, out, _) = run(&h, &["index", "status", "--repo", "r"]);
    assert!(out.contains("branch-changed"), "got: {out}");
    std::fs::write(repo.join(".riltraignore"), "a.txt\n").unwrap();
    let (_, out, _) = run(&h, &["index", "status", "--repo", "r"]);
    assert!(out.contains("config-changed"), "got: {out}");
}

#[test]
fn corrupt_staging_leftover_does_not_block_rebuild() {
    let h = home("staging");
    let repo = mkrepo(&h, "work");
    copy_fixture(&repo, "py/hello.py");
    init_and_index(&h, &repo, "r");
    let (_, before, _) = run(&h, &["search", "--repo", "r", "--symbol", "Worker"]);
    let staging = h.join("index").join("r.db.staging");
    std::fs::write(&staging, b"corrupt leftover").unwrap();
    let (code, out, err) = run(&h, &["index", "--repo", "r"]);
    assert_eq!(code, 0, "rebuild must overwrite corrupt staging: {err}");
    assert!(out.contains("\"status_at_write\":\"fresh\""), "got: {out}");
    assert!(!staging.exists(), "staging must be consumed by promote");
    let (_, after, _) = run(&h, &["search", "--repo", "r", "--symbol", "Worker"]);
    assert!(
        after.contains("\"name\":\"Worker\""),
        "old data intact: {before} vs {after}"
    );
}

#[test]
fn corrupt_live_db_reports_unknown_and_recovers() {
    let h = home("corrupt");
    let repo = mkrepo(&h, "work");
    copy_fixture(&repo, "py/hello.py");
    init_and_index(&h, &repo, "r");
    std::fs::write(h.join("index").join("r.db"), b"not a database").unwrap();
    let (code, out, _) = run(&h, &["index", "status", "--repo", "r"]);
    assert_eq!(code, 0);
    assert!(out.contains("\"freshness\":\"unknown\""), "got: {out}");
    assert!(out.contains("db-unreadable"), "got: {out}");
    let (code, _, _) = run(&h, &["search", "--repo", "r", "--symbol", "Worker"]);
    assert_eq!(code, 3, "unreadable db must fail search loudly");
    let (code, out, err) = run(&h, &["index", "--repo", "r"]);
    assert_eq!(code, 0, "rebuild must recover from corrupt db: {err}");
    assert!(out.contains("\"status_at_write\":\"fresh\""), "got: {out}");
}

#[test]
fn concurrent_queries_during_rebuild_stay_consistent() {
    let h = home("concurrent");
    let repo = mkrepo(&h, "work");
    copy_fixture(&repo, "py/hello.py");
    copy_fixture(&repo, "js/hello.js");
    init_and_index(&h, &repo, "r");
    let home_clone = h.clone();
    let builder = std::thread::spawn(move || {
        for _ in 0..2 {
            let (code, _, err) = run(&home_clone, &["index", "--repo", "r"]);
            assert_eq!(code, 0, "rebuild failed: {err}");
        }
    });
    let mut checked = 0;
    for _ in 0..30 {
        let (code, out, _) = run(&h, &["search", "--repo", "r", "--symbol", "a"]);
        assert_eq!(code, 0);
        assert!(
            snapshot_ids_consistent(&out),
            "mixed snapshots in one response: {out}"
        );
        checked += 1;
    }
    builder.join().unwrap();
    assert!(checked > 0);
}

#[test]
fn two_repositories_stay_isolated() {
    let h = home("isolated");
    let a = mkrepo(&h, "a");
    let b = mkrepo(&h, "b");
    std::fs::write(a.join("x.py"), b"def only_in_a():\n    pass\n").unwrap();
    std::fs::write(b.join("x.py"), b"def only_in_b():\n    pass\n").unwrap();
    init_and_index(&h, &a, "ra");
    init_and_index(&h, &b, "rb");
    assert!(h.join("index").join("ra.db").exists());
    assert!(h.join("index").join("rb.db").exists());
    let (_, out_a, _) = run(&h, &["search", "--repo", "ra", "--symbol", "only_in_b"]);
    assert!(out_a.contains("\"total\":0"), "got: {out_a}");
    let (_, out_b, _) = run(&h, &["search", "--repo", "rb", "--symbol", "only_in_a"]);
    assert!(out_b.contains("\"total\":0"), "got: {out_b}");
    let (_, out_a, _) = run(&h, &["search", "--repo", "ra", "--symbol", "only_in_a"]);
    assert!(out_a.contains("\"name\":\"only_in_a\""), "got: {out_a}");
}

#[test]
fn search_spans_match_m2_goldens() {
    let h = home("golden");
    let repo = mkrepo(&h, "work");
    copy_fixture(&repo, "c/main.c");
    init_and_index(&h, &repo, "r");
    let (_, out, _) = run(&h, &["search", "--repo", "r", "--symbol", "add"]);
    // Values copied from the verified tests/golden/m2-c-main.json contract.
    assert!(out.contains("\"name\":\"add\""), "got: {out}");
    assert!(
        out.contains("\"start_line\":6,\"start_col\":1,\"end_line\":8,\"end_col\":2"),
        "got: {out}"
    );
    assert!(out.contains("\"bytes\":[106,149]"), "got: {out}");
    assert!(out.contains("\"provider\":\"syntax\""), "got: {out}");
    assert!(out.contains("\"freshness\":\"fresh\""), "got: {out}");
}

#[test]
fn unknown_states_are_never_fresh() {
    let h = home("neverfresh");
    let repo = mkrepo(&h, "work");
    let (code, _, _) = run(
        &h,
        &["init", "--repo", repo.to_str().unwrap(), "--name", "r"],
    );
    assert_eq!(code, 0);
    let (_, out, _) = run(&h, &["index", "status", "--repo", "r"]);
    assert!(out.contains("\"freshness\":\"unknown\""), "got: {out}");
    assert!(out.contains("no-index"), "got: {out}");
    let (_, out, _) = run(&h, &["index", "status", "--repo", "nope"]);
    assert!(out.is_empty(), "unknown repos fail loudly, no JSON");
    init_and_index(&h, &repo, "r");
    std::fs::rename(&repo, h.join("gone")).unwrap();
    let (_, out, _) = run(&h, &["index", "status", "--repo", "r"]);
    assert!(out.contains("\"freshness\":\"unknown\""), "got: {out}");
    assert!(out.contains("root-unreadable"), "got: {out}");
}

#[test]
fn pagination_contract_holds() {
    let h = home("paging");
    let repo = mkrepo(&h, "work");
    copy_fixture(&repo, "py/hello.py");
    copy_fixture(&repo, "js/hello.js");
    init_and_index(&h, &repo, "r");
    let (code, out, _) = run(
        &h,
        &["search", "--repo", "r", "--symbol", "a", "--limit", "1"],
    );
    assert_eq!(code, 0);
    assert!(out.contains("\"truncated\":true"), "got: {out}");
    assert!(out.contains("\"limit\":1"), "got: {out}");
    let (_, page2, _) = run(
        &h,
        &[
            "search", "--repo", "r", "--symbol", "a", "--limit", "1", "--offset", "1",
        ],
    );
    assert_ne!(out, page2);
    let (code, _, err) = run(
        &h,
        &["search", "--repo", "r", "--symbol", "a", "--limit", "abc"],
    );
    assert_eq!(code, 2, "bad limit must be a usage error: {err}");
    let (code, _, _) = run(
        &h,
        &["search", "--repo", "r", "--symbol", "a", "--limit", "99999"],
    );
    assert_eq!(code, 2);
}

#[test]
fn index_output_contract_and_unknown_repo() {
    let h = home("contract");
    let repo = mkrepo(&h, "work");
    copy_fixture(&repo, "py/hello.py");
    let (code, _, _) = run(
        &h,
        &["init", "--repo", repo.to_str().unwrap(), "--name", "r"],
    );
    assert_eq!(code, 0);
    let (code, out, _) = run(&h, &["index", "--repo", "r"]);
    assert_eq!(code, 0);
    assert!(out.contains("\"status_at_write\":\"fresh\""), "got: {out}");
    assert!(out.contains("\"processed\":1"), "got: {out}");
    let (code, _, err) = run(&h, &["index", "--repo", "nope"]);
    assert_eq!(code, 3, "unknown repo must fail: {err}");
}

#[test]
fn cli_cold_start_reports_cold_start() {
    let h = home("coldstart");
    let repo = mkrepo(&h, "work");
    copy_fixture(&repo, "py/hello.py");
    let (code, _, err) = run(
        &h,
        &["init", "--repo", repo.to_str().unwrap(), "--name", "r"],
    );
    assert_eq!(code, 0, "init failed: {err}");
    let (code, out, err) = run(&h, &["index", "--repo", "r"]);
    assert_eq!(code, 0, "index failed: {err}");
    assert!(out.contains("\"mode\":\"full\""), "got: {out}");
    assert!(
        out.contains("\"fallback_reason\":\"cold-start\""),
        "got: {out}"
    );
}

#[test]
fn cli_noop_reports_zero_reparsed() {
    let h = home("noop");
    let repo = mkrepo(&h, "work");
    copy_fixture(&repo, "py/hello.py");
    init_and_index(&h, &repo, "r");
    let (code, out, err) = run(&h, &["index", "--repo", "r"]);
    assert_eq!(code, 0, "index failed: {err}");
    assert!(out.contains("\"mode\":\"incremental\""), "got: {out}");
    assert!(out.contains("\"reparsed\":0"), "got: {out}");
    assert!(out.contains("\"fallback_reason\":\"none\""), "got: {out}");
    assert!(out.contains("\"timings_ms\":{\"diff\":"), "got: {out}");
}

#[test]
fn cli_full_flag_forces_full() {
    let h = home("forcedfull");
    let repo = mkrepo(&h, "work");
    copy_fixture(&repo, "py/hello.py");
    init_and_index(&h, &repo, "r");
    let (code, out, err) = run(&h, &["index", "--repo", "r", "--full"]);
    assert_eq!(code, 0, "index failed: {err}");
    assert!(out.contains("\"mode\":\"full\""), "got: {out}");
    assert!(
        out.contains("\"fallback_reason\":\"forced-full\""),
        "got: {out}"
    );
}

/// Replace volatile envelope values (nonce, snapshot ids, timestamps) so
/// two independent builds can be compared string-for-string.
fn normalize_search(out: &str) -> String {
    let mut s = out.to_string();
    if let Some(pos) = s.find("\"nonce\":\"") {
        let start = pos + "\"nonce\":\"".len();
        if let Some(end) = s[start..].find('"') {
            let nonce = s[start..start + end].to_string();
            s = s.replace(&nonce, "NONCE");
        }
    }
    // Rewrite every `"key":<digits>` occurrence for volatile numeric keys.
    for key in ["\"id\":", "\"created_unix\":", "\"snapshot\":"] {
        let mut result = String::new();
        let mut rest = s.as_str();
        while let Some(pos) = rest.find(key) {
            result.push_str(&rest[..pos + key.len()]);
            rest = &rest[pos + key.len()..];
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            rest = &rest[digits.len()..];
            result.push('0');
        }
        result.push_str(rest);
        s = result;
    }
    s
}

fn files_counters(out: &str) -> String {
    let pos = out.find("\"files\":").expect("index JSON must carry files");
    out[pos..].trim_end().to_string()
}

#[test]
fn cli_search_parity_full_vs_incremental() {
    // Guard test (Task 5): PASS-first-run expected.
    let h = home("paritycli");
    let repo = mkrepo(&h, "work");
    copy_fixture(&repo, "py/hello.py");
    let (code, _, err) = run(
        &h,
        &["init", "--repo", repo.to_str().unwrap(), "--name", "r"],
    );
    assert_eq!(code, 0, "init failed: {err}");
    // Evolve through an incremental path in the default home.
    let (code, full_first, err) = run(&h, &["index", "--repo", "r"]);
    assert_eq!(code, 0, "index failed: {err}");
    std::fs::write(repo.join("extra.py"), b"def helper():\n    pass\n").unwrap();
    let (code, inc_out, err) = run(&h, &["index", "--repo", "r"]);
    assert_eq!(code, 0, "index failed: {err}");
    assert!(
        inc_out.contains("\"mode\":\"incremental\""),
        "got: {inc_out}"
    );
    // Independent full rebuild of the same final tree in a second home.
    let h2 = home("paritycli-full");
    let (code, _, err) = run(
        &h2,
        &["init", "--repo", repo.to_str().unwrap(), "--name", "r"],
    );
    assert_eq!(code, 0, "init failed: {err}");
    let (code, full_out, err) = run(&h2, &["index", "--repo", "r"]);
    assert_eq!(code, 0, "index failed: {err}");
    assert_eq!(
        files_counters(&inc_out),
        files_counters(&full_out),
        "fact-bearing counters must match"
    );
    let _ = full_first;
    for symbol in ["Worker", "helper", "missing-symbol-zzz"] {
        let (_, inc_search, _) = run(&h, &["search", "--repo", "r", "--symbol", symbol]);
        let (_, full_search, _) = run(&h2, &["search", "--repo", "r", "--symbol", symbol]);
        assert_eq!(
            normalize_search(&inc_search),
            normalize_search(&full_search),
            "search parity for {symbol}"
        );
    }
}

#[test]
fn concurrent_search_during_incremental_refresh_stays_consistent() {
    let h = home("concurrent-inc");
    let repo = mkrepo(&h, "work");
    copy_fixture(&repo, "py/hello.py");
    copy_fixture(&repo, "js/hello.js");
    init_and_index(&h, &repo, "r");
    let home_clone = h.clone();
    let builder = std::thread::spawn(move || {
        for _ in 0..2 {
            let (code, out, err) = run(&home_clone, &["index", "--repo", "r"]);
            assert_eq!(code, 0, "rebuild failed: {err}");
            assert!(
                out.contains("\"mode\":\"incremental\""),
                "expected incremental refresh: {out}"
            );
        }
    });
    let mut checked = 0;
    for _ in 0..30 {
        let (code, out, _) = run(&h, &["search", "--repo", "r", "--symbol", "a"]);
        assert_eq!(code, 0);
        assert!(
            snapshot_ids_consistent(&out),
            "mixed snapshots in one response: {out}"
        );
        checked += 1;
    }
    builder.join().unwrap();
    assert!(checked > 0);
}

#[test]
fn incremental_two_repositories_stay_isolated() {
    let h = home("isolated-inc");
    let a = mkrepo(&h, "a");
    let b = mkrepo(&h, "b");
    std::fs::write(a.join("x.py"), b"def only_in_a():\n    pass\n").unwrap();
    std::fs::write(b.join("x.py"), b"def only_in_b():\n    pass\n").unwrap();
    init_and_index(&h, &a, "ra");
    init_and_index(&h, &b, "rb");
    let (code, out, err) = run(&h, &["index", "--repo", "ra"]);
    assert_eq!(code, 0, "incremental index failed: {err}");
    assert!(out.contains("\"mode\":\"incremental\""), "got: {out}");
    let (code, out, err) = run(&h, &["index", "--repo", "rb"]);
    assert_eq!(code, 0, "incremental index failed: {err}");
    assert!(out.contains("\"mode\":\"incremental\""), "got: {out}");
    let (_, out_a, _) = run(&h, &["search", "--repo", "ra", "--symbol", "only_in_b"]);
    assert!(out_a.contains("\"total\":0"), "got: {out_a}");
    let (_, out_b, _) = run(&h, &["search", "--repo", "rb", "--symbol", "only_in_a"]);
    assert!(out_b.contains("\"total\":0"), "got: {out_b}");
}

#[test]
fn incremental_recovers_partial_staging() {
    let h = home("staging-inc");
    let repo = mkrepo(&h, "work");
    copy_fixture(&repo, "py/hello.py");
    init_and_index(&h, &repo, "r");
    let staging = h.join("index").join("r.db.staging");
    std::fs::write(&staging, b"corrupt leftover").unwrap();
    let (code, out, err) = run(&h, &["index", "--repo", "r"]);
    assert_eq!(code, 0, "rebuild must overwrite corrupt staging: {err}");
    assert!(!staging.exists(), "staging must be consumed by promote");
    let _ = out;
}

#[test]
fn incremental_recovers_corrupt_live_as_fallback() {
    let h = home("corruptlive-inc");
    let repo = mkrepo(&h, "work");
    copy_fixture(&repo, "py/hello.py");
    init_and_index(&h, &repo, "r");
    std::fs::write(h.join("index").join("r.db"), b"not a database").unwrap();
    let (code, out, err) = run(&h, &["index", "--repo", "r"]);
    assert_eq!(code, 0, "recovery rebuild must succeed: {err}");
    assert!(out.contains("\"mode\":\"fallback-full\""), "got: {out}");
    assert!(
        out.contains("\"fallback_reason\":\"db-unreadable\""),
        "got: {out}"
    );
    let (code, out, err) = run(&h, &["search", "--repo", "r", "--symbol", "Worker"]);
    assert_eq!(code, 0, "search after recovery failed: {err}");
    assert!(out.contains("\"total\":1"), "got: {out}");
}
