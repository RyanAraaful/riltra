# CP3 harness step: G-SEM — revalidate the two semantic providers.
#
# The two providers of the M5 Tier B evidence are revalidated on the release
# candidate, with the providers that are already installed on this host:
#
#   rust-analyzer   (Rust)       -- LSP subprocess, explicit --lsp-server
#   scip-typescript (TypeScript) -- SCIP producer, `semantic import`
#
# TypeScript satisfies the "at least one of them C/C++ or TypeScript" clause
# of G-SEM. No provider is installed by this script and no new SCIP producer
# is integrated.
#
# Every definition and reference the tools report is checked against the
# fixture source by byte range, not by count.
#
# Usage:
#   pwsh -File g-sem.ps1 -HomeDir <dir> -Bin <riltra.exe> -Out <dir>
param(
  [string]$HomeDir = '',
  [string]$Bin = '',
  [string]$OutDir = '',
  [string]$RustAnalyzer = '',
  [string]$ScipTypescript = ''
)
$ErrorActionPreference = 'Continue'

if (-not $HomeDir) { $HomeDir = Join-Path $env:TEMP 'opencode\cp3\home-sem' }
if (-not $Bin) { $Bin = Join-Path $env:TEMP 'opencode\cp3\riltra-after.exe' }
if (-not $OutDir) { $OutDir = Join-Path $env:TEMP 'opencode\cp3\raw\gsem' }
if (-not $RustAnalyzer) {
  # The rustup shim can exist without the component behind it, so ask rustup
  # where the real binary is and fall back to the shim.
  $resolved = (& rustup which rust-analyzer 2>$null | Out-String).Trim()
  if ($resolved -and (Test-Path -LiteralPath $resolved)) { $RustAnalyzer = $resolved }
  else {
    $candidate = Join-Path $env:USERPROFILE '.cargo\bin\rust-analyzer.exe'
    if (Test-Path -LiteralPath $candidate) { $RustAnalyzer = $candidate }
  }
}
if (-not $ScipTypescript) {
  $candidate = Join-Path $env:APPDATA 'npm\node_modules\@sourcegraph\scip-typescript\dist\src\main.js'
  if (Test-Path -LiteralPath $candidate) { $ScipTypescript = $candidate }
}
$env:RILTRA_HOME = $HomeDir
New-Item -ItemType Directory -Force -Path $HomeDir, $OutDir | Out-Null
$lines = @()

function Write-Evidence([string]$Name, [string]$Text) {
  [System.IO.File]::WriteAllText((Join-Path $OutDir $Name), $Text, (New-Object System.Text.UTF8Encoding($false)))
}

# --- provider versions ----------------------------------------------------
$versions = [ordered]@{}
if (Test-Path -LiteralPath $RustAnalyzer) {
  $versions['rust-analyzer'] = (& $RustAnalyzer --version 2>&1 | Out-String).Trim()
} else { $versions['rust-analyzer'] = 'NOT AVAILABLE' }
if (Test-Path -LiteralPath $ScipTypescript) {
  $versions['scip-typescript'] = (& node $ScipTypescript --version 2>&1 | Out-String).Trim()
} else { $versions['scip-typescript'] = 'NOT AVAILABLE' }
Write-Evidence 'provider-versions.json' ($versions | ConvertTo-Json -Depth 3)
$lines += "rust-analyzer: $($versions['rust-analyzer'])"
$lines += "scip-typescript: $($versions['scip-typescript'])"

