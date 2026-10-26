# SPDX-License-Identifier: Apache-2.0
# Does the LSP answer follow the bytes on disk?
#
# The definition of `geom::norm` is deleted from point.cc while the query
# keeps asking for it. Two things can happen, and the experiment records
# which one:
#
#   - the answer moves to the declaration in the header (the definition is
#     really gone), or
#   - the answer keeps pointing at the old line, and the bytes the answer
#     names are NOT the bytes the file now has.
#
# The second case is a provenance mismatch: the adapter stamps the digest of
# the CURRENT file on a span that the current file does not contain.
#
# Each round warms the index up first (a query that must answer with the
# definition), then edits the file, then asks twice.
#
# Usage:
#   pwsh -File staleness.ps1 -Root <bench> -Server <clangd> -Out <out.json>
param(
  [Parameter(Mandatory = $true)][string]$Root,
  [Parameter(Mandatory = $true)][string]$Server,
  [Parameter(Mandatory = $true)][string]$Out,
  [int]$Rounds = 3
)
$ErrorActionPreference = 'Stop'
$Riltra = Join-Path $Root 'riltra.exe'
$env:RILTRA_HOME = Join-Path $Root 'home'
$target = Join-Path $Root 'fixture\src\point.cc'
$report = [ordered]@{}

function Invoke-Query {
  param([string]$Repo, [string]$Path, [int]$Line, [int]$Col)
  $cliArgs = @('semantic', 'query', '--repo', $Repo, '--path', $Path, '--line', "$Line", '--col', "$Col",
    '--lsp-server', $Server)
  $text = (& $Riltra @cliArgs 2>&1 | Out-String).Trim()
  $targetPath = $null
  $span = $null
  if ($text -match '"path":"([^"]+)","span":\{"start_line":(\d+),"start_col":(\d+),"end_line":(\d+),"end_col":(\d+),"byte_start":(\d+),"byte_end":(\d+)') {
    $targetPath = $Matches[1]
    $span = [ordered]@{
      start_line = [int]$Matches[2]; start_col = [int]$Matches[3]
      end_line = [int]$Matches[4]; end_col = [int]$Matches[5]
      byte_start = [int]$Matches[6]; byte_end = [int]$Matches[7]
    }
  }
  # What is actually at those bytes in the file as it is on disk NOW?
  $onDisk = $null
  if ($span) {
    $file = Join-Path $Root ('fixture\' + ($targetPath -replace '/', '\'))
    if (Test-Path -LiteralPath $file) {
      $bytes = [System.IO.File]::ReadAllBytes($file)
      $len = $span.byte_end - $span.byte_start
      if ($span.byte_start -ge 0 -and ($span.byte_start + $len) -le $bytes.Length) {
        $onDisk = [System.Text.Encoding]::UTF8.GetString($bytes, $span.byte_start, $len)
      }
    }
  }
  return [ordered]@{
    path = $targetPath
    span = $span
    bytes_on_disk_now = $onDisk
    span_matches_disk = ($onDisk -eq 'norm')
  }
}

$before = Get-Content -LiteralPath $target -Raw
$changed = $before -replace 'double norm\(const Point& p\) \{ return p\.x; \}', '// definition removed by the pilot'
$cache = Join-Path $Root 'fixture\.cache\clangd\index'
$cycles = @()
for ($r = 1; $r -le $Rounds; $r++) {
  # Warm the index up: the answer must be the definition before the edit.
  $warm = Invoke-Query -Repo 'fixture' -Path 'src/use_point.cc' -Line 9 -Col 27
  Set-Content -LiteralPath $target -Value $changed -NoNewline
  try {
    $first = Invoke-Query -Repo 'fixture' -Path 'src/use_point.cc' -Line 9 -Col 27
    $second = Invoke-Query -Repo 'fixture' -Path 'src/use_point.cc' -Line 9 -Col 27
  } finally {
    Set-Content -LiteralPath $target -Value $before -NoNewline
  }
  $cycles += [ordered]@{ round = $r; index_dropped = $false; warm = $warm; first_after_edit = $first; second_after_edit = $second }
}

# The control: the same edit, but the index is dropped first, so the server
# has to read the file again. If the index is the cause of the stale answer,
# the control must answer with the declaration instead.
$control = @()
for ($r = 1; $r -le 2; $r++) {
  $warm = Invoke-Query -Repo 'fixture' -Path 'src/use_point.cc' -Line 9 -Col 27
  if (Test-Path -LiteralPath $cache) { Remove-Item -LiteralPath $cache -Recurse -Force }
  Set-Content -LiteralPath $target -Value $changed -NoNewline
  try {
    $first = Invoke-Query -Repo 'fixture' -Path 'src/use_point.cc' -Line 9 -Col 27
  } finally {
    Set-Content -LiteralPath $target -Value $before -NoNewline
  }
  $control += [ordered]@{ round = $r; index_dropped = $true; warm = $warm; first_after_edit = $first }
}
$report.rounds = $cycles
$report.control = $control
$report.index_cache_cleared = $cache
$report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $Out -Encoding utf8
"wrote $Out"
