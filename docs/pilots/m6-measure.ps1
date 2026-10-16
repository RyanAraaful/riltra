# SPDX-License-Identifier: Apache-2.0
# Read a launch description on stdin; measure the direct process, not descendants.
$ErrorActionPreference = 'Stop'
$launch = [Console]::In.ReadToEnd() | ConvertFrom-Json
$info = [System.Diagnostics.ProcessStartInfo]::new()
$info.FileName = $launch.exe
$info.WorkingDirectory = $launch.cwd
$info.UseShellExecute = $false
$info.CreateNoWindow = $true
$info.RedirectStandardOutput = $true
$info.RedirectStandardError = $true
foreach ($argument in $launch.args) { $info.ArgumentList.Add($argument) }
$process = [System.Diagnostics.Process]::new()
$process.StartInfo = $info
$timer = [System.Diagnostics.Stopwatch]::StartNew()
$null = $process.Start()
$stdout = $process.StandardOutput.ReadToEndAsync()
$stderr = $process.StandardError.ReadToEndAsync()
$peak = 0L
while (-not $process.WaitForExit(1)) {
    try {
        $process.Refresh()
        $peak = [Math]::Max($peak, $process.PeakWorkingSet64)
    } catch { }
}
$timer.Stop()
$process.WaitForExit()
[ordered]@{
    exit = $process.ExitCode
    stdout = $stdout.GetAwaiter().GetResult()
    stderr = $stderr.GetAwaiter().GetResult()
    elapsed_ms = [Math]::Round($timer.Elapsed.TotalMilliseconds, 3)
    peak_working_set_bytes = $(if ($peak -gt 0) { $peak } else { $null })
} | ConvertTo-Json -Depth 4 -Compress
$process.Dispose()
