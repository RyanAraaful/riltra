$ErrorActionPreference = 'Continue'
$root = '<EVIDENCE_ROOT>'
$src = "$root\src"; $bin = "$root\bin\riltra.exe"; $bench = "$root\bench"
$snap = "$bench\mmsp-d62f06a"; $orig = '<ORIGINAL_MMSP>'; $ev = "$root\evidence"
$cp2 = "$bench\cp2\raw"; $m = "$src\docs\pilots\cp2\method"
$ra = (Get-Command rust-analyzer).Source
$tsMain = Join-Path $env:APPDATA 'npm\node_modules\@sourcegraph\scip-typescript\dist\src\main.js'
$sum = "$ev\summary.txt"
"toolchain: $(rustc -V) | $(cargo -V) | node $(node -v) | $(git --version) | pwsh $($PSVersionTable.PSVersion)" | Set-Content -LiteralPath $sum
"rust-analyzer: $((& $ra --version | Out-String).Trim()) at $ra" | Add-Content -LiteralPath $sum
"binary sha256: $((Get-FileHash -Algorithm SHA256 -LiteralPath $bin).Hash)" | Add-Content -LiteralPath $sum
function Mark([string]$name, [int]$code) { "$name exit=$code at=$(Get-Date -Format o)" | Add-Content -LiteralPath $sum }

pwsh -NoProfile -File "$src\docs\pilots\cp3\method\g-sem.ps1" -HomeDir "$root\gsem\home" -Bin $bin -OutDir "$root\gsem\raw" -RustAnalyzer $ra -ScipTypescript $tsMain *> "$ev\g-sem.log"
Mark 'g-sem' $LASTEXITCODE
$env:RILTRA_HOME = "$root\gsem\home"
& $bin semantic query --repo ts --path src/main.ts --line 3 --col 13 > "$root\gsem\raw\ts-definition-from-callsite.json" 2> "$ev\ts-definition.err"
Mark 'ts-definition-query' $LASTEXITCODE
& $bin semantic status --repo rs > "$root\gsem\raw\status-rs.json" 2> "$ev\status-rs.err"
Mark 'status-rs' $LASTEXITCODE
& $bin semantic status --repo ts > "$root\gsem\raw\status-ts.json" 2> "$ev\status-ts.err"
Mark 'status-ts' $LASTEXITCODE

Push-Location $m
node --test "$m\fact-digest.test.cjs" *> "$ev\selftest-fact-digest.log"
Mark 'selftest-fact-digest' $LASTEXITCODE
node --test "$m\grade.test.cjs" *> "$ev\selftest-grade.log"
Mark 'selftest-grade' $LASTEXITCODE
pwsh -NoProfile -File "$m\test-harness.ps1" *> "$ev\selftest-harness.log"
Mark 'selftest-harness' $LASTEXITCODE
pwsh -NoProfile -File "$m\test-path-safety.ps1" *> "$ev\selftest-path-safety.log"
Mark 'selftest-path-safety' $LASTEXITCODE
Pop-Location

pwsh -NoProfile -File "$m\cold-index.ps1" -BenchRoot $bench -Bin $bin -Snapshot $snap -Out $cp2 *> "$ev\cold-index.log"
Mark 'cold-index' $LASTEXITCODE
pwsh -NoProfile -File "$ev\step2-home-main.ps1" -Bench $bench -Bin $bin -Src $src *> "$ev\home-main.log"
Mark 'home-main' $LASTEXITCODE
pwsh -NoProfile -File "$m\freshness.ps1" -BenchRoot $bench -Bin $bin -Snapshot $snap -MmspRoot $orig -Out $cp2 -HomeDir "$bench\home-main" *> "$ev\freshness.log"
Mark 'freshness' $LASTEXITCODE
pwsh -NoProfile -File "$m\incremental.ps1" -BenchRoot $bench -Bin $bin -Snapshot $snap -MmspRoot $orig -Out $cp2 -HomeDir "$bench\home-main" *> "$ev\incremental.log"
Mark 'incremental' $LASTEXITCODE

"snapshot HEAD=$(git -C $snap rev-parse HEAD) tree=$(git -C $snap rev-parse 'HEAD^{tree}') porcelain=$((git -C $snap status --porcelain | Measure-Object -Line).Lines)" | Set-Content -LiteralPath "$ev\isolation.txt"
"original HEAD=$(git -C $orig rev-parse HEAD) porcelain=$((git -C $orig status --porcelain | Measure-Object -Line).Lines)" | Add-Content -LiteralPath "$ev\isolation.txt"
"DONE $(Get-Date -Format o)" | Add-Content -LiteralPath $sum
