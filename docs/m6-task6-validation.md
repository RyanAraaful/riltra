# M6 Task 6 validation — conservative impact analysis

Local checkpoint on `m6-task6-conservative-impact`, based on accepted Task 5
commit `ec8c2d452fcaf3e99bb7fd3af5add154ea20f85f`. The commit carrying this
report is the audit unit. Scope ends at Task 6: no push or Task 7–8 execution.
No dependency, schema migration, persistent cache, or new graph engine.

## Command and interpretation

```text
riltra impact --repo R --path P --line L --col C
              [--symbol URI] [--depth N] [--limit N] [--offset N]
```

Impact uses Task 4 positional anchor resolution, byte columns and repository
selection. `--to` is rejected. Exit codes remain 0 for a valid answer
(including UNKNOWN/empty results), 2 for invalid arguments, and 3 for
unresolved/ambiguous anchors or path/data errors.

Reverse BFS emits each observed inbound proof path, including intermediate
dependents. `distance: 1` / `scope: "direct"` means one incoming relation;
longer paths are `transitive`. Edges retain their original caller → callee
orientation, but the path array lists the edge nearest the queried anchor
first. In f0 → f1 → f2, impact(f2) returns [f1 → f2] followed by
[f1 → f2, f0 → f1]. Each hop has the shared Task 4 source/digest/producer
artifact/revision evidence. Unknown endpoints remain null and terminal.
The disclaimer is `plausible-impact-not-execution`.

| Verdict | Required evidence |
| --- | --- |
| UNKNOWN | Default, no observed relation, stale source/import, unresolved or heuristic relation, missing owner/midpoint, ambiguous identity/dispatch, coverage gap, traversal cutoff, or incomplete pagination |
| HIGH | At least one observed strict CALLS path, with no detected uncertainty in the conservative query scope |
| MEDIUM | Observed Exact REFERENCES with proven endpoints and no detected uncertainty, when there is no CALLS path |

The implementation has no SAFE or LOW outcome. HIGH/MEDIUM describe observed
static dependencies; neither proves execution or an exhaustive graph.
`Unverified` remains `Unverified`, even for Exact edges and HIGH. A plain SCIP
reference currently has no proven callable owner under the accepted Task 3
derivation: its site evidence is returned, its owner stays null, and impact
returns UNKNOWN. MEDIUM policy is tested with a stored known-owner reference
fixture; this checkpoint does not infer new reference ownership to reach it.

All impact responses have `completeness.status: "partial"` and reason
`graph-completeness-unproven`. Exhausting the observed traversal never changes
that. Repository-wide unknown counters are deliberately conservative: an
unrelated unresolved call or uncovered file can make this query UNKNOWN.
Counters overlap and are diagnostics, not a percentage or completeness proof.

## Bounds, identity, and consistency

Depth defaults to 3, maximum 6; admitted full endpoint identities ≤500;
examined edges ≤3000; emitted proof paths ≤1000; owner mappings ≤500.
The 2000 ms deadline is cooperative, inherited from Task 5: checks occur
between traversal operations and scoped mapping reads. A single SQLite or
filesystem operation cannot be interrupted, and freshness preparation or
serialization can extend total command duration. No hard process-duration
guarantee is claimed. Completed traversals and deterministic work cutoffs
have stable ordering; a time-cut prefix may vary with machine load and
explicitly reports truncation.

FIFO expansion and Task 4's total edge ordering determine results. Path-local
lineage stops recursion/cycles while preserving separate diamond routes.
Already emitted evidence survives later cutoffs. Bound failures are explicit
`depth-budget`, `node-budget`, `edge-budget`, `path-budget`, `mapping-budget`,
or `time-budget` reasons and force UNKNOWN.

SCIP keys retain import and local-document scope. Syntax anchors use only
the validated digest/file/span-bound owner mapping. Multiple mappings remain
ambiguous; a syntax midpoint without a proven semantic identity stops with
`semantic-midpoint-unavailable`. Inbound queries use exact endpoints, with no
name-based candidate selection or speculative target fabrication. Syntax
fallback retains its uncertainty even when an evidence-backed bridge exists.

One read transaction covers the snapshot, anchor, reverse traversal,
coverage, freshness and all output evidence. The existing query-local
`import_verdict()` memo is shared; no cache survives a request. Query assembly
and JSON models are shared with Task 5; existing trace/callers/callees goldens
are unchanged.

Pagination defaults to 50, maximum 1000. It slices the collected path sequence;
`next_offset` continues that sequence rather than a persisted BFS frontier.
Incomplete pages, nonzero offsets, and out-of-range empty pages produce
UNKNOWN. `total_exact` refers only to the collected observed traversal and
is false after a traversal cutoff. Compare snapshot nonces between pages.

## Regression corpus and RED/GREEN evidence

