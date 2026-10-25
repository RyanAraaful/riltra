# M9 amendment: `.inl` coverage (G-SYN)

Dated revision: 2026-10-25.

This amends the coverage gap recorded as "`.inl` headers are not indexed" in
[m9-cp3-evidence.md](m9-cp3-evidence.md) (§6.1 and §9) and the G-SYN row of
[m9-cp4-evidence.md](m9-cp4-evidence.md). Those records are unchanged; this
document is the amendment.

## Decision

- `.inl` files are classified as C++, the same parser and extraction path as
  `.h`. The change is in `classify` in `crates/cli/src/syntax.rs`.
- `.ipp` and `.tpp` stay unsupported. No evidence was gathered for them.
- `.cu` and `.cuh` stay refused as `cuda-experimental`.
- `store::PARSER_VERSION` moves from 5 to 6. The classification change alters
  which files an index covers, so an index built at version 5 answers `stale`
  (`parser-changed`) and rebuilds in full.
- No parser, dependency, output status, or include/macro resolution was added.

## Evidence before the change

The snapshot is MMSP `d62f06a` (HEAD `d62f06a`, clean worktree). It tracks 37
`.inl` files, all under `colmap/src/thirdparty/PoissonRecon/`.

Probe, run before any code change: each `.inl` was copied unchanged into a
scratch directory outside the repository as `.cc`, then scanned with the
unchanged HEAD binary. Parsing depends only on content and language, so the
probe result is the result the `.inl` path produces.

| Outcome | Files |
|---|---|
| Processed, zero syntax errors | 9 |
| Processed, recovery errors (tree-sitter ERROR or MISSING nodes) | 28 (1 to 133 each, 396 in total) |
| Not processed (binary, large, unreadable, symlink) | 0 |

## Result on the MMSP snapshot

Full-tree scan of the same snapshot, before and after the change. The
"before" column reproduces the CP3 scan counts exactly.

| Measure | Before (HEAD `c869fdf`) | After | Delta |
|---|---|---|---|
| Files discovered | 2277 | 2277 | 0 |
| Processed | 1438 | 1475 | +37 |
| Unsupported | 835 | 798 | -37 |
| Skipped as binary | 4 | 4 | 0 |
| Definitions | 17213 | 18785 | +1572 |
| Imports | 6680 | 6680 | 0 |
| Calls (syntax candidates) | 91025 | 99720 | +8695 |
| Syntax errors | 1991 | 2387 | +396 |

Cold full index, same host: ingest 62.3 s before and 67.5 s after, measured
under concurrent load. These timings are not a performance claim. The index
database grew from 55,848,960 to 60,346,368 bytes (+8.1 %).

A repeat scan with the changed binary produced byte-identical output.

`.inl` after the change: 37 processed (all tracked `.inl` files), 9 with zero
syntax errors, 28 with recovery errors, 0 not processed.

## What the error count means

- `status` stays `ok` for every processed file. That is the existing output
  contract, and this change does not alter it.
- `errors` counts tree-sitter ERROR and MISSING nodes. `errors = 0` means the
  file parsed without error recovery. It does not mean every name resolves: a
  syntax-only parse has no include, macro, or type context. Call names stay as
  written.
- A fragment that needs its header context keeps its errors in `errors`, so it
  never reads as a clean parse.

## Freshness

- An index written at parser version 5 answers `stale` with `parser-changed`
  and rebuilds with `fallback-full`. Test:
  `inl_support_rebuilds_an_index_from_parser_version_five`.
- Editing a `.inl` file answers `stale` with `content-changed`. Re-indexing
  restores `fresh`. Test: `inl_edits_make_the_index_stale_until_it_is_rebuilt`.

## Golden files

The `revision` string carries a configuration digest that includes the parser
version. Bumping it changed the `cfg=` token from `1a5c2768082c1072` to
`95fce9bb78e9c9df` in 88 golden files: the 79 M6 corpus goldens and 9 top-level
M6 goldens. The corpus was regenerated with `RILTRA_UPDATE_M6_GOLDENS=1` after
its source-truth assertions passed. For every changed file, removing the
`cfg=` token leaves the old and new lines identical.

## Gate status

- The `.inl` coverage gap is closed for the syntax layer.
- G-SYN remains NOT YET VERIFIED. The open item is still the archived
  release-candidate scan report (CP3 §11). This amendment does not mark any
  gate PASS.

## Limits that remain

- `.ipp` and `.tpp` are not indexed.
- `.cu` and `.cuh` are refused. No CUDA coverage is claimed.
- No semantic resolution, include resolution, or macro expansion is added.
