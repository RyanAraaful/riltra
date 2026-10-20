# CP2 harness: regression tests for the path-safety guard.
#
# The guard must accept a real subdirectory of the bench root and refuse
# everything else: the bench root itself, a parent, a sibling whose name
# starts with the bench root's name, and a junction inside the bench root
# that points outside it.
#
# Usage: pwsh -File test-path-safety.ps1
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'common.ps1')

$failures = @()
function Check([string]$name, [bool]$condition, [string]$detail) {
    if ($condition) {
        "  PASS  $name"
    }
    else {
        "  FAIL  $name  ($detail)"
        $script:failures += $name
    }
}

function Throws([scriptblock]$action) {
    try { & $action | Out-Null; return $false } catch { return $true }
}

# A private bench root for the test, plus a sibling that shares its prefix.
$tmp = Join-Path $env:TEMP ('cp2-pathsafety-' + [Guid]::NewGuid().ToString('N').Substring(0, 8))
$benchRoot = Join-Path $tmp 'bench'
$sibling = Join-Path $tmp 'bench-evil'
$outside = Join-Path $tmp 'outside'
New-Item -ItemType Directory -Force -Path $benchRoot, $sibling, $outside | Out-Null
$valid = Join-Path $benchRoot 'home-main'
New-Item -ItemType Directory -Force -Path $valid | Out-Null
$junction = Join-Path $benchRoot 'escape'
try { New-Item -ItemType Junction -Path $junction -Target $outside | Out-Null } catch { $junction = '' }
# A link can sit at an INTERMEDIATE component, not only at the leaf.
$linkDir = Join-Path $benchRoot 'links'
try { New-Item -ItemType Junction -Path $linkDir -Target $outside | Out-Null } catch { $linkDir = '' }
$throughLink = if ($linkDir) { Join-Path $linkDir 'sub' } else { '' }

try {
    "===== path states ====="
    Check "a real subdirectory is inside" ((Get-BenchPathState -Path $valid -Root $benchRoot) -eq 'inside') "state"
    Check "a sibling with a similar prefix is outside" ((Get-BenchPathState -Path $sibling -Root $benchRoot) -eq 'outside') "state"
    Check "the parent is outside" ((Get-BenchPathState -Path $tmp -Root $benchRoot) -eq 'outside') "state"
    Check "the bench root itself is the root" ((Get-BenchPathState -Path $benchRoot -Root $benchRoot) -eq 'root') "state"
    if ($junction) {
        Check "a junction pointing outside is outside" ((Get-BenchPathState -Path $junction -Root $benchRoot) -eq 'outside') "state"
    }
    if ($throughLink) {
        Check "a path through an intermediate junction is outside" ((Get-BenchPathState -Path $throughLink -Root $benchRoot) -eq 'outside') "state"
    }

    "===== the guard ====="
    Check "Assert-BenchPath accepts a subdirectory" (-not (Throws { Assert-BenchPath -Path $valid -Root $benchRoot | Out-Null })) "threw"
    Check "Assert-BenchPath refuses the bench root" (Throws { Assert-BenchPath -Path $benchRoot -Root $benchRoot | Out-Null }) "did not throw"
    Check "Assert-BenchPath accepts the bench root with -AllowRoot" (-not (Throws { Assert-BenchPath -Path $benchRoot -Root $benchRoot -AllowRoot | Out-Null })) "threw"
    Check "Assert-BenchPath refuses a sibling" (Throws { Assert-BenchPath -Path $sibling -Root $benchRoot | Out-Null }) "did not throw"
    Check "Assert-BenchPath refuses the parent" (Throws { Assert-BenchPath -Path $tmp -Root $benchRoot | Out-Null }) "did not throw"
    if ($junction) {
        Check "Assert-BenchPath refuses a junction pointing outside" (Throws { Assert-BenchPath -Path $junction -Root $benchRoot | Out-Null }) "did not throw"
    }
    if ($throughLink) {
        Check "Assert-BenchPath refuses a path through an intermediate junction" (Throws { Assert-BenchPath -Path $throughLink -Root $benchRoot | Out-Null }) "did not throw"
    }

    "===== the only mass-delete ====="
    Check "Reset-BenchDir refuses the bench root" (Throws { Reset-BenchDir -Path $benchRoot -Root $benchRoot | Out-Null }) "did not throw"
    Check "Reset-BenchDir refuses a sibling" (Throws { Reset-BenchDir -Path $sibling -Root $benchRoot | Out-Null }) "did not throw"
    Check "Reset-BenchDir refuses the parent" (Throws { Reset-BenchDir -Path $tmp -Root $benchRoot | Out-Null }) "did not throw"
    if ($junction) {
        Check "Reset-BenchDir refuses a junction pointing outside" (Throws { Reset-BenchDir -Path $junction -Root $benchRoot | Out-Null }) "did not throw"
    }
    if ($throughLink) {
        Check "Reset-BenchDir refuses a path through an intermediate junction" (Throws { Reset-BenchDir -Path $throughLink -Root $benchRoot | Out-Null }) "did not throw"
    }
    Check "the sibling survived every refused delete" (Test-Path -LiteralPath $sibling) "the sibling was deleted"
    Check "the parent survived every refused delete" (Test-Path -LiteralPath $tmp) "the parent was deleted"

    $marker = Join-Path $valid 'marker.txt'
    Set-Content -Path $marker -Value 'x'
    Reset-BenchDir -Path $valid -Root $benchRoot | Out-Null
    Check "Reset-BenchDir recreates an accepted directory" ((Test-Path -LiteralPath $valid) -and -not (Test-Path -LiteralPath $marker)) "not recreated"
}
finally {
    # Cleanup is guarded the same way: the private test root is below the
    # temp directory, never the temp directory itself.
    if ((Get-BenchPathState -Path $tmp -Root (Split-Path -Parent $tmp)) -eq 'inside') {
        Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
    }
}

"===== result ====="
if ($failures.Count -eq 0) {
    "ALL PATH-SAFETY CHECKS PASSED"
    exit 0
}
"FAILED: $($failures -join ', ')"
exit 1
