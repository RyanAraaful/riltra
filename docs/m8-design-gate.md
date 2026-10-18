# M8 design gate — unified CLI + MCP read interface

**Status: APPROVED — owner GO on 2026-10-18 (D1–D7 resolved). Implementation
authorized for CP1 only; CP2–CP7 each still need their checkpoint evidence and
a fresh owner GO.** Design checkpoint on branch `m8-unified-cli-mcp`, created
directly from `master` at `cb40aea1581683857dd55eff7ea304c311313f31` (M7
closed, merged, CI green). M8 is not closed; no unimplemented gate is PASS.

**Revision 2 (2026-10-18):** bounded correction for review findings F1–F4
(storage read contract, missing-store semantics, tool inputs and parity,
benchmark fairness). Option 1, the 11 tools, and the seven checkpoints are
unchanged.

**Owner resolution (2026-10-18):** D1–D7 approved as recorded in §8; overall
M8 design accepted; CP1 implementation authorized (dependencies + `riltra mcp`
skeleton only). The §3 F1 no-implicit-migration guard remains mandatory for
CP3.

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
(`store.rs:36`) with a rebuild staging sibling (`store.rs:41`); knowledge
`<home>/knowledge/<repo>.db` (`knowledge_store.rs:47`).

**Code-index open path (F1 audit).** `store::open_live` (`store.rs:68`) always
ends at the current schema: every open runs `migrate` (`:60`, `:192`) — an
empty file is built at v4, v1–v3 upgrade in one transaction, v4 is a no-op
(`:201–203`), and newer versions are rejected (`"database schema vN is newer
than supported v4"`, `:196–200`). `store::is_readable_db` (`:74`) never
creates files and does not read the version. `cmd_search` guards with
`db.exists()` (`main.rs:391`); `callers`/`callees`/`trace`/`impact` and
`cmd_index_status` open live unconditionally (`index.rs:1824`, `:2070`,
`main.rs:313`), so a first CLI call on a missing store creates and migrates an
empty database. Knowledge reads are already non-mutating:
`open_knowledge(create=false)` never creates (`knowledge_store.rs:97`), the
read-only open (`:115`) never migrates, and a v1 store reads with documented
defaults and stays v1 (`knowledge_store.rs:4820`).

**Missing-store behavior (F2 audit).** `search` without an index → exit 0,
success document `snapshot:null, freshness:"unknown", reasons:["no-index"]`,
empty results (`main.rs:391–401`). `callers`/`callees`/`trace`/`impact` without
an index → exit 3 `"no snapshot: index the repository first"`
(`index.rs:1826`, `:2072`); `index status` reports `fts` only after an
unconditional live open (`main.rs:313`). `knowledge search` without a store →
exit 0, empty success document with generation 0 (`main.rs:2400–2414`), while
symlink/directory/uninspectable paths fail closed (`main.rs:1362–1379`).
`knowledge show` and `knowledge related` without a store → exit 3 `"no
knowledge store for repository '<repo>'"` (`main.rs:2219`, `:2589`;
`knowledge_store.rs:97`).

**Freshness and provenance.** Index freshness `index::evaluate` (`index.rs:2486`,
`"no-index"` at :2490); per-hit `provenance{provider,grammar,language}` +
`snapshot` (`main.rs:454`); knowledge `EvidenceFreshness`
(`knowledge.rs:468–477`), `Unverified` marker (`knowledge.rs:492`); confirmation
state derives from stored confirm events only (M7 G-PROV).

**Tests.** `tests/{cli,syntax,index,edges,semantic,knowledge}.rs` (knowledge E2E
4318 lines, isolated `RILTRA_HOME`); goldens under `tests/golden/`; the
dependency audit test `new_dependencies_are_audited` (`tests/semantic.rs:7`)
currently rejects `tokio` (:77), `async-std`, `hyper`, `reqwest`, `ureq`,
`protoc-bin-vendored`, `libloading`. Store unit tests already cover the
migration chain (v1→v4, future-version rejection, no-op re-migrate;
`store.rs:2389–2963`); the guard regression tests extend them.

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
small handler extractions plus one small `store` guard helper used before any
index read (see §3). CLI commands stay synchronous; Tokio lives only inside
the `mcp` path.

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
Results are the same result documents the CLI produces for the same inputs,
with two composed exceptions whose shapes the table defines (`riltra_health`,
`riltra_repo_status`). Parity is defined on the parsed document, never on
serialized bytes: CLI stdout stays byte-stable under its existing goldens,
while MCP structured content is serialized by the SDK. Acceptance tests parse
both sides and compare values (object-key order not significant, array order
significant). Results are produced by the CLI's own assembly code (CP2); no
second formatter is maintained.

