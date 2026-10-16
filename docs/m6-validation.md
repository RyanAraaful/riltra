# M6 Task 4 validation — callers / callees

Task 4 checkpoint, based on `8ce51f2` (Tasks 1–3). This continues the
unfinished CLI working-tree changes; it does not implement Tasks 5–8.
The local checkpoint commit carrying this document is the audit unit.

## CLI and wire contract

```text
riltra callers --repo R --path P --line L --col C [--symbol URI] [--limit N] [--offset N]
riltra callees --repo R --path P --line L --col C [--symbol URI] [--limit N] [--offset N]
```

Positions are 1-based lines and UTF-8 byte columns in decoded text. Evidence
spans are half-open offsets into the original raw source, including UTF-16
BOMs where applicable. SCIP producer UTF8/UTF16/UTF32 occurrence columns
are normalized through their stored byte offsets, not compared directly
with CLI columns. M5 storage and query conventions remain unchanged.

`schema_version: 1` includes repository, snapshot ID/nonce/revision,
direction, anchor identity/location/precision/uncertainty, freshness,
verification, completeness, edges/evidence, limits, continuation, and
truncation. Existing manual JSON serialization is retained; no dependency
or schema migration was added. Runtime timings are deliberately excluded
by user decision so repeated queries on the same state are byte-identical.

Semantic anchors use the narrowest containing occurrence with valid SCIP
provenance. Tied distinct identities/imports produce an explicit data
error. `--symbol` checks the positional identity; it cannot merge imports.
Syntax fallback is explicitly `syntax-fallback` / `Heuristic` with
`anchor_uncertainty`, and requires a digest-matching source file. For stale
semantic anchors, uniquely recorded syntax line bases can locate old
single-line occurrences; missing coordinate proof fails rather than guesses.

One SQLite read transaction covers the snapshot, anchor, edge selection,
counts, evidence, and query-local `import_verdict()` memo. The `freshness`
object reports M4 syntax-snapshot freshness; semantic import drift is also
reported through verification and `stale_imports`. `Exact` never elevates
`Unverified` to `Current`; source/import drift downgrades to `Stale` even
for empty pages.

## Relationships, provenance, and coverage

Exact endpoint matching preserves repository, snapshot, import, and local
document scope. Syntax-owner aliases for semantic `callees` reuse the
same digest-bound, unambiguous owner mapping as graph derivation. They
retrieve unresolved outbound calls without changing their identities or
promoting them to `CALLS`.

Inbound syntax candidates are restricted to the same file or conservative
direct imports. Recognized bindings cover Python imports, literal relative
JS/TS imports and C/C++ includes, and Rust `use crate::...`; this is not a
general module resolver. Imported names are not mistaken for module names.
Candidates remain `POTENTIAL_CALL`, with null unresolved targets. Semantic
candidates from a different import are excluded. Unknown owners remain null.

Evidence includes source path, raw byte span, callee/terminal/reference
sub-spans where available, source digest, provider, and proven producer
import identity/artifact/revision/content binding. Unknown-endpoint semantic
edges retain their producer import in evidence; query-time overlap guessing
is forbidden. Legacy v4 unresolved edges without that metadata keep unknown
provenance rather than borrowing another import. Rebuild/reimport regenerates
their evidence; no migration or cross-request cache is needed.

The page limit defaults to 50 and permits 1–1000. Shared SQL predicates
drive count and page selection, with a total order over caller path/start,
full callee key, remaining endpoint/span/relation/provider fields, and row ID.
Filters precede pagination; offsets beyond 1000 work. `next_offset` exists
only when more rows remain. Continue only within the same snapshot nonce;
restart pagination after promotion.

`covered` counts returned rows; `total` counts all matching stored rows.
`denominator.observed_edges` is query-scoped; unknown relationship/provider
and file counters conservatively cover the repository. Diagnostic counters
can overlap and are not a coverage percentage. Unsupported, missing-provider,
dynamic, ambiguous, unknown-owner/target, stale-import, skipped, and parse-error
counts prevent `complete`. Empty results, uncertain anchors, omitted earlier
pages, and further pages also prevent `complete`. Even `complete` describes
only an observed static result set, never execution safety or `Current`.

## Regression evidence

Inherited tests were already green: no historical RED is claimed for them.
The following failures were observed before their respective fixes:

