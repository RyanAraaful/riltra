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

/// A C++ header with a data member, a method declaration and a
/// function-pointer member. The data member is on line 3, its identifier
/// starts at column 10; the class spans the bytes 0..68.
const CPP_FIELDS: &str =
    "class Options {\n public:\n  double max_triangulation_angle = -1.0f;\n};\n";

#[test]
fn a_cpp_data_member_is_searchable_as_a_field() {
    let h = home("field-search");
    let repo = mkrepo(&h, "work");
    std::fs::write(repo.join("options.h"), CPP_FIELDS).unwrap();
    init_and_index(&h, &repo, "r");
    let (code, out, err) = run(
        &h,
        &[
            "search",
            "--repo",
            "r",
            "--symbol",
            "max_triangulation_angle",
        ],
    );
    assert_eq!(code, 0, "search failed: {err}");
    assert!(out.contains("\"kind\":\"field\""), "got: {out}");
    assert!(
        out.contains("\"name\":\"max_triangulation_angle\""),
        "got: {out}"
    );
    assert!(out.contains("options.h"), "got: {out}");
    // The symbol points at its NAME: line 3, column 10.
    assert!(out.contains("\"start_line\":3"), "got: {out}");
    assert!(out.contains("\"start_col\":10"), "got: {out}");
}

#[test]
fn a_cpp_method_or_function_pointer_member_is_not_a_field() {
    let h = home("field-exclude");
    let repo = mkrepo(&h, "work");
    std::fs::write(
        repo.join("options.h"),
        "class Options {\n public:\n  int process(int x);\n\n  int (*callback)();\n};\n",
    )
    .unwrap();
    init_and_index(&h, &repo, "r");
    for name in ["process", "callback"] {
        let (code, out, err) = run(&h, &["search", "--repo", "r", "--symbol", name]);
        assert_eq!(code, 0, "search failed: {err}");
        assert!(
            !out.contains("\"kind\":\"field\""),
            "{name} must not be indexed as a data member: {out}"
        );
    }
}

fn edge_doc(h: &Path, command: &str, path: &str, line: &str, col: &str) -> serde_json::Value {
    let (code, out, err) = run(
        h,
        &[
            command, "--repo", "r", "--path", path, "--line", line, "--col", col,
        ],
    );
    assert_eq!(code, 0, "{command} failed: {err}");
    serde_json::from_str(&out).unwrap_or_else(|e| panic!("not JSON ({e}): {out}"))
}

#[test]
fn a_position_on_a_data_member_name_anchors_the_member_not_its_class() {
    // The anchor of a position on a data member's NAME is the member itself,
    // so callers and impact answer about the member and never about the
    // enclosing class (the class spans 0..68, the member name 34..57).
    let h = home("field-anchor");
    let repo = mkrepo(&h, "work");
    std::fs::write(repo.join("options.h"), CPP_FIELDS).unwrap();
    init_and_index(&h, &repo, "r");
    for command in ["callers", "callees"] {
        let doc = edge_doc(&h, command, "options.h", "3", "10");
        assert_eq!(
            doc["anchor"]["span"],
            serde_json::json!([34, 57]),
            "{command}: {doc}"
        );
        assert_eq!(doc["anchor"]["endpoint"]["byte_start"], 34, "{command}");
    }
    // The anchor says what it is and where the references are listed.
    let doc = edge_doc(&h, "callers", "options.h", "3", "10");
    let note = doc["anchor"]["anchor_uncertainty"].as_str().unwrap();
    assert!(
        note.contains("data member 'max_triangulation_angle'"),
        "{note}"
    );
    assert!(note.contains("search --occurrences"), "{note}");
}

