# Rust semantic feasibility: rust-analyzer SCIP against the E2 call-site oracle

**Run date:** 2026-10-31 · **Pin:** `master` at `86d6e42` (equal to `origin/master`) · **Stage:** feasibility stage 1, read-only; no source or contract change · **Evidence:** [`pilots/riltra-rust-semantic-feasibility-2026-10-31/`](pilots/riltra-rust-semantic-feasibility-2026-10-31/) · **Commit status:** uncommitted at the time of writing

## Decisions recorded on 2026-10-31

These were taken on the owner's instruction in this working session. The owner may change them by a later dated amendment.

- **D-1, E0-S reliability baseline:** recorded as a dated amendment in [`riltra-e0s-semantic-reliability-2026-10-29.md`](riltra-e0s-semantic-reliability-2026-10-29.md). 27 of 28 is accepted as the recorded baseline, with no numeric target and no code change.
- **D-2, E2 follow-up priority: yes.** The next Riltra work prioritises semantic-backed Rust caller and owner resolution for the E2 path, as recommended in §7 of the E2 manual REPORT (kept outside this repository). Present syntax candidates stay as discovery evidence until that coverage exists. Criterion 2 is met as restated under D-4 (8 of 8 non-closure production sites).
- **D-3, E2 acceptance wording: option 1 (the owner's choice).** `Unverified` is the ceiling for SCIP imports, so the criteria are restated without changing the trust contract. The restated criteria and their status are in [Acceptance status](#acceptance-status). Criterion 3 is restated by the same logic; the owner should confirm that reading.
- **D-4, call sites inside a closure: option A (the owner's choice, 2026-10-31).** Such a call stays `POTENTIAL_CALL` with `owner-unproven`, as the M6 corpus requires, and criterion 2 excludes it. No code change.

## Question

Can a Rust SCIP index for the pinned source be (a) produced reproducibly on this host, (b) imported by Riltra, and (c) used to return the frozen E2 oracle for `confined_read` (nine production and seven test call expressions) with a callee identity, without changing the trust contract?

## Method

- **Primary run (provenance):** a clean clone of the pinned commit in the scratch directory, created with `core.longpaths` enabled and `core.autocrlf` off. The clone has 2,920 tracked and 2,920 present files and no working-tree changes, so Riltra's snapshot reads `head=86d6e42…|dirty=false`.
- **Comparison run:** `git archive 86d6e42` extracted to a scratch directory (2,920 files, no `.git`). Its snapshot reads `head=-`, so it carries no commit provenance.
- **Discarded setup attempt:** the first clone did not enable `core.longpaths`. Git did not create 102 files under `docs/pilots/` whose names exceed the Windows path limit. Riltra marked the snapshot dirty and every edge `Stale`. That attempt is not evidence; the clean run replaced it.
- **Producer:** rust-analyzer 1.99.1 (`58cf8f9c 2026-10-13`) with its built-in `scip` subcommand, run offline with `--exclude-vendored-libraries`, a scratch `CARGO_TARGET_DIR`, and `--log-file`:

  ```text
  CARGO_NET_OFFLINE=true CARGO_TARGET_DIR=<SCRATCH>/target \
  rust-analyzer --log-file <SCRATCH>/out/ra-scip.log scip <SRC> \
    --output <SCRATCH>/out/index.scip --exclude-vendored-libraries
  ```

- **Importer:** the riltra 0.1.0 debug binary from the E2 run (sha256 `354a9760…`), built from the same pin. Each run uses a fresh scratch `RILTRA_HOME`:

  ```text
  riltra init --repo <SRC> --name <NAME>
  riltra index --repo <NAME>
  riltra semantic import --repo <NAME> --scip <SCRATCH>/out/index.scip
  riltra semantic status --repo <NAME>
  riltra callers --repo <NAME> --path crates/cli/src/semantic.rs --line 276 --col 8 --limit 200
  riltra impact --repo <NAME> --path crates/cli/src/semantic.rs --line 276 --col 8 --limit 1000
  ```

- **Oracle:** the E2 `EXPECTED.md`, frozen on 2026-10-29 and not changed. It lists nine production call sites and seven test call expressions.
- **Not committed:** the index files. Their metadata embeds the workspace root, so they carry scratch paths. Raw producer logs are not committed for the same reason; the evidence keeps the derived tables and the Riltra outputs.

## Results

### 1. Producer

- Exit code 0 in all three recorded runs: archive cold 82 s, archive warm 31 s, clean clone 37 s (including the clone).
- Diagnostics are identical in all three runs: 177 `Encountered enclosing definition with no name` errors, 2 `… should have been in an SCIP document but was not` errors (`semantic.rs:53` and `semantic.rs:151`), and 93 duplicate-symbol sites. Exit 0 does not reflect them. rust-analyzer attributes the duplicates to known bug classes, such as nested definitions and example binaries sharing symbols. `confined_read()` does not appear among the duplicates.
- **Reproducibility depends on the workspace path.** The two archive-path runs produced byte-identical indexes (`030b2a…219`). The clean-clone index is 7 bytes longer (`5c271d…a2`). All bytes outside a 181-byte region of the archive index (188 bytes in the clean-clone index) are identical, and that region holds the metadata length prefix and the workspace root URI. The 7-byte difference equals the extra `pinned/` path segment. The artifact is reproducible for a given workspace path, and it embeds that path.
- **The producer executes code.** The log shows `Loading build script … run` for workspace and dependency crates, including `rmcp`, `protobuf` and `serde_json`. Riltra's invariant is that it never executes the code it inspects, so a Rust producer must be an explicit opt-in step, like the LSP adapter. The `--config-path` option can change cargo behaviour; its effect was not tested.
- The earlier WIY follow-up reported 171 enclosing-definition errors and 93 duplicates on a different source revision (`2852ddb` plus a patch). The 177 here is a measurement on `86d6e42`, not a regression claim.

### 2. Importer

- The clean-clone and archive runs produce the same summary: 35 documents, 17,846 definitions, 103,833 references, `skipped: []`, `fallback_reason: none`, `verification: Unverified`. The status reports `content_identity_match: true` and `source_revision_match: true`.
- Timing on this host: the second archive-path import took 236 s. On the clean clone, the import step ended 4 min 53 s after the producer finished, including `init` and `index`.
- Producer diagnostics are not carried into the artifact, and the importer has no field for them. The status states that the producer name and version are artifact metadata that Riltra does not check, and that `Current` is unreachable without independent producer attestation.

### 3. Callers at the E2 anchor (`crates/cli/src/semantic.rs:276:8`)

- **Anchor:** symbol `rust-analyzer cargo riltra 0.1.0 semantic/confined_read().`, precision `Heuristic`, `anchor_uncertainty: null`, freshness `fresh`, verification `Unverified`. On the clean clone the snapshot revision is `head=86d6e42…|branch=-|dirty=false|untracked=0`.
- **Edges:** 16 returned, all with the same callee symbol, none outside the oracle, not truncated. The 16 edge rows are identical between the clean-clone run and the archive run (kind, precision, verification, caller path and span, callee, and unresolved reason).

| Oracle site | Class | Edge | Precision | Verification | Enclosing caller |
|---|---|---|---|---|---|
| `edges.rs:111` | production | CALLS | Exact | Unverified | `edges/source_proofs()` |
| `index.rs:815` | production | CALLS | Exact | Unverified | `index/import_source_changed()` |
| `index.rs:1357` | production | POTENTIAL_CALL | Heuristic | Unverified | none (`owner-unproven`) |
| `knowledge_binding.rs:579` | production | CALLS | Exact | Unverified | `knowledge_binding/resolve_code_anchor()` |
| `knowledge_binding.rs:695` | production | CALLS | Exact | Unverified | `knowledge_binding/check_anchor_source()` |
| `knowledge_binding.rs:1111` | production | CALLS | Exact | Unverified | `knowledge_binding/at()` |
| `knowledge_store.rs:880` | production | CALLS | Exact | Unverified | `knowledge_store/capture_evidence()` |
| `knowledge_store.rs:974` | production | CALLS | Exact | Unverified | `knowledge_store/evidence_freshness()` |
| `scip.rs:643` | production | CALLS | Exact | Unverified | `scip/process_document()` |
| `semantic.rs:672` | test | CALLS | Exact | Unverified | `semantic/tests/confined_read_reports_non_not_found_stat_errors()` |
| `semantic.rs:677` | test | CALLS | Exact | Unverified | same as line 672 |
| `semantic.rs:691` | test | REFERENCES | Exact | Unverified | none (inside an assertion macro) |
| `semantic.rs:692` | test | CALLS | Exact | Unverified | `semantic/tests/confined_read_guards_size_and_containment()` |
| `semantic.rs:697` | test | REFERENCES | Exact | Unverified | none (inside an assertion macro) |
| `semantic.rs:698` | test | REFERENCES | Exact | Unverified | none (inside an assertion macro) |
| `semantic.rs:708` | test | CALLS | Exact | Unverified | `semantic/tests/confined_read_guards_size_and_containment()` |

- **Production recall:** 9 of 9 sites returned. Eight are `CALLS` with a named enclosing caller. The ninth, `index.rs:1357`, is a `POTENTIAL_CALL` with an unresolved owner. That call sits inside a closure (`.or_else(|| …)`) in `resolve_edge_anchor_with_source`. The cause is the closure rule, not the producer's diagnostics (see [Diagnosis](#diagnosis-of-the-closure-call-site-2026-10-31)).
- **Test recall:** 7 of 7 sites returned. Four are `CALLS`; the three inside assertion macros are `REFERENCES` with no owner.

### 4. Impact

- Verdict `UNKNOWN` on both runs. Completeness `partial` (21 of 21), `truncated: true`, with `unresolved: ["time-budget", "unresolved-owner"]`. Both runs stopped at 21 paths on the time-budget cutoff.
- Seven snapshot-wide coverage denominators are non-zero: unsupported files 2,716; dynamic calls 1,342; ambiguous calls 108; unresolved owners 86,122; unresolved targets 21,815; missing provider files 169; parse-error files 1. The traversal also truncated one branch.
- No positive affected set is claimed.

### 5. Reproducibility summary

- Two archive-path runs: byte-identical index, identical diagnostics, identical import summary.
- Clean-clone run: identical diagnostics, identical import summary, identical 16 edge rows. Its index differs from the archive index only in the workspace root recorded in its metadata.

## Acceptance status

The owner chose option 1 on 2026-10-31, and option A for the closure case on the same day. The original criteria from E2 REPORT §7 are kept for traceability; the restated criteria apply from this decision.

| Criterion | Original (E2 REPORT §7) | Restated under option 1 | Status |
|---|---|---|---|
| 1 | The anchor resolves to a semantic identity with a non-null symbol URI, Exact precision, and no syntax-only anchor uncertainty. | The anchor resolves to a semantic identity with a non-null symbol URI and no anchor uncertainty. The anchor's precision is reported as produced; `Heuristic` is the current value and is not a criterion. | **Met.** URI present; `anchor_uncertainty: null`. |
| 2 | Callers returns all nine production sites as resolved edges to that identity, with Verified status; test calls are distinguished; name and fixture matches are excluded. | Every production call site from the frozen oracle that is not inside an anonymous scope appears as a resolved `CALLS` edge (edge precision `Exact`) to that callee symbol, with verification `Unverified` and recorded producer provenance. Test calls are distinguishable, and name-only and fixture-string matches are excluded. A call inside an anonymous scope (a closure) is reported as `POTENTIAL_CALL` with `owner-unproven`, by the M6 rule. | **Met as restated (option A): 8 of 8 non-closure production sites.** `crates/cli/src/index.rs:1357` is a closure call, excluded by the M6 rule (see [Diagnosis](#diagnosis-of-the-closure-call-site-2026-10-31)). Met: provenance (tool, version, artifact id, content identity, and commit on the clean clone); no edges outside the oracle; tests distinguished (4 `CALLS`, 3 `REFERENCES`). |
| 3 | Impact reports a positive result only when backed by verified paths and sufficient completeness; otherwise `UNKNOWN`. | Impact reports a positive result only when backed by sufficient completeness; otherwise `UNKNOWN`. `Unverified` is the ceiling under this option, so "verified paths" cannot occur. | **Met, conservatively.** Verdict `UNKNOWN` on both runs. No positive result is claimed. |

Criterion 3 is restated by the same logic. That reading of option 1 should be confirmed by the owner.

## What option 1 does not unlock

Option 1 leaves the contract unchanged, so it does not make a positive impact result reachable for this repository.

- `impact_verdict` (`crates/cli/src/edges.rs:1539–1568`) checks edge precision, edge endpoints, unresolved relations, and truncation. It does not read verification, so `Unverified` does not block a positive verdict by itself.
- The caller of `impact_verdict` (`crates/cli/src/index.rs:2531–2544`) passes `uncertain = true` when the import is `Stale`, the source is not fresh, the anchor is uncertain, a snapshot coverage denominator other than `observed_edges` is non-zero, coverage reasons are present, or the result is paged. The denominators are computed over the whole snapshot (`crates/cli/src/index.rs:2081–2115`).
- This repository has non-zero values in seven of those denominators (§4), so a positive verdict is unreachable here. Closing criterion 2 does not change that.
- The traversal also has a 2-second wall-clock budget (`TRACE_TIME_MS = 2000`, `crates/cli/src/edges.rs:921`). Both runs stopped on `time-budget` at 21 paths, so the cutoff depends on host speed.

A positive impact capability therefore needs a separate owner decision on completeness scope: repository-wide, or limited to the anchor's language and traversed subgraph. That needs a dated amendment and tests, and it is a contract change.

## Diagnosis of the closure call site (2026-10-31)

**Question:** why is the call at `crates/cli/src/index.rs:1357` a `POTENTIAL_CALL` with `owner-unproven`, and is the cause in Riltra or in the producer's data?

**Finding: the cause is Riltra's syntax owner rule, not the producer's data.**

- The call sits inside a closure: `cached_source.map(...).or_else(|| { crate::semantic::confined_read(...) ... })` in `resolve_edge_anchor_with_source`.
- `enclosing_owner` (`crates/cli/src/syntax.rs:273-297`) walks up from each call and returns no owner when it crosses a `closure_expression` or another anonymous scope. The result is stored per call node (`syntax.rs:319`).
- `call_owner` (`crates/cli/src/edges.rs:120-132`) requires that stored bound to match the narrowest named `function` or `method`. With no bound, no owner is found, and the edge falls through to `owner-unproven` (`crates/cli/src/edges.rs:624-628`).
- Riltra reads SCIP `enclosing_range` nowhere outside the generated `scip_proto.rs`. The producer's 177 "enclosing definition with no name" diagnostics are therefore not an input to owner resolution.
- The callee side is not the blocker. A `POTENTIAL_CALL` row carries no callee by design, and the reason check runs before the callee check, so the callee status is not reported for this site. The eight resolved sites use the same callee symbol and definition.

**The rule is pinned by the M6 corpus.** `crates/cli/tests/fixtures/m6/rs/nested_anonymous.rs` contains `let callback = || target();` inside `caller()`. Its manifest, authored independently of the implementation, expects `callees_calls: 0`, `inbound_calls: 0`, and `impact: UNKNOWN`, with the note "Anonymous scope inside a named function cannot borrow the enclosing function as a definite caller." Changing the rule therefore changes a recorded truth, not only code.

**Controlled reproduction.** A two-caller fixture ([`diagnosis/closure-fixture/`](pilots/riltra-rust-semantic-feasibility-2026-10-31/diagnosis/closure-fixture/)) was indexed with the same rust-analyzer flags and imported with the same riltra binary:

- `direct_caller` calls `callee(x)` directly: `CALLS` (`Exact`, `Unverified`), caller `direct_caller()`.
- `closure_caller` calls `None.or_else(|| callee(x))`: `POTENTIAL_CALL`, `owner-unproven`, no caller.
- The producer's log for this fixture has no "enclosing definition with no name" errors, so a simple closure does not produce them. Their source in this repository is not identified.

**Decision (owner, 2026-10-31): option A.** The anonymous-scope rule stays, and criterion 2 excludes call sites inside an anonymous scope, as restated in [Acceptance status](#acceptance-status). No code change, and the M6 truth stands. Options that were not chosen:

- **B. Change the rule** so that a closure call is attributed to the lexically enclosing named function. This changes callers, callees, trace and impact for Rust closures. It requires amending the M6 truth and its note, plus tests, under a dated amendment. It is a contract change.
- **C. Keep the rule but split the reason**, for example `anonymous-scope` instead of `owner-unproven`. This changes output strings, so goldens and CLI/MCP parity tests change too. It could still be done later.

Closures are common in Rust, so the exclusion may leave many real callers without a definite owner. This repository's closure count was not measured.

## Contract observations

1. **Precision and verification are separate.** An edge from a SCIP import reads `Exact` when its callee endpoint is proven inside the import (`crates/cli/src/edges.rs:611` for `CALLS`, `:701-705` for plain references). The import itself stays `Unverified`; `edges.rs` tests assert that `Exact` never implies `Current` (around `:3185-3196`). The anchor fact stays `Heuristic`.
2. **`Current` is unreachable.** `verify_live` never returns `Current` (`crates/cli/src/semantic.rs:444-454`), and the producer's name and version are not checked.
3. **Producer diagnostics are lost at import.** A consumer of the stored data cannot see the 177 or 93 diagnostics.
4. **The freshness gate reacts to a dirty checkout.** The discarded clone was dirty, and every edge read `Stale`. This is correct behaviour, but it means evidence runs need a clean checkout.

## Constraints found

- The producer runs build scripts and procedural macros. This conflicts with the never-execute invariant, so any Rust producer must be an explicit opt-in step.
- The index depends on host configuration. `confined_read_reports_non_not_found_stat_errors` is `#[cfg(windows)]`, and its two calls (`semantic.rs:672` and `:677`) appear only because the producer ran on Windows. This was not tested on Linux or macOS.
- A call inside a closure loses its owner (1 of 9 production sites).
- A call inside an assertion macro becomes a `REFERENCES` edge with no owner (3 of 7 test sites).
- Windows checkouts need `core.longpaths` for this repository: 102 files under `docs/pilots/` have names beyond the path limit.
- Cost on this host: producer 31 to 82 s; import about 236 s for 35 documents.

## Corrections to earlier notes

- The WIY implementation document lists the "Rust SCIP indexer" as absent. rust-analyzer 1.99.1 includes the `scip` subcommand, so no separate indexer is needed. The standalone `scip` CLI, used to inspect indexes, is still absent and was not needed.
- The 171 and 93 figures are in the WIY follow-up document, not the implementation document. On pin `86d6e42`, the producer reports 177 and 93.
- "Reproducible" in the first version of this document meant reproducible for one workspace path. The clean-clone run shows that the artifact embeds the workspace root.

## Not tested and limits

- One pin (`86d6e42`) on one host (Windows 11, rustc 1.99.1). No Linux or macOS run, and no CI run of the producer.
- `--config-path` settings, such as disabling build scripts, were not tested. Their effect on the 16 edges is unknown.
- Occurrence roles and enclosing-definition logic were audited only for the 16 oracle rows.
- Repository-wide recall was not measured. The oracle covers one symbol.
- The riltra binary was taken from the E2 run and not rebuilt.

## Next steps

1. Decide whether to commit the decision, evidence, and amendments on a branch from master (owner go-ahead).
2. Owner decision on impact completeness scope (a contract change; see [What option 1 does not unlock](#what-option-1-does-not-unlock)).
3. Run the clean clone with build scripts disabled through `--config-path`, and compare the 16 rows (about five minutes).
4. Rust macro token trees (remaining WIY item 1) would turn the three assertion-macro `REFERENCES` into call facts. That needs a `PARSER_VERSION` bump and is a separate decision.
5. Optional, test-only: a SCIP-path regression test that pins the closure case (`POTENTIAL_CALL`, `owner-unproven`). It needs the owner's go-ahead.

## Evidence

The folder [`pilots/riltra-rust-semantic-feasibility-2026-10-31/results/`](pilots/riltra-rust-semantic-feasibility-2026-10-31/results/) holds:

- `producer-runs.tsv`: the three producer runs, with exit codes, timings, index sizes and hashes, and diagnostic counts.
- `producer-duplicate-symbols.tsv`: the 93 duplicate-symbol sites, with relative paths only.
- `pinned-import.json`, `pinned-semantic-status.json`, `pinned-callers-anchor.json`, `pinned-impact.json`, `pinned-run-status.txt`: the clean-clone run (primary provenance).
- `import.json`, `semantic-status.json`, `callers-anchor.json`, `impact.json`: the archive run.
- `oracle-accounting.csv`: the 16 oracle rows.
- `diagnosis/closure-fixture/`: the two-caller fixture (`Cargo.toml`, `src/lib.rs`) and Riltra's callers output, for the controlled check.
- `MANIFEST.sha256`: hashes of all result and diagnosis files.
