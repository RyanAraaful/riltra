# CP2 re-evaluation archive — MMSP at `d62f06a`, 2026-10-21

The raw evidence of the **second** CP2 execution, run after the
`riltra_code_semantic` integration (commit `1b162b5`). The narrative and the
grading are in [`docs/m9-cp2-reevaluation.md`](../../m9-cp2-reevaluation.md).

**This directory does not touch the frozen first CP2 archive.** The first
run's evidence and harness stay in [`../cp2/`](../cp2/) exactly as they were
archived on 2026-10-20; nothing there was edited, moved or overwritten.

## What is here

| Path | Content |
|---|---|
| `raw/` | the redacted raw evidence of this run: the measurement records, the workflow query outputs, the freshness verdicts, the incremental parity digests, the warm-query timings, the MCP smoke, the OpenCode session, the isolation record and the first attempt's record |
| `method/` | the scripts this run adds: the step runner, the twelfth-tool check, the agent session and the first-party split probe. Everything else is the frozen harness in [`../cp2/method/`](../cp2/method/), re-run as-is |

## Redaction

The same rule set as the first archive: the benchmark workspace becomes
`<CP2_BENCH>`, the user profile `<USERPROFILE>`, the Riltra checkout
`<RILTRA_ROOT>` and the MMSP checkout `<MMSP_ROOT>`. No credential, token or
key is present. The rule set is the frozen
[`../cp2/method/redact.cjs`](../cp2/method/redact.cjs), which takes the paths
to redact as arguments, so it carries no machine-local path of its own.

## Reproducing

The procedure is the one frozen in `docs/m9-cp1-pilot-spec.md` §A.2 and §A.7.
**Every path is an explicit parameter** — no *required* path falls back to a
placeholder default. Replace the `<...>` placeholders below with the local
values; the optional parameters that still have a default are the derived
ones (the snapshot inside the workspace, the canonical method directory, the
model id).

```powershell
# the workspace: outside both repositories
$bench    = "<a workspace outside both repositories>"
$raw      = "$bench/cp2/raw"
$riltra   = "<repo>/target/debug/riltra.exe"
$mmspOrig = "<the original clone, verified but never modified>"

# 0. the harness self-tests and the path-safety guard
pwsh -File ../cp2/method/test-path-safety.ps1
pwsh -File ../cp2/method/test-harness.ps1
node --test ../cp2/method/grade.test.cjs

# 1. the cold indexes (the harness's own step)
pwsh -File ../cp2/method/cold-index.ps1 -BenchRoot $bench -Bin $riltra `
    -Snapshot "$bench/mmsp-d62f06a" -Out $raw

# 2. the rest of the run, in the frozen order.
#    The step runner replicates run-all.ps1's steps 3-10. On 2026-10-21 the
#    frozen run-all.ps1 was NOT used, because its step 2 copied home-cold1
#    into a directory Reset-BenchDir had already created, which nested the
#    registry one level too deep (docs/m9-cp2-reevaluation.md section 8.1).
#    That defect is fixed since docs/m9-cp2-harness-correction.md, so a new
#    run may use ../cp2/method/run-all.ps1 directly.
pwsh -File method/run-steps.ps1 -BenchRoot $bench -Bin $riltra `
    -Snapshot "$bench/mmsp-d62f06a" -MmspRoot $mmspOrig

# 3. the twelfth tool on the pilot and the agent-side session
pwsh -File method/semantic-tool-check.ps1 -Bin $riltra `
    -HomeDir "$bench/home-cold1" -Out "$raw/semantic-tool"
pwsh -File method/opencode-session-d5.ps1 -Bin $riltra `
    -HomeDir "$bench/home-cold1" -SessionDir "$bench/cp2/opencode-session" `
    -Out "$raw/mcp" -Model opencode/minimax-m3.1-flash-free
```

The original MMSP clone is verified by the harness and never modified; the
snapshot is a plain clone outside both repositories, checked out at the pin.

**The scripts in this directory are the method of the 2026-10-21 run.** They
stay as the record of what actually ran. The canonical harness in
[`../cp2/method/`](../cp2/method/) has since been corrected — see
[`docs/m9-cp2-harness-correction.md`](../../m9-cp2-harness-correction.md) —
and a new evaluation should use it.
