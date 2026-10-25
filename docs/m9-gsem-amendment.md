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
