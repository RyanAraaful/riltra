# Impact evidence: current contract and presentation design

**Date:** 2026-10-31
**Status:** design note; no implementation or contract change proposed here

## Purpose

Explain what Riltra's existing impact result says, what evidence supports it, and why a result can remain `UNKNOWN`. Keep three questions separate:

1. What paths did the bounded traversal observe?
2. What limits or gaps affect those paths?
3. What does the repository-wide coverage inventory say?

This note is based on the current implementation and the recorded What → Impact → Why and Rust SCIP feasibility work. It does not propose new JSON fields, verdicts, tool names, or a stronger trust level.

## Existing contract

`impact` walks inbound edges from an anchor. Each emitted path puts the edge nearest the anchor first; it is evidence of a plausible static dependency path, not execution. The response includes `verdict`, `paths`, per-edge `kind`, `precision`, endpoints, `verification`, unresolved reasons, traversal budgets, freshness, completeness, and the disclaimer `plausible-impact-not-execution`.

The verdict is conservative:

| Verdict | Current meaning |
|---|---|
| `HIGH` | At least one complete observed path contains a `CALLS` edge, and all traversed edges have `Exact` precision, both endpoints, and no unresolved reason. No query-level uncertainty or traversal truncation is present. This is observed call evidence, not an estimate of severity, probability, safety, or runtime execution. |
| `MEDIUM` | Same completeness and endpoint requirements, but observed paths contain `REFERENCES` and no `CALLS`. It indicates an exact reference dependency, not a call chain. |
| `UNKNOWN` | Evidence is absent, heuristic, unresolved, stale, uncertain, paged, truncated, or otherwise insufficient. An empty result is not proof of no impact. |

The positive labels are about evidence strength in the returned bounded graph. They never mean that the graph is exhaustive or the code is safe. The source describes them as observed static evidence, never safety (`crates/cli/src/edges.rs`, `impact_verdict`).

## 1. Evidence paths

The returned `paths[]` is the path-level evidence. Readers should inspect its edges in order, from the anchor toward each observed dependent, and retain the labels and evidence with each edge.

- `CALLS` with `Exact` precision, resolved caller and callee, and no unresolved reason can support `HIGH` when the whole query also passes completeness and freshness checks.
- `REFERENCES` with the same conditions can support `MEDIUM` if no `CALLS` edge is present.
- Each path carries its own `verification`, `unresolved`, `cycle`, and `truncated` state. These are part of the evidence, not presentation noise.
- `POTENTIAL_CALL` / `Heuristic`, an absent endpoint, or an unresolved relation is not promoted into a call path. Such a row may still be useful for investigation, but it does not support a positive impact verdict.

### `Exact` and `Unverified` are independent

`precision` describes the relation's source evidence; `verification` describes its trust/freshness state. A SCIP-backed `CALLS` edge can be `Exact` because the callee endpoint is proven inside that imported index while remaining `Unverified` because the producer is not independently attested. A fresh checkout does not upgrade that import to `Current`; `Exact` does not imply `Verified` or runtime truth.

This distinction is exercised by `crates/cli/src/edges.rs` tests: an imported edge remains `Unverified` even when its precision is `Exact`. The same labels flow through the trace/impact response formatter, which is shared by CLI and MCP (`crates/cli/src/response.rs`; `crates/cli/src/mcp.rs`).

## 2. Limitation paths

The limitations that affect a path are visible in path-level unresolved reasons and traversal state. Examples include `unresolved-owner`, `unproven-relation`, `semantic-midpoint-unavailable`, ambiguous mapping, and budget cutoffs. A path containing one of these can be displayed as a lead to inspect, but cannot be presented as a complete supported path.

Other query-level gates can make the verdict `UNKNOWN` even where individual paths look exact:

- The anchor is syntax-only/uncertain, source freshness is not fresh, or an import is stale.
- Traversal is truncated by depth, node, edge, path, time, or mapping limits.
- Results are paged (`offset > 0` or more paths exist beyond `limit`). A page is not the whole affected set.
- Any snapshot coverage denominator other than `observed_edges` is non-zero, or coverage has unresolved reasons.
- The traversal has no path. Empty output is explicitly marked `empty-result-not-coverage-proof`.

So a visible path and a query verdict answer different questions: the path says what was observed; the verdict says whether the current policy permits a positive evidence label after applying query-wide limits.

## 3. Repository coverage

