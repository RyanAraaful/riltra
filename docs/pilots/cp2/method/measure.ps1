# CP2 measurement harness (outside the repository).
# Usage: measure.ps1 -Label <name> -Out <file> -Command <scriptblock>
# Records wall time, peak RSS (PeakWorkingSet64) and stdout/stderr to <file>.
# The command body is written to a temp script and executed with `pwsh -File`,
# so nested quoting in the caller's scriptblock cannot corrupt the child args.
param(
  [Parameter(Mandatory = $true)][string]$Label,
  [Parameter(Mandatory = $true)][string]$Out,
  [Parameter(Mandatory = $true)][scriptblock]$Command
)

$scriptPath = Join-Path $env:TEMP ("opencode\bench\cp2\_run_" + [Guid]::NewGuid().ToString('N') + ".ps1")
$body = @"
Set-Location '$env:TEMP\opencode\bench\cp2'
`$ErrorActionPreference = 'Continue'
"@ + "`n" + ($Command.ToString())
[System.IO.File]::WriteAllText($scriptPath, $body, (New-Object System.Text.UTF8Encoding($false)))

$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.UseShellExecute = $false
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError = $true
$psi.FileName = (Get-Command pwsh -ErrorAction SilentlyContinue).Source
if (-not $psi.FileName) { $psi.FileName = (Get-Command powershell).Source }
$psi.Arguments = "-NoProfile -NonInteractive -ExecutionPolicy Bypass -File `"$scriptPath`""

$sw = [System.Diagnostics.Stopwatch]::StartNew()
$proc = [System.Diagnostics.Process]::Start($psi)
$outTask = $proc.StandardOutput.ReadToEndAsync()
$errTask = $proc.StandardError.ReadToEndAsync()

$peak = [int64]0
while (-not $proc.HasExited) {
  try {
    $proc.Refresh()
    $ws = $proc.PeakWorkingSet64
    if ($ws -gt $peak) { $peak = $ws }
  } catch { }
  Start-Sleep -Milliseconds 50
}
try { $proc.WaitForExit() } catch { }
$sw.Stop()
try { if ($proc.PeakWorkingSet64 -gt $peak) { $peak = $proc.PeakWorkingSet64 } } catch { }

$stdout = $outTask.Result
$stderr = $errTask.Result
$exit = $proc.ExitCode
Remove-Item $scriptPath -Force -ErrorAction SilentlyContinue

$meta = [ordered]@{
  label            = $Label
  wall_ms          = $sw.ElapsedMilliseconds
  wall_s           = [math]::Round($sw.Elapsed.TotalSeconds, 3)
  peak_working_set = $peak
  peak_rss_mib     = [math]::Round($peak / 1MB, 1)
  exit_code        = $exit
  finished_at      = (Get-Date).ToString('o')
}

$header = '#META ' + ($meta | ConvertTo-Json -Compress)
[System.IO.File]::WriteAllText($Out, "$header`n$stdout`n#STDERR`n$stderr", (New-Object System.Text.UTF8Encoding($false)))
$meta | ConvertTo-Json -Compress
