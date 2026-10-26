# CP2 harness step: warm query performance (blueprint 14.3 diagnostic SLO
# targets). Frozen query list from CP1 section B; >= 5 repetitions each;
# p50/p95 reported. The SLO is a diagnostic target, not a release gate.
#
# Usage:
#   pwsh -File perf.ps1 -BenchRoot <dir> -Bin <riltra.exe> -Out <dir>
param(
  [string]$BenchRoot = '',
  [string]$Bin = '',
  [string]$Out = '',
  [string]$HomeDir = '',
  [int]$Reps = 5
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'common.ps1')
$cfg = Initialize-Cp2Harness -BenchRoot $BenchRoot -Bin $Bin -Out $Out
if (-not $HomeDir) { $HomeDir = Join-Path $cfg.BenchRoot 'home-main' }
$env:RILTRA_HOME = $HomeDir
$out = New-BenchDir -Path (Join-Path $cfg.Out 'perf') -Root $cfg.BenchRoot

$queries = @(
  @{ n = 'search-PatchMatch';        a = @('search','--repo','mmsp-pin','--symbol','PatchMatch') },
  @{ n = 'search-Run';               a = @('search','--repo','mmsp-pin','--symbol','Run') },
  @{ n = 'search-Print';             a = @('search','--repo','mmsp-pin','--symbol','Print') },
  @{ n = 'search-normalize_vec';     a = @('search','--repo','mmsp-pin','--symbol','normalize_vec') },
  @{ n = 'search-max-triang';        a = @('search','--repo','mmsp-pin','--symbol','max_triangulation_angle') },
  @{ n = 'callers-PrintDef';         a = @('callers','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.cc','--line','118','--col','27') },
  @{ n = 'callees-PatchMatchRun';    a = @('callees','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.cc','--line','191','--col','18') },
  @{ n = 'callees-CtrlRun';          a = @('callees','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.cc','--line','233','--col','28') },
  @{ n = 'trace-depth4';             a = @('trace','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.cc','--line','233','--col','28','--depth','4') },
  @{ n = 'impact-member';            a = @('impact','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match_options.h','--line','56','--col','10') },
  @{ n = 'index-status';             a = @('index','status','--repo','mmsp-pin') },
  @{ n = 'repo-status';              a = @('repo','status','--repo','mmsp-pin') }
)

$results = @()
foreach ($q in $queries) {
  $times = @()
  $bytes = 0
  $exit = 0
  for ($i = 1; $i -le $Reps; $i++) {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $text = & $cfg.Bin @($q.a) 2>&1 | Out-String
    $exit = $LASTEXITCODE
    $sw.Stop()
    $times += $sw.Elapsed.TotalMilliseconds
    $bytes = $text.Length
  }
  $sorted = $times | Sort-Object
  $p50 = $sorted[[math]::Floor(($Reps - 1) * 0.50)]
  $p95 = $sorted[[math]::Floor(($Reps - 1) * 0.95)]
  $results += [pscustomobject]@{
    query = $q.n; exit = $exit; bytes = $bytes
    ms_min = [math]::Round($sorted[0], 1)
    ms_p50 = [math]::Round($p50, 1)
    ms_max = [math]::Round($sorted[-1], 1)
    ms_p95 = [math]::Round($p95, 1)
    all = ($times | ForEach-Object { [math]::Round($_, 1) }) -join ','
  }
  "  {0,-24} p50={1,8} ms  p95={2,8} ms  max={3,8} ms  exit={4}" -f $q.n, [math]::Round($p50,1), [math]::Round($p95,1), [math]::Round($sorted[-1],1), $exit
}
$results | ConvertTo-Json -Depth 4 | Set-Content (Join-Path $out 'warm-queries.json') -Encoding utf8
"===== per-query detail ====="
$results | Format-Table -AutoSize | Out-String
"===== SLO comparison (blueprint 14.3, diagnostic target) ====="
"warm query p95 <= 500 ms : $($results | Where-Object { $_.ms_p95 -gt 500 } | ForEach-Object { $_.query } | Join-String -Separator ', ')"
"(empty above = every warm query met the 500 ms p95 target)"
