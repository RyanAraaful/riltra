# CP2 pilot archive — MMSP at `d62f06a`

Frozen evidence of the M9 CP2 run (2026-10-20) against the MMSP pilot
repository at commit `d62f06a576d8e76b705c8ecc57fc380f7ebc9c44`, plus the
targeted C++ direct-initialization defect fix and the independent-audit
corrections of 2026-10-20.

The narrative, the measurements and the decision package are in
[`docs/m9-cp2-evidence.md`](../../m9-cp2-evidence.md). How to run the
harness is in [`method/README.md`](method/README.md).

## What is here

| Path | Content |
|---|---|
| `raw/` | **the original CP2 run, preserved unchanged** — queries, freshness, incremental, perf, MCP, cold index, the first grading |
| `raw-audit/` | **the audit's corrected run** — the queries against the corrected index, the corrected grading, the incremental parity, the MCP smoke, the agent-side session summary |
| `method/` | the harness: the path-safety helpers and their tests, the step scripts, the grader and its tests, the digest implementation, the redaction |

Where the two runs differ, the audit numbers govern: the audit corrected the
C++ direct-initialization capture (the facts asserted from the
most-vexing-parse form are withdrawn, §9.4 of the evidence document) and the
grading (a vacuous `PASS` became `UNAVAILABLE`, §3).

## Redaction

Every archived file has machine-local paths replaced: the benchmark
workspace becomes `<CP2_BENCH>`, the user profile `<USERPROFILE>`, the
Riltra checkout `<RILTRA_ROOT>` and the MMSP checkout `<MMSP_ROOT>`. No
credential, token or key is present. The redaction rule set is
`method/redact.cjs`, which takes the paths to redact as arguments, so it
carries no machine-local path of its own.

## Reproducing

The procedure is the one frozen in `docs/m9-cp1-pilot-spec.md` §A.2 and §A.7.
`method/README.md` lists every script, its parameters and the order to run
them in; `method/run-all.ps1` runs the whole sequence.

1. clone MMSP into a workspace **outside both repositories** and check out
   the pinned commit detached; never modify the original MMSP repository;
2. point `RILTRA_HOME` at a dedicated directory outside both repositories
   and register the snapshot as `mmsp-pin`;
3. run `method/cold-index.ps1` (≥ 2 independent cold runs), then
   `method/queries.ps1`, `method/freshness.ps1`, `method/incremental.ps1`
   and `method/perf.ps1` against that index;
4. grade the workflow answers with `method/grade.cjs` (its own regression
   tests are `method/grade.test.cjs`, run with `node --test`);
5. compare the incremental home with an independent full rebuild using
   `method/fact-digest.cjs` (order-independent fact digests over files,
   symbols, imports, calls and edges) — `method/db-mutation-check.cjs`.

`method/test-path-safety.ps1` is the regression test of the harness path
guard: it must accept a real subdirectory of the benchmark root and refuse
the root itself, a parent, a sibling with a similar prefix, and a junction
inside the root that points outside it.

`method/opencode-session.ps1` is the OpenCode interop session; on the CP2
host it was **blocked** by the OpenCode free tier (see
`docs/m9-cp2-evidence.md` §7.3), and `method/d5-mcp-sequence.cjs` is the
scripted equivalent of the tool sequence an agent would run.

## Note on the pilot

The MMSP snapshot is read-only source at the pinned commit. No dataset,
build, script or configuration of MMSP was read, built or modified, and the
original MMSP repository was verified unchanged (HEAD and empty porcelain)
before and after every phase.