#[test]
fn a_position_in_a_class_body_outside_any_member_name_still_anchors_the_class() {
    let h = home("field-anchor-class");
    let repo = mkrepo(&h, "work");
    std::fs::write(repo.join("options.h"), CPP_FIELDS).unwrap();
    init_and_index(&h, &repo, "r");
    // ` public:` (line 2) and the type of the member (line 3, column 3) are
    // inside the class but on no member name.
    for (line, col) in [("2", "2"), ("3", "4")] {
        let doc = edge_doc(&h, "callers", "options.h", line, col);
        assert_eq!(
            doc["anchor"]["span"],
            serde_json::json!([0, 68]),
            "{line}:{col}"
        );
        assert_eq!(
            doc["anchor"]["anchor_uncertainty"],
            "syntax-only anchor: no semantic identity at this position"
        );
    }
}

#[test]
fn impact_at_a_data_member_is_unknown_and_anchored_at_the_member() {
    let h = home("field-anchor-impact");
    let repo = mkrepo(&h, "work");
    std::fs::write(repo.join("options.h"), CPP_FIELDS).unwrap();
    std::fs::write(repo.join("use.cc"), OCC_USE_CC).unwrap();
    init_and_index(&h, &repo, "r");
    let doc = edge_doc(&h, "impact", "options.h", "3", "10");
    assert_eq!(doc["verdict"], "UNKNOWN");
    assert_eq!(doc["anchor"]["span"], serde_json::json!([34, 57]));
    assert_eq!(doc["completeness"]["status"], "partial");
    assert_eq!(doc["paths"], serde_json::json!([]));
    assert!(
        !doc.to_string().contains("\"occurrences\""),
        "the graph carries no occurrence data, only the pointer in the note: {doc}"
    );
    assert!(doc["anchor"]["anchor_uncertainty"]
        .as_str()
        .unwrap()
        .contains("search --occurrences"));
}

#[test]
fn callers_of_a_data_member_never_list_a_same_named_call() {
    // A member is not callable: a call named like the member (here the method
    // `Box::value`) is no caller of the member `Holder::value`.
    let h = home("field-anchor-nocall");
    let repo = mkrepo(&h, "work");
    std::fs::write(
        repo.join("types.h"),
        "struct Holder {\n  int value;\n};\nstruct Box {\n  int value() const;\n};\n",
    )
    .unwrap();
    std::fs::write(
        repo.join("use.cc"),
        "#include \"types.h\"\nint f(Box& b) {\n  return b.value();\n}\n",
    )
    .unwrap();
    init_and_index(&h, &repo, "r");
    let doc = edge_doc(&h, "callers", "types.h", "2", "7");
    assert_eq!(doc["anchor"]["span"], serde_json::json!([22, 27]), "{doc}");
    assert_eq!(doc["edges"], serde_json::json!([]), "{doc}");
    assert_eq!(doc["completeness"]["total"], 0);
}

#[test]
fn an_index_from_the_previous_parser_version_is_rebuilt() {
    let h = home("field-parser");
    let repo = mkrepo(&h, "work");
    std::fs::write(repo.join("options.h"), CPP_FIELDS).unwrap();
    init_and_index(&h, &repo, "r");
    // An index written by the previous parser version: the old parser
    // version, and no field facts in it.
    let db = h.join("index").join("r.db");
    {
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute("UPDATE snapshots SET parser_version = 1", [])
            .unwrap();
        conn.execute("DELETE FROM symbols WHERE kind = 'field'", [])
            .unwrap();
    }
    let (code, out, err) = run(
        &h,
        &[
            "search",
            "--repo",
            "r",
            "--symbol",
            "max_triangulation_angle",
        ],
    );
    assert_eq!(code, 0, "search failed: {err}");
    assert!(
        !out.contains("\"kind\":\"field\""),
        "the old index must not have field facts: {out}"
    );
    let (code, out, err) = run(&h, &["index", "status", "--repo", "r"]);
    assert_eq!(code, 0, "index status failed: {err}");
    assert!(out.contains("\"freshness\":\"stale\""), "got: {out}");
    assert!(out.contains("parser-changed"), "got: {out}");
    let (code, out, err) = run(&h, &["index", "--repo", "r"]);
    assert_eq!(code, 0, "index failed: {err}");
    assert!(out.contains("\"mode\":\"fallback-full\""), "got: {out}");
    assert!(out.contains("parser-changed"), "got: {out}");
    let (code, out, err) = run(
        &h,
        &[
            "search",
            "--repo",
            "r",
            "--symbol",
            "max_triangulation_angle",
        ],
    );
    assert_eq!(code, 0, "search failed: {err}");
    assert!(out.contains("\"kind\":\"field\""), "got: {out}");
}

