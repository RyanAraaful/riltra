# CP2 harness step: real OpenCode session against the Riltra MCP server.
# The agent may only reach source facts through the Riltra MCP tools.
#
# On the CP2 host this step was BLOCKED: the OpenCode free tier refuses a
# nested CLI session (provider error 403). The script records that outcome
# faithfully instead of claiming a successful agent session.
#
# Usage:
#   pwsh -File opencode-session.ps1 -BenchRoot <dir> -OpenCode <cmd> -Out <dir>
param(
  [string]$BenchRoot = '',
  [string]$Bin = '',
  [string]$Out = '',
  [string]$HomeDir = '',
  [string]$OpenCode = '',
  [string]$Model = 'opencode/deepseek-v4.1-flash-free',
  [int]$TimeoutSec = 600
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'common.ps1')
$cfg = Initialize-Cp2Harness -BenchRoot $BenchRoot -Bin $Bin -Out $Out
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

$before = Home-Digest $HomeDir
"===== home digest before (files=$($before.Split("`n").Count)) ====="

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
    [System.IO.File]::WriteAllText($cmdFile, '@echo off' + "`r`n" + '"' + $OpenCode + '" ' + $argLine + ' "' + ($prompt -replace '"', '""') + '"' + "`r`n")
    $proc = Start-Process -FilePath $cmdFile -WorkingDirectory $sessionDir -NoNewWindow -PassThru `
      -RedirectStandardOutput $stdoutFile -RedirectStandardError $stderrFile
  } else {
    $proc = Start-Process -FilePath $OpenCode `
      -ArgumentList @('run', '--format', 'json', '--auto', '--model', $Model, $prompt) `
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
$toolNames = [regex]::Matches($rawOut, 'riltra_[a-z_]+') | ForEach-Object { $_.Value } | Sort-Object -Unique
"  tools referenced: $($toolNames -join ', ')"
"===== stdout: assistant text (last 3000 chars) ====="
$texts = [regex]::Matches($rawOut, '"text"\s*:\s*"((?:[^"\\]|\\.)*)"') | ForEach-Object { $_.Groups[1].Value }
($texts -join "`n") | Select-Object -Last 1 | ForEach-Object { $_.Substring([math]::Max(0, $_.Length - 3000)) }

# A machine-readable summary for the grader: a blocked session is recorded
# as blocked, never as a successful agent session.
$summary = [ordered]@{
  exit_code = $exit
  reason = if ($exit -ne 0) { 'provider-auth-403' } else { $null }
  error = if ($exit -ne 0) { ($rawOut | Select-String -Pattern '"message":"([^"]+)"' | Select-Object -First 1).Matches.Groups[1].Value } else { $null }
  tools = @($toolNames)
  cited_sources = $false
}
[System.IO.File]::WriteAllText((Join-Path $out 'opencode-session.json'), ($summary | ConvertTo-Json -Depth 4), (New-Object System.Text.UTF8Encoding($false)))
"summary: $($summary | ConvertTo-Json -Compress)"

if ($exit -ne 0) {
  "BLOCKED: the OpenCode session failed with exit $exit (see the stdout and stderr above)"
  exit $exit
}
