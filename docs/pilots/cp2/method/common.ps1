# CP2 harness: shared path-safety helpers.
#
# Rules for every destructive operation in this harness:
#  - never use a PowerShell automatic variable name ($HOME, $PID, $PWD, ...)
#  - every delete, reset, clean or overwrite goes through a guard here
#  - containment is checked COMPONENT BY COMPONENT on the RESOLVED path, so a
#    sibling whose name merely starts with the bench root's name (for example
#    "bench-evil" next to "bench") is outside, and a junction or symlink
#    inside the bench root that points outside resolves to an outside path
#  - the bench root itself is never a mass-delete target
#  - the original repository and the pilot snapshot are never delete targets;
#    the snapshot may only be mutated through the guarded git helpers
#
# The step scripts take their paths as parameters; nothing here hardcodes a
# machine-local path.

function Resolve-RealPath {
    # Full path with reparse points (junctions, symlinks) resolved. A link
    # can sit at ANY component, not only at the leaf, so the path is walked
    # component by component and every prefix is resolved before the next
    # component is appended.
    param([Parameter(Mandatory = $true)][string]$Path)

    $full = [System.IO.Path]::GetFullPath($Path)
    $root = [System.IO.Path]::GetPathRoot($full)
    if (-not $root) { return $full }
    $rest = $full.Substring($root.Length).Split('\', [System.StringSplitOptions]::RemoveEmptyEntries)
    if ($rest.Length -eq 0) { return $full }

    $resolved = $root
    foreach ($part in $rest) {
        $current = Join-Path $resolved $part
        $item = Get-Item -LiteralPath $current -Force -ErrorAction SilentlyContinue
        if (-not $item) {
            # The component does not exist: keep it lexically.
            $resolved = $current
            continue
        }
        $link = $null
        if ($item.PSObject.Properties['LinkTarget'] -and $item.LinkTarget) {
            $link = @($item.LinkTarget)[0]
        }
        elseif ($item.PSObject.Properties['Target'] -and $item.Target) {
            $link = @($item.Target)[0]
        }
        if (-not $link) {
            $resolved = $current
            continue
        }
        if (-not [System.IO.Path]::IsPathRooted($link)) {
            $link = Join-Path (Split-Path -Parent $current) $link
        }
        $resolved = [System.IO.Path]::GetFullPath($link)
    }
    return [System.IO.Path]::GetFullPath($resolved)
}

function Get-BenchPathState {
    # 'inside'  : the resolved path is strictly below the resolved root
    # 'root'    : the resolved path IS the resolved root
    # 'outside' : anything else, including a sibling with a similar prefix
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Root
    )

    $rootFull = (Resolve-RealPath $Root).TrimEnd('\')
    $pathFull = (Resolve-RealPath $Path).TrimEnd('\')
    if ($pathFull.Equals($rootFull, [System.StringComparison]::OrdinalIgnoreCase)) { return 'root' }

    $rootParts = $rootFull.Split('\')
    $pathParts = $pathFull.Split('\')
    if ($pathParts.Length -le $rootParts.Length) { return 'outside' }
    for ($i = 0; $i -lt $rootParts.Length; $i++) {
        if (-not $pathParts[$i].Equals($rootParts[$i], [System.StringComparison]::OrdinalIgnoreCase)) {
            return 'outside'
        }
    }
    return 'inside'
}

function Assert-BenchPath {
    # Returns the resolved full path, or throws. The bench root itself is
    # refused unless -AllowRoot is given.
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Root,
        [switch]$AllowRoot
    )

    $state = Get-BenchPathState -Path $Path -Root $Root
    if ($state -eq 'outside') {
        throw "refusing a path outside the bench root: $Path"
    }
    if ($state -eq 'root' -and -not $AllowRoot) {
        throw "refusing the bench root itself as a target: $Path"
    }
    return (Resolve-RealPath $Path)
}

function Reset-BenchDir {
    # The only mass-delete in the harness. Guarded: inside the bench root,
    # never the bench root itself.
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Root
    )

    $full = Assert-BenchPath -Path $Path -Root $Root
    if (Test-Path -LiteralPath $full) {
        Remove-Item -LiteralPath $full -Recurse -Force -ErrorAction SilentlyContinue
    }
    New-Item -ItemType Directory -Force -Path $full | Out-Null
    return $full
}

function New-BenchDir {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Root
    )

    $full = Assert-BenchPath -Path $Path -Root $Root
    New-Item -ItemType Directory -Force -Path $full | Out-Null
    return $full
}

function Assert-SnapshotPath {
    # The pilot snapshot may be mutated (freshness scenarios), so it must be
    # a git repository inside the bench root and must not be the original
    # repository.
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Root,
        [string]$MmspRoot = ''
    )

    $full = Assert-BenchPath -Path $Path -Root $Root
    if ($MmspRoot) {
        $orig = (Resolve-RealPath $MmspRoot).TrimEnd('\')
        if ($full.TrimEnd('\').Equals($orig, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "refusing to operate on the original repository: $full"
        }
    }
    if (-not (Test-Path -LiteralPath (Join-Path $full '.git'))) {
        throw "the snapshot is not a git repository: $full"
    }
    return $full
}

function Restore-Snapshot {
    # Restores the pilot snapshot to the frozen pin. Guarded by
    # Assert-SnapshotPath before any destructive git command runs.
    param(
        [Parameter(Mandatory = $true)][string]$Snapshot,
        [Parameter(Mandatory = $true)][string]$Root,
        [Parameter(Mandatory = $true)][string]$Pin,
        [string]$MmspRoot = ''
    )

    $snap = Assert-SnapshotPath -Path $Snapshot -Root $Root -MmspRoot $MmspRoot
    git -C $snap checkout --quiet --detach $Pin | Out-Null
    git -C $snap reset --quiet --hard $Pin | Out-Null
    git -C $snap clean -qfd | Out-Null
    git -C $snap branch -D m9-freshness-probe 2>&1 | Out-Null
    return $snap
}

function Get-DefaultBin {
    # The harness lives in <repo>/docs/pilots/cp2/method, so the binary is
    # four levels up. No machine-local path is hardcoded.
    param([string]$Profile = 'debug')

    return (Join-Path $PSScriptRoot ('..\..\..\..\target\' + $Profile + '\riltra.exe'))
}

function Initialize-Cp2Harness {
    # Resolves the harness configuration from the caller's parameters. Every
    # path is a parameter; nothing machine-local is baked in.
    param(
        [string]$BenchRoot = '',
        [string]$Bin = '',
        [string]$Snapshot = '',
        [string]$MmspRoot = '',
        [string]$Out = '',
        [string]$Profile = 'debug'
    )

    if (-not $BenchRoot) { $BenchRoot = Join-Path $env:TEMP 'opencode\bench' }
    if (-not $Bin) { $Bin = Get-DefaultBin -Profile $Profile }
    if (-not $Snapshot) { $Snapshot = Join-Path $BenchRoot 'mmsp-d62f06a' }
    if (-not $Out) { $Out = Join-Path $BenchRoot 'cp2\raw' }

    $config = @{
        BenchRoot = [System.IO.Path]::GetFullPath($BenchRoot)
        Bin       = [System.IO.Path]::GetFullPath($Bin)
        Snapshot  = [System.IO.Path]::GetFullPath($Snapshot)
        MmspRoot  = if ($MmspRoot) { [System.IO.Path]::GetFullPath($MmspRoot) } else { '' }
        Out       = [System.IO.Path]::GetFullPath($Out)
    }
    if (-not (Test-Path -LiteralPath $config.Bin)) {
        throw "riltra binary not found: $($config.Bin)"
    }
    return $config
}
