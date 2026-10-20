# CP2 harness step: incremental correctness (CP1 section A.7 five-point parity
# contract).
#
# The sequence proves the parity on ONE source tree, the tree with the
# one-file change:
#   1. restore the snapshot to the frozen pin
#   2. refresh the baseline index (no-op) on that tree
#   3. change exactly one file
#   4. incremental indexing on the changed tree (reparsed must be 1)
#   5. an independent full rebuild (oracle) on the SAME changed tree
#   6. the fact digests of BOTH indexes, taken BEFORE the restore
#   7. the query parity on the same state
#   8. only then: restore the snapshot to the pin and verify it
#
# The comparison can fail: a digest that differs, is empty or cannot be
# computed, a source tree that does not match, a query that errors, prints
# nothing or exits non-zero - each aborts the step. The comparison logic
# lives in fact-digest.cjs and is covered by fact-digest.test.cjs.
#
# Usage:
#   pwsh -File incremental.ps1 -BenchRoot <dir> -Bin <riltra.exe> -Snapshot <dir>
param(
  [string]$BenchRoot = '',
  [string]$Bin = '',
  [string]$Snapshot = '',
  [string]$MmspRoot = '',
  [string]$Out = '',
  [string]$HomeDir = '',
  [string]$Pin = 'd62f06a576d8e76b705c8ecc57fc380f7ebc9c44'
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'common.ps1')
$cfg = Initialize-Cp2Harness -BenchRoot $BenchRoot -Bin $Bin -Snapshot $Snapshot -MmspRoot $MmspRoot -Out $Out
if (-not $HomeDir) { $HomeDir = Join-Path $cfg.BenchRoot 'home-main' }
$snap = Assert-SnapshotPath -Path $cfg.Snapshot -Root $cfg.BenchRoot -MmspRoot $cfg.MmspRoot
$out = New-BenchDir -Path (Join-Path $cfg.Out 'incremental') -Root $cfg.BenchRoot
$digest = Join-Path $PSScriptRoot 'db-mutation-check.cjs'
$compare = Join-Path $PSScriptRoot 'fact-digest.cjs'
$changedFile = 'colmap/src/colmap/mvs/patch_match_options.h'

# --- 1. restore the snapshot to the frozen pin ---
"===== 1. restore the snapshot to the pin ====="
Restore-Snapshot -Snapshot $snap -Root $cfg.BenchRoot -Pin $Pin -MmspRoot $cfg.MmspRoot | Out-Null
"  HEAD = $(git -C $snap rev-parse HEAD)  porcelain = $((git -C $snap status --porcelain | Measure-Object -Line).Lines) line(s)"

# --- 2. the baseline index on that tree ---
"===== 2. baseline: the index of the pinned tree ====="
$env:RILTRA_HOME = $HomeDir
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$noop = & $cfg.Bin index --repo mmsp-pin 2>&1 | Out-String
$noopCode = $LASTEXITCODE
$sw.Stop()
"exit=$noopCode wall_ms=$($sw.ElapsedMilliseconds)"
if ($noopCode -ne 0) { throw "the baseline refresh failed with exit $noopCode" }
[System.IO.File]::WriteAllText((Join-Path $out 'noop.json'), $noop, (New-Object System.Text.UTF8Encoding($false)))
$noop
$noopJson = $noop | ConvertFrom-Json
if ($noopJson.mode -ne 'incremental') { throw "the baseline refresh was not incremental (mode=$($noopJson.mode))" }
if ($noopJson.status_at_write -ne 'fresh') { throw "the baseline index is not fresh (status=$($noopJson.status_at_write))" }
# The CP1 A.7 contract wants a NO-OP refresh. If the home was out of date
# (for example after an interrupted run), the first refresh converges it and
# the next one must be the no-op; either way the verified baseline is the
# last one.
if ($noopJson.reparsed -ne 0) {
  "  the home was out of date ($($noopJson.reparsed) file(s)); converging and verifying"
  $noop = & $cfg.Bin index --repo mmsp-pin 2>&1 | Out-String
  if ($LASTEXITCODE -ne 0) { throw "the baseline verification refresh failed with exit $LASTEXITCODE" }
  [System.IO.File]::WriteAllText((Join-Path $out 'noop.json'), $noop, (New-Object System.Text.UTF8Encoding($false)))
  $noop
  $noopJson = $noop | ConvertFrom-Json
}
if ($noopJson.reparsed -ne 0) { throw "the baseline refresh is not a no-op (reparsed=$($noopJson.reparsed))" }
"  baseline: mode=$($noopJson.mode) reused=$($noopJson.reused) reparsed=$($noopJson.reparsed) status=$($noopJson.status_at_write)"

