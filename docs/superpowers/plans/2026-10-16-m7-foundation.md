# M7 Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement M7 Foundation knowledge graph (records, revisions, evidence, one-hop relations, lexical search) on a separate authoritative SQLite store.

**Architecture:** New `knowledge.rs` (validation + JSON contracts) over new `knowledge_store.rs` (schema v1, FTS5, single-write-transaction + generation); thin `args.rs`/`main.rs` dispatch for `knowledge.*`; reuse `registry.rs`/`paths.rs`/`semantic.rs` confined reads and `store.rs`-style SQLite patterns without touching M0–M6.

**Tech Stack:** Stable Rust 2021, `rusqlite` (bundled, FTS5), `serde_json` (existing dependency) for stdin parsing with strict duplicate-key/unknown-field rejection plus existing `json.rs` escape helpers for output, no new dependencies.

**Spec:** `docs/superpowers/specs/2026-10-16-m7-native-structured-knowledge-graph-design.md`

## Global Constraints

- Foundation only: kinds `decision`/`constraint`/`benchmark`; statuses `active`/`archived` only — `superseded`/`retracted` record statuses are Full-stage and must be rejected as invalid in Foundation.
- Relationship `retraction` (immutable event on `link`, Foundation) is distinct from record status `retracted` (Full-stage, not implemented) — never mix them.
- One SQLite DB per repo at `$RILTRA_HOME/knowledge/<registered-name>.db`, independent `schema_version: 1`, metadata-bound to canonical root; never touch M6 DB, schema v4, semantic generation, or graph-proof marker.
- Every mutation is one write transaction (revisions/heads, evidence, relations where relevant, FTS, monotonic `knowledge_generation`); writer serialization + bounded lock wait; failure rolls back data and generation.
- Exit codes: 0 success (incl. empty search), 2 invalid args/input, 3 repo/storage/missing-ID/revision/generation conflict; no partial-success JSON.
- Byte limits (not char counts): 1 MiB mutation JSON, 512 B title, 64 KiB body, 64 evidence refs/rev, 4096 B query text, 1000 rows/page, author 256 B, rationale/reason 64 KiB; source reads reuse 1 MiB cap; limit default 50 cap 1000.
- Deterministic JSON, binary collation, total order: history by revision, search by record ID, relations by endpoint/revision/kind + relation ID; FTS5 indexes current title/body only, same transaction; query text is literal lexical terms, never raw SQL.
- Authored text inert: no shell/template/provider launch/code exec/upload/telemetry; author label is a claim, not authenticated identity; claims stay `Unverified` even when evidence is fresh.
- No new dependency, MCP, embeddings, autonomous authoring, cross-repo graph, YAML/Markdown importer, BFS/transitive inference, or `AGENTS.md` change; no M0–M6 refactor; no push/merge without permission; DCO sign-off (`git commit -s`) per task.
- Verify each task with `cargo fmt --all -- --check`, `cargo check --locked`, `cargo clippy --all-targets -- -D warnings`, `cargo test --locked`; no full cross-platform CI per task (separately authorized).

## Review Focus

- Windows junction vs Unix symlink on knowledge dir/DB/`-wal`/`-shm`/`-journal` silently followed instead of rejected as storage error.
- Concurrent `update --expected-revision` pair both succeeding (lost update) instead of one winner + conflict.
- `related` on current revision returning empty without a way to see historical relations, implying never-related.
- Matching evidence bytes flipping a claim to verified instead of staying `Unverified` with fresh evidence.
- Paging `search`/`history`/`related` across a changed `knowledge_generation` returning duplicates/omissions instead of explicit generation-conflict error.

---

### Task 1: Knowledge store open, schema v1, location security base

**Files:**
- Create: `crates/cli/src/knowledge_store.rs`
- Modify: `crates/cli/src/main.rs` (register `mod knowledge_store;` only — minimum to keep every commit buildable)
- Test: unit tests inside `crates/cli/src/knowledge_store.rs` first; no `crates/cli/tests/knowledge.rs` yet (CLI integration stays Task 7)

