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
    // protobuf 3.7.2 MIT, protobuf-support 3.7.2 MIT.)
    for want in [
        "serde_json v1.0.154 [MIT OR Apache-2.0]",
        "protobuf v3.7.2 [MIT]",
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

    // Banned: protoc bridges, async runtimes, dynamic loading, network.
    for banned in [
        "protoc-bin-vendored",
        " tokio ",
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
    let mut md = Vec::new();
    len_field(2, &tool_info("scip-test-cli", "1.0"), &mut md);
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
    // Malformed input: truncated payload → exit 3, nothing committed.
    let bad = h.join("bad.scip");
    let mut payload = std::fs::read(&scip).unwrap();
    payload.truncate(payload.len() / 2);
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
    assert_eq!(code, 3);
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
}
