# M8 design gate — unified CLI + MCP read interface

**Status: PROPOSED — NOT APPROVED.** This document authorizes nothing. It is a
design-only checkpoint on branch `m8-unified-cli-mcp`, created directly from
`master` at `cb40aea1581683857dd55eff7ea304c311313f31` (M7 closed, merged, CI
green). Implementation starts only after reviewer audit and owner GO, with the
open decisions in §8 resolved.

## 1. Current-state evidence

All references verified against the tree at `cb40aea`.

**Shape.** One binary-only crate: `crates/cli` (package `riltra`), no
`lib.rs`. 21 source modules, 36,188 lines in total. Largest: `knowledge_store.rs` 9073,
`index.rs` 3960, `store.rs` 3258, `edges.rs` 3055, `main.rs` 2680 (handlers +
JSON assembly), `args.rs` 1014.

**Dispatch.** `args::Command` (`crates/cli/src/args.rs:6`) parses in
`args::parse` (:189); `main.rs:dispatch` (:639) routes every variant to a
`cmd_*` handler; knowledge operations are `args::KnowledgeOp`
(`args.rs:111–185`) routed through `cmd_knowledge` (`main.rs:1232`).

**Reuse surface (already `pub`, typed results).**

| Capability | Functions (file:line) |
|---|---|
| Repo registry | `registry::load/find/list_json/entry_json` (`registry.rs:27–95`) |
| Index build/freshness | `index::build` (`index.rs:2328`), `build_full` (:293), `evaluate` (:2486) |
| Code search | `store::search_symbols` (`store.rs:1935`), limits `index.rs:16–17` |
| Callers/callees | `index::edge_query` (`index.rs:1797`) → `EdgeQueryOut` (:1585) |
| Trace / impact | `index::trace_query` (`index.rs:2011`), `impact_query` (:2030); budgets `edges.rs:755–759` |
| Knowledge read | `knowledge_store::show_record` (:1471), `search` (:1763), `related` (:2248); pages `HistoryPage` :651, `SearchPage` :1708, `RelatedPage` :1935 |
| Semantic verification | `semantic::verify_live` (`semantic.rs:413`), `capabilities` (:208) |
| Environment | `doctor::run_checks` (`doctor.rs:31`), `git::fingerprint` (`git.rs:87`) |

**Output conventions.** Exit codes 0/2/3 (`main.rs:1226–1230`); usage text on
stderr. Knowledge envelope `{schema_version, repo, generation, operation,
result, pagination?}` via `knowledge_envelope` (`main.rs:2127`); edge/trace/
impact carry `schema_version:1` plus embedded `limits` (`main.rs:893`, :1037).
JSON is hand-built and deterministic (`json.rs:7–23`).

**Pagination and limits.** Code search default 50 / max 1000 (`index.rs:16–17`);
knowledge search default 50 / max 1000 (`knowledge.rs:263–264`); trace/impact
fixed budgets: max_depth 6, nodes 500, edges 3000, paths 1000, time 2000 ms
(`edges.rs:755–759`). Responses carry `total`/`truncated`, with `next_offset`
additionally on edge/trace/impact (`limits`) and on knowledge pages; code search
has no `next_offset`.

**Storage.** `RILTRA_HOME` override with platform default (`paths.rs:11–33`);
registry `registry.json` (:49); code index `<home>/index/<repo>.db`
(`store.rs:36`), opened read-write with migration/WAL (`open_live` :68) — there
is no read-only code-index open; knowledge `<home>/knowledge/<repo>.db`
(`knowledge_store.rs:47`) with a genuine read-only open
(`SQLITE_OPEN_READ_ONLY`, :115).

**Freshness and provenance.** Index freshness `index::evaluate` (`index.rs:2486`,
`"no-index"` at :2490); per-hit `provenance{provider,grammar,language}` +
`snapshot` (`main.rs:454`); knowledge `EvidenceFreshness`
(`knowledge.rs:468–477`), `Unverified` marker (`knowledge.rs:492`); confirmation
state derives from stored confirm events only (M7 G-PROV).

