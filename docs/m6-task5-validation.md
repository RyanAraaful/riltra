# M6 Task 5 validation — bounded static trace

Local checkpoint based on Task 4 commit
`54cd11439b6ca3aebfd120c8d9f2fb3993b405c1`, on branch
`m6-task5-static-trace`. Scope ends at Task 5. No push, impact implementation,
Task 6–8 corpus expansion, dependency, or schema migration is included.

## CLI and result contract

```text
riltra trace --repo R --path P --line L --col C [--symbol URI]
             [--to PATH:LINE:COL] [--depth N] [--limit N] [--offset N]
```

Source and destination use Task 4's semantic-first anchor resolution,
1-based positions, byte columns, and explicit syntax fallback/ambiguity.
Destination parsing splits the last two colons. Syntax fallback requires
unchanged source bytes; stale semantic anchors retain their snapshot proof
and receive `Stale` verification. Invalid bounds return exit 2, unresolved
or ambiguous anchors exit 3, and valid queries (including no paths) exit 0.

With `--to`, BFS returns only paths reaching that resolved endpoint. The
source equaling the destination yields a zero-hop path. Without `--to`,
it enumerates outbound paths ending at observed leaves, cycles, unknown
targets, or the depth boundary. A leaf source has no outbound paths.
Every response says `disclaimer: "plausible-path-not-execution"`.

Schema version 1 reuses Task 4 edge/evidence serialization. It adds
`destination`, `paths`, validated owner `bridges`, `visits`, `budgets`,
cycle and unresolved reasons. Every hop retains raw source spans, terminal
and reference spans, digest, provider, import/artifact/revision binding,
precision, and verification. `Exact` does not mean `Current`. Unknown
endpoints stay null; a potential call never expands to a same-name symbol.

Full normalized JSON examples are checked by `task5_trace_full_json_goldens`:

- [SCIP destination path](../crates/cli/tests/golden/m6-trace-scip.json):
  f0 → f1 → f2, two `CALLS` / `Exact` / `Unverified` hops.
- [Syntax enumeration](../crates/cli/tests/golden/m6-trace-syntax.json):
  `syntax-fallback` / `Heuristic`, one `POTENTIAL_CALL` with null callee,
  explicit anchor uncertainty and unresolved target.

Goldens normalize only snapshot nonce and artifact identity. The real CLI
smoke independently builds the same minimal fixture and imports SCIP
without rebuilding the index; the snapshot nonce stays unchanged.

## Ordering, budgets, and isolation

Default depth is 3; accepted depths are 1–6. At most 500 distinct full
endpoint identities are admitted. Per-path lineage detects cycles while
preserving both routes through a diamond. FIFO expansion and queued
terminal results preserve breadth-first ordering, including leaves and
cycles at equal depth. Sibling selection reuses Task 4's total SQL order.

Additional explicit caps are 3000 examined edges, 1000 emitted paths, and
500 recorded owner mappings. These bound fanout and repeated routes even
when the distinct-node count is small. Mapping lookup caps imports at 500,
scoped definitions at 501 (overflow detection), and distinct owner identity
candidates at two. Reasons include `depth-budget`, `node-budget`,
`edge-budget`, `path-budget`, `mapping-budget`, and `time-budget`.
Already-proven queued destination results survive later branch cutoffs.

The 2000 ms deadline starts at query entry and is checked cooperatively
between traversal operations and mapping reads. It does **not** preempt
an individual SQLite query or filesystem read. Freshness preparation and
evidence serialization may also extend total command time. This is not a
hard 2000 ms process-duration guarantee. Time-cut prefixes can vary with
machine load; they explicitly report truncation. Completed traversals and
deterministic work-budget cutoffs have stable ordering and JSON, without
elapsed timing fields.

Pagination defaults to 50, maximum 1000, over the collected path sequence.
`total` counts collected paths; `total_exact` is false when traversal was
cut short. `next_offset` continues only within collected results, not a
saved search frontier. Consumers must compare snapshot nonces between
pages. Pagination itself can set `truncated` while `total_exact` remains
true. Exhaustive failure to reach a target means `target-unreachable` in
the observed graph; uncertain or bounded failure means `target-not-proven`.
Neither is a claim about runtime reachability or global code coverage.

