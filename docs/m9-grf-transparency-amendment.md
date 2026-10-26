# M9 amendment: CUDA refusal classification and device-side predicate (G-GRF transparency)

Dated revision: 2026-10-26. This amends the two open G-GRF transparency items recorded in
[m9-task6-final-verification.md](m9-task6-final-verification.md) §3. The Task 6 record and its archive
(`pilots/m9-task6/`) are not modified. Corrected outputs and gradings are new files under
[pilots/m9-grf-transparency/](pilots/m9-grf-transparency/).

## Decision

1. **CUDA refusal is named, not reported as a changed source.** In `resolve_edge_anchor`
   (`crates/cli/src/index.rs`), a target that `syntax::classify` marks as CUDA (`.cu`, `.cuh`) returns
   `anchor unresolved: cuda-experimental; CUDA sources are never parsed or indexed`. The reason token comes
   from `scan::unsupported_reason`, the same mapping the scan uses. The check runs after the semantic branch
   and before the source read, so the semantic path is unchanged.
   - Unchanged: path validation and containment, the `source-changed`, `source unavailable` and digest checks
     for supported sources, and every relation. No edge to a CUDA definition is created.
   - Not changed, found by this check: other unsupported extensions. `trace --to README.md:1:1` on the isolated
     snapshot still answers `source-changed`, because the same path has no index row. This is the same
     misclassification in general form. It is outside the requested scope (`.cu`/`.cuh`) and stays open.
     Trigger: a request to classify unsupported extensions in general.
2. **D3-H3 predicate corrected (`docs/pilots/cp2/method/grade.cjs`).** The predicate searched the whole trace
   document for the substring `.cu`. Frozen spec `m9-cp1-pilot-spec.md` §B.3 requires a path that ends at the CUDA
   boundary with the terminus labelled unsupported, and states that such a path is honest (honesty item 3). A
   substring rule FAILs that path, so the contradiction with the frozen spec is confirmed. The predicate now
   counts only path endpoints (`caller`, `callee`) that sit in a `.cu`/`.cuh` file and are not typed
   `unsupported`. Text fields are not claims. Evidence key `mentions_cuda_source` becomes
   `cuda_definition_claims`.
3. **Observed reason kept.** `classifyError` maps the new message to its own category, `cuda-experimental`.
   The grader's actual cause for a CUDA target stays `cuda-unsupported`.
4. **Indirect dispatch: no change.** No graph change and no new `POTENTIAL_CALL` for `ThreadPool::AddTask`. The
   trace still cannot identify the indirect dispatch hop (`ProcessProblem` absent). D3-C5 stays UNMET with
   `trace-does-not-extend-beyond-the-first-hop`. No record in this change calls the trace complete.

Scope kept: no semantic provider, parser, dataflow, schema, frozen-spec, or graph change. `PARSER_VERSION` stays
7 and no golden changes, because `syntax.rs` extraction is untouched. No reindex was needed.

## Regression tests (RED, then GREEN)

Rust, `crates/cli/tests/edges.rs`:

| Test | RED before fix | GREEN after fix |
|---|---|---|
| `cuda_destination_reports_its_unsupported_reason_not_source_changed` (`trace --to kernel.cu:1:17`) | FAIL: `source-changed` | pass: `cuda-experimental`, no `source-changed` |
| `changed_destination_still_reports_source_changed` (control: changed `lib.py` as destination) | pass | pass |

Grader, `docs/pilots/cp2/method/grade.test.cjs`:

| Test | Before predicate fix | After |
|---|---|---|
| D3-H3 passes when the CUDA terminus is labelled unsupported (positive) | FAIL | pass |
| D3-H3 passes when a CUDA file name appears only in text (positive) | FAIL | pass |
| D3-H3 fails when an unproven CUDA definition is a path endpoint (negative) | verdict and reason FAIL, evidence key missing | FAIL, `device-side-definition-claimed`, 1 claim |
| D3-H4 classifies the `cuda-experimental` message and keeps `cuda-unsupported` as the actual cause | pass (guard) | pass |

RED output: [raw/tests/red-observed.txt](pilots/m9-grf-transparency/raw/tests/red-observed.txt).

## Validation

- `cargo test --locked`: 783 passed, 0 failed (781 before, plus 2). Log:
  [raw/tests/cuda-full-test.log](pilots/m9-grf-transparency/raw/tests/cuda-full-test.log).