# --- 3. change exactly one file ---
"===== 3. one-file change ====="
$p = Join-Path $snap $changedFile
(Get-Content $p -Raw) -replace 'double max_triangulation_angle = -1\.0f;', 'double max_triangulation_angle = 60.0f;' | Set-Content -NoNewline $p
# The change must really exist: a pattern that no longer matches would make
# the delta a second no-op, and the parity below would then prove nothing.
$porcelain = @(git -C $snap status --porcelain)
"  changed lines: $($porcelain.Count)"
if ($porcelain.Count -ne 1) { throw "the one-file change touched $($porcelain.Count) entries, expected 1" }
$fileDigest = (Get-FileHash $p -Algorithm SHA256).Hash
"  $changedFile sha256=$fileDigest"

# --- 4. incremental indexing on the changed tree ---
"===== 4. incremental indexing on the changed tree ====="
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$delta = & $cfg.Bin index --repo mmsp-pin 2>&1 | Out-String
$deltaCode = $LASTEXITCODE
$sw.Stop()
"exit=$deltaCode wall_ms=$($sw.ElapsedMilliseconds)"
if ($deltaCode -ne 0) { throw "the incremental run failed with exit $deltaCode" }
[System.IO.File]::WriteAllText((Join-Path $out 'delta.json'), $delta, (New-Object System.Text.UTF8Encoding($false)))
$delta
$deltaJson = $delta | ConvertFrom-Json
if ($deltaJson.reparsed -ne 1) { throw "the incremental run re-parsed $($deltaJson.reparsed) files instead of 1" }
"  reparsed=$($deltaJson.reparsed) reused=$($deltaJson.reused) mode=$($deltaJson.mode)"

# --- 5. an independent full rebuild on the SAME changed tree ---
"===== 5. oracle: independent full rebuild, fresh home, same changed tree ====="
$oracleHome = Reset-BenchDir (Join-Path $cfg.BenchRoot 'home-oracle') -Root $cfg.BenchRoot
$env:RILTRA_HOME = $oracleHome
& $cfg.Bin init --repo $snap --name mmsp-pin | Out-Null
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$oracle = & $cfg.Bin index --repo mmsp-pin --full 2>&1 | Out-String
$oracleCode = $LASTEXITCODE
$sw.Stop()
"exit=$oracleCode wall_ms=$($sw.ElapsedMilliseconds)"
if ($oracleCode -ne 0) { throw "the oracle rebuild failed with exit $oracleCode" }
[System.IO.File]::WriteAllText((Join-Path $out 'oracle.json'), $oracle, (New-Object System.Text.UTF8Encoding($false)))
$oracle
"oracle db bytes = $((Get-ChildItem $oracleHome -Recurse -File -Filter *.db | Measure-Object -Property Length -Sum).Sum)"

# --- 6. the fact digests of BOTH indexes, BEFORE the restore ---
"===== 6. fact digests of both indexes (same changed tree) ====="
$incDb = Join-Path $HomeDir 'index\mmsp-pin.db'
$orcDb = Join-Path $oracleHome 'index\mmsp-pin.db'
$incFacts = Join-Path $out 'delta-inc-facts.json'
$orcFacts = Join-Path $out 'delta-oracle-facts.json'
node $digest $incDb $incFacts | Out-Null
if ($LASTEXITCODE -ne 0) { throw "the incremental fact digest failed with exit $LASTEXITCODE" }
node $digest $orcDb $orcFacts | Out-Null
if ($LASTEXITCODE -ne 0) { throw "the oracle fact digest failed with exit $LASTEXITCODE" }
$inc = Get-Content $incFacts -Raw | ConvertFrom-Json
$orc = Get-Content $orcFacts -Raw | ConvertFrom-Json
foreach ($t in 'files','symbols','imports','calls','edges') {
  "  {0,-9} rows {1,7} vs {2,7}  identical = {3}" -f $t, $inc.tables.$t.rows, $orc.tables.$t.rows, ($inc.tables.$t.digest -eq $orc.tables.$t.digest)
}
"  incremental combined = $($inc.combined_digest)"
"  oracle      combined = $($orc.combined_digest)"
# The comparison itself fails loudly: a difference, an empty digest, a digest
# that could not be computed or a source tree that does not match is a FAIL.
node $compare compare-digests $incFacts $orcFacts
if ($LASTEXITCODE -ne 0) { throw "FACT PARITY on the changed tree: DIFFERENT (exit $LASTEXITCODE)" }

