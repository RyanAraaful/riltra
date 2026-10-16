# M6 Relationship, Trace & Impact Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add source-backed callers/callees, bounded static trace, and defensible impact analysis on the M2–M4 syntax + M5 semantic foundation.

**Architecture:** Syntax builds derive `IMPORTS` + syntax-only `POTENTIAL_CALL` edges; the existing SCIP-import transaction additionally upserts semantic-backed edges (`CALLS` where proven, else `REFERENCES` linkage) without reparsing syntax; full/incremental rebuilds re-derive the graph consistently after semantic carry-over and `import_id` rebinding. Queries are bounded BFS over one read-transaction snapshot. `CALLS` (Exact) requires terminal-callee-identifier overlap with the *matching* SCIP reference occurrence plus same-`import_id` target resolution; everything else stays `POTENTIAL_CALL`/`REFERENCES`.

**Tech Stack:** Rust (std-only CLI, rusqlite bundled, existing tree-sitter grammars, existing protobuf SCIP reader). No new dependencies, no graph engine, no daemon, no embeddings.

**Spec:** User request 2026-10-16 (M6 goals, 8 design questions, Tier A/B acceptance, forbidden scope). Baseline `master` HEAD `2f57cee`. M0–M5 CLOSED — no architecture reopen.

## Global Constraints

- Correctness → Security → DRY/KISS → Performance, in that order.
- Apache-2.0; local-first; no telemetry; explicit provider opt-in; no TK-internal/GitNexus code.
- Commit author `ryanaraaful <terapetralabs@gmail.com>`; no push without approval; no force-push; MMSP read-only, never in CI.
- Single SQLite per repo; schema migration via `store::migrate` one explicit txn; atomic staging + transactional copy-promote; snapshot rowids never permanent.
- Freshness: `fresh`/`stale`/`unknown` with named reasons, never timestamp-only; semantic verification max `Unverified` (never `Current`).
- All source reads via `semantic::confined_read` + `scan::MAX_FILE_BYTES` (1 MB); canonical containment; no new trust-boundary I/O.
- CLI stays std-only (no clap/serde); JSON keys stable; exit codes 0/2/3.
- No M7/M8 scope: no MCP, knowledge graph, cross-repo traversal, UI, embeddings, daemon, runtime tracing, new deps without proven need.

## Design Decisions (locked, not brainstorming)

1. **Edge taxonomy (evidence rule):**
   - `IMPORTS` = syntax `imports` rows verbatim, provider `tree-sitter`. No inference.
   - `REFERENCES` = M5 semantic facts with `roles.reference` (`!definition`), keyed by `(import_id, symbol_uri)` (+ document path for `local` symbols). Both providers. Never derived from syntax calls.
   - `CALLS` (strict, `Exact` only, SCIP path) requires ALL of: (a) syntax call yields a terminal callee identifier/property sub-span split from receiver and arguments (`w.process(x)` → receiver `w`, terminal `process`; `Scope::name()` → terminal `name`); (b) one SCIP `reference`-role occurrence of the *matching target symbol* overlaps that terminal sub-span by byte range in the same file/snapshot; (c) the target definition fact with the same `symbol_uri` exists under the SAME `import_id` (locals additionally confined to the same document path); (d) the import row passes the provenance gate (non-ephemeral provider + non-empty producer artifact + `references` capability + concrete text encoding — `provider_name` alone is never the boundary, since it carries the real tool name). Object/attribute-base references, alias bindings, and ambiguous multi-candidate dispatches never satisfy (c) and fall to `POTENTIAL_CALL`. SCIP alone never proves a call; name equality alone never proves a call; LSP evidence never proves `CALLS` (max `POTENTIAL_CALL`).
   - `POTENTIAL_CALL` (`Heuristic`) = syntax call with a proven callable owner but no proven `CALLS`: `ambiguous:N`, `alias`, `dynamic`, `external`, `no-overlap`, `target-mismatch`, `owner-unproven`, `stale-import`. Always labeled with `unresolved_reason`.
