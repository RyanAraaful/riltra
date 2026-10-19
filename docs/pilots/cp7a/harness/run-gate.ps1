# SPDX-License-Identifier: Apache-2.0
# CP7-A G7 local gate. Runs the repository checks in order and records raw logs.
$ErrorActionPreference = "Continue"
$ProgressPreference = "SilentlyContinue"
$root = "E:\dev\Riltra"
$out = "E:\dev\Riltra\.cp7a"
$steps = @(
  @{ name = "fmt";    args = @("fmt", "--all", "--", "--check") },
  @{ name = "check";  args = @("check", "--locked") },
  @{ name = "clippy"; args = @("clippy", "--locked", "--all-targets", "--", "-D", "warnings") },
  @{ name = "test";   args = @("test", "--locked") }
)
$results = @()
foreach ($step in $steps) {
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  Push-Location -LiteralPath $root
  try {
    $output = & cargo @($step.args) 2>&1
    $code = $LASTEXITCODE
  } finally { Pop-Location }
  $sw.Stop()
  $log = Join-Path $out ("gate-" + $step.name + ".log")
  $output | Set-Content -LiteralPath $log -Encoding utf8NoBOM
  $results += [pscustomobject]@{
    step = $step.name
    args = ($step.args -join " ")
    exit_code = $code
    duration_ms = [math]::Round($sw.Elapsed.TotalMilliseconds, 1)
    log = $log
    tail = (@($output | Select-Object -Last 6) -join "`n")
  }
  Write-Output "GATE $($step.name) exit=$code ms=$([math]::Round($sw.Elapsed.TotalMilliseconds,1))"
}
$results | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $out "gate.json") -Encoding utf8NoBOM
$failed = @($results | Where-Object { $_.exit_code -ne 0 })
if ($failed.Count -gt 0) { exit 1 }
Write-Output "GATE all passed"
