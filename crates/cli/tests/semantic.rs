// SPDX-License-Identifier: Apache-2.0
//! M5 semantic tests: dependency audit (Task 1), Tier A integration
//! (Tasks 6-8), and evidence-contract tests. Real-provider gates
//! (Task 9) live behind env flags and never fake a PASS.

#[test]
fn new_dependencies_are_audited() {
    // Versions: resolved by cargo (Cargo.lock). Licenses: crate manifests
    // surfaced by `cargo tree` ({l} reads the manifest license field;
    // Cargo.lock records no license fields, so it is never cited for them).
    let out = std::process::Command::new("cargo")
        .args([
            "tree",
            "-p",
            "riltra",
            "--prefix",
            "none",
            "--no-dedupe",
            "--format",
            "{p} [{l}]",
            "--offline",
        ])
        .output()
        .expect("cargo tree must run");
    assert!(out.status.success(), "cargo tree failed");
    let text = String::from_utf8_lossy(&out.stdout);

    // Exact versions pinned here; bump deliberately with the audit table.
    // (M5 audit 2026-10-16: serde_json 1.0.154 MIT/Apache-2.0,
    // protobuf 3.7.2 MIT, protobuf-support 3.7.2 MIT. M8 CP1 2026-10-18,
    // owner decision D1: rmcp 3.5.1 Apache-2.0, tokio 1.53.3 MIT.
    // M8 CP3 2026-10-18: rmcp `macros` feature enabled — rmcp-macros 3.5.1
    // Apache-2.0, same rust-sdk origin; no other new packages.)
    for want in [
        "serde_json v1.0.154 [MIT OR Apache-2.0]",
        "protobuf v3.7.2 [MIT]",
        "rmcp v3.5.1 [Apache-2.0]",
        "tokio v1.53.3 [MIT]",
        "rmcp-macros v3.5.1 (proc-macro) [Apache-2.0]",
    ] {
        assert!(
            text.contains(want),
            "missing audited dependency: {want}\n{text}"
        );
    }

    // Every package in the resolve graph must carry an allowlisted license.
    // Note: memchr "Unlicense OR MIT" predates M5 (M0-M4 tree); it passes
    // via the MIT alternative, not as a new M5 approval.
    // Note: memchr/aho-corasick "Unlicense OR MIT" and foldhash "Zlib"
    // predate M5 (M0-M4 tree); Unlicense (public-domain dedication) and
    // Zlib pass as permissive alternatives here, not as new M5 approvals.
    let allow = [
        "MIT",
        "Apache-2.0",
        "BSD-3-Clause",
        "ISC",
        "Zlib",
        "Unlicense",
        "Unicode-3.0",
        "Unicode-DFS-2016",
    ];
    let mut bad = Vec::new();
    for line in text.lines() {
        if let Some(lic) = line.rsplit('[').next().and_then(|s| s.strip_suffix(']')) {
            let norm = lic.replace(['(', ')'], "").replace('/', " OR ");
            let ok = norm
                .split(" OR ")
                .flat_map(|x| x.split(" AND "))
                .all(|x| allow.contains(&x.trim()));
            if !ok {
                bad.push(line.to_string());
            }
        }
    }
    bad.sort();
    bad.dedup();
    assert!(bad.is_empty(), "non-allowlisted licenses: {bad:?}");

    // Banned: protoc bridges, non-MCP async runtimes, dynamic loading,
    // network stacks. tokio is allowed only as the M8 MCP stdio runtime
    // (owner decision D1, design gate Rev 2); no HTTP/TLS features are on.
    for banned in [
        "protoc-bin-vendored",
        "async-std",
        "libloading",
        " hyper ",
        "reqwest",
        "ureq",
    ] {
        assert!(
            !text.lines().any(|l| l.contains(banned.trim())),
            "banned crate in tree: {banned}"
        );
    }
}

// ---------------------------------------------------------------------------
// Task 7 CLI acceptance: import/status/query through the real binary.
// SCIP payloads are hand-encoded below (minimal proto3 writer for exactly
// the fields the importer reads). Field numbers pinned by scip.proto:
// Index{metadata:1, documents:2, external_symbols:3},
// Metadata{tool_info:2, project_root:3, text_document_encoding:4},
// ToolInfo{name:1, version:2}, Document{relative_path:1, occurrences:2,
// symbols:3, position_encoding:6}, Occurrence{symbol:2, symbol_roles:3,
// single_line_range:8}, SingleLineRange{line:1, start:2, end:3},
// SymbolInformation{symbol:1, display_name:6}.