# the source tree identity, so the pair can be tied to one tree
$tree = [ordered]@{
  snapshot_head = (git -C $snap rev-parse HEAD)
  snapshot_porcelain = $porcelain
  changed_file = $changedFile
  changed_file_sha256 = $fileDigest
  incremental_index_head = $inc.snapshot.head
  oracle_index_head = $orc.snapshot.head
  incremental_index_dirty = $inc.snapshot.dirty
  oracle_index_dirty = $orc.snapshot.dirty
  incremental_combined_digest = $inc.combined_digest
  oracle_combined_digest = $orc.combined_digest
}
[System.IO.File]::WriteAllText((Join-Path $out 'delta-tree.json'), ($tree | ConvertTo-Json -Depth 4), (New-Object System.Text.UTF8Encoding($false)))
"  tree identity archived"

# --- 7. the query parity on the same state ---
"===== 7. query parity on the same changed tree ====="
$q = @(
  @('search','--repo','mmsp-pin','--symbol','PatchMatch'),
  @('search','--repo','mmsp-pin','--symbol','Run'),
  @('search','--repo','mmsp-pin','--symbol','normalize_vec'),
  @('search','--repo','mmsp-pin','--symbol','max_triangulation_angle'),
  @('callers','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.cc','--line','118','--col','27'),
  @('callees','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.cc','--line','191','--col','18'),
  @('callees','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.cc','--line','233','--col','28'),
  @('callers','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.cc','--line','54','--col','28'),
  @('callers','--repo','mmsp-pin','--path','colmap/benchmark/reconstruction/evaluation/geometry.py','--line','34','--col','5'),
  @('trace','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.cc','--line','233','--col','28','--depth','4'),
  @('impact','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match_options.h','--line','56','--col','10'),
  @('index','status','--repo','mmsp-pin')
)
$parity = @()
$parityFailures = @()
foreach ($qq in $q) {
  $name = ($qq -join '_') -replace '[\\/:]','_'
  $env:RILTRA_HOME = $HomeDir
  $rawA = & $cfg.Bin @($qq) 2>&1 | Out-String
  $codeA = $LASTEXITCODE
  $env:RILTRA_HOME = $oracleHome
  $rawB = & $cfg.Bin @($qq) 2>&1 | Out-String
  $codeB = $LASTEXITCODE
  $fileA = Join-Path $out "parity-$name.inc.txt"
  $fileB = Join-Path $out "parity-$name.oracle.txt"
  [System.IO.File]::WriteAllText($fileA, $rawA, (New-Object System.Text.UTF8Encoding($false)))
  [System.IO.File]::WriteAllText($fileB, $rawB, (New-Object System.Text.UTF8Encoding($false)))
  # The comparison is the tested one: only the verified volatile snapshot
  # metadata is normalized, and an error or an empty output never matches.
  $verdict = node $compare compare-outputs $fileA $fileB $codeA $codeB
  $same = ($LASTEXITCODE -eq 0)
  if (-not $same) { $parityFailures += "$name ($verdict)" }
  $parity += [pscustomobject]@{ query = ($qq -join ' '); identical = $same; exit_inc = $codeA; exit_oracle = $codeB }
  "  identical=$same  $name"
}
$parity | ConvertTo-Json | Set-Content (Join-Path $out '_parity-summary.json') -Encoding utf8

# --- 8. only now: restore the snapshot and verify it ---
"===== 8. restore the snapshot to the pin ====="
Restore-Snapshot -Snapshot $snap -Root $cfg.BenchRoot -Pin $Pin -MmspRoot $cfg.MmspRoot | Out-Null
$restoredHead = git -C $snap rev-parse HEAD
$restoredPorcelain = (git -C $snap status --porcelain | Measure-Object -Line).Lines
"  snapshot HEAD = $restoredHead  porcelain = $restoredPorcelain line(s)"
if ($restoredHead -ne $Pin -or $restoredPorcelain -ne 0) {
  throw "the snapshot was not restored (HEAD=$restoredHead porcelain=$restoredPorcelain)"
}
"===== re-index the main home back to the pinned tree ====="
$env:RILTRA_HOME = $HomeDir
$re = & $cfg.Bin index --repo mmsp-pin 2>&1 | Out-String
if ($LASTEXITCODE -ne 0) { throw "the restore re-index failed with exit $LASTEXITCODE" }
[System.IO.File]::WriteAllText((Join-Path $out 'restore-reindex.json'), $re, (New-Object System.Text.UTF8Encoding($false)))
$re
if ($cfg.MmspRoot) {
  "original MMSP: HEAD = $(git -C $cfg.MmspRoot rev-parse HEAD) porcelain = $(git -C $cfg.MmspRoot status --porcelain | Measure-Object -Line | ForEach-Object { $_.Lines }) lines"
}

if ($parityFailures.Count -gt 0) {
  throw "parity could not be established: $($parityFailures -join '; ')"
}
"PARITY: the incremental index and the independent full rebuild of the SAME changed tree have identical facts and identical query answers"
