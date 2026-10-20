# SPDX-License-Identifier: Apache-2.0
# CP3-style harness for the clangd pilot: drive the Riltra CLI against a
# language server and capture what the server answered, verified against the
# source bytes.
#
# Every probe states where to click (a marker string in a file), what to ask
# for, and what the answer must contain. The harness never hard-codes a
# line number: the marker is located in the file, so the probe survives an
# edit to the fixture.
#
# Usage:
#   pwsh -File probe.ps1 -Root <bench> -Probes <probes.json> -Out <out.json>
param(
  [Parameter(Mandatory = $true)][string]$Root,
  [Parameter(Mandatory = $true)][string]$Probes,
  [Parameter(Mandatory = $true)][string]$Out,
  [string]$Riltra = '',
  [string]$RiltraHome = '',
  [string]$Server = '',
  [string]$Repo = ''
)
$ErrorActionPreference = 'Stop'
if (-not $Riltra) { $Riltra = Join-Path $Root 'riltra.exe' }
if (-not $RiltraHome) { $RiltraHome = Join-Path $Root 'home' }
$env:RILTRA_HOME = $RiltraHome

# Locate a marker inside a file: returns @{line; col} 1-based, or $null.
function Find-Marker {
  param([string]$Path, [string]$Marker, [int]$Occurrence = 1, [int]$Offset = 0)
  $lines = Get-Content -LiteralPath $Path
  $seen = 0
  for ($i = 0; $i -lt $lines.Count; $i++) {
    $idx = $lines[$i].IndexOf($Marker, [System.StringComparison]::Ordinal)
    while ($idx -ge 0) {
      $seen++
      if ($seen -eq $Occurrence) {
        return @{ line = $i + 1; col = $idx + $Offset + 1 }
      }
      $idx = $lines[$i].IndexOf($Marker, $idx + 1, [System.StringComparison]::Ordinal)
    }
  }
  return $null
}

# Run the CLI once, sample the server's memory while it runs, and return the
# output plus a one-line summary of the answer.
function Invoke-Cli {
  param([string[]]$CliArgs, [string]$StdoutFile, [string]$StderrFile)
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  $proc = Start-Process -FilePath $Riltra -ArgumentList $CliArgs -NoNewWindow -PassThru `
    -RedirectStandardOutput $StdoutFile -RedirectStandardError $StderrFile
  # Sample the server's memory while it runs: the process is gone by the
  # time the CLI returns, so the peak has to be caught in flight. Only the
  # CLI's OWN children are sampled, so a stray clangd on the machine cannot
  # contribute to the number.
  $peak = 0
  $serverName = [System.IO.Path]::GetFileNameWithoutExtension($Server)
  while (-not $proc.HasExited) {
    $children = Get-CimInstance Win32_Process -Filter "ParentProcessId = $($proc.Id)" -ErrorAction SilentlyContinue
    foreach ($child in @($children)) {
      if ($child.Name -ne "$serverName.exe") { continue }
      $sp = Get-Process -Id $child.ProcessId -ErrorAction SilentlyContinue
      if ($sp -and $sp.PeakWorkingSet64 -gt $peak) { $peak = $sp.PeakWorkingSet64 }
    }
    Start-Sleep -Milliseconds 100
  }
  $proc.WaitForExit()
  $sw.Stop()
  $text = ((Get-Content -LiteralPath $StdoutFile -Raw) + (Get-Content -LiteralPath $StderrFile -Raw)).Trim()
  return [ordered]@{
    output = $text
    elapsed_ms = [int]$sw.ElapsedMilliseconds
    peak = $peak
  }
}

# The source text of one reported span, so an answer is checked against the
# bytes and not against a file name alone.
function Get-SpanText {
  param([string]$File, [int]$StartLine, [int]$EndLine, [int]$StartCol, [int]$EndCol)
  $lines = Get-Content -LiteralPath $File
  if ($StartLine -lt 1 -or $EndLine -gt $lines.Count) { return $null }
  if ($StartLine -eq $EndLine) {
    $text = $lines[$StartLine - 1]
    if ($StartCol -gt $text.Length) { return $null }
    $len = [Math]::Min($EndCol - $StartCol, $text.Length - $StartCol)
    return $text.Substring($StartCol - 1, $len)
  }
  $parts = @($lines[$StartLine - 1].Substring($StartCol - 1))
  for ($l = $StartLine + 1; $l -lt $EndLine; $l++) { $parts += $lines[$l - 1] }
  $parts += $lines[$EndLine - 1].Substring(0, [Math]::Min($EndCol, $lines[$EndLine - 1].Length))
  return ($parts -join "`n")
}

