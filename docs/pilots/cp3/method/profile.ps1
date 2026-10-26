# CP3 harness step: E9 diagnostic profiling.
#
# Bounded profiling with existing tooling only. No cache, daemon, service or
# performance refactor is introduced, and the blueprint's SLO is not changed.
#
# The stages are separated by DIFFERENCING the cost of successively heavier
# commands, because the CLI has no built-in stage timer:
#
#   A  --version                      process start + exit
#   B  repo status --repo <r>         A + git fingerprint collection
#   C  index status --repo <r>        B + database open + snapshot validation
#   D  search --repo <r> --symbol ... C + query execution
#   E  callers --repo <r> ...         C + the E8 candidate-scope query
#
# The differences are estimates, not instrumented timers. Every number is
# reported with its sample count, minimum, median and maximum so the noise
# on an idle host is visible.
#
# Usage:
#   pwsh -File profile.ps1 -HomeDir <dir> -Bin <riltra.exe> -Repo <name> -Out <dir>
param(
  [string]$HomeDir = '',
  [string]$Bin = '',
  [string]$Repo = 'mmsp-pin',
  [string]$OutDir = '',
  [int]$Reps = 7
)
$ErrorActionPreference = 'Continue'

if (-not $HomeDir) { $HomeDir = Join-Path $env:TEMP 'opencode\cp3\home' }
if (-not $Bin) { $Bin = Join-Path $env:TEMP 'opencode\cp3\riltra-after.exe' }
if (-not $OutDir) { $OutDir = Join-Path $env:TEMP 'opencode\cp3\raw\perf' }
if (-not (Test-Path -LiteralPath $Bin)) { throw "missing binary: $Bin" }
$env:RILTRA_HOME = $HomeDir
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

function Measure-Stage {
  param([string]$Id, [string]$What, [string[]]$CmdArgs)
  $samples = @()
  # One warm-up run, discarded: it pays the OS file cache and the first
  # database open, which is not what the steady-state cost is.
  & $Bin @CmdArgs 2>&1 | Out-Null
  for ($i = 0; $i -lt $Reps; $i++) {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    & $Bin @CmdArgs 2>&1 | Out-Null
    $code = $LASTEXITCODE
    $sw.Stop()
    $samples += [pscustomobject]@{ ms = $sw.Elapsed.TotalMilliseconds; exit = $code }
  }
  $sorted = @($samples | Sort-Object ms)
  $median = $sorted[[int][math]::Floor($sorted.Count / 2)].ms
  [pscustomobject]@{
    id = $Id
    what = $What
    args = ($CmdArgs -join ' ')
    reps = $Reps
    min_ms = [math]::Round($sorted[0].ms, 1)
    median_ms = [math]::Round($median, 1)
    max_ms = [math]::Round($sorted[-1].ms, 1)
    p95_ms = [math]::Round($sorted[[int][math]::Min($sorted.Count - 1, [math]::Ceiling(0.95 * $sorted.Count) - 1)].ms, 1)
    exits = (@($samples | ForEach-Object { $_.exit }) -join ',')
  }
}

$stages = @(
  (Measure-Stage 'A-process-start' 'process start and exit' @('--version')),
  (Measure-Stage 'B-git-fingerprint' 'git fingerprint collection' @('repo','status','--repo',$Repo)),
  (Measure-Stage 'C-db-open-validate' 'database open and snapshot validation' @('index','status','--repo',$Repo)),
  (Measure-Stage 'D-query-search' 'query execution (search)' @('search','--repo',$Repo,'--symbol','Print','--limit','50')),
  (Measure-Stage 'E-query-callers' 'query execution (callers, E8 candidate scope)' @('callers','--repo',$Repo,'--path','colmap/src/colmap/mvs/patch_match.h','--line','136','--col','8'))
)

# The stage differences, as estimates.
$a = $stages[0].median_ms
$b = $stages[1].median_ms
$c = $stages[2].median_ms
$d = $stages[3].median_ms
$e = $stages[4].median_ms

$breakdown = @(
  [pscustomobject]@{ stage = 'process start and exit';            estimate_ms = [math]::Round($a, 1) },
  [pscustomobject]@{ stage = 'git fingerprint collection';        estimate_ms = [math]::Round($b - $a, 1) },
  [pscustomobject]@{ stage = 'database open and snapshot check';  estimate_ms = [math]::Round($c - $b, 1) },
  [pscustomobject]@{ stage = 'query execution (search)';          estimate_ms = [math]::Round($d - $c, 1) },
  [pscustomobject]@{ stage = 'query execution (callers, E8)';     estimate_ms = [math]::Round($e - $c, 1) }
)

$stages | ConvertTo-Json -Depth 4 | Set-Content -Path (Join-Path $OutDir 'stages.json') -Encoding utf8
$breakdown | ConvertTo-Json -Depth 4 | Set-Content -Path (Join-Path $OutDir 'breakdown.json') -Encoding utf8

"===== stages ====="
$stages | Format-Table id, reps, min_ms, median_ms, p95_ms, max_ms, exits -AutoSize | Out-String
"===== breakdown (differenced medians) ====="
$breakdown | Format-Table -AutoSize | Out-String