The mandatory Task 6 corpus is implemented in the existing edge test suite
using minimal inline fixtures; it does not start Task 7's language matrix.

| Test | Evidence |
| --- | --- |
| `task6_impact_direct_and_transitive_are_proven_without_reindex` | Imported chain, HIGH, direct/transitive scope, per-hop producer proof, Unverified, partial completeness, repeatability |
| `task6_impact_zero_false_safe_syntax_corpus` | Eight UNKNOWN cases: clean leaf, recursion, cycle, dynamic receiver, missing definition, same-name definitions, unproven owner, alias; no guessed inbound endpoints |
| `task6_impact_unknown_for_stale_depth_and_coverage_gaps` | Stale imported graph, depth cutoff, unsupported file, missing provider, imported leaf without inbound evidence; injected file counters equal exactly 1 |
| `task6_impact_cycles_diamond_pagination_and_empty_pages` | Proven cycle terminates; diamond preserves both routes; pages match full ordering; omitted/empty pages UNKNOWN |
| `task6_impact_reference_unknown_owner_retains_site_evidence` | REFERENCES retains raw source span with null owner, UNKNOWN |
| `task6_impact_unmapped_midpoint_is_explicitly_unknown` | Proven direct CALLS retained, unbridged syntax caller never becomes a semantic chain |
| `task6_impact_ambiguous_semantic_dispatch_counts_unknowns` | Two overlapping semantic targets yield exactly one ambiguous call and one unresolved target, UNKNOWN |
| `task6_impact_owner_mapping_import_and_local_scope` | Proven fallback bridge with uncertainty, multiple-import ambiguity, and document-confined local recursion |
| `task6_impact_rejects_bad_bounds_destination_and_escaping_path` | Bounds, forbidden destination, path confinement and exit codes |
| `task6_impact_full_json_goldens` | Entire syntax, imported HIGH, and bounded UNKNOWN wire responses |
| `task6_impact_budget_guards_and_verdicts_never_claim_safety` | Injected node/edge/path/depth/time cutoff reasons, invalid 501-node budget, UNKNOWN priority, known-owner reference MEDIUM, heuristic downgrade |
| `task6_impact_pins_coverage_and_verdict_during_promotion` | Actual impact transaction concurrent with writer-thread promotion; old/new nonce and unknown counts stay separate, old response Stale and UNKNOWN |

Initial RED: all five initial functional CLI tests failed on the missing
impact command (exit 2). After implementation, all passed. A subsequent
regression reproduced HIGH at an unbridged syntax midpoint; it now returns
UNKNOWN and an explicit reason while retaining its direct CALLS evidence.
Direct/transitive metadata also had an observed RED before implementation.
Additional corpus/budget/golden/concurrency tests extend coverage; historical
RED is not claimed for each of those additions.

There are zero SAFE/LOW results in the corpus and smoke checks. Expected
UNKNOWN cases assert UNKNOWN explicitly, including clean leaves with no
observed impact. Positive cases assert the exact verdict and proof, not merely
the absence of SAFE. Inherited Task 3–5 proof, freshness, lifecycle, repository,
import/document, source encoding, LSP, and promotion regressions still pass.

Independent read-only review found no blocking correctness findings. Its
minor denominator assertion recommendation was addressed with exact counts;
follow-up review found no remaining correctness issues.

## Validation and examples

All local commands returned exit 0 on Windows:

```text
cargo fmt --all -- --check
cargo check --locked
cargo clippy --all-targets -- -D warnings
cargo test --locked
cargo build --locked -p riltra
```

**312 tests passed, zero failed/ignored**: 198 unit, 16 CLI, 50 edge
integration, 21 index, 15 semantic, 12 syntax. Baseline: 300; Task 6 adds
12 tests. Linux/macOS CI was not run locally.

Real CLI smoke made 15 invocations: init/index/import; syntax UNKNOWN;
SCIP no-reindex direct/transitive HIGH with partial completeness; exact
repeat; depth cutoff UNKNOWN; first/last/empty pages UNKNOWN; fallback
bridge; stale UNKNOWN; invalid depth/destination; missing anchor. Expected
exit codes 0/2/3, terminal bytes and producer evidence were checked. Impact
queries on the tiny fixture took approximately 17–34 ms, not a scalability
claim. Logs/raw JSON are in gitignored `target/m6-task6/`.

Portable complete JSON examples, checked as goldens:

- [Imported chain / HIGH](../crates/cli/tests/golden/m6-impact-scip.json)
- [Syntax-only / UNKNOWN](../crates/cli/tests/golden/m6-impact-syntax.json)
- [Depth cutoff / UNKNOWN](../crates/cli/tests/golden/m6-impact-truncated.json)

Stop at this local checkpoint for Task 6 audit. No push or Task 7–8 work.
