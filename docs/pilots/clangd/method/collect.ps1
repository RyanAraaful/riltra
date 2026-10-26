# SPDX-License-Identifier: Apache-2.0
# Collect the environment facts and the compiler ground truth of the pilot.
#
# Everything here is evidence about the ENVIRONMENT, not about Riltra:
#   - the tool versions used (clangd, cmake, the MSVC compiler)
#   - the compilation databases, verbatim
#   - the compiler's own verdict on the fixture and on MMSP's metrics.cpp
#   - clangd's own report of which compile command it used, and what it says
#     when the compilation database is missing
#
# Every command runs from the bench root, so no repository path leaks into
# the evidence.
#
# Usage:
#   pwsh -File collect.ps1 -Root <bench> -Tools <tools-root> -Out <dir>
param(
  [Parameter(Mandatory = $true)][string]$Root,
  [Parameter(Mandatory = $true)][string]$Tools,
  [Parameter(Mandatory = $true)][string]$Out
)
$ErrorActionPreference = 'Continue'
. (Join-Path $PSScriptRoot 'msvc.ps1')
$msvc = Get-MsvcPaths
$cl = $msvc.cl
$vcvars = $msvc.vcvars

New-Item -ItemType Directory -Force -Path $Out | Out-Null
Push-Location $Root

try {
  $clangd = Get-ChildItem -Path $Tools -Recurse -Filter 'clangd.exe' -ErrorAction SilentlyContinue | Select-Object -First 1
  $cmake = Get-ChildItem -Path $Tools -Recurse -Filter 'cmake.exe' -ErrorAction SilentlyContinue | Select-Object -First 1

  $envFacts = [ordered]@{
    clangd_version = ((& $clangd.FullName --version 2>&1 | Out-String).Trim())
    cmake_version = ((& $cmake.FullName --version 2>&1 | Out-String).Trim())
    compiler_path = $cl
    compiler_version = ((cmd /c "call `"$vcvars`" >nul && cl 2>&1" | Out-String).Trim() -split "`n" | Select-Object -First 1)
    windows_sdk = (Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\Include' -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Name)
    rustc = (& rustc --version 2>&1 | Out-String).Trim()
    host_os = [System.Environment]::OSVersion.VersionString
  }
  $envFacts | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $Out 'environment.json') -Encoding utf8

  # The compilation databases, verbatim.
  foreach ($pair in @(
      @('fixture\build\compile_commands.json', 'fixture-compile_commands.json'),
      @('mmsp\compile_commands.json', 'mmsp-compile_commands.json'),
      @('mmsp\colmap\compile_commands.json', 'colmap-compile_commands.json'))) {
    $src = Join-Path $Root $pair[0]
    if (Test-Path -LiteralPath $src) { Copy-Item -LiteralPath $src -Destination (Join-Path $Out $pair[1]) -Force }
  }

  # The full CMake build of the fixture, and the run of what it built.
  $buildLog = "call `"$vcvars`" >nul && cd /d `"$Root\fixture`" && `"$($cmake.FullName)`" --build `"$Root\fixture\build`""
  (& cmd /c $buildLog 2>&1 | Out-String) | Set-Content -LiteralPath (Join-Path $Out 'fixture-cmake-build.txt') -Encoding utf8
  $runOut = (& "$Root\fixture\build\demo.exe" 2>&1 | Out-String).Trim()
  "exit code: $LASTEXITCODE" | Set-Content -LiteralPath (Join-Path $Out 'fixture-run.txt') -Encoding utf8
  "output: $runOut" | Add-Content -LiteralPath (Join-Path $Out 'fixture-run.txt') -Encoding utf8

  # The compiler's verdict on the E12 translation unit (the two
  # static_asserts are the E12 ground truth) and on the ambiguity file.
  $fixtureBuild = "call `"$vcvars`" >nul && cd /d `"$Root\fixture`" && `"$cl`" /nologo /std:c++17 /c src\use_widget.cc /Iinclude /Fo`"$env:TEMP\cp4-widget.obj`""
  (& cmd /c $fixtureBuild 2>&1 | Out-String) | Set-Content -LiteralPath (Join-Path $Out 'fixture-e12-compiler-verdict.txt') -Encoding utf8

  $ambiguous = "call `"$vcvars`" >nul && cd /d `"$Root\fixture`" && `"$cl`" /nologo /std:c++17 /c src\ambiguous.cc /Iinclude /Fo`"$env:TEMP\cp4-ambiguous.obj`""
  (& cmd /c $ambiguous 2>&1 | Out-String) | Set-Content -LiteralPath (Join-Path $Out 'fixture-ambiguity-compiler-verdict.txt') -Encoding utf8

  # The MMSP verdict, taken from the database builder itself: it runs the
  # database's own command and reports the exit code, the object and the
  # executable.
  (& pwsh -NoProfile -File (Join-Path $PSScriptRoot 'mmsp-cdb.ps1') -Clone (Join-Path $Root 'mmsp') `
      -Out (Join-Path $Out 'mmsp-compile_commands.json') 2>&1 | Out-String) |
    Set-Content -LiteralPath (Join-Path $Out 'mmsp-cdb-proof.txt') -Encoding utf8

  # What compile command did clangd actually use?
  (& $clangd.FullName --check="$Root\fixture\src\use_point.cc" 2>&1 | Out-String) |
    Set-Content -LiteralPath (Join-Path $Out 'clangd-check-fixture.txt') -Encoding utf8
  (& $clangd.FullName --check="$Root\mmsp\tools\metrics.cpp" 2>&1 | Out-String) |
    Set-Content -LiteralPath (Join-Path $Out 'clangd-check-mmsp.txt') -Encoding utf8

  # What does clangd say when the compilation database is missing? This is
  # the payload that the Riltra adapter currently discards.
  $cdb = Join-Path $Root 'fixture\build\compile_commands.json'
  $stash = Join-Path $Root 'collect-cdb-stash.json'
  Move-Item -LiteralPath $cdb -Destination $stash -Force
  try {
    (& $clangd.FullName --check="$Root\fixture\src\use_point.cc" 2>&1 | Out-String) |
      Set-Content -LiteralPath (Join-Path $Out 'clangd-check-without-database.txt') -Encoding utf8
    # The real LSP payload, captured with the protocol itself.
    (& node (Join-Path $PSScriptRoot 'capture-diagnostics.cjs') --server $clangd.FullName `
        --root (Join-Path $Root 'fixture') --file (Join-Path $Root 'fixture\src\use_point.cc') `
        --wait 6000 2>&1 | Out-String) |
      Set-Content -LiteralPath (Join-Path $Out 'lsp-diagnostics-without-database.json') -Encoding utf8
  } finally {
    Move-Item -LiteralPath $stash -Destination $cdb -Force
  }
  (& node (Join-Path $PSScriptRoot 'capture-diagnostics.cjs') --server $clangd.FullName `
      --root (Join-Path $Root 'fixture') --file (Join-Path $Root 'fixture\src\use_point.cc') `
      --wait 6000 2>&1 | Out-String) |
    Set-Content -LiteralPath (Join-Path $Out 'lsp-diagnostics-with-database.json') -Encoding utf8
} finally {
  Pop-Location
}

"wrote the environment evidence to $Out"
