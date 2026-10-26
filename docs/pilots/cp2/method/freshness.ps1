# CP2 harness step: freshness scenarios F0-F5 (CP1 section C, pre-registered
# expectations). The snapshot is restored to the frozen pin between scenarios.
# Every mutation of the snapshot goes through the guarded helpers: the
# snapshot must be a git repository inside the bench root and must not be the
# original repository.
#
# Usage:
#   pwsh -File freshness.ps1 -BenchRoot <dir> -Bin <riltra.exe> -Snapshot <dir> -MmspRoot <dir>
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
$env:RILTRA_HOME = $HomeDir
$snap = Assert-SnapshotPath -Path $cfg.Snapshot -Root $cfg.BenchRoot -MmspRoot $cfg.MmspRoot
$out = New-BenchDir -Path (Join-Path $cfg.Out 'freshness') -Root $cfg.BenchRoot

function Report([string]$name) {
  $status = & $cfg.Bin index status --repo mmsp-pin 2>&1 | Out-String
  $repo = & $cfg.Bin repo status --repo mmsp-pin 2>&1 | Out-String
  [System.IO.File]::WriteAllText((Join-Path $out "$name-index-status.json"), $status, (New-Object System.Text.UTF8Encoding($false)))
  [System.IO.File]::WriteAllText((Join-Path $out "$name-repo-status.json"), $repo, (New-Object System.Text.UTF8Encoding($false)))
  "----- $name -----"
  "index status: $status"
  "repo status  : $repo"
  "git porcelain: $(git -C $snap status --porcelain | Measure-Object -Line | ForEach-Object { $_.Lines }) lines"
}

"===== baseline (restored pin) ====="
Restore-Snapshot -Snapshot $snap -Root $cfg.BenchRoot -Pin $Pin -MmspRoot $cfg.MmspRoot | Out-Null
Report 'F0-baseline'

"===== F1: file modification (max_triangulation_angle default -1.0f -> 60.0f) ====="
Restore-Snapshot -Snapshot $snap -Root $cfg.BenchRoot -Pin $Pin -MmspRoot $cfg.MmspRoot | Out-Null
$p = Join-Path $snap 'colmap/src/colmap/mvs/patch_match_options.h'
(Get-Content $p -Raw) -replace 'double max_triangulation_angle = -1\.0f;', 'double max_triangulation_angle = 60.0f;' | Set-Content -NoNewline $p
Report 'F1-modify'
Restore-Snapshot -Snapshot $snap -Root $cfg.BenchRoot -Pin $Pin -MmspRoot $cfg.MmspRoot | Out-Null

"===== F2: file rename (patch_match_cuda.h -> patch_match_cuda_renamed.h) ====="
Restore-Snapshot -Snapshot $snap -Root $cfg.BenchRoot -Pin $Pin -MmspRoot $cfg.MmspRoot | Out-Null
git -C $snap mv colmap/src/colmap/mvs/patch_match_cuda.h colmap/src/colmap/mvs/patch_match_cuda_renamed.h 2>&1 | Out-Null
Report 'F2-rename'
Restore-Snapshot -Snapshot $snap -Root $cfg.BenchRoot -Pin $Pin -MmspRoot $cfg.MmspRoot | Out-Null

"===== F3: file deletion (patch_match_options.cc) ====="
Restore-Snapshot -Snapshot $snap -Root $cfg.BenchRoot -Pin $Pin -MmspRoot $cfg.MmspRoot | Out-Null
git -C $snap rm -q colmap/src/colmap/mvs/patch_match_options.cc 2>&1 | Out-Null
Report 'F3-delete'
Restore-Snapshot -Snapshot $snap -Root $cfg.BenchRoot -Pin $Pin -MmspRoot $cfg.MmspRoot | Out-Null

"===== F4: HEAD change (checkout 2b1bf42) ====="
Restore-Snapshot -Snapshot $snap -Root $cfg.BenchRoot -Pin $Pin -MmspRoot $cfg.MmspRoot | Out-Null
git -C $snap checkout --quiet --detach 2b1bf425f46939c6007d182a2009065334a8e78e 2>&1 | Out-Null
Report 'F4-head-change'
Restore-Snapshot -Snapshot $snap -Root $cfg.BenchRoot -Pin $Pin -MmspRoot $cfg.MmspRoot | Out-Null

"===== F5: branch change (same commit, new branch) ====="
Restore-Snapshot -Snapshot $snap -Root $cfg.BenchRoot -Pin $Pin -MmspRoot $cfg.MmspRoot | Out-Null
git -C $snap checkout --quiet -b m9-freshness-probe 2>&1 | Out-Null
Report 'F5-branch-change'
Restore-Snapshot -Snapshot $snap -Root $cfg.BenchRoot -Pin $Pin -MmspRoot $cfg.MmspRoot | Out-Null

"===== final restore verification ====="
"snapshot HEAD = $(git -C $snap rev-parse HEAD)"
"porcelain = $(git -C $snap status --porcelain | Measure-Object -Line | ForEach-Object { $_.Lines }) lines"
if ($cfg.MmspRoot) {
  "original MMSP repo: HEAD = $(git -C $cfg.MmspRoot rev-parse HEAD) porcelain = $(git -C $cfg.MmspRoot status --porcelain | Measure-Object -Line | ForEach-Object { $_.Lines }) lines"
}
