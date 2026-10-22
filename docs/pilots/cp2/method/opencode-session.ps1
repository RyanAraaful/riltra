# CP2 harness step: real OpenCode session against the Riltra MCP server.
# The agent may only reach source facts through the Riltra MCP tools.
#
# The session is self-contained: the MCP server is registered in a
# PROJECT-LOCAL OpenCode configuration (<sessionDir>/opencode.json) that
# points at the isolated pilot home. The global OpenCode configuration is
# never written and never relied upon: the session works whether or not one
# exists. What the AGENT does with a global configuration is OpenCode
# behaviour, outside the harness control.
#
# The prompt is passed as a single-line argument: a command wrapper runs one
# command per line, so a prompt with line breaks was truncated at the first
# break and the agent received no question.
#
# Fail closed: a session that exited cleanly but never CALLED a Riltra tool
# tools is recorded as blocked, never as a passing session.
#
# Usage:
#   pwsh -File opencode-session.ps1 -BenchRoot <dir> -Bin <riltra.exe> -Out <dir> -HomeDir <dir> -MmspRoot <dir>
param(
  [string]$BenchRoot = '',
  [string]$Bin = '',
  [string]$Out = '',
  [string]$HomeDir = '',
  [string]$OpenCode = '',
  [string]$Model = 'opencode/deepseek-v4.1-flash-free',
  [Parameter(Mandatory = $true)][string]$MmspRoot,
  [int]$TimeoutSec = 600
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'common.ps1')
$cfg = Initialize-Cp2Harness -BenchRoot $BenchRoot -Bin $Bin -Out $Out -MmspRoot $MmspRoot
if (-not $HomeDir) { $HomeDir = Join-Path $cfg.BenchRoot 'home-main' }
if (-not $OpenCode) {
  # The npm shim is a .cmd on Windows; Start-Process cannot run it directly.
  $shim = Join-Path $env:APPDATA 'npm\opencode.cmd'
  if (Test-Path -LiteralPath $shim) { $OpenCode = $shim }
  else { $OpenCode = (Get-Command opencode -ErrorAction SilentlyContinue).Source }
}
$isBatch = $OpenCode -and ($OpenCode -match '\.(cmd|bat)$')
$sessionDir = New-BenchDir -Path (Join-Path $cfg.BenchRoot 'cp2\opencode-session') -Root $cfg.BenchRoot
$out = New-BenchDir -Path (Join-Path $cfg.Out 'mcp') -Root $cfg.BenchRoot
# A summary of an earlier run must never be graded as this run's evidence:
# every failure path (no CLI, a timeout, a throw) must leave no summary, so
# the grader sees the evidence as missing.
$summaryPath = Join-Path $out 'opencode-session.json'
Remove-Item -LiteralPath $summaryPath -Force -ErrorAction SilentlyContinue

function Home-Digest([string]$dir) {
  $items = @()
  if (Test-Path $dir) {
    Get-ChildItem $dir -Recurse -File | Sort-Object FullName | ForEach-Object {
      $items += ("{0}|{1}|{2}" -f $_.FullName.Replace($dir, ''), $_.Length, (Get-FileHash $_.FullName -Algorithm SHA256).Hash)
    }
  }
  return ($items -join "`n")
}

$prompt = @'
You are answering a code question about the MMSP repository through the Riltra MCP tools only. The filesystem is not readable, so every fact must come from a tool result.

Task 1: In the MMSP repository, how is the max_triangulation_angle option wired from the command line into the source-image selection, and which files would I need to touch to change its default? Use the Riltra MCP tools (search, callers, callees, trace, impact) to find out. For every fact, give the repository-relative file path and line number exactly as the tool reported it. If a tool cannot resolve something, say so explicitly instead of guessing.

Task 2: Call the same code-search tool once with a repository name that does not exist, and report exactly what the tool returned (the error text, quoted). Do not retry with another name.

Then state, in one line each: which repository you queried, and whether any tool reported a freshness status.
'@
# One argument, one line: a command wrapper runs one command per line, so a
# prompt with line breaks was truncated at the first break.
$promptArg = ConvertTo-SingleLinePrompt -Prompt $prompt

$before = Home-Digest $HomeDir
"===== home digest before (files=$($before.Split("`n").Count)) ====="

# The session must not be able to reach the original repository.
Assert-IsolatedHome -HomeDir $HomeDir -MmspRoot $cfg.MmspRoot | Out-Null

# The project-local configuration: the Riltra MCP server, pointed at the
# isolated pilot home. The global OpenCode configuration is untouched.
$configPath = New-OpenCodeProjectConfig -Path (Join-Path $sessionDir 'opencode.json') `
  -Bin $cfg.Bin -HomeDir $HomeDir -Model $Model
"===== project-local config ====="
"  $configPath"

if (-not $OpenCode) {
  "opencode was not found on PATH: the agent-side session cannot run here."
  "BLOCKED: no OpenCode CLI available"
  exit 3
}

