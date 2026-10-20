# SPDX-License-Identifier: Apache-2.0
# The index readiness race.
#
# clangd builds a project index in the background. The first question asked
# after the index has been dropped can therefore be answered from the
# current translation unit alone - which yields the DECLARATION - while a
# later question, asked once the index exists, yields the DEFINITION.
#
# The index cache is written to `.cache/clangd/index/` in the PROJECT ROOT,
# which is the parent of the directory that holds the compilation database
# (for the fixture: <fixture>/.cache, for MMSP: <mmsp>/.cache, for colmap:
# <mmsp>/colmap/.cache). The script prints the path it clears, so the
# evidence records it.
#
# Each cycle: drop the index, ask once (the answer can differ), ask again
# (the index exists by now). The control cycle asks without dropping
# anything, so the difference is attributable to the index and not to the
# clock.
#
# Usage:
#   pwsh -File readiness.ps1 -Root <bench> -Server <clangd> -Out <out.json>
param(
  [Parameter(Mandatory = $true)][string]$Root,
  [Parameter(Mandatory = $true)][string]$Server,
  [Parameter(Mandatory = $true)][string]$Out,
  [int]$Rounds = 5
)
$ErrorActionPreference = 'Stop'
$Riltra = Join-Path $Root 'riltra.exe'
$env:RILTRA_HOME = Join-Path $Root 'home'
$cache = Join-Path $Root 'fixture\.cache\clangd\index'
$report = [ordered]@{}
$report.index_cache_cleared = $cache

function Invoke-Query {
  param([string]$Repo, [string]$Path, [int]$Line, [int]$Col)
  $cliArgs = @('semantic', 'query', '--repo', $Repo, '--path', $Path, '--line', "$Line", '--col', "$Col",
    '--lsp-server', $Server)
  $text = (& $Riltra @cliArgs 2>&1 | Out-String).Trim()
  $target = $null
  if ($text -match '"path":"([^"]+)"') { $target = $Matches[1] }
  return [ordered]@{ target = $target }
}

$cycles = @()
for ($r = 1; $r -le $Rounds; $r++) {
  if (Test-Path -LiteralPath $cache) { Remove-Item -LiteralPath $cache -Recurse -Force }
  $first = Invoke-Query -Repo 'fixture' -Path 'src/use_point.cc' -Line 9 -Col 27
  $second = Invoke-Query -Repo 'fixture' -Path 'src/use_point.cc' -Line 9 -Col 27
  $cycles += [ordered]@{
    round = $r
    index_dropped = $true
    first_after_drop = $first
    second = $second
  }
}

# The control: nothing is dropped, and the index is already there.
$control = @()
for ($r = 1; $r -le 3; $r++) {
  $q = Invoke-Query -Repo 'fixture' -Path 'src/use_point.cc' -Line 9 -Col 27
  $control += [ordered]@{ round = $r; index_dropped = $false; answer = $q }
}
$report.rounds = $cycles
$report.control = $control
$report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $Out -Encoding utf8
"wrote $Out"
