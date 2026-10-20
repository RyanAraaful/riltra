# CP3 harness step: the multi-repository isolation matrix.
#
# ONE isolated RILTRA_HOME holds exactly two registered repositories:
#   mmsp-pin    MMSP at the frozen pin d62f06a
#   riltra-self Riltra self-indexed at the frozen snapshot fca1185
# Both are separate, isolated clones. The original repositories are never
# touched.
#
# Every case writes its command output, exit code and timing to the archive
# and records a verdict, so the matrix can be audited case by case.
#
# Usage:
#   pwsh -File multirepo.ps1 -HomeDir <dir> -BenchRoot <dir> -Bin <riltra.exe> \
#         -MmspClone <dir> -RiltraClone <dir> -Out <dir>
param(
  [string]$HomeDir = '',
  [string]$BenchRoot = '',
  [string]$Bin = '',
  [string]$MmspClone = '',
  [string]$RiltraClone = '',
  [string]$OutDir = '',
  [string]$MmspPin = 'd62f06a576d8e76b705c8ecc57fc380f7ebc9c44',
  [string]$RiltraPin = 'fca1185678de612552ad24a7932bb3f09f175507'
)
$ErrorActionPreference = 'Continue'
. (Join-Path $PSScriptRoot '..\..\cp2\method\common.ps1')

if (-not $HomeDir) { $HomeDir = Join-Path $env:TEMP 'opencode\cp3\home' }
if (-not $BenchRoot) { $BenchRoot = Join-Path $env:TEMP 'opencode\cp3' }
if (-not $Bin) { $Bin = Join-Path $BenchRoot 'riltra-after.exe' }
if (-not $MmspClone) { $MmspClone = Join-Path $BenchRoot 'clones\mmsp' }
if (-not $RiltraClone) { $RiltraClone = Join-Path $BenchRoot 'clones\riltra' }
if (-not $OutDir) { $OutDir = Join-Path $BenchRoot 'raw\multirepo' }
foreach ($p in @($HomeDir, $BenchRoot, $Bin, $MmspClone, $RiltraClone)) {
  if (-not (Test-Path -LiteralPath $p)) { throw "missing path: $p" }
}
$env:RILTRA_HOME = $HomeDir
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

$script:cases = @()
function Case {
  param([string]$Id, [string]$What, [scriptblock]$Run)
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  $result = & $Run
  $sw.Stop()
  $script:cases += [pscustomobject]@{
    id = $Id; what = $What; ms = $sw.ElapsedMilliseconds; result = @($result)
  }
  "== $Id  $($sw.ElapsedMilliseconds) ms  $What"
  @($result) | ForEach-Object { "   $_" }
}

function Run-Cmd {
  param([string[]]$CmdArgs, [string]$Name)
  $text = & $Bin @CmdArgs 2>&1 | Out-String
  $code = $LASTEXITCODE
  $file = Join-Path $OutDir ($Name + '.json')
  [System.IO.File]::WriteAllText($file, $text, (New-Object System.Text.UTF8Encoding($false)))
  return [pscustomobject]@{ exit = $code; text = $text; file = $Name }
}

# --- C1: both repositories register, each at its own pin ------------------
Case 'C1-register' 'both repositories register with their own root' {
  $r = Run-Cmd @('repo','list') 'c1-repo-list'
  $v = $r.text | ConvertFrom-Json
  $names = @($v.repos | ForEach-Object { $_.name })
  $roots = @($v.repos | ForEach-Object { "$($_.name)=$($_.root)" })
  "exit=$($r.exit) repos=$($names -join ',')"
  $roots
}

Case 'C2-index-status' 'each repository has its own fresh index' {
  $a = Run-Cmd @('index','status','--repo','mmsp-pin') 'c2-mmsp-status'
  $b = Run-Cmd @('index','status','--repo','riltra-self') 'c2-riltra-status'
  $ja = $a.text | ConvertFrom-Json
  $jb = $b.text | ConvertFrom-Json
  "mmsp-pin freshness=$($ja.freshness) head=$($ja.snapshot.head) reasons=$($ja.reasons -join ';')"
  "riltra-self status=$($jb.freshness) head=$($jb.snapshot.head) reasons=$($jb.reasons -join ';')"
}

# --- C3: a query targets the selected repository ---------------------------
# "main" exists in BOTH repositories (30 MMSP paths, 10 Riltra paths).
Case 'C3-query-targeting' 'the same symbol name resolves only inside the selected repository' {
  $a = Run-Cmd @('search','--repo','mmsp-pin','--symbol','main','--limit','200') 'c3-search-mmsp'
  $b = Run-Cmd @('search','--repo','riltra-self','--symbol','main','--limit','200') 'c3-search-riltra'
  $ja = $a.text | ConvertFrom-Json
  $jb = $b.text | ConvertFrom-Json
  $pa = @($ja.results | ForEach-Object { $_.path })
  $pb = @($jb.results | ForEach-Object { $_.path })
  $leakA = @($pa | Where-Object { $_ -like 'crates/*' -or $_ -like 'docs/*' })
  $leakB = @($pb | Where-Object { $_ -like 'colmap/*' })
  "mmsp-pin hits=$($pa.Count) riltra-self hits=$($pb.Count)"
  "mmsp-pin paths belonging to the other repository: $($leakA.Count)"
  "riltra-self paths belonging to the other repository: $($leakB.Count)"
}