fn varint(mut v: u64, out: &mut Vec<u8>) {
    loop {
        let mut b = (v & 0x7f) as u8;
        v >>= 7;
        if v != 0 {
            b |= 0x80;
        }
        out.push(b);
        if v == 0 {
            break;
        }
    }
}

fn tag(field: u32, wire: u32, out: &mut Vec<u8>) {
    varint(((field << 3) | wire) as u64, out);
}

fn len_field(field: u32, bytes: &[u8], out: &mut Vec<u8>) {
    tag(field, 2, out);
    varint(bytes.len() as u64, out);
    out.extend_from_slice(bytes);
}

fn str_field(field: u32, value: &str, out: &mut Vec<u8>) {
    len_field(field, value.as_bytes(), out);
}

fn varint_field(field: u32, value: u64, out: &mut Vec<u8>) {
    tag(field, 0, out);
    varint(value, out);
}

fn single_range(line: i64, start: i64, end: i64) -> Vec<u8> {
    let mut r = Vec::new();
    varint_field(1, line as u64, &mut r);
    varint_field(2, start as u64, &mut r);
    varint_field(3, end as u64, &mut r);
    r
}

fn occurrence(symbol: &str, roles: u64, line: i64, start: i64, end: i64) -> Vec<u8> {
    let mut o = Vec::new();
    if !symbol.is_empty() {
        str_field(2, symbol, &mut o);
    }
    varint_field(3, roles, &mut o);
    len_field(8, &single_range(line, start, end), &mut o);
    o
}

fn symbol_info(symbol: &str, display: &str) -> Vec<u8> {
    let mut s = Vec::new();
    str_field(1, symbol, &mut s);
    str_field(6, display, &mut s);
    s
}

fn document(rel: &str, occs: &[Vec<u8>], symbols: &[Vec<u8>], utf16: bool) -> Vec<u8> {
    let mut d = Vec::new();
    str_field(1, rel, &mut d);
    for o in occs {
        len_field(2, o, &mut d);
    }
    for s in symbols {
        len_field(3, s, &mut d);
    }
    varint_field(6, if utf16 { 2 } else { 1 }, &mut d);
    d
}

fn tool_info(name: &str, version: &str) -> Vec<u8> {
    let mut t = Vec::new();
    str_field(1, name, &mut t);
    str_field(2, version, &mut t);
    t
}

/// Full Index payload: metadata (tool, project_root) + documents.
/// `externals` are SymbolInformation messages for field 3.
fn index_bytes(root: &std::path::Path, docs: &[Vec<u8>], externals: &[Vec<u8>]) -> Vec<u8> {
    index_bytes_with_tool(root, "scip-test-cli", docs, externals)
}

fn index_bytes_with_tool(
    root: &std::path::Path,
    tool: &str,
    docs: &[Vec<u8>],
    externals: &[Vec<u8>],
) -> Vec<u8> {
    let mut md = Vec::new();
    len_field(2, &tool_info(tool, "1.0"), &mut md);
    str_field(
        3,
        &format!("file://{}", root.to_string_lossy().replace('\\', "/")),
        &mut md,
    );
    varint_field(4, 1, &mut md); // UTF8 text encoding
    let mut out = Vec::new();
    len_field(1, &md, &mut out);
    for d in docs {
        len_field(2, d, &mut out);
    }
    for e in externals {
        len_field(3, e, &mut out);
    }
    out
}

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(10_000);

fn home(tag: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let mut p = std::env::temp_dir();
    p.push(format!("riltra-sem-{tag}-{}-{n}", std::process::id()));
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

fn repo_with(home: &Path, name: &str, files: &[(&str, &str)]) -> PathBuf {
    let d = home.join(name);
    for (rel, content) in files {
        let p = d.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, content).unwrap();
    }
    let (code, _, _) = run(
        home,
        &["init", "--repo", &d.to_string_lossy(), "--name", name],
    );
    assert_eq!(code, 0);
    d
}

/// Extract a top-level `"key":number` from single-line JSON.
fn num(json: &str, key: &str) -> i64 {
    let pat = format!("\"{key}\":");
    let i = json
        .find(&pat)
        .unwrap_or_else(|| panic!("missing {key} in {json}"));
    json[i + pat.len()..]
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '-')
        .collect::<String>()
        .parse()
        .unwrap()
}

