# The agent-side OpenCode session of the 2026-10-21 re-evaluation.
#
# Why this exists: the harness's `opencode-session.ps1` passed a multi-line
# prompt through a generated `.cmd` file, and the prompt never reached the
# agent ("No question came through in your message"), and it expected the
# Riltra MCP server to be registered in the GLOBAL OpenCode configuration -
# it is not, so the agent registered a server of its own, against another
# repository. This runner delivers the same prompt as a single argument and
# writes a project-local OpenCode config that registers the pilot
# snapshot's MCP server.
#
# Both defects are fixed in the canonical harness since
# `docs/m9-cp2-harness-correction.md`; this script stays as the record of
# how the re-evaluation's agent session was actually run.
#
# Usage (every path is explicit):
#   pwsh -File opencode-session-d5.ps1 -Bin <riltra.exe> -HomeDir <pilot home> `
#       -SessionDir <dir> -Out <dir> [-Model <id>] [-TimeoutSec <n>]
param(
    [Parameter(Mandatory = $true)][string]$Bin,
    [Parameter(Mandatory = $true)][string]$HomeDir,
    [Parameter(Mandatory = $true)][string]$SessionDir,
    [Parameter(Mandatory = $true)][string]$Out,
    [string]$Model = 'opencode/minimax-m3.1-flash-free',
    [int]$TimeoutSec = 1200
)
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force -Path $SessionDir | Out-Null
New-Item -ItemType Directory -Force -Path $Out | Out-Null

# The project-local OpenCode config: the MCP server is the pilot's, and the
# environment points it at the pilot home, so the agent can only see the
# registered snapshot.
$config = [ordered]@{
    '$schema' = 'https://opencode.ai/config.json'
    model     = $Model
    mcp       = [ordered]@{ servers = [ordered]@{ riltra = [ordered]@{
                type        = 'local'
                command     = @($Bin, 'mcp')
                environment = [ordered]@{ RILTRA_HOME = $HomeDir }
                codemode    = $false
            } } }
}
Set-Content -Path (Join-Path $SessionDir 'opencode.json') -Value ($config | ConvertTo-Json -Depth 8) -Encoding UTF8

# The frozen harness's prompt, as one argument (its line breaks are kept as
# spaces so the text is unchanged).
$prompt = 'You are answering a code question about the MMSP repository through the Riltra MCP tools only. The filesystem is not readable, so every fact must come from a tool result. Task 1: In the MMSP repository, how is the max_triangulation_angle option wired from the command line into the source-image selection, and which files would I need to touch to change its default? Use the Riltra MCP tools (search, callers, callees, trace, impact) to find out. For every fact, give the repository-relative file path and line number exactly as the tool reported it. If a tool cannot resolve something, say so explicitly instead of guessing. Task 2: Call the same code-search tool once with a repository name that does not exist, and report exactly what the tool returned (the error text, quoted). Do not retry with another name. Then state, in one line each: which repository you queried, and whether any tool reported a freshness status.'

$stdoutFile = Join-Path $Out 'opencode-session.stdout.json'
$stderrFile = Join-Path $Out 'opencode-session.stderr.txt'
# The npm shim is a .cmd on Windows; a generated wrapper keeps the
# single-line prompt intact (the frozen harness's multi-line prompt does not
# survive a .cmd).
$shim = Join-Path $env:APPDATA 'npm\opencode.cmd'
$cmdFile = Join-Path $SessionDir '_opencode.cmd'
$cmdLine = '"' + $shim + '" run --format json --auto --model ' + $Model + ' "' + ($prompt -replace '"', '""') + '"'
[System.IO.File]::WriteAllText($cmdFile, '@echo off' + "`r`n" + $cmdLine + "`r`n")
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$p = Start-Process -FilePath $cmdFile -WorkingDirectory $SessionDir -NoNewWindow -PassThru `
    -RedirectStandardOutput $stdoutFile -RedirectStandardError $stderrFile
if (-not $p.WaitForExit($TimeoutSec * 1000)) {
    $p.Kill()
    throw "the OpenCode session did not finish within $TimeoutSec s"
}
$sw.Stop()
$stdout = Get-Content $stdoutFile -Raw
$stderr = Get-Content $stderrFile -Raw

$toolNames = @([regex]::Matches($stdout, 'riltra_[a-z_]+') | ForEach-Object { $_.Value } | Sort-Object -Unique)
# A citation is a repository-relative path with a line number.
$cited = $stdout -match 'colmap/src/colmap/[\w/.]+\.\w+:\d+'
$summary = [ordered]@{
    exit_code     = $p.ExitCode
    reason        = if ($p.ExitCode -ne 0) { 'session-failed' } else { $null }
    error         = if ($p.ExitCode -ne 0) { ($stdout | Select-String -Pattern '"message":"([^"]+)"' | Select-Object -First 1).Matches.Groups[1].Value } else { $null }
    tools         = $toolNames
    cited_sources = $cited
    wall_s        = [math]::Round($sw.Elapsed.TotalSeconds, 1)
    stdout_bytes  = $stdout.Length
    stderr_bytes  = $stderr.Length
}
[System.IO.File]::WriteAllText((Join-Path $Out 'opencode-session.json'), ($summary | ConvertTo-Json -Depth 4), (New-Object System.Text.UTF8Encoding($false)))
Write-Host ("exit={0} wall={1}s stdout={2}B stderr={3}B tools={4} cited_sources={5}" -f $p.ExitCode, $summary.wall_s, $summary.stdout_bytes, $summary.stderr_bytes, $toolNames.Count, $cited)
$texts = @([regex]::Matches($stdout, '"text"\s*:\s*"((?:[^"\\]|\\.)*)"') | ForEach-Object { $_.Groups[1].Value })
$answer = $texts | Select-Object -Last 1
if ($answer) { Write-Host '--- the agent answer (last 4000 chars) ---'; $answer.Substring([math]::Max(0, $answer.Length - 4000)) }


