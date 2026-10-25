# M9 amendment: G-SEM re-verification and `.inl` recovery review

Dated revision: 2026-10-25. This amends the G-SEM status recorded in
[m9-cp3-evidence.md](m9-cp3-evidence.md) §6.2 and in
[m9-cp4-evidence.md](m9-cp4-evidence.md). Those records are unchanged.

## Result

- **G-SEM: PASS** on the acceptance test in
  [m9-design-gate.md](m9-design-gate.md): two providers from different language
  families (rust-analyzer for Rust, scip-typescript for TypeScript), each with
  source-grounded definitions and references, verification labelled
  `Unverified`, and no syntax candidate promoted to a semantic label.
- Owner decisions F1, F2 and F5 below stay open. F5 may conflict with a
  `CLAUDE.md` invariant and should be settled before the release candidate.
- **C++ on MMSP: NOT YET VERIFIED.** D-M9-2 defers clangd; MMSP has no
  compilation database; no C++ provider has run.
- **CUDA: unsupported.**

## Task 2 review: `.inl` recovery risk

| File | Syntax errors | Definitions (nameless) | Calls (cast-like) | Span mismatches | Name mismatches |
|---|---|---|---|---|---|
| `Array.inl` (no errors) | 0 | 84 (39) | 100 (1) | 0 | 0 |
| `FEMTree.inl` (moderate) | 14 | 84 (3) | 625 (4) | 0 | 0 |
| `FEMTree.System.inl` (highest) | 133 | 118 (8) | 1609 (33) | 0 | 0 |

Method: each definition and call from `scan` was checked against the source
by byte range. The byte range matches the reported start and end line and
column, and the byte slice contains the reported name (whitespace ignored).
Spot checks against the source lines matched, for example `_assertBounds`
(line 237), `FEMTree< Dim , Real >::merge` (line 1944) and the `_InBounds(p)`
call (line 1102).

Observations, all present in the C++ syntax layer before this change and
present in `.h` headers too. None comes from recovery, and none changes Task 2:

- Operator overloads are recorded with `name: null`.
- C-style and `*_cast<>` expressions are recorded as call candidates, for
  example `(Real)(value/2)`. They stay `POTENTIAL_CALL`. `FEMTree.h` has 53.
- Forward declarations such as `class X;` are recorded as `class` definitions.

Recovery did not produce `CALLS`. `callers` of `FEMTree< Dim , Real >::_dot`
in `FEMTree.System.inl` returned only `POTENTIAL_CALL` edges (provider
`syntax`, `Unverified`, precision `Heuristic`, `callee` null). The MMSP
semantic status lists no imports, so no SCIP-backed edge exists for MMSP.

Error visibility: `scan` reports `errors` per file and in `summary` (MMSP:
2387 in total, across 179 files with errors). `callers` reports
`completeness.status: partial` and `parse_error_files: 179`, a repository-wide
count. `status` stays `ok` for processed files; the output contract is
unchanged.

Conclusion: no defect in the `.inl` path was found. Task 2 is unchanged.

## Providers and configuration

- **rust-analyzer 1.99.1** (`58cf8f9c 2026-10-13`), stable toolchain component,
  run as an LSP subprocess with an absolute `--lsp-server` path. Installed for
  this review with `rustup component add rust-analyzer`, a host toolchain
  change.
- **scip-typescript 0.4.0**, installed globally with
  `npm i -g @sourcegraph/scip-typescript@0.4.0`, a host change. Producer
  command: `scip-typescript index --cwd <fixture> --output index.scip`. The
  fixture has no local `node_modules`, and the producer run succeeded.
- **riltra**: debug build at HEAD `f3e171c`. No code changed in this revision.
- **Harness**: [`docs/pilots/cp3/method/g-sem.ps1`](pilots/cp3/method/g-sem.ps1),
  unchanged. Its fixtures are the `fixture-rs` and `fixture-ts` projects the
  script writes.
- Raw outputs, redacted, are archived in
  [`docs/pilots/m9-gsem/raw/`](pilots/m9-gsem/raw/).

## Runs

Rust answers depend on the bounded analysis wait (15 s). Each attempt is listed.

