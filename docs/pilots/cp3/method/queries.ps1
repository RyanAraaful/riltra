# CP3 harness step: the E8 before/after query set.
#
# The index is NOT rebuilt between the two runs: the E8 correction is a
# query-side change (the caller candidate scope), so the same pilot index
# serves both binaries and the diff isolates the change exactly.
#
# The query set is CP2's set verbatim, plus the anchors the E8 decision
# touches (a C++ header whose definition is reached from other translation
# units through a rooted include).
#
# Usage:
#   pwsh -File queries.ps1 -HomeDir <dir> -BinBefore <riltra.exe> \
#         -BinAfter <riltra.exe> -Out <dir>
param(
  [string]$HomeDir = '',
  [string]$BinBefore = '',
  [string]$BinAfter = '',
  [string]$Out = ''
)
$ErrorActionPreference = 'Continue'

if (-not $HomeDir) { $HomeDir = Join-Path $env:TEMP 'opencode\cp3\home' }
if (-not $BinBefore) { $BinBefore = Join-Path $env:TEMP 'opencode\cp3\riltra-before.exe' }
if (-not $BinAfter) { $BinAfter = Join-Path $env:TEMP 'opencode\cp3\riltra-after.exe' }
if (-not $Out) { $Out = Join-Path $env:TEMP 'opencode\cp3\raw\queries' }
foreach ($p in @($HomeDir, $BinBefore, $BinAfter)) {
  if (-not (Test-Path -LiteralPath $p)) { throw "missing path: $p" }
}
$env:RILTRA_HOME = $HomeDir
New-Item -ItemType Directory -Force -Path $Out | Out-Null

# CP2's set, verbatim, so the pairing is exact.
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
  @{ n = 'p1-callers-class-PatchMatchCuda'; a = @('callers','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match_cuda.h','--line','49','--col','1') },
  @{ n = 'p2-callers-class-PatchMatch';      a = @('callers','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.h','--line','129','--col','1') },
  @{ n = 'p3-search-qualified-Print';        a = @('search','--repo','mmsp-pin','--symbol','PatchMatch::Problem::Print') }
)

# E8-sensitive anchors: a C++ header definition reached from other
# translation units through a rooted include.
$e8 = @(
  @{ n = 'e8-1-callers-PatchMatch-class';    a = @('callers','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.h','--line','55','--col','7') },
  @{ n = 'e8-2-callers-PatchMatchCuda-class';a = @('callers','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match_cuda.h','--line','49','--col','7') },
  @{ n = 'e8-3-callers-PatchMatchOptions';   a = @('callers','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match_options.h','--line','37','--col','7') },
  @{ n = 'e8-4-callers-Controller';          a = @('callers','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.h','--line','129','--col','7') },
  @{ n = 'e8-5-callees-PatchMatchRun';       a = @('callees','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.cc','--line','191','--col','18') },
  @{ n = 'e8-6-impact-PatchMatch-class';     a = @('impact','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.h','--line','55','--col','7') }
)
$all = $queries + $e8

function Invoke-Set {
  param([string]$Bin, [string]$Dir)
  New-Item -ItemType Directory -Force -Path $Dir | Out-Null
  $summary = @()
  foreach ($q in $all) {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $outText = & $Bin @($q.a) 2>&1 | Out-String
    $code = $LASTEXITCODE
    $sw.Stop()
    $path = Join-Path $Dir ($q.n + '.json')
    [System.IO.File]::WriteAllText($path, $outText, (New-Object System.Text.UTF8Encoding($false)))
    $summary += [pscustomobject]@{
      name = $q.n; ms = $sw.ElapsedMilliseconds; exit = $code; bytes = $outText.Length
    }
    "== $($q.n)  exit=$code  $($sw.ElapsedMilliseconds) ms  $($outText.Length) bytes"
  }
  $summary | ConvertTo-Json | Set-Content -Path (Join-Path $Dir '_summary.json') -Encoding utf8
  return $summary
}

"===== BEFORE ====="
$before = Invoke-Set -Bin $BinBefore -Dir (Join-Path $Out 'before')
"===== AFTER ====="
$after = Invoke-Set -Bin $BinAfter -Dir (Join-Path $Out 'after')

# The pairing table: what changed, and by how much.
$rows = @()
foreach ($b in $before) {
  $a = $after | Where-Object { $_.name -eq $b.name }
  $bp = Join-Path (Join-Path $Out 'before') ($b.name + '.json')
  $ap = Join-Path (Join-Path $Out 'after') ($b.name + '.json')
  $changed = if ((Test-Path $bp) -and (Test-Path $ap)) {
    (Get-FileHash $bp -Algorithm SHA256).Hash -ne (Get-FileHash $ap -Algorithm SHA256).Hash
  } else { $false }
  $rows += [pscustomobject]@{
    name = $b.name
    before_bytes = $b.bytes
    after_bytes = $a.bytes
    before_ms = $b.ms
    after_ms = $a.ms
    exit_before = $b.exit
    exit_after = $a.exit
    changed = $changed
  }
}
$rows | ConvertTo-Json | Set-Content -Path (Join-Path $Out 'pairing.json') -Encoding utf8
"===== pairing ====="
$rows | Format-Table -AutoSize | Out-String
