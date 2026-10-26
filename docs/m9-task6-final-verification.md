# M9 Task 6: final G-SYN and G-GRF verification (2026-10-26)

Dated revision. The gate records in [m9-design-gate.md](m9-design-gate.md) are unchanged. This document records the verification run on `m9-multirepo-release-candidate`. No gate is marked PASS here except where stated. Evidence: [pilots/m9-task6/](pilots/m9-task6/).

## 1. Result

| Item | Verdict | Basis |
|---|---|---|
| G-SYN | **FAIL** (narrow) | 17 of 6,680 import facts store `end_line` equal to the start line of a multi-line import (§3.4). All other checks pass. |
| G-GRF, correctness and safety | **PASS** | No promoted candidate, no fabricated endpoint, no `Exact` precision, all 71 candidate citations and 13 anchors grounded (§5.2). Two transparency items stay open (§5.4). |
| D1-C6 method identity | UNMET | `identity-not-proven-at-667` |
| D2-C6 constructor attribution | UNMET | `construction-sites-not-attributed` |
| D3-C5 multi-hop trace | UNMET | `trace-does-not-extend-beyond-the-first-hop` |
| D4-C5 impact analysis | UNMET | `affected-set-empty` |

No code, schema, API or grader change. No SCIP or clangd import. The import defect is recorded here and not fixed (scope). A correctness PASS does not make a capability PASS.

## 2. Identities, tools and commands

- Riltra HEAD `72f576a`, tree `680b2ca3`. `crates/` tree `d172c0ac`. `git diff --stat e911052 HEAD -- crates` is empty.
- Binary `<RILTRA_ROOT>/target/debug/riltra.exe`: `riltra 0.1.0`, SHA-256 `ce6eb97a78c8aa6a92c4acdb4aba8f3799ae31a6dfe713d78b954ccbb5e0a5a0`, built with `cargo build --locked` (debug).
- MMSP snapshot `<CP2_BENCH>/mmsp-d62f06a`: isolated clone of the original, HEAD `d62f06a`, tree `c5482ea`, 2,277 tracked files. `git status --porcelain` is empty before and after. The SHA-256 of every tracked file is identical before and after. The 8 key `.git` files are identical too.
- Original MMSP `<MMSP_ROOT>`: HEAD `fa5f74d`, tree `e25844a`, porcelain empty, git directories intact. Read only.
- Tools: node v24.19.0, cargo and rustc 1.99.1, pwsh 7.6.6, git 2.53.0, Windows 11 Pro 10.0.26300.
- Context7, `/websites/nodejs_latest-v24_x_api`: `new DatabaseSync(path, { readOnly: true })` opens read-only and fails when the file is missing. The check scripts use it that way.
- Harness steps used: `cold-index.ps1` (two cold runs, fresh `RILTRA_HOME` each), `queries.ps1` (on `home-main`, a copy of `home-cold1` made by the harness copy step), `grade.cjs`, `fact-digest.cjs`, `db-mutation-check.cjs`, `redact.cjs`, the harness self-tests. `run-all.ps1` was not run. Freshness, incremental, performance, MCP smoke and the OpenCode session are outside this task.

## 3. G-SYN

### 3.1 Two cold scans

- Both runs: `init` and `index --full` exit 0, `fallback_reason` `forced-full`, `reparsed` 1,475.
- Index counters identical in both runs: discovered 2,277, processed 1,475, definitions 18,785, imports 6,680, calls 99,720, errors 2,387, skipped binary 4, unsupported 798. Only `timings_ms` differs. Wall time 102.7 s and 108.2 s on this host; not comparable with earlier hosts.
- Fact digest, `db-mutation-check.cjs` then `fact-digest.cjs compare-digests`: IDENTICAL. Combined digest `acd93511…796c13`. Rows: files 1,475, symbols 18,785, imports 6,680, calls 99,720, edges 106,905.
- Raw DB files have equal size (60,370,944 B) and different SHA-256. The `snapshots` row stores `created_unix` and `nonce`, which differ between runs. The size is 24,576 B above the figure in [m9-inl-amendment.md](m9-inl-amendment.md). Not investigated; no fact effect.

### 3.2 Scan partition

`riltra scan` on the cold-run-1 registry:

| Status | Reason | Files |
|---|---|---|
| ok | | 1,475 |
| unsupported | unsupported-extension | 309 |
| unsupported | cuda-experimental (`.cu`, `.cuh`) | 489 |
| skipped | binary-or-non-utf8 | 4 |
| total | | 2,277 |

