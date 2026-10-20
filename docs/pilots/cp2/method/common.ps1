# CP2 shared helpers (outside the repository).
# Guard rules:
#  - never use a PowerShell automatic variable name ($HOME, $PID, $PWD, ...)
#  - never remove a path that is not under the bench root
$script:BenchRoot = [System.IO.Path]::GetFullPath("$env:TEMP\opencode\bench")

function Assert-BenchPath([string]$Path) {
  $full = [System.IO.Path]::GetFullPath($Path)
  if (-not $full.StartsWith($script:BenchRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to touch a path outside the bench root: $full"
  }
  return $full
}

function Reset-BenchDir([string]$Path) {
  $full = Assert-BenchPath $Path
  Remove-Item -LiteralPath $full -Recurse -Force -ErrorAction SilentlyContinue
  New-Item -ItemType Directory -Force -Path $full | Out-Null
  return $full
}

function New-BenchDir([string]$Path) {
  $full = Assert-BenchPath $Path
  New-Item -ItemType Directory -Force -Path $full | Out-Null
  return $full
}
