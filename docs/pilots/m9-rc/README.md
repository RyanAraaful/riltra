# M9 release-candidate archive — MMSP at `d62f06a`, 2026-10-25

The raw evidence behind `docs/m9-cp4-evidence.md` and `docs/m9-e7-implementation.md`:
two executions of the frozen CP2 harness (`docs/pilots/cp2/method/`, run as-is) on the
same pinned snapshot, on the same host, in the same debug profile.

| Path | Content |
|---|---|
| `raw-before/` | the M9 tip `f65c183` (before the template-callee commits and this work) |
| `raw-after/` | the release-candidate code (`4bc8ed5`) |
| `raw-ci/` | step output of the release-candidate workflow on the three CI runners (sources of `f45ddb8`), runner paths replaced |
| `comparison.md` | the generated before/after tables (`method/compare.cjs`) |
| `method/` | what this run adds to the frozen harness (below) |

## What is in `method/`

| Script | Purpose |
|---|---|
| `e7-pilot.cjs` | E7 Slice 2 pilot acceptance (spec rows P1-P5, A5) and the F1 measurements; expectations were written from the frozen evidence before any run |
| `occurrence-parity.cjs` | row-for-row comparison of `reference_occurrences` between indexes of the same tree (the frozen fact digest does not cover that table) |
| `run-rest.ps1` | steps 5-11 of `run-all.ps1`: that script throws at the first graded FAIL, which stopped the after-run at step 4 (the D2-H2 FAIL), so the remaining steps ran through this script with the same sub-scripts and arguments |
| `compare.cjs` | the before/after report |

## Things the reader must know

- **D2-H2 FAILS in the after-run.** The callee names of `PatchMatch::Run` no longer contain
  `PatchMatchCuda`: the template-callee fix names the called function (`make_unique`), not
  `make_unique<PatchMatchCuda>`. The frozen grader was not changed and the criterion was not
  lowered; see `docs/m9-cp4-evidence.md` section 3.
- **F4 (HEAD change) was not exercised in the first pass of either run.** The snapshot is a
  depth-1 fetch and did not hold the later commit `2b1bf42`, so the checkout did nothing and the
  matrix reported `fresh`. The commit was fetched into both snapshots and the freshness step was
  run again for both; `freshness-f4-not-exercised/` keeps the first pass, `freshness/` is the
  valid one (`stale` with `head-changed`, as pre-registered).
- **No real OpenCode session ran.** `raw-after/mcp/opencode-session.json` records
  `mcp-server-not-reached`: the OpenCode API key on the host had expired. The scripted D5 tool
  sequence over the real MCP transport (`mcp-d5-scripted/`) and the MCP stdio smoke did run.
- `home_unchanged: false` in `mcp-smoke.json` is the accepted WAL-sidecar behaviour (the store's
  content digest is unchanged; the read creates `-wal` and `-shm`), identical before and after.
- The timings are one host, one build profile and one run each; the absolute figures are not
  comparable with the CP2 archives, which ran on another machine.

## Redaction

Machine-local paths were replaced with `<M9_BENCH_BEFORE>`, `<M9_BENCH_AFTER>`, `<USERPROFILE>`,
`<MMSP_ROOT>`, `<RILTRA_ROOT>` and `<SCRATCH>` by the frozen `docs/pilots/cp2/method/redact.cjs`.
No credential or key is present.

## Reproducing

```powershell
# bench roots outside both repositories, short paths (MMSP has 152-character tracked paths)
$bench = "<short path>"; $snap = "$bench/mmsp-d62f06a"   # plain clone of the pin, core.longpaths=true
pwsh -File ../cp2/method/run-all.ps1 -BenchRoot $bench -Bin <riltra.exe> -Snapshot $snap -MmspRoot <original clone> -Out $bench/raw
pwsh -File method/run-rest.ps1       -BenchRoot $bench -Bin <riltra.exe> -Snapshot $snap -MmspRoot <original clone> -Out $bench/raw   # only after a graded FAIL
node method/e7-pilot.cjs <riltra.exe> $bench/home-main $bench/raw/e7-pilot.json mmsp-pin
node method/compare.cjs <before run-all.log> <before raw> <after run-all.log> <after raw> comparison.md
```