**Tests.** `tests/{cli,syntax,index,edges,semantic,knowledge}.rs` (knowledge E2E
4318 lines, isolated `RILTRA_HOME`); goldens under `tests/golden/`; the
dependency audit test `new_dependencies_are_audited` (`tests/semantic.rs:7`)
currently rejects `tokio` (:77), `async-std`, `hyper`, `reqwest`, `ureq`,
`protoc-bin-vendored`, `libloading`.

**Public promises.** README roadmap already names "an MCP server so AI agents
can query code and knowledge through one interface" (README.md:110);
`docs/licensing.md:27` reserves the `rmcp` inventory note.

## 2. Recommended minimum architecture

**Verified dependency facts (Context7):** `rmcp` 3.5.1 (crates.io; license
Apache-2.0 — confirm the exact pinned LICENSE at implementation time per the
`licensing.md` policy). Server API: `#[tool_router]` + `#[tool_handler]` on a
`ServerHandler`; server-side stdio via the `transport-io` feature; input
schemas via the `schemars` feature; results as `CallToolResult::structured` /
`structured_error`; read-only tools marked with `annotations(read_only_hint =
true)`. `rmcp` is async — it needs a Tokio runtime.

### Option 1 — recommended: in-crate `mcp` module + `riltra mcp` subcommand

One new module (`crates/cli/src/mcp.rs`, split further only if it outgrows one
file) inside the existing binary crate, plus a `Command::Mcp` variant that
builds a single-threaded Tokio runtime and serves MCP over stdio. Tool handlers
are thin: validate input → call the existing `pub` domain functions listed in
§1 → wrap their results in the same envelopes the CLI emits (today only the
knowledge envelope builders exist as helpers — `knowledge_envelope`,
`knowledge_query_json` in `main.rs`; search/edge/trace/impact JSON is assembled
inline in its handlers, and CP2 exposes that assembly for shared use). No new
crate, no `lib.rs` extraction, and no storage or domain changes beyond those
small handler extractions. CLI commands stay synchronous; Tokio lives only
inside the `mcp` path.

Layering: `rmcp` transport (stdio) → tool contract (schemas, error mapping) →
domain functions (unchanged) → storage (unchanged).

### Option 2 — considered: process-boundary adapter

A thin server that executes the built `riltra` CLI per tool call and wraps its
JSON stdout. Zero changes to existing code and literal contract parity with
golden-tested CLI output; but every call pays process spawn + SQLite open,
error mapping is limited to exit codes, and output parsing couples the server
to text formatting. Kept as the fallback if Option 1 is rejected.

**Rejected: extracting `crates/core` + a second binary now.** A library split
across M0–M7 code is exactly the large refactor the M8 scope forbids; revisit
only if a second non-CLI consumer appears (YAGNI).

## 3. Proposed MCP contract (read-only)

Server: name `riltra`, stdio only, tools only (no resources/prompts in M8).
Every result is the same deterministic JSON the CLI emits for the same
inputs (unwrapped from MCP framing), with two composed exceptions whose shapes
are defined in the table: `riltra_health` and `riltra_repo_status`.

| Tool | Maps to | Inputs (mirror CLI flags) | Result |
|---|---|---|---|
| `riltra_repo_list` | `repo list` | — | registry JSON |
| `riltra_repo_status` | `repo status` + `index status` | repo | `repo status` JSON plus an `index_status` object (composed) |
| `riltra_code_search` | `search` | repo, query, generation, limit, offset | hits + `total`/`truncated` + freshness + provenance |
| `riltra_code_callers` | `edge` (callers) | repo, anchor path+line+col, limit, offset | edge JSON |
| `riltra_code_callees` | `edge` (callees) | repo, anchor path+line+col, limit, offset | edge JSON |
| `riltra_code_trace` | `trace` | repo, endpoints, flags, paging | trace JSON + budgets |
| `riltra_code_impact` | `impact` | repo, symbol, flags, paging | impact JSON + verdict |
| `riltra_knowledge_search` | `knowledge search` | repo, query, kind, generation, visibility flags, limit/offset | knowledge envelope + `SearchPage` |
| `riltra_knowledge_show` | `knowledge show` | repo, id, revision | knowledge envelope + record + evidence freshness |
| `riltra_knowledge_related` | `knowledge related` | repo, id, revision, generation, direction, include-retracted, limit/offset | knowledge envelope + `RelatedPage` |
| `riltra_health` | `status` + `doctor` + constants | — | versions (riltra, schema v4/v2), capabilities, limits, repo count |