- `cargo fmt --all -- --check`: exit 0. `cargo fmt --all` reformatted the new test lines.
- `cargo clippy --all-targets -- -D warnings`: exit 0 ([raw/tests/cuda-clippy.log](pilots/m9-grf-transparency/raw/tests/cuda-clippy.log)).
- `node --test grade.test.cjs`: 56 of 56 (52 before, plus 4). `node --test fact-digest.test.cjs`: 16 of 16.
- Targeted GREEN run: [raw/tests/edges-targeted-green.txt](pilots/m9-grf-transparency/raw/tests/edges-targeted-green.txt).

## Output changes on the isolated snapshot

Environment: MMSP snapshot `mmsp-d62f06a` (HEAD `d62f06a`, porcelain empty, 2,277 tracked files). Binary
SHA-256 `6f5ec846…` (debug, `cargo build --locked`). Index: the G-SYN index (parser 7, cfg `c4f60b3e67e71144`),
copied and not reindexed. Original MMSP repository untouched (HEAD `fa5f74d`, porcelain empty).
Details: [raw/env/binary-and-snapshot.json](pilots/m9-grf-transparency/raw/env/binary-and-snapshot.json).

Frozen queries (22), compared with the G-SYN run on the same index
([raw/queries-compare-vs-gsyn.json](pilots/m9-grf-transparency/raw/queries-compare-vs-gsyn.json)):

- 20 of 20 successful answers identical after normalization, including `d3-1-trace-depth4`. No new edge.
- Error answers: `d0-3` identical. `d3-2-trace-to-cuda` changed (exit 3 unchanged):
  - before: `error: anchor unresolved: source-changed; reindex before syntax fallback`
  - after: `error: anchor unresolved: cuda-experimental; CUDA sources are never parsed or indexed`

Grading, reported as two separate layers
([raw/grading/grading.json](pilots/m9-grf-transparency/raw/grading/grading.json)):

- **Honesty: 17 PASS, 1 UNAVAILABLE.** Same counts as before. The UNAVAILABLE is D5-H1 (`mcp-smoke-missing`).
  - D3-H3: PASS, evidence `cuda_definition_claims: 0` (was `mentions_cuda_source: false`).
  - D3-H4: PASS, observed reason `cuda-experimental` (was `source-changed`), actual cause `cuda-unsupported`.
- **Capability: 4 UNMET** (D1-C6, D2-C6, D3-C5, D4-C5), same reasons as before. Still UNMET.
- **Evidence: 2 UNAVAILABLE** (D3-E1 `cuda-unsupported`, D5-E1 `session-not-run`). Same as before.

Regrade of the archives with the corrected grader (no archived file overwritten):

- G-SYN outputs and the Task 6 archive: every check has the same verdict and reason. Only the D3-H3 evidence key
  differs ([cmp-gsyn-regrade-vs-archived.json](pilots/m9-grf-transparency/raw/regrade/cmp-gsyn-regrade-vs-archived.json),
  [cmp-task6-regrade-vs-archived.json](pilots/m9-grf-transparency/raw/regrade/cmp-task6-regrade-vs-archived.json)).

## Status

- G-GRF correctness and safety: unchanged PASS from Task 6. The CUDA refusal is now named correctly.
- Transparency item (b), "the trace does not name the CUDA boundary": **still open.** Only the `--to` refusal is
  named. The path does not end at the CUDA boundary. Closing it changes trace output or graph, so it is out of scope.
- Transparency item (a), indirect dispatch: **still open.** It is not a candidate, and no record calls the trace complete.
- The `type: "unsupported"` endpoint label is the grader's expectation from spec §B.3. The trace does not produce
  that label today.
- Capability verdicts D1-C6, D2-C6, D3-C5, D4-C5: all UNMET, unchanged.
- Context7 was not used: the change adds no third-party API. Node built-ins and the existing Rust code only.

## Evidence

- [pilots/m9-grf-transparency/](pilots/m9-grf-transparency/): redacted queries, grading, regrade, test logs,
  environment identity. Local paths replaced by placeholders (`<BENCH>`, `<RILTRA_ROOT>`, `<DEV_ROOT>`).
- [pilots/m9-grf-transparency/method/cmp-grading.cjs](pilots/m9-grf-transparency/method/cmp-grading.cjs): per-check
  comparison. The query comparison uses `pilots/m9-gsyn-fix/method/cmp-query-dirs.cjs`.