One SQLite read transaction covers both anchors, traversal, mappings,
coverage and evidence, with a shared request-local `import_verdict()` memo.
No cross-request cache was introduced. SCIP identities include import and
local-document scope. Syntax/SCIP bridging requires the existing callable
owner proof, file identity, digest, source span, name, and unique identity;
name equality alone never joins nodes. Multiple import mappings remain
explicitly ambiguous. Stale snapshot mappings retain old digests and never
claim current source identity. Digest-changing promotion removes the old
semantic binding. Ephemeral LSP identities remain outside persisted edges.

## Regression evidence

The Task 4 baseline had 285 tests. This checkpoint has 300: 196 unit,
16 CLI, 40 edge integration, 21 index, 15 semantic, and 12 syntax tests.
All pass with zero ignored tests. The 15 added tests cover:

| Evidence | Tests |
| --- | --- |
| Destination, outbound chain, per-hop producer proof | `task5_trace_destination_and_outbound_chain` |
| Cycle, unreachable destination, depth truncation | `task5_trace_cycle_depth_and_unreachable_are_explicit` |
| Diamond routes, disjoint pagination, repeated results | `task5_trace_diamond_pagination_and_repeatability` |
| Syntax-only mode and no name-spliced destination | `task5_trace_syntax_unknown_target_is_not_a_path_to_another_name` |
| Validated owner mapping and stale source label | `task5_trace_verified_syntax_bridge_and_stale_label` |
| Bounds and malformed destination exit codes | `task5_trace_rejects_invalid_depth_and_destination` |
| Destination fallback, exact max-depth reach, over-depth failure | `task5_trace_destination_fallback_and_max_depth_boundary` |
| Ambiguous cross-import mapping cannot join | `task5_trace_ambiguous_import_mapping_does_not_join` |
| Identical local symbols stay document-scoped | `task5_trace_local_cycles_stay_in_document` |
| Entire syntax/SCIP wire responses | `task5_trace_full_json_goldens` |
| Injected node/edge/path bounds and expired deadline | `trace_node_edge_path_and_time_budgets_are_explicit` |
| Internal bounds cannot exceed public contract | `trace_rejects_internal_budget_above_contract` |
| Leaf/cycle BFS tie ordering | `trace_bfs_orders_leaf_and_cycle_paths_at_same_depth` |
| Found destination survives later frontier cutoff | `trace_retains_completed_target_when_later_branch_exceeds_budget` |
| Actual trace query concurrent with digest-changing promotion | `task5_trace_pins_paths_and_digests_across_concurrent_promotion` |

Initial RED: five CLI functional cases failed because `trace` was unknown
(exit 2); the invalid-argument case already passed. Subsequent RED/GREEN
reproduced acceptance of an internal 501-node budget and loss of a completed
target when a later branch exhausted its budget. Both now pass. The other
new tests provide additional coverage; no historical RED is claimed for them.

The concurrency test pauses the actual read transaction after pinning its
snapshot, then runs a real writer thread that edits and promotes the index.
The pinned query returns the old semantic anchor, target text, digest, and
`Stale` evidence; the next query sees a new nonce, digest and target text,
with syntax fallback rather than reusing the old semantic identity.
All inherited Task 3/4 lifecycle, same-name, repository/import/document
isolation, source encoding, proof, LSP, and freshness regressions still pass.

Independent review identified an unbounded all-import-facts read during
mapping. It was replaced with scoped definition-only reads, bounded owner
identity selection, and scoped file lookup. Follow-up review found the
original issue resolved. Shared Task 4 coverage assembly was extracted for
reuse; its four existing goldens remain unchanged.

## Final local validation

All commands below returned exit 0 on Windows:

```text
cargo fmt --all -- --check
cargo check --locked
cargo clippy --all-targets -- -D warnings
cargo test --locked
cargo build --locked -p riltra
```

Real CLI smoke made 14 invocations: init/index/import; syntax enumeration
and unsuccessful destination; SCIP destination and repeat; depth-2
enumeration; depth cutoff; empty pagination; syntax-owner bridge; invalid
depth; missing anchor; stale source. Successful trace commands took
approximately 15–23 ms on the tiny fixture; this is not a large-repository
performance claim. Each hop's terminal bytes and producer provenance were
checked, along with exact repeated JSON and expected exit codes 0/2/3.

Scratch RED/GREEN logs, final gate logs and raw smoke responses are under
gitignored `target/m6-task5/`. Tracked goldens and this report are the portable
audit evidence. Linux/macOS CI and a hard wall-clock cancellation guarantee
were not established by this local run. Stop here for Task 5 audit.