2. **Owner/callee precision (minimum change):** current `calls` span covers the whole call expression (`syntax.rs:298-311`) — unusable for overlap joins. Minimum fix: `push_call` records TWO sub-spans from the `function` field: the full callee expression range AND the terminal identifier/property range (receiver vs terminal split per language: `.`/`::`/`->` separators; `None` when the grammar yields no separable terminal). Owner (caller) is derived at edge-build time: narrowest enclosing `symbols` definition of callable kind (`function|method|class-associated fn`; class bodies, anonymous lambdas/arrows without a named callable owner do NOT yield definite caller relations — such calls get `owner-unproven` `POTENTIAL_CALL` with an `owner-fallback` count, never a fabricated caller). Owner→Scip bridging additionally requires file-identity evidence for the owner definition fact (fact `file_id` match when present, else digest match against the files row) in the same file/import — same display name alone never bridges. No name-guessing, no cross-provider identity (`symbol_uri + import_id` only, M5 rule).
3. **Storage + lifecycle (minimum):** reuse SQLite; ONE table `code_edges` in schema v4 (no graph engine). v4 = `ALTER TABLE calls ADD COLUMN` callee_*/tcallee_* (guarded, only when missing) + `CREATE TABLE code_edges` + inbound/outbound indexes (see Interfaces). Lifecycle: (a) syntax build derives `IMPORTS` + syntax-only `POTENTIAL_CALL` edges; (b) the existing `scip::import_file` transaction upserts semantic-backed edges for the imported documents only (no syntax reparse; failure rolls back edges with the import); (c) full/incremental rebuild re-derives the whole graph AFTER semantic carry-over and `import_id` rebinding, so remapped `file_id`/`import_id` never dangle. Edge-copy rule: edges are NEVER carried or copied between snapshots — staging always re-derives them from its own facts (safe recomputation default; no selective-reuse claim). Caller/target endpoints that are unknown are stored as `None` keys with callsite/import location evidence — never a fabricated `EdgeKey` — and BFS never expands them. `SyntaxDef`↔`Scip` unification happens ONLY through validated evidence (same file + def byte-range containment + same snapshot + same `import_id` + same `symbol_uri`); an A→B→C chain links through B only when B's key unifies on both sides, otherwise traversal halts with `unresolved` (never name-spliced). LSP boundary: M5 LSP is ephemeral/location-only — no graph relationship is ever stored from LSP evidence and no SCIP identity is fabricated for it (upsert denylists the ephemeral provider; stored rows are SCIP-provenance by construction). Rejected: separate graph DB, re-parse at query time, petgraph (BFS ≤500 nodes needs none).
4. **CLI contract:** `riltra callers|callees --repo R --path P --line L --col C [--symbol URI] [--limit N] [--offset N]` and `riltra trace --repo R --path P --line L --col C --to PATH:LINE:COL [--depth N] [--limit N]` (destination-anchored; without `--to`, bounded path enumeration from the anchor). Anchor resolution: semantic anchor first (narrowest half-open occurrence → `symbol_uri`, `local` stays in document, `import_id` scope); syntax-only fallback when no semantic anchor (nearest enclosing syntax def by byte containment, returned with `"anchor_kind":"syntax-fallback"` + `"precision":"Heuristic"` + explicit `anchor_uncertainty` reason — never presented as resolved identity). Every answer carries `repo, snapshot{id,nonce}, anchor{kind,...}, verification, completeness{status,covered,total,denominator,unresolved[]}, edges[]` with per-edge `evidence{source path/span, provider/import identity}`. Pagination mirrors `search`: default 50, max 1000, deterministic ordering (caller path, caller byte_start, callee key) so `limit/offset` pages are stable. `trace` returns `paths[]` (each a list of edge keys) with `truncated`/`cycle` flags + `plausible-path-not-execution` disclaimer. Impact verdict (early M6): `UNKNOWN` whenever impact is unprovable (see Decision 6); `HIGH`/`MEDIUM` only on observed relations; `LOW`/`SAFE` are NOT returned in M6 (no completeness proof exists yet — the clean-leaf case is tested as `UNKNOWN`, not `LOW`).
5. **Traversal safety:** BFS over `code_edges` within one read txn; budgets default `depth 3 / max 6`, `nodes 500`, `time 2000 ms`; visited-set on full edge keys (SCIP: `(import_id, symbol_uri)`; syntax-only: `(snapshot_id, caller_path, caller_bs, caller_be)` / callee def key) — never on bare names; recursion/cycle reported as `cycle` edge, not infinite loop; ambiguous fan-out capped by `limit` with `truncated:true`; missing targets listed in `unresolved[]`.
6. **Freshness/isolation + completeness denominator:** all four commands run under the M5 single-read-txn pattern (`latest_snapshot` + `evaluate_snapshot` + facts + `import_verdict` memo) and downgrade to `Stale` on `!source_fresh || drifted`. `Unverified` is NOT `Current`: absence of edges is never evidence of no impact. `completeness.denominator` = observed edge closures + explicitly counted unknowns: `unsupported_files + missing_provider_files + dynamic_calls + ambiguous_calls + truncated_branches + unresolved_owner + unresolved_target + stale_imports`. `status:"complete"` requires denominator == observed AND fresh AND zero truncation; anything else is `"partial"`. No cross-request cache (per-query memo only). No `Current` elevation.
7. **Perf:** report per query `visits{nodes,edges}`, `elapsed_ms`, `rss_kb` (process peak, best-effort), `output_bytes`. No speedup claims without benchmark. Small fixtures first, then one sanctioned pilot repo.

