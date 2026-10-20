# CP2 step 6b: same-tree parity (incremental home vs independent full rebuild)
# and the release-profile measurements for the blueprint SLO comparison.
$ErrorActionPreference = 'Stop'
. "$env:TEMP\opencode\bench\cp2\common.ps1"

$cp2 = "$env:TEMP\opencode\bench\cp2"
$bench = "$env:TEMP\opencode\bench"
$bin = '<RILTRA_ROOT>\target\debug\riltra.exe'
$rel = '<RILTRA_ROOT>\target\release\riltra.exe'
$snap = "$bench\mmsp-d62f06a"
$raw = "$cp2\raw"

"===== 1. restore the snapshot to the pinned tree ====="
git -C $snap checkout --quiet --detach d62f06a576d8e76b705c8ecc57fc380f7ebc9c44 | Out-Null
git -C $snap reset --quiet --hard d62f06a576d8e76b705c8ecc57fc380f7ebc9c44 | Out-Null
git -C $snap clean -qfd | Out-Null
"snapshot HEAD = $(git -C $snap rev-parse HEAD)  porcelain = $((git -C $snap status --porcelain | Measure-Object -Line).Lines)"

"===== 2. incremental home: re-index the pinned tree (no-op refresh) ====="
$env:RILTRA_HOME = "$bench\home-main"
$noop = & $bin index --repo mmsp-pin 2>&1 | Out-String
[System.IO.File]::WriteAllText("$raw\incremental\reeindex-pinned.json", $noop, (New-Object System.Text.UTF8Encoding($false)))
$j = $noop | ConvertFrom-Json
"  mode=$($j.mode) reused=$($j.reused) reparsed=$($j.reparsed) reason=$($j.fallback_reason) status=$($j.status_at_write)"

"===== 3. oracle: independent full rebuild, fresh home, same tree ====="
$oracleDir = Reset-BenchDir "$bench\home-oracle2"
$env:RILTRA_HOME = $oracleDir
& $bin init --repo $snap --name mmsp-pin | Out-Null
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$full = & $bin index --repo mmsp-pin --full 2>&1 | Out-String
$sw.Stop()
[System.IO.File]::WriteAllText("$raw\incremental\oracle2.json", $full, (New-Object System.Text.UTF8Encoding($false)))
$o = $full | ConvertFrom-Json
"  mode=$($o.mode) reused=$($o.reused) reparsed=$($o.reparsed) reason=$($o.fallback_reason) wall_s=$([math]::Round($sw.Elapsed.TotalSeconds,2))"

"===== 4. same-tree fact parity (order-independent digests) ====="
node "$cp2\db-mutation-check.cjs" "$bench\home-main\index\mmsp-pin.db" "$bench\inc-facts.json" > $null 2>&1
node "$cp2\db-mutation-check.cjs" "$bench\home-oracle2\index\mmsp-pin.db" "$bench\oracle-facts.json" > $null 2>&1
$ii = Get-Content "$bench\inc-facts.json" -Raw | ConvertFrom-Json
$oo = Get-Content "$bench\oracle-facts.json" -Raw | ConvertFrom-Json
foreach ($t in 'files','symbols','imports','calls','edges') {
  "  {0,-9} rows {1,7} vs {2,7}  identical = {3}" -f $t, $ii.tables.$t.rows, $oo.tables.$t.rows, ($ii.tables.$t.digest -eq $oo.tables.$t.digest)
}
"  COMBINED IDENTICAL = $($ii.combined_digest -eq $oo.combined_digest)"
"  combined: $($ii.combined_digest) / $($oo.combined_digest)"

"===== 5. release profile: cold index ====="
$relHome = Reset-BenchDir "$bench\home-release"
$env:RILTRA_HOME = $relHome
& $rel init --repo $snap --name mmsp-pin | Out-Null
$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $rel
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
"  release cold index: wall_s=$([math]::Round($sw.Elapsed.TotalSeconds,2)) peak_rss_mib=$([math]::Round($peak/1MB,1))"
$relOut
[System.IO.File]::WriteAllText("$raw\cold-index-release.txt", $relOut, (New-Object System.Text.UTF8Encoding($false)))

"===== 6. release profile: warm queries (5 reps) ====="
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
    $null = & $rel @($q.a) 2>&1 | Out-String
    $sw.Stop()
    $times += $sw.Elapsed.TotalMilliseconds
  }
  $s = $times | Sort-Object
  $p50 = $s[2]; $p95 = $s[4]
  $rows += [pscustomobject]@{ query = $q.n; ms_p50 = [math]::Round($p50,1); ms_p95 = [math]::Round($p95,1); ms_max = [math]::Round($s[-1],1) }
  "  {0,-24} p50={1,7} ms  p95={2,7} ms  max={3,7} ms" -f $q.n, [math]::Round($p50,1), [math]::Round($p95,1), [math]::Round($s[-1],1)
}
$rows | ConvertTo-Json | Set-Content "$raw\perf-release.json" -Encoding utf8
"  SLO p95 <= 500 ms: $($rows | Where-Object { $_.ms_p95 -gt 500 } | ForEach-Object { $_.query })"
"DONE-PARITY-RELEASE"
