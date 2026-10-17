// SPDX-License-Identifier: Apache-2.0
//! M7 knowledge CLI integration tests: each case spawns the real binary with
//! an isolated `RILTRA_HOME`. Covers the eight subcommands, exit codes,
//! strict input, M6 isolation, evidence freshness and persistence.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn home(tag: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let mut p = std::env::temp_dir();
    p.push(format!(
        "riltra-knowledge-{}-{}-{}",
        tag,
        std::process::id(),
        n
    ));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn mkrepo(home: &Path, name: &str) -> PathBuf {
    let d = home.join(name);
    std::fs::create_dir_all(&d).unwrap();
    d
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

fn run_stdin(home: &Path, args: &[&str], input: &[u8]) -> (i32, String, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_riltra"))
        .args(args)
        .env("RILTRA_HOME", home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn riltra");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(input)
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait riltra");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn init(home: &Path, dir: &Path, name: &str) {
    let (code, _, err) = run(
        home,
        &["init", "--repo", dir.to_str().unwrap(), "--name", name],
    );
    assert_eq!(code, 0, "init failed: {err}");
}

fn create(home: &Path, repo: &str, id: &str, title: &str, body: &str) -> (i32, String, String) {
    let input = format!(
        "{{\"id\":\"{id}\",\"kind\":\"decision\",\"title\":\"{title}\",\"body\":\"{body}\",\"status\":\"active\",\"author\":\"a\",\"evidence\":[]}}"
    );
    run_stdin(
        home,
        &["knowledge", "create", "--repo", repo, "--input", "-"],
        input.as_bytes(),
    )
}

fn field_num(out: &str, key: &str) -> u64 {
    let needle = format!("\"{key}\":");
    let start = out
        .find(&needle)
        .unwrap_or_else(|| panic!("missing {key} in {out}"))
        + needle.len();
    let digits: String = out[start..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits
        .parse()
        .unwrap_or_else(|_| panic!("bad {key} in {out}"))
}

const CREATE_D1: &[u8] =
    br#"{"id":"d1","kind":"decision","title":"First","body":"alpha zqxbody","status":"active","author":"a","evidence":[]}"#;

#[test]
fn every_subcommand_round_trips() {
    let h = home("roundtrip");
    let dir = mkrepo(&h, "krepo");
    init(&h, &dir, "krepo");

    let (code, out, err) = run_stdin(
        &h,
        &["knowledge", "create", "--repo", "krepo", "--input", "-"],
        CREATE_D1,
    );
    assert_eq!(code, 0, "create: {err}");
    assert!(out.contains("\"schema_version\":1"), "{out}");
    assert!(out.contains("\"operation\":\"create\""), "{out}");
    assert!(out.contains("\"revision\":1"), "{out}");
    assert!(out.contains("\"verification\":\"Unverified\""), "{out}");
    assert!(out.contains("\"support\":\"unknown\""), "{out}");

    let (code, out, err) = run(&h, &["knowledge", "show", "--repo", "krepo", "--id", "d1"]);
    assert_eq!(code, 0, "show: {err}");
    assert!(out.contains("\"operation\":\"show\""), "{out}");
    assert!(out.contains("\"title\":\"First\""), "{out}");

    let update =
        br#"{"title":"Second","body":"beta zqxbody","status":"active","author":"a","evidence":[]}"#;
    let (code, out, err) = run_stdin(
        &h,
        &[
            "knowledge",
            "update",
            "--repo",
            "krepo",
            "--id",
            "d1",
            "--expected-revision",
            "1",
            "--input",
            "-",
        ],
        update,
    );
    assert_eq!(code, 0, "update: {err}");
    assert!(out.contains("\"revision\":2"), "{out}");

    let (code, out, err) = run(
        &h,
        &["knowledge", "history", "--repo", "krepo", "--id", "d1"],
    );
    assert_eq!(code, 0, "history: {err}");
    assert!(out.contains("\"operation\":\"history\""), "{out}");
    assert!(out.contains("\"total\":2"), "{out}");

    let (code, out, err) = run(
        &h,
        &["knowledge", "search", "--repo", "krepo", "--query", "beta"],
    );
    assert_eq!(code, 0, "search: {err}");
    assert!(out.contains("\"operation\":\"search\""), "{out}");
    assert!(out.contains("\"total\":1"), "{out}");
    assert!(out.contains("\"id\":\"d1\""), "{out}");

    let create2 = br#"{"id":"d2","kind":"decision","title":"Other","body":"gamma","status":"active","author":"a","evidence":[]}"#;
    assert_eq!(
        run_stdin(
            &h,
            &["knowledge", "create", "--repo", "krepo", "--input", "-"],
            create2
        )
        .0,
        0
    );
    let link = br#"{"relation_id":"r1","src_id":"d1","src_revision":2,"dst_id":"d2","dst_revision":1,"kind":"supports","author":"a","rationale":"because"}"#;
    let (code, out, err) = run_stdin(
        &h,
        &["knowledge", "link", "--repo", "krepo", "--input", "-"],
        link,
    );
    assert_eq!(code, 0, "link: {err}");
    assert!(out.contains("\"operation\":\"link\""), "{out}");
    assert!(out.contains("\"relation_id\":\"r1\""), "{out}");

    let (code, out, err) = run(
        &h,
        &[
            "knowledge",
            "related",
            "--repo",
            "krepo",
            "--id",
            "d1",
            "--revision",
            "2",
        ],
    );
    assert_eq!(code, 0, "related: {err}");
    assert!(out.contains("\"relation_id\":\"r1\""), "{out}");

    let unlink = br#"{"author":"a","reason":"no longer"}"#;
    let (code, out, err) = run_stdin(
        &h,
        &[
            "knowledge",
            "unlink",
            "--repo",
            "krepo",
            "--relation",
            "r1",
            "--input",
            "-",
        ],
        unlink,
    );
    assert_eq!(code, 0, "unlink: {err}");
    assert!(out.contains("\"operation\":\"unlink\""), "{out}");
    assert!(out.contains("\"retracted\":true"), "{out}");

    let (code, out, _) = run(
        &h,
        &[
            "knowledge",
            "related",
            "--repo",
            "krepo",
            "--id",
            "d1",
            "--revision",
            "2",
        ],
    );
    assert_eq!(code, 0);
    assert!(
        !out.contains("\"relation_id\":\"r1\""),
        "normal read omits: {out}"
    );
    let (code, out, err) = run(
        &h,
        &[
            "knowledge",
            "related",
            "--repo",
            "krepo",
            "--id",
            "d1",
            "--revision",
            "2",
            "--include-retracted",
        ],
    );
    assert_eq!(code, 0, "audit: {err}");
    assert!(out.contains("\"relation_id\":\"r1\""), "{out}");
    assert!(out.contains("\"retraction_author\":\"a\""), "{out}");
    assert!(out.contains("\"retraction_reason\":\"no longer\""), "{out}");
}

#[test]
fn strict_mutation_json_and_validation_failures() {
    let h = home("strict");
    let dir = mkrepo(&h, "krepo");
    init(&h, &dir, "krepo");

    let (code, out, err) = run_stdin(
        &h,
        &["knowledge", "create", "--repo", "krepo", "--input", "-"],
        b"{not json",
    );
    assert_eq!(code, 2, "invalid json: {err}");
    assert!(out.is_empty());

    let bad_inputs: &[&[u8]] = &[
        br#"{"id":"d1","kind":"decision","title":"t","body":"b","status":"active","author":"a","evidence":[],"extra":1}"#,
        br#"{"id":"d1","kind":"decision","title":"t","title":"t2","body":"b","status":"active","author":"a","evidence":[]}"#,
        br#"{"kind":"decision","title":"t","body":"b","status":"active","author":"a","evidence":[]}"#,
        br#"{"id":"d1","kind":"decision","title":"t","body":"b","status":"superseded","author":"a","evidence":[]}"#,
        br#"{"id":"d1","kind":"note","title":"t","body":"b","status":"active","author":"a","evidence":[]}"#,
    ];
    for bad in bad_inputs {
        let (code, _, _) = run_stdin(
            &h,
            &["knowledge", "create", "--repo", "krepo", "--input", "-"],
            bad,
        );
        assert_eq!(code, 2, "must reject {bad:?}");
    }

    assert_eq!(
        run_stdin(
            &h,
            &["knowledge", "create", "--repo", "krepo", "--input", "-"],
            CREATE_D1
        )
        .0,
        0
    );
    // An update cannot rename id or kind.
    let (code, _, _) = run_stdin(
        &h,
        &[
            "knowledge",
            "update",
            "--repo",
            "krepo",
            "--id",
            "d1",
            "--expected-revision",
            "1",
            "--input",
            "-",
        ],
        br#"{"id":"d1","title":"t","body":"b","status":"active","author":"a","evidence":[]}"#,
    );
    assert_eq!(code, 2);

    // Link input: unknown and missing fields are rejected.
    let bad_links: &[&[u8]] = &[
        br#"{"relation_id":"r1","src_id":"d1","src_revision":1,"dst_id":"d1","dst_revision":1,"kind":"supports","author":"a","rationale":"r","bogus":1}"#,
        br#"{"src_id":"d1","src_revision":1,"dst_id":"d1","dst_revision":1,"kind":"supports","author":"a","rationale":"r"}"#,
        br#"{"relation_id":"r1","src_id":"d1","src_revision":1,"dst_id":"d1","dst_revision":1,"kind":"certifies","author":"a","rationale":"r"}"#,
    ];
    for bad in bad_links {
        let (code, _, _) = run_stdin(
            &h,
            &["knowledge", "link", "--repo", "krepo", "--input", "-"],
            bad,
        );
        assert_eq!(code, 2, "must reject {bad:?}");
    }
    let (code, _, _) = run_stdin(
        &h,
        &[
            "knowledge",
            "unlink",
            "--repo",
            "krepo",
            "--relation",
            "r1",
            "--input",
            "-",
        ],
        br#"{"author":"a"}"#,
    );
    assert_eq!(code, 2);
}

#[test]
fn exit_codes_and_stream_separation() {
    let h = home("codes");
    let dir = mkrepo(&h, "krepo");
    init(&h, &dir, "krepo");

    let (code, out, err) = run(
        &h,
        &[
            "knowledge",
            "show",
            "--repo",
            "krepo",
            "--id",
            "d1",
            "--bogus",
        ],
    );
    assert_eq!(code, 2);
    assert!(out.is_empty());
    assert!(!err.is_empty());

    assert_eq!(
        run_stdin(
            &h,
            &["knowledge", "create", "--repo", "krepo", "--input", "-"],
            CREATE_D1
        )
        .0,
        0
    );
    let (code, out, err) = run(
        &h,
        &["knowledge", "show", "--repo", "krepo", "--id", "nope"],
    );
    assert_eq!(code, 3);
    assert!(out.is_empty());
    assert!(err.contains("unknown record"), "{err}");

    let update = br#"{"title":"t2","body":"b2","status":"active","author":"a","evidence":[]}"#;
    assert_eq!(
        run_stdin(
            &h,
            &[
                "knowledge",
                "update",
                "--repo",
                "krepo",
                "--id",
                "d1",
                "--expected-revision",
                "1",
                "--input",
                "-",
            ],
            update
        )
        .0,
        0
    );
    let (code, out, err) = run_stdin(
        &h,
        &[
            "knowledge",
            "update",
            "--repo",
            "krepo",
            "--id",
            "d1",
            "--expected-revision",
            "1",
            "--input",
            "-",
        ],
        update,
    );
    assert_eq!(code, 3);
    assert!(out.is_empty());
    assert!(err.contains("revision conflict"), "{err}");
}

#[test]
fn missing_store_search_empty_without_creating_db() {
    let h = home("nostore");
    let dir = mkrepo(&h, "krepo");
    init(&h, &dir, "krepo");
    let db = h.join("knowledge").join("krepo.db");

    let (code, out, err) = run(
        &h,
        &[
            "knowledge",
            "search",
            "--repo",
            "krepo",
            "--query",
            "anything",
        ],
    );
    assert_eq!(code, 0, "search: {err}");
    assert!(out.contains("\"hits\":[]"), "{out}");
    assert!(out.contains("\"total\":0"), "{out}");
    assert!(!db.exists(), "search must not create a store");

    let (code, out, err) = run(&h, &["knowledge", "show", "--repo", "krepo", "--id", "d1"]);
    assert_eq!(code, 3);
    assert!(out.is_empty());
    assert!(err.contains("no knowledge store"), "{err}");
    assert!(!db.exists());
}

#[test]
fn corrupt_store_fails_closed_without_delete_or_recreate() {
    let h = home("corrupt");
    let dir = mkrepo(&h, "krepo");
    init(&h, &dir, "krepo");
    assert_eq!(
        run_stdin(
            &h,
            &["knowledge", "create", "--repo", "krepo", "--input", "-"],
            CREATE_D1
        )
        .0,
        0
    );
    let db = h.join("knowledge").join("krepo.db");
    std::fs::write(&db, b"not a sqlite database").unwrap();

    let (code, out, err) = run(&h, &["knowledge", "show", "--repo", "krepo", "--id", "d1"]);
    assert_eq!(code, 3, "show: {err}");
    assert!(out.is_empty());
    let (code, out, err) = run(
        &h,
        &["knowledge", "search", "--repo", "krepo", "--query", "First"],
    );
    assert_eq!(code, 3, "search: {err}");
    assert!(out.is_empty());
    assert_eq!(
        std::fs::read(&db).unwrap(),
        b"not a sqlite database".to_vec(),
        "corrupt store must not be deleted or recreated"
    );
}

#[test]
fn stale_expected_revision_and_generation_conflict() {
    let h = home("conflict");
    let dir = mkrepo(&h, "krepo");
    init(&h, &dir, "krepo");
    assert_eq!(
        run_stdin(
            &h,
            &["knowledge", "create", "--repo", "krepo", "--input", "-"],
            CREATE_D1
        )
        .0,
        0
    );
    let (_, out, _) = run(
        &h,
        &["knowledge", "search", "--repo", "krepo", "--query", "alpha"],
    );
    let generation = field_num(&out, "generation");

    // A concurrent mutation bumps generation: old pages fail explicitly.
    assert_eq!(create(&h, "krepo", "d2", "Other", "beta").0, 0);
    let (code, out, err) = run(
        &h,
        &[
            "knowledge",
            "search",
            "--repo",
            "krepo",
            "--query",
            "alpha",
            "--generation",
            &generation.to_string(),
        ],
    );
    assert_eq!(code, 3, "stale generation: {err}");
    assert!(out.is_empty());
    assert!(err.contains("generation"), "{err}");
    let (code, out, err) = run(
        &h,
        &[
            "knowledge",
            "history",
            "--repo",
            "krepo",
            "--id",
            "d1",
            "--generation",
            &generation.to_string(),
        ],
    );
    assert_eq!(code, 3, "stale history generation: {err}");
    assert!(out.is_empty());
}

#[test]
fn history_is_immutable_and_ordered() {
    let h = home("immutable");
    let dir = mkrepo(&h, "krepo");
    init(&h, &dir, "krepo");
    assert_eq!(
        run_stdin(
            &h,
            &["knowledge", "create", "--repo", "krepo", "--input", "-"],
            CREATE_D1
        )
        .0,
        0
    );
    for (expected, body) in [(2u64, "beta"), (3, "gamma")] {
        let input = format!(
            "{{\"title\":\"t\",\"body\":\"{body}\",\"status\":\"active\",\"author\":\"a\",\"evidence\":[]}}"
        );
        let (code, out, err) = run_stdin(
            &h,
            &[
                "knowledge",
                "update",
                "--repo",
                "krepo",
                "--id",
                "d1",
                "--expected-revision",
                &(expected - 1).to_string(),
                "--input",
                "-",
            ],
            input.as_bytes(),
        );
        assert_eq!(code, 0, "update: {err}");
        assert!(out.contains(&format!("\"revision\":{expected}")), "{out}");
    }
    let (code, out, err) = run(
        &h,
        &["knowledge", "history", "--repo", "krepo", "--id", "d1"],
    );
    assert_eq!(code, 0, "history: {err}");
    assert!(out.contains("\"total\":3"), "{out}");
    let first = out.find("\"revision\":1").expect("rev1");
    let second = out.find("\"revision\":2").expect("rev2");
    let third = out.find("\"revision\":3").expect("rev3");
    assert!(
        first < second && second < third,
        "history orders by revision: {out}"
    );
    // Historical revisions remain readable and unchanged.
    let (code, out, _) = run(
        &h,
        &[
            "knowledge",
            "show",
            "--repo",
            "krepo",
            "--id",
            "d1",
            "--revision",
            "1",
        ],
    );
    assert_eq!(code, 0);
    assert!(out.contains("\"body\":\"alpha zqxbody\""), "{out}");
}

#[test]
fn search_orders_filters_and_terminates() {
    let h = home("ordering");
    let dir = mkrepo(&h, "krepo");
    init(&h, &dir, "krepo");
    // c1 is a constraint, d1/d2 are decisions; the filter narrows first.
    let c1 = br#"{"id":"c1","kind":"constraint","title":"c","body":"common zqxterm","status":"active","author":"a","evidence":[]}"#;
    let (code, _, err) = run_stdin(
        &h,
        &["knowledge", "create", "--repo", "krepo", "--input", "-"],
        c1,
    );
    assert_eq!(code, 0, "create constraint: {err}");
    assert_eq!(create(&h, "krepo", "d1", "d", "common zqxterm").0, 0);
    assert_eq!(create(&h, "krepo", "d2", "d", "common zqxterm").0, 0);
    // Archive d2 by a new revision.
    let archive =
        br#"{"title":"d","body":"common zqxterm","status":"archived","author":"a","evidence":[]}"#;
    assert_eq!(
        run_stdin(
            &h,
            &[
                "knowledge",
                "update",
                "--repo",
                "krepo",
                "--id",
                "d2",
                "--expected-revision",
                "1",
                "--input",
                "-",
            ],
            archive
        )
        .0,
        0
    );

    let (code, out, err) = run(
        &h,
        &[
            "knowledge",
            "search",
            "--repo",
            "krepo",
            "--query",
            "common",
        ],
    );
    assert_eq!(code, 0, "search: {err}");
    assert!(out.contains("\"total\":2"), "archived hidden: {out}");
    let c1 = out.find("\"id\":\"c1\"").expect("c1");
    let d1 = out.find("\"id\":\"d1\"").expect("d1");
    assert!(c1 < d1, "binary id order: {out}");
    assert!(!out.contains("\"id\":\"d2\""), "{out}");

    let (code, out, err) = run(
        &h,
        &[
            "knowledge",
            "search",
            "--repo",
            "krepo",
            "--query",
            "common",
            "--include-archived",
        ],
    );
    assert_eq!(code, 0, "archived search: {err}");
    assert!(out.contains("\"total\":3"), "{out}");

    let (code, out, err) = run(
        &h,
        &[
            "knowledge",
            "search",
            "--repo",
            "krepo",
            "--query",
            "common",
            "--kind",
            "constraint",
        ],
    );
    assert_eq!(code, 0, "kind search: {err}");
    assert!(out.contains("\"total\":1"), "{out}");
    assert!(out.contains("\"id\":\"c1\""), "{out}");

    // Pagination: limit 1 advances; the final page terminates with no next.
    let (code, out, err) = run(
        &h,
        &[
            "knowledge",
            "search",
            "--repo",
            "krepo",
            "--query",
            "common",
            "--include-archived",
            "--limit",
            "1",
        ],
    );
    assert_eq!(code, 0, "page1: {err}");
    assert!(out.contains("\"total\":3"), "{out}");
    assert!(out.contains("\"truncated\":true"), "{out}");
    assert!(out.contains("\"next_offset\":1"), "{out}");
    let (code, out, err) = run(
        &h,
        &[
            "knowledge",
            "search",
            "--repo",
            "krepo",
            "--query",
            "common",
            "--include-archived",
            "--limit",
            "1",
            "--offset",
            "3",
        ],
    );
    assert_eq!(code, 0, "past end: {err}");
    assert!(out.contains("\"truncated\":false"), "{out}");
    assert!(out.contains("\"next_offset\":null"), "{out}");
}

#[test]
fn relations_are_pinned_and_one_hop() {
    let h = home("relations");
    let dir = mkrepo(&h, "krepo");
    init(&h, &dir, "krepo");
    assert_eq!(create(&h, "krepo", "d1", "one", "first").0, 0);
    assert_eq!(create(&h, "krepo", "d2", "two", "second").0, 0);
    assert_eq!(create(&h, "krepo", "d3", "three", "third").0, 0);

    let link = |rel: &str, src: &str, dst: &str, kind: &str| {
        let input = format!(
            "{{\"relation_id\":\"{rel}\",\"src_id\":\"{src}\",\"src_revision\":1,\"dst_id\":\"{dst}\",\"dst_revision\":1,\"kind\":\"{kind}\",\"author\":\"a\",\"rationale\":\"r\"}}"
        );
        run_stdin(
            &h,
            &["knowledge", "link", "--repo", "krepo", "--input", "-"],
            input.as_bytes(),
        )
    };
    assert_eq!(link("r1", "d1", "d2", "supports").0, 0);
    assert_eq!(link("r2", "d2", "d3", "relates_to").0, 0);

    // Updating d1 does not move its relations to the new revision.
    let update =
        br#"{"title":"one v2","body":"first v2","status":"active","author":"a","evidence":[]}"#;
    assert_eq!(
        run_stdin(
            &h,
            &[
                "knowledge",
                "update",
                "--repo",
                "krepo",
                "--id",
                "d1",
                "--expected-revision",
                "1",
                "--input",
                "-",
            ],
            update
        )
        .0,
        0
    );
    let (code, out, _) = run(
        &h,
        &["knowledge", "related", "--repo", "krepo", "--id", "d1"],
    );
    assert_eq!(code, 0);
    assert!(
        !out.contains("\"relation_id\":\"r1\""),
        "current rev2 has none: {out}"
    );
    let (code, out, err) = run(
        &h,
        &[
            "knowledge",
            "related",
            "--repo",
            "krepo",
            "--id",
            "d1",
            "--revision",
            "1",
        ],
    );
    assert_eq!(code, 0, "historical: {err}");
    assert!(out.contains("\"relation_id\":\"r1\""), "{out}");
    assert!(
        !out.contains("\"relation_id\":\"r2\""),
        "one hop only: {out}"
    );

    // Inbound for d2 sees r1; outbound for d2 sees r2 (no transitive d1).
    let (code, out, _) = run(
        &h,
        &[
            "knowledge",
            "related",
            "--repo",
            "krepo",
            "--id",
            "d2",
            "--direction",
            "inbound",
        ],
    );
    assert_eq!(code, 0);
    assert!(out.contains("\"relation_id\":\"r1\""), "{out}");
    let (code, out, _) = run(
        &h,
        &[
            "knowledge",
            "related",
            "--repo",
            "krepo",
            "--id",
            "d2",
            "--direction",
            "outbound",
        ],
    );
    assert_eq!(code, 0);
    assert!(out.contains("\"relation_id\":\"r2\""), "{out}");
    assert!(!out.contains("\"relation_id\":\"r1\""), "{out}");
}

#[test]
fn evidence_freshness_and_verification_separation() {
    let h = home("evidence");
    let dir = mkrepo(&h, "krepo");
    init(&h, &dir, "krepo");
    std::fs::write(dir.join("src.rs"), b"alpha zqxevidence\n").unwrap();

    let with_evidence = br#"{"id":"e1","kind":"decision","title":"t","body":"b","status":"active","author":"a","evidence":[{"path":"src.rs"}]}"#;
    let (code, out, err) = run_stdin(
        &h,
        &["knowledge", "create", "--repo", "krepo", "--input", "-"],
        with_evidence,
    );
    assert_eq!(code, 0, "create with evidence: {err}");
    assert!(out.contains("\"status\":\"fresh\""), "{out}");
    assert!(out.contains("\"verification\":\"Unverified\""), "{out}");
    assert!(out.contains("\"support\":\"unverified\""), "{out}");

    // Edited bytes: stale, original binding preserved, never verified.
    std::fs::write(dir.join("src.rs"), b"alpha zqxevidence changed\n").unwrap();
    let (code, out, _) = run(&h, &["knowledge", "show", "--repo", "krepo", "--id", "e1"]);
    assert_eq!(code, 0);
    assert!(out.contains("\"status\":\"stale\""), "{out}");
    assert!(out.contains("\"verification\":\"Unverified\""), "{out}");

    // Missing file: unresolved with a reason.
    std::fs::remove_file(dir.join("src.rs")).unwrap();
    let (code, out, _) = run(&h, &["knowledge", "show", "--repo", "krepo", "--id", "e1"]);
    assert_eq!(code, 0);
    assert!(out.contains("\"status\":\"unresolved\""), "{out}");

    // No evidence means unknown support, never a completeness claim.
    assert_eq!(create(&h, "krepo", "e2", "t", "b").0, 0);
    let (code, out, _) = run(&h, &["knowledge", "show", "--repo", "krepo", "--id", "e2"]);
    assert_eq!(code, 0);
    assert!(out.contains("\"support\":\"unknown\""), "{out}");
    assert!(out.contains("\"verification\":\"Unverified\""), "{out}");
}

#[test]
fn independent_stores_and_cross_repo_rejection() {
    let h = home("isolated");
    let a = mkrepo(&h, "ra");
    let b = mkrepo(&h, "rb");
    init(&h, &a, "ra");
    init(&h, &b, "rb");

    assert_eq!(
        run_stdin(
            &h,
            &["knowledge", "create", "--repo", "ra", "--input", "-"],
            br#"{"id":"d1","kind":"decision","title":"in-a","body":"A","status":"active","author":"a","evidence":[]}"#
        )
        .0,
        0
    );
    assert_eq!(
        run_stdin(
            &h,
            &["knowledge", "create", "--repo", "rb", "--input", "-"],
            br#"{"id":"d1","kind":"decision","title":"in-b","body":"B","status":"active","author":"a","evidence":[]}"#
        )
        .0,
        0
    );
    let (_, out_a, _) = run(&h, &["knowledge", "show", "--repo", "ra", "--id", "d1"]);
    let (_, out_b, _) = run(&h, &["knowledge", "show", "--repo", "rb", "--id", "d1"]);
    assert!(out_a.contains("\"title\":\"in-a\""), "{out_a}");
    assert!(out_b.contains("\"title\":\"in-b\""), "{out_b}");
    assert_ne!(out_a, out_b);

    // A same-named record in another store is not a valid endpoint here.
    let link = br#"{"relation_id":"r1","src_id":"d1","src_revision":1,"dst_id":"only-in-rb","dst_revision":1,"kind":"supports","author":"a","rationale":"r"}"#;
    let (code, _, err) = run_stdin(
        &h,
        &["knowledge", "link", "--repo", "ra", "--input", "-"],
        link,
    );
    assert_eq!(code, 3, "cross-repo endpoint: {err}");
    assert!(err.contains("endpoint"), "{err}");
}

// --- minimal SCIP encoder for the persistence test -------------------------
// Field numbers pinned by proto/scip.proto; only the fields the importer
// reads are emitted.

fn varint(mut v: u64, out: &mut Vec<u8>) {
    loop {
        let mut byte = (v & 0x7f) as u8;
        v >>= 7;
        if v != 0 {
            byte |= 0x80;
        }
        out.push(byte);
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

fn occurrence(symbol: &str, line: i64, start: i64, end: i64) -> Vec<u8> {
    let mut o = Vec::new();
    str_field(2, symbol, &mut o);
    varint_field(3, 1, &mut o); // definition role
    let mut range = Vec::new();
    varint_field(1, line as u64, &mut range);
    varint_field(2, start as u64, &mut range);
    varint_field(3, end as u64, &mut range);
    len_field(8, &range, &mut o);
    o
}

fn document(rel: &str, occ: Vec<u8>) -> Vec<u8> {
    let mut d = Vec::new();
    str_field(1, rel, &mut d);
    len_field(2, &occ, &mut d);
    varint_field(6, 1, &mut d); // UTF8
    d
}

fn scip_index(root: &Path, rel: &str, symbol: &str) -> Vec<u8> {
    let mut md = Vec::new();
    let mut tool = Vec::new();
    str_field(1, "riltra-knowledge-test", &mut tool);
    str_field(2, "1.0", &mut tool);
    len_field(2, &tool, &mut md);
    str_field(
        3,
        &format!("file://{}", root.to_string_lossy().replace('\\', "/")),
        &mut md,
    );
    varint_field(4, 1, &mut md);
    let mut out = Vec::new();
    len_field(1, &md, &mut out);
    len_field(2, &document(rel, occurrence(symbol, 0, 0, 3)), &mut out);
    out
}

#[test]
fn knowledge_persists_across_index_and_import() {
    let h = home("persist");
    let dir = mkrepo(&h, "krepo");
    init(&h, &dir, "krepo");
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src").join("a.ts"), "export function foo() {}\n").unwrap();

    let (code, _, err) = run(&h, &["index", "--repo", "krepo", "--full"]);
    assert_eq!(code, 0, "index: {err}");
    let index_db = h.join("index").join("krepo.db");
    assert!(index_db.exists(), "M6 index db must exist");

    // Knowledge mutations never touch the M6 database.
    let before = std::fs::read(&index_db).unwrap();
    assert_eq!(
        run_stdin(
            &h,
            &["knowledge", "create", "--repo", "krepo", "--input", "-"],
            CREATE_D1
        )
        .0,
        0
    );
    assert_eq!(
        std::fs::read(&index_db).unwrap(),
        before,
        "knowledge touched M6"
    );

    // A second full index must not rebuild knowledge from a cache.
    let (code, _, err) = run(&h, &["index", "--repo", "krepo", "--full"]);
    assert_eq!(code, 0, "second index: {err}");
    let (code, out, err) = run(&h, &["knowledge", "show", "--repo", "krepo", "--id", "d1"]);
    assert_eq!(code, 0, "knowledge after reindex: {err}");
    assert!(out.contains("\"title\":\"First\""), "{out}");

    // Valid SCIP import (graph-proof revalidation) keeps knowledge intact.
    let payload = scip_index(&dir, "src/a.ts", "scip-kt test pkg 1.0.0 src/a.ts/foo().");
    let scip = dir.join("index.scip");
    std::fs::write(&scip, &payload).unwrap();
    let (code, out, err) = run(
        &h,
        &[
            "semantic",
            "import",
            "--repo",
            "krepo",
            "--scip",
            scip.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0, "semantic import: {err}");
    assert!(out.contains("\"definitions\":1"), "{out}");
    let (code, out, err) = run(&h, &["knowledge", "show", "--repo", "krepo", "--id", "d1"]);
    assert_eq!(code, 0, "knowledge after import: {err}");
    assert!(out.contains("\"title\":\"First\""), "{out}");

    // Failed import changes nothing.
    let bad = dir.join("bad.scip");
    std::fs::write(&bad, b"garbage").unwrap();
    let (code, _, err) = run(
        &h,
        &[
            "semantic",
            "import",
            "--repo",
            "krepo",
            "--scip",
            bad.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 3, "bad import: {err}");
    assert_eq!(
        run(&h, &["knowledge", "show", "--repo", "krepo", "--id", "d1"]).0,
        0
    );
}

#[test]
fn knowledge_works_without_code_index() {
    let h = home("noindex");
    let dir = mkrepo(&h, "krepo");
    init(&h, &dir, "krepo");
    assert_eq!(
        run_stdin(
            &h,
            &["knowledge", "create", "--repo", "krepo", "--input", "-"],
            CREATE_D1
        )
        .0,
        0
    );
    assert_eq!(
        run(&h, &["knowledge", "show", "--repo", "krepo", "--id", "d1"]).0,
        0
    );
    assert_eq!(
        run(
            &h,
            &["knowledge", "search", "--repo", "krepo", "--query", "alpha"]
        )
        .0,
        0
    );
    assert_eq!(
        run(
            &h,
            &[
                "knowledge",
                "related",
                "--repo",
                "krepo",
                "--id",
                "d1",
                "--direction",
                "outbound"
            ]
        )
        .0,
        0
    );
    assert!(
        !h.join("index").exists(),
        "knowledge must not require a code index"
    );
}

#[test]
fn knowledge_does_not_touch_m6_index() {
    let h = home("m6");
    let dir = mkrepo(&h, "krepo");
    init(&h, &dir, "krepo");
    std::fs::write(dir.join("a.py"), "def f():\n    pass\n").unwrap();
    let (code, _, err) = run(&h, &["index", "--repo", "krepo", "--full"]);
    assert_eq!(code, 0, "index: {err}");
    let index_db = h.join("index").join("krepo.db");
    let knowledge_db = h.join("knowledge").join("krepo.db");
    assert!(index_db.exists());
    assert!(!knowledge_db.exists());

    let before = std::fs::read(&index_db).unwrap();
    assert_eq!(
        run_stdin(
            &h,
            &["knowledge", "create", "--repo", "krepo", "--input", "-"],
            CREATE_D1
        )
        .0,
        0
    );
    assert!(knowledge_db.exists());
    assert_eq!(
        std::fs::read(&index_db).unwrap(),
        before,
        "knowledge touched M6"
    );

    // Index status still reports a real snapshot after knowledge writes.
    let (code, out, err) = run(&h, &["index", "status", "--repo", "krepo"]);
    assert_eq!(code, 0, "index status: {err}");
    assert!(out.contains("\"snapshot\""), "{out}");
}
