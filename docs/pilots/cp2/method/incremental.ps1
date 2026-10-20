# CP2 step 5: incremental correctness (CP1 section A.7 five-point parity contract).
#   1. no-op incremental refresh (mode-specific counters)
#   2. one-file delta
#   3. independent full rebuild on the SAME final tree, fresh home (oracle)
#   4. fact parity: fact counters + query outputs (spans/digests included)
$ErrorActionPreference = 'Stop'
. "$env:TEMP\opencode\bench\cp2\common.ps1"
$bin = '<RILTRA_ROOT>\target\debug\riltra.exe'
$snap = "$env:TEMP\opencode\bench\mmsp-d62f06a"
$bench = "$env:TEMP\opencode\bench"
$out = "$bench\cp2\raw\incremental"
New-Item -ItemType Directory -Force -Path $out | Out-Null
$pin = 'd62f06a576d8e76b705c8ecc57fc380f7ebc9c44'

function Restore-Snapshot {
  git -C $snap checkout --quiet --detach $pin | Out-Null
  git -C $snap reset --quiet --hard $pin | Out-Null
  git -C $snap clean -qfd | Out-Null
}

# --- 1. no-op incremental refresh on the main home ---
$env:RILTRA_HOME = "$bench\home-main"
"===== no-op incremental refresh ====="
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$noop = & $bin index --repo mmsp-pin 2>&1 | Out-String
$noopCode = $LASTEXITCODE
$sw.Stop()
"exit=$noopCode wall_ms=$($sw.ElapsedMilliseconds)"
[System.IO.File]::WriteAllText("$out\noop.json", $noop, (New-Object System.Text.UTF8Encoding($false)))
$noop

# --- 2. one-file delta ---
"===== one-file delta ====="
Restore-Snapshot
$p = Join-Path $snap 'colmap/src/colmap/mvs/patch_match_options.h'
(Get-Content $p -Raw) -replace 'double max_triangulation_angle = -1\.0f;', 'double max_triangulation_angle = 60.0f;' | Set-Content -NoNewline $p
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$delta = & $bin index --repo mmsp-pin 2>&1 | Out-String
$deltaCode = $LASTEXITCODE
$sw.Stop()
"exit=$deltaCode wall_ms=$($sw.ElapsedMilliseconds)"
[System.IO.File]::WriteAllText("$out\delta.json", $delta, (New-Object System.Text.UTF8Encoding($false)))
$delta
"delta tree state: $(git -C $snap status --porcelain | Measure-Object -Line | ForEach-Object { $_.Lines }) porcelain line(s)"

# --- 3. independent full rebuild (oracle) on the SAME final tree, fresh home ---
"===== oracle: independent full rebuild, fresh home, same final tree ====="
$oracleHome = Reset-BenchDir "$bench\home-oracle"
$env:RILTRA_HOME = $oracleHome
& $bin init --repo $snap --name mmsp-pin | Out-Null
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$oracle = & $bin index --repo mmsp-pin --full 2>&1 | Out-String
$oracleCode = $LASTEXITCODE
$sw.Stop()
"exit=$oracleCode wall_ms=$($sw.ElapsedMilliseconds)"
[System.IO.File]::WriteAllText("$out\oracle.json", $oracle, (New-Object System.Text.UTF8Encoding($false)))
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
  $t = $text
  $t = $t -replace '"id":\d+', '"id":<SNAP>'
  $t = $t -replace '"nonce":"[^"]*"', '"nonce":"<NONCE>"'
  $t = $t -replace '"revision":"[^"]*"', '"revision":"<REVISION>"'
  $t = $t -replace '"snapshot_id":\d+', '"snapshot_id":<SNAP>'
  $t = $t -replace 'nonce=[0-9a-f]+', 'nonce=<NONCE>'
  $t = $t -replace '"added_unix":\d+', '"added_unix":<TS>'
  return $t
}
$parity = @()
foreach ($qq in $q) {
  $name = ($qq -join '_') -replace '[\\/:]','_'
  $env:RILTRA_HOME = "$bench\home-main"
  $a = Normalize ((& $bin @($qq) 2>&1 | Out-String))
  $env:RILTRA_HOME = "$bench\home-oracle"
  $b = Normalize ((& $bin @($qq) 2>&1 | Out-String))
  $same = ($a -eq $b)
  $parity += [pscustomobject]@{ query = ($qq -join ' '); identical = $same; len_a = $a.Length; len_b = $b.Length }
  [System.IO.File]::WriteAllText("$out\parity-$name.inc.txt", $a, (New-Object System.Text.UTF8Encoding($false)))
  [System.IO.File]::WriteAllText("$out\parity-$name.oracle.txt", $b, (New-Object System.Text.UTF8Encoding($false)))
  "  identical=$same  $name"
}
$parity | ConvertTo-Json | Set-Content "$out\_parity-summary.json" -Encoding utf8

"===== restore the snapshot ====="
Restore-Snapshot
"===== re-index the main home back to the pinned tree ====="
$env:RILTRA_HOME = "$bench\home-main"
$re = & $bin index --repo mmsp-pin 2>&1 | Out-String
[System.IO.File]::WriteAllText("$out\restore-reindex.json", $re, (New-Object System.Text.UTF8Encoding($false)))
$re
"snapshot HEAD = $(git -C $snap rev-parse HEAD)  porcelain = $(git -C $snap status --porcelain | Measure-Object -Line | ForEach-Object { $_.Lines }) lines"
"original MMSP: HEAD = $(git -C <MMSP_ROOT> rev-parse HEAD) porcelain = $(git -C <MMSP_ROOT> status --porcelain | Measure-Object -Line | ForEach-Object { $_.Lines }) lines"
