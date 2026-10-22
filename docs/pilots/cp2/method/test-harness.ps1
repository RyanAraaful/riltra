# CP2 harness: regression tests for the shared harness helpers.
#
# These tests pin the behaviour the CP2 procedure depends on:
#   - a bench directory copied into another bench directory holds the
#     source's CONTENTS at its own root (a plain Copy-Item of a directory
#     into an existing directory nests the source folder instead, which
#     buried the registry one level too deep and made every query answer
#     "unknown repository");
#   - the OpenCode session's prompt keeps its instruction content when it
#     is passed through a command wrapper;
#   - the session's MCP configuration is project-local, registers exactly
#     one server and points at the isolated pilot home;
#   - a home that registers the original repository is refused;
#   - a session that never reached the Riltra tools does not pass.
#
# Usage: pwsh -NoProfile -File test-harness.ps1
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

# The message of the refusal, so a check can assert WHY it was refused and
# not only that something threw.
function Refusal([scriptblock]$action) {
    try { & $action | Out-Null; return '' } catch { return [string]$_.Exception.Message }
}

# The digest of the user's global OpenCode configuration. The harness must
# never depend on it and never change it; the test only reads it.
function Global-ConfigDigest {
    $p = Join-Path $env:USERPROFILE '.config\opencode\opencode.json'
    if (-not (Test-Path -LiteralPath $p)) { return '(absent)' }
    return (Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash
}

# A failing case reports its own error instead of aborting the script.
function Test-That([string]$name, [scriptblock]$body) {
    try { & $body }
    catch { "  FAIL  $name  ($($_.Exception.Message))"; $script:failures += $name }
}

# A private bench root for the tests.
$tmp = Join-Path $env:TEMP ('cp2-harness-' + [Guid]::NewGuid().ToString('N').Substring(0, 8))
$benchRoot = Join-Path $tmp 'bench'
$cold = Join-Path $benchRoot 'home-cold1'
New-Item -ItemType Directory -Force -Path (Join-Path $cold 'index') | Out-Null
Set-Content -Path (Join-Path $cold 'registry.json') -Value '{"version":1,"repos":[]}' -Encoding UTF8
Set-Content -Path (Join-Path $cold 'index\mmsp-pin.db') -Value 'db-bytes' -Encoding UTF8

try {
    "===== the home copy ====="
    Test-That "the copied home holds the cold home's contents at its own root" {
        $main = Reset-BenchDir (Join-Path $benchRoot 'home-main') -Root $benchRoot
        Copy-BenchDir -Source $cold -Destination $main -Root $benchRoot | Out-Null
        Check "the registry is at the root of the copied home" (Test-Path (Join-Path $main 'registry.json')) "registry.json is missing"
        Check "the index is at the root of the copied home" (Test-Path (Join-Path $main 'index\mmsp-pin.db')) "index/mmsp-pin.db is missing"
        Check "the source folder is not nested in the copied home" (-not (Test-Path (Join-Path $main 'home-cold1'))) "home-cold1 is nested"
    }

    Test-That "the historical copy operation nests the source folder" {
        # The defect's mechanism, pinned: Copy-Item of a DIRECTORY into an
        # existing directory copies the source folder INTO it. The harness
        # resets the destination first, so the destination always exists.
        $main = Reset-BenchDir (Join-Path $benchRoot 'home-main-historical') -Root $benchRoot
        Copy-Item $cold $main -Recurse -Force
        Check "the historical operation nests the source folder" (Test-Path (Join-Path $main 'home-cold1\registry.json')) "the historical operation did not nest"
        Check "the historical operation leaves no registry at the root" (-not (Test-Path (Join-Path $main 'registry.json'))) "the historical operation put a registry at the root"
    }

    "===== the session prompt ====="
    Test-That "the prompt keeps its instruction content when it is flattened" {
        $prompt = @'
You are answering a code question about the MMSP repository through the Riltra MCP tools only.

Task 1: how is the max_triangulation_angle option wired? Give the repository-relative file path and line number exactly as the tool reported it.

Task 2: report the error text, quoted. Do not retry with another name.
'@
        $flat = ConvertTo-SingleLinePrompt -Prompt $prompt
        Check "the flattened prompt has no line break" (-not ($flat -match '[\r\n]')) "the flattened prompt still has a line break"
        Check "the flattened prompt keeps the first instruction" ($flat -like '*answering a code question about the MMSP repository*') "the first instruction is missing"
        Check "the flattened prompt keeps task 1" ($flat -like '*max_triangulation_angle option wired*') "task 1 is missing"
        Check "the flattened prompt keeps task 2" ($flat -like '*Do not retry with another name*') "task 2 is missing"
    }

    "===== the session configuration ====="
    Test-That "the project-local config registers one server and points at the isolated home" {
        $configDir = Join-Path $benchRoot 'session'
        New-Item -ItemType Directory -Force -Path $configDir | Out-Null
        $configPath = Join-Path $configDir 'opencode.json'
        $bin = Join-Path $benchRoot 'riltra.exe'
        $pilotHome = Join-Path $benchRoot 'home-cold1'
        $globalBefore = Global-ConfigDigest
        New-OpenCodeProjectConfig -Path $configPath -Bin $bin -HomeDir $pilotHome -Model 'opencode/some-model' | Out-Null
        Check "the project-local config exists" (Test-Path -LiteralPath $configPath) "the config was not written"
        $config = Get-Content -LiteralPath $configPath -Raw | ConvertFrom-Json
        $servers = @($config.mcp.servers.PSObject.Properties)
        Check "the config registers exactly one MCP server" ($servers.Count -eq 1) "$($servers.Count) servers"
        Check "the server is named riltra" ($servers[0].Name -eq 'riltra') "$($servers[0].Name)"
        $server = $servers[0].Value
        Check "the server is a local process" ($server.type -eq 'local') "$($server.type)"
        Check "the server runs the riltra MCP" (@($server.command).Count -eq 2 -and $server.command[1] -eq 'mcp') "$($server.command -join ' ')"
        Check "the server uses the given binary" ($server.command[0] -eq $bin) "$($server.command[0])"
        Check "the server points at the isolated home" ($server.environment.RILTRA_HOME -eq $pilotHome) "$($server.environment.RILTRA_HOME)"
        Check "the global OpenCode configuration is untouched" ((Global-ConfigDigest) -eq $globalBefore) "the global configuration changed"
    }

    "===== the session isolation ====="
    Test-That "a home that reaches the original repository is refused" {
        $orig = Join-Path $tmp 'original-mmsp'
        New-Item -ItemType Directory -Force -Path (Join-Path $orig 'colmap') | Out-Null
        New-Item -ItemType Directory -Force -Path $orig | Out-Null

        function New-Home([string]$name, [string]$registeredRoot) {
            $dir = Join-Path $benchRoot $name
            New-Item -ItemType Directory -Force -Path $dir | Out-Null
            $registry = [ordered]@{
                version = 1
                repos   = @([ordered]@{ name = 'repo'; root = $registeredRoot })
            } | ConvertTo-Json -Compress
            Set-Content -Path (Join-Path $dir 'registry.json') -Value $registry -Encoding UTF8
            return $dir
        }

        $exact = New-Home 'home-exact' ('\\?\' + $orig + '\')
        $exactRefusal = Refusal { Assert-IsolatedHome -HomeDir $exact -MmspRoot $orig | Out-Null }
        Check "a home registering the original repository is refused" ($exactRefusal -like '*original repository*') "the refusal was: '$exactRefusal'"

        $sub = New-Home 'home-sub' ('\\?\' + (Join-Path $orig 'colmap') + '\')
        $subRefusal = Refusal { Assert-IsolatedHome -HomeDir $sub -MmspRoot $orig | Out-Null }
        Check "a home registering a subdirectory of the original is refused" ($subRefusal -like '*original repository*') "the refusal was: '$subRefusal'"

        $parent = New-Home 'home-parent' ('\\?\' + $tmp + '\')
        $parentRefusal = Refusal { Assert-IsolatedHome -HomeDir $parent -MmspRoot $orig | Out-Null }
        Check "a home registering a parent of the original is refused" ($parentRefusal -like '*original repository*') "the refusal was: '$parentRefusal'"

        $pilot = New-Home 'home-pilot' ('\\?\' + (Join-Path $benchRoot 'pilot-snapshot') + '\')
        Check "a home registering the pilot is accepted" (-not (Throws { Assert-IsolatedHome -HomeDir $pilot -MmspRoot $orig | Out-Null })) "it was refused"

        $origRefusal = Refusal { Assert-IsolatedHome -HomeDir $orig -MmspRoot $orig | Out-Null }
        Check "the original repository itself is refused" ($origRefusal -like '*original repository*') "the refusal was: '$origRefusal'"

        $inside = Join-Path $orig 'inside'
        New-Item -ItemType Directory -Force -Path $inside | Out-Null
        $insideRefusal = Refusal { Assert-IsolatedHome -HomeDir $inside -MmspRoot $orig | Out-Null }
        Check "a home inside the original repository is refused" ($insideRefusal -like '*original repository*') "the refusal was: '$insideRefusal'"

        $noOriginal = Refusal { Assert-IsolatedHome -HomeDir $pilot -MmspRoot '' | Out-Null }
        Check "the guard cannot be switched off by omitting the original repository" ($noOriginal -like '*original repository*' -or $noOriginal -like '*MmspRoot*') "the refusal was: '$noOriginal'"
    }

    "===== the copy guards ====="
    Test-That "the copy refuses a source and destination that are not disjoint" {
        $same = Refusal { Copy-BenchDir -Source $cold -Destination $cold -Root $benchRoot | Out-Null }
        Check "copying a directory into itself is refused" ($same -like '*disjoint*') "the refusal was: '$same'"
        $child = Join-Path $cold 'child'
        New-Item -ItemType Directory -Force -Path $child | Out-Null
        $nested = Refusal { Copy-BenchDir -Source $cold -Destination $child -Root $benchRoot | Out-Null }
        Check "copying a directory into its own subdirectory is refused" ($nested -like '*disjoint*') "the refusal was: '$nested'"
    }

    Test-That "the copy refuses a destination that still holds content" {
        $stale = Reset-BenchDir (Join-Path $benchRoot 'home-stale') -Root $benchRoot
        Set-Content -LiteralPath (Join-Path $stale 'leftover.json') -Value '{}' -Encoding UTF8
        $merge = Refusal { Copy-BenchDir -Source $cold -Destination $stale -Root $benchRoot | Out-Null }
        Check "a destination with leftover content is refused" ($merge -like '*empty*') "the refusal was: '$merge'"
    }

    "===== the session outcome ====="
    Test-That "a session that never reached the Riltra tools does not pass" {
        # The tool name in the TEXT is not a tool call. The stream's
        # structured tool_use events are the only evidence that the MCP
        # server was reached.
        $textOnly = Join-Path $benchRoot 'session\text-only.json'
        Set-Content -LiteralPath $textOnly -Encoding UTF8 -Value @(
            '{"type":"text","part":{"type":"text","text":"I could not call riltra_code_search"}}'
        )
        Check "a session whose text names a Riltra tool fails closed" ((Get-SessionExitCode -ExitCode 0 -ToolCalls @(Get-SessionToolCalls -StdoutFile $textOnly)) -ne 0) "it passed"
        Check "that session made no tool call" (@(Get-SessionToolCalls -StdoutFile $textOnly).Count -eq 0) "a tool call was recorded"

        $called = Join-Path $benchRoot 'session\tool-called.json'
        Set-Content -LiteralPath $called -Encoding UTF8 -Value @(
            '{"type":"tool_use","part":{"type":"tool","tool":"riltra_riltra_code_search","state":{"status":"completed"}}}'
        )
        Check "a session that called a Riltra tool passes" ((Get-SessionExitCode -ExitCode 0 -ToolCalls @(Get-SessionToolCalls -StdoutFile $called)) -eq 0) "it failed"
        Check "the tool call is recorded with its status" (@(Get-SessionToolCalls -StdoutFile $called)[0].status -eq 'completed') "the status was not recorded"
        Check "a failed session stays failed" ((Get-SessionExitCode -ExitCode 3 -ToolCalls @(Get-SessionToolCalls -StdoutFile $called)) -eq 3) "it passed"
    }

    "===== the session answer ====="
    Test-That "the answer is the agent's final message" {
        # An intermediate note may mention a path; only the final message is
        # the answer.
        $twoParts = Join-Path $benchRoot 'session\two-parts.json'
        Set-Content -LiteralPath $twoParts -Encoding UTF8 -Value @(
            '{"type":"text","part":{"type":"text","text":"an intermediate note mentioning colmap/src/colmap/mvs/patch_match.cc:118"}}'
            '{"type":"text","part":{"type":"text","text":"the final answer, without a citation"}}'
        )
        $answer = Get-SessionAnswer -StdoutFile $twoParts
        Check "the answer is the last text part" ($answer -eq 'the final answer, without a citation') "the answer was: '$answer'"
        Check "a citation in an intermediate note is not a citation of the answer" (-not ($answer -match 'colmap/[A-Za-z0-9_./-]+:[0-9]+')) "the answer matched a citation"
    }

    "===== the call sites ====="
    Test-That "the procedure uses the corrected helpers" {
        $runAll = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'run-all.ps1') -Raw
        Check "run-all.ps1 copies the home with Copy-BenchDir" ($runAll -match 'Copy-BenchDir\s+-Source') "the copy step does not use Copy-BenchDir"
        Check "run-all.ps1 no longer copies a directory into an existing one" (-not ($runAll -match 'Copy-Item\s+\(Join-Path \$bench ''home-cold1''\)')) "the historical copy is still there"
        $session = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'opencode-session.ps1') -Raw
        Check "opencode-session.ps1 writes the project-local configuration" ($session -match 'New-OpenCodeProjectConfig') "the configuration writer is not called"
        Check "opencode-session.ps1 asserts the isolated home" ($session -match 'Assert-IsolatedHome') "the isolation guard is not called"
        Check "opencode-session.ps1 computes the outcome from the tool calls" ($session -match 'Get-SessionExitCode') "the fail-closed outcome is not computed"
        Check "opencode-session.ps1 reads the answer from the event stream" ($session -match 'Get-SessionAnswer') "the answer is not read from the stream"
    }
}
finally {
    # The only delete in this script, guarded the same way the harness
    # guards its own: the path must be inside the temp directory.
    if ((Get-BenchPathState -Path $tmp -Root $env:TEMP) -eq 'inside') {
        Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
    }
}

if ($failures.Count -gt 0) {
    "===== result ====="
    "FAILED: $($failures.Count) check(s): $($failures -join '; ')"
    exit 1
}
"===== result ====="
"ALL HARNESS CHECKS PASSED"
