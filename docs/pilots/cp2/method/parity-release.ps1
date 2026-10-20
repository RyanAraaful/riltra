# CP2 harness step: same-tree fact parity (incremental home vs an independent
# full rebuild) plus the release-profile cold index and warm queries, which
# are the fair comparison with the blueprint's 14.3 warm-query target.
#
# Usage:
#   pwsh -File parity-release.ps1 -BenchRoot <dir> -Bin <riltra.exe> -Snapshot <dir>
param(
  [string]$BenchRoot = '',
  [string]$Bin = '',
  [string]$Snapshot = '',
  [string]$MmspRoot = '',
  [string]$Out = '',
  [string]$HomeDir = '',
  [string]$ReleaseBin = '',
  [string]$Pin = 'd62f06a576d8e76b705c8ecc57fc380f7ebc9c44'
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'common.ps1')
$cfg = Initialize-Cp2Harness -BenchRoot $BenchRoot -Bin $Bin -Snapshot $Snapshot -MmspRoot $MmspRoot -Out $Out
if (-not $HomeDir) { $HomeDir = Join-Path $cfg.BenchRoot 'home-main' }
if (-not $ReleaseBin) { $ReleaseBin = Get-DefaultBin -Profile 'release' }
$snap = Assert-SnapshotPath -Path $cfg.Snapshot -Root $cfg.BenchRoot -MmspRoot $cfg.MmspRoot
$raw = New-BenchDir -Path (Join-Path $cfg.Out 'incremental') -Root $cfg.BenchRoot
$digest = Join-Path $PSScriptRoot 'db-mutation-check.cjs'

"===== 1. restore the snapshot to the pinned tree ====="
Restore-Snapshot -Snapshot $snap -Root $cfg.BenchRoot -Pin $Pin -MmspRoot $cfg.MmspRoot | Out-Null
"snapshot HEAD = $(git -C $snap rev-parse HEAD)  porcelain = $((git -C $snap status --porcelain | Measure-Object -Line).Lines)"

"===== 2. incremental home: re-index the pinned tree (no-op refresh) ====="
$env:RILTRA_HOME = $HomeDir
$noop = & $cfg.Bin index --repo mmsp-pin 2>&1 | Out-String
if ($LASTEXITCODE -ne 0) { throw "the no-op refresh failed with exit $LASTEXITCODE" }
[System.IO.File]::WriteAllText((Join-Path $raw 'reeindex-pinned.json'), $noop, (New-Object System.Text.UTF8Encoding($false)))
$j = $noop | ConvertFrom-Json
"  mode=$($j.mode) reused=$($j.reused) reparsed=$($j.reparsed) reason=$($j.fallback_reason) status=$($j.status_at_write)"

"===== 3. oracle: independent full rebuild, fresh home, same tree ====="
$oracleDir = Reset-BenchDir (Join-Path $cfg.BenchRoot 'home-oracle2') -Root $cfg.BenchRoot
$env:RILTRA_HOME = $oracleDir
& $cfg.Bin init --repo $snap --name mmsp-pin | Out-Null
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$full = & $cfg.Bin index --repo mmsp-pin --full 2>&1 | Out-String
$code = $LASTEXITCODE
$sw.Stop()
if ($code -ne 0) { throw "the oracle rebuild failed with exit $code" }
[System.IO.File]::WriteAllText((Join-Path $raw 'oracle2.json'), $full, (New-Object System.Text.UTF8Encoding($false)))
$o = $full | ConvertFrom-Json
"  mode=$($o.mode) reused=$($o.reused) reparsed=$($o.reparsed) reason=$($o.fallback_reason) wall_s=$([math]::Round($sw.Elapsed.TotalSeconds,2))"

"===== 4. same-tree fact parity (order-independent digests) ====="
# The digests are archived with the run, so the parity can be re-verified
# from the archive alone.
$incJson = Join-Path $raw 'inc-facts.json'
$orcJson = Join-Path $raw 'oracle-facts.json'
node $digest (Join-Path $HomeDir 'index\mmsp-pin.db') $incJson | Out-Null
if ($LASTEXITCODE -ne 0) { throw "the fact digest script failed" }
node $digest (Join-Path $oracleDir 'index\mmsp-pin.db') $orcJson | Out-Null
if ($LASTEXITCODE -ne 0) { throw "the fact digest script failed" }
$ii = Get-Content $incJson -Raw | ConvertFrom-Json
$oo = Get-Content $orcJson -Raw | ConvertFrom-Json
$parityFailures = @()
foreach ($t in 'files','symbols','imports','calls','edges') {
  "  {0,-9} rows {1,7} vs {2,7}  identical = {3}" -f $t, $ii.tables.$t.rows, $oo.tables.$t.rows, ($ii.tables.$t.digest -eq $oo.tables.$t.digest)
  if ($ii.tables.$t.digest -ne $oo.tables.$t.digest) { $parityFailures += $t }
}
"  COMBINED IDENTICAL = $($ii.combined_digest -eq $oo.combined_digest)"
if ($ii.combined_digest -ne $oo.combined_digest) { $parityFailures += 'combined' }