# --- C4: the graph directions never cross the repository boundary ----------
Case 'C4-graph-boundary' 'callers, callees, trace and impact stay inside the selected repository' {
  $lines = @()
  $graphCases = @(
    @{ n = 'c4-callers-mmsp';   a = @('callers','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.h','--line','136','--col','8') },
    @{ n = 'c4-callees-mmsp';   a = @('callees','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.cc','--line','191','--col','18') },
    @{ n = 'c4-trace-mmsp';     a = @('trace','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match.cc','--line','233','--col','28','--depth','4') },
    @{ n = 'c4-impact-mmsp';    a = @('impact','--repo','mmsp-pin','--path','colmap/src/colmap/mvs/patch_match_options.h','--line','56','--col','10') },
    @{ n = 'c4-callers-riltra'; a = @('callers','--repo','riltra-self','--path','crates/cli/src/main.rs','--line','219','--col','1') },
    @{ n = 'c4-impact-riltra';  a = @('impact','--repo','riltra-self','--path','crates/cli/src/main.rs','--line','219','--col','1') }
  )
  foreach ($c in $graphCases) {
    $r = Run-Cmd $c.a $c.n
    $foreign = if ($c.n -like '*mmsp') { @('crates/','docs/') } else { @('colmap/') }
    $leak = 0
    foreach ($f in $foreign) { $leak += ([regex]::Matches($r.text, [regex]::Escape($f))).Count }
    $lines += "$($c.n) exit=$($r.exit) foreignPathMentions=$leak"
  }
  $lines
}

# --- C5: freshness is independent per repository ---------------------------
Case 'C5-freshness-independence' 'a change in one repository does not affect the other' {
  $lines = @()
  $before = (Run-Cmd @('index','status','--repo','mmsp-pin') 'c5-mmsp-before').text | ConvertFrom-Json
  $otherBefore = (Run-Cmd @('index','status','--repo','riltra-self') 'c5-riltra-before').text | ConvertFrom-Json
  $lines += "before: mmsp-pin=$($before.freshness) riltra-self=$($otherBefore.freshness)"
  # Mutate the isolated MMSP clone only: an indexed source file, so the
  # change is one the index can actually see.
  $marker = Join-Path $MmspClone 'colmap\src\colmap\mvs\patch_match.cc'
  Add-Content -LiteralPath $marker -Value "`n// cp3 freshness probe" -Encoding utf8
  $mmspAfter = (Run-Cmd @('index','status','--repo','mmsp-pin') 'c5-mmsp-after').text | ConvertFrom-Json
  $riltraAfter = (Run-Cmd @('index','status','--repo','riltra-self') 'c5-riltra-after').text | ConvertFrom-Json
  $lines += "after:  mmsp-pin=$($mmspAfter.freshness) ($($mmspAfter.reasons -join ';')) riltra-self=$($riltraAfter.freshness)"
  # Restore the pinned tree exactly.
  git -C $MmspClone checkout --quiet --detach $MmspPin | Out-Null
  git -C $MmspClone reset --quiet --hard $MmspPin | Out-Null
  git -C $MmspClone clean -qfd | Out-Null
  $lines += "restored head=$(git -C $MmspClone rev-parse HEAD) dirtyLines=$(@(git -C $MmspClone status --porcelain).Count)"
  # Rebuild so the index matches the restored tree again.
  $rb = Run-Cmd @('index','--repo','mmsp-pin') 'c5-mmsp-reindex'
  $mmspRestored = (Run-Cmd @('index','status','--repo','mmsp-pin') 'c5-mmsp-restored').text | ConvertFrom-Json
  $riltraRestored = (Run-Cmd @('index','status','--repo','riltra-self') 'c5-riltra-restored').text | ConvertFrom-Json
  $lines += "restored: mmsp-pin=$($mmspRestored.freshness) riltra-self=$($riltraRestored.freshness) reindexExit=$($rb.exit)"
  $lines
}

# --- C6: generation, snapshot and citation belong to one repository --------
Case 'C6-snapshot-ownership' 'every answer carries its own repository snapshot' {
  $lines = @()
  foreach ($repo in @('mmsp-pin','riltra-self')) {
    $r = Run-Cmd @('search','--repo',$repo,'--symbol','main','--limit','5') "c6-$repo"
    $j = $r.text | ConvertFrom-Json
    $lines += "$repo repo=$($j.repo) head=$($j.snapshot.head) snapshotId=$($j.snapshot.id) results=$(@($j.results).Count)"
  }
  $lines
}