- `.cu` and `.cuh`: 489 files, all `cuda-experimental`, 0 rows in the index. No CUDA fact or coverage claim.
- `.inl`: 37 files, all processed as `cpp`. 9 with zero syntax errors, 28 with recovery errors, 396 errors in total. Matches the Task 3 probe ([m9-inl-amendment.md](m9-inl-amendment.md)).
- Processed by language: cpp 1,307, c 34, python 114, typescript 18, javascript 2.
- Syntax errors: 2,387 in 179 files. Same totals as the Task 2 review ([m9-gsem-amendment.md](m9-gsem-amendment.md)). These are parser errors (tree-sitter ERROR and MISSING nodes). Fact errors are separate (§3.4).

### 3.3 Source grounding, all facts

Method, `pilots/m9-task6/method/span-check.cjs`: each fact's byte span is mapped to line and column in the snapshot source. Lines must equal the reported lines. The byte slice must contain the reported name, whitespace ignored. File digests use Riltra's content digest, FNV-1a 64 (`crates/cli/src/store.rs`, `content_digest`).

| Table | Rows | Line mismatch | Name checked | Name mismatch | Nameless |
|---|---|---|---|---|---|
| files | 1,475 | digest match 1,475, size match 1,475 | | | |
| symbols | 18,785 | 0 | 17,878 | 0 | 907 |
| calls | 99,720 | 0 | 99,720 | 0 | 0 |
| imports | 6,680 | 17 | 6,680 | 0 | 0 |

The column base is 1 and byte-based for every row of every table. Cold run 2 gives a byte-identical span report.

### 3.4 Fact defect

- The 17 multi-line import facts (15 Python, 2 TypeScript) store `end_line` equal to the start line. Their byte span and `end_col` reach the closing line. Example: an import at `colmap/benchmark/reconstruction/download.py:42` has stored `end_line` 42; its byte span ends on line 47.
- The 6,663 single-line imports are correct.
- Root cause: `push_import` in `crates/cli/src/syntax.rs` (lines 367–389) sets `end_line: (pos.row + 1)` with `pos = node.start_position()`. `end_col` comes from the trimmed end byte. The two end fields come from different positions.
- Not documented in `docs/`. Not fixed here. A fix needs its own owner-approved change and a regression test on multi-line Python and TypeScript imports.

### 3.5 `.inl` sample with error recovery (Task 2 and Task 3 reproduction)

| File | Errors | Definitions (nameless) | Calls (cast-like) | Task 2 record |
|---|---|---|---|---|
| `Array.inl` | 0 | 84 (39) | 100 (1) | identical |
| `FEMTree.inl` | 14 | 84 (3) | 625 (4) | identical |
| `FEMTree.System.inl` | 133 | 118 (8) | 1,609 (33) | identical |

Cast-like means the call name contains `cast` or starts with `(`. This definition reproduces the three Task 2 counts; it was inferred from the names, not stated in the Task 2 record. Span and name checks for these files: 0 mismatches (§3.3). Facts from the recovered regions are source-grounded.

### 3.6 Determinism

- Fact digest identical across cold runs (§3.1).
- Frozen query answers, `home-main` against `home-cold2`: 20 of 20 successful answers identical after normalizing volatile snapshot metadata only (`fact-digest.cjs compare-outputs`). The 2 error answers (`d0-3`, `d3-2`) are byte-identical. Evidence: `raw/env/query-determinism.json`.

### 3.7 Judgement

- G-SYN is not PASS. §3.4 breaks "source-grounded spans" for 17 facts.
- The error counts (2,387) are parser errors and match the Task 2 and Task 3 records. They are not the failure.
- The G-SYN STOP conditions (fabricated facts, nondeterminism, CUDA coverage claim) do not occur.

## 4. Regression and checks

- `cargo fmt --all -- --check`: exit 0.
- `cargo clippy --all-targets -- -D warnings`: exit 0.
- `cargo test --locked`: 777 passed, 0 failed. Per target: unit 425, cli 16, edges 149, index 43, knowledge 52, mcp 58, semantic 22, syntax 12.
- `node --test grade.test.cjs`: 52 passed, 0 failed. `node --test fact-digest.test.cjs`: 16 passed, 0 failed.
- Harness self-tests `test-path-safety.ps1` and `test-harness.ps1`: exit 0.

## 5. G-GRF

### 5.1 Harness grading

`grade.cjs` (latest: G1–G5, Q1–Q5, D1-H6) over this run's 22 outputs. Exit 0, no FAIL.

- Honesty: 17 PASS, 1 UNAVAILABLE (D5-H1, `mcp-smoke-missing`; MCP smoke is out of scope).
- Capability: 4 UNMET (D1-C6, D2-C6, D3-C5, D4-C5).
- Evidence: 2 UNAVAILABLE (D3-E1 `cuda-unsupported`; D5-E1 `session-not-run`).
- D2-H2 is PASS on this run (factory-template-argument candidate, `c869fdf`). The archived `pilots/m9-rc/raw-after` grading still says FAIL. That archive predates `c869fdf`. It is not re-graded here.

