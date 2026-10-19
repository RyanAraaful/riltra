# SPDX-License-Identifier: Apache-2.0
# CP7-A G6 OpenCode subtest: structured MCP domain error must be surfaced to the
# model without killing the server. This is a separate smoke session; it is not a
# corpus task and is excluded from the paired token totals.
param(
  [string]$Root = "",
  [string]$Out = "",
  [int]$TimeoutSec = 180
)
$ErrorActionPreference = "Stop"
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..\..")).Path
if (-not $Root) { $Root = Join-Path $RepoRoot ".cp7a" }
if (-not $Out) { $Out = Join-Path $Root "g6-smoke" }
$ArmDir = Join-Path ([System.IO.Path]::GetTempPath()) "opencode\cp7a\arm-b"
$PluginLog = Join-Path $Root "plugin-events.jsonl"
$HomeLive = Join-Path $Root "home-template"
$HomePristine = Join-Path $Root "home-pristine"
$MainFiles = @("registry.json", "index\riltra.db", "index\other.db", "knowledge\riltra.db", "knowledge\other.db")
$Expected = @("read", "riltra_riltra_code_callees", "riltra_riltra_code_callers", "riltra_riltra_code_impact", "riltra_riltra_code_search", "riltra_riltra_code_trace", "riltra_riltra_health", "riltra_riltra_knowledge_related", "riltra_riltra_knowledge_search", "riltra_riltra_knowledge_show", "riltra_riltra_repo_list", "riltra_riltra_repo_status")
$Model = @{ providerID = "opencode"; id = "deepseek-v4.1-flash-free" }
$Agent = "cp7a-bench"
$Prompt = @"
You are testing the Riltra MCP server's structured domain-error handling. Use only the riltra_riltra_* tools; do not use read or shell.

1. Call riltra_riltra_code_callers exactly once with repo="does-not-exist", path="crates/cli/src/knowledge.rs", line=89, col=1. Do this even though you expect an error.
2. Then call riltra_riltra_health with no arguments.
3. Report the exact error returned by the first call and whether the health tool still responds. Do not retry the first call with another repository.
"@

