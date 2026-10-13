// SPDX-License-Identifier: Apache-2.0
//! End-to-end CLI tests: each case spawns the real binary with an isolated
//! `RILTRA_HOME` so the developer's registry is never touched.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn home(tag: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let mut p = std::env::temp_dir();
    p.push(format!("riltra-cli-{}-{}-{}", tag, std::process::id(), n));
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

use std::path::Path;

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

fn git_fixture(dir: &Path) {
    let g = |args: &[&str]| {
        assert!(Command::new("git")
            .args(args)
            .current_dir(dir)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .status()
            .expect("run git")
            .success());
    };
    g(&["init", "-b", "main", "."]);
    g(&["config", "user.email", "t@t"]);
    g(&["config", "user.name", "t"]);
    std::fs::write(dir.join("a.txt"), "hello\n").unwrap();
    g(&["add", "a.txt"]);
    g(&["commit", "-m", "init"]);
}

#[test]
fn init_and_list_round_trip() {
    let h = home("roundtrip");
    let target = mkrepo(&h, "work");
    let (code, out, _) = run(&h, &["init", "--repo", target.to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(out.contains("\"name\":\"work\""));
    assert!(out.contains("\"registered\":true"));
    let (code, out, _) = run(&h, &["init", "--repo", target.to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(out.contains("\"registered\":false"));
    let (code, out, _) = run(&h, &["repo", "list"]);
    assert_eq!(code, 0);
    assert!(out.contains("\"name\":\"work\""));
    assert!(out.starts_with("{\"repos\":["));
}

#[test]
fn init_rejects_missing_path_and_bad_name() {
    let h = home("init-err");
    let missing = h.join("nope");
    let (code, _, err) = run(&h, &["init", "--repo", missing.to_str().unwrap()]);
    assert_eq!(code, 3);
    assert!(err.contains("error"));
    let target = mkrepo(&h, "work");
    let (code, _, _) = run(
        &h,
        &[
            "init",
            "--repo",
            target.to_str().unwrap(),
            "--name",
            "../evil",
        ],
    );
    assert_eq!(code, 2);
}

#[test]
fn status_unknown_repo_exits_3_without_stdout() {
    let h = home("unknown");
    let (code, out, err) = run(&h, &["repo", "status", "--repo", "nope"]);
    assert_eq!(code, 3);
    assert!(out.is_empty());
    assert!(err.contains("unknown repository"));
}

#[test]
fn status_plain_dir_reports_absent_git() {
    let h = home("plain");
    let target = mkrepo(&h, "plain");
    assert_eq!(run(&h, &["init", "--repo", target.to_str().unwrap()]).0, 0);
    let (code, out, _) = run(&h, &["repo", "status", "--repo", "plain"]);
    assert_eq!(code, 0);
    assert!(out.contains("\"present\":false"));
    assert!(out.contains("\"ignore\":"));
}

#[test]
fn status_git_repo_reports_head_and_branch() {
    if !git_available() {
        return;
    }
    let h = home("gitrep");
    let target = mkrepo(&h, "code");
    git_fixture(&target);
    assert_eq!(run(&h, &["init", "--repo", target.to_str().unwrap()]).0, 0);
    let (code, out, _) = run(&h, &["repo", "status", "--repo", "code"]);
    assert_eq!(code, 0);
    assert!(out.contains("\"present\":true"));
    assert!(out.contains("\"valid\":true"));
    assert!(out.contains("\"branch\":\"main\""));
    assert!(out.contains("\"dirty\":false"));
    assert!(out.contains("\"head\":\""));
}

#[test]
fn status_corrupt_git_reports_invalid_with_exit_3() {
    let h = home("corrupt");
    let target = mkrepo(&h, "broken");
    std::fs::create_dir_all(target.join(".git")).unwrap();
    std::fs::write(target.join(".git").join("HEAD"), "garbage").unwrap();
    assert_eq!(run(&h, &["init", "--repo", target.to_str().unwrap()]).0, 0);
    let (code, out, _) = run(&h, &["repo", "status", "--repo", "broken"]);
    assert_eq!(code, 3);
    assert!(out.contains("\"valid\":false"));
}

#[test]
fn doctor_healthy_dir_passes_and_env_doctor_reports_scope() {
    let h = home("doctor");
    let target = mkrepo(&h, "ok");
    assert_eq!(run(&h, &["init", "--repo", target.to_str().unwrap()]).0, 0);
    let (code, out, _) = run(&h, &["doctor", "--repo", "ok"]);
    assert_eq!(code, 0);
    assert!(out.contains("\"pass\":true"));
    let (code, out, _) = run(&h, &["doctor"]);
    assert_eq!(code, 0);
    assert!(out.contains("\"scope\":\"environment\""));
}

#[test]
fn repos_are_isolated_by_name() {
    let h = home("isolated");
    let a = mkrepo(&h, "a");
    let b = mkrepo(&h, "b");
    std::fs::write(a.join("same.txt"), "A").unwrap();
    std::fs::write(b.join("same.txt"), "B").unwrap();
    assert_eq!(
        run(&h, &["init", "--repo", a.to_str().unwrap(), "--name", "ra"]).0,
        0
    );
    assert_eq!(
        run(&h, &["init", "--repo", b.to_str().unwrap(), "--name", "rb"]).0,
        0
    );
    let (_, out_a, _) = run(&h, &["repo", "status", "--repo", "ra"]);
    let (_, out_b, _) = run(&h, &["repo", "status", "--repo", "rb"]);
    assert!(!out_a.contains("same.txt")); // status carries metadata, never file content
    assert_ne!(out_a, out_b);
}
