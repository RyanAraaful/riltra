# SPDX-License-Identifier: Apache-2.0
# Steps 5-11 of the frozen CP2 harness (docs/pilots/cp2/method/run-all.ps1),
# for a run whose step 4 (the grading) exited non-zero.
#
# run-all.ps1 throws as soon as the grader reports a FAIL, so a graded FAIL
# also stops the freshness, incremental, performance and MCP steps. The
# grading itself is archived before it throws; this script runs the remaining
# steps in the same order, with the same sub-scripts and the same arguments,
# and only differs in that a FAIL of the final grading is reported, not thrown.
#
# Usage:
#   pwsh -File run-rest.ps1 -BenchRoot <dir> -Bin <riltra.exe> -Snapshot <dir> -MmspRoot <dir> -Out <dir>
# The bench root must already hold home-main (run-all.ps1 step 2).
param(
  [string]$BenchRoot = '',
  [string]$Bin = '',
  [string]$Snapshot = '',
  [Parameter(Mandatory = $true)][string]$MmspRoot,
  [string]$Out = ''
)
$ErrorActionPreference = 'Stop'
$method = Join-Path $PSScriptRoot '..\..\cp2\method' | Resolve-Path | Select-Object -ExpandProperty Path
. (Join-Path $method 'common.ps1')
$cfg = Initialize-Cp2Harness -BenchRoot $BenchRoot -Bin $Bin -Snapshot $Snapshot -MmspRoot $MmspRoot -Out $Out
$raw = $cfg.Out
$bench = $cfg.BenchRoot
$snap = Assert-SnapshotPath -Path $cfg.Snapshot -Root $cfg.BenchRoot -MmspRoot $cfg.MmspRoot
$homeMain = Join-Path $bench 'home-main'
if (-not (Test-Path -LiteralPath $homeMain)) { throw "home-main is missing: run run-all.ps1 through step 2 first" }
Remove-Item Env:CP2_MCP_SMOKE, Env:CP2_OPENCODE_SESSION -ErrorAction SilentlyContinue

function Section([string]$t) { "`n================ $t ================" }

Section "5. freshness F1-F5"
& (Join-Path $method 'freshness.ps1') -BenchRoot $bench -Bin $cfg.Bin -Snapshot $snap -MmspRoot $cfg.MmspRoot -Out $raw -HomeDir $homeMain

Section "6. incremental correctness"
& (Join-Path $method 'incremental.ps1') -BenchRoot $bench -Bin $cfg.Bin -Snapshot $snap -MmspRoot $cfg.MmspRoot -Out $raw -HomeDir $homeMain

Section "7. fact-level parity (the same-state pair from step 6)"
$incFacts = Join-Path $raw 'incremental\delta-inc-facts.json'
$orcFacts = Join-Path $raw 'incremental\delta-oracle-facts.json'
if (-not (Test-Path -LiteralPath $incFacts) -or -not (Test-Path -LiteralPath $orcFacts)) {
  throw "the delta digest pair is missing; step 6 did not archive it"
}
node (Join-Path $method 'fact-digest.cjs') compare-digests $incFacts $orcFacts
if ($LASTEXITCODE -ne 0) { throw "FACT PARITY: DIFFERENT (exit $LASTEXITCODE)" }
"  incremental: $((Get-Content $incFacts -Raw | ConvertFrom-Json).combined_digest)"
"  oracle     : $((Get-Content $orcFacts -Raw | ConvertFrom-Json).combined_digest)"
"  FACT PARITY: IDENTICAL (both indexes are from the same one-file-changed tree)"

Section "8. warm query performance (5 reps each)"
& (Join-Path $method 'perf.ps1') -BenchRoot $bench -Bin $cfg.Bin -Out $raw -HomeDir $homeMain

Section "9. MCP stdio smoke"
$env:CP2_HOME = $homeMain
$env:CP2_OUT = Join-Path $raw 'mcp'
$env:CP2_RILTRA = $cfg.Bin
node (Join-Path $method 'mcp-smoke.cjs')
if ($LASTEXITCODE -ne 0) { throw "the MCP smoke failed with exit $LASTEXITCODE" }

Section "10. the D.5 grading (reported, not thrown)"
$env:CP2_QDIR = Join-Path $raw 'queries'
$env:CP2_OUT = $raw
$env:CP2_MCP_SMOKE = Join-Path $raw 'mcp\mcp-smoke.json'
$sessionSummary = Join-Path $raw 'mcp\opencode-session.json'
$env:CP2_OPENCODE_SESSION = if (Test-Path -LiteralPath $sessionSummary) { $sessionSummary } else { '' }
node (Join-Path $method 'grade.cjs') (Join-Path $raw 'queries') (Join-Path $raw 'grading.json')
"  grading exit code: $LASTEXITCODE (non-zero means a graded FAIL, see grading.json)"

Section "11. isolation assertions"
"  snapshot HEAD  = $(git -C $snap rev-parse HEAD)"
"  snapshot tree  = $(git -C $snap rev-parse 'HEAD^{tree}')"
"  snapshot porcelain = $((git -C $snap status --porcelain | Measure-Object -Line).Lines) lines"
"  original MMSP HEAD = $(git -C $cfg.MmspRoot rev-parse HEAD)"
"  original MMSP porcelain = $((git -C $cfg.MmspRoot status --porcelain | Measure-Object -Line).Lines) lines"
"DONE"
