# CP2 pilot archive — MMSP at `d62f06a`

Frozen evidence of the M9 CP2 run (2026-10-20) against the MMSP pilot
repository at commit `d62f06a576d8e76b705c8ecc57fc380f7ebc9c44`, plus the
targeted C++ direct-initialization defect fix.

The narrative, the measurements and the decision package are in
[`docs/m9-cp2-evidence.md`](../../m9-cp2-evidence.md).

## What is here

| Path | Content |
|---|---|
| `raw/queries/` | the 20 CP1 §D workflow queries, verbatim JSON, plus the class-anchor probes |
| `raw/freshness/` | the CP1 §C scenarios F0–F5: `index status` and `repo status` per scenario |
| `raw/incremental/` | the no-op refresh, the one-file delta, the independent full rebuild, and the same-tree fact digests |
| `raw/perf/` | warm-query timings (debug and release) |
| `raw/mcp/` | the MCP stdio smoke, the D.5 tool sequence, the raw stdout capture |
| `raw/cold-index*.txt` | the cold-index runs, debug and release |
| `raw/grading.json` | the mechanical grading of the CP1 §D criteria |
| `raw/cp2-suite.txt` | the full console transcript of the post-fix suite |
| `method/` | the harness used for the run (measurement, queries, freshness, incremental, performance, MCP, grading, fact digests) |

## Redaction

Every archived file has machine-local paths replaced: the benchmark
workspace becomes `<CP2_BENCH>`, the user profile `<USERPROFILE>`, the
Riltra checkout `<RILTRA_ROOT>` and the MMSP checkout `<MMSP_ROOT>`. No
credential, token or key is present. The redaction rule set is
`method/redact.cjs`.

## Reproducing

The procedure is the one frozen in `docs/m9-cp1-pilot-spec.md` §A.2 and §A.7:

1. clone MMSP into a workspace **outside both repositories** and check out
   the pinned commit detached; never modify the original MMSP repository;
2. point `RILTRA_HOME` at a dedicated directory outside both repositories
   and register the snapshot as `mmsp-pin`;
3. run `method/cold-index.ps1` (≥ 2 independent cold runs), then
   `method/queries.ps1`, `method/freshness.ps1`, `method/incremental.ps1`
   and `method/perf.ps1` against that index;
4. grade the workflow answers with `method/grade.cjs`;
5. compare the incremental home with an independent full rebuild using
   `method/db-mutation-check.cjs` (order-independent fact digests over
   files, symbols, imports, calls and edges).

`method/opencode-session.ps1` is the OpenCode interop session; on the CP2
host it was **blocked** by the OpenCode free tier (see
`docs/m9-cp2-evidence.md` §7.3), and `method/d5-mcp-sequence.cjs` is the
scripted equivalent of the tool sequence an agent would run.

## Note on the pilot

The MMSP snapshot is read-only source at the pinned commit. No dataset,
build, script or configuration of MMSP was read, built or modified, and the
original MMSP repository was verified unchanged (HEAD and empty porcelain)
before and after every phase.
