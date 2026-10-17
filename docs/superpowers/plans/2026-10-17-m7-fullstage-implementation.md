# M7 Full-stage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement M7 Full-stage (G-LIFE, G-PORT, G-BACK, G-PROV) on schema v2 per the approved spec delta (`docs/superpowers/specs/2026-10-17-m7-fullstage-spec-delta.md`).

**Architecture:** Extend the existing Foundation modules in place. `knowledge.rs` (validation/model/JSON contracts) and `knowledge_store.rs` (schema, migration, FTS5, transactions) gain Full-stage behavior; `args.rs`/`main.rs` gain the new flags and commands. Reuse `registry.rs`/`paths.rs`/`semantic.rs` confined reads and `json.rs` escape helpers. No new dependencies, no M0–M6 changes, no MCP.

**Tech stack:** Stable Rust 2021, `rusqlite` (bundled, FTS5), `serde_json` (existing), existing `json.rs` escape helpers.

**Frozen decisions:** D1–D9 are resolved in the spec delta (schema v2 guarded migration; active-only search; acyclic supersedes lineage; Markdown-wrapper approved; one-transaction import with limits; SQLite-consistent backup with structural integrity only; candidate-by-default provenance; finalized command surface and JSONL/manifest schemas). This plan implements those decisions; it does not reopen them.

**Engineering priority:** Correctness → Security → DRY/KISS → Performance.

## Global Constraints

- Schema v2 = Foundation tables (unchanged) + `record_lifecycle` + `relation_lifecycle`, created by a guarded migration that runs **only inside a Full-stage write transaction**, never on a read (spec §1.3).
- Every mutation is one SQLite write transaction covering revision/heads, evidence, relations where relevant, lifecycle events, FTS, and the monotonic `knowledge_generation`; failure rolls back everything including generation and migration.
- Exit codes unchanged: 0 success (incl. empty search), 2 invalid args/input, 3 repo/storage/missing-ID/revision/generation conflict/corrupt backup.
- Byte limits unchanged: 1 MiB mutation JSON, 512 B title, 64 KiB body, 64 evidence refs/rev, 4096 B query, 1000 rows/page, author 256 B, rationale/reason 64 KiB. Import file ≤ 256 MiB, line ≤ 16 MiB, ≤ 1M records, ≤ 2M relations.
- No digest/checksum (no hash dependency). Integrity is structural only.
- Deterministic JSON, binary collation, total orders per spec.
- Author labels are claims, not authenticated identity; claims stay `Unverified`.
- No new dependency, MCP, embeddings, autonomous authoring, cross-repo graph, or `AGENTS.md`/CI change; DCO sign-off (`git commit -s`) per task.
- Verify each task with `cargo fmt --all -- --check`, `cargo check --locked`, `cargo clippy --all-targets -- -D warnings`, `cargo test --locked`; real CLI smoke for new commands; no push without explicit permission.

## Interfaces (existing, to reuse)

- `knowledge_store.rs`: `KNOWLEDGE_SCHEMA_VERSION`, `SCHEMA_SQL`, `open_inner`, `check_schema_complete`, `check_meta`, `record_from_row`, `relation_from_row`, `create_inner`/`update_inner`, `link`, `unlink`, `related`, `history_inner`, `search`, `read_generation`, `bump_generation`, `revalidate_open_paths`, `capture_evidence`/`write_evidence`/`read_evidence`, `relation_by_id`.
- `knowledge.rs`: `validate_status`, `validate_relation_kind`, `validate_relation_id`, `validate_new`/`validate_update`, `validate_create_json`/`validate_update_json`, `NewRecord`, `UpdateRecord`, `RecordVersion`, `BenchmarkData`, `MetricData`, `CapturedEvidence`, `EvidenceRef`, `VERIFICATION_UNVERIFIED`, `evidence_support`, `MAX_*`.
- `main.rs`: `cmd_knowledge`, `knowledge_open`, `knowledge_repo`, `knowledge_input`, `knowledge_page_args`, `knowledge_store_absent`, `strict_object`, `parse_link_input`/`parse_unlink_input`, `record_json`, `relation_json`, `pagination_json`, `knowledge_envelope`, `knowledge_query_json`, and the eight `knowledge_*` command fns.
- `args.rs`: `parse_knowledge_op`, `knowledge_flags`, `knowledge_value`, `KnowledgeOp`.
- Tests: `crates/cli/tests/knowledge.rs` (CLI integration, `RILTRA_HOME` isolation); unit tests inside `knowledge.rs`/`knowledge_store.rs`; `tests/m6_corpus/upgrade.rs` (rusqlite fixture precedent for direct DB seeding).