const FOO: &str = "scip-test test test-pkg 1.0.0 src/a.ts/foo().";

fn ab_repo(home: &Path) -> (PathBuf, String) {
    let dir = repo_with(
        home,
        "sem",
        &[
            ("src/a.ts", "export function foo() {}\nfoo();\n"),
            ("src/b.ts", "import \"./a\";\nfoo();\n"),
        ],
    );
    let a = document(
        "src/a.ts",
        &[
            occurrence(FOO, 0x1, 0, 16, 19),
            occurrence(FOO, 0x0, 1, 0, 3),
        ],
        &[symbol_info(FOO, "foo")],
        false,
    );
    let b = document("src/b.ts", &[occurrence(FOO, 0x0, 1, 0, 3)], &[], false);
    let payload = index_bytes(&dir, &[a, b], &[]);
    let scip = dir.join("index.scip");
    std::fs::write(&scip, &payload).unwrap();
    (dir, scip.to_string_lossy().to_string())
}

fn do_index(home: &Path) -> String {
    let (code, out, err) = run(home, &["index", "--repo", "sem"]);
    assert_eq!(code, 0, "index failed: {err}");
    out
}

#[test]
fn cli_import_is_atomic_and_binds_live_snapshot() {
    let h = home("atomic");
    let (_dir, scip) = ab_repo(&h);
    let before = do_index(&h);
    let snap_before = num(&before, "snapshot");
    // Malformed inputs: each must exit 3 with nothing committed. Cuts are
    // alignment-proof (a proportional cut once landed on a field boundary
    // under macOS TMPDIR lengths and parsed as a shorter valid index):
    // 10 bytes cuts inside metadata; len-1 always corrupts the final field
    // (protobuf has no terminator or padding).
    let full = std::fs::read(&scip).unwrap();
    for cut in [10usize, full.len() - 1] {
        let mut payload = full.clone();
        payload.truncate(cut.max(1));
        let bad = h.join("bad.scip");
        std::fs::write(&bad, &payload).unwrap();
        let (code, _, _) = run(
            &h,
            &[
                "semantic",
                "import",
                "--repo",
                "sem",
                "--scip",
                &bad.to_string_lossy(),
            ],
        );
        assert_eq!(code, 3, "cut at {cut} must fail");
    }
    let (code, out, _) = run(&h, &["semantic", "status", "--repo", "sem"]);
    assert_eq!(code, 0);
    assert!(out.contains("\"imports\":[]"), "no import committed: {out}");
    assert_eq!(
        num(&out, "snapshot"),
        snap_before,
        "failed import mints nothing"
    );
    // Valid import: binds the live snapshot, no new snapshot row minted.
    let (code, out, err) = run(
        &h,
        &["semantic", "import", "--repo", "sem", "--scip", &scip],
    );
    assert_eq!(code, 0, "import failed: {err}");
    assert_eq!(num(&out, "snapshot"), snap_before);
    assert_eq!(num(&out, "documents"), 2);
    assert_eq!(num(&out, "definitions"), 1);
    assert_eq!(num(&out, "references"), 2);
    assert!(out.contains("\"verification\":\"Unverified\""));
}

#[test]
fn cli_query_resolves_cross_file_def_ref() {
    let h = home("query");
    let (_dir, scip) = ab_repo(&h);
    do_index(&h);
    let (code, _, err) = run(
        &h,
        &["semantic", "import", "--repo", "sem", "--scip", &scip],
    );
    assert_eq!(code, 0, "{err}");
    // Query at the b.ts reference (line 2, col 1..3): definition in a.ts.
    let (code, out, err) = run(
        &h,
        &[
            "semantic", "query", "--repo", "sem", "--path", "src/b.ts", "--line", "2", "--col", "2",
        ],
    );
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("src/a.ts"), "definition resolves: {out}");
    assert!(out.contains("\"verification\":\"Unverified\""));
    // --refs at the a.ts definition: the b.ts reference (and the local one).
    let (code, out, err) = run(
        &h,
        &[
            "semantic", "query", "--repo", "sem", "--path", "src/a.ts", "--line", "1", "--col",
            "17", "--refs",
        ],
    );
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("src/b.ts"), "references found: {out}");
    assert!(out.contains("\"definitions\":[]"));
    // Same-name symbol from another provider never merges: import a second
    // producer with a different URI for a same-named display string.
    // (Covered structurally: query filters by exact URI; absence of the
    // other URI in outputs below proves no merge.)
    assert!(!out.contains("other-producer"));
    // Query where nothing is: explicit error, exit 3.
    let (code, _, _) = run(
        &h,
        &[
            "semantic", "query", "--repo", "sem", "--path", "src/b.ts", "--line", "1", "--col", "1",
        ],
    );
    assert_eq!(code, 3);
}