## Review Focus

- A Python `from x import (a,\n b)` multiline import still pins `end_line` to start line (syntax.rs:243-249) — import-closure resolution must not depend on import end position, only on parsed module binding.
- Tree-sitter columns are byte-columns (break on multibyte) — callee overlap joins must use byte offsets first, line/col only for display.
- Anonymous/arrow/lambda bodies are not callable owners — calls inside them get `owner-unproven` POTENTIAL_CALL (counted), never a fabricated named caller and never a definite CALLS edge.
- `require()` in JS is a call, not an import — must not enter import closure.
- C `Scope::name` qualified names kept verbatim — terminal split keeps the full name in evidence; resolution uses the terminal only as a candidate filter, never as proof.

---

## File Map

- Modify `crates/cli/src/syntax.rs` — `Fact` gains `callee: Option<CalleeSpan>` + `tcallee: Option<CalleeSpan>` (terminal identifier/property split from receiver); `push_call` records both from the `function` field.
- Modify `crates/cli/src/store.rs` — schema v4: `calls` +12 nullable cols (callee_* + tcallee_*), `code_edges` table + inbound/outbound indexes, guarded `migrate` (v1→v4 chain, no double-ALTER on fresh DBs), `promote_staging`, conditional edge carry (no blind caller-file copy), `CallRow` + insert/select, `EdgeRow`/`EdgeKey` + insert/query helpers.
- Modify `crates/cli/src/index.rs` — syntax pipelines derive `IMPORTS` + syntax-only `POTENTIAL_CALL`; `scip::import_file` path upserts semantic-backed edges in-transaction; full/incremental rebuild re-derives after semantic carry-over + `import_id` rebinding; four query functions with single-txn + memo reuse.
- Modify `crates/cli/src/args.rs`, `main.rs` — four subcommands, JSON contracts, bounded `--depth/--limit/--offset`.
- Create `crates/cli/src/edges.rs` — taxonomy, owner resolution, import-closure, BFS traversal, impact verdict. (One new module; traversal logic must not leak into `index.rs`.)
- Tests: extend `crates/cli/tests/` golden corpus (new `tests/edges.rs` + fixtures under `tests/fixtures/m6/` for py/ts/rs/c); Tier B pilot script (manual, not CI).
- Docs: `docs/m6-validation.md` (new), `docs/m5-validation.md` untouched.

## Interfaces

