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

// --- Gate I revision regressions ------------------------------------------

/// The link response must find the relation it just committed by exact id, not
/// by scanning one bounded `related` page. With more than one page of outbound
/// relations the new row sorts past that page, so a page scan reports a false
/// failure after a successful commit.
#[test]
fn link_response_reads_exact_relation_beyond_one_page() {
    let h = home("linkdeep");
    let dir = mkrepo(&h, "krepo");
    init(&h, &dir, "krepo");
    assert_eq!(create(&h, "krepo", "d1", "one", "first").0, 0);
    assert_eq!(create(&h, "krepo", "d2", "two", "second").0, 0);
    assert_eq!(create(&h, "krepo", "d3", "three", "third").0, 0);

    // Seed 1000 outbound relations from d1 rev1 to d2 rev1 directly, so the new
    // link (to d3) sorts beyond the 1000-row first page. CLI-only seeding would
    // need 1000 spawns; seeding the store file as a fixture follows the
    // existing pattern in tests/m6_corpus/upgrade.rs.
    let db = h.join("knowledge").join("krepo.db");
    {
        let mut conn = rusqlite::Connection::open(&db).expect("open seed db");
        let tx = conn.transaction().unwrap();
        for n in 0..1000 {
            let kind = ["relates_to", "supports", "contradicts"][n % 3];
            tx.execute(
                "INSERT INTO relations (relation_id, src_id, src_revision, dst_id,
                   dst_revision, kind, author, rationale, created_unix)
                 VALUES (?1, 'd1', 1, 'd2', 1, ?2, 'a', 'r', 0)",
                rusqlite::params![format!("r{n:04}"), kind],
            )
            .unwrap();
        }
        tx.commit().unwrap();
    }

    let (code, out, err) = run(
        &h,
        &["knowledge", "search", "--repo", "krepo", "--query", "zzz"],
    );
    assert_eq!(code, 0, "seed generation probe: {err}");
    let gen_before = field_num(&out, "generation");

    let input = br#"{"relation_id":"z1","src_id":"d1","src_revision":1,"dst_id":"d3","dst_revision":1,"kind":"supports","author":"a","rationale":"r"}"#;
    let (code, out, err) = run_stdin(
        &h,
        &["knowledge", "link", "--repo", "krepo", "--input", "-"],
        input,
    );
    assert_eq!(code, 0, "link beyond one page must succeed: {err}");
    assert!(out.contains("\"operation\":\"link\""), "{out}");
    assert!(out.contains("\"relation_id\":\"z1\""), "{out}");
    assert!(out.contains("\"dst_id\":\"d3\""), "{out}");
    assert_eq!(
        field_num(&out, "generation"),
        gen_before + 1,
        "exactly one generation bump: {out}"
    );

    // Committed exactly once: visible on the second page, total grew by one.
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
            "--limit",
            "1",
            "--offset",
            "1000",
        ],
    );
    assert_eq!(code, 0, "related second page: {err}");
    assert!(
        out.contains("\"relation_id\":\"z1\""),
        "relation must be committed: {out}"
    );
    assert!(out.contains("\"total\":1001"), "{out}");

    // Re-linking the same id fails, proving it was committed exactly once.
    let (code, _, err) = run_stdin(
        &h,
        &["knowledge", "link", "--repo", "krepo", "--input", "-"],
        input,
    );
    assert_eq!(code, 3, "re-link must be a duplicate: {err}");
    assert!(err.contains("duplicate relation id"), "{err}");
}

/// An unregistered repository must not look like a store that happens to be
/// empty; the registration and root binding are validated before any answer.
#[test]
fn search_unregistered_repo_is_not_empty_success() {
    let h = home("unknownrepo");
    let (code, out, err) = run(
        &h,
        &["knowledge", "search", "--repo", "ghost", "--query", "x"],
    );
    assert_eq!(code, 3, "unknown repo must fail: {err}");
    assert!(out.is_empty(), "no misleading empty page: {out}");
    assert!(err.contains("unknown repository"), "{err}");
    assert!(!h.join("knowledge").join("ghost.db").exists());
}

/// A hostile path where the store should be (directory, dangling symlink) is
/// not an absent store: search must fail closed and leave the path untouched.
#[test]
fn search_fails_closed_on_hostile_store_paths() {
    let h = home("hostile");
    let dir = mkrepo(&h, "krepo");
    init(&h, &dir, "krepo");
    let db = h.join("knowledge").join("krepo.db");

    std::fs::create_dir_all(&db).unwrap();
    let (code, out, err) = run(
        &h,
        &["knowledge", "search", "--repo", "krepo", "--query", "x"],
    );
    assert_eq!(code, 3, "directory at store path must fail closed: {err}");
    assert!(out.is_empty());
    assert!(db.is_dir(), "hostile path must be left untouched");
    std::fs::remove_dir(&db).unwrap();

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(h.join("does-not-exist"), &db).unwrap();
        let (code, out, err) = run(
            &h,
            &["knowledge", "search", "--repo", "krepo", "--query", "x"],
        );
        assert_eq!(code, 3, "dangling store symlink must fail closed: {err}");
        assert!(out.is_empty());
        assert!(
            std::fs::symlink_metadata(&db)
                .unwrap()
                .file_type()
                .is_symlink(),
            "symlink must not be replaced"
        );
        std::fs::remove_file(&db).unwrap();
    }
}

/// History pagination must report the real next offset, advance across pages
/// and terminate, while preserving revision order and generation guarding.
#[test]
fn history_reports_next_offset_across_pages() {
    let h = home("histpage");
    let dir = mkrepo(&h, "krepo");
    init(&h, &dir, "krepo");
    assert_eq!(create(&h, "krepo", "d1", "t", "b1").0, 0);
    for (expected, body) in [(2u64, "b2"), (3, "b3")] {
        let input = format!(
            "{{\"title\":\"t\",\"body\":\"{body}\",\"status\":\"active\",\"author\":\"a\",\"evidence\":[]}}"
        );
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
                    &(expected - 1).to_string(),
                    "--input",
                    "-",
                ],
                input.as_bytes(),
            )
            .0,
            0
        );
    }

    let mut seen = Vec::new();
    let mut offset = 0usize;
    for expected_next in [1usize, 2] {
        let (code, out, err) = run(
            &h,
            &[
                "knowledge",
                "history",
                "--repo",
                "krepo",
                "--id",
                "d1",
                "--limit",
                "1",
                "--offset",
                &offset.to_string(),
            ],
        );
        assert_eq!(code, 0, "history page: {err}");
        assert!(out.contains("\"truncated\":true"), "{out}");
        assert!(
            out.contains(&format!("\"next_offset\":{expected_next}")),
            "{out}"
        );
        seen.push(field_num(&out, "revision"));
        offset = expected_next;
    }

    let (code, out, err) = run(
        &h,
        &[
            "knowledge",
            "history",
            "--repo",
            "krepo",
            "--id",
            "d1",
            "--limit",
            "1",
            "--offset",
            "2",
        ],
    );
    assert_eq!(code, 0, "final page: {err}");
    assert!(out.contains("\"truncated\":false"), "{out}");
    assert!(out.contains("\"next_offset\":null"), "{out}");
    seen.push(field_num(&out, "revision"));

    let (code, out, err) = run(
        &h,
        &[
            "knowledge",
            "history",
            "--repo",
            "krepo",
            "--id",
            "d1",
            "--limit",
            "1",
            "--offset",
            "9",
        ],
    );
    assert_eq!(code, 0, "past end: {err}");
    assert!(out.contains("\"truncated\":false"), "{out}");
    assert!(out.contains("\"next_offset\":null"), "{out}");
    assert_eq!(seen, vec![1, 2, 3], "ordering unchanged across pages");

    // Generation guarding still applies to the paged read.
    let (code, out, err) = run(
        &h,
        &["knowledge", "search", "--repo", "krepo", "--query", "zzz"],
    );
    assert_eq!(code, 0, "generation probe: {err}");
    let gen = field_num(&out, "generation");
    assert_eq!(create(&h, "krepo", "d2", "t2", "b").0, 0);
    let (code, _, err) = run(
        &h,
        &[
            "knowledge",
            "history",
            "--repo",
            "krepo",
            "--id",
            "d1",
            "--generation",
            &gen.to_string(),
        ],
    );
    assert_eq!(code, 3, "stale generation must still conflict: {err}");
}

