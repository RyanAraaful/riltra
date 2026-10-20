# CP2 step 2: two independent cold-index measurements (D-M9-3: >= 2 runs).
$ErrorActionPreference = 'Stop'
. "$env:TEMP\opencode\bench\cp2\common.ps1"

$bin = '<RILTRA_ROOT>\target\debug\riltra.exe'
$snap = "$env:TEMP\opencode\bench\mmsp-d62f06a"

foreach ($n in 1, 2) {
  $homeDir = Reset-BenchDir "$env:TEMP\opencode\bench\home-cold$n"
  $env:RILTRA_HOME = $homeDir

  "===== cold run $n : init ====="
  & $bin init --repo $snap --name mmsp-pin
  "init exit=$LASTEXITCODE"

  "===== cold run $n : index --full ====="
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  & $bin index --repo mmsp-pin --full
  $code = $LASTEXITCODE
  $sw.Stop()
  "index exit=$code wall_s=$([math]::Round($sw.Elapsed.TotalSeconds,3))"

  "===== cold run $n : index size ====="
  $dbFiles = Get-ChildItem $homeDir -Recurse -File -Filter *.db -ErrorAction SilentlyContinue
  $dbFiles | ForEach-Object { "  db=$($_.Name) bytes=$($_.Length)" }
  $all = Get-ChildItem $homeDir -Recurse -File
  "  home_total_bytes=$(($all | Measure-Object -Property Length -Sum).Sum) files=$($all.Count)"

  "===== cold run $n : repo status ====="
  & $bin repo status --repo mmsp-pin
  "===== cold run $n : index status ====="
  & $bin index status --repo mmsp-pin
}
