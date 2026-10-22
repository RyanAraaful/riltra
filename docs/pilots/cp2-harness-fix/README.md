# CP2 harness correction — the new evidence (2026-10-22)

The evidence produced by the harness correction described in
[`docs/m9-cp2-harness-correction.md`](../../m9-cp2-harness-correction.md).

**This directory is new and separate.** It does not touch, replace or
re-grade anything in the frozen archives:

| Frozen archive | Status |
|---|---|
| `docs/pilots/cp2/raw/**` | untouched |
| `docs/pilots/cp2/raw-audit/**` | untouched |
| `docs/pilots/cp2-reeval/raw/**` | untouched — including its `grading.json`, whose historical `D5-H1 = FAIL` verdict stands |

## What is here

| File | Content |
|---|---|
| `raw/grading-corrected.json` | the **corrected** grader applied to the **archived** evidence: the re-evaluation's queries and MCP smoke (`docs/pilots/cp2-reeval/raw/`) and **this** run's session summary (`raw/opencode-session.json`, the one whose `tool_calls` the record lists). `D5-H1` is `PASS` there (`tool_count = 12`, the catalog is exactly the approved set); every other verdict is unchanged. This is additional evidence for the next evaluation, not a replacement of the historical grading |
| `raw/copy-smoke.txt` | the copy smoke: the original clone's state before and after, the copy's shape (the registry and the index at the root of the copy, no nested source folder) and `riltra index status --repo mmsp-pin` answering `fresh` with **exit 0** on the copied home |
| `raw/opencode-session.json` | the machine-readable summary of the corrected session harness: `exit_code`, `reason`, `error`, the tools the agent **called** (from the stream's structured `tool_use` events, with their statuses), `cited_sources` and `citations_found` (both measured from the agent's final message), the model, the wall time and the output sizes |
| `raw/opencode-session.answer.txt` | the agent's final message: its answer to the D.5 question, with its repository-relative `path:line` citations and its own statement of what it could not resolve |
| `raw/opencode-session.execute-inputs.txt` | the inputs of the agent's `execute` calls (its Code Mode snippets): the first two call `tools.riltra_*` — the Riltra MCP tools — and the last two call `tools.opencode.list_mcp_resources`; **none of them reads the source filesystem** |
| `raw/home-digest-before.txt`, `raw/home-digest-after.txt` | the store's digest before and after the session: the two files are **identical** — the database, the two `-wal`/`-shm` sidecars and the registry are unchanged |

The full model transcript was dropped as reproducible noise; the summary and
the answer text are archived instead.

## Redaction

The same rule set as the other archives, applied with the frozen
[`../cp2/method/redact.cjs`](../cp2/method/redact.cjs): the benchmark
workspace becomes `<CP2_BENCH>`, the user profile `<USERPROFILE>`, the
Riltra checkout `<RILTRA_ROOT>` and the MMSP checkout `<MMSP_ROOT>`. A scan
for the machine-local paths (in the plain **and** the JSON-escaped form)
finds none.

## Reproducing

```powershell
$bench    = "<a workspace outside both repositories>"
$riltra   = "<repo>/target/debug/riltra.exe"
$mmspOrig = "<the original clone, verified but never modified>"

# the harness regression tests
pwsh -File ../cp2/method/test-harness.ps1
node --test ../cp2/method/grade.test.cjs

# the copy smoke: copy the verified cold home and ask the real binary
. ../cp2/method/common.ps1
$main = Reset-BenchDir "$bench/home-main" -Root $bench
Copy-BenchDir -Source "$bench/home-cold1" -Destination $main -Root $bench
$env:RILTRA_HOME = $main
& $riltra index status --repo mmsp-pin

# the grader smoke: the corrected grader over the archived evidence
$env:CP2_MCP_SMOKE        = "<repo>/docs/pilots/cp2-reeval/raw/mcp/mcp-smoke.json"
$env:CP2_OPENCODE_SESSION = "<repo>/docs/pilots/cp2-harness-fix/raw/opencode-session.json"
$env:CP2_QDIR = "<repo>/docs/pilots/cp2-reeval/raw/queries"
node ../cp2/method/grade.cjs $env:CP2_QDIR "<out>/grading-corrected.json"

# the session smoke: a real agent against the isolated pilot home
pwsh -File ../cp2/method/opencode-session.ps1 -BenchRoot $bench -Bin $riltra `
    -Out "$bench/harness-fix" -HomeDir $main -MmspRoot $mmspOrig `
    -Model opencode/minimax-m3.1-flash-free -TimeoutSec 3600
```