#[test]
fn an_index_from_the_previous_parser_version_has_no_occurrences_until_it_is_rebuilt() {
    let h = home("occ-parser");
    let repo = mkrepo(&h, "work");
    std::fs::write(repo.join("options.h"), CPP_FIELDS).unwrap();
    std::fs::write(
        repo.join("use.cc"),
        "#include \"options.h\"\ndouble read(Options& o) {\n  return o.max_triangulation_angle;\n}\n",
    )
    .unwrap();
    init_and_index(&h, &repo, "r");
    let db = h.join("index").join("r.db");
    let occurrences = || -> i64 {
        rusqlite::Connection::open(&db)
            .unwrap()
            .query_row("SELECT COUNT(*) FROM reference_occurrences", [], |r| {
                r.get(0)
            })
            .unwrap()
    };
    assert_eq!(occurrences(), 1, "the member access is stored by the index");
    // An index written by the previous parser version has no occurrence rows.
    {
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute(
            "UPDATE snapshots SET parser_version = parser_version - 1",
            [],
        )
        .unwrap();
        conn.execute("DELETE FROM reference_occurrences", [])
            .unwrap();
    }
    assert_eq!(occurrences(), 0);
    let (code, out, err) = run(&h, &["index", "status", "--repo", "r"]);
    assert_eq!(code, 0, "index status failed: {err}");
    assert!(out.contains("\"freshness\":\"stale\""), "got: {out}");
    assert!(out.contains("parser-changed"), "got: {out}");
    let (code, out, err) = run(&h, &["index", "--repo", "r"]);
    assert_eq!(code, 0, "index failed: {err}");
    assert!(out.contains("\"mode\":\"fallback-full\""), "got: {out}");
    assert!(out.contains("parser-changed"), "got: {out}");
    assert_eq!(occurrences(), 1, "the rebuild stores the occurrence again");
}

/// Uses of the `CPP_FIELDS` member: a member access in a same-file macro
/// argument (line 4), the address of the member (line 5), a plain member read
/// (line 6) and a parameter of the same name passed to the macro (line 9).
const OCC_USE_CC: &str = "#include \"options.h\"\n#define PRINT(x) (x)\ndouble read(Options& o) {\n  PRINT(o.max_triangulation_angle);\n  double* p = &o.max_triangulation_angle;\n  return o.max_triangulation_angle;\n}\nvoid show(double max_triangulation_angle) {\n  PRINT(max_triangulation_angle);\n}\n";

fn occurrence_repo(tag: &str) -> (PathBuf, PathBuf) {
    let h = home(tag);
    let repo = mkrepo(&h, "work");
    std::fs::write(repo.join("options.h"), CPP_FIELDS).unwrap();
    std::fs::write(repo.join("use.cc"), OCC_USE_CC).unwrap();
    init_and_index(&h, &repo, "r");
    (h, repo)
}

fn occurrence_search(h: &Path, name: &str, extra: &[&str]) -> (String, serde_json::Value) {
    let mut args = vec!["search", "--repo", "r", "--symbol", name, "--occurrences"];
    args.extend_from_slice(extra);
    let (code, out, err) = run(h, &args);
    assert_eq!(code, 0, "search failed: {err}");
    let doc = serde_json::from_str(&out).unwrap_or_else(|e| panic!("not JSON ({e}): {out}"));
    (out, doc)
}