$stdoutFile = Join-Path $out 'opencode-session.stdout.json'
$stderrFile = Join-Path $out 'opencode-session.stderr.txt'
$sw = [System.Diagnostics.Stopwatch]::StartNew()
Push-Location $sessionDir
try {
  if ($isBatch) {
    $cmdFile = Join-Path $sessionDir '_opencode.cmd'
    $argLine = (@('run', '--format', 'json', '--auto', '--model', $Model) | ForEach-Object { '"' + $_ + '"' }) -join ' '
    [System.IO.File]::WriteAllText($cmdFile, '@echo off' + "`r`n" + '"' + $OpenCode + '" ' + $argLine + ' "' + ($promptArg -replace '"', '""') + '"' + "`r`n")
    $proc = Start-Process -FilePath $cmdFile -WorkingDirectory $sessionDir -NoNewWindow -PassThru `
      -RedirectStandardOutput $stdoutFile -RedirectStandardError $stderrFile
  } else {
    # ONE quoted argument string: Start-Process joins an argument array with
    # spaces and does not quote it, which would split the prompt into many
    # arguments.
    $argLine = (@('run', '--format', 'json', '--auto', '--model', $Model) | ForEach-Object { '"' + $_ + '"' }) -join ' '
    $proc = Start-Process -FilePath $OpenCode -ArgumentList ($argLine + ' "' + ($promptArg -replace '"', '\"') + '"') `
      -WorkingDirectory $sessionDir -NoNewWindow -PassThru `
      -RedirectStandardOutput $stdoutFile -RedirectStandardError $stderrFile
  }
  if (-not $proc.WaitForExit($TimeoutSec * 1000)) {
    $proc.Kill()
    throw "the OpenCode session did not finish within $TimeoutSec s"
  }
  $exit = $proc.ExitCode
} finally { Pop-Location }
$sw.Stop()

"opencode exit=$exit wall_s=$([math]::Round($sw.Elapsed.TotalSeconds,1))"
"stdout bytes = $((Get-Item $stdoutFile).Length)   stderr bytes = $((Get-Item $stderrFile).Length)"

$after = Home-Digest $HomeDir
"home unchanged after session = $($before -eq $after)"
[System.IO.File]::WriteAllText((Join-Path $out 'home-digest-before.txt'), $before)
[System.IO.File]::WriteAllText((Join-Path $out 'home-digest-after.txt'), $after)

"===== stderr (first 20 lines) ====="
Get-Content $stderrFile -ErrorAction SilentlyContinue | Select-Object -First 20
"===== stdout: tool calls used ====="
$rawOut = Get-Content $stdoutFile -Raw
# The tool calls are read from the stream's structured events, never scraped
# from the text: a tool name in a sentence is not a call.
$toolCalls = @(Get-SessionToolCalls -StdoutFile $stdoutFile)
$toolNames = @($toolCalls | ForEach-Object { $_.tool } | Sort-Object -Unique)
"  tools called: $($toolNames -join ', ')"
"  tool calls: $($toolCalls.Count)"
"===== stdout: assistant text (last 3000 chars) ====="
# The answer is the agent's final message; the earlier text parts are
# intermediate notes.
$answerText = Get-SessionAnswer -StdoutFile $stdoutFile
$answerText | Select-Object -Last 1 | ForEach-Object { $_.Substring([math]::Max(0, $_.Length - 3000)) }

# Fail closed: a clean exit that never called a Riltra tool did not answer
# the question - the agent found no server (it may have registered one of its
# own, against another repository).
$effectiveExit = Get-SessionExitCode -ExitCode $exit -ToolCalls $toolCalls
if ($effectiveExit -ne $exit) {
  "BLOCKED: the session exited $exit but never called a Riltra MCP tool"
}

# Did the ANSWER cite repository-relative file:line locations? The value is
# measured from the answer, never assumed.
$citationPattern = 'colmap/[A-Za-z0-9_./-]+:[0-9]+'
$citedSources = $answerText -match $citationPattern
$citationsFound = ([regex]::Matches($answerText, $citationPattern)).Count

# A machine-readable summary for the grader: a blocked session is recorded
# as blocked, never as a successful agent session. The error is the LAST
# "message" in the stream: the first one can be an intermediate tool error
# (the semantic tool's provider refusal), not the session's own failure.
$errMatch = $rawOut | Select-String -Pattern '"message":"([^"]+)"' | Select-Object -Last 1
$summary = [ordered]@{
  exit_code       = $effectiveExit
  reason          = if ($effectiveExit -eq 0) { $null } elseif (@($toolCalls | Where-Object { [string]$_.tool -like 'riltra_*' }).Count -eq 0) { 'mcp-server-not-reached' } else { 'session-failed' }
  error           = if ($effectiveExit -ne 0 -and $errMatch) { $errMatch.Matches.Groups[1].Value } else { $null }
  tools           = @($toolNames)
  tool_calls      = @($toolCalls | ForEach-Object { "$($_.tool):$($_.status)" })
  cited_sources   = $citedSources
  citations_found = $citationsFound
  model           = $Model
  wall_s          = [math]::Round($sw.Elapsed.TotalSeconds, 1)
  stdout_bytes    = (Get-Item -LiteralPath $stdoutFile).Length
  stderr_bytes    = (Get-Item -LiteralPath $stderrFile).Length
}
[System.IO.File]::WriteAllText($summaryPath, ($summary | ConvertTo-Json -Depth 4), (New-Object System.Text.UTF8Encoding($false)))
"summary: $($summary | ConvertTo-Json -Compress)"

if ($effectiveExit -ne 0) {
  "BLOCKED: the OpenCode session failed with exit $effectiveExit (see the stdout and stderr above)"
  exit $effectiveExit
}