$probeList = Get-Content -LiteralPath $Probes -Raw | ConvertFrom-Json
$results = @()
foreach ($probe in $probeList) {
  $repoRoot = Join-Path $Root $probe.repo
  $abs = Join-Path $repoRoot $probe.path
  $occurrence = if ($null -ne $probe.occurrence) { [int]$probe.occurrence } else { 1 }
  $offset = if ($null -ne $probe.offset) { [int]$probe.offset } else { 0 }
  $pos = Find-Marker -Path $abs -Marker $probe.find -Occurrence $occurrence -Offset $offset
  if (-not $pos) {
    $results += [ordered]@{ id = $probe.id; error = "marker not found: '$($probe.find)' in $($probe.path)" }
    continue
  }
  $cliArgs = @('semantic', 'query', '--repo', $probe.repo, '--path', $probe.path,
    '--line', "$($pos.line)", '--col', "$($pos.col)")
  if ($probe.kind -eq 'references') { $cliArgs += '--refs' }
  if ($Server) { $cliArgs += @('--lsp-server', $Server) }

  # Every probe is asked TWICE. The first answer can be the declaration
  # while clangd's project index is still being built (see the readiness
  # experiment); the second is the stable one. Both are recorded, and the
  # expectation is checked against the second.
  $stdoutFile = [System.IO.Path]::GetTempFileName()
  $stderrFile = [System.IO.Path]::GetTempFileName()
  $firstOut = (Invoke-Cli -CliArgs $cliArgs -StdoutFile $stdoutFile -StderrFile $stderrFile)
  $secondOut = (Invoke-Cli -CliArgs $cliArgs -StdoutFile $stdoutFile -StderrFile $stderrFile)
  Remove-Item -LiteralPath $stdoutFile, $stderrFile -Force -ErrorAction SilentlyContinue
  $outText = $secondOut.output

  $entry = [ordered]@{
    id = $probe.id
    kind = $probe.kind
    path = $probe.path
    marker = $probe.find
    line = $pos.line
    col = $pos.col
    elapsed_ms = $secondOut.elapsed_ms
    first_elapsed_ms = $firstOut.elapsed_ms
    server_peak_working_set = $secondOut.peak
    first_server_peak_working_set = $firstOut.peak
    first_answer = $firstOut.output
    second_answer = $secondOut.output
    stable = ($firstOut.output -eq $secondOut.output)
    raw = $outText.Trim()
  }
  try {
    $json = $outText | ConvertFrom-Json
    $entry.provider = $json.provider
    $entry.provider_version = $json.provider_version
    $entry.verification = $json.verification
    $entry.fallback = $json.fallback
    $entry.server_diagnostics = @($json.server_diagnostics)
    $hits = @()
    if ($probe.kind -eq 'references') { $hits = @($json.references) } else { $hits = @($json.definitions) }
    $entry.hit_count = $hits.Count
    $entry.hits = @()
    foreach ($h in $hits) {
      $span = $h.span
      $hfile = Join-Path $repoRoot ($h.path -replace '/', '\')
      $text = $null
      if (Test-Path -LiteralPath $hfile) {
        $text = Get-SpanText -File $hfile -StartLine ([int]$span.start_line + 1) -EndLine ([int]$span.end_line + 1) `
          -StartCol ([int]$span.start_col + 1) -EndCol ([int]$span.end_col + 1)
      }
      $entry.hits += [ordered]@{
        path = $h.path
        line = [int]$span.start_line + 1
        col = [int]$span.start_col + 1
        end_line = [int]$span.end_line + 1
        end_col = [int]$span.end_col + 1
        byte_start = $span.byte_start
        byte_end = $span.byte_end
        roles = $h.roles
        precision = $h.precision
        verification = $h.verification
        text = $text
      }
    }
    if ($probe.expect) {
      $entry.expect = $probe.expect
      $entry.expect_holds = $null
      $expectedPath = $probe.expect.path
      $matched = $hits | Where-Object { $_.path -eq $expectedPath }
      if ($probe.expect.find) {
        # The expected text must sit on the REPORTED line, not merely
        # somewhere in the expected file: a wrong answer inside the right
        # file must not pass.
        $ok = $false
        foreach ($m in $matched) {
          $mfile = Join-Path $repoRoot ($m.path -replace '/', '\')
          if (-not (Test-Path -LiteralPath $mfile)) { continue }
          $all = Get-Content -LiteralPath $mfile
          $ln = [int]$m.span.start_line
          if ($ln -ge 0 -and $ln -lt $all.Count -and $all[$ln].Contains($probe.expect.find)) {
            $ok = $true
            break
          }
        }
        $entry.expect_holds = $ok
      } elseif ($probe.expect.count) {
        $entry.expect_holds = ($hits.Count -eq [int]$probe.expect.count)
      }
      # An exclusion: no reported hit may sit on a line carrying this text.
      # It is how "this call site must NOT be a reference" is stated.
      if ($probe.expect.exclude_find) {
        $leaked = $false
        foreach ($h in $hits) {
          $hfile = Join-Path $repoRoot ($h.path -replace '/', '\')
          if (-not (Test-Path -LiteralPath $hfile)) { continue }
          $all = Get-Content -LiteralPath $hfile
          $ln = [int]$h.span.start_line
          if ($ln -ge 0 -and $ln -lt $all.Count -and $all[$ln].Contains($probe.expect.exclude_find)) {
            $leaked = $true
          }
        }
        $entry.exclude_leaked = $leaked
        if ($null -eq $entry.expect_holds) { $entry.expect_holds = (-not $leaked) }
        else { $entry.expect_holds = ($entry.expect_holds -and (-not $leaked)) }
      }
    }
  } catch {
    $entry.parse_error = $_.Exception.Message
  }
  $results += $entry
}

$results | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $Out -Encoding utf8
"wrote $Out ($($results.Count) probes)"