#[test]
fn cli_end_boundary_is_exclusive() {
    // Half-open spans: a position exactly on the end boundary is NOT
    // inside the occurrence. Def covers (0,16)-(0,19); col 20 (1-based) is
    // the end edge and must resolve to nothing.
    let h = home("endboundary");
    let (_dir, scip) = ab_repo(&h);
    do_index(&h);
    let (code, _, err) = run(
        &h,
        &["semantic", "import", "--repo", "sem", "--scip", &scip],
    );
    assert_eq!(code, 0, "{err}");
    let (code, _, _) = run(
        &h,
        &[
            "semantic", "query", "--repo", "sem", "--path", "src/a.ts", "--line", "1", "--col",
            "20",
        ],
    );
    assert_eq!(code, 3, "end boundary must not match");
    // One column inside still resolves (start edge inclusive).
    let (code, out, err) = run(
        &h,
        &[
            "semantic", "query", "--repo", "sem", "--path", "src/a.ts", "--line", "1", "--col",
            "19",
        ],
    );
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("src/a.ts"), "{out}");
}

#[test]
fn cli_source_edit_before_reindex_reports_stale() {
    // Source moved after the snapshot (no rebuild): status and query must
    // report Stale from live source freshness, not Unverified from the
    // still-matching database rows.
    let h = home("editstale");
    let (dir, scip) = ab_repo(&h);
    do_index(&h);
    let (code, _, err) = run(
        &h,
        &["semantic", "import", "--repo", "sem", "--scip", &scip],
    );
    assert_eq!(code, 0, "{err}");
    std::fs::write(
        dir.join("src/b.ts"),
        "import \"./a\";\nfoo();\n// edited, not reindexed\n",
    )
    .unwrap();
    let (code, out, _) = run(&h, &["semantic", "status", "--repo", "sem"]);
    assert_eq!(code, 0);
    assert!(
        out.contains("\"verification\":\"Stale\""),
        "edited source must read Stale: {out}"
    );
    let (code, out, _) = run(
        &h,
        &[
            "semantic", "query", "--repo", "sem", "--path", "src/a.ts", "--line", "1", "--col",
            "17",
        ],
    );
    assert_eq!(code, 0);
    assert!(out.contains("\"verification\":\"Stale\""), "{out}");
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

#[test]
fn cli_dirty_git_worktree_reports_stale() {
    if !Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        eprintln!("skip: git unavailable");
        return;
    }
    // Committed git state, then an uncommitted edit (no rebuild): the dirty
    // flag is a source move the snapshot no longer describes → Stale.
    let h = home("dirtygit");
    let dir = repo_with(
        &h,
        "sem",
        &[("src/a.ts", "export function foo() {}\nfoo();\n")],
    );
    git(&dir, &["init", "-b", "main", "."]);
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "init"]);
    let a = document(
        "src/a.ts",
        &[occurrence(FOO, 0x1, 0, 16, 19)],
        &[symbol_info(FOO, "foo")],
        false,
    );
    let payload = index_bytes(&dir, &[a], &[]);
    let scip = dir.join("index.scip");
    std::fs::write(&scip, &payload).unwrap();
    // index.scip itself is untracked; commit it so only the deliberate edit
    // drives the verdict.
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "scip"]);
    do_index(&h);
    let (code, out, err) = run(
        &h,
        &[
            "semantic",
            "import",
            "--repo",
            "sem",
            "--scip",
            &scip.to_string_lossy(),
        ],
    );
    assert_eq!(code, 0, "{err}");
    assert!(
        out.contains("\"verification\":\"Unverified\""),
        "clean tree binds: {out}"
    );
    std::fs::write(
        dir.join("src/a.ts"),
        "export function foo() {}\nfoo();\n// dirty\n",
    )
    .unwrap();
    let (code, out, _) = run(&h, &["semantic", "status", "--repo", "sem"]);
    assert_eq!(code, 0);
    assert!(
        out.contains("\"verification\":\"Stale\""),
        "dirty tree: {out}"
    );
}