#[test]
fn search_occurrences_lists_each_use_with_its_role_and_the_ambiguity_counts() {
    let (h, repo) = occurrence_repo("occ-list");
    let (out, doc) = occurrence_search(&h, "max_triangulation_angle", &[]);
    // The section follows `truncated`, with a fixed key order.
    assert!(
        out.contains("\"truncated\":false,\"occurrences\":{\"scope\":\"repository-wide-exact-name\",\"binding\":\"unbound\",\"field_declarations\":1,\"symbol_declarations\":1,\"total\":4,\"truncated\":false,\"items\":[{\"path\":\"use.cc\",\"role\":\"member-access\",\"confidence\":\"observed\",\"span\":{\"start_line\":4,\"start_col\":11,"),
        "got: {out}"
    );
    let occ = &doc["occurrences"];
    let items: Vec<(String, String, String, u64, u64)> = occ["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| {
            (
                i["path"].as_str().unwrap().to_string(),
                i["role"].as_str().unwrap().to_string(),
                i["confidence"].as_str().unwrap().to_string(),
                i["span"]["start_line"].as_u64().unwrap(),
                i["span"]["start_col"].as_u64().unwrap(),
            )
        })
        .collect();
    let row = |role: &str, confidence: &str, line: u64, col: u64| {
        (
            "use.cc".to_string(),
            role.to_string(),
            confidence.to_string(),
            line,
            col,
        )
    };
    assert_eq!(
        items,
        vec![
            row("member-access", "observed", 4, 11),
            row("address-of-binding", "observed", 5, 18),
            row("member-access", "observed", 6, 12),
            row("macro-argument", "candidate", 9, 9),
        ]
    );
    // The recorded bytes address exactly the identifier.
    let source = std::fs::read(repo.join("use.cc")).unwrap();
    for item in occ["items"].as_array().unwrap() {
        let start = item["bytes"][0].as_u64().unwrap() as usize;
        let end = item["bytes"][1].as_u64().unwrap() as usize;
        assert_eq!(&source[start..end], b"max_triangulation_angle");
    }
    assert_eq!(
        occ["items"][0]["provenance"],
        serde_json::json!({"provider": "syntax", "grammar": "cpp:0.23.4", "language": "cpp"})
    );
    // The declaration is a search hit and never an occurrence.
    let results = doc["results"].as_array().unwrap();
    assert!(results
        .iter()
        .any(|r| r["kind"] == "field" && r["path"] == "options.h"));
    assert_eq!(doc["freshness"], "fresh");
}

#[test]
fn search_without_the_occurrences_flag_has_no_occurrences_member() {
    let (h, _repo) = occurrence_repo("occ-absent-flag");
    let (code, out, err) = run(
        &h,
        &[
            "search",
            "--repo",
            "r",
            "--symbol",
            "max_triangulation_angle",
        ],
    );
    assert_eq!(code, 0, "search failed: {err}");
    assert!(!out.contains("occurrences"), "got: {out}");
}

#[test]
fn an_absent_name_has_an_explicit_empty_occurrence_list() {
    let (h, _repo) = occurrence_repo("occ-empty");
    let (_, doc) = occurrence_search(&h, "no_such_member", &[]);
    let occ = &doc["occurrences"];
    assert_eq!(occ["total"], 0);
    assert_eq!(occ["truncated"], false);
    assert_eq!(occ["items"], serde_json::json!([]));
    assert_eq!(occ["field_declarations"], 0);
    assert_eq!(occ["scope"], "repository-wide-exact-name");
    assert_eq!(occ["binding"], "unbound");
    assert_eq!(
        doc["freshness"], "fresh",
        "the verdict travels with the list"
    );
}

#[test]
fn the_occurrence_lookup_is_exact_while_the_symbol_list_keeps_prefix_matching() {
    let h = home("occ-exact");
    let repo = mkrepo(&h, "work");
    std::fs::write(
        repo.join("options.h"),
        "class Options {\n public:\n  double angle = 1.0;\n  double angle_rad = 2.0;\n};\n",
    )
    .unwrap();
    std::fs::write(
        repo.join("use.cc"),
        "#include \"options.h\"\ndouble read(Options& o) {\n  return o.angle + o.angle_rad;\n}\n",
    )
    .unwrap();
    init_and_index(&h, &repo, "r");
    let (_, doc) = occurrence_search(&h, "angle", &[]);
    assert_eq!(doc["total"], 2, "the symbol list matches both field names");
    let occ = &doc["occurrences"];
    assert_eq!(occ["symbol_declarations"], 1, "exact name only");
    assert_eq!(occ["total"], 1, "only the exact name's use: {occ}");
}