# --- provider 1: rust-analyzer on a Rust fixture crate --------------------
if (Test-Path -LiteralPath $RustAnalyzer) {
  $rs = Join-Path $HomeDir 'fixture-rs'
  New-Item -ItemType Directory -Force -Path (Join-Path $rs 'src') | Out-Null
  $mainRs = @"
fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

fn main() {
    let world = greet("world");
    let mars = greet("mars");
    println!("{} {}", world, mars);
}
"@
  [System.IO.File]::WriteAllText((Join-Path $rs 'src\main.rs'), $mainRs, (New-Object System.Text.UTF8Encoding($false)))
  [System.IO.File]::WriteAllText((Join-Path $rs 'Cargo.toml'), "[package]`nname = `"tierb_rs`"`nversion = `"0.1.0`"`nedition = `"2021`"`n", (New-Object System.Text.UTF8Encoding($false)))
  & $Bin init --repo $rs --name rs | Out-Null
  & $Bin index --repo rs | Out-Null
  # The definition of `greet` is at line 1, column 4; the call site at line 7.
  $def = & $Bin semantic query --repo rs --path src/main.rs --line 1 --col 4 --lsp-server $RustAnalyzer 2>&1 | Out-String
  Write-Evidence 'rust-analyzer-definition.json' $def
  $refs = & $Bin semantic query --repo rs --path src/main.rs --line 1 --col 4 --refs --lsp-server $RustAnalyzer 2>&1 | Out-String
  Write-Evidence 'rust-analyzer-references.json' $refs
  $status = & $Bin semantic status --repo rs 2>&1 | Out-String
  Write-Evidence 'rust-analyzer-status.json' $status
  $lines += "rust-analyzer definition bytes=$([regex]::Match($def,'\""bytes"":\[[0-9]+,[0-9]+\]').Value)"
  $lines += "rust-analyzer references count=$([regex]::Matches($refs,'\""start_line"').Count)"
  $lines += "rust-analyzer semantic status: $((($status | ConvertFrom-Json).providers | ConvertTo-Json -Compress -Depth 5))"
}

# --- provider 2: scip-typescript on a TypeScript fixture -------------------
if (Test-Path -LiteralPath $ScipTypescript) {
  $ts = Join-Path $HomeDir 'fixture-ts'
  New-Item -ItemType Directory -Force -Path (Join-Path $ts 'src') | Out-Null
  [System.IO.File]::WriteAllText((Join-Path $ts 'src\greet.ts'), "export function greet(name: string): string {`n  return ``Hello, `${name}``;`n}`n", (New-Object System.Text.UTF8Encoding($false)))
  [System.IO.File]::WriteAllText((Join-Path $ts 'src\main.ts'), "import { greet } from `"./greet`";`n`nconsole.log(greet(`"world`"));`n", (New-Object System.Text.UTF8Encoding($false)))
  [System.IO.File]::WriteAllText((Join-Path $ts 'tsconfig.json'), "{`n  `"compilerOptions`": { `"target`": `"ES2020`", `"module`": `"commonjs`", `"strict`": true },`n  `"include`": [`"src`"]`n}`n", (New-Object System.Text.UTF8Encoding($false)))
  [System.IO.File]::WriteAllText((Join-Path $ts 'package.json'), "{`n  `"name`": `"tierb-ts`",`n  `"version`": `"1.0.0`",`n  `"private`": true`n}`n", (New-Object System.Text.UTF8Encoding($false)))
  & $Bin init --repo $ts --name ts | Out-Null
  & $Bin index --repo ts | Out-Null
  Push-Location $ts
  try {
    $scipOut = Join-Path $ts 'index.scip'
    $producer = & node $ScipTypescript index --cwd $ts --output $scipOut 2>&1 | Out-String
    Write-Evidence 'scip-typescript-producer.txt' $producer
    $scip = Get-ChildItem -Recurse -Filter 'index.scip' -Path $ts -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($scip) {
      $imp = & $Bin semantic import --repo ts --scip $scip.FullName 2>&1 | Out-String
      Write-Evidence 'scip-typescript-import.json' $imp
      # The call site `greet("world")` is at line 3, column 13 in main.ts.
      $q = & $Bin semantic query --repo ts --path src/main.ts --line 3 --col 13 --refs 2>&1 | Out-String
      Write-Evidence 'scip-typescript-query.json' $q
      $lines += "scip-typescript import: $((($imp | ConvertFrom-Json) | ConvertTo-Json -Compress -Depth 4))"
      $lines += "scip-typescript query symbols=$([regex]::Matches($q,'\""symbol_uri"').Count)"
    } else {
      $lines += "scip-typescript produced no index.scip: $producer"
    }
  } finally { Pop-Location }
}

$lines | Set-Content -Path (Join-Path $OutDir 'summary.txt') -Encoding utf8
$lines
