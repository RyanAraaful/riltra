# CP2 step 4: freshness scenarios F1-F5 (CP1 section C, pre-registered expectations).
# The snapshot is restored to the frozen pin between scenarios.
$ErrorActionPreference = 'Stop'
. "$env:TEMP\opencode\bench\cp2\common.ps1"
$bin = '<RILTRA_ROOT>\target\debug\riltra.exe'
$env:RILTRA_HOME = "$env:TEMP\opencode\bench\home-main"
$snap = "$env:TEMP\opencode\bench\mmsp-d62f06a"
$out = "$env:TEMP\opencode\bench\cp2\raw\freshness"
New-Item -ItemType Directory -Force -Path $out | Out-Null
$pin = 'd62f06a576d8e76b705c8ecc57fc380f7ebc9c44'

function Restore-Snapshot {
  git -C $snap checkout --quiet --detach $pin | Out-Null
  git -C $snap reset --quiet --hard $pin | Out-Null
  git -C $snap clean -qfd | Out-Null
  git -C $snap branch -D m9-freshness-probe 2>&1 | Out-Null
}

function Report([string]$name) {
  $status = & $bin index status --repo mmsp-pin 2>&1 | Out-String
  $repo = & $bin repo status --repo mmsp-pin 2>&1 | Out-String
  [System.IO.File]::WriteAllText((Join-Path $out "$name-index-status.json"), $status, (New-Object System.Text.UTF8Encoding($false)))
  [System.IO.File]::WriteAllText((Join-Path $out "$name-repo-status.json"), $repo, (New-Object System.Text.UTF8Encoding($false)))
  "----- $name -----"
  "index status: $status"
  "repo status  : $repo"
  "git porcelain: $(git -C $snap status --porcelain | Measure-Object -Line | ForEach-Object { $_.Lines }) lines"
}

"===== baseline (restored pin) ====="
Restore-Snapshot
Report 'F0-baseline'

"===== F1: file modification (max_triangulation_angle default -1.0f -> 60.0f) ====="
Restore-Snapshot
$p = Join-Path $snap 'colmap/src/colmap/mvs/patch_match_options.h'
(Get-Content $p -Raw) -replace 'double max_triangulation_angle = -1\.0f;', 'double max_triangulation_angle = 60.0f;' | Set-Content -NoNewline $p
Report 'F1-modify'
Restore-Snapshot

"===== F2: file rename (patch_match_cuda.h -> patch_match_cuda_renamed.h) ====="
Restore-Snapshot
git -C $snap mv colmap/src/colmap/mvs/patch_match_cuda.h colmap/src/colmap/mvs/patch_match_cuda_renamed.h 2>&1 | Out-Null
Report 'F2-rename'
Restore-Snapshot

"===== F3: file deletion (patch_match_options.cc) ====="
Restore-Snapshot
git -C $snap rm -q colmap/src/colmap/mvs/patch_match_options.cc 2>&1 | Out-Null
Report 'F3-delete'
Restore-Snapshot

"===== F4: HEAD change (checkout 2b1bf42) ====="
Restore-Snapshot
git -C $snap checkout --quiet --detach 2b1bf425f46939c6007d182a2009065334a8e78e 2>&1 | Out-Null
Report 'F4-head-change'
Restore-Snapshot

"===== F5: branch change (same commit, new branch) ====="
Restore-Snapshot
git -C $snap checkout --quiet -b m9-freshness-probe 2>&1 | Out-Null
Report 'F5-branch-change'
Restore-Snapshot

"===== final restore verification ====="
"snapshot HEAD = $(git -C $snap rev-parse HEAD)"
"porcelain = $(git -C $snap status --porcelain | Measure-Object -Line | ForEach-Object { $_.Lines }) lines"
"original MMSP repo: HEAD = $(git -C <MMSP_ROOT> rev-parse HEAD) porcelain = $(git -C <MMSP_ROOT> status --porcelain | Measure-Object -Line | ForEach-Object { $_.Lines }) lines"