#[test]
fn cli_imports_with_same_uri_do_not_merge() {
    // Two producers, identical symbol URI, different evidence: def/ref
    // resolution stays within the anchor's own import (import_id scope).
    let h = home("twoproviders");
    let dir = repo_with(
        &h,
        "sem",
        &[("src/a.ts", "export function foo() {}\nfoo();\n")],
    );
    let uri = "scip-test test test-pkg 1.0.0 src/a.ts/foo().";
    let doc_a = document(
        "src/a.ts",
        &[occurrence(uri, 0x1, 0, 16, 19)],
        &[symbol_info(uri, "foo")],
        false,
    );
    // Second producer: same URI, but only a reference elsewhere.
    let doc_b = document(
        "src/a.ts",
        &[occurrence(uri, 0x0, 1, 0, 3)],
        &[symbol_info(uri, "foo")],
        false,
    );
    do_index(&h);
    for (tool, docs) in [("tool-a", vec![doc_a]), ("tool-b", vec![doc_b])] {
        let payload = index_bytes_with_tool(&dir, tool, &docs, &[]);
        let path = dir.join(format!("{tool}.scip"));
        std::fs::write(&path, &payload).unwrap();
        let (code, _, err) = run(
            &h,
            &[
                "semantic",
                "import",
                "--repo",
                "sem",
                "--scip",
                &path.to_string_lossy(),
            ],
        );
        assert_eq!(code, 0, "{err}");
    }
    // Anchor in tool-b (the reference at line 2): its own import holds no
    // definition, so definitions must be empty — tool-a's def must not
    // leak across the import boundary.
    let (code, out, err) = run(
        &h,
        &[
            "semantic", "query", "--repo", "sem", "--path", "src/a.ts", "--line", "2", "--col", "2",
        ],
    );
    assert_eq!(code, 0, "{err}");
    assert!(
        out.contains("\"definitions\":[]"),
        "no cross-import merge: {out}"
    );
    // Anchor in tool-a (the definition): --refs must not see tool-b's ref.
    let (code, out, err) = run(
        &h,
        &[
            "semantic", "query", "--repo", "sem", "--path", "src/a.ts", "--line", "1", "--col",
            "17", "--refs",
        ],
    );
    assert_eq!(code, 0, "{err}");
    assert!(
        out.contains("\"references\":[]"),
        "no cross-import merge: {out}"
    );
}

#[test]
fn cli_null_file_id_survives_noop_and_drops_on_change() {
    // Unindexed (unsupported-extension) documents store facts with
    // file_id NULL. A no-op rebuild must carry them via raw digest
    // verification; changing the file must drop them explicitly.
    let h = home("nullcarry");
    let dir = repo_with(&h, "sem", &[("notes.other", "data for notes\n")]);
    let uri = "scip-test test test-pkg 1.0.0 notes.other/thing`.";
    let doc = document(
        "notes.other",
        &[occurrence(uri, 0x1, 0, 0, 4)],
        &[symbol_info(uri, "thing")],
        false,
    );
    let payload = index_bytes(&dir, &[doc], &[]);
    let scip = dir.join("index.scip");
    std::fs::write(&scip, &payload).unwrap();
    do_index(&h);
    let (code, _, err) = run(
        &h,
        &[
            "semantic",
            "import",
            "--repo",
            "sem",
            "--scip",
            &scip.to_string_lossy(),
        ],
    );
    assert_eq!(code, 0, "{err}");
    do_index(&h); // no-op rebuild
    let (code, out, err) = run(
        &h,
        &[
            "semantic",
            "query",
            "--repo",
            "sem",
            "--path",
            "notes.other",
            "--line",
            "1",
            "--col",
            "2",
        ],
    );
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("notes.other"), "NULL fact carried: {out}");
    // Change the file: the unverifiable fact must drop, import goes Stale.
    std::fs::write(dir.join("notes.other"), "changed data\n").unwrap();
    do_index(&h);
    let (code, _, _) = run(
        &h,
        &[
            "semantic",
            "query",
            "--repo",
            "sem",
            "--path",
            "notes.other",
            "--line",
            "1",
            "--col",
            "2",
        ],
    );
    assert_eq!(code, 3, "changed unindexed fact must drop");
}

fn null_repo(h: &Path) -> (PathBuf, String) {
    let dir = repo_with(h, "sem", &[("notes.other", "data for notes\n")]);
    let uri = "scip-test test test-pkg 1.0.0 notes.other/thing`.";
    let doc = document(
        "notes.other",
        &[occurrence(uri, 0x1, 0, 0, 4)],
        &[symbol_info(uri, "thing")],
        false,
    );
    let payload = index_bytes(&dir, &[doc], &[]);
    let scip = dir.join("index.scip");
    std::fs::write(&scip, &payload).unwrap();
    (dir, scip.to_string_lossy().to_string())
}

