# M9 amendment: multi-line import span end (G-SYN)

Dated revision: 2026-10-26. This amends the G-SYN finding in
[m9-task6-final-verification.md](m9-task6-final-verification.md) §3.4. The Task 6
record itself is unchanged.

## Decision

- `push_import` in `crates/cli/src/syntax.rs` takes both end coordinates from the same
  trimmed end offset, through the existing `position_of()` helper. Before, `end_line` was
  the start row of the statement and `end_col` came from the end offset.
- `line_start()` is removed. Its only caller was `push_import`.
- `store::PARSER_VERSION` moves from 6 to 7, because the stored representation of import
  spans changed. An index written at version 6 answers `stale` (`parser-changed`,
  `config-changed`) and is rebuilt. The mechanism is unchanged.
- Unchanged: `byte_start`, `byte_end`, the import text, the start coordinates, and every
  other fact. No schema, API, graph, SCIP, LSP, grader or language change.

## Regression tests (RED, then GREEN)

Four tests in `syntax::tests`, added before the fix:

| Test | Input | Expected end (line, col) | Before fix |
|---|---|---|---|
| `python_multiline_import_end_is_on_its_last_line` | parenthesized `from ... import (...)`, 4 lines | (4, 2) | FAIL, got (1, 2) |
| `typescript_multiline_import_end_is_on_its_last_line` | `import {...} from "./pkg";`, 4 lines | (4, 16) | FAIL, got (1, 16) |
| `single_line_python_import_span_is_unchanged` | `import os` | (1, 10) | pass (guard) |
| `include_trailing_newline_is_trimmed_before_end_coordinates` | `#include <stdio.h>` followed by `\n` | byte_end 18, (1, 19) | pass (guard) |

RED: 48 passed, 2 failed. GREEN after the fix: 50 passed. The two RED failures are the
`end_line` assertions only. The two guard tests pass before and after, so they protect the
trimming and the single-line case.

## Golden updates

- The configuration digest includes `PARSER_VERSION`, so the `cfg=` token changed from
  `95fce9bb78e9c9df` to `c4f60b3e67e71144` in 88 golden files (780 occurrences).
- 79 files under `crates/cli/tests/golden/m6-corpus/` were regenerated with
  `RILTRA_UPDATE_M6_GOLDENS=1 cargo test --locked --test edges task7::`. The source-truth
  assertions ran first and passed.
- 9 top-level `crates/cli/tests/golden/m6-*.json` files have no regeneration switch. Their
  token was replaced by hand (17 occurrences). This is the one step that is not a regeneration.
- Check for all 88: with the `cfg=<hex>` token removed, each file equals its HEAD version
  (88 of 88, `raw/golden/golden-cfg-check.json`).
- A search of the fixtures and goldens for parenthesized or brace-opened multi-line import
  headers found none. No other golden could change.

## Validation on MMSP `d62f06a`

Isolated bench, a fresh clone at the pin, and the new debug binary (SHA-256
`5bde071d…d2da`). Before-state: the Task 6 index, built by the previous binary.

- Two cold scans (`cold-index.ps1`): exit 0. Counters are identical to Task 6: discovered
  2,277, processed 1,475, definitions 18,785, imports 6,680, calls 99,720, errors 2,387,
  unsupported 798, binary 4.
- The 6,680 import facts match the Task 6 facts one to one by (path, `byte_start`,
  `byte_end`). `sl`, `sc`, `ec`, text, provider, grammar and language differ in 0 rows.
  `end_line` differs in 17 rows. All 17 are multi-line imports, and each new value equals
  the line of `byte_end`.
- Import position mismatches: 17 before, 0 after.
- Full-tree span check: symbols 18,785 and calls 99,720 have 0 line and 0 name mismatches
  (same as Task 6). Imports: 6,680 rows, 0 line mismatches, 0 name mismatches. Column base
  is 1 and byte-based in every table.
- Fact digests: `files`, `symbols`, `calls` and `edges` are identical to Task 6. Only
  `imports` differs, as expected. Cold run 1 and cold run 2 after the fix are identical.
- Frozen queries (22): 20 successful answers match Task 6 after normalization (bench root,
  snapshot id, `created_unix`, `nonce`, `revision`). The 2 error outputs are byte-identical.
  Cold run 1 and cold run 2 match on all 22. Grader verdicts are unchanged: honesty 17 PASS
  and 1 UNAVAILABLE, capability 4 UNMET, evidence 2 UNAVAILABLE.
- Scan partition unchanged: ok 1,475, unsupported 798, skipped 4.
- Invalidation: the Task 6 index (version 6), read by the version 7 binary, answers
  `freshness: stale` with `parser-changed` and `config-changed`.
- Snapshot: the fresh clone has HEAD `d62f06a`, tree `c5482ea`, empty porcelain, and tracked
  file hashes identical to the Task 6 clone (2,277 of 2,277). The original MMSP repository is
  unchanged (HEAD `fa5f74d`, empty porcelain).

## Test suite and checks

- `cargo test --locked`: 781 passed, 0 failed (777 before, plus the 4 new tests).
- `cargo fmt --all -- --check`: exit 0. `cargo fmt --all` reformatted one line of a new test.
- `cargo clippy --all-targets -- -D warnings`: exit 0.

## Status

- The import span defect recorded in Task 6 is closed. For the checks in this run, G-SYN
  passes: source-grounded spans for all fact tables, deterministic repeats, exact unsupported
  counts, and parser errors recorded.
- The gate text still refers to the archived release-candidate scan report (CP3 §11). This
  change does not produce that report.
- G-GRF is unchanged. Its two open transparency items (indirect dispatch absent, CUDA
  boundary not named by the trace) are out of scope here.
- Capability verdicts are unchanged: D1-C6, D2-C6, D3-C5 and D4-C5 stay UNMET.

## Evidence

- [pilots/m9-gsyn-fix/raw/](pilots/m9-gsyn-fix/raw/), redacted: test logs, golden check,
  cold-run logs and counters, span checks, digest and import comparisons, query
  comparisons, grading, scan summary, and environment identity.
- [pilots/m9-gsyn-fix/method/](pilots/m9-gsyn-fix/method/): `import-before-after.cjs` and
  `cmp-query-dirs.cjs`. The span and query checks are in
  [pilots/m9-task6/method/](pilots/m9-task6/method/).
- The Task 6 archive ([pilots/m9-task6/](pilots/m9-task6/)) is not modified.