| Attempt | rust-analyzer definition | rust-analyzer references | TypeScript import | Notes |
|---|---|---|---|---|
| `cold-attempt` | 0 | 0 | import 1: 2 documents, 4 definitions, 6 references | empty answers with `analysis wait timed out; answers may be empty` |
| `warm-second-run` | 1 | 3 | import 2 (same artifact) | answers present, same diagnostic |
| `run1` (fresh home) | 1 | 3 | import 1 | answers present, same diagnostic |

Byte verification against the fixture source, automated:

- rust-analyzer definition: bytes 3 to 8 are `greet` (`run1`, `warm-second-run`).
- rust-analyzer references: bytes 96 to 101, 127 to 132 and 3 to 8 are `greet`.
- scip-typescript references: bytes 9 to 14 (import binding) and 46 to 51
  (call site) are `greet`. Symbol: `scip-typescript npm tierb-ts 1.0.0 src/\`greet.ts\`/greet().`
- scip-typescript definition from the call site: `src/greet.ts` bytes 16 to 21
  are `greet`.

Line and column fields in semantic answers are 0-based, following the SCIP
convention noted at `crates/cli/src/semantic.rs:53`. Byte offsets are the
checkable location.

## Trust boundary

Probes on the `run1` home:

| Probe | Result |
|---|---|
| Absolute LSP path that does not exist | exit 0, `fallback` with `reason: language server not found` and `syntax_fallback: true`; no answers |
| Relative LSP path | exit 3, `language server path must be absolute` |
| Semantic query with no provider configured | exit 3, `no semantic occurrence here` |
| `callers` on the `greet` definition (SCIP anchor) | one `REFERENCES` edge, provider `scip-import`, `Unverified`; no `CALLS` |
| `callers` on the Rust `greet` (syntax anchor) | two `POTENTIAL_CALL` edges, provider `syntax`, `Unverified`, `callee` null |
| Same SCIP artifact imported a second time | exit 3, `ambiguous anchor: same identity from 2 imports, resolve with a rebuild` |

Provenance: each import records `provider`, `provider_version`, `artifact_id`,
`content_identity_match` and `source_revision_match`. Each answer records
`provider`, `provider_version` (LSP) and `verification`.

Verification labels: every semantic answer is `Unverified`. `Current` was not
observed.

## Open items for the owner

- **F1.** `semantic status` reports `limitation: "unsupported semantic provider:
  'scip-typescript'"` for the TypeScript import, yet answers are served. The
  status resolves the producer name (`crates/cli/src/index.rs:957`) against the
  registry, which holds only the transports `scip-import` and `lsp-subprocess`
  (`crates/cli/src/semantic.rs:224`). The label misleads, but no claim is
  unverified.
- **F2.** Importing the same artifact twice creates a second import with the
  same identity. `callers` then refuses with `ambiguous anchor` until a
  rebuild. The refusal is explicit; re-import is not idempotent.
- **F3.** The rust-analyzer answer depends on the 15-second analysis wait. The
  cold attempt returned empty answers. The diagnostic appears whenever the wait
  times out, so the result is never silent.
- **F4.** `semantic status` lists no LSP provider, because LSP runs as a
  subprocess per query. The harness status line therefore prints `null`.
- **F5.** SCIP-backed `CALLS` and `REFERENCES` edges carry `precision: "Exact"`
  (`crates/cli/src/edges.rs:578` and `:677`). `Precision::Exact` is documented
  as never constructed in M5, and `CLAUDE.md` requires an independent proof
  before Current or Exact is constructed. Verification stays `Unverified`. The
  owner should confirm the M6 contract (exact span from the producer) or
  relabel the field.

## Limits that remain

- C++ semantics on MMSP: NOT YET VERIFIED, for the reasons above.
- CUDA semantics: unsupported.
- Provider coverage is fixture-level only. MMSP-scale provider behaviour is not
  measured here.

## Revision 2026-10-25: F1 resolved