| Tool | Maps to | Inputs (mirror CLI flags) | Result |
|---|---|---|---|
| `riltra_repo_list` | `repo list` | — | registry JSON |
| `riltra_repo_status` | `repo status` + `index status` | repo | `repo status` JSON plus an `index_status` object (composed) |
| `riltra_code_search` | `search` | repo, query, limit, offset | hits + `total`/`truncated` + freshness + provenance |
| `riltra_code_callers` | `callers` | repo, path, line, col (required), symbol (optional check), limit, offset | edge JSON |
| `riltra_code_callees` | `callees` | repo, path, line, col (required), symbol (optional check), limit, offset | edge JSON |
| `riltra_code_trace` | `trace` | repo, path, line, col (required), symbol, to, depth, limit, offset | trace JSON + budgets |
| `riltra_code_impact` | `impact` | repo, path, line, col (required), symbol, depth, limit, offset | impact JSON + verdict |
| `riltra_knowledge_search` | `knowledge search` | repo, query, kind, generation, visibility flags, limit/offset | knowledge envelope + `SearchPage` |
| `riltra_knowledge_show` | `knowledge show` | repo, id, revision | knowledge envelope + record + evidence freshness |
| `riltra_knowledge_related` | `knowledge related` | repo, id, revision, generation, direction, include-retracted, limit/offset | knowledge envelope + `RelatedPage` |
| `riltra_health` | `status` + `doctor` + constants | — | versions (riltra, schema v4/v2), capabilities, limits, repo count |

Input mapping: `query` is free text (CLI `search <repo> <query>`);
`path`+`line`+`col` is the required file anchor and `symbol` an optional
identity check (CLI `callers`/`callees`/`trace`/`impact`); `to` and `depth`
exist only where the CLI accepts them (`trace`; `trace` and `impact`);
knowledge tools take a record `id` with optional `revision`/`generation`.
Tools expose exactly the flags their CLI command accepts — `code_search` has
no generation guard because CLI `search` has none (`main.rs:345–352`).

Conventions for every tool:

- **Pagination/limits** identical to the CLI: same defaults, same caps
  (1000), same field sets (`total`/`truncated` everywhere; `next_offset` only
  where the CLI has it); trace/impact echo the fixed budgets. No new limits are
  invented.
- **Errors** are structured tool results (`is_error: true`), classified by the
  existing taxonomy: usage/input problems (exit-2 semantics) as
  `kind:"invalid_input"`; repository/storage problems (exit-3 semantics) as
  `kind:"not_registered" | "no_index" | "store_missing" | "schema_outdated" |
  "schema_ahead" | "store_error" | "conflict"` with the CLI's message text.
  Missing stores mirror the CLI per capability, without side effects: search
  tools (`riltra_code_search`, `riltra_knowledge_search`) keep the CLI's
  **success** documents (`no-index` freshness / empty hits, nothing created);
  `callers`/`callees`/`trace`/`impact` → `no_index` with `"no snapshot: index
  the repository first"`; `knowledge_show`/`related` → `store_missing` with
  `"no knowledge store for repository '<repo>'"`. Malformed JSON-RPC still
  yields protocol errors; domain failures never do.
- **Freshness, evidence, provenance, uncertainty** are surfaced exactly as the
  CLI surfaces them (`no-index`, `Unverified`, confirmation derived from stored
  events). No tool claims precision the implementation does not provide.
