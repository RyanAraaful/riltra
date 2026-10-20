# CP2 harness step: incremental correctness (CP1 section A.7 five-point parity
# contract).
#   1. no-op incremental refresh (mode-specific counters)
#   2. one-file delta
#   3. independent full rebuild on the SAME final tree, fresh home (oracle)
#   4. fact parity: fact counters + query outputs (spans/digests included)
#
# The comparison fails LOUDLY: a query that errors, produces no output or
# exits non-zero aborts the step instead of being compared as "identical".
#
# Usage:
#   pwsh -File incremental.ps1 -BenchRoot <dir> -Bin <riltra.exe> -Snapshot <dir>
param(
  [string]$BenchRoot = '',
  [string]$Bin = '',
  [string]$Snapshot = '',
  [string]$MmspRoot = '',
  [string]$Out = '',
  [string]$HomeDir = '',
  [string]$Pin = 'd62f06a576d8e76b705c8ecc57fc380f7ebc9c44'
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'common.ps1')
$cfg = Initialize-Cp2Harness -BenchRoot $BenchRoot -Bin $Bin -Snapshot $Snapshot -MmspRoot $MmspRoot -Out $Out
if (-not $HomeDir) { $HomeDir = Join-Path $cfg.BenchRoot 'home-main' }
$snap = Assert-SnapshotPath -Path $cfg.Snapshot -Root $cfg.BenchRoot -MmspRoot $cfg.MmspRoot
$out = New-BenchDir -Path (Join-Path $cfg.Out 'incremental') -Root $cfg.BenchRoot

# --- 1. no-op incremental refresh on the main home ---
$env:RILTRA_HOME = $HomeDir
"===== no-op incremental refresh ====="
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$noop = & $cfg.Bin index --repo mmsp-pin 2>&1 | Out-String
$noopCode = $LASTEXITCODE
$sw.Stop()
"exit=$noopCode wall_ms=$($sw.ElapsedMilliseconds)"
if ($noopCode -ne 0) { throw "the no-op refresh failed with exit $noopCode" }
[System.IO.File]::WriteAllText((Join-Path $out 'noop.json'), $noop, (New-Object System.Text.UTF8Encoding($false)))
$noop

# --- 2. one-file delta ---
"===== one-file delta ====="
Restore-Snapshot -Snapshot $snap -Root $cfg.BenchRoot -Pin $Pin -MmspRoot $cfg.MmspRoot | Out-Null
$p = Join-Path $snap 'colmap/src/colmap/mvs/patch_match_options.h'
(Get-Content $p -Raw) -replace 'double max_triangulation_angle = -1\.0f;', 'double max_triangulation_angle = 60.0f;' | Set-Content -NoNewline $p
# The delta must really exist: a pattern that no longer matches would make
# this a second no-op, and the parity below would then prove nothing.
$changed = (git -C $snap status --porcelain | Measure-Object -Line).Lines
"delta tree state: $changed porcelain line(s)"
if ($changed -lt 1) { throw "the one-file delta did not change the tree; the parity would prove nothing" }
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$delta = & $cfg.Bin index --repo mmsp-pin 2>&1 | Out-String
$deltaCode = $LASTEXITCODE
$sw.Stop()
"exit=$deltaCode wall_ms=$($sw.ElapsedMilliseconds)"
if ($deltaCode -ne 0) { throw "the one-file delta failed with exit $deltaCode" }
[System.IO.File]::WriteAllText((Join-Path $out 'delta.json'), $delta, (New-Object System.Text.UTF8Encoding($false)))
$delta
$deltaJson = $delta | ConvertFrom-Json
if ($deltaJson.reparsed -ne 1) { throw "the delta re-parsed $($deltaJson.reparsed) files instead of 1" }

# --- 3. independent full rebuild (oracle) on the SAME final tree, fresh home ---
"===== oracle: independent full rebuild, fresh home, same final tree ====="
$oracleHome = Reset-BenchDir (Join-Path $cfg.BenchRoot 'home-oracle') -Root $cfg.BenchRoot
$env:RILTRA_HOME = $oracleHome
& $cfg.Bin init --repo $snap --name mmsp-pin | Out-Null
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$oracle = & $cfg.Bin index --repo mmsp-pin --full 2>&1 | Out-String
$oracleCode = $LASTEXITCODE
$sw.Stop()
"exit=$oracleCode wall_ms=$($sw.ElapsedMilliseconds)"
if ($oracleCode -ne 0) { throw "the oracle rebuild failed with exit $oracleCode" }
[System.IO.File]::WriteAllText((Join-Path $out 'oracle.json'), $oracle, (New-Object System.Text.UTF8Encoding($false)))
$oracle
"oracle db bytes = $((Get-ChildItem $oracleHome -Recurse -File -Filter *.db | Measure-Object -Property Length -Sum).Sum)"

