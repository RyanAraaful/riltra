# CP2 orchestration: post-fix cold index, workflow queries, freshness,
# incremental parity, warm-query performance, MCP stdio smoke.
$ErrorActionPreference = 'Stop'
. "$env:TEMP\opencode\bench\cp2\common.ps1"

$cp2 = "$env:TEMP\opencode\bench\cp2"
$raw = "$cp2\raw"
$bench = "$env:TEMP\opencode\bench"
$bin = '<RILTRA_ROOT>\target\debug\riltra.exe'
$snap = "$bench\mmsp-d62f06a"

function Section([string]$t) { "`n================ $t ================" }

Section "1. cold index x2 (post-fix)"
& "$cp2\cold-index.ps1"

Section "2. home-main from the verified cold run 1"
Remove-Item "$bench\home-main" -Recurse -Force -ErrorAction SilentlyContinue
Copy-Item "$bench\home-cold1" "$bench\home-main" -Recurse -Force
$env:RILTRA_HOME = "$bench\home-main"
& $bin index status --repo mmsp-pin

Section "3. workflow queries (CP1 section D)"
& "$cp2\queries.ps1"

Section "4. grading"
$env:CP2_QDIR = "$raw\queries"
$env:CP2_OUT = $raw
node "$cp2\grade.cjs"

Section "5. freshness F1-F5"
& "$cp2\freshness.ps1"

Section "6. incremental correctness"
& "$cp2\incremental.ps1"

Section "7. fact-level parity (DB digests: incremental vs oracle)"
Copy-Item "$bench\home-main\index\mmsp-pin.db" "$cp2\inc.db" -Force
Copy-Item "$bench\home-oracle\index\mmsp-pin.db" "$cp2\oracle.db" -Force
$inc = node "$cp2\db-facts.cjs" "$cp2\inc.db" digest 2>$null
$orc = node "$cp2\db-facts.cjs" "$cp2\oracle.db" digest 2>$null
"  incremental: $($inc -join ' ')"
"  oracle     : $($orc -join ' ')"
if ($inc -eq $orc) { "  FACT PARITY: IDENTICAL" } else { "  FACT PARITY: DIFFERENT" }

Section "8. warm query performance (5 reps each)"
& "$cp2\perf.ps1"

Section "9. MCP stdio smoke"
$env:CP2_HOME = "$bench\home-main"
$env:CP2_OUT = "$raw\mcp"
node "$cp2\mcp-smoke.cjs"

Section "10. isolation assertions"
"  snapshot HEAD  = $(git -C $snap rev-parse HEAD)"
"  snapshot tree  = $(git -C $snap rev-parse 'HEAD^{tree}')"
"  snapshot porcelain = $((git -C $snap status --porcelain | Measure-Object -Line).Lines) lines"
"  original MMSP HEAD = $(git -C <MMSP_ROOT> rev-parse HEAD)"
"  original MMSP porcelain = $((git -C <MMSP_ROOT> status --porcelain | Measure-Object -Line).Lines) lines"
"  Riltra worktree: $(git -C <RILTRA_ROOT> status --porcelain | Measure-Object -Line).Lines changed line(s)"
"DONE"