# --- C7: a read-only MCP session creates and migrates no store -------------
Case 'C7-mcp-read-only-purity' 'read-only MCP tools create or migrate no store' {
  $lines = @()
  $empty = Join-Path $BenchRoot 'home-mcp-empty'
  if (Test-Path -LiteralPath $empty) { Remove-Item -LiteralPath $empty -Recurse -Force }
  New-Item -ItemType Directory -Force -Path $empty | Out-Null
  $probe = Join-Path $OutDir 'c7-mcp-empty-home'
  $env:CP2_RILTRA = $Bin
  $env:CP2_HOME = $empty
  $env:CP2_OUT = $probe
  $env:CP2_REPO = 'mmsp-pin'
  node (Join-Path $PSScriptRoot '..\..\cp2\method\mcp-smoke.cjs') 2>&1 | Out-Null
  $entries = @(Get-ChildItem -Recurse -Force -LiteralPath $empty -ErrorAction SilentlyContinue)
  $lines += "empty home entries after a read-only MCP session: $($entries.Count)"
  foreach ($e in $entries) { $lines += "  $($e.Name)" }
  # The same session against the populated home must not change it either.
  $populatedBefore = @(Get-ChildItem -Recurse -Force -LiteralPath $HomeDir -ErrorAction SilentlyContinue)
  $env:CP2_HOME = $HomeDir
  $env:CP2_OUT = (Join-Path $OutDir 'c7-mcp-populated-home')
  node (Join-Path $PSScriptRoot '..\..\cp2\method\mcp-smoke.cjs') 2>&1 | Out-Null
  $populatedAfter = @(Get-ChildItem -Recurse -Force -LiteralPath $HomeDir -ErrorAction SilentlyContinue)
  $lines += "populated home entries before=$($populatedBefore.Count) after=$($populatedAfter.Count)"
  $lines
}

# --- C8: a junction or symlink escape stays refused -----------------------
Case 'C8-escape-refused' 'a link that leaves the repository root is not followed' {
  $lines = @()
  $outsideDir = Join-Path $BenchRoot 'outside'
  New-Item -ItemType Directory -Force -Path $outsideDir | Out-Null
  Set-Content -LiteralPath (Join-Path $outsideDir 'secret.h') -Value 'int Secret();' -Encoding utf8
  $link = Join-Path $MmspClone 'colmap\src\colmap\cp3-escape'
  if (Test-Path -LiteralPath $link) { Remove-Item -LiteralPath $link -Recurse -Force }
  $made = $false
  try {
    New-Item -ItemType Junction -Path $link -Target $outsideDir -ErrorAction Stop | Out-Null
    $made = $true
  } catch {
    try {
      New-Item -ItemType SymbolicLink -Path $link -Target $outsideDir -ErrorAction Stop | Out-Null
      $made = $true
    } catch { $lines += "could not create the link: $($_.Exception.Message)" }
  }
  $lines += "link created: $made"
  if ($made) {
    $r = Run-Cmd @('scan','--repo','mmsp-pin','--path','colmap/src/colmap/cp3-escape/secret.h') 'c8-scan-escape'
    $lines += "scan exit=$($r.exit) refused=$($r.text -match 'escapes repository root')"
    $r2 = Run-Cmd @('index','--repo','mmsp-pin') 'c8-index-escape'
    $lines += "index exit=$($r2.exit)"
    $r3 = Run-Cmd @('search','--repo','mmsp-pin','--symbol','Secret') 'c8-search-secret'
    $j3 = $r3.text | ConvertFrom-Json
    $lines += "search exit=$($r3.exit) total=$($j3.total) results=$(@($j3.results).Count)"
    $r4 = Run-Cmd @('doctor','--repo','mmsp-pin') 'c8-doctor'
    $lines += "doctor exit=$($r4.exit)"
  }
  # Remove the link again and restore the pinned tree.
  if ($made) { Remove-Item -LiteralPath $link -Recurse -Force -ErrorAction SilentlyContinue }
  git -C $MmspClone checkout --quiet --detach $MmspPin | Out-Null
  git -C $MmspClone reset --quiet --hard $MmspPin | Out-Null
  git -C $MmspClone clean -qfd | Out-Null
  $rb = Run-Cmd @('index','--repo','mmsp-pin') 'c8-reindex'
  $lines += "restored head=$(git -C $MmspClone rev-parse HEAD) reindexExit=$($rb.exit)"
  $lines
}

$script:cases | ConvertTo-Json -Depth 6 | Set-Content -Path (Join-Path $OutDir 'cases.json') -Encoding utf8
"===== cases ====="
$script:cases | Format-Table -AutoSize | Out-String