"===== 5. release profile: cold index ====="
if (-not (Test-Path -LiteralPath $ReleaseBin)) { throw "release binary not found: $ReleaseBin" }
$relHome = Reset-BenchDir (Join-Path $cfg.BenchRoot 'home-release') -Root $cfg.BenchRoot
$env:RILTRA_HOME = $relHome
& $ReleaseBin init --repo $snap --name mmsp-pin | Out-Null
$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $ReleaseBin
$psi.Arguments = 'index --repo mmsp-pin --full'
$psi.UseShellExecute = $false
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError = $true
$psi.Environment['RILTRA_HOME'] = $relHome
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$proc = [System.Diagnostics.Process]::Start($psi)
$outTask = $proc.StandardOutput.ReadToEndAsync()
$peak = [int64]0
while (-not $proc.HasExited) {
  try { $proc.Refresh(); if ($proc.PeakWorkingSet64 -gt $peak) { $peak = $proc.PeakWorkingSet64 } } catch { }
  Start-Sleep -Milliseconds 40
}
$proc.WaitForExit()
$sw.Stop()
$relOut = $outTask.Result
"  release cold index: wall_s=$([math]::Round($sw.Elapsed.TotalSeconds,2)) peak_rss_mib=$([math]::Round($peak/1MB,1)) exit=$($proc.ExitCode)"
$relOut
[System.IO.File]::WriteAllText((Join-Path $cfg.Out 'cold-index-release.txt'), $relOut, (New-Object System.Text.UTF8Encoding($false)))

"===== 6. release profile: warm queries ====="
$env:RILTRA_HOME = $relHome
$queries = @(
  @{ n = 'search-PatchMatch';     a = @('search','--repo','mmsp-pin','--symbol','PatchMatch') },
  @{ n = 'search-Run';            a = @('search','--repo','mmsp-pin','--symbol','Run') },
  @{ n = 'callers-PrintDef';      a = @('callers','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.cc','--line','118','--col','27') },
  @{ n = 'callees-PatchMatchRun'; a = @('callees','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.cc','--line','191','--col','18') },
  @{ n = 'trace-depth4';          a = @('trace','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.cc','--line','233','--col','28','--depth','4') },
  @{ n = 'impact-member';         a = @('impact','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match_options.h','--line','56','--col','10') },
  @{ n = 'index-status';          a = @('index','status','--repo','mmsp-pin') },
  @{ n = 'repo-status';           a = @('repo','status','--repo','mmsp-pin') }
)
$rows = @()
foreach ($q in $queries) {
  $times = @()
  for ($i = 1; $i -le 5; $i++) {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $null = & $ReleaseBin @($q.a) 2>&1 | Out-String
    $sw.Stop()
    $times += $sw.Elapsed.TotalMilliseconds
  }
  $s = $times | Sort-Object
  $p50 = $s[2]; $p95 = $s[4]
  $rows += [pscustomobject]@{ query = $q.n; ms_p50 = [math]::Round($p50,1); ms_p95 = [math]::Round($p95,1); ms_max = [math]::Round($s[-1],1) }
  "  {0,-24} p50={1,7} ms  p95={2,7} ms  max={3,7} ms" -f $q.n, [math]::Round($p50,1), [math]::Round($p95,1), [math]::Round($s[-1],1)
}
$rows | ConvertTo-Json | Set-Content (Join-Path $cfg.Out 'perf-release.json') -Encoding utf8
"  SLO p95 <= 500 ms missed by: $($rows | Where-Object { $_.ms_p95 -gt 500 } | ForEach-Object { $_.query })"

if ($parityFailures.Count -gt 0) {
  throw "fact parity failed for: $($parityFailures -join ', ')"
}
"DONE-PARITY-RELEASE"