Conventions for every tool:

- **Pagination/limits** identical to the CLI: same defaults, same caps
  (1000), same field sets (`total`/`truncated` everywhere; `next_offset` only
  where the CLI has it); trace/impact echo the fixed budgets. No new limits are
  invented.
- **Errors** are structured tool results (`is_error: true`), classified by the
  existing taxonomy: usage/input problems (exit-2 semantics) as
  `kind:"invalid_input"`; repository/storage problems (exit-3 semantics) as
  `kind:"not_registered" | "no_index" | "store_missing" | "store_error" |
  "conflict"` with the
  CLI's message text. Missing stores mirror the CLI per capability:
  `riltra_code_search` stays a **success** envelope with `freshness:"unknown"`,
  `reasons:["no-index"]`, while edge/trace/impact map to `no_index` and
  knowledge tools to `store_missing`. Malformed JSON-RPC still yields protocol
  errors; domain failures never do.
- **Freshness, evidence, provenance, uncertainty** are surfaced exactly as the
  CLI surfaces them (`no-index`, `Unverified`, confirmation derived from stored
  events). No tool claims precision the implementation does not provide.
- **No side effects on missing stores**: graph tools check the index store
  exists before opening, so a read never creates or migrates a store (the CLI's
  writable open would); the adapter returns the CLI's error without touching
  disk.
- **Annotations**: `read_only_hint: true` on every tool; `open_world_hint:
  false`; idempotent where the underlying read is.
- **No mutating tools** in M8: no create/update/supersede/link/import/
  backup/restore over MCP.

## 4. Security and compatibility constraints

- Transport is stdio only: no listener, no daemon, no HTTP; the process lives
  and dies with its client.
- All repository access stays confined to registered roots via existing
  containment checks; no new filesystem surface.
- Knowledge reads use the existing read-only open. Note the accepted M7
  limitation L2: read-only opens can leave `-wal`/`-shm` companions, which is
  already true for read-only CLI commands; MCP changes nothing about restore
  fail-closed behavior. No change is made to restore or checkpointing.