#[test]
fn occurrence_paging_applies_to_each_list_with_its_own_totals() {
    let (h, _repo) = occurrence_repo("occ-page");
    // Symbol list: one hit, so offset 1 is past it; occurrence list: 4 rows.
    let (_, doc) = occurrence_search(
        &h,
        "max_triangulation_angle",
        &["--limit", "1", "--offset", "1"],
    );
    assert_eq!(doc["total"], 1);
    assert_eq!(doc["results"], serde_json::json!([]));
    assert_eq!(doc["truncated"], false);
    let occ = &doc["occurrences"];
    assert_eq!(occ["total"], 4);
    assert_eq!(occ["items"].as_array().unwrap().len(), 1);
    assert_eq!(occ["items"][0]["span"]["start_line"], 5, "second row");
    assert_eq!(occ["truncated"], true);
    let (_, last) = occurrence_search(
        &h,
        "max_triangulation_angle",
        &["--limit", "2", "--offset", "3"],
    );
    assert_eq!(last["occurrences"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(last["occurrences"]["truncated"], false);
}

#[test]
fn search_occurrences_is_deterministic() {
    let (h, _repo) = occurrence_repo("occ-determinism");
    let (first, _) = occurrence_search(&h, "max_triangulation_angle", &[]);
    let (second, _) = occurrence_search(&h, "max_triangulation_angle", &[]);
    assert_eq!(first, second);
}

#[test]
fn search_occurrences_without_an_index_adds_no_member() {
    let h = home("occ-no-index");
    let repo = mkrepo(&h, "work");
    std::fs::write(repo.join("options.h"), CPP_FIELDS).unwrap();
    let (code, _, err) = run(
        &h,
        &["init", "--repo", repo.to_str().unwrap(), "--name", "r"],
    );
    assert_eq!(code, 0, "init failed: {err}");
    let (code, out, err) = run(
        &h,
        &[
            "search",
            "--repo",
            "r",
            "--symbol",
            "max_triangulation_angle",
            "--occurrences",
        ],
    );
    assert_eq!(code, 0, "search failed: {err}");
    assert!(out.contains("\"freshness\":\"unknown\""), "got: {out}");
    assert!(out.contains("no-index"), "got: {out}");
    assert!(
        !out.contains("occurrences"),
        "the flag cannot manufacture facts: {out}"
    );
}

#[test]
fn an_old_index_answers_stale_with_no_occurrences_until_it_is_rebuilt() {
    let (h, _repo) = occurrence_repo("occ-stale");
    let db = h.join("index").join("r.db");
    {
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute(
            "UPDATE snapshots SET parser_version = parser_version - 1",
            [],
        )
        .unwrap();
        conn.execute("DELETE FROM reference_occurrences", [])
            .unwrap();
    }
    let (_, stale) = occurrence_search(&h, "max_triangulation_angle", &[]);
    assert_eq!(stale["freshness"], "stale");
    assert!(stale["reasons"].to_string().contains("parser-changed"));
    assert_eq!(
        stale["occurrences"]["total"], 0,
        "the stale reason travels with it"
    );
    let (code, _, err) = run(&h, &["index", "--repo", "r"]);
    assert_eq!(code, 0, "index failed: {err}");
    let (_, fresh) = occurrence_search(&h, "max_triangulation_angle", &[]);
    assert_eq!(fresh["freshness"], "fresh");
    assert_eq!(fresh["occurrences"]["total"], 4);
}

#[test]
fn the_usage_line_documents_the_occurrences_flag() {
    let h = home("occ-usage");
    let (code, out, _) = run(&h, &["--help"]);
    assert_eq!(code, 0);
    assert!(
        out.contains(
            "search --repo <name> --symbol <substr> [--limit <n>] [--offset <n>] [--occurrences]"
        ),
        "got: {out}"
    );
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