### 5.2 Manual checks

Method, `pilots/m9-task6/method/query-check.cjs`:

- Edges: 71, all `POTENTIAL_CALL`, `Heuristic`, `Unverified`. 0 `CALLS`. 0 `Exact`. No `Exact` or `Current` literal in any output (F5).
- Unresolved reasons on edges: `dynamic` 36, `no-overlap` 25, `factory-template-argument` 10.
- Citations: 71 of 71 terminal spans equal the callee name as written and lie inside the caller span.
- Endpoints: 13 of 13 anchor definitions exist in the index. Candidate edges carry `callee: null`. No endpoint is invented.
- Search definitions: 88 of 88 names occur inside the reported span lines. Three spans start on a return-type line with the name on the next line; the span is correct.
- Indirect dispatch: no `ProcessProblem` edge in any output (§5.4a).
- Trace `d3-1`: 23 paths, one edge each (max hops 1). Unresolved reasons `dynamic`, `no-overlap`, `unresolved-target`, `factory-template-argument`. Disclaimer `plausible-path-not-execution`. Visits 1 node and 23 edges against budgets 500 and 3,000: not cut.
- Impact `d4-1`, `d4-2`: verdict `UNKNOWN`, completeness `partial`, reasons include `graph-completeness-unproven`, `insufficient-impact-evidence`, `empty-result-not-coverage-proof`. Disclaimer `plausible-impact-not-execution`.
- Search `Run`: `total` 190, returned 50, `truncated: true`. The result limit is reported.
- Completeness denominators, `d1-5`: ambiguous_calls 299, dynamic_calls 45,877, unresolved_target 100,225, unresolved_owner 24,994, missing_provider_files 1,475, parse_error_files 179, unsupported_files 802 (798 unsupported plus 4 binary). Against the archived pre-`.inl` run (`raw-after`): missing_provider_files +37, parse_error_files +28, unsupported_files −37. These match the `.inl` change. The other denominators moved with the added `.inl` facts; not attributed further.

### 5.3 CUDA destination

`d3-2` (`--to` at `patch_match_cuda.cu:1326:22`): exit 3, `error: anchor unresolved: source-changed; reindex before syntax fallback`. The same message is the frozen expectation in [m9-task5-capability-gap-evaluation.md](m9-task5-capability-gap-evaluation.md). The grader records the actual cause, `cuda-unsupported`, next to it.

### 5.4 Open transparency items

These are not PASS and are not fixed. The owner decides whether they block G-GRF.

- a. Indirect dispatch: the `ProcessProblem` hop through `ThreadPool::AddTask` (`patch_match.cc:268, 292, 309`) is absent from every output. The frozen text asks that it remain a candidate. The index emits no candidate for this address-of reference. The grader accepts absence (D2-H3). Task 5 names a labelled candidate class as the missing piece.
- b. The trace output `d3-1` does not name the CUDA boundary. The boundary is visible only through the failed `--to` probe, whose message is `source-changed` (§5.3). The frozen expectation matches, but the trace itself does not state the boundary.

### 5.5 Capability verdicts

D1-C6, D2-C6, D3-C5 and D4-C5 stay UNMET. The run has no `CALLS` edge, no `Exact` edge and no SCIP import, so no capability can change.

## 6. Scope and open blockers

- No feature, schema, API or graph change. No clangd or scip-clang. No SCIP import. No dataflow. No G-TOK measurement. No nested agent. No push, merge, tag or release.
- Open blockers: §3.4 (G-SYN FAIL); §5.4a and §5.4b (owner decision on G-GRF transparency); G-MCP evidence, blocked by the expired OpenCode key on this host.

## 7. Evidence

- Archive, redacted: [pilots/m9-task6/raw/](pilots/m9-task6/raw/). Cold runs, scan summary, digests, span reports, 22 query outputs, grading, query-check, determinism, tests, environment identity.
- Check scripts, new and reproducible: [pilots/m9-task6/method/](pilots/m9-task6/method/) (`span-check.cjs`, `query-check.cjs`, `scan-summary.cjs`).
- Redaction: `redact.cjs` with `<CP2_BENCH>`, `<USERPROFILE>`, `<RILTRA_ROOT>`, `<MMSP_ROOT>`. A post-redaction scan found no drive-letter or user-profile path.
- The first snapshot file listing carried a label offset bug. The per-tracked-file SHA-256 comparison in `env/snapshot-fingerprint.json` replaces it.
