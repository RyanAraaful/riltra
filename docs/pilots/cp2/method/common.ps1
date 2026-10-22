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
        # A reset that did not happen would leave stale content behind, and a
        # copy into it would merge the two. Fail loudly instead.
        if (Test-Path -LiteralPath $full) {
            throw "the directory could not be reset (it is still there): $full"
        }
    }
    New-Item -ItemType Directory -Force -Path $full | Out-Null
    return $full
}

function Copy-BenchDir {
    # Copies the CONTENTS of one bench directory into another.
    #
    # A plain `Copy-Item <dir> <existing dir> -Recurse` copies the source
    # FOLDER into the destination, not its contents. The harness resets the
    # destination first (Reset-BenchDir creates it), so the destination
    # always exists and the copy always nested the source folder: the
    # registry ended up at <dest>/<source>/registry.json and every command
    # answered "unknown repository". Copying the children instead keeps the
    # destination's own root the root of the copy.
    param(
        [Parameter(Mandatory = $true)][string]$Source,
        [Parameter(Mandatory = $true)][string]$Destination,
        [Parameter(Mandatory = $true)][string]$Root
    )

    $src = Assert-BenchPath -Path $Source -Root $Root
    $dst = Assert-BenchPath -Path $Destination -Root $Root
    if (-not (Test-Path -LiteralPath $src -PathType Container)) {
        throw "the source directory does not exist: $src"
    }
    if (-not (Test-Path -LiteralPath $dst -PathType Container)) {
        throw "the destination directory does not exist: $dst"
    }
    if ($src.Equals($dst, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "refusing to copy a directory into itself: the source and the destination must be disjoint ($src)"
    }
    if ((Get-BenchPathState -Path $dst -Root $src) -ne 'outside') {
        throw "refusing to copy a directory into its own subdirectory: the source and the destination must be disjoint ($src -> $dst)"
    }
    if (@(Get-ChildItem -LiteralPath $dst -Force).Count -gt 0) {
        throw "refusing to copy into a destination that is not empty: $dst"
    }
    Get-ChildItem -LiteralPath $src -Force | Copy-Item -Destination $dst -Recurse -Force
    return $dst
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

function ConvertTo-ComparablePath {
    # The registry stores its roots with the \\?\ verbatim prefix, and
    # \\?\UNC\ for a network share. The prefix is put back into the form the
    # path stands for, so that two paths are compared in the same form. An
    # ordinary path is not changed.
    param([Parameter(Mandatory = $true)][string]$Path)

    return (($Path -replace '^\\\\\?\\UNC\\', '\\') -replace '^\\\\\?\\', '')
}

function Assert-IsolatedHome {
    # The session's RILTRA_HOME must be the isolated pilot home. It must not
    # BE the original repository, must not live inside it, and must not
    # register it: a session that can reach the original repository can
    # answer from it, which the CP1 isolation procedure forbids. (The home
    # itself is not required to be inside the bench root here; the harness
    # resolves it and compares it with the original.)
    #
    # The original repository is MANDATORY: a guard that can be switched off
    # by omitting its subject is not a guard.
    param(
        [Parameter(Mandatory = $true)][string]$HomeDir,
        [Parameter(Mandatory = $true)][string]$MmspRoot
    )

    if (-not $MmspRoot) {
        throw "refusing to run without the original repository: the isolation guard needs -MmspRoot"
    }
    # The original repository must be a real directory: Resolve-RealPath keeps
    # a path that does not exist lexically, so a typo in -MmspRoot would leave
    # the guard comparing against nothing.
    if (-not (Test-Path -LiteralPath $MmspRoot)) {
        throw "refusing to run: the original repository does not exist: $MmspRoot"
    }
    if (-not (Get-Item -LiteralPath $MmspRoot -Force).PSIsContainer) {
        throw "refusing to run: the original repository is not a directory: $MmspRoot"
    }
    # The registry stores its roots with the \\?\ verbatim prefix; a home or an
    # original given in that form is compared with the same paths.
    $homeResolved = Resolve-RealPath (ConvertTo-ComparablePath $HomeDir)
    $origResolved = Resolve-RealPath (ConvertTo-ComparablePath $MmspRoot)
    $homeFull = $homeResolved.TrimEnd('\')
    $orig = $origResolved.TrimEnd('\')
    if ($homeFull.Equals($orig, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "refusing the original repository as the session home: $homeFull"
    }
    if ($homeFull.StartsWith($orig + '\', [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "refusing a session home inside the original repository: $homeFull"
    }
    # A filesystem root contains every path on its volume, and it cannot be
    # compared component by component: the root would be reduced to a drive
    # relative path, and the verdict would depend on the working directory.
    $homeVolume = [System.IO.Path]::GetPathRoot($homeResolved)
    $origVolume = [System.IO.Path]::GetPathRoot($origResolved)
    if ($homeVolume -and $homeVolume.Equals($origVolume, [System.StringComparison]::OrdinalIgnoreCase) -and
        $homeFull.Equals($homeVolume.TrimEnd('\'), [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "refusing a session home that is a filesystem root containing the original repository: $homeVolume"
    }
    # The home must not CONTAIN the original either: a home that is an
    # ancestor of it reads the original's content recursively. The comparison
    # is component by component on the resolved paths, so a sibling whose name
    # merely starts with the original's name stays outside.
    $contains = Get-BenchPathState -Path $orig -Root $homeFull
    if ($contains -eq 'inside' -or $contains -eq 'root') {
        throw "refusing a session home that contains the original repository: $homeFull"
    }
    $registry = Join-Path $homeFull 'registry.json'
    if (Test-Path -LiteralPath $registry) {
        $repos = @()
        try {
            $repos = @((Get-Content -LiteralPath $registry -Raw | ConvertFrom-Json).repos)
        }
        catch {
            throw "refusing a session home whose registry cannot be read: $registry"
        }
        foreach ($repo in $repos) {
            if (-not $repo) { continue }
            $root = [string]$repo.root
            if (-not $root) { continue }
            # The registry stores the root with the \\?\ verbatim prefix; the
            # resolved root is compared, so a subdirectory, a parent or a
            # junction of the original is caught too.
            $normalized = (ConvertTo-ComparablePath $root).TrimEnd('\')
            $resolved = (Resolve-RealPath $normalized).TrimEnd('\')
            $state = Get-BenchPathState -Path $resolved -Root $orig
            if ($state -eq 'inside' -or $state -eq 'root') {
                throw "refusing a session home that registers the original repository: $resolved"
            }
            $reverse = Get-BenchPathState -Path $orig -Root $resolved
            if ($reverse -eq 'inside' -or $reverse -eq 'root') {
                throw "refusing a session home that registers a parent of the original repository: $resolved"
            }
        }
    }
    return $homeFull
}

function Get-SessionEvents {
    # The OpenCode JSON event stream: one JSON object per line. A line that
    # is not JSON is ignored (the transport may add one).
    param([Parameter(Mandatory = $true)][string]$StdoutFile)

    $events = @()
    if (-not (Test-Path -LiteralPath $StdoutFile)) { return $events }
    foreach ($line in (Get-Content -LiteralPath $StdoutFile -ErrorAction SilentlyContinue)) {
        if (-not $line.Trim()) { continue }
        try { $events += ($line | ConvertFrom-Json -ErrorAction Stop) } catch { }
    }
    return $events
}

function Get-SessionToolCalls {
    # The tool calls the agent actually made, read from the stream's
    # structured tool_use events - never scraped from the text. A tool name
    # that only appears in a sentence, in an error or in a tool result is
    # not a call.
    param([Parameter(Mandatory = $true)][string]$StdoutFile)

    $calls = @()
    foreach ($e in (Get-SessionEvents -StdoutFile $StdoutFile)) {
        if ($e.type -ne 'tool_use' -or -not $e.part) { continue }
        $name = [string]$e.part.tool
        if (-not $name) { continue }
        $status = [string]$e.part.state.status
        $calls += [pscustomobject]@{
            tool   = $name
            status = $(if ($status) { $status } else { 'unknown' })
        }
    }
    return $calls
}

function Get-SessionAnswer {
    # The agent's final message: the last text part of the stream. The
    # earlier text parts are intermediate notes, not the answer.
    param([Parameter(Mandatory = $true)][string]$StdoutFile)

    $answer = ''
    foreach ($e in (Get-SessionEvents -StdoutFile $StdoutFile)) {
        if ($e.type -eq 'text' -and $e.part -and $e.part.text) { $answer = [string]$e.part.text }
    }
    return $answer
}

function ConvertTo-SingleLinePrompt {
    # A command wrapper (a .cmd file) runs one command per line, so a prompt
    # with line breaks is truncated at the first break. The prompt is passed
    # as one argument with every whitespace run collapsed to a single space:
    # the instruction content is preserved, the line breaks are not.
    param([Parameter(Mandatory = $true)][string]$Prompt)

    return (($Prompt -replace '\s+', ' ').Trim())
}

function New-OpenCodeProjectConfig {
    # Writes the project-local OpenCode configuration that registers the
    # Riltra MCP server for the isolated pilot home. The session runs in the
    # project directory, so OpenCode discovers <project>/opencode.json
    # without relying on the global configuration: the session does not depend
    # on it. The global configuration is never written. What the AGENT does with
    # a global configuration is OpenCode behaviour, outside the harness control.
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Bin,
        [Parameter(Mandatory = $true)][string]$HomeDir,
        [string]$Model = ''
    )

    $config = [ordered]@{
        '$schema' = 'https://opencode.ai/config.json'
        mcp       = [ordered]@{
            servers = [ordered]@{
                riltra = [ordered]@{
                    type        = 'local'
                    command     = @($Bin, 'mcp')
                    environment = [ordered]@{ RILTRA_HOME = $HomeDir }
                    codemode    = $false
                }
            }
        }
    }
    if ($Model) { $config['model'] = $Model }
    [System.IO.File]::WriteAllText($Path, ($config | ConvertTo-Json -Depth 8), (New-Object System.Text.UTF8Encoding($false)))
    return $Path
}

function Get-SessionExitCode {
    # Fail closed. A session that exited cleanly but never called a Riltra
    # tool did not answer the question: it found no server (it may have
    # registered one of its own, against another repository). Such a session
    # is recorded as blocked, never as a pass.
    param(
        [Parameter(Mandatory = $true)][int]$ExitCode,
        [Parameter(Mandatory = $true)][AllowEmptyCollection()][object[]]$ToolCalls
    )

    if ($ExitCode -ne 0) { return $ExitCode }
    $riltra = @($ToolCalls | Where-Object { [string]$_.tool -like 'riltra_*' })
    if ($riltra.Count -eq 0) { return 3 }
    return 0
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
