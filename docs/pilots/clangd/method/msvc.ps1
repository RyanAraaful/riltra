# SPDX-License-Identifier: Apache-2.0
# Locate the Visual Studio C++ toolchain without hard-coding a path, so the
# pilot harness works on any host that has the Build Tools installed.
#
# Dot-source it, then call Get-MsvcPaths:
#   . (Join-Path $PSScriptRoot 'msvc.ps1')
#   $msvc = Get-MsvcPaths
#   $msvc.cl      # the full path of cl.exe
#   $msvc.vcvars  # the full path of vcvars64.bat
function Get-MsvcPaths {
  $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
  if (-not (Test-Path -LiteralPath $vswhere)) { throw "vswhere not found: $vswhere" }
  $vcvars = & $vswhere -latest -products * -find 'VC\Auxiliary\Build\vcvars64.bat'
  $cl = & $vswhere -latest -products * -find 'VC\Tools\MSVC\**\bin\Hostx64\x64\cl.exe'
  if (-not $vcvars) { throw 'no MSVC toolchain found (vcvars64.bat)' }
  if (-not $cl) { throw 'no MSVC toolchain found (cl.exe)' }
  return [ordered]@{
    vcvars = ($vcvars | Select-Object -First 1)
    cl = ($cl | Select-Object -First 1)
  }
}
