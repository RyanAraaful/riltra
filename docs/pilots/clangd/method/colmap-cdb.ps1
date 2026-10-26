# SPDX-License-Identifier: Apache-2.0
# A RECONSTRUCTED compilation database for the colmap sources vendored in
# MMSP. It is NOT the real build configuration, and it must never be treated
# as one.
#
# Why it is a reconstruction:
#   - colmap's own CMakeLists declares C++17 (line 125) and the include root
#     `src` (line 309); those two facts are taken from the project itself.
#   - everything else in a real colmap build comes from dependencies that are
#     NOT installed here (Eigen, glog, Ceres, Boost, FLANN, FreeImage) and
#     from optional features (CASPAR, CUDA, HIP) whose `add_compile_definitions`
#     calls only fire when those dependencies are found. colmap's CMake also
#     uses FetchContent (FETCH_POSELIB / FETCH_FAISS / FETCH_ONNX), so
#     configuring it would download from the network: not authorized here.
#   - Therefore the defines and the dependency include paths of a real build
#     are NOT reproduced, and the parse of these files will be incomplete.
#
# The point of measuring it is exactly to show the difference between a
# reconstruction and the real configuration.
#
# Usage:
#   pwsh -File colmap-cdb.ps1 -Clone <clone> -Out <compile_commands.json>
param(
  [Parameter(Mandatory = $true)][string]$Clone,
  [Parameter(Mandatory = $true)][string]$Out
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'msvc.ps1')
$msvc = Get-MsvcPaths
$cl = $msvc.cl

$root = Join-Path $Clone 'colmap'
if (-not (Test-Path -LiteralPath (Join-Path $root 'src'))) { throw "colmap sources not found under $root" }

$files = @(
  'src/colmap/exe/mvs.cc',
  'src/colmap/ui/dense_reconstruction_widget.cc'
)
$db = @()
foreach ($f in $files) {
  $abs = Join-Path $root ($f -replace '/', '\')
  if (-not (Test-Path -LiteralPath $abs)) { throw "source not found: $abs" }
  $db += [ordered]@{
    directory = ($root -replace '\\', '/')
    command = '"{0}" /nologo /EHsc /std:c++17 -I"{1}\src" -c {2}' -f $cl, $root, $f
    file = ($abs -replace '\\', '/')
  }
}
# -InputObject keeps the database an ARRAY, as a compilation database must be.
ConvertTo-Json -InputObject $db -Depth 4 | Set-Content -LiteralPath $Out -Encoding utf8
"wrote $Out ($($db.Count) reconstructed entries; RECONSTRUCTION, not the real build configuration)"
