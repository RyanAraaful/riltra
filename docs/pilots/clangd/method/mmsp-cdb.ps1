# SPDX-License-Identifier: Apache-2.0
# The MMSP compilation database, derived from the repository's OWN documented
# build command.
#
# MMSP README.md:62 says:
#
#     cl /O2 /std:c++17 tools\metrics.cpp   # -> metrics.exe
#
# That is the only C++ file MMSP builds, and the README is the repository's
# own statement of how it is compiled. The database below is that command
# with `cl` resolved to the installed compiler and `/nologo` added (banner
# suppression only, no semantic effect), so that clangd can drive the
# compiler in driver mode. The script then RUNS the database's own command
# and reports the compiler's verdict, so the database is proven to be the
# real build command and not a reconstruction.
#
# Usage:
#   pwsh -File mmsp-cdb.ps1 -Clone <clone> -Out <compile_commands.json>
param(
  [Parameter(Mandatory = $true)][string]$Clone,
  [Parameter(Mandatory = $true)][string]$Out
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'msvc.ps1')
$msvc = Get-MsvcPaths
$cl = $msvc.cl
$vcvars = $msvc.vcvars

$source = Join-Path $Clone 'tools\metrics.cpp'
if (-not (Test-Path -LiteralPath $source)) { throw "source not found: $source" }
$file = $source -replace '\\', '/'

$command = '"{0}" /nologo /O2 /std:c++17 tools\metrics.cpp' -f $cl
$db = @(
  [ordered]@{
    directory = ($Clone -replace '\\', '/')
    command = $command
    file = $file
  }
)
# -InputObject keeps a one-element database an ARRAY: a compilation database
# is a JSON array, and clangd rejects anything else.
ConvertTo-Json -InputObject $db -Depth 4 | Set-Content -LiteralPath $Out -Encoding utf8
"wrote $Out"

# Prove the database: run its own command in its own directory and report
# what the compiler says. The README's command assumes a Visual Studio
# developer prompt (that is where `cl` and the standard library live), so
# the proof runs inside the same environment. The source is untouched: the
# object file goes to a temporary directory.
$tmp = Join-Path $env:TEMP ('mmsp-cdb-proof-' + [guid]::NewGuid().ToString('N').Substring(0, 8))
New-Item -ItemType Directory -Force -Path $tmp | Out-Null
$vcvars = $msvc.vcvars
$cmd = "call `"$vcvars`" >nul && cd /d `"$Clone`" && `"$cl`" /nologo /O2 /std:c++17 /Fo`"$tmp\metrics.obj`" /Fe`"$tmp\metrics.exe`" tools\metrics.cpp"
$output = cmd /c $cmd 2>&1 | Out-String
$exit = $LASTEXITCODE
$built = Test-Path -LiteralPath (Join-Path $tmp 'metrics.exe')
"compiler exit code: $exit"
"compiler output:"
$output.Trim()
"object produced: $(Test-Path (Join-Path $tmp 'metrics.obj'))"
"exe produced in tmp: $built"
if ($exit -ne 0) { throw "the documented command does not compile: exit $exit" }