| Regression | Observed RED | GREEN evidence |
|---|---|---|
| Initial CLI gaps | Six failures: missing JSON contract, false `complete`, changed-source fallback accepted, zero position accepted, total capped at 1000, imported name treated as module | `task4_json_contract_and_source_evidence`, `task4_unresolved_and_empty_are_not_complete`, `task4_syntax_fallback_rejects_changed_source`, `task4_rejects_zero_positions_and_limit`, `task4_pagination_beyond_one_thousand`, `task4_import_name_is_not_module_identity` |
| Conflicting owner digest and path escaping | Matching row ID bridged old digest; malformed evidence JSON | `edges_bridge_matching_file_id_does_not_override_changed_digest`, `edges_evidence_escapes_source_paths` |
| Unknown producer | Unknown-owner semantic edge lost producer import (0 instead of 1) | `task4_unicode_terminal_and_unknown_owner_have_no_fabrication` |
| Import and page isolation | Other-import candidate returned; last page incorrectly `complete` | `task4_cross_import_identity_and_ambiguous_anchor`, `task4_last_page_is_not_full_coverage` |
| Unobserved callees | Unknown call elsewhere ignored by completeness | `task4_unresolved_elsewhere_is_not_proof_of_complete_callers` |
| Owner and source proof | Ambiguous owner selected first; stale reference/target yielded `CALLS` | `edges_bridge_ambiguous_semantic_owners_stays_syntax`, `edges_stale_reference_or_target_digest_never_proves_calls` |
| Independent review findings | Semantic `callees` returned 0 unresolved edges instead of 1; UTF16-position identifier byte column 11 selected fallback | `task4_semantic_callees_keeps_unresolved_syntax_owner_edges`, `task4_semantic_anchor_uses_byte_columns_for_all_scip_encodings` |
| Binary coverage | Skipped binary source still reported `complete` | `task4_binary_source_is_an_explicit_coverage_gap` |
| Multiple dispatch candidates | One terminal with two symbol identities produced 2 `CALLS` | `edges_two_semantic_targets_at_one_terminal_are_potential_only`: 0 `CALLS`, 1 ambiguous candidate |

Additional coverage proves same-name/cross-repository isolation, local symbols
in two documents, stale empty-page verification, UTF-16 raw evidence,
half-open Unicode boundaries, full JSON goldens, deterministic repetition,
empty/overshoot pages, rollback, and incremental/full lifecycle preservation.
The live LSP mock test executes locally without an opt-in environment flag:
the graph response is identical before/after the query and no provider is
persisted.

`task4_actual_query_pins_anchor_edges_and_evidence_across_promotion` pauses
the actual query after its first snapshot read, joins a writer thread that
adds a file and promotes an incremental index through another connection,
then finishes the original query. It retains the original nonce, one edge,
anchor, and source digest; the following query sees the new nonce and two
edges. The old result is `Stale`. No sleeps or global hooks are involved.

Small Task 3 repairs are limited to evidence escaping, source binding,
ambiguous owner/terminal proof, and persisting the producer import for
unresolved edges. Their regressions are listed above. No M0–M5 refactor,
new dependency, database version change, trace, or impact implementation.

## CLI smoke and JSON examples

An isolated two-file Python repository was registered and indexed using
the built CLI. A minimal real protobuf SCIP artifact was imported using
`semantic import`, without reindexing. All successful commands exited 0;
zero line exited 2 with empty stdout, and out-of-range anchor exited 3.
After changing `lib.py`, the same semantic query returned `Stale`.

| Query | Anchor | Relation / verification | Wall time | Output bytes |
|---|---|---|---:|---:|
| callers, syntax | syntax-fallback | POTENTIAL_CALL / Unverified | 17.39 ms | 1681 |
| callees, syntax | syntax-fallback | POTENTIAL_CALL / Unverified | 16.41 ms | 1685 |
| callers, SCIP | semantic | CALLS / Exact / Unverified | 16.01 ms | 2017 |
| callees, SCIP | semantic | CALLS / Exact / Unverified | 15.16 ms | 2021 |
| callers after edit | semantic | CALLS / Exact / Stale | 16.73 ms | 2059 |

These are single local process measurements, not benchmarks or speed claims.
Snapshot nonce remained identical across syntax indexing and semantic import.
Repeated callers output was identical. The SCIP artifact inside the demo
repository is an unsupported file, so coverage remains honestly `partial`.

Full examples, asserted as parsed JSON goldens (only nonce and producer
artifact identity normalized):

- [SCIP callers](../crates/cli/tests/golden/m6-callers-scip.json)
- [SCIP callees](../crates/cli/tests/golden/m6-callees-scip.json)
- [Syntax callers](../crates/cli/tests/golden/m6-callers-syntax.json)
- [Syntax callees](../crates/cli/tests/golden/m6-callees-syntax.json)

Reproduce the smoke path through `cargo test --locked --test edges`; the
tests write the protobuf artifacts and execute the actual CLI. For manual
inspection, set `RILTRA_HOME` to a disposable directory, register the fixture,
then run `index`, `callers`, `callees`, `semantic import`, and repeat queries.

## Validation gates and limits

Final local gates (Windows, all exit 0):

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo check --locked` | PASS, no warnings |
| `cargo clippy --all-targets -- -D warnings` | PASS |
| `cargo test --locked` | PASS: 285 tests, 0 failed/ignored |

Test breakdown: 191 unit + 16 CLI + 30 edges + 21 index + 15 semantic +
12 syntax. The inherited baseline had 261 passing tests; 24 tests were added.
The initial Clippy failures were repaired; final gates have no suppressions
added for Task 4. A local independent reviewer reproduced two important
findings, verified their RED/GREEN fixes, and reported no new important
findings in the focused follow-up review.

Testing is local Windows; this checkpoint does not claim a new remote CI or real external
language-provider run. Existing opt-in external-provider/pilot tests remain
gated. No remote push was performed.

Page materialization is bounded to 1000 edges; source reads retain the 1 MB
cap and canonical confinement. Freshness/file coverage still use the existing
repository walk, so runtime is not constant in repository size. There is no
Task 5 traversal/time budget or cross-request cache. Sparse changed-source
coordinates without snapshot proof require reindexing. Conservative import
candidate recognition can omit unresolved candidates, always with partial
coverage rather than invented targets.

Stop at this checkpoint for GO / REVISE / BLOCKED audit before Task 5.