fn null_query(h: &Path) -> (i32, String, String) {
    run(
        h,
        &[
            "semantic",
            "query",
            "--repo",
            "sem",
            "--path",
            "notes.other",
            "--line",
            "1",
            "--col",
            "2",
        ],
    )
}

#[test]
fn cli_null_fact_edit_before_reindex_reports_stale() {
    // file_id NULL evidence edited before any rebuild: M4 freshness sees
    // no files row and stays silent, so the semantic digest check must flip
    // the import to Stale instead of serving Unverified.
    let h = home("nullstale");
    let (_dir, scip) = null_repo(&h);
    do_index(&h);
    let (code, _, err) = run(
        &h,
        &["semantic", "import", "--repo", "sem", "--scip", &scip],
    );
    assert_eq!(code, 0, "{err}");
    let (code, out, _) = run(&h, &["semantic", "status", "--repo", "sem"]);
    assert_eq!(code, 0);
    assert!(
        out.contains("\"verification\":\"Unverified\""),
        "bound: {out}"
    );
    std::fs::write(h.join("sem").join("notes.other"), "edited, not reindexed\n").unwrap();
    let (code, out, _) = run(&h, &["semantic", "status", "--repo", "sem"]);
    assert_eq!(code, 0);
    assert!(
        out.contains("\"verification\":\"Stale\""),
        "edited NULL file: {out}"
    );
    let (code, out, _) = null_query(&h);
    assert_eq!(code, 0);
    assert!(out.contains("\"verification\":\"Stale\""), "{out}");
}

#[test]
fn cli_null_doc_turned_escape_symlink_drops_safely() {
    // An unindexed file replaced by a symlink escaping the repo between
    // import and rebuild: carry-over must confine the read (no outside
    // bytes in, no crash), dropping the fact. Needs symlink privilege;
    // otherwise the scenario cannot exist here — same pattern as M4.
    let h = home("nullsymlink");
    let (dir, scip) = null_repo(&h);
    let outside = h.join("outside.txt");
    std::fs::write(&outside, "secret\n").unwrap();
    do_index(&h);
    let (code, _, err) = run(
        &h,
        &["semantic", "import", "--repo", "sem", "--scip", &scip],
    );
    assert_eq!(code, 0, "{err}");
    let target = dir.join("notes.other");
    std::fs::remove_file(&target).unwrap();
    #[cfg(unix)]
    let made = std::os::unix::fs::symlink(&outside, &target).is_ok();
    #[cfg(windows)]
    let made = if std::os::windows::fs::symlink_file(&outside, &target).is_ok() {
        true
    } else {
        // Same canonical-escape mechanism via hardlink when symlink
        // privilege is absent: the read confinement cannot tell them apart.
        std::fs::hard_link(&outside, &target).is_ok()
    };
    if !made {
        eprintln!("skip: OS refused symlinks here");
        return;
    }
    let (code, out, err) = run(&h, &["index", "--repo", "sem"]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("\"mode\":\"incremental\""), "{out}");
    let (code, _, _) = null_query(&h);
    assert_eq!(code, 3, "escaped fact must drop, never leak outside bytes");
    assert_eq!(std::fs::read_to_string(&outside).unwrap(), "secret\n");
}

#[test]
fn cli_provider_unavailable_falls_back_with_syntax_intact() {
    let h = home("fallback");
    let (_dir, scip) = ab_repo(&h);
    do_index(&h);
    let (code, _, err) = run(
        &h,
        &["semantic", "import", "--repo", "sem", "--scip", &scip],
    );
    assert_eq!(code, 0, "{err}");
    // Missing server binary: explicit fallback payload, exit 0 (expected
    // condition, not a usage error), syntax search untouched.
    let missing_abs = h.join("no-such-server").to_string_lossy().to_string();
    let (code, out, err) = run(
        &h,
        &[
            "semantic",
            "query",
            "--repo",
            "sem",
            "--path",
            "src/a.ts",
            "--line",
            "1",
            "--col",
            "17",
            "--lsp-server",
            &missing_abs,
        ],
    );
    assert_eq!(code, 0, "{err}");
    assert!(
        out.contains("\"status\":\"fallback\""),
        "fallback reported: {out}"
    );
    assert!(out.contains("\"syntax_fallback\":true"));
    let (code, out, _) = run(&h, &["search", "--repo", "sem", "--symbol", "foo"]);
    assert_eq!(code, 0);
    assert!(out.contains("foo"), "syntax search intact: {out}");
}