`completeness.denominator` is a set of diagnostic counts, not a percentage and not a count of omitted impact paths. Counters can overlap. Current categories include unsupported files, dynamic and ambiguous calls, unresolved owners and targets, missing provider files, parse errors, skipped files, stale imports, and truncated branches. `observed_edges` is the traversal's local work count; it must not be confused with a repository-size denominator.

The current implementation gates a positive impact verdict on snapshot-wide completeness reasons and non-zero coverage counters (excluding `observed_edges`). That is stricter than checking only the returned path. It is why a precise local path can coexist with `UNKNOWN` for a repository whose graph inventory is incomplete.

Recorded examples show the distinction:

- The What → Impact → Why frozen snapshot had zero traversed edges and partial coverage; it correctly returned `UNKNOWN` rather than treating an empty affected set as safety ([implementation evidence](riltra-what-impact-why-implementation.md#impact-contract-and-provider-result)).
- The Rust SCIP feasibility snapshot returned eight of nine non-closure production call sites as `CALLS` and all 16 oracle expressions, yet impact remained `UNKNOWN`: coverage was partial, seven snapshot-wide counters were non-zero, traversal stopped at the two-second budget after 21 paths, and an unresolved owner remained ([feasibility findings](riltra-rust-semantic-feasibility-2026-10-31.md#4-impact), [coverage gate analysis](riltra-rust-semantic-feasibility-2026-10-31.md#what-option-1-does-not-unlock)).
- The focused Rust regression fixture `rs_impact_keeps_scip_path_when_another_caller_lacks_provider_coverage` imports a target and one caller from SCIP while leaving a second caller file without provider facts. Impact still displays the observed direct `CALLS` path with `Exact` precision and source spans, digest, and import provenance; the snapshot reports one `missing_provider_files` and verdict `UNKNOWN`. This demonstrates that the returned path is observed evidence while repository coverage can remain incomplete. The uncovered caller itself is not claimed as a path, and the fixture does not prove producer correctness or runtime behavior.

This is a bounded result for the measured repository and snapshot, not a general claim about all Rust projects. The recorded denominator and path counts should travel with any summary.

## CLI and MCP use without a contract change

Keep the existing entry points and response shape:

- CLI: `riltra impact --repo <name> --path <rel> --line <n> --col <n> [--symbol <uri>] [--depth <n>] [--limit <n>] [--offset <n>]`.
- MCP: `riltra_code_impact` with the corresponding anchor and traversal inputs.
- Both routes use the same query and response formatter; MCP is documented to preserve CLI freshness, provenance, and `POTENTIAL_CALL`/`CALLS` semantics ([README MCP contract](../README.md#mcp-server-twelve-read-only-tools-one-provider-backed-tool)).

For a human-facing explanation made from the current response, use this order:

1. State the verdict and its narrow meaning from the table above.
2. Show observed `paths[]` and preserve edge kinds, precision, endpoints, per-edge verification, and unresolved reasons.
3. State `freshness`, `completeness.status`, `completeness.unresolved`, denominator counts, `truncated`, and pagination limits.
4. Say explicitly that `UNKNOWN` or an empty path list does not mean “unaffected”; a positive label does not prove runtime behavior or safety.

This is documentation and client interpretation only. No schema key, CLI option, MCP tool, status enum, golden, or trust rule changes. If a future proposal scopes completeness to a language or traversed subgraph, that would change which positive verdicts are reachable and therefore requires a separate owner decision and contract amendment; it is outside this note.

## Source of truth

- Verdict gate and reverse traversal: [`crates/cli/src/edges.rs`](../crates/cli/src/edges.rs), `traverse_impact` and `impact_verdict`.
- Snapshot coverage and query uncertainty gate: [`crates/cli/src/index.rs`](../crates/cli/src/index.rs), `EdgeRead::coverage` and `trace_query_inner`.
- Stable JSON shape and disclaimer: [`crates/cli/src/response.rs`](../crates/cli/src/response.rs), `trace_query_json`.
- CLI and MCP parity: [`crates/cli/src/main.rs`](../crates/cli/src/main.rs), `cmd_graph`; [`crates/cli/src/mcp.rs`](../crates/cli/src/mcp.rs), graph tools.
- Earlier impact contract and current Rust evidence: [`riltra-what-impact-why-implementation.md`](riltra-what-impact-why-implementation.md); [`riltra-rust-semantic-feasibility-2026-10-31.md`](riltra-rust-semantic-feasibility-2026-10-31.md).