**Interfaces:**
- Consumes: `registry::RepoEntry { name, root }`, `paths::{canonical_root, contains, validate_name, config_dir}`, `store::{content_digest, is_readable_db}` patterns.
- Produces: `pub fn knowledge_db_path(home: &Path, repo: &str) -> PathBuf`, `pub const KNOWLEDGE_SCHEMA_VERSION: u32 = 1`, `pub fn open_knowledge(repo: &RepoEntry, create: bool) -> Result<Connection, String>` (fails exit-3 class on missing-store-read/corrupt/forward/mismatch/escape; never creates on read; never deletes). `meta` binds `{repo_name, canonical_root, knowledge_schema_version: 1}` exactly.

- [ ] **Step 1: Write failing unit tests** in `knowledge_store.rs`: missing-store `open_knowledge(create=false)` errors and creates no file; corrupt file fails closed; forward `user_version>1` fails; metadata mismatch (name/root/version) fails; path-security matrix — knowledge-dir symlink escape, DB-file symlink swap, pre-planted `-wal`/`-shm`/`-journal` symlink, Unix symlink cases always run plus Windows-junction case where supported (`#[cfg(windows)]`/junction-capable, skipped with reason otherwise); canonical form re-checked after SQLite open (SQLite follows links, so post-open check is normative). Assert `knowledge_db_path` is `$HOME/knowledge/<name>.db`.
- [ ] **Step 2: Run to verify they fail** — Run: `cargo test --locked knowledge_store_open` Expected: FAIL (module/function not defined).
- [ ] **Step 3: Implement** `knowledge_db_path`, `KNOWLEDGE_SCHEMA_VERSION`, schema v1 DDL (`meta`, `records` revisions immutable, `heads`, `evidence`, `relations`, `retractions`, `knowledge_generation`, FTS5 virtual table + triggers maintained in same transaction), `open_knowledge` with: pre-open dir canonical check, `create=false` returns before any file creation, post-open file canonical re-check for DB + companions, per-open canonical-root + registry-binding check against `meta`, `PRAGMA busy_timeout`, `user_version` check. TOCTOU across arbitrary FS races is out of scope beyond SQLite transactions + bounded writer wait; the pre/post-open checks above are the auditable concrete behavior.
- [ ] **Step 4: Run tests** — Run: `cargo test --locked knowledge_store` Expected: PASS; plus `cargo fmt --all -- --check`.
- [ ] **Step 5: Commit** — Run: `git add crates/cli/src/knowledge_store.rs crates/cli/src/main.rs && git commit -s -m "m7: knowledge store open schema-v1 location checks"` Expected: commit created, buildable (`cargo check --locked` passes).
- [ ] **Step 6: STOP checkpoint (storage-security audit)** — No further tasks until reviewer audits Task 1 path-security tests and metadata binding and gives explicit GO for Task 2+ (early Gate I sub-gate; Gate P plan approval does not replace it).

Gates: F-STORE (open/bind/missing/corrupt/forward/escape core).

### Task 2: Record revisions, concurrency, create/show/update/history

**Files:**
- Create: `crates/cli/src/knowledge.rs` (model + validation + response JSON)
- Modify: `crates/cli/src/knowledge_store.rs` (revision writes/reads), `crates/cli/src/main.rs` (add `mod knowledge;` — minimum to keep this commit buildable)
- Test: unit tests inside `crates/cli/src/knowledge.rs` / `knowledge_store.rs` first; still no `crates/cli/tests/knowledge.rs` (CLI integration stays Task 7)

**Interfaces:**
- Consumes: Task 1 `open_knowledge`, `store::content_digest`, `json::escape`.
- Produces: `pub struct KnowledgeInput` parsing, `pub fn create_record(conn, input) -> Result<(u64 /*rev*/, u64 /*generation*/), String>`, `pub fn show_record(conn, id, revision: Option<u64>)`, `pub fn update_record(conn, id, expected_revision: u64, input)`, `pub fn history(conn, id, limit, offset, generation: Option<u64>)`; ID rule `1–128 ASCII [a-z0-9][a-z0-9._-]*`; `status` Foundation-only `active|archived` (reject `superseded|retracted` with exit-2 class).

