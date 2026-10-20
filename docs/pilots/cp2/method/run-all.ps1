# CP2 harness: the full run.
#   1. cold index x2        5. freshness F1-F5
#   2. home-main            6. incremental correctness
#   3. workflow queries     7. fact parity (DB digests)
#   4. grading              8. warm query performance
#                           9. MCP stdio smoke
#                          10. isolation assertions
#
# Every comparison fails loudly: a script that crashes, produces no output
# or exits non-zero aborts the run instead of being counted as a match.
#
# Usage:
#   pwsh -File run-all.ps1 -BenchRoot <dir> -Bin <riltra.exe> -Snapshot <dir> -MmspRoot <dir>
param(
  [string]$BenchRoot = '',
  [string]$Bin = '',
  [string]$Snapshot = '',
  [string]$MmspRoot = '',
  [string]$Out = ''
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'common.ps1')
$cfg = Initialize-Cp2Harness -BenchRoot $BenchRoot -Bin $Bin -Snapshot $Snapshot -MmspRoot $MmspRoot -Out $Out
$raw = $cfg.Out
$bench = $cfg.BenchRoot
$snap = Assert-SnapshotPath -Path $cfg.Snapshot -Root $cfg.BenchRoot -MmspRoot $cfg.MmspRoot
$method = $PSScriptRoot

function Section([string]$t) { "`n================ $t ================" }

Section "1. cold index x2"
& (Join-Path $method 'cold-index.ps1') -BenchRoot $bench -Bin $cfg.Bin -Snapshot $snap -Out $raw

Section "2. home-main from the verified cold run 1"
$homeMain = Reset-BenchDir (Join-Path $bench 'home-main') -Root $bench
Copy-Item (Join-Path $bench 'home-cold1') $homeMain -Recurse -Force
$env:RILTRA_HOME = $homeMain
& $cfg.Bin index status --repo mmsp-pin

Section "3. workflow queries (CP1 section D)"
& (Join-Path $method 'queries.ps1') -BenchRoot $bench -Bin $cfg.Bin -Out $raw -HomeDir $homeMain

Section "4. grading"
$env:CP2_QDIR = Join-Path $raw 'queries'
$env:CP2_OUT = $raw
# The grader must read THIS run's evidence, not the archived one.
node (Join-Path $method 'grade.cjs') (Join-Path $raw 'queries') (Join-Path $raw 'grading.json')
if ($LASTEXITCODE -ne 0) { throw "the grading script failed with exit $LASTEXITCODE" }

Section "5. freshness F1-F5"
& (Join-Path $method 'freshness.ps1') -BenchRoot $bench -Bin $cfg.Bin -Snapshot $snap -MmspRoot $cfg.MmspRoot -Out $raw -HomeDir $homeMain

Section "6. incremental correctness"
& (Join-Path $method 'incremental.ps1') -BenchRoot $bench -Bin $cfg.Bin -Snapshot $snap -MmspRoot $cfg.MmspRoot -Out $raw -HomeDir $homeMain

Section "7. fact-level parity (DB digests: incremental vs oracle)"
$incDb = Join-Path $raw 'inc.db'
$orcDb = Join-Path $raw 'oracle.db'
Copy-Item (Join-Path $homeMain 'index\mmsp-pin.db') $incDb -Force
Copy-Item (Join-Path $bench 'home-oracle\index\mmsp-pin.db') $orcDb -Force
$inc = node (Join-Path $method 'db-mutation-check.cjs') $incDb (Join-Path $raw 'inc-facts.json')
if ($LASTEXITCODE -ne 0) { throw "the incremental fact digest failed with exit $LASTEXITCODE" }
$orc = node (Join-Path $method 'db-mutation-check.cjs') $orcDb (Join-Path $raw 'oracle-facts.json')
if ($LASTEXITCODE -ne 0) { throw "the oracle fact digest failed with exit $LASTEXITCODE" }
$incDigest = (Get-Content (Join-Path $raw 'inc-facts.json') -Raw | ConvertFrom-Json).combined_digest
$orcDigest = (Get-Content (Join-Path $raw 'oracle-facts.json') -Raw | ConvertFrom-Json).combined_digest
if (-not $incDigest -or -not $orcDigest) { throw "a fact digest is missing; the comparison cannot be made" }
"  incremental: $incDigest"
"  oracle     : $orcDigest"
if ($incDigest -eq $orcDigest) {
  "  FACT PARITY: IDENTICAL"
}
else {
  throw "FACT PARITY: DIFFERENT ($incDigest vs $orcDigest)"
}

Section "8. warm query performance (5 reps each)"
& (Join-Path $method 'perf.ps1') -BenchRoot $bench -Bin $cfg.Bin -Out $raw -HomeDir $homeMain

Section "9. MCP stdio smoke"
$env:CP2_HOME = $homeMain
$env:CP2_OUT = Join-Path $raw 'mcp'
$env:CP2_RILTRA = $cfg.Bin
node (Join-Path $method 'mcp-smoke.cjs')
if ($LASTEXITCODE -ne 0) { throw "the MCP smoke failed with exit $LASTEXITCODE" }

Section "10. isolation assertions"
"  snapshot HEAD  = $(git -C $snap rev-parse HEAD)"
"  snapshot tree  = $(git -C $snap rev-parse 'HEAD^{tree}')"
"  snapshot porcelain = $((git -C $snap status --porcelain | Measure-Object -Line).Lines) lines"
if ($cfg.MmspRoot) {
  "  original MMSP HEAD = $(git -C $cfg.MmspRoot rev-parse HEAD)"
  "  original MMSP porcelain = $((git -C $cfg.MmspRoot status --porcelain | Measure-Object -Line).Lines) lines"
}
"DONE"
