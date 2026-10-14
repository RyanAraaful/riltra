# M4 Validation Summary

Scope: file-level incremental syntax refresh on top of the M3 SQLite
snapshot store. `index --repo` reuses digest-verified per-file facts and
parses only what changed; `--full` forces a full rebuild. No new database,
daemon, watcher, dependency, or schema migration (`SCHEMA_VERSION` stays 2).

## Architecture delta (as built)

- `store.rs`: `file_id_map` (path → rowid + digest), `attach_src` /
  `detach_src`, `copy_file_row` (files row only, `None` on vanished source
  rows), session-local `TEMP` old→new id map (`create_file_map` /
  `record_file_map`, no migration, invisible to other connections), and
  `copy_mapped_facts` (three joined bulk `INSERT..SELECT`s + errors
  aggregate, counts via `changes()`).
- `index.rs`: shared pipeline helpers (`prepare_staging`, `ingest_file`,
  `finish_snapshot_tx`, `promote_live`) used by both pipelines; internal
  `diff_files` classification (unchanged / ingest / removed / dropped);
  `build_incremental` (reuse + bulk copy + parse-the-rest);
  `build()` dispatcher with explicit fallback reasons (`cold-start`,
  `forced-full`, `db-unreadable`, `no-snapshot`, `root-moved`,
  `parser-changed`, `grammar-changed`, `config-changed`, `head-changed`,
  `branch-changed`, `none`).
- Reuse safety: classification inputs from one live read transaction; copy
  reads share the staging transaction's snapshot over the ATTACHed live
  database with `snapshot_id`-predicated statements plus zero-row fallback
  to ingest; the shared `verify_snapshot_content` gate and atomic promotion
  are unchanged. Fresh nonces per revision; FTS5 follows triggers on a
  fresh staging database, so removed facts can never linger.
- CLI: `index --repo <name> [--full]`; JSON gains `mode` (`full` /
  `incremental` / `fallback-full`), `reused`, `reparsed` (files actually
  parsed — hashes don't inflate it), `added`, `removed`,
  `fallback_reason`, and `timings_ms` (`diff` / `ingest` / `verify` /
  `promote`). All pre-existing keys untouched.

## Tier A — deterministic fixtures

155 tests green (106 unit + 16 store + 21 index integration + 12 cli):
cold start is a full parse; no-op refresh parses 0 files; one-file deltas
parse only that file; add/remove/rename exact; repeated changes duplicate
nothing (FTS totals stable); binary/large/unsupported/symlink/ignore follow
M2/M3 policy through the new path; every incompatibility falls back with
its exact reason while dirty-with-same-HEAD stays incremental; a stale
previous snapshot still yields verified reuse; mid-window mutations surface
as `stale`, never silent-fresh; crash leftovers and corrupt live databases
recover with explicit reasons; concurrent search never mixes revisions
(single-revision reads pinned); two repositories stay isolated.

Parity gate: three evolution stages (modify, add+delete, rename) through
`build()` equal an independent `build_full` of the same final tree —
files, definitions, imports, unresolved call candidates (names, kinds,
spans, byte offsets, digests, provenance) — plus CLI search-output parity
after volatile-field normalization. Any unexplained fact difference would
be FAIL; none found.

## Tier B — real repository (MMSP, read-only, local only)

Target: MMSP clone, `main/d62f06a`, porcelain 0 throughout. Nothing
pulled, checked out, built, or modified; one-file delta used an isolated
temp copy, never the original tree. Debug binary, same machine/config,
two repetitions each.

- **A. Full (`--full`):** A1 488.4 s peak 584.5 MB; A2 210.8 s
  (peak sampler under-read at 50.2 MB — artifact, not a reduction).
  A1 ran contended with other foreground work; A2 matches the M3 215 s
  baseline. Both exit 0, `fresh`, DB 76,382,208 bytes holding one snapshot.
  Counts byte-identical to M3 (15,176 discovered / 11,765 processed /
  170,586 definitions / 64,954 imports / 562,384 call candidates /
  152,543 errors; 291 large / 492 binary / 2,628 unsupported).
  Phase split (A2): diff 308 ms, ingest(parse) 201 s, verify 2.7 s,
  promote 2.5 s — parsing dominates.
- **B. No-op incremental (same home):** 15.8 s / 15.3 s, `reused` 11,765,
  `reparsed` 0, `fresh`, identical counts. Phase split: diff ~1.9 s,
  ingest(copy) ~6.3 s, verify ~2.7 s, promote ~3.9 s. About **13.6×**
  faster than the paired full (210.8 s → ~15.5 s).
- **C. One-file delta (isolated 277-file subset copy):** full 4.9 s cold /
  2.85 s warm; no-op 2.61 s / 2.65 s (`reparsed` 0, `reused` 270);
  one-file delta 2.53 s (`reparsed` 1, `reused` 269); counters
  byte-identical across all runs. At this scale parsing is cheap so the
  wall-time gap is modest — the hard guarantees are single-file reparsing
  and fact parity, both proven.
- **D. Search/freshness:** `PatchMatch` 2.50/2.48 s (`total` 29, exact
  spans as M3), `index status` 2.38/2.35 s `fresh` — M3 latencies
  unchanged, no query impact.
- **No-op verdict:** promote (~3.9 s of ~15.5 s) does not dominate; copy
  inserts do. The no-promotion short-circuit is declined — a second code
  path to save a quarter of the time is not worth it.

## RED → GREEN evidence (selection)

- `noop_refresh_reparses_zero_files` + five core tests: compile-FAIL
  before `build()`/`diff_files` existed, green after.
- Tier B no-op burned 22 min CPU-bound on the first copy design
  (per-file filtered child copies: one full snapshot scan per file,
  quadratic without a `file_id` index). Replaced with the TEMP-map bulk
  copy under a new RED test; 15.5 s after. No schema migration needed.
- `DETACH DATABASE src` inside a write transaction that read it reports
  locked (proven by throwaway matrix test, since deleted): detach runs
  after commit on the single-use staging connection.
- The Task 2 refactor exposed a real Windows bug (open staging handle
  blocked staging-file removal, breaking corrupt-staging recovery):
  pipeline split into finish-then-drop-then-promote, caught by the
  pre-existing regression test.
- Two test-expectation corrections during Task 6 (ledgered): dirty trees
  are build-time `fresh` + query-time `stale` (M3 dirty semantics), and a
  rebuild always incorporates source so stale labels were fabricated via
  `UPDATE` to test the reuse policy exactly.

## Gates

- `cargo fmt --all -- --check`, `cargo check --locked`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test --locked`:
  green locally (155/155). CI Windows/Ubuntu/macOS pending push approval.
- No MMSP path in CI configuration or committed tests; Tier B outputs
  live outside the repo (private blueprint copy).

## Open findings (honest, not fixed in M4)

- Full-build wall time varies with machine contention (A1 488 s vs A2
  211 s); paired A2/B numbers are the honest comparison.
- DB file is 76 MB vs M3's 81 MB with identical row counts — page-packing
  variance, not investigated (same schema, same facts).
- Unreadable-file transition is covered on unix only (`cfg(unix)` chmod
  test); Windows lacks a portable unreadable-file fixture. The ingest
  path is shared, so behavior is identical by construction.
- `ingest_rels` in `FileDiff` and `build_full` carry `#[allow(dead_code)]`
  (test-asserted diff output / M3-compatible oracle); production routes
  the single ordered walk off the unchanged set.
- M5+ explicitly out of scope; no semantic indexing added.
