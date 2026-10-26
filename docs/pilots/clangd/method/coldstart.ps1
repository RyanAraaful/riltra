# SPDX-License-Identifier: Apache-2.0
# The cold start: what the answer is before clangd has built its project
# index.
#
# The project index is cached next to the compilation database, and the
# standard library headers are cached in the user cache directory. Both are
# cleared here, so the first runs have to build the index from scratch, and
# the flip from "declaration" to "definition" can be observed.
#
# Usage:
#   pwsh -File coldstart.ps1 -Root <bench> -Server <clangd> -Out <out.json>
param(
  [Parameter(Mandatory = $true)][string]$Root,
  [Parameter(Mandatory = $true)][string]$Server,
  [Parameter(Mandatory = $true)][string]$Out
)
$ErrorActionPreference = 'Stop'
$Riltra = Join-Path $Root 'riltra.exe'
$env:RILTRA_HOME = Join-Path $Root 'home'
$report = [ordered]@{}

function Invoke-Query {
  param([string]$Repo, [string]$Path, [int]$Line, [int]$Col)
  $cliArgs = @('semantic', 'query', '--repo', $Repo, '--path', $Path, '--line', "$Line", '--col', "$Col",
    '--lsp-server', $Server)
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  $text = (& $Riltra @cliArgs 2>&1 | Out-String).Trim()
  $sw.Stop()
  $target = $null
  if ($text -match '"path":"([^"]+)"') { $target = $Matches[1] }
  return [ordered]@{ elapsed_ms = [int]$sw.ElapsedMilliseconds; target = $target; output = $text }
}

$projectCache = Join-Path $Root 'fixture\.cache\clangd\index'
$stdlibCache = Join-Path $env:LOCALAPPDATA 'clangd\index'
$stashProject = Join-Path $Root 'stash-project-index'
$stashStdlib = Join-Path $Root 'stash-stdlib-index'

foreach ($pair in @(@($projectCache, $stashProject), @($stdlibCache, $stashStdlib))) {
  if (Test-Path -LiteralPath $pair[0]) { Move-Item -LiteralPath $pair[0] -Destination $pair[1] -Force }
}
try {
  $runs = @()
  for ($i = 1; $i -le 6; $i++) {
    $q = Invoke-Query -Repo 'fixture' -Path 'src/use_point.cc' -Line 9 -Col 27
    $runs += [ordered]@{ run = $i; elapsed_ms = $q.elapsed_ms; target = $q.target }
  }
  $report.cold_runs = $runs
} finally {
  foreach ($pair in @(@($projectCache, $stashProject), @($stdlibCache, $stashStdlib))) {
    if (Test-Path -LiteralPath $pair[0]) { Remove-Item -LiteralPath $pair[0] -Recurse -Force }
    if (Test-Path -LiteralPath $pair[1]) { Move-Item -LiteralPath $pair[1] -Destination $pair[0] -Force }
  }
}

$report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $Out -Encoding utf8
"wrote $Out"