- **Index-read guard (no creation, no migration)**: every index-reading tool
  runs one small `store` helper first — regular-file existence, read-only open,
  `PRAGMA user_version` — and never reaches the CLI's migrating open on a
  store that would need it. Missing → per-tool semantics above (no file
  created; `riltra_repo_status` reports no-index freshness without creating);
  v<4 → `schema_outdated` with the instruction to run a CLI index command
  (e.g. `riltra index status`) to migrate, then retry; v>4 → `schema_ahead`;
  unreadable/corrupt → `store_error`. At v4 the existing query path runs
  unchanged (its migration step is a verified no-op). Knowledge reads already
  satisfy the same contract and need no guard.
- **Residual I/O limits (accurate)**: opening or reading any WAL-mode store can
  create or touch its `-wal`/`-shm` sidecars — read-only WAL access requires
  readable sidecars, a writable directory so they can be created, or
  `immutable=1`, which is unsafe here (the CLI may rebuild the index
  concurrently) and is therefore not used. Reads never modify schema or data;
  concurrency is identical to the CLI's read commands (WAL snapshot reads;
  rebuilds promote through the staging file).
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
- Code-index reads run the §3 guard (regular-file check, read-only version
  probe) and refuse `schema_outdated`/`schema_ahead` stores instead of
  migrating; v4 stores then use the existing query paths unchanged. No CLI
  behaviour changes. Repository isolation is untouched: repositories resolve
  through the registry and `store::db_path`, and the guard builds no new paths
  (open decision D3).
- No SQLite schema change, no change to CLI output bytes (goldens enforce),
  no new config, no secrets in prompts/logs.
- Dependency additions (`rmcp`, Tokio, `schemars`) require a deliberate
  amendment of the dependency audit test and `licensing.md` entries with exact
  versions and licenses (open decision D1). No other dependency changes.
- Determinism, exit-code contracts, and offline behavior of the CLI are
  untouched; MCP is additive.
- Windows/macOS/Linux: stdio only, no platform-specific code planned.

## 5. Efficiency benchmark (method; no results yet)

- **Frozen corpus**: one versioned set of 12–20 developer tasks against this
  repository pinned at the M8 branch head (offline, license-clean) plus the
  existing fixtures, with identical task text, instructions, and starting
  context for both arms; each task carries a golden answer with file:line (or
  record id) evidence, verified identically for both arms.
- **Fair A/B (primary comparison)**: same agent — same model and version, same
  harness settings, same environment, same pinned repository state,
  `RILTRA_HOME` rebuilt to the same state before each task — with only the
  tool access differing: CLI commands through a shell vs MCP tools. A scripted
  CLI baseline (fixed command sequences per task) is an additional
  deterministic reference for output size and latency floor, never the sole
  comparator.
- **Frozen measurement rules**: tokenizer and counting rule fixed before the
  experiment; counts cover all task-attributable tokens including tool outputs
  and retries, with startup/handshake cost counted separately.
- **Separated metrics per task**: answer correctness, evidence correctness
  (cited locations resolve and match), startup cost, per-call latency,
  tool-call count, total tokens per completed task.
- **Interoperability clients (concrete candidates)**: `opencode` (local stdio
  MCP server via its config; docs verified through Context7) and the `claude`
  CLI / Claude Code (`claude mcp add`, stdio child process; docs verified
  through Context7). Pin exact client versions and config syntax at CP7;
  compatibility is proven only by the actual test, not by this document.
- **Reproducibility**: corpus, scripts, raw logs and tokenizer settings are
  archived; a third party can re-run the benchmark.

## 6. Acceptance gates

No gate below is met until measured and evidenced; the design only defines
them.

- **G1** Task correctness (MCP arm) is at least equal to the CLI baseline on
  the frozen corpus; no task regresses.
- **G2** Evidence correctness: every location/record cited by an answer
  resolves; zero fabricated references.
- **G3** Tokens per completed task are no worse than the CLI arm (≤), measured
  under the frozen §5 rules.
- **G4** Error paths: unregistered repo, invalid params, oversized limits,
  missing stores (per-tool semantics, nothing created), and
  migration-needed/too-new stores (`schema_outdated`/`schema_ahead` with a CLI
  instruction, store left untouched) return classified structured errors with
  CLI exit-2/exit-3 semantics; no panics, no hangs.
