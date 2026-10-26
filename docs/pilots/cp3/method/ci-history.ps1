# CP3 harness step: the CI runner evidence.
#
# Collects, from the GitHub API, the facts behind the macOS CI finding:
#   - the cancelled macOS job payloads (no runner, zero steps)
#   - GitHub's own job annotations
#   - the macOS job history across the repository's recent runs
#   - the required status checks of the `protect-master` ruleset
#
# Usage:
#   pwsh -File ci-history.ps1 -Out <dir>
param(
  [string]$OutDir = '',
  [string]$Repo = 'RyanAraaful/riltra',
  [string]$Run = '39438093811',
  [string]$SuccessRun = '',
  [int]$RunCount = 40,
  [string]$JobIds = ''
)
$ErrorActionPreference = 'Continue'
if (-not $OutDir) { $OutDir = Join-Path $env:TEMP 'opencode\cp3\raw\ci' }
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

function Get-Api {
  param([string]$Path)
  $raw = gh api $Path 2>$null | Out-String
  if (-not $raw) { return $null }
  try { return $raw | ConvertFrom-Json } catch { return $raw }
}

# --- the cancelled macOS job and its annotations --------------------------
# The job ids of every attempt are archived: a re-run creates a NEW job id.
$ids = @()
if ($JobIds) { $ids = @($JobIds -split ',' | ForEach-Object { $_.Trim() } | Where-Object { $_ }) }
$jobs = Get-Api "repos/$Repo/actions/runs/$Run/jobs?per_page=20"
foreach ($j in @($jobs.jobs | Where-Object { $_.name -like '*macos*' })) {
  if ($ids -notcontains "$($j.id)") { $ids += "$($j.id)" }
}
foreach ($id in $ids) {
  $j = Get-Api "repos/$Repo/actions/jobs/$id"
  if (-not $j) { continue }
  $payload = [ordered]@{
    run = $Run
    job_id = $j.id
    job_name = $j.name
    status = $j.status
    conclusion = $j.conclusion
    started_at = $j.started_at
    completed_at = $j.completed_at
    runner_id = $j.runner_id
    runner_name = $j.runner_name
    runner_group_id = $j.runner_group_id
    labels = $j.labels
    steps_executed = @($j.steps).Count
    step_names = @($j.steps | ForEach-Object { $_.name })
    annotations = @()
  }
  $ann = Get-Api "repos/$Repo/check-runs/$id/annotations"
  if ($ann) {
    $payload.annotations = @($ann | ForEach-Object {
      [ordered]@{ level = $_.annotation_level; path = $_.path; message = $_.message }
    })
  }
  $file = Join-Path $OutDir ("cancelled-macos-job-$id.json")
  [System.IO.File]::WriteAllText($file, ($payload | ConvertTo-Json -Depth 6), (New-Object System.Text.UTF8Encoding($false)))
  "archived $file"
}

# --- the macOS job of the post-change run (the runner that was used) ------
if ($SuccessRun) {
  $sj = Get-Api "repos/$Repo/actions/runs/$SuccessRun/jobs?per_page=20"
  foreach ($j in @($sj.jobs | Where-Object { $_.name -like '*macos*' })) {
    $payload = [ordered]@{
      run = $SuccessRun
      job_id = $j.id
      job_name = $j.name
      status = $j.status
      conclusion = $j.conclusion
      labels = $j.labels
      runner_name = $j.runner_name
      steps_executed = @($j.steps).Count
      step_names = @($j.steps | ForEach-Object { $_.name })
      started_at = $j.started_at
      completed_at = $j.completed_at
    }
    $file = Join-Path $OutDir "post-change-macos-job-$($j.id).json"
    [System.IO.File]::WriteAllText($file, ($payload | ConvertTo-Json -Depth 6), (New-Object System.Text.UTF8Encoding($false)))
    "archived $file"
  }
}

# --- the macOS job history over the repository's recent runs ---------------
$rows = @()
$runs = Get-Api "repos/$Repo/actions/runs?per_page=$RunCount"
foreach ($r in $runs.workflow_runs) {
  $rj = Get-Api "repos/$Repo/actions/runs/$($r.id)/jobs?per_page=20"
  if (-not $rj) { continue }
  foreach ($j in $rj.jobs) {
    if ($j.name -notlike '*macos*') { continue }
    $rows += [pscustomobject]@{
      run_id = $r.id
      head_branch = $r.head_branch
      head_sha = $r.head_sha
      job_name = $j.name
      label = ($j.labels -join ',')
      conclusion = $j.conclusion
      runner_name = $j.runner_name
      runner_id = $j.runner_id
      steps_executed = @($j.steps).Count
      started_at = $j.started_at
      completed_at = $j.completed_at
    }
  }
}
$rows | ConvertTo-Json -Depth 4 | Set-Content -Path (Join-Path $OutDir 'macos-job-history.json') -Encoding utf8
"archived macos-job-history.json ($($rows.Count) macOS jobs)"

# --- the required status checks of the ruleset ----------------------------
$rulesets = Get-Api "repos/$Repo/rulesets"
foreach ($rs in @($rulesets)) {
  $detail = Get-Api "repos/$Repo/rulesets/$($rs.id)"
  $checks = @()
  foreach ($rule in $detail.rules) {
    if ($rule.type -eq 'required_status_checks') {
      $checks = @($rule.parameters.required_status_checks | ForEach-Object { $_.context })
    }
  }
  $out = [ordered]@{
    id = $detail.id
    name = $detail.name
    enforcement = $detail.enforcement
    target = $detail.target
    branch_conditions = $detail.conditions
    required_status_checks = $checks
  }
  $file = Join-Path $OutDir "ruleset-$($detail.name).json"
  [System.IO.File]::WriteAllText($file, ($out | ConvertTo-Json -Depth 6), (New-Object System.Text.UTF8Encoding($false)))
  "archived $file"
}