F1 is resolved. `semantic status` checks every stored import against the
`scip-import` transport (`SCIP_IMPORT` in `crates/cli/src/semantic.rs`). A
`scip-typescript` import no longer reports `unsupported semantic provider`.
The producer name and version stay in the row metadata. The limitation now
reads: "producer name and version are artifact metadata, not checked by
riltra". An unknown producer still appears with its name, marked as unchecked.
No producer joins the registry, and the test that rejects `scip-typescript`
as a registry key is unchanged.

Tests (red before the fix, green after):
`cli_f1_scip_status_names_the_transport_and_keeps_producer_metadata` and
`cli_f1_unknown_producer_is_reported_as_unchecked_metadata`.

The original findings above are kept as recorded. F2 and F5 are unchanged in
this revision.

## Revision 2026-10-25: F2 resolved

Owner decision: option (a), revalidation of identical imports. An identical
re-import on the same snapshot is accepted and revalidates the existing import.
It is no longer refused with `already imported`.

Behaviour (`import_file` in `crates/cli/src/scip.rs`, helpers in
`crates/cli/src/store.rs`):

- The oldest identical import keeps its id and takes the facts of the new
  attempt. Older identical rows, left by the earlier append-only import, are
  removed. One import row per artifact remains on a snapshot.
- Edges are re-derived for the snapshot. Import ids, provider metadata, the
  snapshot identity, and the nonce are unchanged. The semantic generation
  advances, as for any import.
- A source-digest claim that the earlier import carried survives a re-import
  that brings none. A claim supplied with the re-import replaces it.
- A different artifact that claims the same identity still imports. The anchor
  stays ambiguous until a rebuild.

Change against the F2 requirement: the database is not byte-identical after an
identical re-import, because the surviving import's fact rows are rewritten with
new row ids. Row counts, import ids, and every answer are unchanged. The owner
accepted this trade-off with option (a).

Contract change: `task7_upgrade::reimport_revalidates_every_import_without_changing_snapshot`
asserted that an identical re-import makes the anchor ambiguous. It now asserts
that every case, `alias` included, is restored, since the duplicate no longer
exists. The ambiguity rule is still tested for distinct artifacts.

Tests:

- `cli_f2_identical_reimport_revalidates_the_existing_import`: red before the
  change (a second import row appeared), green after.
- `cli_f2_identical_reimport_collapses_legacy_duplicates_to_the_oldest_id`.
- `cli_f2_revalidation_keeps_the_earlier_source_claim_unless_replaced`.
- `cli_f2_distinct_artifact_with_the_same_identity_stays_ambiguous`: passes
  before and after.
- `task7_upgrade::reimport_revalidates_every_import_without_changing_snapshot`:
  red before, green after, with the assertion above.
- `cli_graph_import_after_rebuild_rebinds` and
  `edges_lifecycle_rebuild_rederives_after_target_change`: unchanged, passing.

Full suite (`cargo test --locked`): 776 passed, 0 failed, 8 test targets (772
before this revision, plus four new F2 tests). `cargo fmt --all -- --check` and
`cargo clippy --all-targets -- -D warnings` are clean.

## Revision 2026-10-25: F5 reviewed; syntax import edges corrected

F5 asked whether `precision: "Exact"` on SCIP-backed edges agrees with the M5
and M6 contracts and with the invariant in `CLAUDE.md`. This revision records
the review and one correction. The historical records are unchanged.

### What `Exact` means

- In M6 and in the code, `Exact` labels the evidence strength of one edge: the
  span and endpoints come from the SCIP producer and are bound to the
  digest-matched source riltra indexed. It is not verification. `Verification`
  stays `Unverified`, and `Exact` never implies `Current`
  ([m6-task4-validation.md](m6-task4-validation.md),
  [m6-validation.md](m6-validation.md)).
- Two older statements use the word differently. `crates/cli/src/semantic.rs`
  (lines 12-17) calls `Precision::Exact` "compiler-backed" and says it is never
  constructed in M5. Lines 184-185 of the same file and
  `crates/cli/src/scip.rs:607` say no provider output is `Exact` without
  producer attestation. Riltra has no producer attestation: the producer name
  and version are unchecked metadata (F1).
- `CLAUDE.md` says `Current` and `Exact` need an independent proof mechanism
  before they are constructed.

### Every product path that constructs `Exact`

