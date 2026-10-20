# Multi-repository isolation matrix — CP3 phase 4

One isolated `RILTRA_HOME` holds exactly **two** registered repositories,
each in its own isolated clone. The original repositories are never touched.

| Repository | Root | Pin | Indexed files | Symbols | Imports |
|---|---|---|---|---|---|
| `mmsp-pin` | isolated clone of MMSP | `d62f06a576d8e76b705c8ecc57fc380f7ebc9c44` | 1 438 | 11 344 | 6 680 |
| `riltra-self` | isolated clone of Riltra | `fca1185678de612552ad24a7932bb3f09f175507` | 152 | 2 211 | 208 |

The Riltra self-index input is pinned at `fca1185`; the implementation under
test advances on the M9 branch, the self-index input does not.

Machine provenance: `cases.json` (every case's verdict), `c*.json` (every
command's raw output), `probe.json` (the store-level comparison),
`mcp-empty/` and `mcp-populated/` (the MCP sessions).

## The matrix

| # | Check | Result |
|---|---|---|
| C1 | both repositories register, each with its own root | **PASS** — `mmsp-pin`, `riltra-self`, two distinct roots |
| C2 | each repository has its own fresh index at its own head | **PASS** — both `fresh`, reasons empty, heads `d62f06a` / `fca1185` |
| C3 | a query targets the selected repository | **PASS** — `search --symbol main`: `mmsp-pin` 107 hits, `riltra-self` 10 hits; **0** paths from the other repository in either answer |
| C4 | callers, callees, trace and impact never cross the boundary | **PASS** — 6 graph queries (4 on MMSP, 2 on Riltra), **0** foreign path mentions in any answer |
| C5 | freshness is independent per repository | **PASS** — editing one indexed MMSP source file makes `mmsp-pin` **stale** (`dirty;content-changed`) while `riltra-self` stays **fresh**; after the restore both are fresh again |
| C6 | generation, snapshot and citation belong to one repository | **PASS** — every answer carries its own `repo` name and its own `snapshot.head` |
| C7 | read-only MCP tools create or migrate no store | **PASS (with the accepted WAL caveat below)** |
| C8 | a junction that leaves the repository root is not followed | **PASS** — see below |

## C3 and C4 — identically named symbols

27 symbol names exist in **both** repositories (`main`, `build`, `evaluate`,
`Index`, `update`, `load`, `run`, `PatchMatch::Problem::Print`, …). `main` is
the widest: 30 repository-relative paths in MMSP, 10 in Riltra.

The two stores share **0** repository-relative paths (`probe.json` →
`shared_repository_relative_paths: 0`), and every query answer contains only
its own repository's paths. The E8 candidate scope cannot leak either: the
suffix rule resolves a literal **inside one snapshot's file table**, and each
repository has its own store.

## C7 — read-only MCP purity

| Session | Result |
|---|---|
| read-only MCP session against an **empty** home | `home_unchanged: true`, **0** files created; 11 tools; stdout 6/6 JSON lines, 0 non-JSON; unknown repository → classified `not_registered` error |
| read-only MCP session against the **populated** home | 11 tools; stdout pure; classified error; **no store file created** (`store files created: 0`) and **no store file migrated** (`mmsp-pin.db` 50 020 352 bytes before and after, `riltra-self.db` 9 379 840 bytes before and after) |

**The accepted WAL caveat, stated accurately.** Against the populated home
the session leaves two extra files: `mmsp-pin.db-shm` and `mmsp-pin.db-wal`.
These are **SQLite's own WAL sidecars** for the store that already existed:

- **no store file is created** (`store files created: 0`);
- the store files' sizes are **unchanged** before and after, so **no schema
  migration ran**;
- the sidecars belong to the database SQLite opens for reading, and are
  removed again on the next clean close.

This is the WAL behaviour already accepted and documented for the repository,
not a read tool creating or migrating a store. The decisive evidence is the
empty-home session: a read-only MCP session against a home with no store
creates **nothing at all**.

## C8 — escape refused

A **junction** was created inside the MMSP clone pointing at a directory
outside the repository root.

| Action | Result |
|---|---|
| `scan --path colmap/src/colmap/cp3-escape/secret.h` | exit 3, `error: --path escapes repository root` |
| `index --repo mmsp-pin` with the junction present | exit 0, the outside file is **not** indexed |
| `search --repo mmsp-pin --symbol Secret` | `total: 0`, `results: []` |
| `doctor --repo mmsp-pin` | exit 0 |

The link was removed afterwards and the pinned tree restored
(`git rev-parse HEAD` = `d62f06a…`, 0 dirty lines) and re-indexed.

## Scope of this evidence

Riltra self-indexing is **internal validation**. It shows that the same
binary, the same home and the same code path keep two repositories apart. It
is **not** independent evidence that cross-repository behaviour is correct on
a third, unrelated codebase; the MMSP pilot remains the only external
repository under test, and no third pilot repository was added.