- [ ] **Step 1: Write failing unit tests**: create→show→update→history preserves revs 1,2; stale `--expected-revision` fails with no write and no generation bump; duplicate ID fails; ID/kind rename attempt fails; `archived` then restore each adds a revision; history ordering by revision with limit/offset/total/truncation + generation guard; concurrent writers: two `Connection`s to the same store file both read head rev N, both attempt `update_record(expected N)` — exactly one wins, the loser gets a revision-conflict error with no write and no generation bump.
- [ ] **Step 2: Run to verify they fail** — Run: `cargo test --locked record_revisions` Expected: FAIL.
- [ ] **Step 3: Implement** one atomic write transaction per update (`BEGIN IMMEDIATE` + bounded busy wait): verify `heads.current == expected_revision` inside the transaction, then insert new immutable revision + swap head + maintain FTS + bump `knowledge_generation` before commit; any check failure rolls back everything including generation. Status changes only through this path (no separate archive command).
- [ ] **Step 4: Run tests** — Run: `cargo test --locked record_ && cargo clippy --all-targets -- -D warnings` Expected: PASS, no warnings; commit stays buildable.
- [ ] **Step 5: Commit** — Run: `git add crates/cli/src/knowledge.rs crates/cli/src/knowledge_store.rs crates/cli/src/main.rs && git commit -s -m "m7: immutable record revisions with expected-revision concurrency"` Expected: commit created.

Gates: F-REV; contributes F-ATOMIC (duplicate/rename atomicity).

### Task 3: Strict input validation, benchmark fields, byte limits, fault atomicity

**Files:**
- Modify: `crates/cli/src/knowledge.rs`, `crates/cli/src/knowledge_store.rs`
- Test: `crates/cli/tests/knowledge.rs`

**Interfaces:**
- Consumes: Task 2 create/update paths, `serde_json` (existing dependency).
- Produces: `pub fn validate_mutation_json(bytes: &[u8]) -> Result<ValidatedInput, String>` parsing stdin with `serde_json` (no new manual parser), enforcing 1 MiB cap, required fields, strict unknown-field and duplicate-key rejection, types, all byte limits; benchmark requires method, environment, positive sample count, unique finite numeric metrics with units (no execution/normalization/certification).

- [ ] **Step 1: Write failing unit tests**: validation matrix (missing/unknown/duplicate/type/limit cases incl. title 513 B, body 64 KiB+1, author 257 B, 65 evidence refs, 4097 B query, metric dup/non-finite/missing-unit, `{"a":1,"a":2}` duplicate-key input) all fail atomically with no head/generation change; injected mid-write failure (test hook returning error after first insert) preserves heads/history/FTS/generation.
- [ ] **Step 2: Run to verify they fail** — Run: `cargo test --locked validation_matrix` Expected: FAIL.
- [ ] **Step 3: Implement** `serde_json` pre-write validation before opening the write transaction (duplicate keys rejected during deserialization, unknown fields denied); bound SQL values everywhere; fault hook `#[cfg(test)]` only.
- [ ] **Step 4: Run tests** — Run: `cargo test --locked validation_ && cargo check --locked` Expected: PASS.
- [ ] **Step 5: Commit** — Run: `git add crates/cli/src/knowledge.rs crates/cli/src/knowledge_store.rs && git commit -s -m "m7: strict typed validation and atomic failure rollback"` Expected: commit created, buildable.

Gates: F-ATOMIC.

### Task 4: Evidence capture, freshness, truth boundary

**Files:**
- Modify: `crates/cli/src/knowledge.rs`, `crates/cli/src/knowledge_store.rs`
- Test: unit tests in modified modules first (no integration file yet)

**Interfaces:**
- Consumes: `paths::contains`, `semantic.rs` confined-read + digest pattern, `git.rs` head/branch read, Task 2 revision write.
- Produces: evidence items `{path, byte_start?, byte_end?, digest, git_head?, git_branch?}` recorded at write; `pub fn evidence_freshness(recorded, live_root) -> Fresh|Stale|Unresolved { reason }`; reads report `verification: "Unverified"` always.