| Site | Edge | Meets the definition | Finding |
|---|---|---|---|
| `crates/cli/src/edges.rs:309` | IMPORTS, provider `syntax` | no | Syntax only, no producer. Every other syntax edge is `Heuristic` (`edges.rs:269`, `:384`, `:720`). Corrected to `Heuristic`. |
| `crates/cli/src/edges.rs:579` | CALLS, `scip-import` | yes | The callee must be a callable declaration at the producer's target span in riltra's own parse of the digest-bound source (`callable_target`, `edges.rs:545`), and the caller owner must be proven. This is the "source-proven" CALLS that `CLAUDE.md` names. |
| `crates/cli/src/edges.rs:678` | REFERENCES, `scip-import` | partly | The producer span is exact and digest-bound, but the callee is not checked. A temporary probe, removed and not committed, stored a plain reference to a symbol with no definition in the import as `REFERENCES / Exact / scip-import`. |

Test-only constructions (`edges.rs:1687`, `:3184`, `index.rs:4437`,
`store.rs:2444`) are not product paths.

### Consistency

- M5: still holds for semantic facts, which stay `Heuristic` (`scip.rs:608`).
  Edge-level `Exact` postdates M5 and was introduced by M6.
- M6: consistent for CALLS and for REFERENCES with a definition. Inconsistent
  for IMPORTS (corrected here) and for REFERENCES to an undefined target. The
  M6 wording "Exact REFERENCES with proven endpoints"
  ([m6-task6-validation.md](m6-task6-validation.md), line 33) is not met for an
  undefined target.
- `CLAUDE.md`: its phrase "CALLS (SCIP-backed, source-proven)" matches the CALLS
  path. The sentence "`Current`/`Exact` exist only for contract stability" is
  stale, because M6 constructs `Exact`. Nothing constructs `Current`; its only
  use is the name mapping at `semantic.rs:45`.
- MEDIUM is unreachable from production derivation. REFERENCES is emitted only
  at `edges.rs:678`, always without a caller, and `impact_verdict` returns
  UNKNOWN for any edge without a caller. The M6 corpus has zero MEDIUM answers
  ([m6-task7-validation.md](m6-task7-validation.md)).

### Correction in this revision

- `edges.rs:309`: IMPORTS precision `Exact` becomes `Heuristic`. No code reads
  the value. IMPORTS is excluded from edge selection (`store.rs:1955`), and no
  golden contains IMPORTS. Indexes built earlier keep the old value until they
  are derived again. No answer changes.
- Regression: `edges_derive_syntax_only_calls_are_honest_potentials` now asserts
  that every syntax-provider edge is `Heuristic`. Red before (IMPORTS was
  `Exact`), green after.

### Owner decision: definition of `Exact` for REFERENCES

Three options follow from the sources:

- **B. Proven endpoint (recommended).** `Exact` on REFERENCES only when the
  callee has a definition in the same import. Otherwise `Heuristic`. The change
  is one condition in the plain-reference branch (`edges.rs:668-682`). Labels
  change only for REFERENCES to undefined targets in `callers` and `callees`;
  impact is unaffected. This matches the M6 wording. `CLAUDE.md` needs only its
  stale sentence updated.
- **A. Producer-stated span.** `Exact` means a producer span that is
  digest-bound and import-scoped, as the F5 wording states. No code change
  beyond IMPORTS. `CLAUDE.md` and the M6 wording "proven endpoints" must be
  amended.
- **C. Strict `CLAUDE.md`.** No `Exact` on SCIP-derived REFERENCES. CALLS keeps
  `Exact` only if the owner accepts the callable-declaration proof as
  independent. This changes every REFERENCES label; 11 golden files contain
  REFERENCES.

Recommendation: B. B is not an independent proof in the strict sense of
`CLAUDE.md`, and only C meets that reading. B is the smallest change that makes
the code match the M6 text without editing `CLAUDE.md`. `CLAUDE.md` is not
tracked by git in this repository, so its wording is owner work.

Tests: full suite `cargo test --locked`: 776 passed, 0 failed, 8 test targets.
`cargo fmt --all -- --check` and `cargo clippy --all-targets -- -D warnings`
are clean.