- **G5** Parity: for identical inputs, the parsed MCP result document equals
  the parsed CLI result document (object-key order not significant, array order
  significant; parser-based comparison, no serializer assumptions), except the
  two composed tools; CLI stdout stays byte-stable under its existing goldens.
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
3. **CP3 — repository + search tools + read guard.** `riltra_repo_list`,
   `riltra_repo_status`, `riltra_code_search`, and the small `store` read
   guard. Verification: contract tests (parity G5, pagination caps, `no-index`
   behavior) plus guard regression tests: v4 reads leave `user_version`
   unchanged; v1/v2/v3 fixtures refused as `schema_outdated` with the store
   untouched and still CLI-migratable afterwards; `user_version=99` refused as
   `schema_ahead`; missing store → per-tool semantics with no file created.
4. **CP4 — graph tools.** callers/callees/trace/impact with identical budgets,
   anchors, and paging, reusing the CP3 guard. Verification: parity,
   error-path, and no-store-creation tests.
5. **CP5 — knowledge tools.** search/show/related with visibility flags,
   generation guard, freshness/provenance fields. Verification: parity tests;
   non-migrating reads confirmed by the existing read tests
   (`knowledge_store.rs:4820`) and the missing-store semantics above.
6. **CP6 — health tool + hard error paths.** `riltra_health`; classified
   errors for every mapped taxonomy branch. Verification: adversarial tests
   (bad repo, bad id, huge limit, malformed args).
7. **CP7 — benchmark + interop + docs.** Build the corpus, freeze the §5
   measurement rules, run the fair A/B plus the scripted reference and both
   clients, archive raw data, update the README usage note. Verification:
   gates G1–G6 reported with raw evidence (claims allowed only here).

A task-level TDD plan is deliberately deferred until this gate is approved;
detailed checkpoint sketches stay in the private, ignored planning location.

## 8. Open decisions requiring owner approval

- **D1 — Dependency governance.** Accept adding `rmcp` + Tokio + `schemars`
  and amending the dependency audit test allowlist, with `licensing.md`
  entries? (Recommended: yes, in CP1, explicitly documented. Reviewer
  recommendation: accept with license governance.)
- **D2 — Tool naming and set.** `riltra_*` prefix as above, with separate
  callers/callees tools and no history tool in the MVP? (Recommended: yes.
  Reviewer recommendation: accept the 11 prefixed tools.)
- **D3 — Code-index open mode.** Revised by F1: MCP reads never create or
  migrate; an outdated store is refused as `schema_outdated` with a CLI
  migration instruction (§3 guard); v4 stores use the existing query paths.
  Alternative kept on record: mirror the CLI's migrating open. (Reviewer
  recommendation: revise to prevent implicit migration in MCP reads.)
- **D4 — L2 interplay.** Accept that MCP read-only knowledge access can leave
  `-wal`/`-shm` companions exactly like read-only CLI commands, with no
  change to restore behavior? (Recommended: accept. Reviewer recommendation:
  accept the M7 L2 limitation.)
- **D5 — Benchmark specifics.** Approve the §5 fairness protocol (frozen
  corpus, same agent/model/instructions/environment for both arms, frozen
  token rules, separated metrics) and the two interop clients (`opencode`,
  Claude Code CLI) before CP7. (Reviewer recommendation: fix the fair
  benchmark protocol before measurement.)
- **D6 — Command surface.** `riltra mcp` subcommand (vs. a separate binary
  name). (Recommended: subcommand. Reviewer recommendation: accept
  `riltra mcp`.)
- **D7 — History tool.** Defer `knowledge history` (and `knowledge export`
  read-only export) out of the M8 MVP? (Recommended: defer. Reviewer
  recommendation: defer history/export tools.)

**Resolution (2026-10-18):** the owner approved D1–D7 and accepted the
overall M8 design, authorizing CP1 implementation only. CP2–CP7 stay blocked
until their own checkpoint evidence and a fresh owner GO.

**Out of scope for M8 (fixed):** MCP write tools, HTTP/daemon, embeddings/
vector stores, UI, new semantic engines, SQLite schema changes, M0–M7
refactors, M7 restore maintenance, and any code from other projects
(Terapetra, GitNexus).
