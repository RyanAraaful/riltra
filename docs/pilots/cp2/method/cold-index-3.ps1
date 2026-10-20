# CP2 harness step: a third cold index, measured directly on the riltra
# process (the harness measures the child shell; the indexer's own peak RSS
# is needed).
#
# Usage:
#   pwsh -File cold-index-3.ps1 -BenchRoot <dir> -Bin <riltra.exe> -Snapshot <dir>
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
$homeDir = Reset-BenchDir (Join-Path $cfg.BenchRoot 'home-cold3') -Root $cfg.BenchRoot

$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $cfg.Bin
$psi.Arguments = 'init --repo "' + $snap + '" --name mmsp-pin'
$psi.UseShellExecute = $false
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError = $true
$psi.Environment['RILTRA_HOME'] = $homeDir
$p = [System.Diagnostics.Process]::Start($psi)
$p.WaitForExit()
"init exit=$($p.ExitCode): $($p.StandardOutput.ReadToEnd().Trim())"

$psi2 = New-Object System.Diagnostics.ProcessStartInfo
$psi2.FileName = $cfg.Bin
$psi2.Arguments = 'index --repo mmsp-pin --full'
$psi2.UseShellExecute = $false
$psi2.RedirectStandardOutput = $true
$psi2.RedirectStandardError = $true
$psi2.Environment['RILTRA_HOME'] = $homeDir

$sw = [System.Diagnostics.Stopwatch]::StartNew()
$proc = [System.Diagnostics.Process]::Start($psi2)
$outTask = $proc.StandardOutput.ReadToEndAsync()
$errTask = $proc.StandardError.ReadToEndAsync()
$peak = [int64]0
while (-not $proc.HasExited) {
  try { $proc.Refresh(); if ($proc.PeakWorkingSet64 -gt $peak) { $peak = $proc.PeakWorkingSet64 } } catch { }
  Start-Sleep -Milliseconds 40
}
$proc.WaitForExit()
$sw.Stop()
try { if ($proc.PeakWorkingSet64 -gt $peak) { $peak = $proc.PeakWorkingSet64 } } catch { }
$stdout = $outTask.Result

$meta = [ordered]@{
  label = 'cold-index-3-direct'
  wall_s = [math]::Round($sw.Elapsed.TotalSeconds, 3)
  indexer_peak_working_set = $peak
  indexer_peak_rss_mib = [math]::Round($peak / 1MB, 1)
  exit_code = $proc.ExitCode
  finished_at = (Get-Date).ToString('o')
}
"===== META ====="
$meta | ConvertTo-Json -Compress
"===== index output ====="
$stdout
"===== db size ====="
(Get-ChildItem $homeDir -Recurse -File -Filter *.db | Measure-Object -Property Length -Sum).Sum
New-BenchDir -Path (Join-Path $cfg.Out '') -Root $cfg.BenchRoot | Out-Null
$outFile = Join-Path $cfg.Out 'cold-index-3.txt'
[System.IO.File]::WriteAllText($outFile, ("#META " + ($meta | ConvertTo-Json -Compress) + "`n" + $stdout), (New-Object System.Text.UTF8Encoding($false)))