- [ ] **Step 1: Write failing unit tests**: write with path+span captures digest; read with matching bytes → fresh; edited bytes → stale preserving original path/span/digest; missing/escaping/unreadable/over-cap → unresolved with reason, never replacement target; outside-root/symlink-escape/invalid-span rejects whole mutation; no-evidence → unknown support; fresh evidence still `Unverified`.
- [ ] **Step 2: Run to verify they fail** — Run: `cargo test --locked evidence_` Expected: FAIL.
- [ ] **Step 3: Implement** confined bounded reads at write, Git metadata best-effort, freshness sampling on inspecting reads (knowledge rows + generation in one read txn; FS sampled separately, reported honestly).
- [ ] **Step 4: Run tests** — Run: `cargo test --locked evidence_` Expected: PASS.
- [ ] **Step 5: Commit** — Run: `git add crates/cli/src/knowledge.rs crates/cli/src/knowledge_store.rs && git commit -s -m "m7: evidence provenance with fresh-stale-unresolved"` Expected: commit created, buildable.

Gates: F-EVID.

### Task 5: Lexical search, pagination, generation guard

**Files:**
- Modify: `crates/cli/src/knowledge_store.rs`, `crates/cli/src/knowledge.rs`
- Test: unit tests in modified modules first (no integration file yet)

**Interfaces:**
- Consumes: Tasks 1–2 (FTS index, heads, generation).
- Produces: `pub fn search(conn, query, kind: Option<&str>, include_archived: bool, limit, offset, generation: Option<u64>)` with FTS5 literal terms over current title/body only, `schema_version: 1` + generation + limit/next-offset/total/truncation in response.

- [ ] **Step 1: Write failing unit tests**: ordering by record ID; boundary/empty pages; truncation flags; changed-generation paging fails explicitly; SQL-injection-looking text treated literally; historical-revision text not searchable.
- [ ] **Step 2: Run to verify they fail** — Run: `cargo test --locked search_` Expected: FAIL.
- [ ] **Step 3: Implement** FTS5 triggers in same write txn, literal-term binding, filters-before-counts, generation check on subsequent pages.
- [ ] **Step 4: Run tests** — Run: `cargo test --locked search_ && cargo fmt --all -- --check` Expected: PASS.
- [ ] **Step 5: Commit** — Run: `git add crates/cli/src/knowledge.rs crates/cli/src/knowledge_store.rs && git commit -s -m "m7: deterministic FTS5 search with generation guard"` Expected: commit created, buildable.

Gates: F-QUERY (search/history part).

### Task 6: Revision-pinned relations, retraction vs record-status guard

**Files:**
- Modify: `crates/cli/src/knowledge.rs`, `crates/cli/src/knowledge_store.rs`
- Test: unit tests in modified modules first (no integration file yet)

**Interfaces:**
- Consumes: Tasks 1–2 writers/generation.
- Produces: `pub fn link(conn, src_id, src_rev, dst_id, dst_rev, kind, author, rationale)`, `pub fn unlink(conn, relation_id, author, reason)`, `pub fn related(conn, id, revision: Option<u64>, direction, include_retracted: bool, limit, offset, generation)`; relation kinds `relates_to|supports|contradicts`; opaque repo-local relation IDs.

- [ ] **Step 1: Write failing unit tests**: both endpoint revs must exist same-DB else reject; no null/foreign/guessed/name-matched; no dup active endpoint/kind tuple; record update does not move relations; `related` current vs historical separated (current-empty still reports correctly, historical query shows old link); normal `related` omits retracted, `include-retracted`/audit retains; double-`unlink` fails; cross-repo IDs rejected; cycles/self-links allowed one-hop; record payload with `status: "retracted"` rejected (Full-stage, not Foundation) while `unlink` retraction event succeeds — pinned as separate tests so the two lifecycles never mix.
- [ ] **Step 2: Run to verify they fail** — Run: `cargo test --locked relation_` Expected: FAIL.
- [ ] **Step 3: Implement** pinned endpoints, retraction-events table (append-only), one-hop only, same-transaction generation bump.
- [ ] **Step 4: Run tests** — Run: `cargo test --locked relation_` Expected: PASS.
- [ ] **Step 5: Commit** — Run: `git add crates/cli/src/knowledge.rs crates/cli/src/knowledge_store.rs && git commit -s -m "m7: revision-pinned relations with retraction events"` Expected: commit created, buildable.

