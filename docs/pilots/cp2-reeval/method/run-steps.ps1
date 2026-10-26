# The CP2 re-evaluation, run with the frozen harness step by step.
#
# This is the method of the 2026-10-21 re-evaluation. It ran the frozen
# harness steps in the frozen order, with one deviation: step 2 copied
# `home-cold1` to `home-main` with `Copy-Item -Recurse`, which nests the
# folder when the destination already exists - and `Reset-BenchDir` creates
# it, so the registry ended up one level too deep and every query reported
# "unknown repository". This runner therefore used the verified cold run 1
# home directly.
#
# The defect is fixed in the canonical harness since
# `docs/m9-cp2-harness-correction.md`: `run-all.ps1` now uses
# `Copy-BenchDir` and runs as-is. This script stays as the record of how
# the re-evaluation was actually run.
#
# Usage (every path is explicit):
#   pwsh -File run-steps.ps1 -BenchRoot <workspace> -Bin <riltra.exe> `
#       -MmspRoot <the original clone> [-Snapshot <dir>] [-Method <dir>]
param(
    [Parameter(Mandatory = $true)][string]$BenchRoot,
    [Parameter(Mandatory = $true)][string]$Bin,
    [string]$Snapshot = '',
    [Parameter(Mandatory = $true)][string]$MmspRoot,
    [string]$Method = ''
)
$ErrorActionPreference = 'Stop'
if (-not $Snapshot) { $Snapshot = Join-Path $BenchRoot 'mmsp-d62f06a' }
if (-not $Method) { $Method = (Resolve-Path (Join-Path $PSScriptRoot '..\..\cp2\method')).Path }
$raw = Join-Path $BenchRoot 'cp2\raw'
$homeMain = Join-Path $BenchRoot 'home-cold1'   # the verified cold run 1
New-Item -ItemType Directory -Force -Path $raw | Out-Null
function Section([string]$t) { "`n================ $t ================" }

Section "2. the main home is the verified cold run 1"
$env:RILTRA_HOME = $homeMain
& $Bin index status --repo mmsp-pin

Section "3. workflow queries (CP1 section D)"
& (Join-Path $Method 'queries.ps1') -BenchRoot $BenchRoot -Bin $Bin -Out $raw -HomeDir $homeMain

Section "4. grading"
$env:CP2_QDIR = Join-Path $raw 'queries'
$env:CP2_OUT = $raw
node (Join-Path $Method 'grade.cjs') (Join-Path $raw 'queries') (Join-Path $raw 'grading.json')
if ($LASTEXITCODE -ne 0) { throw "the grading script failed with exit $LASTEXITCODE" }

Section "5. freshness F1-F5"
& (Join-Path $Method 'freshness.ps1') -BenchRoot $BenchRoot -Bin $Bin -Snapshot $Snapshot -MmspRoot $MmspRoot -Out $raw -HomeDir $homeMain

Section "6. incremental correctness"
& (Join-Path $Method 'incremental.ps1') -BenchRoot $BenchRoot -Bin $Bin -Snapshot $Snapshot -MmspRoot $MmspRoot -Out $raw -HomeDir $homeMain

Section "7. fact-level parity (the same-state pair from step 6)"
$incFacts = Join-Path $raw 'incremental\delta-inc-facts.json'
$orcFacts = Join-Path $raw 'incremental\delta-oracle-facts.json'
if (-not (Test-Path -LiteralPath $incFacts) -or -not (Test-Path -LiteralPath $orcFacts)) {
    throw "the delta digest pair is missing; step 6 did not archive it"
}
node (Join-Path $Method 'fact-digest.cjs') compare-digests $incFacts $orcFacts
if ($LASTEXITCODE -ne 0) { throw "FACT PARITY: DIFFERENT (exit $LASTEXITCODE)" }
$incDigest = (Get-Content $incFacts -Raw | ConvertFrom-Json).combined_digest
$orcDigest = (Get-Content $orcFacts -Raw | ConvertFrom-Json).combined_digest
"  incremental: $incDigest"
"  oracle     : $orcDigest"
"  FACT PARITY: IDENTICAL (both indexes are from the same one-file-changed tree)"

Section "8. warm query performance (5 reps each)"
& (Join-Path $Method 'perf.ps1') -BenchRoot $BenchRoot -Bin $Bin -Out $raw -HomeDir $homeMain

Section "9. MCP stdio smoke"
$env:CP2_HOME = $homeMain
$env:CP2_OUT = Join-Path $raw 'mcp'
$env:CP2_RILTRA = $Bin
node (Join-Path $Method 'mcp-smoke.cjs')
if ($LASTEXITCODE -ne 0) { throw "the MCP smoke failed with exit $LASTEXITCODE" }

Section "10. isolation assertions"
"  snapshot HEAD  = $(git -C $Snapshot rev-parse HEAD)"
"  snapshot tree  = $(git -C $Snapshot rev-parse 'HEAD^{tree}')"
"  snapshot porcelain = $((git -C $Snapshot status --porcelain | Measure-Object -Line).Lines) lines"
if ($MmspRoot) {
    "  original MMSP HEAD = $(git -C $MmspRoot rev-parse HEAD)"
    "  original MMSP porcelain = $((git -C $MmspRoot status --porcelain | Measure-Object -Line).Lines) lines"
}
"DONE"

