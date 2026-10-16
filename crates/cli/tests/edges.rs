// SPDX-License-Identifier: Apache-2.0
//! M6 graph lifecycle CLI tests (Task 3): import rollback, incremental
//! modes, and re-import after rebuild. Edge *content* is asserted in
//! `src/edges.rs` unit tests (in-crate store access); these tests pin the
//! CLI-observable lifecycle the graph relies on. Query commands arrive in
//! Task 4 — no callers/callees assertions here.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(20_000);

fn home(tag: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let mut p = std::env::temp_dir();
    p.push(format!("riltra-edges-{tag}-{}-{n}", std::process::id()));
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
    let (code, _, err) = run(
        home,
        &["init", "--repo", &d.to_string_lossy(), "--name", name],
    );
    assert_eq!(code, 0, "init failed: {err}");
    d
}

fn do_index(home: &Path) -> String {
    let (code, out, err) = run(home, &["index", "--repo", "sem"]);
    assert_eq!(code, 0, "index failed: {err}");
    out
}

// Minimal proto3 writer for exactly the fields the importer reads (field
// numbers pinned by scip.proto; same shape as tests/semantic.rs).
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

fn occurrence(symbol: &str, roles: u64, line: i64, start: i64, end: i64) -> Vec<u8> {
    let mut r = Vec::new();
    varint_field(1, line as u64, &mut r);
    varint_field(2, start as u64, &mut r);
    varint_field(3, end as u64, &mut r);
    let mut o = Vec::new();
    str_field(2, symbol, &mut o);
    varint_field(3, roles, &mut o);
    len_field(8, &r, &mut o);
    o
}

const T_URI: &str = "scip-test test test-pkg 1.0.0 lib.py/target().";
const C_URI: &str = "scip-test test test-pkg 1.0.0 main.py/caller().";

fn scip_for(dir: &Path) -> String {
    let mut sym = Vec::new();
    str_field(1, T_URI, &mut sym);
    str_field(6, "target", &mut sym);
    let lib_occ = occurrence(T_URI, 0x1, 0, 4, 10);
    let mut doc_lib_full = Vec::new();
    str_field(1, "lib.py", &mut doc_lib_full);
    len_field(2, &lib_occ, &mut doc_lib_full);
    len_field(3, &sym, &mut doc_lib_full);
    varint_field(6, 1, &mut doc_lib_full);

    let mut sym_c = Vec::new();
    str_field(1, C_URI, &mut sym_c);
    str_field(6, "caller", &mut sym_c);
    let mut doc_main = Vec::new();
    str_field(1, "main.py", &mut doc_main);
    for o in [
        occurrence(C_URI, 0x1, 2, 4, 10),
        occurrence(T_URI, 0x0, 3, 11, 17),
    ] {
        len_field(2, &o, &mut doc_main);
    }
    len_field(3, &sym_c, &mut doc_main);
    len_field(3, &sym, &mut doc_main);
    varint_field(6, 1, &mut doc_main);

    let mut md = Vec::new();
    let mut tool = Vec::new();
    str_field(1, "scip-test", &mut tool);
    str_field(2, "1.0", &mut tool);
    len_field(2, &tool, &mut md);
    str_field(
        3,
        &format!("file://{}", dir.to_string_lossy().replace('\\', "/")),
        &mut md,
    );
    varint_field(4, 1, &mut md);
    let mut out = Vec::new();
    len_field(1, &md, &mut out);
    len_field(2, &doc_lib_full, &mut out);
    len_field(2, &doc_main, &mut out);
    let scip = dir.join("index.scip");
    std::fs::write(&scip, &out).unwrap();
    scip.to_string_lossy().to_string()
}

fn edge_repo(h: &Path) -> PathBuf {
    repo_with(
        h,
        "sem",
        &[
            ("lib.py", "def target():\n    return 1\n"),
            (
                "main.py",
                "from lib import target\n\ndef caller():\n    return target()\n",
            ),
        ],
    )
}

#[test]
fn cli_graph_import_failure_rolls_back() {
    let h = home("rollback");
    edge_repo(&h);
    do_index(&h);
    let bad = h.join("sem").join("bad.scip");
    std::fs::write(&bad, b"not a scip payload").unwrap();
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
    assert_ne!(code, 0, "corrupt SCIP must fail the import");
    let (code, out, _) = run(&h, &["semantic", "status", "--repo", "sem"]);
    assert_eq!(code, 0);
    assert!(
        out.contains("\"imports\":[]"),
        "failed import stores nothing: {out}"
    );
}

#[test]
fn cli_graph_incremental_modes_hold_with_edges() {
    let h = home("modes");
    edge_repo(&h);
    do_index(&h);
    let (code, out, err) = run(&h, &["index", "--repo", "sem"]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("\"mode\":\"incremental\""), "{out}");
    std::fs::write(
        h.join("sem").join("lib.py"),
        "def target():\n    return 2\n",
    )
    .unwrap();
    let (code, out, err) = run(&h, &["index", "--repo", "sem"]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("\"mode\":\"incremental\""), "{out}");
    let (code, out, err) = run(&h, &["index", "--repo", "sem", "--full"]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("\"mode\":\"full\""), "{out}");
}

#[test]
fn cli_graph_import_after_rebuild_rebinds() {
    // Import → rebuild (facts carry) → re-import on the new snapshot: the
    // lifecycle the graph re-derivation relies on, CLI-observed.
    let h = home("rebind");
    let dir = edge_repo(&h);
    do_index(&h);
    let scip = scip_for(&dir);
    let (code, _, err) = run(
        &h,
        &["semantic", "import", "--repo", "sem", "--scip", &scip],
    );
    assert_eq!(code, 0, "{err}");
    std::fs::write(dir.join("lib.py"), "def target():\n    return 2\n").unwrap();
    let (code, _, err) = run(&h, &["index", "--repo", "sem"]);
    assert_eq!(code, 0, "{err}");
    let (code, _, err) = run(
        &h,
        &["semantic", "import", "--repo", "sem", "--scip", &scip],
    );
    assert_eq!(code, 0, "re-import on the new snapshot: {err}");
    let (code, out, _) = run(&h, &["semantic", "status", "--repo", "sem"]);
    assert_eq!(code, 0);
    assert!(out.contains("\"verification\""), "{out}");
}