/// Mutation stdin is bounded to the documented cap plus one detection byte:
/// over-limit input is rejected (exit 2) without waiting for EOF, while input
/// under the cap is preserved unchanged.
#[test]
fn mutation_stdin_is_bounded() {
    let h = home("boundedstdin");
    let dir = mkrepo(&h, "krepo");
    init(&h, &dir, "krepo");
    let cap = 1024 * 1024usize;

    assert_eq!(
        run_stdin(
            &h,
            &["knowledge", "create", "--repo", "krepo", "--input", "-"],
            CREATE_D1
        )
        .0,
        0
    );

    let mut child = Command::new(env!("CARGO_BIN_EXE_riltra"))
        .args(["knowledge", "create", "--repo", "krepo", "--input", "-"])
        .env("RILTRA_HOME", &h)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn riltra");
    let mut stdin = child.stdin.take().expect("stdin");
    stdin
        .write_all(&vec![b'x'; cap + 1])
        .expect("write oversized stdin");
    // Keep `stdin` open on purpose: a bounded reader rejects after cap+1 bytes,
    // an unbounded one would block here waiting for EOF forever.
    let mut exited = false;
    for _ in 0..200 {
        if child.try_wait().expect("try_wait").is_some() {
            exited = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    if !exited {
        let _ = child.kill();
        let _ = child.wait();
    }
    assert!(
        exited,
        "oversized stdin must be rejected without waiting for EOF"
    );
    drop(stdin);
    let output = child.wait_with_output().expect("collect riltra");
    assert_eq!(output.status.code(), Some(2));
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(err.contains("too large"), "{err}");
    assert!(
        err.contains(&cap.to_string()),
        "the cap must be documented in the error: {err}"
    );
}

// --- Full-stage lifecycle (schema v2 / G-LIFE) -----------------------------

fn db_path(home: &Path, repo: &str) -> PathBuf {
    home.join("knowledge").join(format!("{repo}.db"))
}

/// Independent inspection of the store file: the migration is asserted from
/// the file itself, not from CLI output.
fn user_version(home: &Path, repo: &str) -> u32 {
    let conn = rusqlite::Connection::open(db_path(home, repo)).unwrap();
    conn.query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap()
}

fn update(home: &Path, repo: &str, id: &str, expected: u64, input: &str) -> (i32, String, String) {
    run_stdin(
        home,
        &[
            "knowledge",
            "update",
            "--repo",
            repo,
            "--id",
            id,
            "--expected-revision",
            &expected.to_string(),
            "--input",
            "-",
        ],
        input.as_bytes(),
    )
}

fn supersede_input(title: &str, target: &str, revision: u64, reason: &str) -> String {
    format!(
        "{{\"title\":\"{title}\",\"body\":\"{title} body zqxbody\",\"status\":\"superseded\",\
         \"author\":\"a\",\"evidence\":[],\
         \"superseded_by\":{{\"id\":\"{target}\",\"revision\":{revision}}},\"reason\":\"{reason}\"}}"
    )
}

fn retract_input(title: &str, reason: &str) -> String {
    format!(
        "{{\"title\":\"{title}\",\"body\":\"{title} body zqxbody\",\"status\":\"retracted\",\
         \"author\":\"a\",\"evidence\":[],\"reason\":\"{reason}\"}}"
    )
}

#[test]
fn lifecycle_supersede_and_retract_round_trip_via_cli() {
    let h = home("lifecycle");
    let dir = mkrepo(&h, "krepo");
    init(&h, &dir, "krepo");
    assert_eq!(create(&h, "krepo", "d1", "First", "alpha zqxbody").0, 0);
    assert_eq!(create(&h, "krepo", "d2", "Second", "beta zqxbody").0, 0);
    assert_eq!(user_version(&h, "krepo"), 1, "Foundation writes stay v1");

    // Supersede d1 by the exact d2 revision, with a reason.
    let (code, out, err) = update(
        &h,
        "krepo",
        "d1",
        1,
        &supersede_input("First", "d2", 1, "d2 replaces d1"),
    );
    assert_eq!(code, 0, "supersede: {err}");
    assert!(out.contains("\"status\":\"superseded\""), "{out}");
    assert!(
        out.contains("\"superseded_by\":{\"id\":\"d2\",\"revision\":1}"),
        "{out}"
    );
    assert!(
        !out.contains("retracted_reason"),
        "a supersede must not report retraction fields: {out}"
    );
    // The write migrated the store and recorded the immutable event plus the
    // deterministic lineage relation.
    assert_eq!(user_version(&h, "krepo"), 2);
    {
        let conn = rusqlite::Connection::open(db_path(&h, "krepo")).unwrap();
        let events: i64 = conn
            .query_row(
                "SELECT count(*) FROM record_lifecycle WHERE id = 'd1' AND revision = 2
                 AND event = 'supersede' AND target_id = 'd2' AND target_revision = 1
                 AND reason = 'd2 replaces d1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(events, 1);
        let relations: i64 = conn
            .query_row(
                "SELECT count(*) FROM relations WHERE relation_id = 'supersedes:d2:1:d1:2'
                 AND kind = 'supersedes'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(relations, 1);
    }
    // The lineage is visible from both ends.
    let (code, out, err) = run(
        &h,
        &[
            "knowledge",
            "related",
            "--repo",
            "krepo",
            "--id",
            "d1",
            "--direction",
            "inbound",
        ],
    );
    assert_eq!(code, 0, "related: {err}");
    assert!(out.contains("supersedes:d2:1:d1:2"), "{out}");
    assert!(out.contains("\"kind\":\"supersedes\""), "{out}");

    // Default search hides the superseded record; the explicit flag shows it.
    let (code, out, _) = run(
        &h,
        &[
            "knowledge",
            "search",
            "--repo",
            "krepo",
            "--query",
            "zqxbody",
        ],
    );
    assert_eq!(code, 0);
    assert!(
        !out.contains("\"id\":\"d1\""),
        "superseded must be hidden: {out}"
    );
    assert!(out.contains("\"id\":\"d2\""), "{out}");
    let (code, out, _) = run(
        &h,
        &[
            "knowledge",
            "search",
            "--repo",
            "krepo",
            "--query",
            "zqxbody",
            "--include-superseded",
        ],
    );
    assert_eq!(code, 0);
    assert!(out.contains("\"id\":\"d1\""), "{out}");

    // Retract d2: the reason and author are reported, and default search is
    // empty once both records are historical.
    let (code, out, err) = update(
        &h,
        "krepo",
        "d2",
        1,
        &retract_input("Second", "bench invalid"),
    );
    assert_eq!(code, 0, "retract: {err}");
    assert!(out.contains("\"status\":\"retracted\""), "{out}");
    assert!(
        out.contains("\"retracted_reason\":\"bench invalid\""),
        "{out}"
    );
    assert!(out.contains("\"retracted_author\":\"a\""), "{out}");
    let (code, out, _) = run(
        &h,
        &[
            "knowledge",
            "search",
            "--repo",
            "krepo",
            "--query",
            "zqxbody",
        ],
    );
    assert_eq!(code, 0);
    assert!(out.contains("\"total\":0"), "{out}");
    let (code, out, _) = run(
        &h,
        &[
            "knowledge",
            "search",
            "--repo",
            "krepo",
            "--query",
            "zqxbody",
            "--include-retracted",
        ],
    );
    assert_eq!(code, 0);
    assert!(out.contains("\"id\":\"d2\""), "{out}");
    assert!(
        !out.contains("\"id\":\"d1\""),
        "superseded stays hidden: {out}"
    );
    // History keeps every revision and its lifecycle state.
    let (code, out, _) = run(
        &h,
        &["knowledge", "history", "--repo", "krepo", "--id", "d1"],
    );
    assert_eq!(code, 0);
    assert!(out.contains("\"revision\":1"), "{out}");
    assert!(out.contains("\"revision\":2"), "{out}");
    assert!(
        out.contains("\"superseded_by\":{\"id\":\"d2\",\"revision\":1}"),
        "{out}"
    );
}

#[test]
fn lifecycle_rejections_surface_documented_exit_codes() {
    let h = home("lifecycle-codes");
    let dir = mkrepo(&h, "krepo");
    init(&h, &dir, "krepo");
    assert_eq!(create(&h, "krepo", "d1", "First", "alpha zqxbody").0, 0);
    assert_eq!(create(&h, "krepo", "d2", "Second", "beta zqxbody").0, 0);
    assert_eq!(create(&h, "krepo", "d3", "Third", "gamma zqxbody").0, 0);

    // Self-supersession is an input error (exit 2).
    let (code, _, err) = update(
        &h,
        "krepo",
        "d1",
        1,
        &supersede_input("First", "d1", 1, "self"),
    );
    assert_eq!(code, 2, "self-supersession: {err}");
    // Missing lifecycle payload fields are input errors (exit 2).
    let missing = "{\"title\":\"First\",\"body\":\"b\",\"status\":\"superseded\",\"author\":\"a\",\"evidence\":[],\"reason\":\"r\"}";
    assert_eq!(update(&h, "krepo", "d1", 1, missing).0, 2);
    let missing = "{\"title\":\"First\",\"body\":\"b\",\"status\":\"superseded\",\"author\":\"a\",\"evidence\":[],\"superseded_by\":{\"id\":\"d2\",\"revision\":1}}";
    assert_eq!(update(&h, "krepo", "d1", 1, missing).0, 2);
    // Unknown target revisions are storage conflicts (exit 3).
    let (code, _, err) = update(
        &h,
        "krepo",
        "d1",
        1,
        &supersede_input("First", "d2", 99, "ghost"),
    );
    assert_eq!(code, 3, "unknown target: {err}");
    assert!(err.contains("unknown supersede target"), "{err}");
    // Create may never start in a lifecycle status (exit 2).
    let created = run_stdin(
        &h,
        &["knowledge", "create", "--repo", "krepo", "--input", "-"],
        br#"{"id":"x1","kind":"decision","title":"t","body":"b","status":"superseded","author":"a","evidence":[]}"#,
    );
    assert_eq!(created.0, 2);
    // Double retraction (exit 2), and superseding a retracted record (exit 2).
    assert_eq!(
        update(&h, "krepo", "d3", 1, &retract_input("Third", "gone")).0,
        0
    );
    let (code, _, err) = update(&h, "krepo", "d3", 2, &retract_input("Third", "again"));
    assert_eq!(code, 2, "double retract: {err}");
    assert!(err.contains("already retracted"), "{err}");
    let (code, _, err) = update(
        &h,
        "krepo",
        "d3",
        2,
        &supersede_input("Third", "d1", 1, "revive"),
    );
    assert_eq!(code, 2, "supersede retracted: {err}");
    // Cycles are rejected (exit 2), and rejected transitions write nothing.
    assert_eq!(
        update(
            &h,
            "krepo",
            "d2",
            1,
            &supersede_input("Second", "d1", 1, "swap")
        )
        .0,
        0
    );
    let (code, _, err) = update(
        &h,
        "krepo",
        "d1",
        1,
        &supersede_input("First", "d2", 2, "close the loop"),
    );
    assert_eq!(code, 2, "cycle: {err}");
    assert!(err.contains("cycle"), "{err}");
    let (code, out, _) = run(
        &h,
        &["knowledge", "history", "--repo", "krepo", "--id", "d1"],
    );
    assert_eq!(code, 0);
    assert!(
        out.contains("\"total\":1"),
        "cycle attempt wrote nothing: {out}"
    );
}

#[test]
fn lifecycle_max_length_ids_retract_full_derived_relation_id_via_cli() {
    let h = home("lifecycle-maxid");
    let dir = mkrepo(&h, "krepo");
    init(&h, &dir, "krepo");
    let old = "o".repeat(128);
    let new = "n".repeat(128);
    assert_eq!(create(&h, "krepo", &old, "Old max", "old zqxmax body").0, 0);
    assert_eq!(create(&h, "krepo", &new, "New max", "new zqxmax body").0, 0);
    // The derived lineage identity exceeds the 128-byte caller cap by
    // construction; it must stay complete, never truncated.
    let derived = format!("supersedes:{new}:1:{old}:2");
    assert!(derived.len() > 128, "derived id must exceed the caller cap");
    let (code, out, err) = update(
        &h,
        "krepo",
        &old,
        1,
        &supersede_input("Old max", &new, 1, "max ids"),
    );
    assert_eq!(code, 0, "supersede: {err}");
    assert!(out.contains("\"status\":\"superseded\""), "{out}");
    assert_eq!(user_version(&h, "krepo"), 2);
    // The full identity is listed, never truncated.
    let (code, out, err) = run(
        &h,
        &[
            "knowledge",
            "related",
            "--repo",
            "krepo",
            "--id",
            &old,
            "--direction",
            "inbound",
        ],
    );
    assert_eq!(code, 0, "related: {err}");
    assert!(out.contains(&derived), "full derived identity: {out}");
    let (_, show_pre_unlink, _) = run(&h, &["knowledge", "show", "--repo", "krepo", "--id", &old]);
    let gen_before_unlink = field_num(&show_pre_unlink, "generation");
    // The full-length id is retractable through the CLI.
    let (code, out, err) = run_stdin(
        &h,
        &[
            "knowledge",
            "unlink",
            "--repo",
            "krepo",
            "--relation",
            &derived,
            "--input",
            "-",
        ],
        br#"{"author":"a","reason":"lineage mistake"}"#,
    );
    assert_eq!(code, 0, "unlink derived id: {err}");
    assert!(out.contains("\"operation\":\"unlink\""), "{out}");
    assert!(out.contains("\"retracted\":true"), "{out}");
    assert!(out.contains(&derived), "{out}");
    let (_, show_post_unlink, _) = run(&h, &["knowledge", "show", "--repo", "krepo", "--id", &old]);
    assert_eq!(
        field_num(&show_post_unlink, "generation"),
        gen_before_unlink + 1,
        "a successful retraction must bump the generation once"
    );
    // Retraction hides the relation by default and keeps it for audit reads.
    let (code, out, _) = run(
        &h,
        &[
            "knowledge",
            "related",
            "--repo",
            "krepo",
            "--id",
            &old,
            "--direction",
            "inbound",
        ],
    );
    assert_eq!(code, 0);
    assert!(out.contains("\"total\":0"), "{out}");
    let (code, out, _) = run(
        &h,
        &[
            "knowledge",
            "related",
            "--repo",
            "krepo",
            "--id",
            &old,
            "--direction",
            "inbound",
            "--include-retracted",
        ],
    );
    assert_eq!(code, 0);
    assert!(out.contains(&derived), "{out}");
    assert!(out.contains("\"retracted\":true"), "{out}");
    // Repeating the retraction fails (exit 3) and changes nothing.
    let (_, show_before, _) = run(&h, &["knowledge", "show", "--repo", "krepo", "--id", &old]);
    let generation = field_num(&show_before, "generation");
    let (code, _, err) = run_stdin(
        &h,
        &[
            "knowledge",
            "unlink",
            "--repo",
            "krepo",
            "--relation",
            &derived,
            "--input",
            "-",
        ],
        br#"{"author":"a","reason":"again"}"#,
    );
    assert_eq!(code, 3, "repeat retraction: {err}");
    assert!(err.contains("already retracted"), "{err}");
    let (_, show_after, _) = run(&h, &["knowledge", "show", "--repo", "krepo", "--id", &old]);
    assert_eq!(
        field_num(&show_after, "generation"),
        generation,
        "a rejected retraction must not bump the generation"
    );
}

#[test]
fn lifecycle_over_limit_relation_ids_stay_strictly_validated() {
    let h = home("lifecycle-relref");
    let dir = mkrepo(&h, "krepo");
    init(&h, &dir, "krepo");
    assert_eq!(create(&h, "krepo", "d1", "First", "alpha zqxref body").0, 0);
    let unlink_input = br#"{"author":"a","reason":"junk"}"#;
    let (_, show_before, _) = run(&h, &["knowledge", "show", "--repo", "krepo", "--id", "d1"]);
    let generation = field_num(&show_before, "generation");
    // Arbitrary over-limit caller ids are rejected as input errors, whatever
    // the derived-format exemption: nothing over the cap without the exact
    // derived shape.
    let junk = "x".repeat(200);
    let (code, _, err) = run_stdin(
        &h,
        &[
            "knowledge",
            "unlink",
            "--repo",
            "krepo",
            "--relation",
            &junk,
            "--input",
            "-",
        ],
        unlink_input,
    );
    assert_eq!(code, 2, "arbitrary over-limit id: {err}");
    assert!(err.contains("invalid relation id"), "{err}");
    // Malformed or unrelated over-limit shapes are rejected too.
    for malformed in [
        format!("supersedes:{}:x:{}:1", "a".repeat(128), "b".repeat(128)),
        format!("supersedes:{}:1:{}", "a".repeat(128), "b".repeat(128)),
        format!("supersedes:{}:1:{}:1:9", "a".repeat(128), "b".repeat(128)),
        format!("superseded:{}:1:{}:1", "a".repeat(128), "b".repeat(128)),
    ] {
        let (code, _, err) = run_stdin(
            &h,
            &[
                "knowledge",
                "unlink",
                "--repo",
                "krepo",
                "--relation",
                &malformed,
                "--input",
                "-",
            ],
            unlink_input,
        );
        assert_eq!(code, 2, "malformed over-limit id: {err}");
        assert!(err.contains("invalid relation id"), "{err}");
    }
    // A shape-valid but non-existent derived id is a storage conflict: it
    // reaches the store and resolves to "unknown", never to a mutation.
    let unrelated = format!("supersedes:{}:1:{}:2", "a".repeat(128), "b".repeat(128));
    let (code, _, err) = run_stdin(
        &h,
        &[
            "knowledge",
            "unlink",
            "--repo",
            "krepo",
            "--relation",
            &unrelated,
            "--input",
            "-",
        ],
        unlink_input,
    );
    assert_eq!(code, 3, "unrelated derived id: {err}");
    assert!(err.contains("unknown relation id"), "{err}");
    // No rejection mutated the store: generation, history, relations and the
    // store version are untouched.
    let (_, show_after, _) = run(&h, &["knowledge", "show", "--repo", "krepo", "--id", "d1"]);
    assert_eq!(field_num(&show_after, "generation"), generation);
    let (_, history, _) = run(
        &h,
        &["knowledge", "history", "--repo", "krepo", "--id", "d1"],
    );
    assert!(history.contains("\"total\":1"), "{history}");
    let (_, related, _) = run(
        &h,
        &[
            "knowledge",
            "related",
            "--repo",
            "krepo",
            "--id",
            "d1",
            "--direction",
            "outbound",
            "--include-retracted",
        ],
    );
    assert!(related.contains("\"total\":0"), "{related}");
    // A rejected retraction cannot even leave an orphan row: either the v2
    // retractions table was never created, or it is empty.
    let conn = rusqlite::Connection::open(db_path(&h, "krepo")).unwrap();
    let retractions_table: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'retractions'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let orphan_rows: i64 = if retractions_table == 0 {
        0
    } else {
        conn.query_row("SELECT count(*) FROM retractions", [], |r| r.get(0))
            .unwrap()
    };
    conn.close().unwrap();
    assert_eq!(orphan_rows, 0);
    assert_eq!(user_version(&h, "krepo"), 1, "rejections must not migrate");
}

#[test]
fn lifecycle_out_of_range_supersede_target_revision_rejected_atomically() {
    let h = home("lifecycle-targetrange");
    let dir = mkrepo(&h, "krepo");
    init(&h, &dir, "krepo");
    assert_eq!(
        create(&h, "krepo", "d1", "First", "alpha zqxrange body").0,
        0
    );
    assert_eq!(
        create(&h, "krepo", "d2", "Second", "beta zqxrange body").0,
        0
    );
    let (_, show_before, _) = run(&h, &["knowledge", "show", "--repo", "krepo", "--id", "d1"]);
    let generation = field_num(&show_before, "generation");
    // A target revision beyond the representable range is an input error: the
    // CLI validator rejects it before any store write (exit 2). The store's
    // own out-of-range guard stays as defense in depth (unit-tested).
    let (code, _, err) = update(
        &h,
        "krepo",
        "d1",
        1,
        &supersede_input("First", "d2", u64::MAX, "overflow"),
    );
    assert_eq!(code, 2, "out-of-range target revision: {err}");
    assert!(err.contains("out of range"), "{err}");
    // Nothing mutated: the CLI rejects before the store is even opened, so
    // this pins the CLI layer (in-store rejection rollback is pinned by the
    // store unit test); head, generation, history, lineage and store version
    // stay untouched.
    let (_, show_after, _) = run(&h, &["knowledge", "show", "--repo", "krepo", "--id", "d1"]);
    assert!(show_after.contains("\"status\":\"active\""), "{show_after}");
    assert!(show_after.contains("\"revision\":1"), "{show_after}");
    assert_eq!(field_num(&show_after, "generation"), generation);
    assert_eq!(
        user_version(&h, "krepo"),
        1,
        "a CLI-rejected transition must not touch the store version"
    );
    let (_, history, _) = run(
        &h,
        &["knowledge", "history", "--repo", "krepo", "--id", "d1"],
    );
    assert!(history.contains("\"total\":1"), "{history}");
    let (_, related, _) = run(
        &h,
        &[
            "knowledge",
            "related",
            "--repo",
            "krepo",
            "--id",
            "d1",
            "--direction",
            "outbound",
            "--include-retracted",
        ],
    );
    assert!(related.contains("\"total\":0"), "{related}");
}

#[test]
fn lifecycle_search_visibility_flags_are_independent_via_cli() {
    let h = home("lifecycle-visibility");
    let dir = mkrepo(&h, "krepo");
    init(&h, &dir, "krepo");
    assert_eq!(create(&h, "krepo", "r1", "Active", "zqxvis active").0, 0);
    assert_eq!(
        create(&h, "krepo", "r2", "Archived", "zqxvis archived").0,
        0
    );
    assert_eq!(
        create(&h, "krepo", "r3", "Superseded", "zqxvis superseded").0,
        0
    );
    assert_eq!(
        create(&h, "krepo", "r4", "Retracted", "zqxvis retracted").0,
        0
    );
    let archived = "{\"title\":\"Archived\",\"body\":\"zqxvis archived\",\"status\":\"archived\",\"author\":\"a\",\"evidence\":[]}";
    assert_eq!(update(&h, "krepo", "r2", 1, archived).0, 0);
    let superseded = "{\"title\":\"Superseded\",\"body\":\"zqxvis superseded\",\"status\":\"superseded\",\"author\":\"a\",\"evidence\":[],\"superseded_by\":{\"id\":\"r1\",\"revision\":1},\"reason\":\"replaced\"}";
    assert_eq!(update(&h, "krepo", "r3", 1, superseded).0, 0);
    let retracted = "{\"title\":\"Retracted\",\"body\":\"zqxvis retracted\",\"status\":\"retracted\",\"author\":\"a\",\"evidence\":[],\"reason\":\"wrong\"}";
    assert_eq!(update(&h, "krepo", "r4", 1, retracted).0, 0);

    fn visibility(h: &Path, flags: &[&str]) -> String {
        let mut args = vec![
            "knowledge",
            "search",
            "--repo",
            "krepo",
            "--query",
            "zqxvis",
        ];
        args.extend_from_slice(flags);
        let (code, out, err) = run(h, &args);
        assert_eq!(code, 0, "search: {err}");
        out
    }
    let hits = |out: &str, id: &str| out.contains(&format!("\"id\":\"{id}\""));
    // Default view: active only.
    let all = visibility(&h, &[]);
    assert!(
        hits(&all, "r1") && !hits(&all, "r2") && !hits(&all, "r3") && !hits(&all, "r4"),
        "{all}"
    );
    // Each historical visibility is independently opt-in.
    let archived = visibility(&h, &["--include-archived"]);
    assert!(
        hits(&archived, "r1")
            && hits(&archived, "r2")
            && !hits(&archived, "r3")
            && !hits(&archived, "r4"),
        "{archived}"
    );
    let superseded = visibility(&h, &["--include-superseded"]);
    assert!(
        hits(&superseded, "r1")
            && !hits(&superseded, "r2")
            && hits(&superseded, "r3")
            && !hits(&superseded, "r4"),
        "{superseded}"
    );
    let retracted = visibility(&h, &["--include-retracted"]);
    assert!(
        hits(&retracted, "r1")
            && !hits(&retracted, "r2")
            && !hits(&retracted, "r3")
            && hits(&retracted, "r4"),
        "{retracted}"
    );
    let everything = visibility(
        &h,
        &[
            "--include-archived",
            "--include-superseded",
            "--include-retracted",
        ],
    );
    assert!(
        hits(&everything, "r1")
            && hits(&everything, "r2")
            && hits(&everything, "r3")
            && hits(&everything, "r4"),
        "{everything}"
    );
    // Historical hits carry their lifecycle payload.
    assert!(
        superseded.contains("\"superseded_by\":{\"id\":\"r1\",\"revision\":1}"),
        "{superseded}"
    );
    assert!(
        retracted.contains("\"retracted_reason\":\"wrong\""),
        "{retracted}"
    );
}

#[test]
fn lifecycle_plain_foundation_flow_never_migrates() {
    let h = home("stillv1");
    let dir = mkrepo(&h, "krepo");
    init(&h, &dir, "krepo");
    assert_eq!(create(&h, "krepo", "d1", "First", "alpha zqxbody").0, 0);
    // Archive and restore are Foundation transitions: no migration.
    let archived = "{\"title\":\"First\",\"body\":\"alpha zqxbody\",\"status\":\"archived\",\"author\":\"a\",\"evidence\":[]}";
    assert_eq!(update(&h, "krepo", "d1", 1, archived).0, 0);
    let restored = "{\"title\":\"First\",\"body\":\"alpha zqxbody\",\"status\":\"active\",\"author\":\"a\",\"evidence\":[]}";
    assert_eq!(update(&h, "krepo", "d1", 2, restored).0, 0);
    let (code, out, err) = run(
        &h,
        &[
            "knowledge",
            "search",
            "--repo",
            "krepo",
            "--query",
            "zqxbody",
            "--include-archived",
        ],
    );
    assert_eq!(code, 0, "search: {err}");
    assert!(out.contains("\"id\":\"d1\""), "{out}");
    // History visibility flags are accepted against a v1 store too, and the
    // absent lifecycle tables simply contribute no weights: no migration, no
    // error, no fabricated lifecycle fields.
    let (code, out, err) = run(
        &h,
        &[
            "knowledge",
            "search",
            "--repo",
            "krepo",
            "--query",
            "zqxbody",
            "--include-superseded",
            "--include-retracted",
        ],
    );
    assert_eq!(code, 0, "search: {err}");
    assert!(out.contains("\"id\":\"d1\""), "{out}");
    assert!(!out.contains("superseded_by"), "{out}");
    assert_eq!(
        user_version(&h, "krepo"),
        1,
        "Foundation writes and reads must never migrate a v1 store"
    );
}

// --- Full-stage provenance (G-PROV) ----------------------------------------

fn confirm_input(title: &str, body: &str) -> String {
    format!(
        "{{\"title\":\"{title}\",\"body\":\"{body}\",\"status\":\"active\",\"author\":\"a\",\
         \"evidence\":[],\"confirm_state\":\"confirmed\"}}"
    )
}

#[allow(clippy::too_many_arguments)]
fn link_input(
    relation_id: &str,
    src: &str,
    src_rev: u64,
    dst: &str,
    dst_rev: u64,
    kind: &str,
    rationale: &str,
    confirm: bool,
) -> String {
    let confirm_field = if confirm {
        ",\"confirm_state\":\"confirmed\""
    } else {
        ""
    };
    format!(
        "{{\"relation_id\":\"{relation_id}\",\"src_id\":\"{src}\",\"src_revision\":{src_rev},\
         \"dst_id\":\"{dst}\",\"dst_revision\":{dst_rev},\"kind\":\"{kind}\",\"author\":\"a\",\
         \"rationale\":\"{rationale}\"{confirm_field}}}"
    )
}

fn link(home: &Path, repo: &str, input: &str) -> (i32, String, String) {
    run_stdin(
        home,
        &["knowledge", "link", "--repo", repo, "--input", "-"],
        input.as_bytes(),
    )
}

#[test]
fn provenance_confirm_round_trip_via_cli() {
    let h = home("provenance");
    let dir = mkrepo(&h, "krepo");
    init(&h, &dir, "krepo");
    assert_eq!(create(&h, "krepo", "d1", "First", "alpha zqxbody").0, 0);
    assert_eq!(user_version(&h, "krepo"), 1);
    // A v1 store has no confirmation state to report.
    let (code, out, _) = run(&h, &["knowledge", "show", "--repo", "krepo", "--id", "d1"]);
    assert_eq!(code, 0);
    assert!(
        !out.contains("confirm_state"),
        "v1 output must omit it: {out}"
    );

    // An explicit confirmation writes a confirming revision and migrates.
    let (code, out, err) = update(
        &h,
        "krepo",
        "d1",
        1,
        &confirm_input("First", "alpha zqxbody"),
    );
    assert_eq!(code, 0, "confirm: {err}");
    assert!(out.contains("\"confirm_state\":\"confirmed\""), "{out}");
    assert!(out.contains("\"revision\":2"), "{out}");
    assert_eq!(user_version(&h, "krepo"), 2);
    // Confirmation never implies verification or evidence.
    assert!(out.contains("\"verification\":\"Unverified\""), "{out}");
    // On the migrated store the confirmed revision reports confirmed and the
    // revision it confirms reads as the candidate it was.
    let (code, out, _) = run(&h, &["knowledge", "show", "--repo", "krepo", "--id", "d1"]);
    assert_eq!(code, 0);
    assert!(out.contains("\"confirm_state\":\"confirmed\""), "{out}");
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
    assert!(out.contains("\"confirm_state\":\"candidate\""), "{out}");

    // Re-confirming the identical head is a successful no-op: no new
    // revision, no generation bump.
    let gen_before = field_num(&out, "generation");
    let (code, out, err) = update(
        &h,
        "krepo",
        "d1",
        2,
        &confirm_input("First", "alpha zqxbody"),
    );
    assert_eq!(code, 0, "reconfirm: {err}");
    assert!(out.contains("\"confirm_state\":\"confirmed\""), "{out}");
    assert!(
        out.contains("\"revision\":2"),
        "the confirming revision is unchanged: {out}"
    );
    let (code, history_out, _) = run(
        &h,
        &["knowledge", "history", "--repo", "krepo", "--id", "d1"],
    );
    assert_eq!(code, 0);
    assert!(history_out.contains("\"total\":2"), "{history_out}");
    let (code, show_out, _) = run(&h, &["knowledge", "show", "--repo", "krepo", "--id", "d1"]);
    assert_eq!(code, 0);
    assert_eq!(
        field_num(&show_out, "generation"),
        gen_before,
        "a no-op confirmation must not bump the generation"
    );

    // Search reports the confirmed state on the hit.
    let (code, out, err) = run(
        &h,
        &[
            "knowledge",
            "search",
            "--repo",
            "krepo",
            "--query",
            "zqxbody",
        ],
    );
    assert_eq!(code, 0, "search: {err}");
    assert!(out.contains("\"id\":\"d1\""), "{out}");
    assert!(out.contains("\"confirm_state\":\"confirmed\""), "{out}");
}

#[test]
fn provenance_confirm_requests_are_strict() {
    let h = home("provenance-strict");
    let dir = mkrepo(&h, "krepo");
    init(&h, &dir, "krepo");
    assert_eq!(create(&h, "krepo", "d1", "First", "alpha zqxbody").0, 0);
    assert_eq!(create(&h, "krepo", "d2", "Second", "beta zqxbody").0, 0);
    // Only the literal request parses; "candidate" is a state, not a request.
    for bad in ["\"candidate\"", "true", "1", "null"] {
        let input = format!(
            "{{\"title\":\"First\",\"body\":\"alpha zqxbody\",\"status\":\"active\",\
             \"author\":\"a\",\"evidence\":[],\"confirm_state\":{bad}}}"
        );
        let (code, _, err) = update(&h, "krepo", "d1", 1, &input);
        assert_eq!(code, 2, "confirm_state {bad}: {err}");
    }
    // Confirmation never combines with a lifecycle transition.
    let combined = "{\"title\":\"First\",\"body\":\"alpha zqxbody\",\"status\":\"superseded\",\
         \"author\":\"a\",\"evidence\":[],\"superseded_by\":{\"id\":\"d2\",\"revision\":1},\
         \"reason\":\"r\",\"confirm_state\":\"confirmed\"}";
    let (code, _, err) = update(&h, "krepo", "d1", 1, combined);
    assert_eq!(code, 2, "combined: {err}");
    // A stale expectation is a conflict (exit 3) and rolls back with nothing
    // written; the store is still v1 afterwards.
    let (code, _, err) = update(
        &h,
        "krepo",
        "d1",
        7,
        &confirm_input("First", "alpha zqxbody"),
    );
    assert_eq!(code, 3, "stale confirm: {err}");
    assert!(err.contains("conflict"), "{err}");
    let (code, out, _) = run(
        &h,
        &["knowledge", "history", "--repo", "krepo", "--id", "d1"],
    );
    assert_eq!(code, 0);
    assert!(out.contains("\"total\":1"), "{out}");
    assert_eq!(
        user_version(&h, "krepo"),
        1,
        "rejected confirmations must roll their migration back"
    );
}

#[test]
fn provenance_relation_confirm_via_cli() {
    let h = home("provenance-rel");
    let dir = mkrepo(&h, "krepo");
    init(&h, &dir, "krepo");
    assert_eq!(create(&h, "krepo", "d1", "First", "alpha zqxbody").0, 0);
    assert_eq!(create(&h, "krepo", "d2", "Second", "beta zqxbody").0, 0);
    // An ordinary link on a v1 store carries no confirmation state.
    let (code, out, err) = link(
        &h,
        "krepo",
        &link_input("r1", "d1", 1, "d2", 1, "supports", "related work", false),
    );
    assert_eq!(code, 0, "link: {err}");
    assert!(
        !out.contains("confirm_state"),
        "v1 output must omit it: {out}"
    );
    assert_eq!(user_version(&h, "krepo"), 1);
    // A confirmation request whose link is rejected (unknown endpoint) rolls
    // its migration back: the v1 store is untouched.
    let (code, _, err) = link(
        &h,
        "krepo",
        &link_input("r9", "d1", 99, "d2", 1, "supports", "related work", true),
    );
    assert_eq!(code, 3, "unknown endpoint: {err}");
    assert_eq!(
        user_version(&h, "krepo"),
        1,
        "a rejected link confirmation must roll its migration back"
    );
    // A brand-new relation confirmed in its first write migrates the store.
    let (code, out, err) = link(
        &h,
        "krepo",
        &link_input("r2", "d2", 1, "d1", 1, "contradicts", "tension", true),
    );
    assert_eq!(code, 0, "link r2: {err}");
    assert!(out.contains("\"confirm_state\":\"confirmed\""), "{out}");
    assert_eq!(user_version(&h, "krepo"), 2);
    // Confirming the byte-identical row is a promotion, not a duplicate.
    let (code, out, err) = link(
        &h,
        "krepo",
        &link_input("r1", "d1", 1, "d2", 1, "supports", "related work", true),
    );
    assert_eq!(code, 0, "promote: {err}");
    assert!(out.contains("\"confirm_state\":\"confirmed\""), "{out}");
    let gen = field_num(&out, "generation");
    assert_eq!(user_version(&h, "krepo"), 2);
    // Repeating the confirmation succeeds without writing: same generation,
    // still one relation.
    let (code, out, err) = link(
        &h,
        "krepo",
        &link_input("r1", "d1", 1, "d2", 1, "supports", "related work", true),
    );
    assert_eq!(code, 0, "reconfirm: {err}");
    assert_eq!(field_num(&out, "generation"), gen, "{out}");
    let (code, out, _) = run(
        &h,
        &[
            "knowledge",
            "related",
            "--repo",
            "krepo",
            "--id",
            "d1",
            "--direction",
            "outbound",
        ],
    );
    assert_eq!(code, 0);
    assert!(out.contains("\"total\":1"), "{out}");
    assert!(out.contains("\"confirm_state\":\"confirmed\""), "{out}");
    // Identical re-link without the request, and a changed shape with it, are
    // both conflicts.
    let (code, _, err) = link(
        &h,
        "krepo",
        &link_input("r1", "d1", 1, "d2", 1, "supports", "related work", false),
    );
    assert_eq!(code, 3, "plain relink: {err}");
    assert!(err.contains("duplicate relation id"), "{err}");
    let (code, _, err) = link(
        &h,
        "krepo",
        &link_input("r1", "d1", 1, "d2", 1, "supports", "different", true),
    );
    assert_eq!(code, 3, "changed shape: {err}");
    // Retraction does not erase the confirmation, and a retracted relation is
    // never revived by re-link confirmation.
    let unlink_input = "{\"author\":\"a\",\"reason\":\"mistake\"}";
    let (code, _, err) = run_stdin(
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
        unlink_input.as_bytes(),
    );
    assert_eq!(code, 0, "unlink: {err}");
    let (code, out, _) = run(
        &h,
        &[
            "knowledge",
            "related",
            "--repo",
            "krepo",
            "--id",
            "d1",
            "--direction",
            "outbound",
            "--include-retracted",
        ],
    );
    assert_eq!(code, 0);
    assert!(out.contains("\"retracted\":true"), "{out}");
    assert!(out.contains("\"confirm_state\":\"confirmed\""), "{out}");
    let (code, _, err) = link(
        &h,
        "krepo",
        &link_input("r1", "d1", 1, "d2", 1, "supports", "related work", true),
    );
    assert_eq!(code, 3, "revive retracted: {err}");
}

// --- G-PORT: canonical JSONL/Markdown export + import -----------------------

fn export(home: &Path, repo: &str, extra: &[&str]) -> (i32, String, String) {
    let mut args = vec!["knowledge", "export", "--repo", repo];
    args.extend_from_slice(extra);
    run(home, &args)
}

fn import(home: &Path, repo: &str, file: &Path) -> (i32, String, String) {
    run(
        home,
        &[
            "knowledge",
            "import",
            "--repo",
            repo,
            "--file",
            file.to_str().unwrap(),
        ],
    )
}

fn read_text(path: &Path) -> String {
    String::from_utf8(std::fs::read(path).unwrap()).unwrap()
}

fn write_text(path: &Path, text: &str) {
    std::fs::write(path, text).unwrap();
}

/// A minimal canonical payload line pair for hand-built import files.
fn meta_line(records: usize, relations: usize) -> String {
    format!(
        "{{\"type\":\"meta\",\"format\":\"riltra-knowledge\",\"version\":1,\"repo\":\"krepo\",\"records\":{records},\"relations\":{relations}}}"
    )
}

fn record_line(id: &str, title: &str, body: &str) -> String {
    format!(
        "{{\"type\":\"record\",\"id\":\"{id}\",\"kind\":\"decision\",\"head_revision\":1,\"status\":\"active\",\
         \"revisions\":[{{\"revision\":1,\"title\":\"{title}\",\"body\":\"{body}\",\"status\":\"active\",\
         \"author\":\"a\",\"created_unix\":10,\"evidence\":[],\"events\":[]}}]}}"
    )
}

/// Rich store: record supersede/retract/confirm events, an evidence binding, a
/// benchmark measurement, a confirmed and a retracted relation, and the
/// implicit `supersedes` relation written by the supersede transition.
fn port_seed(home: &Path) -> PathBuf {
    let dir = mkrepo(home, "krepo");
    init(home, &dir, "krepo");
    std::fs::write(dir.join("a.py"), "alpha beta gamma").unwrap();
    let (code, _, err) = run_stdin(
        home,
        &["knowledge", "create", "--repo", "krepo", "--input", "-"],
        br#"{"id":"d1","kind":"decision","title":"First","body":"body one","status":"active","author":"a","evidence":[{"path":"a.py","byte_start":0,"byte_end":5}]}"#,
    );
    assert_eq!(code, 0, "create d1: {err}");
    assert_eq!(create(home, "krepo", "d2", "Second", "body two").0, 0);
    assert_eq!(create(home, "krepo", "d3", "Third", "body three").0, 0);
    let (code, _, err) = run_stdin(
        home,
        &["knowledge", "create", "--repo", "krepo", "--input", "-"],
        br#"{"id":"b1","kind":"benchmark","title":"Perf","body":"body bench","status":"active","author":"a","evidence":[],"measurement":{"method":"m","environment":"e","sample_count":3,"metrics":{"latency":{"value":1.5,"unit":"ms"},"p99":{"value":42.0,"unit":"ms"}}}}"#,
    );
    assert_eq!(code, 0, "create b1: {err}");
    let (code, _, err) = update(
        home,
        "krepo",
        "d1",
        1,
        &supersede_input("First v2", "d2", 1, "replaced by d2"),
    );
    assert_eq!(code, 0, "supersede d1: {err}");
    let (code, _, err) = update(
        home,
        "krepo",
        "d2",
        1,
        &confirm_input("Second v2", "body two v2"),
    );
    assert_eq!(code, 0, "confirm d2: {err}");
    let (code, _, err) = update(home, "krepo", "d3", 1, &retract_input("Third", "was wrong"));
    assert_eq!(code, 0, "retract d3: {err}");
    let (code, _, err) = link(
        home,
        "krepo",
        &link_input("r1", "d2", 2, "d1", 2, "supports", "the reason", true),
    );
    assert_eq!(code, 0, "link r1: {err}");
    let (code, _, err) = link(
        home,
        "krepo",
        &link_input("r2", "d3", 2, "d1", 2, "relates_to", "maybe", false),
    );
    assert_eq!(code, 0, "link r2: {err}");
    let (code, _, err) = run_stdin(
        home,
        &[
            "knowledge",
            "unlink",
            "--repo",
            "krepo",
            "--relation",
            "r2",
            "--input",
            "-",
        ],
        br#"{"author":"a","reason":"mistake"}"#,
    );
    assert_eq!(code, 0, "unlink r2: {err}");
    dir
}

#[test]
fn port_jsonl_round_trip_is_byte_identical_and_preserves_audit_events() {
    let h1 = home("port-jsonl-1");
    port_seed(&h1);
    let x = h1.join("x.jsonl");
    let (code, out, err) = export(&h1, "krepo", &["--out", x.to_str().unwrap()]);
    assert_eq!(code, 0, "export: {err}");
    assert!(out.contains("\"operation\":\"export\""), "{out}");
    let x_text = read_text(&x);
    // Lifecycle audit metadata rides as structured events; the withdrawn
    // draft fields never appear.
    assert!(x_text.contains("\"event\":\"supersede\""), "{x_text}");
    assert!(
        x_text.contains("\"target_id\":\"d2\",\"target_revision\":1,\"reason\":\"replaced by d2\""),
        "{x_text}"
    );
    assert!(x_text.contains("\"event\":\"confirm\""), "{x_text}");
    assert!(
        x_text.contains("\"event\":\"retract\",\"reason\":\"was wrong\""),
        "{x_text}"
    );
    assert!(x_text.contains("\"reason\":\"mistake\""), "{x_text}");
    assert!(
        x_text.contains("\"relation_id\":\"supersedes:d2:1:d1:2\""),
        "{x_text}"
    );
    assert!(x_text.contains("\"digest\":"), "{x_text}");
    assert!(x_text.contains("\"value\":1.5"), "{x_text}");
    assert!(x_text.contains("\"value\":42.0"), "{x_text}");
    for withdrawn in [
        "superseded_by",
        "confirm_state",
        "retracted_reason",
        "retracted_author",
        "retraction_author",
        "retraction_reason",
    ] {
        assert!(
            !x_text.contains(withdrawn),
            "withdrawn field {withdrawn}: {x_text}"
        );
    }
    assert_eq!(field_num(&x_text, "records"), 4, "{x_text}");
    assert_eq!(field_num(&x_text, "relations"), 3, "{x_text}");

    // Independent home, same repo name: byte-identical re-export.
    let h2 = home("port-jsonl-2");
    let dir2 = mkrepo(&h2, "krepo");
    init(&h2, &dir2, "krepo");
    let (code, out, err) = import(&h2, "krepo", &x);
    assert_eq!(code, 0, "import: {err}");
    assert!(out.contains("\"operation\":\"import\""), "{out}");
    assert_eq!(
        user_version(&h2, "krepo"),
        2,
        "imported events must migrate"
    );
    let y = h2.join("y.jsonl");
    let (code, _, err) = export(&h2, "krepo", &["--out", y.to_str().unwrap()]);
    assert_eq!(code, 0, "export 2: {err}");
    assert_eq!(
        read_text(&y),
        x_text,
        "export -> import -> export must be byte-identical"
    );
    // User-facing state semantics are rebuilt from the imported events.
    let (code, out, err) = run(&h2, &["knowledge", "show", "--repo", "krepo", "--id", "d1"]);
    assert_eq!(code, 0, "show d1: {err}");
    assert!(
        out.contains("\"superseded_by\":{\"id\":\"d2\",\"revision\":1}"),
        "{out}"
    );
    let (code, out, err) = run(&h2, &["knowledge", "show", "--repo", "krepo", "--id", "d2"]);
    assert_eq!(code, 0, "show d2: {err}");
    assert!(out.contains("\"confirm_state\":\"confirmed\""), "{out}");
    let (code, out, err) = run(
        &h2,
        &[
            "knowledge",
            "related",
            "--repo",
            "krepo",
            "--id",
            "d3",
            "--direction",
            "outbound",
            "--include-retracted",
        ],
    );
    assert_eq!(code, 0, "related: {err}");
    assert!(out.contains("\"retracted\":true"), "{out}");

    // Repeated identical import: no writes, no generation bump.
    let g = field_num(&out, "generation");
    let (code, out, err) = import(&h2, "krepo", &x);
    assert_eq!(code, 0, "re-import: {err}");
    assert!(out.contains("\"records_inserted\":0"), "{out}");
    assert!(out.contains("\"relations_inserted\":0"), "{out}");
    assert_eq!(
        field_num(&out, "generation"),
        g,
        "no-op import must not bump generation"
    );
}

#[test]
fn port_markdown_round_trip_and_import_extraction_rules() {
    let h1 = home("port-md-1");
    port_seed(&h1);
    let m1 = h1.join("m1.md");
    let (code, _, err) = export(
        &h1,
        "krepo",
        &["--format", "md", "--out", m1.to_str().unwrap()],
    );
    assert_eq!(code, 0, "md export: {err}");
    let m1_text = read_text(&m1);
    assert!(m1_text.contains("# Riltra knowledge export"), "{m1_text}");
    assert!(m1_text.contains("```json"), "{m1_text}");
    assert!(
        m1_text.contains("## d1 — decision (current revision 2, superseded)"),
        "{m1_text}"
    );
    let x1 = h1.join("x.jsonl");
    assert_eq!(export(&h1, "krepo", &["--out", x1.to_str().unwrap()]).0, 0);

    // Markdown import carries the same canonical content as the JSONL export.
    let h2 = home("port-md-2");
    let dir2 = mkrepo(&h2, "krepo");
    init(&h2, &dir2, "krepo");
    let (code, _, err) = import(&h2, "krepo", &m1);
    assert_eq!(code, 0, "md import: {err}");
    let x2 = h2.join("x2.jsonl");
    assert_eq!(export(&h2, "krepo", &["--out", x2.to_str().unwrap()]).0, 0);
    assert_eq!(read_text(&x2), read_text(&x1));
    let m2 = h2.join("m2.md");
    assert_eq!(
        export(
            &h2,
            "krepo",
            &["--format", "md", "--out", m2.to_str().unwrap()]
        )
        .0,
        0
    );
    assert_eq!(read_text(&m2), m1_text, "markdown export is deterministic");

    // Prose JSON-lookalikes outside fenced blocks are presentational only.
    let poisoned = h2.join("poisoned.md");
    let fake = "{\"type\":\"record\",\"id\":\"zzz\",\"kind\":\"decision\",\"head_revision\":1,\"status\":\"active\",\"revisions\":[]}";
    write_text(
        &poisoned,
        &format!("=== notes ===\n{fake}\n=== payload ===\n{m1_text}"),
    );
    let h3 = home("port-md-3");
    let dir3 = mkrepo(&h3, "krepo");
    init(&h3, &dir3, "krepo");
    let (code, _, err) = import(&h3, "krepo", &poisoned);
    assert_eq!(code, 0, "prose must be ignored: {err}");
    let x3 = h3.join("x3.jsonl");
    assert_eq!(export(&h3, "krepo", &["--out", x3.to_str().unwrap()]).0, 0);
    assert_eq!(read_text(&x3), read_text(&x1));

    // Missing fences, invalid fenced JSON, and an unterminated fence: exit 2,
    // store untouched.
    let h4 = home("port-md-4");
    let dir4 = mkrepo(&h4, "krepo");
    init(&h4, &dir4, "krepo");
    assert_eq!(create(&h4, "krepo", "d1", "First", "body").0, 0);
    let before = std::fs::read(db_path(&h4, "krepo")).unwrap();
    for (tag, text, needle) in [
        ("nofence", "just prose\nno payload\n", "fenced"),
        (
            "badjson",
            "```json\n{\"type\":\"meta\",\n```\n",
            "invalid import payload",
        ),
        (
            "unterminated",
            "```json\n{\"type\":\"meta\"}\n",
            "unterminated",
        ),
    ] {
        let f = h4.join(format!("{tag}.md"));
        write_text(&f, text);
        let (code, _, err) = import(&h4, "krepo", &f);
        assert_eq!(code, 2, "{tag} must be invalid: {err}");
        assert!(err.contains(needle), "{tag}: {err}");
    }
    assert_eq!(
        std::fs::read(db_path(&h4, "krepo")).unwrap(),
        before,
        "rejected markdown imports must not touch the store"
    );
}

#[test]
fn port_import_rejections_are_classified_and_atomic() {
    let h = home("port-reject");
    let dir = mkrepo(&h, "krepo");
    init(&h, &dir, "krepo");
    assert_eq!(create(&h, "krepo", "d1", "First", "body one").0, 0);
    let before = std::fs::read(db_path(&h, "krepo")).unwrap();
    let base = h.join("base.jsonl");
    let (code, out, _) = export(&h, "krepo", &["--out", base.to_str().unwrap()]);
    assert_eq!(code, 0);
    let g = field_num(&out, "generation");

    let d1_line = record_line("d1", "First", "body one");
    let unsorted = format!("{}\n{}\n", record_line("d2", "Two", "b2"), d1_line);
    let cases: Vec<(&str, String, i32, &str)> = vec![
        (
            "truncated",
            format!("{}\n{{\"type\":\"record\",\"id\":\"d1\",\n", meta_line(0, 0)),
            2,
            "invalid import payload",
        ),
        (
            "badversion",
            "{\"type\":\"meta\",\"format\":\"riltra-knowledge\",\"version\":2,\"repo\":\"krepo\",\"records\":0,\"relations\":0}\n".to_string(),
            2,
            "invalid import payload",
        ),
        (
            "dup",
            format!("{}\n{d1_line}\n{d1_line}\n", meta_line(2, 0)),
            3,
            "corrupt import file",
        ),
        (
            "counts",
            format!("{}\n{d1_line}\n", meta_line(9, 0)),
            3,
            "corrupt import file",
        ),
        (
            "unsorted",
            format!("{}\n{unsorted}\n", meta_line(2, 0)),
            3,
            "corrupt import file",
        ),
        (
            "divergent",
            format!("{}\n{}\n", meta_line(1, 0), record_line("d1", "Changed", "body one")),
            3,
            "import conflict",
        ),
        (
            "endpoint",
            format!(
                "{}\n{{\"type\":\"relation\",\"relation_id\":\"r9\",\"src_id\":\"nope\",\"src_revision\":1,\"dst_id\":\"d1\",\"dst_revision\":1,\"kind\":\"relates_to\",\"author\":\"a\",\"rationale\":\"why\",\"created_unix\":20,\"events\":[]}}\n",
                meta_line(0, 1)
            ),
            2,
            "invalid import payload",
        ),
        (
            "badsupersede",
            format!(
                "{}\n{{\"type\":\"record\",\"id\":\"d9\",\"kind\":\"decision\",\"head_revision\":1,\"status\":\"active\",\"revisions\":[{{\"revision\":1,\"title\":\"Nine\",\"body\":\"b9\",\"status\":\"active\",\"author\":\"a\",\"created_unix\":10,\"evidence\":[],\"events\":[{{\"event\":\"supersede\",\"target_id\":\"nope\",\"target_revision\":1,\"reason\":\"r\",\"author\":\"a\",\"created_unix\":11}}]}}]}}\n",
                meta_line(1, 0)
            ),
            2,
            "invalid import payload",
        ),
    ];
    for (tag, text, want_code, needle) in cases {
        let f = h.join(format!("{tag}.jsonl"));
        write_text(&f, &text);
        let (code, _, err) = import(&h, "krepo", &f);
        assert_eq!(code, want_code, "{tag}: {err}");
        assert!(err.contains(needle), "{tag}: {err}");
    }
    // Oversized line and oversized file are refused before any parse or write.
    let long = h.join("longline.jsonl");
    write_text(
        &long,
        &format!(
            "{}\n{}\n",
            meta_line(0, 0),
            "x".repeat(16 * 1024 * 1024 + 1)
        ),
    );
    let (code, _, err) = import(&h, "krepo", &long);
    assert_eq!(code, 2, "long line: {err}");
    assert!(err.contains("line"), "long line: {err}");
    let huge = h.join("huge.jsonl");
    let fh = std::fs::File::create(&huge).unwrap();
    fh.set_len(256 * 1024 * 1024 + 1).unwrap();
    drop(fh);
    let (code, _, err) = import(&h, "krepo", &huge);
    assert_eq!(code, 2, "huge file: {err}");
    assert!(err.contains("too large"), "huge file: {err}");

    // Nothing above changed the store: same bytes, same generation, same head
    // revision content.
    assert_eq!(
        std::fs::read(db_path(&h, "krepo")).unwrap(),
        before,
        "rejections must not mutate the store"
    );
    let after = h.join("after.jsonl");
    let (_, out, err) = export(&h, "krepo", &["--out", after.to_str().unwrap()]);
    assert!(err.is_empty(), "post-rejection export: {err}");
    assert_eq!(field_num(&out, "generation"), g);
    assert_eq!(read_text(&after), read_text(&base));
}

#[test]
fn port_v1_store_export_import_keeps_foundation_compatibility() {
    let h = home("port-v1");
    let dir = mkrepo(&h, "krepo");
    init(&h, &dir, "krepo");
    // A store that does not exist yet exports an empty metadata-only payload
    // and creates nothing.
    let (code, out, err) = export(&h, "krepo", &[]);
    assert_eq!(code, 0, "absent store export: {err}");
    assert!(out.contains("\"records\":0,\"relations\":0"), "{out}");
    assert!(
        !db_path(&h, "krepo").exists(),
        "export must not create the store"
    );
    // Malformed input never creates a store either.
    let bad = h.join("bad.jsonl");
    write_text(&bad, "not json\n");
    let (code, _, err) = import(&h, "krepo", &bad);
    assert_eq!(code, 2, "malformed import must be exit 2: {err}");
    assert!(
        !db_path(&h, "krepo").exists(),
        "invalid payload must not create a store"
    );
    assert_eq!(create(&h, "krepo", "d1", "First", "body one").0, 0);
    assert_eq!(user_version(&h, "krepo"), 1);
    // Without `--out` the raw payload goes to stdout.
    let (code, out, err) = export(&h, "krepo", &[]);
    assert_eq!(code, 0, "export: {err}");
    assert!(out.starts_with("{\"type\":\"meta\""), "{out}");
    assert!(out.contains("\"events\":[]"), "{out}");
    assert!(!out.contains("confirm_state"), "{out}");
    assert_eq!(user_version(&h, "krepo"), 1, "export must not migrate");

    // A Foundation-only payload round-trips without raising the schema.
    let f = h.join("v1.jsonl");
    write_text(&f, &out);
    let (code, _, err) = import(&h, "krepo", &f);
    assert_eq!(code, 0, "no-op import: {err}");
    let (code, out2, err) = import(&h, "krepo", &f);
    assert_eq!(code, 0, "re-import: {err}");
    assert!(out2.contains("\"records_inserted\":0"), "{out2}");
    assert_eq!(
        user_version(&h, "krepo"),
        1,
        "a Foundation-only payload must not migrate the store"
    );

    // A payload that writes an event migrates exactly then: the confirmation
    // sits on an appended revision, the only place a coherent store can hold
    // one.
    let ev = h.join("ev.jsonl");
    write_text(
        &ev,
        &format!(
            "{}\n{{\"type\":\"record\",\"id\":\"d9\",\"kind\":\"decision\",\"head_revision\":2,\"status\":\"active\",\"revisions\":[{{\"revision\":1,\"title\":\"Nine\",\"body\":\"b9\",\"status\":\"active\",\"author\":\"a\",\"created_unix\":10,\"evidence\":[],\"events\":[]}},{{\"revision\":2,\"title\":\"Nine\",\"body\":\"b9\",\"status\":\"active\",\"author\":\"a\",\"created_unix\":11,\"evidence\":[],\"events\":[{{\"event\":\"confirm\",\"author\":\"a\",\"created_unix\":12}}]}}]}}\n",
            meta_line(1, 0)
        ),
    );
    let (code, _, err) = import(&h, "krepo", &ev);
    assert_eq!(code, 0, "event import: {err}");
    assert_eq!(
        user_version(&h, "krepo"),
        2,
        "an event-carrying payload migrates the store"
    );
}

/// A record line with explicit status and revision events.
fn record_line_with_events(id: &str, status: &str, events: &str) -> String {
    format!(
        "{{\"type\":\"record\",\"id\":\"{id}\",\"kind\":\"decision\",\"head_revision\":1,\
         \"status\":\"{status}\",\"revisions\":[{{\"revision\":1,\"title\":\"T\",\"body\":\"B\",\
         \"status\":\"{status}\",\"author\":\"a\",\"created_unix\":10,\"evidence\":[],\
         \"events\":{events}}}]}}"
    )
}

/// A supersedes relation line between two revisions.
fn supersedes_relation_line(rid: &str, src_id: &str, dst_id: &str) -> String {
    format!(
        "{{\"type\":\"relation\",\"relation_id\":\"{rid}\",\"src_id\":\"{src_id}\",\
         \"src_revision\":1,\"dst_id\":\"{dst_id}\",\"dst_revision\":1,\"kind\":\"supersedes\",\
         \"author\":\"a\",\"rationale\":\"r\",\"created_unix\":12,\"events\":[]}}"
    )
}

#[test]
fn port_import_coherence_rejections_are_atomic() {
    let supersede_event = r#"[{"event":"supersede","target_id":"d2","target_revision":1,"reason":"r","author":"a","created_unix":11}]"#;
    // Each payload violates one authored-data invariant the store enforces.
    let cases: Vec<(&str, String)> = vec![
        // Lifecycle status without its recorded event.
        (
            "status without event",
            format!(
                "{}\n{}\n",
                meta_line(1, 0),
                record_line_with_events("d1", "superseded", "[]")
            ),
        ),
        // Supersede event without the status transition.
        (
            "event without status",
            format!(
                "{}\n{}\n{}\n{}\n",
                meta_line(2, 1),
                record_line_with_events("d1", "active", supersede_event),
                record_line_with_events("d2", "active", "[]"),
                supersedes_relation_line("supersedes:d2:1:d1:1", "d2", "d1")
            ),
        ),
        // Benchmark revision without measurement data.
        (
            "benchmark without measurement",
            format!(
                "{}\n{{\"type\":\"record\",\"id\":\"b1\",\"kind\":\"benchmark\",\"head_revision\":1,\
                 \"status\":\"active\",\"revisions\":[{{\"revision\":1,\"title\":\"T\",\"body\":\"B\",\
                 \"status\":\"active\",\"author\":\"a\",\"created_unix\":10,\"evidence\":[],\"events\":[]}}]}}\n",
                meta_line(1, 0)
            ),
        ),
        // Non-benchmark revision carrying measurement data.
        (
            "measurement without benchmark",
            format!(
                "{}\n{{\"type\":\"record\",\"id\":\"d1\",\"kind\":\"decision\",\"head_revision\":1,\
                 \"status\":\"active\",\"revisions\":[{{\"revision\":1,\"title\":\"T\",\"body\":\"B\",\
                 \"status\":\"active\",\"author\":\"a\",\"created_unix\":10,\"evidence\":[],\
                 \"measurement\":{{\"method\":\"m\",\"environment\":\"e\",\"sample_count\":3,\
                 \"metrics\":[{{\"name\":\"latency\",\"value\":1.5,\"unit\":\"ms\"}}]}},\"events\":[]}}]}}\n",
                meta_line(1, 0)
            ),
        ),
        // Supersede event without its revision-pinned implicit relation.
        (
            "event without relation",
            format!(
                "{}\n{}\n{}\n",
                meta_line(2, 0),
                record_line_with_events("d1", "superseded", supersede_event),
                record_line_with_events("d2", "active", "[]")
            ),
        ),
        // Mutual replacement: a supersedes cycle.
        (
            "supersedes cycle",
            format!(
                "{}\n{}\n{}\n{}\n{}\n",
                meta_line(2, 2),
                record_line_with_events("d1", "superseded", supersede_event),
                record_line_with_events(
                    "d2",
                    "superseded",
                    r#"[{"event":"supersede","target_id":"d1","target_revision":1,"reason":"r","author":"a","created_unix":11}]"#
                ),
                supersedes_relation_line("supersedes:d1:1:d2:1", "d1", "d2"),
                supersedes_relation_line("supersedes:d2:1:d1:1", "d2", "d1")
            ),
        ),
    ];
    for (name, payload) in cases {
        let h = home("port-coh");
        let dir = mkrepo(&h, "krepo");
        init(&h, &dir, "krepo");
        let f = h.join("p.jsonl");
        write_text(&f, &payload);
        let (code, _, err) = import(&h, "krepo", &f);
        assert_eq!(code, 2, "{name}: {err}");
        assert!(err.contains("invalid import payload"), "{name}: {err}");
        assert!(
            !db_path(&h, "krepo").exists(),
            "{name}: a refused import must not create a store"
        );
    }

    // A seeded store is left byte-identical by a refused import.
    let h = home("port-coh-seed");
    port_seed(&h);
    let good = h.join("good.jsonl");
    assert_eq!(export(&h, "krepo", &["--out", good.to_str().unwrap()]).0, 0);
    let good_text = read_text(&good);
    // Drop the implicit supersedes relation and fix the declared count: the
    // event on d1 then has no revision-pinned relation to agree with.
    let filtered: String = good_text
        .lines()
        .filter(|l| !l.contains("\"supersedes:d2:1:d1:2\""))
        .collect::<Vec<_>>()
        .join("\n");
    let mutated = format!("{filtered}\n").replacen("\"relations\":3", "\"relations\":2", 1);
    let bad = h.join("bad.jsonl");
    write_text(&bad, &mutated);
    let (code, _, err) = import(&h, "krepo", &bad);
    assert_eq!(code, 2, "seeded: {err}");
    assert!(err.contains("invalid import payload"), "seeded: {err}");
    let after = h.join("after.jsonl");
    assert_eq!(
        export(&h, "krepo", &["--out", after.to_str().unwrap()]).0,
        0
    );
    assert_eq!(
        read_text(&after),
        good_text,
        "a refused import must not change the store"
    );
}

#[test]
fn port_merge_appends_supersets_skips_subsets_and_rejects_divergence() {
    let h1 = home("port-merge-1");
    let dir1 = mkrepo(&h1, "krepo");
    init(&h1, &dir1, "krepo");
    assert_eq!(create(&h1, "krepo", "d1", "First", "body one").0, 0);
    let sub = h1.join("sub.jsonl");
    assert_eq!(export(&h1, "krepo", &["--out", sub.to_str().unwrap()]).0, 0);
    assert_eq!(
        update(
            &h1,
            "krepo",
            "d1",
            1,
            "{\"title\":\"First v2\",\"body\":\"body two\",\"status\":\"active\",\"author\":\"a\",\"evidence\":[]}"
        )
        .0,
        0
    );
    let full = h1.join("full.jsonl");
    assert_eq!(
        export(&h1, "krepo", &["--out", full.to_str().unwrap()]).0,
        0
    );

    // Full import, then the identical-prefix subset: no change, no shrink.
    let h2 = home("port-merge-2");
    let dir2 = mkrepo(&h2, "krepo");
    init(&h2, &dir2, "krepo");
    let (code, _, err) = import(&h2, "krepo", &full);
    assert_eq!(code, 0, "full: {err}");
    let (code, out, err) = import(&h2, "krepo", &sub);
    assert_eq!(code, 0, "subset: {err}");
    assert!(out.contains("\"records_inserted\":0"), "{out}");
    assert!(out.contains("\"records_unchanged\":1"), "{out}");
    let y2 = h2.join("y2.jsonl");
    assert_eq!(export(&h2, "krepo", &["--out", y2.to_str().unwrap()]).0, 0);
    assert_eq!(
        read_text(&y2),
        read_text(&full),
        "subset import must not shrink the store"
    );

    // Subset first, then the superset: the missing revision is appended.
    let h3 = home("port-merge-3");
    let dir3 = mkrepo(&h3, "krepo");
    init(&h3, &dir3, "krepo");
    assert_eq!(import(&h3, "krepo", &sub).0, 0);
    let (code, out, err) = import(&h3, "krepo", &full);
    assert_eq!(code, 0, "append: {err}");
    assert!(out.contains("\"revisions_appended\":1"), "{out}");
    let y3 = h3.join("y3.jsonl");
    assert_eq!(export(&h3, "krepo", &["--out", y3.to_str().unwrap()]).0, 0);
    assert_eq!(read_text(&y3), read_text(&full));

    // Same revision, different content: conflict, atomic failure.
    let div = h3.join("div.jsonl");
    write_text(
        &div,
        &format!(
            "{}\n{}\n",
            meta_line(1, 0),
            record_line("d1", "First v9", "body nine")
        ),
    );
    let (code, _, err) = import(&h3, "krepo", &div);
    assert_eq!(code, 3, "divergence: {err}");
    assert!(err.contains("import conflict"), "{err}");
    let y4 = h3.join("y4.jsonl");
    assert_eq!(export(&h3, "krepo", &["--out", y4.to_str().unwrap()]).0, 0);
    assert_eq!(
        read_text(&y4),
        read_text(&full),
        "a conflicting import must not change the store"
    );
}

#[test]
fn port_relation_merge_rules() {
    let h1 = home("port-rel-1");
    let dir1 = mkrepo(&h1, "krepo");
    init(&h1, &dir1, "krepo");
    assert_eq!(create(&h1, "krepo", "d1", "First", "body one").0, 0);
    assert_eq!(create(&h1, "krepo", "d2", "Second", "body two").0, 0);
    assert_eq!(
        link(
            &h1,
            "krepo",
            &link_input("r1", "d2", 1, "d1", 1, "supports", "the reason", false)
        )
        .0,
        0
    );
    assert_eq!(
        link(
            &h1,
            "krepo",
            &link_input("r2", "d2", 1, "d1", 1, "relates_to", "maybe", false)
        )
        .0,
        0
    );
    assert_eq!(
        run_stdin(
            &h1,
            &[
                "knowledge",
                "unlink",
                "--repo",
                "krepo",
                "--relation",
                "r2",
                "--input",
                "-",
            ],
            br#"{"author":"a","reason":"mistake"}"#,
        )
        .0,
        0
    );
    let full = h1.join("rel.jsonl");
    assert_eq!(
        export(&h1, "krepo", &["--out", full.to_str().unwrap()]).0,
        0
    );
    let base = read_text(&full);

    let h2 = home("port-rel-2");
    let dir2 = mkrepo(&h2, "krepo");
    init(&h2, &dir2, "krepo");
    assert_eq!(import(&h2, "krepo", &full).0, 0);

    // Same relation id, different content: conflict.
    let diff = h2.join("diff.jsonl");
    write_text(
        &diff,
        &base.replace("\"rationale\":\"maybe\"", "\"rationale\":\"other\""),
    );
    let (code, _, err) = import(&h2, "krepo", &diff);
    assert_eq!(code, 3, "{err}");
    assert!(err.contains("import conflict"), "{err}");

    // New id, identical live tuple: conflict (one live assertion per tuple).
    let dup = h2.join("dup.jsonl");
    write_text(
        &dup,
        &base.replace("\"relation_id\":\"r1\"", "\"relation_id\":\"r9\""),
    );
    let (code, _, err) = import(&h2, "krepo", &dup);
    assert_eq!(code, 3, "{err}");
    assert!(err.contains("import conflict"), "{err}");

    // A retracted store tuple does not block a new live relation id.
    let relive = h2.join("relive.jsonl");
    write_text(
        &relive,
        &base.replace("\"relation_id\":\"r2\"", "\"relation_id\":\"r8\""),
    );
    let (code, _, err) = import(&h2, "krepo", &relive);
    assert_eq!(code, 0, "retracted tuple must not block: {err}");
    let y = h2.join("y.jsonl");
    assert_eq!(export(&h2, "krepo", &["--out", y.to_str().unwrap()]).0, 0);
    assert!(
        read_text(&y).contains("\"relation_id\":\"r8\""),
        "r8 must be stored"
    );
}

#[test]
fn port_import_rejects_cross_store_supersedes_cycles() {
    let h = home("port-xcycle");
    let dir = mkrepo(&h, "krepo");
    init(&h, &dir, "krepo");
    assert_eq!(create(&h, "krepo", "d1", "First", "body one").0, 0);
    assert_eq!(create(&h, "krepo", "d2", "Second", "body two").0, 0);
    // d1 is superseded by d2 revision 1: the store holds the historical
    // record-level edge d2 -> d1 (d2 is the replacement).
    assert_eq!(
        update(
            &h,
            "krepo",
            "d1",
            1,
            &supersede_input("First v2", "d2", 1, "replaced by d2")
        )
        .0,
        0
    );
    let base = h.join("xbase.jsonl");
    let (code, base_out, err) = export(&h, "krepo", &["--out", base.to_str().unwrap()]);
    assert_eq!(code, 0, "export: {err}");
    let base_text = read_text(&base);
    let before = std::fs::read(db_path(&h, "krepo")).unwrap();

    // The payload drops d1 and its relation and appends to d2 a revision 2
    // that is superseded by the stored d1 revision 2. d2's stored revision 1
    // stays byte-identical. The payload is acyclic on its own; combined with
    // the stored d2 -> d1 edge it closes a record-level supersedes cycle, a
    // state the store's own write path refuses.
    let mut payload = meta_line(1, 1);
    payload.push('\n');
    for line in base_text.lines() {
        if line.starts_with("{\"type\":\"meta\"") || line.contains("\"supersedes:d2:1:d1:2\"") {
            continue;
        }
        if line.starts_with("{\"type\":\"record\",\"id\":\"d2\"") {
            let extended = line
                .replacen("\"head_revision\":1,", "\"head_revision\":2,", 1)
                .replacen(
                    "\"status\":\"active\",\"revisions\"",
                    "\"status\":\"superseded\",\"revisions\"",
                    1,
                );
            payload.push_str(extended.strip_suffix("]}").unwrap());
            payload.push_str(
                ",{\"revision\":2,\"title\":\"Second v2\",\"body\":\"body two v2\",\
                 \"status\":\"superseded\",\"author\":\"a\",\"created_unix\":30,\"evidence\":[],\
                 \"events\":[{\"event\":\"supersede\",\"target_id\":\"d1\",\"target_revision\":2,\
                 \"reason\":\"replaced by d1\",\"author\":\"a\",\"created_unix\":30}]}]}",
            );
            payload.push('\n');
            continue;
        }
        if line.starts_with("{\"type\":\"record\",\"id\":\"d1\"") {
            continue;
        }
        payload.push_str(line);
        payload.push('\n');
    }
    payload.push_str(
        "{\"type\":\"relation\",\"relation_id\":\"supersedes:d1:2:d2:2\",\"src_id\":\"d1\",\
         \"src_revision\":2,\"dst_id\":\"d2\",\"dst_revision\":2,\"kind\":\"supersedes\",\
         \"author\":\"a\",\"rationale\":\"replaced by d1\",\"created_unix\":30,\"events\":[]}\n",
    );
    let f = h.join("xcycle.jsonl");
    write_text(&f, &payload);
    let (code, _, err) = import(&h, "krepo", &f);
    assert_eq!(
        code, 2,
        "a payload that closes a stored supersedes cycle must be refused: {err}"
    );
    assert!(err.contains("invalid import payload"), "{err}");
    assert!(err.contains("supersedes lineage"), "{err}");
    assert_eq!(
        std::fs::read(db_path(&h, "krepo")).unwrap(),
        before,
        "a refused import must not change revisions, relations, events or generation"
    );
    let after = h.join("xafter.jsonl");
    let (_, after_out, err) = export(&h, "krepo", &["--out", after.to_str().unwrap()]);
    assert!(err.is_empty(), "post-rejection export: {err}");
    assert_eq!(
        read_text(&after),
        base_text,
        "a refused import must leave the export byte-identical"
    );
    assert_eq!(
        field_num(&after_out, "generation"),
        field_num(&base_out, "generation"),
        "a refused import must not advance the generation"
    );
    // Retracting the stored lineage relation does not lift the rule: the
    // lineage is history, so the same payload is still refused.
    let (code, _, err) = run_stdin(
        &h,
        &[
            "knowledge",
            "unlink",
            "--repo",
            "krepo",
            "--relation",
            "supersedes:d2:1:d1:2",
            "--input",
            "-",
        ],
        br#"{"author":"a","reason":"no longer relied on"}"#,
    );
    assert_eq!(code, 0, "retract stored lineage: {err}");
    let retracted = std::fs::read(db_path(&h, "krepo")).unwrap();
    let (code, _, err) = import(&h, "krepo", &f);
    assert_eq!(
        code, 2,
        "a retracted stored lineage edge still closes the cycle: {err}"
    );
    assert!(err.contains("supersedes lineage"), "{err}");
    assert_eq!(
        std::fs::read(db_path(&h, "krepo")).unwrap(),
        retracted,
        "a refused import must not change the store"
    );
}