---

### Task 1: Schema v2 guarded migration

**Scope:** bump `KNOWLEDGE_SCHEMA_VERSION` to 2; accept stores with `user_version` 1 or 2; refuse `> 2`; refuse `user_version` ≠ `meta.knowledge_schema_version`; create `record_lifecycle` + `relation_lifecycle` lazily inside a Full-stage write transaction.

**Files:** `crates/cli/src/knowledge_store.rs` (open/init/schema); `crates/cli/src/main.rs` (none unless needed).

**Preconditions:** none (first task).

**Interfaces produced:** `pub const KNOWLEDGE_SCHEMA_VERSION: u32 = 2`; `pub fn migrate_to_v2(conn: &mut Connection) -> Result<(), String>` (idempotent, transactional); lifecycle table DDL.

- [ ] **Step 1: Write failing unit tests** in `knowledge_store.rs`: (a) v1 store opens read-only under v2 binary with no migration (tables absent, `user_version` still 1); (b) a Full-stage write against v1 migrates atomically (tables created, `user_version`=2, meta stamped, data byte-identical, generation unchanged); (c) injected mid-migration failure → rollback, store still v1, tables absent, generation unchanged; (d) corrupt v2 (`user_version`=2 missing tables, or version≠meta) → exit 3; (e) forward `user_version`>2 → exit 3; (f) a Foundation write against v1 does not migrate.
- [ ] **Step 2: Run to verify they fail** — `cargo test --locked migration_`. Expected: FAIL.
- [ ] **Step 3: Implement** the version gate and `migrate_to_v2` inside the write path; Foundation writes stay on v1.
- [ ] **Step 4: Run tests** — `cargo test --locked migration_ && cargo clippy --all-targets -- -D warnings`. Expected: PASS.
- [ ] **Step 5: Commit** — `git commit -s -m "m7: schema v2 with guarded write-triggered migration"`.

**Atomicity/compatibility/security:** migration shares the write transaction; `PRAGMA user_version` is transactional; reads never migrate; Foundation binary refuses v2 (forward-version check already exists).

**Gates:** schema-v2 foundation for all later tasks.

---

### Task 2: G-LIFE — superseded/retracted lifecycle + acyclic supersedes

**Scope:** add `superseded`/`retracted` statuses; supersede requires `superseded_by{id,revision}` + `reason`; retract requires `reason`; implicit atomic `supersedes` relation; acyclic supersedes lineage; lifecycle events in `record_lifecycle`; search visibility default active-only with explicit flags.

**Files:** `crates/cli/src/knowledge.rs` (validation), `crates/cli/src/knowledge_store.rs` (writes, supersedes relation, cycle check), `crates/cli/src/main.rs` (search visibility, lifecycle JSON fields), `crates/cli/src/args.rs` (`--include-superseded`, `--include-retracted`, supersede payload fields), `crates/cli/tests/knowledge.rs` (CLI tests).

**Preconditions:** Task 1 (migration/lifecycle tables).

**Interfaces:** `validate_status` gains `superseded`/`retracted`; new `validate_lifecycle_transition`; `knowledge_store::supersede_record`, `knowledge_store::retract_record`, `knowledge_store::would_create_supersedes_cycle`; `knowledge_store::search` gains visibility filter; `relation_by_id` reused for supersedes lookup.

