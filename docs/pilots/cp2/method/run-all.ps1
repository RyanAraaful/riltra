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
  [Parameter(Mandatory = $true)][string]$MmspRoot,
  [string]$Out = ''
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'common.ps1')
$cfg = Initialize-Cp2Harness -BenchRoot $BenchRoot -Bin $Bin -Snapshot $Snapshot -MmspRoot $MmspRoot -Out $Out
$raw = $cfg.Out
$bench = $cfg.BenchRoot
$snap = Assert-SnapshotPath -Path $cfg.Snapshot -Root $cfg.BenchRoot -MmspRoot $cfg.MmspRoot
$method = $PSScriptRoot
# Step 4 grades before the MCP smoke exists: never let ambient variables make
# it read archived evidence instead of this run's.
Remove-Item Env:CP2_MCP_SMOKE, Env:CP2_OPENCODE_SESSION -ErrorAction SilentlyContinue

function Section([string]$t) { "`n================ $t ================" }

Section "0. harness self-tests (the comparison must be able to fail)"
Push-Location $method
try {
  node --test (Join-Path $method 'fact-digest.test.cjs')
  if ($LASTEXITCODE -ne 0) { throw "the fact-digest tests failed with exit $LASTEXITCODE" }
  node --test (Join-Path $method 'grade.test.cjs')
  if ($LASTEXITCODE -ne 0) { throw "the grader tests failed with exit $LASTEXITCODE" }
  pwsh -NoProfile -File (Join-Path $method 'test-harness.ps1')
  if ($LASTEXITCODE -ne 0) { throw "the harness tests failed with exit $LASTEXITCODE" }
  pwsh -NoProfile -File (Join-Path $method 'test-path-safety.ps1')
  if ($LASTEXITCODE -ne 0) { throw "the path-safety tests failed with exit $LASTEXITCODE" }
} finally { Pop-Location }

Section "1. cold index x2"
& (Join-Path $method 'cold-index.ps1') -BenchRoot $bench -Bin $cfg.Bin -Snapshot $snap -Out $raw

Section "2. home-main from the verified cold run 1"
$homeMain = Reset-BenchDir (Join-Path $bench 'home-main') -Root $bench
Copy-BenchDir -Source (Join-Path $bench 'home-cold1') -Destination $homeMain -Root $bench
$env:RILTRA_HOME = $homeMain
& $cfg.Bin index status --repo mmsp-pin
if ($LASTEXITCODE -ne 0) { throw "the copied home does not answer: index status failed with exit $LASTEXITCODE" }

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

Section "7. fact-level parity (the same-state pair from step 6)"
# Step 6 proved the parity on ONE source tree: the tree with the one-file
# change, indexed incrementally and rebuilt from scratch. The digests are
# read back from the archive, so the comparison is re-made here with the
# tested comparison. Two indexes from DIFFERENT source states are never
# compared: after step 6 the main home has been restored to the pinned tree,
# so comparing it with the oracle (built on the changed tree) would prove
# nothing.
$incFacts = Join-Path $raw 'incremental\delta-inc-facts.json'
$orcFacts = Join-Path $raw 'incremental\delta-oracle-facts.json'
if (-not (Test-Path -LiteralPath $incFacts) -or -not (Test-Path -LiteralPath $orcFacts)) {
  throw "the delta digest pair is missing; step 6 did not archive it"
}
node (Join-Path $method 'fact-digest.cjs') compare-digests $incFacts $orcFacts
if ($LASTEXITCODE -ne 0) { throw "FACT PARITY: DIFFERENT (exit $LASTEXITCODE)" }
$incDigest = (Get-Content $incFacts -Raw | ConvertFrom-Json).combined_digest
$orcDigest = (Get-Content $orcFacts -Raw | ConvertFrom-Json).combined_digest
"  incremental: $incDigest"
"  oracle     : $orcDigest"
"  FACT PARITY: IDENTICAL (both indexes are from the same one-file-changed tree)"
$tree = Get-Content (Join-Path $raw 'incremental\delta-tree.json') -Raw | ConvertFrom-Json
"  tree: head=$($tree.snapshot_head) changed=$($tree.changed_file) sha256=$($tree.changed_file_sha256.Substring(0,16))..."

Section "8. warm query performance (5 reps each)"
& (Join-Path $method 'perf.ps1') -BenchRoot $bench -Bin $cfg.Bin -Out $raw -HomeDir $homeMain

Section "9. MCP stdio smoke"
$env:CP2_HOME = $homeMain
$env:CP2_OUT = Join-Path $raw 'mcp'
$env:CP2_RILTRA = $cfg.Bin
node (Join-Path $method 'mcp-smoke.cjs')
if ($LASTEXITCODE -ne 0) { throw "the MCP smoke failed with exit $LASTEXITCODE" }

Section "10. the D.5 grading (the smoke and the session are the D.5 evidence)"
# The D5 checks are graded from the smoke and the session, both of which are
# produced after step 4, so the grading is redone here. The historical
# grader graded before the smoke existed and reported D5-H1 as UNAVAILABLE.
# The session is NOT run by this script (it needs the model provider): run
# opencode-session.ps1 with -Out $raw first, and the grading picks its
# summary up from there.
$env:CP2_MCP_SMOKE = Join-Path $raw 'mcp\mcp-smoke.json'
$sessionSummary = Join-Path $raw 'mcp\opencode-session.json'
if (Test-Path -LiteralPath $sessionSummary) {
  $env:CP2_OPENCODE_SESSION = $sessionSummary
} else {
  $env:CP2_OPENCODE_SESSION = ''
  "  no session summary at ${sessionSummary}: D5-E1 will be UNAVAILABLE"
}
node (Join-Path $method 'grade.cjs') (Join-Path $raw 'queries') (Join-Path $raw 'grading.json')
if ($LASTEXITCODE -ne 0) { throw "the D.5 grading failed with exit $LASTEXITCODE" }

Section "11. isolation assertions"
"  snapshot HEAD  = $(git -C $snap rev-parse HEAD)"
"  snapshot tree  = $(git -C $snap rev-parse 'HEAD^{tree}')"
"  snapshot porcelain = $((git -C $snap status --porcelain | Measure-Object -Line).Lines) lines"
if ($cfg.MmspRoot) {
  "  original MMSP HEAD = $(git -C $cfg.MmspRoot rev-parse HEAD)"
  "  original MMSP porcelain = $((git -C $cfg.MmspRoot status --porcelain | Measure-Object -Line).Lines) lines"
}
"DONE"
