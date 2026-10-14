# M4 Safe Incremental Syntax Refresh — Implementation Plan (rev 1)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `index --repo` reuses verified per-file facts from the previous SQLite snapshot so unchanged files are never reparsed, with full-rebuild parity and M3 invariants intact.

**Architecture:** `build_full` stays the fallback and parity oracle. Task 2 extracts its staging setup, per-file ingest, and finalize/promote steps into shared helpers so full and incremental differ only in the parse-vs-reuse decision plus mode diagnostics. Incremental classifies each walked file with an internal `diff_files` pass, copies verified-unchanged facts set-based (`INSERT INTO ... SELECT` over an ATTACHed live database inside the staging transaction, so reuse reads one consistent old revision), and parses only the rest. Dispatcher `build()` picks full / incremental / fallback-full. No new database, daemon, watcher, dependency, or public test-only API.

**Tech Stack:** Rust, std-only CLI, `rusqlite` 0.40.2 bundled (existing), tree-sitter via existing `syntax.rs`, existing `scan`/`store`/`git`/`ignore` modules.

**Spec:** User-supplied M4 brief in session (file-level incremental; `index` auto-incremental; `--full` forces full rebuild; no `refresh` command; correctness → simplicity → performance) + 8 review corrections in session (read-txn isolation; set-based copy; shared pipeline; controlled race test; binary-only test placement; runnable test commands; cold-vs-recovery diagnostics; measured no-op path). Fallback table, JSON shape, and parity rules in Tasks 3–5 below travel with the plan.

## Global Constraints

- Baseline HEAD `1d3b2ff`, tree clean before Task 1; M0–M3 CLOSED — Task 1 adds store helpers, Task 2 refactors `build_full` with zero behavior change; nothing else touches M1–M3 logic.
- No new dependencies (`cargo check --locked` green with no third-party lockfile churn). No schema migration (`SCHEMA_VERSION` stays 2).
- No lib target and no public API created for tests: deep assertions live in unit tests inside `src/`; integration tests use only the CLI binary plus string-normalized JSON.
- Never `mtime`-only evidence; never assume atomic filesystem snapshots; never report stale as current; never mix two revisions in one response.
- CLI stays std-only; JSON envelopes stay deterministic; pre-existing keys byte-compatible.
- Every new behavior lands with a RED test first (Task 5 parity/integration guards are the stated exception).
- Commit per task; never push without the human partner's explicit approval; never start M5.

## Review Focus

1. File changes between the reuse-verification read and the staging commit → final verification must yield `stale`, never `fresh`. (Pinned in Task 3 by a controlled two-read test through the real `diff_files` + `verify_snapshot_content` functions.)
2. Reuse reads must see exactly one old revision even if a concurrent rebuild promotes mid-refresh. (Pinned in Task 6 by a held-read-transaction test; the refresh path inherits it because the ATTACHed copy runs inside the staging transaction.)
3. Path replaced by a symlink/junction at the same location → old facts must not be reused and the link must not be followed. (Pinned in Task 6.)
4. Ignore-rule edit that excludes previously indexed files → `config-changed` forces `fallback-full`; now-ignored facts are never silently reused. Case-only rename on Windows → exactly one removed + one added, no duplicates. (Pinned in Task 6; Windows CI proves the rename.)
5. No-op cost: if the Task 7 measurement shows stage/promote dominating no-op time, only then a short-circuit may skip staging+promotion under the identical verification gate and JSON shape — never speculatively. (Pinned in Task 7.)

---

### Task 1: Minimal store helpers — id map, attach, set-based file copy

**Files:**
- Modify: `crates/cli/src/store.rs` (append near `files_of` / `promote_staging`)
- Test: `crates/cli/src/store.rs` tests module (unit; `testutil::tmpdir` already used there)

