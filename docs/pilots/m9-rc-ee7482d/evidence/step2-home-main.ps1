param([string]$Bench, [string]$Bin, [string]$Src)
. (Join-Path $Src 'docs\pilots\cp2\method\common.ps1')
$homeMain = Reset-BenchDir (Join-Path $Bench 'home-main') -Root $Bench
Copy-BenchDir -Source (Join-Path $Bench 'home-cold1') -Destination $homeMain -Root $Bench | Out-Null
$env:RILTRA_HOME = $homeMain
& $Bin index status --repo mmsp-pin
if ($LASTEXITCODE -ne 0) { throw "the copied home does not answer: index status failed with exit $LASTEXITCODE" }