- Code-index reads on an existing store mirror CLI read commands (same open
  path, migration included); tools never create or migrate a missing store
  (they short-circuit to the CLI's `no_index` error first). No new read-only
  mode is added in M8 (open decision D3).
- No SQLite schema change, no change to CLI output bytes (goldens enforce),
  no new config, no secrets in prompts/logs.
- Dependency additions (`rmcp`, Tokio, `schemars`) require a deliberate
  amendment of the dependency audit test and `licensing.md` entries with exact
  versions and licenses (open decision D1). No other dependency changes.
- Determinism, exit-code contracts, and offline behavior of the CLI are
  untouched; MCP is additive.
- Windows/macOS/Linux: stdio only, no platform-specific code planned.

## 5. Efficiency benchmark (method; no results yet)

- **Corpus**: a frozen, versioned set of 12–20 developer tasks against this
  repository pinned at the M8 branch head (offline, license-clean) plus the
  existing fixtures; each task has a golden answer with file:line (or record id)
  evidence.
- **Arms**: (a) manual CLI baseline — scripted command sequences per task;
  (b) MCP-assisted agent workflow using `riltra mcp` in a real MCP client.
- **Metrics per task**: answer correctness, evidence correctness (cited
  locations resolve and match), tokens per completed task (fixed tokenizer,
  counting rule fixed before measuring), MCP request count, wall-clock latency.
- **Interoperability**: two independent MCP stdio clients (chosen with the
  owner, D5); both run a smoke subset of the corpus.
- **Reproducibility**: corpus, scripts, raw logs and tokenizer settings are
  archived; the benchmark must be re-runnable by a third party.

## 6. Acceptance gates

No gate below is met until measured and evidenced; the design only defines
them.

- **G1** Task correctness (MCP arm) is at least equal to the CLI baseline on
  the frozen corpus; no task regresses.
- **G2** Evidence correctness: every location/record cited by an answer
  resolves; zero fabricated references.
- **G3** Tokens per completed task are no worse than the baseline (≤).
- **G4** Error paths: unregistered repo, invalid params, oversized limits, and
  — where the CLI itself errors — missing stores return classified structured
  errors with CLI exit-2/exit-3 semantics; no panics, no hangs.
- **G5** Parity: for identical inputs, each tool's result JSON matches the CLI
  envelope byte-for-byte, except the two composed tools whose shapes §3 defines
  (`riltra_health`, `riltra_repo_status`); enforced by tests.
- **G6** Interop: both selected clients complete the smoke corpus.
- **G7** No M0–M7 regressions: fmt/clippy/check/test plus goldens unchanged on
  all three CI platforms.
- **G8** Dependency governance: audit allowlist amendment and `licensing.md`
  entries merged in the same checkpoint that adds the dependencies.

## 7. Implementation checkpoints

Each checkpoint lands tested, DCO-signed commits on `m8-unified-cli-mcp` and
keeps CI green on all three platforms; nothing merges to `master` before the
owner's post-benchmark GO.

1. **CP1 — server skeleton + dependency governance.** Pin `rmcp`/Tokio/
   `schemars`; amend the dependency audit test deliberately; add
   `licensing.md` entries; `riltra mcp` serves initialize/tools-list.
   Verification: handshake test via an in-process or child-process client;
   CLI goldens untouched.
2. **CP2 — shared response assembly.** Expose the inline handler assembly
   (search/edge/trace/impact) alongside the existing knowledge envelope
   builders, so CLI and MCP build results from one code path. Verification:
   existing goldens byte-identical; no duplicate formatter.
3. **CP3 — repository + search tools.** `riltra_repo_list`,
   `riltra_repo_status`, `riltra_code_search`. Verification: contract tests
   (parity G5, pagination caps, `no-index` behavior).
4. **CP4 — graph tools.** callers/callees/trace/impact with identical budgets
   and paging. Verification: parity, error-path, and no-store-creation tests.
5. **CP5 — knowledge tools.** search/show/related with visibility flags,
   generation guard, freshness/provenance fields. Verification: parity tests;
   read-only behavior confirmed.
6. **CP6 — health tool + hard error paths.** `riltra_health`; classified
   errors for every mapped taxonomy branch. Verification: adversarial tests
   (bad repo, bad id, huge limit, malformed args).
7. **CP7 — benchmark + interop + docs.** Build the corpus, run both arms and
   both clients, archive raw data, update README usage note. Verification:
   gates G1–G6 reported with raw evidence (claims allowed only here).

A task-level TDD plan is deliberately deferred until this gate is approved;
detailed checkpoint sketches stay in the private, ignored planning location.

## 8. Open decisions requiring owner approval

- **D1 — Dependency governance.** Accept adding `rmcp` + Tokio + `schemars`
  and amending the dependency audit test allowlist, with `licensing.md`
  entries? (Recommended: yes, in CP1, explicitly documented.)
- **D2 — Tool naming and set.** `riltra_*` prefix as above, with separate
  callers/callees tools and no history tool in the MVP? (Recommended: yes.)
- **D3 — Code-index open mode.** Mirror the CLI's read commands (existing
  writable open, migration included) instead of adding a read-only open in
  M8? (Recommended: mirror; revisit only if a real hazard appears.)
- **D4 — L2 interplay.** Accept that MCP read-only knowledge access can leave
  `-wal`/`-shm` companions exactly like read-only CLI commands, with no
  change to restore behavior? (Recommended: accept.)
- **D5 — Benchmark specifics.** Approve the tokenizer/counting rule, the corpus
  base (this repository at the M8 branch head), and name the two MCP clients for
  interop before CP7 starts.
- **D6 — Command surface.** `riltra mcp` subcommand (vs. a separate binary
  name). (Recommended: subcommand.)
- **D7 — History tool.** Defer `knowledge history` (and `knowledge export`
  read-only export) out of the M8 MVP? (Recommended: defer.)

**Out of scope for M8 (fixed):** MCP write tools, HTTP/daemon, embeddings/
vector stores, UI, new semantic engines, SQLite schema changes, M0–M7
refactors, M7 restore maintenance, and any code from other projects
(Terapetra, GitNexus).
