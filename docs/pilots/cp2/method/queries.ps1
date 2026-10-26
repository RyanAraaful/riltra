# CP2 harness step: ground-truth correctness queries (CP1 section D workflows).
# Each query is run against the pilot index; raw JSON + exit code + timing are
# archived. A query that does not answer is archived with its exit code, so
# the grader can tell "no answer" from "an answer that says nothing".
#
# Usage:
#   pwsh -File queries.ps1 -BenchRoot <dir> -Bin <riltra.exe> -Out <dir>
param(
  [string]$BenchRoot = '',
  [string]$Bin = '',
  [string]$Out = '',
  [string]$HomeDir = ''
)
$ErrorActionPreference = 'Continue'
. (Join-Path $PSScriptRoot 'common.ps1')
$cfg = Initialize-Cp2Harness -BenchRoot $BenchRoot -Bin $Bin -Out $Out
if (-not $HomeDir) { $HomeDir = Join-Path $cfg.BenchRoot 'home-main' }
$env:RILTRA_HOME = $HomeDir
$out = New-BenchDir -Path (Join-Path $cfg.Out 'queries') -Root $cfg.BenchRoot

# name, then CLI argument list
$queries = @(
  @{ n = 'd1-1-search-Print';              a = @('search','--repo','mmsp-pin','--symbol','Print') },
  @{ n = 'd1-2-search-normalize_vec';       a = @('search','--repo','mmsp-pin','--symbol','normalize_vec') },
  @{ n = 'd1-3-search-Run';                 a = @('search','--repo','mmsp-pin','--symbol','Run') },
  @{ n = 'd1-4-search-max_triangulation';   a = @('search','--repo','mmsp-pin','--symbol','max_triangulation_angle') },
  @{ n = 'd1-5-callers-PrintDef';           a = @('callers','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.cc','--line','118','--col','27') },
  @{ n = 'd1-6-callers-PrintDecl';          a = @('callers','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.h','--line','74','--col','10') },
  @{ n = 'd1-7-callers-normalize_vec';      a = @('callers','--repo','mmsp-pin','--path','colmap/benchmark/reconstruction/evaluation/geometry.py','--line','34','--col','5') },
  @{ n = 'd2-1-callers-PatchMatchCtrlRun';  a = @('callers','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.h','--line','136','--col','8') },
  @{ n = 'd2-2-callees-PatchMatchRun';      a = @('callees','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.cc','--line','191','--col','18') },
  @{ n = 'd2-3-callees-CtrlRun';            a = @('callees','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.cc','--line','233','--col','28') },
  @{ n = 'd2-4-callers-CreateWorkspace';    a = @('callers','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.cc','--line','54','--col','28') },
  @{ n = 'd2-5-callers-optmgr-binding';     a = @('callers','--repo','mmsp-pin','--path','colmap/src/colmap/controllers/option_manager.cc','--line','1019','--col','41') },
  @{ n = 'd3-1-trace-depth4';               a = @('trace','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.cc','--line','233','--col','28','--depth','4') },
  @{ n = 'd3-2-trace-to-cuda';              a = @('trace','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.cc','--line','233','--col','28','--depth','6','--to','colmap/src/colmap/mvs/patch_match_cuda.cu:1326:22') },
  @{ n = 'd4-1-impact-member';              a = @('impact','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match_options.h','--line','56','--col','10') },
  @{ n = 'd4-2-impact-function';            a = @('impact','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.cc','--line','191','--col','18') },
  @{ n = 'd0-1-index-status';               a = @('index','status','--repo','mmsp-pin') },
  @{ n = 'd0-2-repo-status';                a = @('repo','status','--repo','mmsp-pin') },
  @{ n = 'd0-3-unknown-repo-classified';    a = @('search','--repo','does-not-exist','--symbol','Print') },
  # Probes: the anchors the grader needs for the class-level capability gate.
  @{ n = 'p1-callers-class-PatchMatchCuda'; a = @('callers','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match_cuda.h','--line','49','--col','1') },
  @{ n = 'p2-callers-class-PatchMatch';      a = @('callers','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.h','--line','129','--col','1') },
  @{ n = 'p3-search-qualified-Print';        a = @('search','--repo','mmsp-pin','--symbol','PatchMatch::Problem::Print') }
)

$summary = @()
foreach ($q in $queries) {
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  $outText = & $cfg.Bin @($q.a) 2>&1 | Out-String
  $code = $LASTEXITCODE
  $sw.Stop()
  $path = Join-Path $out ($q.n + '.json')
  [System.IO.File]::WriteAllText($path, $outText, (New-Object System.Text.UTF8Encoding($false)))
  $len = $outText.Length
  $summary += [pscustomobject]@{ name = $q.n; ms = $sw.ElapsedMilliseconds; exit = $code; bytes = $len }
  "== $($q.n)  exit=$code  $($sw.ElapsedMilliseconds) ms  $len bytes"
}
$summary | ConvertTo-Json | Set-Content -Path (Join-Path $out '_summary.json') -Encoding utf8
"===== summary ====="
$summary | Format-Table -AutoSize | Out-String