**Interfaces:**
- Consumes: existing row structs and `insert_*` (unchanged), `BTreeMap` (add the import if missing).
- Produces (exact signatures, `pub` — plain binary-crate items, no lib API):
  - `pub fn file_id_map(conn: &Connection, snapshot_id: i64) -> Result<BTreeMap<String, (i64, String)>, String>` — path → (rowid, digest), one query, ordered by path.
  - `pub fn attach_src(conn: &Connection, path: &Path) -> Result<(), String>` — `ATTACH '<escaped>' AS src` with the same single-quote escaping `promote_staging` uses. (`promote_staging` itself is left untouched.)
  - `pub fn detach_src(conn: &Connection) -> Result<(), String>` — `DETACH DATABASE src`.
  - `pub fn copy_file_row(staging, new_snapshot_id, old_snapshot_id, old_file_id) -> Result<Option<i64>, String>` - the files row only; None when the source row is absent. Plus `create_file_map` / `record_file_map` (session-local TEMP old-new id map, no schema migration) and `copy_mapped_facts` (three joined bulk INSERT..SELECTs with counts; per-file filtered child copies would scan the whole snapshot once per file without a file_id index - quadratic at MMSP scale).
- Read-isolation argument (one comment at the call site in Task 3, not here): the copy runs inside the staging transaction, so all `src` reads see a single consistent revision even if a concurrent promote commits mid-refresh.

- [ ] **Step 1: Write the failing tests** in the store.rs tests module: `file_map_bulk_copy_roundtrip` - file-backed live db with snapshots + files + symbol/import/call rows via existing `insert_*`; file-backed staging db; `attach_src`, new snapshot, `copy_file_row` + `record_file_map` per file, `copy_mapped_facts`, `detach_src`; assert exact copies, totals, and searchability. (File-backed because ATTACH needs paths.)
- [ ] **Step 2: Run it to verify it fails.** Run: `cargo test --locked --bin riltra -- file_map_bulk_copy_roundtrip`. Expected: FAIL, compile error (functions not defined).
- [ ] **Step 3: Implement the four functions** in `crates/cli/src/store.rs`.
- [ ] **Step 4: Run the test to verify it passes.** Run: `cargo test --locked --bin riltra -- file_map_bulk_copy_roundtrip`. Expected: PASS.
- [ ] **Step 5: Commit.** `git add crates/cli/src/store.rs; git commit -s -m "feat: set-based per-file fact copy helpers for incremental reuse"`.

### Task 2: Shared pipeline helpers (pure refactor, zero behavior change)

**Files:**
- Modify: `crates/cli/src/index.rs` (`build_full` body only)
- Test: none new — the whole existing suite is the test.

**Interfaces:**
- Consumes: everything `build_full` uses today.
- Produces (all private `fn`, same file):
  - `fn ingest_file(conn: &rusqlite::Connection, snapshot_id: i64, item: &scan::PlannedFile, summary: &mut IndexSummary) -> Result<(), String>` — the current per-file loop body verbatim (skip counting, content gates, parse, inserts). Callers pass `&tx` (deref coercion, as today). Confirm `PlannedFile` field names (`rel`, `full`, `action`) in `scan.rs` before writing the signature.
  - `struct StagingMeta { canon_root: PathBuf, git_state: <existing fingerprint type>, cfg: String, pre: String }` (reuse the exact `git::fingerprint` return type; do not redefine it) plus `fn prepare_staging(home: &Path, name: &str, root: &Path) -> Result<(Connection, PathBuf, PathBuf, StagingMeta), String>` returning the staging connection and `(db_path, staging_path)` — everything before transaction creation (dir creation, staging-lock removal, canonicalize, fingerprints, `open_live(staging)`). The 3-line transaction-open + `insert_snapshot` stays inline at each caller (a returned `Transaction` cannot borrow the returned `Connection`; no closure framework to dodge this).
  - `fn finish_snapshot_tx(tx: rusqlite::Transaction<'_>, snapshot_id: i64, summary: &mut IndexSummary, meta: &StagingMeta)` — post fingerprint, `verify_snapshot_content(&tx)`, `fresh`-iff-`pre == post`-and-verified, `finish_snapshot`, `tx.commit()`, fill `status_at_write`/`post_fingerprint`. Takes `tx` by value (commit consumes it) — plus `fn promote_live(db: &Path, staging: &Path)` — open live with corrupt recovery, `promote_staging`, staging cleanup. Split in two so callers drop the staging connection between them: on Windows an open handle blocks staging-file removal.