# --- 4. fact parity: query outputs from both homes ---
"===== parity: same queries, both homes, normalized ====="
$q = @(
  @('search','--repo','mmsp-pin','--symbol','PatchMatch'),
  @('search','--repo','mmsp-pin','--symbol','Run'),
  @('search','--repo','mmsp-pin','--symbol','normalize_vec'),
  @('search','--repo','mmsp-pin','--symbol','max_triangulation_angle'),
  @('callers','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.cc','--line','118','--col','27'),
  @('callees','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.cc','--line','191','--col','18'),
  @('callees','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.cc','--line','233','--col','28'),
  @('callers','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.cc','--line','54','--col','28'),
  @('callers','--repo','mmsp-pin','--path','colmap/benchmark/reconstruction/evaluation/geometry.py','--line','34','--col','5'),
  @('trace','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.cc','--line','233','--col','28','--depth','4'),
  @('impact','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match_options.h','--line','56','--col','10'),
  @('index','status','--repo','mmsp-pin')
)
function Normalize([string]$text) {
  # Only snapshot/registry METADATA is volatile between two homes: the
  # snapshot id, its creation timestamp, its nonce, and the registry's
  # added timestamp. Everything else (results, spans, digests, counters) is
  # content and must match byte for byte.
  $t = $text
  $t = $t -replace '"id":\d+', '"id":<SNAP>'
  $t = $t -replace '"created_unix":\d+', '"created_unix":<TS>'
  $t = $t -replace '"nonce":"[^"]*"', '"nonce":"<NONCE>"'
  $t = $t -replace '"revision":"[^"]*"', '"revision":"<REVISION>"'
  $t = $t -replace '"snapshot_id":\d+', '"snapshot_id":<SNAP>'
  $t = $t -replace 'nonce=[0-9a-f]+', 'nonce=<NONCE>'
  $t = $t -replace '"added_unix":\d+', '"added_unix":<TS>'
  return $t
}
$parity = @()
$parityFailures = @()
foreach ($qq in $q) {
  $name = ($qq -join '_') -replace '[\\/:]','_'
  $env:RILTRA_HOME = $HomeDir
  $rawA = & $cfg.Bin @($qq) 2>&1 | Out-String
  $codeA = $LASTEXITCODE
  $env:RILTRA_HOME = $oracleHome
  $rawB = & $cfg.Bin @($qq) 2>&1 | Out-String
  $codeB = $LASTEXITCODE
  [System.IO.File]::WriteAllText((Join-Path $out "parity-$name.inc.txt"), $rawA, (New-Object System.Text.UTF8Encoding($false)))
  [System.IO.File]::WriteAllText((Join-Path $out "parity-$name.oracle.txt"), $rawB, (New-Object System.Text.UTF8Encoding($false)))
  if ($codeA -ne 0 -or $codeB -ne 0) {
    $parityFailures += "$name (exit $codeA/$codeB)"
    "  IDENTICAL=UNVERIFIED exit=$codeA/$codeB  $name"
    continue
  }
  $a = Normalize $rawA
  $b = Normalize $rawB
  if ([string]::IsNullOrWhiteSpace($a) -or [string]::IsNullOrWhiteSpace($b)) {
    $parityFailures += "$name (empty output)"
    "  IDENTICAL=UNVERIFIED empty output  $name"
    continue
  }
  $same = ($a -eq $b)
  if (-not $same) { $parityFailures += "$name (content differs)" }
  $parity += [pscustomobject]@{ query = ($qq -join ' '); identical = $same; len_a = $a.Length; len_b = $b.Length }
  "  identical=$same  $name"
}
$parity | ConvertTo-Json | Set-Content (Join-Path $out '_parity-summary.json') -Encoding utf8

"===== restore the snapshot ====="
Restore-Snapshot -Snapshot $snap -Root $cfg.BenchRoot -Pin $Pin -MmspRoot $cfg.MmspRoot | Out-Null
"===== re-index the main home back to the pinned tree ====="
$env:RILTRA_HOME = $HomeDir
$re = & $cfg.Bin index --repo mmsp-pin 2>&1 | Out-String
[System.IO.File]::WriteAllText((Join-Path $out 'restore-reindex.json'), $re, (New-Object System.Text.UTF8Encoding($false)))
$re
"snapshot HEAD = $(git -C $snap rev-parse HEAD)  porcelain = $(git -C $snap status --porcelain | Measure-Object -Line | ForEach-Object { $_.Lines }) lines"
if ($cfg.MmspRoot) {
  "original MMSP: HEAD = $(git -C $cfg.MmspRoot rev-parse HEAD) porcelain = $(git -C $cfg.MmspRoot status --porcelain | Measure-Object -Line | ForEach-Object { $_.Lines }) lines"
}

if ($parityFailures.Count -gt 0) {
  throw "parity could not be established: $($parityFailures -join '; ')"
}
"PARITY: every query compared and identical after normalizing volatile snapshot metadata"
