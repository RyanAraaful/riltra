# M3 Validation Summary

Scope: durable SQLite code graph with atomic rebuilds and explicit freshness
(`fresh` / `stale` / `unknown` with reasons). No incremental indexing, no
semantic providers, no callers/callees/trace/impact, no knowledge graph, no
MCP. Calls remain unresolved candidates; symbol rowids are per-snapshot and
never permanent cross-revision identities.

## Architecture (as built)

- `store.rs`: `rusqlite` 0.40 + bundled SQLite (FTS5 verified per OS by test).
  Schema v1 (`PRAGMA user_version`): `meta`, `snapshots`, `files`, `symbols`,
  `imports`, `calls`, `symbols_fts` (external-content FTS5 with triggers).
  One database file per repository under the local Riltra home
  (`<home>/index/<name>.db`); forward-incompatible versions are rejected,
  garbage files fail to open.
- `index.rs`: full rebuilds stream internal parser facts straight into one
  staging transaction (never buffered as JSON), then copy into the live
  database inside a second single transaction — readers see the old snapshot
  or the new one, never a mix. Input fingerprints (git state + config) are
  compared before and after; mixed states are recorded stale, never fresh.
- Freshness is evaluated at query time against live state with named reasons
  (`dirty`, `head-changed`, `branch-changed`, `parser-changed`,
  `grammar-changed`, `config-changed`, `file-added`, `file-removed`,
  `content-changed`, `changed-during-index`, `root-moved`); anything
  unverifiable is `unknown`, never `fresh`.
- CLI: `index --repo` (build), `index status --repo` (freshness + FTS flag),
  `search --repo --symbol [--limit] [--offset]` (FTS5 prefix first, LIKE
  substring fallback, deterministic path order, pagination with totals,
  provenance per hit). Exit codes `0` ok, `2` usage, `3` repository/data.

## Tier A — self-contained fixtures (CI)

Twelve integration tests plus unit coverage prove: build→close→reopen→search
consistency; dirty-without-HEAD detection with old data still served as stale;
rename/delete/new files; branch/HEAD/ignore changes; fabricated
parser-version and mid-index changes; missing/corrupt/moved inputs as
`unknown`; corrupt staging leftovers and corrupt live databases recovered by
rebuild; concurrent queries during rebuilds internally consistent; two-repo
isolation; search spans matching the M2 golden contracts; pagination
contract; and stale/unknown never reported fresh.

## Tier B — real repository (MMSP, read-only, local only)

Target: MMSP clone, branch `main`, clean worktree at test time. Read-only
access (source + `git` metadata); nothing pulled, checked out, built, or
modified; no MMSP source copied anywhere; all outputs outside the MMSP tree.

- **Subset:** five representative files indexed, reopened, and searched;
  sample definitions, includes, call candidates, and spans checked by hand
  against the sources with zero errors.
- **Broad smoke:** full-tree index with the explicit scan policy; counts,
  timings, database size, peak memory, search latency, and freshness-check
  cost below.

- **Metrics (final build, MMSP `main/d62f06a`, clean):** index build 219.1 s,
  peak 338.4 MB RSS, exit 0; database 81,281,024 bytes (~77.5 MiB) holding one
  snapshot. Discovered 15,176 files; processed 11,765 (170,586 definitions,
  64,954 imports, 562,384 unresolved call candidates); 152,543 errors counted
  across 5,481 files, of which only 55 are first-party (rest: build outputs
  and vendored third-party); 291 skipped-large, 492 skipped-binary, 2,628
  unsupported (all 562 `.cu` as `cuda-experimental`); languages cpp=11,394,
  python=299, c=52, typescript=18, javascript=2. Reopen + `index status`:
  `fresh` with empty reasons in ~2.5 s (digest re-walk dominates).
  Hand-verified searches (`PatchMatch::Problem::Print`,
  `normalize_vec`, `CreateWorkspace`) hit the exact source spans in ~2.6 s
  end-to-end (freshness check included); pagination verified (`total:29`,
  distinct pages, `truncated:true`). A mid-validation rebuild raced the live
  ninja build tree and was honestly reported `stale(file-added)` — the
  freshness net working as designed, not a defect.

## Limitations observed (honest, not fixed in M3)

- Full rebuilds only; no incremental refresh (explicitly M4).
- Freshness re-walks content digests per query: correct but O(tree); measured
  below, no optimization without profiling.
- Concurrent `index` builds are not coordinated beyond the staging-lock
  check; concurrent queries are safe.
- Macro interiors stay opaque (inherited M2 grammar limit).

## Gates

- `cargo fmt --check`, `cargo check --locked`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test --locked`: green
  locally and on Windows, Linux, and macOS CI using self-contained fixtures
  only. No MMSP path exists in CI configuration or committed tests.
- Dependency and grammar license audit: `docs/licensing.md`.
