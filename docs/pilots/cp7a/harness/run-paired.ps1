# SPDX-License-Identifier: Apache-2.0
# CP7-A paired OpenCode runner. Writes raw sessions, home hashes, plugin evidence,
# and timing metadata. The corpus and rubrics are frozen; this script never edits them.
param(
  [string]$Root = "E:\dev\Riltra\.cp7a",
  [string]$Runs = "E:\dev\Riltra\.cp7a\runs",
  [string[]]$Only = @(),
  [switch]$Resume,
  [switch]$WarmOnly,
  [switch]$SkipWarmup,
  [int]$TaskTimeoutSec = 240,
  [int]$WarmTimeoutSec = 180
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"
$CorpusPath = Join-Path $Root "corpus\corpus.json"
$PluginLog = Join-Path $Root "plugin-events.jsonl"
$HomeLive = Join-Path $Root "home-template"
$HomePristine = Join-Path $Root "home-pristine"
$ArmDir = @{
  A = "C:\Users\ryan\AppData\Local\Temp\opencode\cp7a\arm-a"
  B = "C:\Users\ryan\AppData\Local\Temp\opencode\cp7a\arm-b"
}
$McpTools = @(
  "riltra_riltra_code_callees", "riltra_riltra_code_callers", "riltra_riltra_code_impact",
  "riltra_riltra_code_search", "riltra_riltra_code_trace", "riltra_riltra_health",
  "riltra_riltra_knowledge_related", "riltra_riltra_knowledge_search",
  "riltra_riltra_knowledge_show", "riltra_riltra_repo_list", "riltra_riltra_repo_status"
)
$Expected = @{
  A = @("read", "shell")
  B = @("read") + $McpTools
}
$MainFiles = @("registry.json", "index\riltra.db", "index\other.db", "knowledge\riltra.db", "knowledge\other.db")
$Model = @{ providerID = "opencode"; id = "deepseek-v4.1-flash-free" }
$Agent = "cp7a-bench"

function New-Dir([string]$Path) {
  New-Item -ItemType Directory -Force -Path $Path | Out-Null
}

function Write-Json([string]$Path, $Value) {
  $Value | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $Path -Encoding utf8NoBOM
}

function Get-MainHashes {
  $main = [ordered]@{}
  foreach ($rel in $MainFiles) {
    $p = Join-Path $HomeLive $rel
    if (Test-Path -LiteralPath $p) {
      $main[$rel] = (Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash
    } else {
      $main[$rel] = $null
    }
  }
  $sidecars = @(Get-ChildItem -LiteralPath $HomeLive -Recurse -File -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -match '-(wal|shm)$' } |
    ForEach-Object { $_.FullName.Substring($HomeLive.Length + 1) })
  [pscustomobject]@{
    main = $main
    sidecars = $sidecars
    empty_index = (Test-Path -LiteralPath (Join-Path $HomeLive "index\empty.db"))
    empty_knowledge = (Test-Path -LiteralPath (Join-Path $HomeLive "knowledge\empty.db"))
  }
}

function Reset-Home {
  foreach ($d in @("index", "knowledge")) {
    $p = Join-Path $HomeLive $d
    if (Test-Path -LiteralPath $p) { Remove-Item -LiteralPath $p -Recurse -Force }
    New-Dir $p
  }
  foreach ($rel in $MainFiles) {
    Copy-Item -LiteralPath (Join-Path $HomePristine $rel) -Destination (Join-Path $HomeLive $rel) -Force
  }
}

function Compare-Main($Before, $After) {
  $ok = $true
  foreach ($rel in $MainFiles) {
    if ($Before.main[$rel] -ne $After.main[$rel]) { $ok = $false }
  }
  if ($After.empty_index -or $After.empty_knowledge) { $ok = $false }
  $ok
}

function Invoke-Api([string]$Dir, [string[]]$ApiArgs) {
  Push-Location -LiteralPath $Dir
  try {
    $out = & opencode @ApiArgs 2>&1
    $code = $LASTEXITCODE
    $text = ($out | ForEach-Object { "$_" }) -join "`n"
  } catch {
    $code = 1
    $text = "$_"
  } finally {
    Pop-Location
  }
  [pscustomobject]@{ code = $code; text = $text }
}

function Get-PluginEvidence([string]$Sid) {
  $lines = @()
  if (Test-Path -LiteralPath $PluginLog) {
    $lines = @(Get-Content -LiteralPath $PluginLog -ErrorAction SilentlyContinue | Where-Object { $_ -like "*$Sid*" })
  }
  $events = @()
  foreach ($line in $lines) {
    try { $events += ($line | ConvertFrom-Json) } catch {}
  }
  $ctx = $events | Where-Object kind -eq "context" | Select-Object -First 1
  $kept = @()
  $removed = @()
  if ($ctx) {
    if ($ctx.kept) { $kept = @($ctx.kept) }
    if ($ctx.removed) { $removed = @($ctx.removed) }
  }
  [pscustomobject]@{
    lines = $lines
    events = $events
    kept = $kept
    removed = $removed
    blocked = @($events | Where-Object kind -eq "blocked").Count
    allowed = @($events | Where-Object kind -eq "allowed").Count
  }
}

function Test-SetEqual($A, $B) {
  $sa = @($A | Sort-Object -Unique)
  $sb = @($B | Sort-Object -Unique)
  return (($sa -join ",") -eq ($sb -join ","))
}

function New-Session([string]$Dir, [string]$Title) {
  $body = @{
    agent = $Agent
    model = $Model
    location = @{ directory = ($Dir -replace "\\", "/") }
    title = $Title
  } | ConvertTo-Json -Compress -Depth 10
  Invoke-Api $Dir @("api", "session.create", "-d", $body)
}

function Post-Prompt([string]$Dir, [string]$Sid, [string]$Prompt) {
  $body = @{ text = $Prompt } | ConvertTo-Json -Compress -Depth 5
  Invoke-Api $Dir @("api", "post", "/api/session/$Sid/prompt", "-d", $body)
}

function Get-Messages([string]$Dir, [string]$Sid) {
  Invoke-Api $Dir @("api", "get", "/api/session/$Sid/message", "--param", "limit=1000")
}

function Get-Session([string]$Dir, [string]$Sid) {
  Invoke-Api $Dir @("api", "get", "/api/session/$Sid")
}

function Wait-Idle([string]$Dir, [string]$Sid, [int]$TimeoutSec) {
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  $last = $null
  $lastRaw = $null
  $idle = $null
  $polls = 0
  $errors = 0
  while ($sw.Elapsed.TotalSeconds -lt $TimeoutSec) {
    Start-Sleep -Milliseconds 1500
    $polls++
    $r = Get-Messages $Dir $Sid
    if ($r.code -ne 0) { $errors++; continue }
    try {
      $m = $r.text | ConvertFrom-Json
      $last = $m
      $lastRaw = $r.text
      $idle = @($m.data | Where-Object type -eq "idle") | Select-Object -Last 1
      if ($idle) { break }
    } catch { $errors++ }
  }
  [pscustomobject]@{
    msgs = $last
    raw = $lastRaw
    idle = $idle
    timed_out = (-not $idle)
    polls = $polls
    errors = $errors
    elapsed_ms = [math]::Round($sw.Elapsed.TotalMilliseconds, 1)
  }
}

function Run-Once {
  param(
    [string]$TaskId,
    [string]$Arm,
    [string]$Order,
    [string]$Title,
    [string]$Prompt,
    [string]$RunDir,
    [bool]$Warmup,
    [int]$TimeoutSec
  )
  New-Dir $RunDir
  $expected = $Expected[$Arm]
  $before = $null
  $after = $null
  $created = $null
  $promptPosted = $false
  $wait = $null
  $session = $null
  $sessionRaw = $null
  $plugin = $null
  $errorText = $null
  $timings = [ordered]@{}
  $startUtc = (Get-Date).ToUniversalTime().ToString("o")
  $createMs = $null
  $promptMs = $null
  $promptCompletedMs = $null

  try {
    Reset-Home
    $before = Get-MainHashes
    Write-Json (Join-Path $RunDir "home-before.json") $before

    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $cs = [System.Diagnostics.Stopwatch]::StartNew()
    $r = New-Session $ArmDir[$Arm] $Title
    $cs.Stop()
    $createMs = [math]::Round($cs.Elapsed.TotalMilliseconds, 1)
    if ($r.code -ne 0) { throw "session.create failed: $($r.text)" }
    $created = $r.text | ConvertFrom-Json
    $sid = $created.data.id
    if (-not $sid) { throw "session.create returned no id: $($r.text)" }

    $ps = [System.Diagnostics.Stopwatch]::StartNew()
    $p = Post-Prompt $ArmDir[$Arm] $sid $Prompt
    $ps.Stop()
    $promptMs = [math]::Round($ps.Elapsed.TotalMilliseconds, 1)
    if ($p.code -ne 0) { throw "prompt post failed: $($p.text)" }
    $promptPosted = $true
    $promptCompletedMs = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()

    $wait = Wait-Idle $ArmDir[$Arm] $sid $TimeoutSec
    $sr = Get-Session $ArmDir[$Arm] $sid
    if ($sr.code -eq 0) {
      $sessionRaw = $sr.text
      $session = $sr.text | ConvertFrom-Json
    }

    if ($wait.raw) {
      Set-Content -LiteralPath (Join-Path $RunDir "messages.json") -Value $wait.raw -Encoding utf8NoBOM -NoNewline
    } else {
      Set-Content -LiteralPath (Join-Path $RunDir "messages.json") -Value '{"data":[]}' -Encoding utf8NoBOM -NoNewline
    }
    if ($sessionRaw) {
      Set-Content -LiteralPath (Join-Path $RunDir "session.json") -Value $sessionRaw -Encoding utf8NoBOM -NoNewline
    }
    Set-Content -LiteralPath (Join-Path $RunDir "prompt.txt") -Value $Prompt -Encoding utf8NoBOM -NoNewline

    Start-Sleep -Milliseconds 300
    $plugin = Get-PluginEvidence $sid
    if ($plugin.lines.Count -gt 0) {
      Set-Content -LiteralPath (Join-Path $RunDir "plugin.jsonl") -Value $plugin.lines -Encoding utf8NoBOM
    }

    $after = Get-MainHashes
    Write-Json (Join-Path $RunDir "home-after.json") $after

    $msgs = $wait.msgs
    $assistants = @()
    if ($msgs -and $msgs.data) { $assistants = @($msgs.data | Where-Object type -eq "assistant") }
    $first = $assistants | Sort-Object { $_.time.created } | Select-Object -First 1
    $firstMs = $null
    if ($first -and $promptCompletedMs) { $firstMs = [long]$first.time.created - $promptCompletedMs }
    $idleMsg = $wait.idle
    $outcome = $null
    if ($idleMsg) { $outcome = $idleMsg.outcome }
    if ($session -and $session.data.outcome) { $outcome = $session.data.outcome }

    $status = "ok"
    if ($wait.timed_out) { $status = "timeout" }
    if ($wait.errors -gt 0 -and -not $wait.raw) { $status = "error" }
    if ($outcome -and $outcome -ne "succeeded") { $status = "error" }

    $meta = [ordered]@{
      task = $TaskId
      arm = $Arm
      order = $Order
      warmup = $Warmup
      title = $Title
      agent = $Agent
      model = $Model
      location = $ArmDir[$Arm]
      session = $sid
      prompt = $Prompt
      started_utc = $startUtc
      finished_utc = (Get-Date).ToUniversalTime().ToString("o")
      status = $status
      outcome = $outcome
      timed_out = $wait.timed_out
      polls = $wait.polls
      poll_errors = $wait.errors
      session_create_ms = $createMs
      prompt_post_ms = $promptMs
      turn_ms = $wait.elapsed_ms
      prompt_completed_utc_ms = $promptCompletedMs
      first_assistant_ms = $firstMs
      session_time = $session.data.time
      session_tokens = $session.data.tokens
      session_model = $session.data.model
      message_count = @($msgs.data).Count
      assistant_count = $assistants.Count
      home_unchanged = (Compare-Main $before $after)
      home_before = $before
      home_after = $after
      expected_tools = $expected
      context_kept = $plugin.kept
      context_removed = $plugin.removed
      exposure_ok = (Test-SetEqual $plugin.kept $expected)
      plugin_blocked = $plugin.blocked
      plugin_allowed = $plugin.allowed
      plugin_events = $plugin.events.Count
      error = $errorText
      messages_file = "messages.json"
      session_file = $(if ($sessionRaw) { "session.json" } else { $null })
      plugin_file = $(if ($plugin.lines.Count -gt 0) { "plugin.jsonl" } else { $null })
    }
  } catch {
    $errorText = "$_"
    $sid = $null
    if ($created -and $created.data) { $sid = $created.data.id }
    if (-not $after) { try { $after = Get-MainHashes } catch {} }
    if ($before -and $after) {
      try { Write-Json (Join-Path $RunDir "home-after.json") $after } catch {}
    }
    if (-not (Test-Path -LiteralPath (Join-Path $RunDir "messages.json"))) {
      Set-Content -LiteralPath (Join-Path $RunDir "messages.json") -Value '{"data":[]}' -Encoding utf8NoBOM -NoNewline
    }
    $meta = [ordered]@{
      task = $TaskId
      arm = $Arm
      order = $Order
      warmup = $Warmup
      title = $Title
      agent = $Agent
      model = $Model
      location = $ArmDir[$Arm]
      session = $sid
      prompt = $Prompt
      started_utc = $startUtc
      finished_utc = (Get-Date).ToUniversalTime().ToString("o")
      status = "error"
      outcome = $null
      timed_out = $false
      polls = 0
      poll_errors = 0
      session_create_ms = $createMs
      prompt_post_ms = $promptMs
      turn_ms = $(if ($wait) { $wait.elapsed_ms } else { $null })
      prompt_completed_utc_ms = $promptCompletedMs
      first_assistant_ms = $null
      session_time = $null
      session_tokens = $null
      session_model = $null
      message_count = 0
      assistant_count = 0
      home_unchanged = $(if ($before -and $after) { Compare-Main $before $after } else { $false })
      home_before = $before
      home_after = $after
      expected_tools = $expected
      context_kept = @()
      context_removed = @()
      exposure_ok = $false
      plugin_blocked = 0
      plugin_allowed = 0
      plugin_events = 0
      error = $errorText
      messages_file = "messages.json"
      session_file = $null
      plugin_file = $null
    }
  }

  Write-Json (Join-Path $RunDir "meta.json") $meta
  [pscustomobject]$meta
}

function Warmup([string]$Arm) {
  $dir = Join-Path $Runs "startup\$Arm"
  New-Dir $dir
  $metaPath = Join-Path $dir "meta.json"
  if ($SkipWarmup) {
    if (-not (Test-Path -LiteralPath $metaPath)) { throw "warmup metadata missing for $Arm" }
    $m = Get-Content -LiteralPath $metaPath -Raw | ConvertFrom-Json
    if (-not $m.success) { throw "warmup was not successful for $Arm" }
    Write-Output "WARMUP reused $Arm session=$($m.session)"
    return
  }
  $attempts = @()
  $success = $false
  $chosen = $null
  for ($n = 1; $n -le 3; $n++) {
    $attemptDir = Join-Path $dir "attempt-$n"
    $m = Run-Once -TaskId "startup" -Arm $Arm -Order "" -Title "CP7A startup READY $Arm" -Prompt "Reply READY. Do not call any tools." -RunDir $attemptDir -Warmup $true -TimeoutSec $WarmTimeoutSec
    $attempts += [pscustomobject]@{ attempt = $n; session = $m.session; status = $m.status; exposure_ok = $m.exposure_ok; context_kept = $m.context_kept; turn_ms = $m.turn_ms }
    if ($m.status -eq "ok" -and $m.exposure_ok) { $success = $true; $chosen = $m; break }
    Start-Sleep -Seconds 5
  }
  $summary = [ordered]@{
    arm = $Arm
    success = $success
    chosen_session = $(if ($chosen) { $chosen.session } else { $null })
    chosen_dir = $(if ($chosen) { "attempt-$($attempts.Count)" } else { $null })
    attempts = $attempts
    expected_tools = $Expected[$Arm]
    ready_baseline = $success
    finished_utc = (Get-Date).ToUniversalTime().ToString("o")
  }
  Write-Json $metaPath $summary
  if (-not $success) { throw "warmup failed for $Arm" }
  Write-Output "WARMUP ok $Arm session=$($chosen.session) kept=$($chosen.context_kept -join ',')"
}

$corpus = Get-Content -LiteralPath $CorpusPath -Raw | ConvertFrom-Json
$corpusHash = (Get-FileHash -LiteralPath $CorpusPath -Algorithm SHA256).Hash
New-Dir $Runs
New-Dir (Join-Path $Runs "tasks")
New-Dir (Join-Path $Runs "startup")
Write-Output "CORPUS $corpusHash tasks=$($corpus.tasks.Count)"
Write-Output "MODEL $($Model.providerID)/$($Model.id) AGENT $Agent"

Warmup "A"
Warmup "B"
if ($WarmOnly) {
  Write-Output "WARMONLY done"
  exit 0
}

$tasks = @($corpus.tasks | Where-Object { $Only.Count -eq 0 -or $Only -contains $_.id })
foreach ($task in $tasks) {
  $arms = @("A", "B")
  if ($task.order -eq "B") { $arms = @("B", "A") }
  foreach ($arm in $arms) {
    $runDir = Join-Path $Runs "tasks\$($task.id)\$arm"
    if ($Resume -and (Test-Path -LiteralPath (Join-Path $runDir "meta.json"))) {
      Write-Output "SKIP $($task.id) $arm"
      continue
    }
    $prompt = $task.question + "`n`n" + $corpus.access.$arm
    Write-Output "START $($task.id) $arm"
    $m = Run-Once -TaskId $task.id -Arm $arm -Order $task.order -Title "CP7A $($task.id) $arm" -Prompt $prompt -RunDir $runDir -Warmup $false -TimeoutSec $TaskTimeoutSec
    Write-Output "DONE $($task.id) $arm status=$($m.status) exposure=$($m.exposure_ok) home=$($m.home_unchanged) session=$($m.session) turn_ms=$($m.turn_ms)"
  }
}
Write-Output "RUNNER done tasks=$($tasks.Count)"
