# M9 amendment: G-KNW release-level round trip

Dated revision: 2026-10-25. This amends the G-KNW status recorded in
[m9-design-gate.md](m9-design-gate.md) (gate table and the G-KNW section),
[m9-cp3-evidence.md](m9-cp3-evidence.md) §6.5 and its status table, and
[m9-cp4-evidence.md](m9-cp4-evidence.md) status table. Those records are
unchanged.

## Result

- **G-KNW: PASS** on the acceptance test in
  [m9-design-gate.md](m9-design-gate.md): isolated-home backup, verify, store
  loss, restore, and an export that is byte-identical before and after. The
  export is deterministic, and CLI and MCP reads leave the store unchanged.
- Evidence covers HEAD `f9eed48` (debug build, `cargo build --locked`) on one
  Windows 11 host. This commit adds evidence files only. No source, schema,
  table, API, or MCP tool changed.
- Not covered: other hosts, MMSP-scale data, and release artifacts beyond this
  host (see Limitations).

## Reproduction

Run from the repository root with an empty scratch directory:

```bash
WORK=<empty-scratch-dir> bash docs/pilots/m9-gknw/method/g-knw.sh
```

Every riltra call uses `RILTRA_HOME` inside `WORK`. The user's knowledge store
is not touched. The script prints no absolute paths. The output of the second
run is archived in [pilots/m9-gknw/raw/run.log](pilots/m9-gknw/raw/run.log):
73 checks, 0 failed.

Seed (the M7 fixture shape `port_seed` in `crates/cli/tests/knowledge.rs`,
written through the CLI only): 4 records (`d1` superseded by `d2`, `d2`
confirmed, `d3` retracted, `b1` benchmark with a measurement; `d1` evidence
bound to `a.py`) and 3 relations (`r1` supports, `r2` unlinked, and the implicit
`supersedes`). Ten mutations, so store generation 10.

## Results

| Step | Result |
|---|---|
| Export, initial | JSONL 2688 bytes, Markdown 3170 bytes; a second export of each is byte-identical |
| Backup | `VACUUM INTO` snapshot and manifest (schema 2, 4 records, 3 relations); no `-wal` or `-shm` beside it; live store bytes unchanged |
| Verify backup | Restored into a separate home: `overwrote: false`, 4 records, 3 relations; `verify` ok; export byte-identical to the initial export; independent `sqlite3` `PRAGMA integrity_check` ok, `user_version` 2 (on a copy) |
| Store loss | Before loss: `krepo.db-wal` 0 bytes, `krepo.db-shm` 32768 bytes; database and companions removed |
| Absent store | Export: generation 0. Search: 0 hits. Neither creates a database |
| Restore | Into the main home: `overwrote: false`, 4 records, 3 relations, generation 10 |
| Export after restore | JSONL and Markdown byte-identical to the initial export; generation 10; `verify` ok |
| Read-only, CLI | 9 invocations (verify, show, show at revision, history, search, related outbound and inbound, export, verify): store file sha256 unchanged, generation unchanged, export identical |
| Read-only, MCP | `riltra_knowledge_show`, `_search`, `_related` over stdio: 4 responses (initialize plus 3), no `isError`; store file sha256 unchanged |

Byte evidence (second run): the restored store file has the same sha256 as the
backup snapshot (`78dd5f42…`), because restore renames a verified staged copy.
The live store before loss (`d3e94e4f…`) differs from the snapshot in bytes,
as expected for a compacted copy; the export comparison shows equal logical
content.

Hashes differ between runs (first run snapshot `cc34fb5c…`, second run
`78dd5f42…`) because exports and database bytes embed `created_unix`. Byte
identity is therefore a within-run property. It held in both runs.

## Regression tests

| Target | Selection | Result |
|---|---|---|
| `tests/knowledge.rs` | all | 52 passed, 0 failed |
| unit tests | name filter `knowledge` | 124 passed, 0 failed |
| `tests/mcp.rs` | name filter `knowledge` | 7 passed, 0 failed |

Total: 183 passed, 0 failed. The full suite was not rerun: no code has changed
since the full run at `f9eed48` (772 passed, 0 failed).

## Context7 check

SQLite documentation was checked through Context7 (`/websites/sqlite_docs`) for
generic library behaviour:

- `VACUUM INTO` yields a consistent snapshot, leaves the source unchanged, and
  requires a target that does not exist or is empty. This matches `backup_to`,
  which refuses existing outputs.
- A WAL store loses its `-wal` and `-shm` on the last clean close, but a last
  client that is read-only can leave them. This matches the sidecars seen in this
  run and the WAL limitation recorded in the G-SEC evidence.

No conflict with the installed behaviour was found.

## Limitations

- One host (Windows 11), debug build, 4-record fixture. No size, timing, or
  MMSP-scale claim.
- No CLI command verifies a backup file in place: `knowledge verify` checks the
  live store only. Backup verification therefore restores into a separate home
  (restore runs the structural check on the staged snapshot against the
  manifest), then verifies and compares exports. A `verify --file` command was
  not added, since it would be new API.
- Restore fails closed while `-wal` or `-shm` exist. Read-only opens of a WAL
  store leave them behind, so an operator must confirm that no process uses the
  store before removing them. Here the removed `-wal` held 0 bytes and the
  snapshot already held the data (export comparison). This does not establish
  safety for other stores.
- The script is manual evidence for Windows (Git Bash, `cygpath`). CI does not
  run it.
- Unchanged and out of scope: F2 (held in a local stash), F5, and C++ on MMSP
  (NOT YET VERIFIED, D-M9-2).

## Status

G-KNW: PASS for the isolated-home acceptance test at `f9eed48`. The
"NOT YET VERIFIED (release)" entry for G-KNW is closed for this host only.
