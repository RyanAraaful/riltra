# SPDX-License-Identifier: Apache-2.0
# The behaviour of the shipped LSP path when the world is not ideal.
#
# Four situations, each measured, not asserted:
#   1. clangd is not there at all.
#   2. there is no compilation database (the "fallback flags" case).
#   3. the source changed under the index.
#   4. the project index is cold (the answer differs from the warm one).
#
# Usage:
#   pwsh -File robustness.ps1 -Root <bench> -Server <clangd> -Out <out.json>
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
  param([string]$Repo, [string]$Path, [int]$Line, [int]$Col, [switch]$Refs, [string]$Srv)
  $cliArgs = @('semantic', 'query', '--repo', $Repo, '--path', $Path, '--line', "$Line", '--col', "$Col")
  if ($Refs) { $cliArgs += '--refs' }
  if ($Srv) { $cliArgs += @('--lsp-server', $Srv) }
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  $text = (& $Riltra @cliArgs 2>&1 | Out-String).Trim()
  $code = $LASTEXITCODE
  $sw.Stop()
  return [ordered]@{ elapsed_ms = [int]$sw.ElapsedMilliseconds; exit_code = $code; output = $text }
}

# --- 1. clangd is not there ----------------------------------------------
$missing = Join-Path $Root 'no-such-clangd.exe'
$report.clangd_absent = Invoke-Query -Repo 'fixture' -Path 'src/use_point.cc' -Line 9 -Col 27 -Srv $missing

# --- 2. no compilation database -------------------------------------------
# The fixture's database is moved aside: clangd falls back to `clang $FILE`,
# which is NOT the project's build configuration.
$cdb = Join-Path $Root 'fixture\build\compile_commands.json'
$stash = Join-Path $Root 'compile_commands.json.stash'
Move-Item -LiteralPath $cdb -Destination $stash -Force
try {
  $report.no_cdb = [ordered]@{
    definition = Invoke-Query -Repo 'fixture' -Path 'src/use_point.cc' -Line 9 -Col 27 -Srv $Server
    references = Invoke-Query -Repo 'fixture' -Path 'src/use_point.cc' -Line 11 -Col 9 -Refs -Srv $Server
  }
} finally {
  Move-Item -LiteralPath $stash -Destination $cdb -Force
}

# --- 3. the source changed under the index --------------------------------
# The live query reads the file from disk, so the answer must follow the
# current bytes. The DEFINITION of `geom::norm` is deleted from point.cc,
# leaving only the declaration in the header: a live answer must move from
# the definition to the declaration. A cached answer would not move.
$target = Join-Path $Root 'fixture\src\point.cc'
$before = Get-Content -LiteralPath $target -Raw
$changed = $before -replace 'double norm\(const Point& p\) \{ return p\.x; \}', '// definition removed by the pilot'
Set-Content -LiteralPath $target -Value $changed -NoNewline
try {
  $report.source_changed = [ordered]@{
    bytes_changed = ((Get-Content -LiteralPath $target -Raw) -ne $before)
    after_edit = Invoke-Query -Repo 'fixture' -Path 'src/use_point.cc' -Line 9 -Col 27 -Srv $Server
  }
} finally {
  Set-Content -LiteralPath $target -Value $before -NoNewline
}

# --- 4. cold index vs warm index ------------------------------------------
# The project index is cached next to the compilation database. Clearing it
# makes the first answers declaration-only; once the background index has
# caught up, the same query answers with the definition. The flip is
# measured, not assumed.
$cache = Join-Path $Root 'fixture\.cache\clangd\index'
$cacheStash = Join-Path $Root 'index-cache.stash'
if (Test-Path -LiteralPath $cache) { Move-Item -LiteralPath $cache -Destination $cacheStash -Force }
try {
  $runs = @()
  for ($i = 1; $i -le 6; $i++) {
    $q = Invoke-Query -Repo 'fixture' -Path 'src/use_point.cc' -Line 9 -Col 27 -Srv $Server
    $runs += [ordered]@{
      run = $i
      elapsed_ms = $q.elapsed_ms
      answered = ($q.output -match '"definitions":\[\{')
      target = if ($q.output -match '"path":"([^"]+)"') { $Matches[1] } else { $null }
    }
  }
  $report.cold_vs_warm = [ordered]@{ runs = $runs }
} finally {
  if (Test-Path -LiteralPath $cache) { Remove-Item -LiteralPath $cache -Recurse -Force }
  if (Test-Path -LiteralPath $cacheStash) { Move-Item -LiteralPath $cacheStash -Destination $cache -Force }
}

$report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $Out -Encoding utf8
"wrote $Out"
