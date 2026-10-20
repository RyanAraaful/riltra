# CP2 harness step: two independent cold-index measurements (D-M9-3).
#
# Usage:
#   pwsh -File cold-index.ps1 -BenchRoot <dir> -Bin <riltra.exe> -Snapshot <dir>
param(
  [string]$BenchRoot = '',
  [string]$Bin = '',
  [string]$Snapshot = '',
  [string]$Out = ''
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'common.ps1')
$cfg = Initialize-Cp2Harness -BenchRoot $BenchRoot -Bin $Bin -Snapshot $Snapshot -Out $Out
$snap = Assert-SnapshotPath -Path $cfg.Snapshot -Root $cfg.BenchRoot

foreach ($n in 1, 2) {
  $homeDir = Reset-BenchDir (Join-Path $cfg.BenchRoot "home-cold$n") -Root $cfg.BenchRoot
  $env:RILTRA_HOME = $homeDir

  "===== cold run $n : init ====="
  & $cfg.Bin init --repo $snap --name mmsp-pin
  "init exit=$LASTEXITCODE"

  "===== cold run $n : index --full ====="
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  & $cfg.Bin index --repo mmsp-pin --full
  $code = $LASTEXITCODE
  $sw.Stop()
  "index exit=$code wall_s=$([math]::Round($sw.Elapsed.TotalSeconds,3))"

  "===== cold run $n : index size ====="
  $dbFiles = Get-ChildItem $homeDir -Recurse -File -Filter *.db -ErrorAction SilentlyContinue
  $dbFiles | ForEach-Object { "  db=$($_.Name) bytes=$($_.Length)" }
  $all = Get-ChildItem $homeDir -Recurse -File
  "  home_total_bytes=$(($all | Measure-Object -Property Length -Sum).Sum) files=$($all.Count)"

  "===== cold run $n : repo status ====="
  & $cfg.Bin repo status --repo mmsp-pin
  "===== cold run $n : index status ====="
  & $cfg.Bin index status --repo mmsp-pin
}