- `build_full` becomes: `prepare_staging` → open tx + `insert_snapshot` → loop `ingest_file` → `finish_snapshot_tx` → `drop(conn)` → `promote_live`. Behavior identical.

- [ ] **Step 1: Extract the three helpers**; rewrite `build_full` as the thin orchestration. No logic edits, no counter renames.
- [ ] **Step 2: Run the full suite.** Run: `cargo test --locked`. Expected: all green, zero failures.
- [ ] **Step 3: Commit.** `git add crates/cli/src/index.rs; git commit -s -m "refactor: shared staging pipeline helpers, no behavior change"`. Any failure in Step 2 rejects this task; do not proceed to Task 3.

### Task 3: Incremental core — diff, reuse, dispatcher, diagnostics

**Files:**
- Modify: `crates/cli/src/index.rs`
- Test: `crates/cli/src/index.rs` tests module (unit only)

**Interfaces:**
- Consumes: Task 1 helpers, Task 2 helpers, `content_diff`, `verify_snapshot_content`, `config_fingerprint`, `input_fingerprint`, `git::fingerprint`, `store::PARSER_VERSION`, `store::grammar_versions`.
- Produces:
  - `pub struct PhaseMs { pub diff: u64, pub ingest: u64, pub verify: u64, pub promote: u64 }` (milliseconds via `std::time::Instant`).
  - `IndexSummary` gains `pub mode: String`, `pub reused: usize`, `pub reparsed: usize`, `pub added: usize` (reparsed files whose path was not stored), `pub removed: usize`, `pub fallback_reason: String`, `pub timings_ms: PhaseMs`. Kept meanings: `processed` = files contributing facts (`reused + reparsed`); full builds set `reused: 0, added: 0, removed: 0, reparsed: processed`.
  - `struct FileDiff { unchanged: Vec<(String, i64)>, ingest_rels: Vec<String>, removed: Vec<String> }` (private; paths owned so no lifetime coupling to the plan).
  - `fn diff_files(stored: &BTreeMap<String, (i64, String)>, plan: &[scan::PlannedFile], canon_root: &Path) -> FileDiff` (private): for each `Parse` plan item passing the same content gates as `ingest_file`, read bytes and compare `digest(bytes)` to the stored digest — equal path+digest → `unchanged` (carry old file_id); eligible-but-different/missing → `ingest_rels`; stored paths with no current eligible item → `removed`. (Changed files are read once more by `ingest_file`; hashing-only for unchanged files is the intended cost profile — note it, don't optimize it.)
  - `pub fn build(home: &Path, name: &str, root: &Path, full: bool) -> Result<IndexSummary, String>` — dispatcher; `full == true` → `build_full` with `mode: "full"`.
  - `build_full` keeps its signature; fills the new fields per above.
- Fallback table (first match wins):
  - live db file absent → `build_full`, `mode: "full"`, reason `"cold-start"`.
  - `full == true` → `build_full`, `mode: "full"`, reason `"forced-full"`.
  - live db present but unreadable, or no snapshot row → `build_full`, `mode: "fallback-full"`, reason `"db-unreadable"` / `"no-snapshot"` (recovery, distinct from cold start).
  - stored `root != canon_root` → `"root-moved"`; `parser_version` mismatch → `"parser-changed"`; `grammar_versions` mismatch → `"grammar-changed"`; `config_fingerprint` mismatch → `"config-changed"`; live `head` mismatch → `"head-changed"`; live `branch` mismatch → `"branch-changed"` — all `mode: "fallback-full"` via `build_full`.
  - dirty worktree or untracked-count change with same head/branch → **incremental** (digests decide). Previous `status_at_write == "stale"` → still incremental (each reused file independently verified).
- Incremental flow: open live + `file_id_map` under one held read transaction; `plan_walk`; `diff_files`; `prepare_staging` + tx + `insert_snapshot` (fresh nonce); `attach_src` on the staging connection (copy reads then share the staging transaction's consistent view of `src` — one revision even under concurrent promote; say so in one comment); per unchanged file `copy_file_row` + `record_file_map`; per `ingest_rels` item `ingest_file`; one `copy_mapped_facts` bulk pass; detach after commit; `finish_snapshot_tx` + drop + `promote_live` (Task 2 split). ATTACH/read failure of live mid-refresh → return `Err` (live snapshot untouched; operator retries, `--full` always available).

- [ ] **Step 1: Write the failing tests** (index.rs tests module):
  - `noop_refresh_reparses_zero_files`: build, `build()` unchanged → `mode == "incremental"`, `reparsed == 0`, `added == 0`, `removed == 0`, `reused == processed`, `status_at_write == "fresh"`, reason `"none"`.
  - `one_file_delta_reparses_only_that_file`: two files, build, change one → `reparsed == 1`, `reused == 1`, new symbol present, untouched facts intact.
  - `add_remove_rename_yield_exact_facts`: add + delete + rename across a rebuild → deleted symbol absent via `search_symbols` on the new snapshot, renamed path present exactly once.
  - `repeated_changes_do_not_duplicate_facts`: same file changed three times with rebuilds between → `files_of` length and `search_symbols` totals stable.
  - `empty_repo_stays_fresh_and_empty_file_is_eligible`: empty dir → `fresh`, `processed == 0`; add zero-byte `.py` → rebuilt as eligible (`processed == 1`).
  - `reuse_window_mutation_is_caught` (Review Focus 1): build v1; compute `diff_files` from the live snapshot (all unchanged); mutate one file; run the real `verify_snapshot_content` against v1 → `false`; recomputed `diff_files` marks it for ingest. This brackets the refresh race window with the actual functions: anything changing between the two reads surfaces as `stale`, never `fresh`.
- [ ] **Step 2: Run them to verify they fail.** Run each (single filter per invocation, `--` form):
  `cargo test --locked --bin riltra -- noop_refresh_reparses_zero_files`
  `cargo test --locked --bin riltra -- one_file_delta_reparses_only_that_file`
  `cargo test --locked --bin riltra -- add_remove_rename_yield_exact_facts`
  `cargo test --locked --bin riltra -- repeated_changes_do_not_duplicate_facts`
  `cargo test --locked --bin riltra -- empty_repo_stays_fresh_and_empty_file_is_eligible`
  `cargo test --locked --bin riltra -- reuse_window_mutation_is_caught`
  Expected: FAIL, compile error (`build` / `diff_files` not defined).
- [ ] **Step 3: Implement** dispatcher, table, `diff_files`, copy loop, summary fields, `PhaseMs` per the interfaces above.
- [ ] **Step 4: Run the tests to verify they pass.** Run: the six commands from Step 2, then `cargo test --locked`. Expected: all green.
- [ ] **Step 5: Commit.** `git add crates/cli/src/index.rs; git commit -s -m "feat: file-level incremental indexing with verified set-based reuse"`.

### Task 4: CLI `--full` flag and mode/counter JSON

**Files:**
- Modify: `crates/cli/src/args.rs`, `crates/cli/src/main.rs` (`cmd_index`, usage text)
- Test: `crates/cli/src/args.rs` tests module; `crates/cli/tests/index.rs` (CLI-level only)

**Interfaces:**
- Consumes: Task 3 `build()` and new `IndexSummary` fields.
- Produces: `Command::Index { repo: String, full: bool }`; `index --repo <name> [--full]` — match exact token `--full` before `take_value` (a bare `--full` must not swallow the next arg; `--full=x` is a usage error); usage line becomes `index --repo <name> [--full]`; `cmd_index` calls `build(&home, name, root, full)`; JSON appends `"mode"`, `"reused"`, `"reparsed"`, `"added"`, `"removed"`, `"fallback_reason"`, `"timings_ms":{"diff":..,"ingest":..,"verify":..,"promote":..}` after the existing `"files"` object — pre-existing keys untouched. (Existing `Command::Index { .. }` patterns keep compiling.)

- [ ] **Step 1: Write the failing tests**: args test `parses_index_full_flag` (`--full` → `true`; absent → `false`; `--full=x` and `index bogus` err); integration `cli_noop_reports_zero_reparsed` (init + index + index → second JSON `"mode":"incremental"`, `"reparsed":0`, `"fallback_reason":"none"`); integration `cli_full_flag_forces_full` (second run with `--full` → `"mode":"full"`, `"fallback_reason":"forced-full"`); integration `cli_cold_start_reports_cold_start` (first index on fresh home → `"mode":"full"`, `"fallback_reason":"cold-start"`).
- [ ] **Step 2: Run them to verify they fail.** Run: `cargo test --locked --bin riltra -- parses_index_full_flag` then `cargo test --locked --test index -- cli_noop_reports_zero_reparsed` then `cargo test --locked --test index -- cli_full_flag_forces_full` then `cargo test --locked --test index -- cli_cold_start_reports_cold_start`. Expected: FAIL (no `full` field / no mode keys).
- [ ] **Step 3: Implement** the args arm, `cmd_index` wiring, and JSON keys.
- [ ] **Step 4: Run the tests to verify they pass.** Run: the Step 2 commands, then `cargo test --locked`. Expected: all green.
- [ ] **Step 5: Commit.** `git add crates/cli/src/args.rs crates/cli/src/main.rs crates/cli/tests/index.rs; git commit -s -m "feat: index --full flag and incremental mode JSON"`.

### Task 5: Parity gate — incremental vs independent full rebuild

**Files:**
- Test: unit test in `crates/cli/src/index.rs` tests module (deep fact comparison — has crate-internal DB access); one integration test in `crates/cli/tests/index.rs` (CLI search-output parity on normalized JSON).
- Modify: none expected (on FAIL fix the source, never loosen normalization).

**Interfaces:**
- Consumes: Task 1–3; `files_of`, `search_symbols`.
- Unit normalization (exact): drop `snapshots.id/nonce/created_unix`, all table rowids and `file_id`s (re-key facts by file path); compare ordered vectors of file `(path, language, bytes, digest, errors)`, symbol `(path, kind, name, span, byte offsets, provider, grammar, language)`, import `(path, text, span, bytes, provider, grammar, language)`, call `(path, name, span, bytes, unresolved, provider, grammar, language)`.
- Integration normalization (exact): run the same `search --repo --symbol` for three fixed symbols against the incremental and full homes; replace the two `nonce` values, `"id":N`, and `"created_unix":N` occurrences with constants; assert full-string equality. Assert the fact-bearing index counters (`discovered/processed/definitions/imports/calls/errors/skips/unsupported`) equal across both runs (`mode`/`reused`/`reparsed`/`timings_ms` intentionally differ).

- [ ] **Step 1: Write `incremental_matches_independent_full_rebuild`** (unit): fixture with several files across supported languages + one binary + one large + one unsupported; full build into home A; three evolution stages (modify, add, delete, rename) with `build()` into home B after each; final `build_full` into home C on the same final tree; assert normalized dumps B == C. Write `cli_search_parity_full_vs_incremental` (integration) per the integration normalization.
- [ ] **Step 2: Run them.** Run: `cargo test --locked --bin riltra -- incremental_matches_independent_full_rebuild` then `cargo test --locked --test index -- cli_search_parity_full_vs_incremental`. Expected: PASS on first run — guard tests; any FAIL stops the line (return to Task 3; never adjust normalization to hide differences).
- [ ] **Step 3: Commit.** `git add crates/cli/src/index.rs crates/cli/tests/index.rs; git commit -s -m "test: incremental vs full rebuild parity gate"`.

### Task 6: Tier A remainder — isolation, policy, fallback, crash

**Files:**
- Test: `crates/cli/src/index.rs` tests module (mechanism-level) and/or `crates/cli/tests/index.rs` (behavior-level); one place per test.
- Modify: `crates/cli/src/index.rs` only if a test exposes a defect (minimum fix, same commit as the test).

**Interfaces:** Consumes Tasks 1–4. Every test below runs through `build()` (the new path); each pins one Review Focus line or spec §5 item.

- [ ] **Step 1: Write the failing tests** (RED on behavior; a PASS-first-run marks a guard — say so in the test comment):
  - `reuse_reads_see_single_revision` (unit, Review Focus 2 mechanism): open live, hold a read transaction, promote a new snapshot from a second connection, assert the held transaction still reads the old revision (old nonce, old file set). The refresh path inherits this via the ATTACH-in-transaction copy — no separate refresh-level hook test.
  - `symlink_at_known_path_is_not_reused` (Review Focus 3): replace indexed `a.py` with a symlink to another file → rebuild drops its facts, follows nothing.
  - `incompatible_snapshot_falls_back_with_reason`: fabricated parser version, fabricated grammar string, ignore-file edit, HEAD change, branch change → each yields `mode == "fallback-full"` with exactly `"parser-changed"` / `"grammar-changed"` / `"config-changed"` / `"head-changed"` / `"branch-changed"`; dirty-only change with same HEAD stays `"incremental"`.
  - `ignored_files_are_never_reused`: index, then add an ignore rule excluding an indexed file → `fallback-full` / `"config-changed"` (never silent reuse of now-ignored facts).
  - `stale_snapshot_still_reuses_verified_files`: force the previous snapshot stale via an unrelated added file, remove that file, rebuild → `reused > 0`, result `fresh`.
  - `ineligible_transition_drops_facts`: indexed `a.py` turned binary / oversized / unreadable → facts gone, correct skip counter increments, verdict honestly `stale`-or-`fresh` per content (never stale facts served).
  - `crash_leftover_and_concurrent_search_stay_consistent`: partial/corrupt staging file present + `search_symbols` during `build()` → last whole snapshot intact, responses never mix revisions (reuse M3's two-connection pattern).
  - `two_repos_stay_isolated_under_incremental`: interleaved `build()` on two repos → facts and counters never cross.
  - `case_only_rename_has_no_duplicates` (guard; Windows CI proves): `a.py` → `A.py` → exactly one removed + one added, search totals exact.
- [ ] **Step 2: Run them.** Run: `cargo test --locked`. Expected: new tests fail on real defects (implement the minimum `index.rs` fix each) or pass as guards.
- [ ] **Step 3: Re-run to green.** Run: `cargo test --locked` then `cargo clippy --all-targets -- -D warnings`. Expected: all green, clippy clean.
- [ ] **Step 4: Commit.** `git add -A; git commit -s -m "feat: M4 Tier A policy, fallback, race, and isolation coverage"`.

### Task 7: Benchmark, Tier B, no-op verdict, docs/m4-validation.md, gate

**Files:**
- Create: `docs/m4-validation.md` (mirror `docs/m3-validation.md`: scope, architecture delta, Tier A summary, Tier B measurements, gates).
- Modify: `crates/cli/src/index.rs` + tests only if the no-op verdict below says so.

**Interfaces:** Consumes Tasks 1–6. Tier B uses MMSP strictly read-only; the one-file delta uses an isolated temp copy, never the original tree. Paired runs on the same machine/config, two repetitions each: (A) `--full` — wall time, peak RSS (M3 sampling technique), parsed files, DB bytes; (B) no-op incremental — wall time, `reparsed == 0` proof, phase `timings_ms`; (C) one-file delta on the isolated copy — wall time, only-target-reparsed proof, parity vs full rebuild of the same copy; (D) search/freshness latency before/after. Never quote M3's 215 s / 357 MB as M4 speedup without the paired rerun.
- No-op verdict (Review Focus 5): read the `verify`/`promote` phase split from run B. If stage/promote dominates and skipping them is safe, implement the short-circuit ONLY under all of: `reparsed == 0 && added == 0 && removed == 0`, the identical `verify_snapshot_content` gate passes, JSON shape unchanged, old snapshot id/nonce reported untouched (never rewrite `status_at_write`). Pin with `noop_shortcircuit_keeps_old_snapshot`. Default: no short-circuit.

- [ ] **Step 1: Run Tier B measurements** (A–D, two repetitions) and the no-op verdict; write exact numbers plus the bottleneck reading and the fallback behavior observed into `docs/m4-validation.md`.
- [ ] **Step 2: Run the full gate.** Run: `cargo fmt --all -- --check` then `cargo check --locked` then `cargo clippy --all-targets -- -D warnings` then `cargo test --locked`. Expected: all green.
- [ ] **Step 3: Commit.** `git add docs/m4-validation.md crates/cli/src/index.rs; git commit -s -m "docs: M4 validation summary and Tier B benchmark"`. Then report HEAD, file list, RED→GREEN evidence, parity result, before/after numbers, gate PASS/FAIL/BLOCKED, open findings. STOP — no push without approval, no M5.

## Self-Review

1. **Spec coverage:** §1 parity/counters/latency → Tasks 3–5, 7. §2 DRY/KISS → Task 1 (one map reader + ATTACH copy, no Vec-per-fact helpers), Task 2 (shared prepare/ingest/finalize — one pipeline, not two), Task 7 (no committed harness), zero new deps. §3 ten-step flow → Task 3 (steps 1–8, 10; fresh nonce kept) + Task 3 race test (step 9). §4 invariants → Tasks 3 (nonce/schema-2/read-txn untouched, FTS via triggers on fresh staging), 5 (span/digest/provenance/FTS parity), 6 (rename/symlink/ineligible/no-exec). §5 Tier A 1–10 → Tasks 3 (1, 2, 3, 5), 5 (parity), 6 (4, 6, 7, 8, 9, 10). §6 Tier B A–D → Task 7. §7 counters → Tasks 3–4 (`mode/reused/reparsed/added/removed/fallback_reason/timings_ms`). §8 exclusions → Global Constraints. §9 acceptance → Task 7 gate + report.
2. **Step scan:** test steps name exact fns and assertions; code steps name exact signatures and reason strings; verification steps use only the single-filter `--` form (`cargo test --locked --bin riltra -- <one>` / `--test index -- <one>`) or bare `cargo test --locked` — every command runnable as written. Fixed inline.
3. **Type consistency:** `build(home: &Path, name: &str, root: &Path, full: bool)`; `file_id_map` → `BTreeMap<String, (i64, String)>`; `copy_file_row` / `copy_mapped_facts` / `CopiedTotals`; summary `mode/reused/reparsed/added/removed/fallback_reason/timings_ms: PhaseMs{diff,ingest,verify,promote}`; `FileDiff{unchanged: Vec<(String,i64)>, ingest_rels: Vec<String>, removed: Vec<String>}` — identical in Tasks 1–4.
4. **Review Focus:** five lines above, each pinned to Task 3, 6, or 7.
5. **Proportion:** decisions and tests stated, bodies left to the implementer from signatures plus existing code; the ATTACH-in-transaction isolation argument recorded where it matters (Tasks 1, 3, 6).