- `syntax.rs`: `pub struct CalleeSpan { start_line: u32, start_col: u32, end_line: u32, end_col: u32, byte_start: usize, byte_end: usize }`; `Fact.callee: Option<CalleeSpan>` (full callee expression) + `Fact.tcallee: Option<CalleeSpan>` (terminal identifier/property).
- `store.rs`: `pub enum EdgeEndpoint { Scip { import_id: i64, symbol_uri: String, doc: Option<String> }, SyntaxDef { path: String, byte_start: usize, byte_end: usize } }` (`doc` set for `local` symbols; NO `Unknown` variant — unknown endpoints are `None`); `pub struct EdgeRow { snapshot_id: i64, file_id: i64, caller: Option<EdgeEndpoint>, caller_path: String, caller_span_bytes: (usize, usize), callee: Option<EdgeEndpoint>, callee_name: String, edge_kind: String, precision: String, provider: String, evidence: String, unresolved: Option<String> }`; `pub fn insert_edge(conn, &EdgeRow)`, `pub fn edges_outbound(conn, snapshot_id, &EdgeKey, limit, offset)` (callees/dowhile direction), `pub fn edges_inbound(...)` (callers direction); `SCHEMA_VERSION = 4`. Indexes match the two query patterns: `idx_edges_out(snapshot_id, caller_key_cols)` + `idx_edges_in(snapshot_id, callee_key_cols)` (concrete columns mirror the EdgeKey encoding, never bare names).
- `edges.rs`: `pub enum EdgeKind { Calls, PotentialCall, References, Imports }`; `pub fn derive_syntax_edges(snapshot_id, symbols, calls, imports) -> Vec<EdgeRow>` (IMPORTS + syntax-only POTENTIAL_CALL; Task 2 does NOT create this module); `pub fn upsert_semantic_edges(conn, snapshot_id, import_id) -> Result<UpsertStats, String>` (CALLS where proven per Decision 1, else REFERENCES linkage; runs INSIDE the existing `scip::import_file` transaction; provenance-gated (non-ephemeral provider + artifact + capability + encoding) — stored rows are SCIP-provenance by construction, and the row's `provider_name` is the real tool name by upsert time so an allowlist would misfire); `pub fn rederive_after_carry(conn, snapshot_id) -> Result<RebuildStats, String>` (post-carry full re-derivation; edges never copied); `pub fn unify(a: &EdgeEndpoint, b: &EdgeEndpoint) -> bool` (SyntaxDef↔Scip validated-evidence equality; anything un-unitable stays unlinked); `pub fn traverse(conn, snapshot_id, start: &EdgeEndpoint, dir: Dir, budget: &Budget, to: Option<&EdgeEndpoint>) -> Traversal` (Task 5; halts — never name-splices — on un-unitable midpoints); `pub fn impact_verdict(traversal, completeness) -> ImpactLevel` (`High|Medium|Unknown` in M6 — no `Low`/`Safe` returned; clean leaf with no observed impact evidence yields `Unknown`).
- `index.rs`: `pub fn edge_callers(home, name, root, anchor: &SpanQuery, limit, offset)`, `edge_callees(...)`, `edge_trace(..., depth, ...)`, `edge_impact(...)` — all `-> Result<EdgeQueryOut, String>` under one read txn.

---

### Task 1: Callee + terminal-identifier sub-spans in syntax

**Files:**
- Modify: `crates/cli/src/syntax.rs:298-311` (`push_call`), `crates/cli/src/syntax.rs:91-101` (`Fact`)
- Test: `crates/cli/src/syntax.rs` unit tests

**Interfaces:**
- Consumes: tree-sitter `node.child_by_field_name("function")` node range + separator-aware terminal split.
- Produces: `Fact.callee: Option<CalleeSpan>` + `Fact.tcallee: Option<CalleeSpan>` for `edges.rs` overlap joins.

- [ ] **Step 1: Write failing test** — `foo(a, b)` yields `callee` covering `foo` and `tcallee` covering `foo`; `w.process(x)` yields `callee` covering `w.process`, `tcallee` covering `process` only; `Scope::name()` yields `tcallee` covering `name` with full name preserved in evidence.
- [ ] **Step 2: Run to verify it fails** — Run: `cargo test --locked callee_span` Expected: FAIL (`tcallee` field missing).
- [ ] **Step 3: Implement `CalleeSpan` ×2 + `push_call` function-field ranges** in `syntax.rs` (whole-call `span/bytes` unchanged for parity; both additive `Option`, `None` when grammar yields no separable terminal).
- [ ] **Step 4: Re-run** — Run: `cargo test --locked` Expected: PASS, existing goldens byte-identical except new fields.
- [ ] **Step 5: Commit** — `git add crates/cli/src/syntax.rs && git commit -m "m6: record callee and terminal-identifier sub-spans in syntax calls"`

### Task 2: Schema v4 + code_edges storage (store only — no edges.rs yet)

**Files:**
- Modify: `crates/cli/src/store.rs:12` (version), `:94-165` (DDL), `:699-742` (promote list only), `CallRow` + insert/select/copy column lists; `crates/cli/src/index.rs` (`ingest_file` writes callee/tcallee cols)
- Test: `crates/cli/src/store.rs` migrate/promote tests

**Interfaces:**
- Consumes: Task 1 spans.
- Produces: v4 `migrate`, `code_edges` table + `EdgeRow`/`EdgeEndpoint` + `insert_edge` + `edges_outbound/edges_inbound` helpers. (Derivation, import upsert, and rebuild hooks are Task 3 — this commit stays buildable with edges written only by tests.)

- [ ] **Step 1: Write failing tests** — (a) `migrate` v1→v4, v2→v4, v3→v4 keeps data + creates `code_edges`; fresh DB builds at v4 with no double-ALTER error; (b) `promote_staging` carries a hand-inserted edge row; (c) `CallRow` callee/tcallee round-trip incl. NULL legacy rows.
- [ ] **Step 2: Run to verify they fail** — Run: `cargo test --locked store_m6_lifecycle` Expected: FAIL (no `code_edges`).
- [ ] **Step 3: Implement DDL + storage** — guarded `ALTER TABLE calls ADD COLUMN` callee_*/tcallee_* (only when missing, so fresh-DB `CREATE` + migrated-DB `ALTER` never collide) + `CREATE TABLE code_edges` + `idx_edges_out/idx_edges_in` matching the two traversal patterns; bump `SCHEMA_VERSION=4` (migrate sets `user_version = 4`); extend `promote_staging` table list; extend `copy_mapped_facts` calls column list; `ingest_file` populates the new cols from `Fact`. No edge derivation here; NULL callee/tcallee rows persist as-is (re-extraction is Task 3's re-derivation).
- [ ] **Step 4: Re-run** — Run: `cargo test --locked` Expected: PASS, v1→v4 chain green.
- [ ] **Step 5: Commit** — `git add crates/cli/src/store.rs crates/cli/src/index.rs && git commit -m "m6: schema v4 code_edges storage with guarded migrate"`

### Task 3: Edge derivation + lifecycle hooks (new module; every commit buildable)

**Files:**
- Create: `crates/cli/src/edges.rs`
- Modify: `crates/cli/src/main.rs` (`mod edges`), `crates/cli/src/scip.rs` (upsert hook inside `import_file` txn), `crates/cli/src/index.rs` (syntax-pipeline `derive_syntax_edges`; rebuild `rederive_after_carry`)
- Test: unit tests in `edges.rs` + `crates/cli/tests/edges.rs` lifecycle tests

**Interfaces:**
- Consumes: Tasks 1–2 (`symbols`, `calls`+callee/tcallee, `imports`, `semantic_facts` pairs).
- Produces: `derive_syntax_edges` / `upsert_semantic_edges` / `rederive_after_carry` used by Task 4 queries.

Rules (locked): owner = narrowest enclosing CALLABLE `symbols` def (`function|method`; class bodies and anonymous lambda/arrow bodies never own definite caller edges → `owner-unproven` POTENTIAL_CALL); import closure = per-file parsed import text → module binding (text-match only); `CALLS` requires terminal-sub-span overlap with the matching target occurrence + same-`import_id` target definition (locals: same document); everything else (`ambiguous:N`, `alias`, `dynamic`, `external`, `no-overlap`, `target-mismatch`, `owner-unproven`, `stale-import`) is `POTENTIAL_CALL` with reason. A→B→C links through B only if `unify` proves both sides; else halt with `unresolved`. LSP rows never produce stored edges.

- [ ] **Step 1: Write failing tests** — direct call → CALLS; ref-without-call → REFERENCES only; member call `w.process(x)` with only an object-base reference → POTENTIAL_CALL `target-mismatch` (never CALLS); same-name two defs → POTENTIAL_CALL `ambiguous:2`; alias → POTENTIAL_CALL `alias`; recursion → CALLS self-edge; call in class body / bare lambda → POTENTIAL_CALL `owner-unproven`; missing def → POTENTIAL_CALL `unresolved`; A→B→C cross-file links only when B unifies, else `unresolved` halt (no name splice); ephemeral `lsp-subprocess` provider row yields zero edges (LSP boundary); forged import metadata (SCIP-sounding provider, empty artifact, non-SCIP encoding/capability) yields zero edges (provenance gate); file-unbound owner facts (stale digest, foreign file_id, ghost document) fall back to snapshot-local caller without losing the CALLS edge (bridging audit); index → SCIP import → stored CALLS without reindex; import failure rolls back edges; noop/delta/full parity; target-change re-derivation with unchanged caller; import-vs-promotion race aborts clean.
- [ ] **Step 2: Run to verify they fail** — Run: `cargo test --locked edges_derive` Expected: FAIL (module missing).
- [ ] **Step 3: Implement `edges.rs` derivation + hooks** (byte-offset overlap on the TERMINAL sub-span first; line/col display only; SCIP pair input = reference occurrence + definition fact with same `(symbol_uri, import_id)`; `upsert_semantic_edges` called from `scip::import_file` before it returns, inside the caller's transaction; rebuilds call `rederive_after_carry` after `carry_semantic`).
- [ ] **Step 4: Re-run + M0–M5 suite** — Run: `cargo test --locked` Expected: PASS, syntax/incremental parity intact.
- [ ] **Step 5: Commit** — `git add crates/cli/src/edges.rs crates/cli/src/main.rs crates/cli/src/scip.rs crates/cli/src/index.rs crates/cli/tests/edges.rs && git commit -m "m6: derive CALLS/POTENTIAL_CALL/REFERENCES/IMPORTS edges with strict proof"`

### Task 4: CLI callers/callees (+JSON contract)

**Files:**
- Modify: `crates/cli/src/args.rs:39-60`, `crates/cli/src/main.rs` (new arms + `edge_json`), `crates/cli/src/index.rs` (query fns)
- Test: `crates/cli/tests/edges.rs` (new CLI golden)

**Interfaces:**
- Consumes: Tasks 2–3, M5 `import_verdict` memo + single-txn pattern.
- Produces: `riltra callers|callees` JSON consumed by Tasks 5–6.

Contract: `{"repo","snapshot":{"id","nonce"},"anchor":{"kind":"semantic|syntax-fallback",...,"anchor_uncertainty":null|reason},"verification","completeness":{"status":"complete|partial","covered":n,"total":m,"denominator":{...counts},"unresolved":[reasons]},"edges":[{"kind","precision","caller":{...},"callee":{...},"evidence":{"path","span","provider","import":id}}],"truncated":bool,"timings_ms":{...},"limits":{"limit","offset"}}`. Ordering deterministic (caller path, caller byte_start, callee key) for stable pagination.

- [x] **Step 1: Write failing CLI tests** — extend inherited tests with full caller/callee goldens on the minimal Python corpus, semantic/syntax-fallback anchors, uncertainty, import/document/repository isolation, stale edits, pagination, source proof, encoding, and actual query/promotion concurrency. See `docs/m6-validation.md` for the assertion-to-test map.
- [x] **Step 2: Run to verify they fail** — inherited commands already existed; actual RED runs reproduced six initial contract/query gaps and subsequent proof/identity/coverage regressions. No historical unknown-command RED is claimed.
- [x] **Step 3: Implement args + query fns + JSON** — single read txn, semantic byte-span anchors, validated syntax-owner aliases, source-safe fallback, per-query verdict memo, full evidence, exact SQL counts, total ordering, default 50/max 1000, and continuation. Task 4 user decision: runtime timings remain outside JSON for byte determinism; `limits` also carries `next_offset`, evidence carries proven `imports[]`, and conservative denominator counters are documented in `docs/m6-validation.md`.
- [x] **Step 4: Re-run** — `cargo test --locked --test edges`: 30 passed; full locked suite: 285 passed. Format, check, and warning-denying Clippy pass locally; real CLI syntax/SCIP smoke verified.
- [x] **Step 5: Commit** — signed local checkpoint `m6: callers/callees CLI with evidence contract`, including query/store/proof helpers, tests, four goldens, and `docs/m6-validation.md`. Keep the branch local and stop for audit; no push or Task 5 work.

### Task 5: Bounded static trace (`--to` destination or bounded enumeration)

**Files:**
- Modify: `crates/cli/src/edges.rs` (`traverse`), `args.rs`/`main.rs`/`index.rs` (`trace` arm)
- Test: `crates/cli/tests/edges.rs` trace cases

Semantics (locked): `trace --to PATH:LINE:COL` = destination-anchored BFS (paths from source anchor to the resolved `--to` key, both anchors resolved with the same semantic-first/syntax-fallback rule); without `--to` = bounded enumeration of outbound paths from the anchor up to depth/node budgets. Both deterministic under the Task 4 ordering; every path carries per-edge evidence; `truncated`/`cycle` flags explicit.

- [x] **Step 1: Write failing tests** — destination, outbound chain, cycle/unreachable/depth, deterministic diamond pagination, syntax fallback, mapping isolation, max-depth, evidence goldens, budgets, and actual promotion concurrency. See `docs/m6-task5-validation.md`.
- [x] **Step 2: Run to verify they fail** — five initial functional CLI cases failed on unknown `trace`; subsequent RED/GREEN reproduced an excessive internal node budget and loss of a completed target after a later frontier cutoff. No RED is claimed for already-passing invalid-argument coverage.
- [x] **Step 3: Implement BFS** — depth default 3/max 6, 500 nodes, cooperative 2000 ms deadline, full scoped identities, per-path cycle detection, per-hop evidence, verified owner mappings, and explicit unresolved/truncated results. Additional work bounds: 3000 edges, 1000 paths, 500 mappings. Deadline and pagination limits are documented in `docs/m6-task5-validation.md`.
- [x] **Step 4: Re-run** — all 40 edge integration tests and all 300 locked tests pass. Formatting, check, warning-denying Clippy, build, real syntax/SCIP CLI smoke, and independent review pass locally.
- [x] **Step 5: Commit** — signed local checkpoint `m6: bounded static trace with cycle/depth/node/time guards`, including scoped store/proof helpers, tests, two goldens, and `docs/m6-task5-validation.md`. Stop for Task 5 audit; no push or Task 6–8 work.

### Task 6: Impact (UNKNOWN-first, never false-safe)

**Files:**
- Modify: `crates/cli/src/edges.rs` (`impact_verdict`), `index.rs`/`main.rs` (`impact` arm)
- Test: `crates/cli/tests/edges.rs` impact goldens (mandatory zero-false-safe corpus)

Rule (locked, early-M6): verdict ∈ {HIGH, MEDIUM, UNKNOWN}; `LOW`/`SAFE` are never returned (no completeness proof exists yet). `UNKNOWN` whenever the closure contains ANY of: POTENTIAL_CALL, unresolved owner/target, dynamic, ambiguous, truncated traversal, stale verification, unsupported-language files, or missing-provider files in scope — AND whenever no observed impact evidence exists at all (a clean leaf with zero observed relations yields `UNKNOWN`, never `MEDIUM`). `HIGH`/`MEDIUM` only on observed `CALLS`/`REFERENCES` relations with partial-but-informative coverage.

- [x] **Step 1: Write failing tests** — mandatory zero-false-safe corpus, proven direct/transitive paths, plain references with unknown owners, mapping/import/document isolation, exact injected coverage counts, budgets, pagination, full JSON goldens and actual promotion concurrency. Clean leaves explicitly UNKNOWN. See `docs/m6-task6-validation.md`.
- [x] **Step 2: Run to verify they fail** — five initial CLI cases failed on missing `impact`; subsequent RED/GREEN reproduced HIGH at an unbridged syntax midpoint and missing direct/transitive metadata.
- [x] **Step 3: Implement reverse-BFS + verdict** — shared Task 5 query transaction/evidence and bounded traversal primitives, UNKNOWN priority, no SAFE/LOW, exact counters for injected gaps. Impact completeness always partial with `graph-completeness-unproven`, even for an exhausted observed graph. Depth/node/time and extra work bounds retain Task 5 semantics; cooperative deadline limitation documented.
- [x] **Step 4: Re-run** — all 50 edge integration tests and all 312 locked tests pass; fmt/check/Clippy/build, real CLI smoke and independent review pass. Zero SAFE/LOW in the corpus; insufficient evidence cases UNKNOWN.
- [x] **Step 5: Commit** — signed local checkpoint `m6: impact analysis with UNKNOWN-first no-false-safe rule`, including args, tests, three goldens and `docs/m6-task6-validation.md`. Stop for Task 6 audit; no push or Task 7–8 execution.

### Task 7: Tier A golden corpus (4 languages × mandatory cases)

**Files:**
- Create: `crates/cli/tests/fixtures/m6/{py,ts,rs,c}/...`, extend `crates/cli/tests/edges.rs`
- Test: full matrix

Cases (each as a minimal fixture, direct + golden JSON): direct calls, ref-without-call, nested calls, local scope, same-name symbols, alias, recursion, cycles, missing definitions, dynamic dispatch, unresolved, stale-after-edit.

- [ ] **Step 1: Write all RED goldens** (one test per case per language where the construct exists; C macros-out-of-scope explicitly skipped with reason).
- [ ] **Step 2: Run** — Run: `cargo test --locked --test edges` Expected: FAIL on new cases.
- [ ] **Step 3: Fix derivation gaps only** (no taxonomy changes; per-case `unresolved_reason` tuning allowed).
- [ ] **Step 4: Re-run full suite** — Run: `cargo test --locked` Expected: PASS M0–M6.
- [ ] **Step 5: Commit** — `git add crates/cli/tests && git commit -m "m6: Tier A golden corpus py/ts/rs/c"`

### Task 8: Tier B pilot + perf ledger + close-out doc

**Files:**
- Create: `docs/m6-validation.md`
- Test: manual pilot (NOT committed fixtures of external repos)

Pilot: one TS fixture repo via scip-typescript 0.4.0 (import + `callers` spot-check: SCIP-backed CALLS present, LSP never claimed as resolved calls); one Rust fixture crate via `RILTRA_TEST_LSP` rust-analyzer (refs stay REFERENCES/POTENTIAL_CALL, never CALLS). Record `elapsed_ms, visits, rss_kb, output_bytes` warm.

- [ ] **Step 1: Run pilot commands, paste evidence into `docs/m6-validation.md`** (gates, Tier A counts, Tier B observations, limitations incl. Windows symlink note carry-over).
- [ ] **Step 2: Run gates** — Run: `cargo fmt --all -- --check && cargo check --locked && cargo clippy --all-targets -- -D warnings && cargo test --locked` Expected: all green.
- [ ] **Step 3: Commit** — `git add docs/m6-validation.md && git commit -m "m6: validation with Tier A/B evidence and perf ledger"`

## Acceptance Gates

- `cargo fmt/check/clippy/test` green locally + CI Windows/Ubuntu/macOS green.
- Tier A: full matrix green (py/ts/rs/c mandatory cases); impact corpus zero false-safe (`SAFE`/`LOW` never emitted; clean leaf asserts `UNKNOWN`, per the locked Task 6 rule).
- Tier B: TS SCIP CALLS demonstrated via index → import → callers with no reindex; Rust LSP refs never claimed as CALLS (paired, no contention).
- Lifecycle: import-failure rollback, noop/delta/full parity, target-change re-derivation, and import-vs-promotion race tests green.
- Migration: v1/v2/v3→v4 chains green; fresh-DB single-build green (no double-ALTER); NULL-callee re-extraction covered.
- Regression: M0–M5 suite green; syntax/incremental parity byte-identical modulo additive callee/tcallee cols; core-only useful without providers.
- No new deps; 8 tasks kept; file map kept; no M7/M8 scope.

## Risks & Rejected Alternatives

- **SCIP has no call bit; member calls mislead** — risk of over-claiming CALLS from object-base overlap. Mitigation: terminal-identifier overlap + same-`import_id` target-definition proof; rejected: whole-callee overlap, treating all references as calls.
- **Name-based resolution lies** (aliases, methods, `::`). Mitigation: POTENTIAL_CALL default with reasons; rejected: global name→def map.
- **Stale graph after SCIP import** — risk of missing CALLS until reindex. Mitigation: in-transaction `upsert_semantic_edges` + post-carry re-derivation; rejected: syntax-build-only derivation.
- **Blind edge copy across rebuilds** — risk of dangling targets. Mitigation: conditional copy (caller digest AND target binding verified) else re-derive; safe recomputation default, no O(changed-files) claim; rejected: caller-file-only copy predicate.
- **v4 migration collisions** — risk of double-ALTER on fresh DBs. Mitigation: guarded ADD COLUMN (only when missing) + v1/v2/v3 chain tests; rejected: unconditional ALTER.
- **Traversal blowup**: hard budgets + full-key visited set; rejected: unbounded DFS, separate graph engine, bare-name visited keys.
- **False-safe impact**: UNKNOWN-first rule + denominator accounting + golden corpus; `LOW`/`SAFE` never emitted in M6; rejected: heuristic LOW/SAFE, clean-leaf→LOW test.
- **LSP as call proof**: forbidden by Tier B gate; LSP stays REFERENCES/POTENTIAL_CALL max.
- **Trace ambiguity**: `--to` destination semantics vs bounded enumeration, both deterministic; rejected: open-ended path dump.

## Revision Notes (REVISE 2026-10-16, not yet approved)

What changed vs the first draft, and which correction each closes:
1. Lifecycle (P0): syntax build vs import-txn upsert vs post-carry re-derivation split; conditional-copy rule; 7 lifecycle tests incl. race — Tasks 2–3, Interfaces, Gates.
2. CALLS proof (P0): terminal-identifier split, matching-occurrence + same-import target definition, callable-owner rule, LSP-never-CALLS — Decisions 1–2, Task 1/3 tests.
3. Edge identity (P0): `EdgeKey` (SCIP `import_id+uri+doc` / syntax file+range) + inbound/outbound indexes matching query patterns — Interfaces, Decision 5 visited keys.
4. Migration/incremental (P0): guarded ALTER, v1/v2/v3 chains, NULL-callee re-extraction, safe-recompute default — Task 2, Gates.
5. Impact/completeness (P0): denominator definition, UNKNOWN-first, no LOW/SAFE in M6, clean-leaf→UNKNOWN — Decisions 4/6, Task 6, Gates. Corrected the earlier contradictory revision note at the Task 6 checkpoint.
6. CLI anchor/trace (P0): `syntax-fallback` anchor with explicit uncertainty, `--to` semantics, deterministic pagination, single-filter test commands — Decision 4, Task 4/5.