- [ ] **Step 1: Write failing tests** (unit in `knowledge_store.rs` + CLI in `knowledge.rs`): supersede happy path (history rev1→rev2, supersedes relation, generation+1); retract happy path; cycle rejection (direct A→B then B→A; indirect A→B→C→A); self-reference; invalid superseding revision (exit 3); missing `superseded_by`/`reason`; supersede-of-retracted; double-retract; supersede/retracted on create; stale `--expected-revision` conflict; `relates_to` cycle still allowed; visibility matrix (default active-only × 3 flags × 4 statuses); record retraction never touches relations; filter applies to counts/pagination.
- [ ] **Step 2: Run to verify they fail** — `cargo test --locked supersede_ && cargo test --locked --test knowledge lifecycle_`. Expected: FAIL.
- [ ] **Step 3: Implement** supersede/retract transitions (single transaction: validate target + cycle check, insert revision, append lifecycle event, append implicit `supersedes` relation, bump generation); extend `validate_status`; search visibility filter; JSON fields.
- [ ] **Step 4: Run tests** — focused then full `cargo test --locked`; clippy.
- [ ] **Step 5: Commit** — `git commit -s -m "m7: superseded and retracted record lifecycle with acyclic supersedes lineage"`.

**Atomicity/compatibility/security:** supersede/retract commit in one transaction; rejection leaves no write/generation bump; cycle check inside the transaction; Foundation active/archived behavior byte-identical.

**Gates:** G-LIFE.

---

### Task 3: G-PROV — candidate/confirmed provenance

**Scope:** `confirm_state` ∈ {candidate, confirmed}, default candidate; explicit confirmation via `update`/`link` payload; `confirm` lifecycle events; idempotent re-confirm; orthogonal to verification/freshness.

**Files:** `crates/cli/src/knowledge.rs` (payload validation), `crates/cli/src/knowledge_store.rs` (confirm writes, lifecycle events), `crates/cli/src/main.rs` (confirm_state JSON), `crates/cli/src/args.rs` (payload field), `crates/cli/tests/knowledge.rs`.

**Preconditions:** Task 1 (migration/lifecycle tables). Independent of Task 2.

**Interfaces:** `knowledge_store::confirm_record`, `knowledge_store::confirm_relation`; `confirm_state` derived from `record_lifecycle`/`relation_lifecycle`.

- [ ] **Step 1: Write failing tests:** confirm record/relation → `confirm_state: confirmed`, history shows confirming revision; re-confirm idempotent no-op; confirm with evidence stays `Unverified`; confirm with stale `--expected-revision` → exit 3; source change after confirmation → stays confirmed, evidence stale; no automatic promotion.
- [ ] **Step 2: Run to verify they fail** — `cargo test --locked confirm_`. Expected: FAIL.
- [ ] **Step 3: Implement** confirm transitions (single transaction, lifecycle event + revision/relation); derive `confirm_state`; JSON field.
- [ ] **Step 4: Run tests** — focused then full; clippy.
- [ ] **Step 5: Commit** — `git commit -s -m "m7: candidate/confirmed provenance with immutable audit events"`.

**Atomicity/compatibility/security:** confirmation is explicit-only, by author label (a claim); never maps to `Unverified`→verified or fresh→confirmed.

**Gates:** G-PROV.

---

### Task 4: G-PORT — canonical JSONL export/import

**Scope:** deterministic JSONL export (meta/record/relation lines, canonical orders, null-omission rules, no digest); Markdown-wrapper export (fenced canonical blocks + lossy prose); JSONL import (idempotent, merge policy, resource limits, validate-before-promotion, all-or-nothing); Markdown-wrapper import (extract fenced blocks).

**Files:** `crates/cli/src/knowledge_store.rs` (export reader, import writer, validation), `crates/cli/src/main.rs` (export/import commands, JSONL/Markdown emission), `crates/cli/src/args.rs` (`export`/`import` commands, `--format`, `--file`, `--out`), `crates/cli/tests/knowledge.rs`.

**Preconditions:** Tasks 1–3 (export must serialize lifecycle + provenance fields).

**Interfaces:** `knowledge_store::export_jsonl(conn) -> Result<String, String>`; `knowledge_store::import_jsonl(conn, &str) -> Result<ImportReport, String>`; `knowledge_store::export_markdown(conn) -> Result<String, String>`; reuse `record_from_row`/`relation_by_row` readers and `relation_by_id`; reuse strict JSON parsing from `main.rs`.