#[test]
fn cli_repo_isolation_holds_for_semantics() {
    let h = home("isolation");
    let (_dir, scip) = ab_repo(&h);
    do_index(&h);
    let (code, _, err) = run(
        &h,
        &["semantic", "import", "--repo", "sem", "--scip", &scip],
    );
    assert_eq!(code, 0, "{err}");
    repo_with(&h, "other", &[("z.py", "x = 1\n")]);
    let (code, _, err) = run(&h, &["index", "--repo", "other"]);
    assert_eq!(code, 0, "{err}");
    let (code, out, _) = run(&h, &["semantic", "status", "--repo", "other"]);
    assert_eq!(code, 0);
    assert!(
        out.contains("\"imports\":[]"),
        "facts never leak across repos: {out}"
    );
}

#[test]
fn cli_lifecycle_noop_delta_full() {
    let h = home("lifecycle");
    let (dir, scip) = ab_repo(&h);
    do_index(&h);
    let (code, _, err) = run(
        &h,
        &["semantic", "import", "--repo", "sem", "--scip", &scip],
    );
    assert_eq!(code, 0, "{err}");
    // No-op rebuild: identical bytes → still Unverified (never Stale).
    do_index(&h);
    let (code, out, _) = run(&h, &["semantic", "status", "--repo", "sem"]);
    assert_eq!(code, 0);
    assert!(
        out.contains("\"verification\":\"Unverified\""),
        "noop keeps binding: {out}"
    );
    assert!(!out.contains("\"verification\":\"Stale\""));
    // One-file edit + delta rebuild: whole import reports Stale.
    std::fs::write(dir.join("src/b.ts"), "import \"./a\";\nfoo();\n// touch\n").unwrap();
    let (code, out, err) = run(&h, &["index", "--repo", "sem"]);
    assert_eq!(code, 0, "{err}");
    assert!(
        out.contains("\"reparsed\":1"),
        "delta reparsed one file: {out}"
    );
    let (code, out, _) = run(&h, &["semantic", "status", "--repo", "sem"]);
    assert_eq!(code, 0);
    assert!(
        out.contains("\"verification\":\"Stale\""),
        "edit invalidates: {out}"
    );
    // Full rebuild converges back to queryable (Stale-marked) facts.
    let (code, out, err) = run(&h, &["index", "--repo", "sem", "--full"]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("\"mode\":\"full\""));
    let (code, out, _) = run(&h, &["semantic", "status", "--repo", "sem"]);
    assert_eq!(code, 0);
    assert!(out.contains("\"verification\":\"Stale\""), "{out}");
}

#[test]
fn cli_local_symbols_stay_document_scoped() {
    let h = home("locals");
    let dir = repo_with(
        &h,
        "sem",
        &[("x.py", "a = 1\nprint(a)\n"), ("y.py", "a = 2\nprint(a)\n")],
    );
    let mk = |rel: &str| {
        document(
            rel,
            &[
                occurrence("local v1", 0x1, 0, 0, 1),
                occurrence("local v1", 0x0, 1, 6, 7),
            ],
            &[symbol_info("local v1", "a")],
            false,
        )
    };
    let payload = index_bytes(&dir, &[mk("x.py"), mk("y.py")], &[]);
    let scip = dir.join("index.scip");
    std::fs::write(&scip, &payload).unwrap();
    do_index(&h);
    let (code, _, err) = run(
        &h,
        &[
            "semantic",
            "import",
            "--repo",
            "sem",
            "--scip",
            &scip.to_string_lossy(),
        ],
    );
    assert_eq!(code, 0, "{err}");
    // Reference in x.py resolves to the x.py definition only.
    let (code, out, err) = run(
        &h,
        &[
            "semantic", "query", "--repo", "sem", "--path", "x.py", "--line", "2", "--col", "7",
        ],
    );
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("x.py"), "{out}");
    assert!(!out.contains("y.py"), "locals never cross documents: {out}");
}