function New-Dir([string]$Path) { New-Item -ItemType Directory -Force -Path $Path | Out-Null }
function Write-Json([string]$Path, $Value) { $Value | ConvertTo-Json -Depth 40 | Set-Content -LiteralPath $Path -Encoding utf8NoBOM }
function Invoke-Api([string]$Dir, [string[]]$ApiArgs) {
  Push-Location -LiteralPath $Dir
  try { $out = & opencode @ApiArgs 2>&1; $code = $LASTEXITCODE; $text = ($out | ForEach-Object { "$_" }) -join "`n" }
  catch { $code = 1; $text = "$_" } finally { Pop-Location }
  [pscustomobject]@{ code = $code; text = $text }
}
function Get-MainHashes {
  $main = [ordered]@{}
  foreach ($rel in $MainFiles) {
    $p = Join-Path $HomeLive $rel
    $main[$rel] = $(if (Test-Path -LiteralPath $p) { (Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash } else { $null })
  }
  [pscustomobject]@{
    main = $main
    empty_index = (Test-Path -LiteralPath (Join-Path $HomeLive "index\empty.db"))
    empty_knowledge = (Test-Path -LiteralPath (Join-Path $HomeLive "knowledge\empty.db"))
  }
}
function Compare-Main($Before, $After) {
  $ok = $true
  foreach ($rel in $MainFiles) { if ($Before.main[$rel] -ne $After.main[$rel]) { $ok = $false } }
  if ($After.empty_index -or $After.empty_knowledge) { $ok = $false }
  $ok
}
function Reset-Home {
  foreach ($d in @("index", "knowledge")) {
    $p = Join-Path $HomeLive $d
    if (Test-Path -LiteralPath $p) { Remove-Item -LiteralPath $p -Recurse -Force }
    New-Dir $p
  }
  foreach ($rel in $MainFiles) { Copy-Item -LiteralPath (Join-Path $HomePristine $rel) -Destination (Join-Path $HomeLive $rel) -Force }
}
function Get-PluginEvidence([string]$Sid) {
  $lines = @()
  if (Test-Path -LiteralPath $PluginLog) { $lines = @(Get-Content -LiteralPath $PluginLog -ErrorAction SilentlyContinue | Where-Object { $_ -like "*$Sid*" }) }
  $events = @()
  foreach ($line in $lines) { try { $events += ($line | ConvertFrom-Json) } catch {} }
  $ctx = $events | Where-Object kind -eq "context" | Select-Object -First 1
  $kept = @(); $removed = @()
  if ($ctx) { if ($ctx.kept) { $kept = @($ctx.kept) }; if ($ctx.removed) { $removed = @($ctx.removed) } }
  [pscustomobject]@{ lines = $lines; events = $events; kept = $kept; removed = $removed; blocked = @($events | Where-Object kind -eq "blocked").Count; allowed = @($events | Where-Object kind -eq "allowed").Count }
}
function Test-SetEqual($A, $B) {
  $sa = @($A | Sort-Object -Unique); $sb = @($B | Sort-Object -Unique)
  return (($sa -join ",") -eq ($sb -join ","))
}

New-Dir $Out
Reset-Home
$before = Get-MainHashes
Write-Json (Join-Path $Out "home-before.json") $before
$startUtc = (Get-Date).ToUniversalTime().ToString("o")
$sid = $null
$session = $null
$sessionRaw = $null
$wait = $null
$errorText = $null
$createMs = $null
$postMs = $null
$promptCompletedMs = $null
try {
  $body = @{ agent = $Agent; model = $Model; location = @{ directory = ($ArmDir -replace "\\", "/") }; title = "CP7A G6 structured-error smoke" } | ConvertTo-Json -Compress -Depth 10
  $sw = [System.Diagnostics.Stopwatch]::StartNew(); $cs = [System.Diagnostics.Stopwatch]::StartNew()
  $r = Invoke-Api $ArmDir @("api", "session.create", "-d", $body)
  $cs.Stop(); $createMs = [math]::Round($cs.Elapsed.TotalMilliseconds, 1)
  if ($r.code -ne 0) { throw "session.create failed: $($r.text)" }
  $created = $r.text | ConvertFrom-Json
  $sid = $created.data.id
  if (-not $sid) { throw "session.create returned no id: $($r.text)" }
  $pbody = @{ text = $Prompt } | ConvertTo-Json -Compress -Depth 5
  $ps = [System.Diagnostics.Stopwatch]::StartNew()
  $p = Invoke-Api $ArmDir @("api", "post", "/api/session/$sid/prompt", "-d", $pbody)
  $ps.Stop(); $postMs = [math]::Round($ps.Elapsed.TotalMilliseconds, 1)
  if ($p.code -ne 0) { throw "prompt post failed: $($p.text)" }
  $promptCompletedMs = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
  $loop = [System.Diagnostics.Stopwatch]::StartNew()
  $polls = 0; $pollErrors = 0; $idle = $null; $last = $null; $lastRaw = $null
  while ($loop.Elapsed.TotalSeconds -lt $TimeoutSec) {
    Start-Sleep -Milliseconds 1200
    $polls++
    $g = Invoke-Api $ArmDir @("api", "get", "/api/session/$sid/message", "--param", "limit=1000")
    if ($g.code -ne 0) { $pollErrors++; continue }
    try {
      $last = $g.text | ConvertFrom-Json; $lastRaw = $g.text
      $idle = @($last.data | Where-Object type -eq "idle") | Select-Object -Last 1
      if ($idle) { break }
    } catch { $pollErrors++ }
  }
  $wait = [pscustomobject]@{ msgs = $last; raw = $lastRaw; idle = $idle; timed_out = (-not $idle); polls = $polls; errors = $pollErrors; elapsed_ms = [math]::Round($loop.Elapsed.TotalMilliseconds, 1) }
  $sr = Invoke-Api $ArmDir @("api", "get", "/api/session/$sid")
  if ($sr.code -eq 0) { $sessionRaw = $sr.text; $session = $sr.text | ConvertFrom-Json }
  if ($wait.raw) { Set-Content -LiteralPath (Join-Path $Out "messages.json") -Value $wait.raw -Encoding utf8NoBOM -NoNewline } else { Set-Content -LiteralPath (Join-Path $Out "messages.json") -Value '{"data":[]}' -Encoding utf8NoBOM -NoNewline }
  if ($sessionRaw) { Set-Content -LiteralPath (Join-Path $Out "session.json") -Value $sessionRaw -Encoding utf8NoBOM -NoNewline }
  Set-Content -LiteralPath (Join-Path $Out "prompt.txt") -Value $Prompt -Encoding utf8NoBOM -NoNewline
  Start-Sleep -Milliseconds 300
  $plugin = Get-PluginEvidence $sid
  if ($plugin.lines.Count -gt 0) { Set-Content -LiteralPath (Join-Path $Out "plugin.jsonl") -Value $plugin.lines -Encoding utf8NoBOM }
  $after = Get-MainHashes
  Write-Json (Join-Path $Out "home-after.json") $after
  $toolItems = @()
  foreach ($m in @($wait.msgs.data)) {
    if ($m.type -ne "assistant") { continue }
    foreach ($c in @($m.content)) {
      if ($c.type -ne "tool") { continue }
      $contentText = $null
      try { $contentText = ($c.state.content | ConvertTo-Json -Depth 20 -Compress) } catch { $contentText = "$($c.state.content)" }
      $toolItems += [pscustomobject]@{ name = $c.name; status = $c.state.status; input = $c.state.input; content = $contentText }
    }
  }
  $invalid = $toolItems | Where-Object name -eq "riltra_riltra_code_callers" | Select-Object -First 1
  $health = $toolItems | Where-Object name -eq "riltra_riltra_health" | Select-Object -First 1
  $checks = [pscustomobject]@{
    invalid_call_found = [bool]$invalid
    invalid_status = $(if ($invalid) { $invalid.status } else { $null })
    invalid_content = $(if ($invalid) { $invalid.content } else { $null })
    invalid_mentions_error = $(if ($invalid) { [bool]($invalid.content -match "error|not registered|unknown repo|repository") } else { $false })
    health_call_found = [bool]$health
    health_status = $(if ($health) { $health.status } else { $null })
    health_content = $(if ($health) { $health.content } else { $null })
    health_responded = $(if ($health) { [bool]($health.content -match "0\.1\.0|status|version|doctor") } else { $false })
    server_alive_after_error = [bool]($health -and $health.status -ne "error")
    tool_names = @($toolItems | ForEach-Object { $_.name })
    exposure_ok = (Test-SetEqual $plugin.kept $Expected)
    context_kept = @($plugin.kept)
    context_removed = @($plugin.removed)
    blocked = $plugin.blocked
    home_unchanged = (Compare-Main $before $after)
    empty_index = $after.empty_index
    empty_knowledge = $after.empty_knowledge
    status = $(if ($wait.timed_out) { "timeout" } elseif ($idle.outcome -eq "succeeded") { "ok" } else { "error" })
    outcome = $(if ($idle) { $idle.outcome } else { $null })
  }
  $meta = [ordered]@{
    task = "g6-structured-error-smoke"; arm = "B"; session = $sid; agent = $Agent; model = $Model; location = $ArmDir
    started_utc = $startUtc; finished_utc = (Get-Date).ToUniversalTime().ToString("o")
    timed_out = $wait.timed_out; polls = $wait.polls; poll_errors = $wait.errors
    session_create_ms = $createMs; prompt_post_ms = $postMs; turn_ms = $wait.elapsed_ms; first_assistant_ms = $null
    session_time = $session.data.time; session_tokens = $session.data.tokens; session_model = $session.data.model
    home_before = $before; home_after = $after; expected_tools = $Expected; context_kept = $plugin.kept; context_removed = $plugin.removed
    exposure_ok = $checks.exposure_ok; plugin_blocked = $plugin.blocked; plugin_allowed = $plugin.allowed; plugin_events = $plugin.events.Count
    checks = $checks; error = $null
  }
  Write-Json (Join-Path $Out "checks.json") $checks
  Write-Json (Join-Path $Out "meta.json") $meta
  Write-Output ($checks | ConvertTo-Json -Depth 20)
} catch {
  $errorText = "$_"
  $after = Get-MainHashes
  Write-Json (Join-Path $Out "home-after.json") $after
  if (-not (Test-Path -LiteralPath (Join-Path $Out "messages.json"))) { Set-Content -LiteralPath (Join-Path $Out "messages.json") -Value '{"data":[]}' -Encoding utf8NoBOM -NoNewline }
  $meta = [ordered]@{ task = "g6-structured-error-smoke"; arm = "B"; session = $sid; agent = $Agent; model = $Model; location = $ArmDir; started_utc = $startUtc; finished_utc = (Get-Date).ToUniversalTime().ToString("o"); timed_out = $false; session_create_ms = $createMs; prompt_post_ms = $postMs; turn_ms = $(if ($wait) { $wait.elapsed_ms } else { $null }); session_time = $null; session_tokens = $null; home_before = $before; home_after = $after; home_unchanged = (Compare-Main $before $after); exposure_ok = $false; context_kept = @(); context_removed = @(); plugin_blocked = 0; plugin_allowed = 0; plugin_events = 0; checks = $null; error = $errorText }
  Write-Json (Join-Path $Out "meta.json") $meta
  Write-Error $errorText
  exit 1
}
