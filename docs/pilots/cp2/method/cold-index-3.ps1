# CP2 step 2c: a third cold index, measured directly on the riltra process
# (the harness measures the child shell; the indexer's own peak RSS is needed).
$ErrorActionPreference = 'Stop'
. "$env:TEMP\opencode\bench\cp2\common.ps1"

$bin = '<RILTRA_ROOT>\target\debug\riltra.exe'
$snap = "$env:TEMP\opencode\bench\mmsp-d62f06a"
$homeDir = Reset-BenchDir "$env:TEMP\opencode\bench\home-cold3"

$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $bin
$psi.Arguments = 'init --repo "' + $snap + '" --name mmsp-pin'
$psi.UseShellExecute = $false
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError = $true
$psi.Environment['RILTRA_HOME'] = $homeDir
$p = [System.Diagnostics.Process]::Start($psi)
$p.WaitForExit()
"init exit=$($p.ExitCode): $($p.StandardOutput.ReadToEnd().Trim())"

$psi2 = New-Object System.Diagnostics.ProcessStartInfo
$psi2.FileName = $bin
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
$outFile = "$env:TEMP\opencode\bench\cp2\raw\cold-index-3.txt"
[System.IO.File]::WriteAllText($outFile, ("#META " + ($meta | ConvertTo-Json -Compress) + "`n" + $stdout), (New-Object System.Text.UTF8Encoding($false)))
