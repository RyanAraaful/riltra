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