- [ ] **Step 1: Write failing tests:** export→import round-trip content-equal (all supported fields); determinism (two exports byte-identical); idempotent re-import (no generation bump); superset append; divergent conflict → exit 3 atomic; malformed/truncated/duplicate → exit 2; over-limit → exit 2; unresolved relation endpoint → exit 2; Markdown export→import round-trip; prose JSON outside fenced blocks ignored; no fenced blocks → exit 2; export read-only (store bytes + generation unchanged).
- [ ] **Step 2: Run to verify they fail** — `cargo test --locked export_ && cargo test --locked --test knowledge port_`. Expected: FAIL.
- [ ] **Step 3: Implement** export (one read transaction, canonical order), import (validate fully, then one write transaction), Markdown wrapper (fenced blocks = payload).
- [ ] **Step 4: Run tests** — focused then full; clippy.
- [ ] **Step 5: Commit** — `git commit -s -m "m7: deterministic JSONL and Markdown-wrapper export/import"`.

**Atomicity/compatibility/security:** import is all-or-nothing; never silently overwrites conflicts; resource limits before any write; export read-only.

**Gates:** G-PORT.

---

### Task 5: G-BACK — backup, verify, restore

**Scope:** SQLite-consistent snapshot backup (`.db` + manifest, no digest); `knowledge verify`; offline restore with binding/version/verification preconditions and `--overwrite`; temp-file + atomic rename; fail-closed on detected corruption.

**Files:** `crates/cli/src/knowledge_store.rs` (backup snapshot via `VACUUM INTO`/Online Backup API, verify, restore), `crates/cli/src/main.rs` (commands), `crates/cli/src/args.rs` (`backup`/`restore`/`verify`, `--out`/`--file`/`--overwrite`), `crates/cli/tests/knowledge.rs`.

**Preconditions:** Task 1 (schema v2 for manifest `schema_version`). Parallelizable with Tasks 2–4.

**Interfaces:** `knowledge_store::backup_to(conn, path) -> Result<(), String>`; `knowledge_store::verify_store(conn) -> Result<(), String>`; `knowledge_store::restore_from(path, overwrite) -> Result<(), String>`; reuse `reject_link`/location-security and `knowledge_repo`.

- [ ] **Step 1: Write failing tests:** backup→verify→restore content-equal; backup read-only (store unchanged); manifest binding/version/count checks; restore different repo/root → exit 3; restore over existing without `--overwrite` → exit 3; corrupt backup (flipped byte/truncated) → exit 3, live store intact; forward-version backup → exit 3; symlink at target → exit 3; tampered-but-structurally-valid backup (no digest) → may pass (documented).
- [ ] **Step 2: Run to verify they fail** — `cargo test --locked backup_ && cargo test --locked --test knowledge backup_`. Expected: FAIL.
- [ ] **Step 3: Implement** backup (consistent snapshot + manifest), verify (integrity_check + structural), restore (preconditions + temp + atomic rename).
- [ ] **Step 4: Run tests** — focused then full; clippy.
- [ ] **Step 5: Commit** — `git commit -s -m "m7: offline backup, verify, and restore with integrity boundaries"`.

**Atomicity/compatibility/security:** restore atomic against interruption (temp + rename); Windows overwrite atomicity honestly limited; no cross-repo restore; no authenticity claim.

**Gates:** G-BACK.

---

## Task dependency map

```
Task 1 (schema v2 migration)
├── Task 2 (G-LIFE)
├── Task 3 (G-PROV)          [parallel with Task 2]
├── Task 4 (G-PORT)          [after Tasks 2+3: export needs lifecycle+provenance fields]
└── Task 5 (G-BACK)          [after Task 1; parallel with Tasks 2–4]
```

Full local gate after all tasks: `cargo fmt --all -- --check && cargo check --locked && cargo clippy --all-targets -- -D warnings && cargo test --locked` (498+ tests) plus real CLI smoke for `export`/`import`/`backup`/`restore`/`verify` and the new flags. Cross-platform CI only when separately authorized.

## STOP checkpoint

STOP after all five tasks pass their gates and the full local suite is green. Report per-task commits, gate→task map (G-LIFE→T2, G-PORT→T4, G-BACK→T5, G-PROV→T3, schema-v2→T1), CLI smoke evidence, and remaining risks (Windows overwrite atomicity, import performance bounds, G-PROV blueprint). Do not declare M7 CLOSED; await reviewer GO before any further stage.