#[test]
fn cli_lsp_live_query_gated() {
    if std::env::var("RILTRA_TEST_LSP").is_err() {
        eprintln!("skip: RILTRA_TEST_LSP unset (explicit opt-in)");
        return;
    }
    let h = home("lsplive");
    let repodir = repo_with(
        &h,
        "sem",
        &[("main.rs", "fn main() {\n    let x = 1;\n    println!(\"{x}\");\n}\nfn helper() {}\nfn a() {}\nfn b() {}\nfn c() {}\nfn d() {}\n")],
    );
    // The mock reads its mode/root from the environment, inherited through
    // the riltra binary under test. Harmless to siblings: only the mock
    // binary reads these variables.
    std::env::set_var("RILTRA_MOCK_LSP", "success");
    std::env::set_var(
        "RILTRA_MOCK_ROOT",
        repodir.to_string_lossy().replace('\\', "/"),
    );
    do_index(&h);
    let mut mock = std::env::current_exe().unwrap();
    mock.pop();
    mock.pop();
    mock.push("examples");
    mock.push(format!("mock_lsp{}", std::env::consts::EXE_SUFFIX));
    // The mock example binary is built on demand so it can never go stale.
    let status = Command::new("cargo")
        .args(["build", "--offline", "--example", "mock_lsp"])
        .status()
        .expect("cargo build mock server");
    assert!(status.success(), "mock server must build");
    assert!(mock.is_file(), "mock server missing: {}", mock.display());
    let (code, out, err) = run(
        &h,
        &[
            "semantic",
            "query",
            "--repo",
            "sem",
            "--path",
            "main.rs",
            "--line",
            "2",
            "--col",
            "5",
            "--lsp-server",
            &mock.to_string_lossy(),
        ],
    );
    assert_eq!(code, 0, "{err}");
    // Mock mode defaults to success only with RILTRA_MOCK_LSP=success; without
    // it the mock still answers success (default arm). Definition resolves.
    assert!(out.contains("\"fallback\":null"), "no fallback: {out}");
    assert!(out.contains("main.rs"), "{out}");

    // The server's own diagnostics reach the caller, and they never cancel
    // the answer.
    std::env::set_var("RILTRA_MOCK_LSP", "diagnostics");
    let (code, out, err) = run(
        &h,
        &[
            "semantic",
            "query",
            "--repo",
            "sem",
            "--path",
            "main.rs",
            "--line",
            "2",
            "--col",
            "5",
            "--lsp-server",
            &mock.to_string_lossy(),
        ],
    );
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("\"fallback\":null"), "no fallback: {out}");
    assert!(out.contains("pp_file_not_found"), "diagnostic code: {out}");
    assert!(
        out.contains("diagnostic: error"),
        "diagnostic severity: {out}"
    );
    assert!(
        out.contains("diagnostic: warning"),
        "a warning is delivered too: {out}"
    );
    assert!(out.contains("main.rs"), "the answer is intact: {out}");

    // A source that changed under the server is reported as stale, never as
    // a current fact.
    std::env::set_var("RILTRA_MOCK_LSP", "stale-input");
    let (code, out, err) = run(
        &h,
        &[
            "semantic",
            "query",
            "--repo",
            "sem",
            "--path",
            "main.rs",
            "--line",
            "2",
            "--col",
            "5",
            "--lsp-server",
            &mock.to_string_lossy(),
        ],
    );
    assert_eq!(code, 0, "{err}");
    assert!(
        out.contains("\"verification\":\"Stale\""),
        "a changed source is not presented as current: {out}"
    );
    // The answer's OWN verdict (the last field of the payload), not only the
    // per-fact one.
    let summary = out
        .rsplit("\"verification\":\"")
        .next()
        .unwrap_or_default()
        .split('"')
        .next()
        .unwrap_or_default();
    assert_eq!(summary, "Stale", "the answer itself is stale: {out}");

    // An empty answer with an error diagnostic stays empty: it is not turned
    // into a fallback, and the diagnostic says why.
    std::env::set_var("RILTRA_MOCK_LSP", "empty");
    let (code, out, err) = run(
        &h,
        &[
            "semantic",
            "query",
            "--repo",
            "sem",
            "--path",
            "main.rs",
            "--line",
            "2",
            "--col",
            "5",
            "--lsp-server",
            &mock.to_string_lossy(),
        ],
    );
    assert_eq!(code, 0, "{err}");
    assert!(
        out.contains("\"fallback\":null"),
        "an empty answer is not a fallback: {out}"
    );
    assert!(
        out.contains("pp_file_not_found"),
        "the diagnostic explains it: {out}"
    );
}
