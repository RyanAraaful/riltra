# SPDX-License-Identifier: Apache-2.0
# Does the compilation database decide whether the answer is complete?
#
# The definition of `geom::norm` lives in src/point.cc; the query is asked
# from src/use_point.cc. Three databases are compared:
#   - the full one,
#   - one without src/point.cc (the definition's translation unit),
#   - one without src/use_point.cc (the queried translation unit).
# The removals are anchored, so the wildcard cannot take the other file with
# it, and each variant removes exactly one entry.
#
# Usage:
#   pwsh -File index-coverage.ps1 -Root <bench> -Server <clangd> -Out <out.json>
param(
  [Parameter(Mandatory = $true)][string]$Root,
  [Parameter(Mandatory = $true)][string]$Server,
  [Parameter(Mandatory = $true)][string]$Out
)
$ErrorActionPreference = 'Stop'
$Riltra = Join-Path $Root 'riltra.exe'
$env:RILTRA_HOME = Join-Path $Root 'home'
$cdb = Join-Path $Root 'fixture\build\compile_commands.json'
$cache = Join-Path $Root 'fixture\.cache\clangd\index'
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
  return [ordered]@{ elapsed_ms = [int]$sw.ElapsedMilliseconds; target = $target }
}

function Clear-Index {
  if (Test-Path -LiteralPath $cache) { Remove-Item -LiteralPath $cache -Recurse -Force }
}

# Each variant is measured SEVERAL times after the index cache is cleared,
# because the project index is built in the background: the first answer
# can be the declaration (the index is not ready yet) while a later one is
# the definition. Running each variant more than once separates "the index
# was not ready" from "the database does not cover the definition".
function Measure-Variant {
  param([string]$Label)
  $runs = @()
  for ($i = 1; $i -le 4; $i++) {
    $q = Invoke-Query -Repo 'fixture' -Path 'src/use_point.cc' -Line 9 -Col 27
    $runs += [ordered]@{ run = $i; elapsed_ms = $q.elapsed_ms; target = $q.target }
  }
  return [ordered]@{ variant = $Label; runs = $runs }
}

# 1. The full database.
$full = Get-Content -LiteralPath $cdb -Raw | ConvertFrom-Json
Clear-Index
$report.database_complete = Measure-Variant -Label 'complete'

# 2. The database without the definition's translation unit.
$withoutDefinition = @($full | Where-Object { $_.file -notmatch '[\\/]src[\\/]point\.cc$' })
$withoutDefinition | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $cdb -Encoding utf8
try {
  Clear-Index
  $report.database_without_the_definition_tu = Measure-Variant -Label 'without src/point.cc'
} finally {
  $full | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $cdb -Encoding utf8
}

# 3. The control: the database without the QUERIED translation unit.
$withoutQueried = @($full | Where-Object { $_.file -notmatch '[\\/]src[\\/]use_point\.cc$' })
$withoutQueried | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $cdb -Encoding utf8
try {
  Clear-Index
  $report.database_without_the_queried_tu = Measure-Variant -Label 'without src/use_point.cc'
} finally {
  $full | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $cdb -Encoding utf8
  Clear-Index
}

$report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $Out -Encoding utf8
"wrote $Out"