Gates: F-REL; F-QUERY (related part).

### Task 7: CLI wiring, isolation from M6, final STOP checkpoint

**Files:**
- Modify: `crates/cli/src/args.rs`, `crates/cli/src/main.rs`
- Test: create `crates/cli/tests/knowledge.rs` here (first and only CLI integration tests: `RILTRA_HOME` isolation, exit codes)

**Interfaces:**
- Consumes: Tasks 1–6 store/model functions.
- Produces: `knowledge create|show|update|history|search|link|unlink|related` per spec surface (`--repo`, `--id`, `--revision`, `--expected-revision`, `--input -`, `--query`, `--kind`, `--include-archived`, `--direction`, `--include-retracted`, `--limit/--offset/--generation`); usage text + dispatch; responses carry `schema_version: 1`, repo, generation, verification/freshness fields.

- [ ] **Step 1: Write failing CLI tests**: each subcommand happy path + exit codes (0/2/3); invalid JSON → 2; missing ID / stale revision / generation conflict → 3; missing-store search empty 0 (no DB created) vs show 3; corrupt store 3 without delete; knowledge persists across `index --full`, failed promotion path, `semantic import`, graph-proof revalidation; no M6 file/golden change (assert code-index DB untouched); works with no code index.
- [ ] **Step 2: Run to verify they fail** — Run: `cargo test --locked --test knowledge cli_` Expected: FAIL.
- [ ] **Step 3: Implement** arg parsing (both `--flag value` and `--flag=value` per existing style), `serde_json` strict stdin input, dispatch with repository selection + stderr diagnostics, no M6 imports touched.
- [ ] **Step 4: Run full local gate** — Run: `cargo fmt --all -- --check && cargo check --locked && cargo clippy --all-targets -- -D warnings && cargo test --locked` Expected: all PASS; confirm `git status --porcelain` shows only M7 files + plan.
- [ ] **Step 5: Commit** — Run: `git add crates/cli/src/args.rs crates/cli/src/main.rs crates/cli/tests/knowledge.rs && git commit -s -m "m7: wire knowledge CLI with M6 isolation"` Expected: commit created.
- [ ] **Step 6: STOP checkpoint (final plan gate)** — No further tasks; report per-task commits, gate→task map (F-REV→T2, F-QUERY→T5+T6, F-ATOMIC→T2+T3, F-REL→T6, F-EVID→T4, F-STORE→T1+T7, F-PRESERVE→T7), full local evidence output, and remaining risks (companion-file TOCTOU limits, Windows junction coverage). Correct gate order: Gate P approves this plan and authorizes Foundation implementation (Tasks 1–7); the Task 1 STOP above is an early storage-security sub-gate inside Gate I; this final STOP awaits Foundation-closure audit (Gate I GO) before any Full-stage work.

Gates: F-STORE (remainder) + F-PRESERVE; closes Foundation plan.

## Gate→Task Map

- F-REV → Task 2 (+ atomic part Task 3)
- F-QUERY → Task 5 (search/history) + Task 6 (related)
- F-ATOMIC → Task 3 (Task 2 duplicate/rename included)
- F-REL → Task 6 (with retraction-vs-record-status guard)
- F-EVID → Task 4
- F-STORE → Task 1 + Task 7 CLI remainder
- F-PRESERVE → Task 7
- Full G-LIFE/G-PORT/G-BACK/G-PROV → explicitly out of scope; Task 2/6 reject `superseded`/`retracted` record statuses.
