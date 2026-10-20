# CP2 step 7b: real OpenCode session against the Riltra MCP server.
# The agent may only reach source facts through the Riltra MCP tools.
$ErrorActionPreference = 'Stop'
. "$env:TEMP\opencode\bench\cp2\common.ps1"

$sessionDir = "$env:TEMP\opencode\bench\cp2\opencode-session"
$out = "$env:TEMP\opencode\bench\cp2\raw\mcp"
New-Item -ItemType Directory -Force -Path $out | Out-Null
$homeDir = "$env:TEMP\opencode\bench\home-main"

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

$before = Home-Digest $homeDir
"===== home digest before (files=$($before.Split("`n").Count)) ====="

$sw = [System.Diagnostics.Stopwatch]::StartNew()
Push-Location $sessionDir
try {
  $stdoutFile = "$out\opencode-session.stdout.json"
  $stderrFile = "$out\opencode-session.stderr.txt"
  $proc = Start-Process -FilePath "<USERPROFILE>\AppData\Roaming\npm\opencode.cmd" `
    -ArgumentList @('run', '--format', 'json', '--auto', '--model', 'opencode/deepseek-v4.1-flash-free', $prompt) `
    -WorkingDirectory $sessionDir -NoNewWindow -PassThru `
    -RedirectStandardOutput $stdoutFile -RedirectStandardError $stderrFile
  $proc.WaitForExit()
  $exit = $proc.ExitCode
} finally { Pop-Location }
$sw.Stop()

"opencode exit=$exit wall_s=$([math]::Round($sw.Elapsed.TotalSeconds,1))"
"stdout bytes = $((Get-Item $stdoutFile).Length)   stderr bytes = $((Get-Item $stderrFile).Length)"

$after = Home-Digest $homeDir
"home unchanged after session = $($before -eq $after)"
[System.IO.File]::WriteAllText("$out\home-digest-before.txt", $before)
[System.IO.File]::WriteAllText("$out\home-digest-after.txt", $after)

"===== stderr (first 20 lines) ====="
Get-Content $stderrFile -ErrorAction SilentlyContinue | Select-Object -First 20
"===== stdout: tool calls used ====="
$raw = Get-Content $stdoutFile -Raw
$toolNames = [regex]::Matches($raw, 'riltra_[a-z_]+') | ForEach-Object { $_.Value } | Sort-Object -Unique
"  tools referenced: $($toolNames -join ', ')"
"===== stdout: assistant text (last 3000 chars) ====="
$texts = [regex]::Matches($raw, '"text"\s*:\s*"((?:[^"\\]|\\.)*)"') | ForEach-Object { $_.Groups[1].Value }
($texts -join "`n") | Select-Object -Last 1 | ForEach-Object { $_.Substring([math]::Max(0, $_.Length - 3000)) }
