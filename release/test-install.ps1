# SPDX-License-Identifier: Apache-2.0
#
# Phase A acceptance tests for release/install.ps1.
#
# Usage: pwsh -NoProfile -File release/test-install.ps1 -ArchivePath <zip>
#
# The archive and its adjacent .sha256 must come from release/package.sh. The
# test uses a throwaway LOCALAPPDATA, so nothing is installed for the user.
param(
    [Parameter(Mandatory = $true)]
    [string]$ArchivePath
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$LASTEXITCODE = 0

$root = Split-Path -Parent $PSScriptRoot
$installer = Join-Path $PSScriptRoot 'install.ps1'
if (-not (Test-Path -LiteralPath $installer -PathType Leaf)) {
    Write-Error "test-install: installer not found: $installer"
    exit 2
}
$archive = (Resolve-Path -LiteralPath $ArchivePath).Path
$shaFile = "$archive.sha256"
if (-not (Test-Path -LiteralPath $shaFile -PathType Leaf)) {
    Write-Error "test-install: checksum not found: $shaFile"
    exit 2
}

$work = Join-Path ([System.IO.Path]::GetTempPath()) ('riltra-install-test-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $work | Out-Null
$fakeLocal = Join-Path $work 'LocalAppData'
New-Item -ItemType Directory -Path $fakeLocal | Out-Null
$installDir = Join-Path $fakeLocal 'Programs\Riltra\bin'
$oldLocal = $env:LOCALAPPDATA
$env:LOCALAPPDATA = $fakeLocal

function Pass([string]$Message) { Write-Output "ok   $Message" }
function Fail([string]$Message) { Write-Output "FAIL $Message" | Out-Host; exit 1 }
function Invoke-Installer([string]$Exe, [string]$File) {
    & $Exe -NoProfile -ExecutionPolicy Bypass -File $installer -ArchivePath $File 2>&1 | ForEach-Object { Write-Host $_ }
    return $LASTEXITCODE
}
function Hash-File([string]$Path) { (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() }
function Copy-Case([string]$Name) {
    $dir = Join-Path $work "cases\$Name"
    New-Item -ItemType Directory -Path $dir -Force | Out-Null
    $file = Join-Path $dir (Split-Path -Leaf $archive)
    Copy-Item -LiteralPath $archive -Destination $file
    Copy-Item -LiteralPath $shaFile -Destination "$file.sha256"
    return $file
}
function Expect-InstallFail([string]$Name, [string]$File) {
    $code = Invoke-Installer 'pwsh' $File
    if ($code -eq 0) { Fail "$Name (installer exited 0)" }
    Pass $Name
}
function Expect-Unchanged([string]$Name, [string]$Expected) {
    if ((Hash-File (Join-Path $installDir 'riltra.exe')) -ne $Expected) { Fail "$Name (installed binary changed)" }
    Pass $Name
}
function Assert-NoStaging {
    $parent = Split-Path -Parent $installDir
    if (Test-Path -LiteralPath $parent) {
        $left = @(Get-ChildItem -LiteralPath $parent -Force | Where-Object { $_.Name -like '.stage-*' -or $_.Name -like '.previous-*' })
        if ($left.Count -ne 0) { Fail 'staging directory left behind' }
    }
}

try {
    # --- positive install with Windows PowerShell 5.1 ------------------------
    $code = Invoke-Installer 'powershell.exe' $archive
    if ($code -ne 0) { Fail 'positive install (Windows PowerShell 5.1)' }
    $exe = Join-Path $installDir 'riltra.exe'
    if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) { Fail 'payload binary' }
    foreach ($doc in @('LICENSE', 'NOTICE', 'THIRD_PARTY_LICENSES.txt', 'sbom.cdx.json', 'INSTALL.md')) {
        if (-not (Test-Path -LiteralPath (Join-Path $installDir $doc) -PathType Leaf)) { Fail "payload document $doc" }
        if ((Get-Item -LiteralPath (Join-Path $installDir $doc)).Length -eq 0) { Fail "payload document $doc is empty" }
    }
    Pass 'positive install (Windows PowerShell 5.1)'

    $version = & $exe --version
    if ($LASTEXITCODE -ne 0 -or $version -notmatch '^riltra ') { Fail 'installed binary runs' }
    Pass 'installed binary runs'

    if (-not $exe.StartsWith($fakeLocal, [StringComparison]::OrdinalIgnoreCase)) { Fail 'install path is outside LOCALAPPDATA' }
    Pass 'install path stays under LOCALAPPDATA'

    # --- reinstall with PowerShell 7 ----------------------------------------
    $before = Hash-File $exe
    $code = Invoke-Installer 'pwsh' $archive
    if ($code -ne 0) { Fail 'reinstall (PowerShell 7)' }
    if ((Hash-File $exe) -ne $before) { Fail 'reinstall changed the binary' }
    Pass 'reinstall (PowerShell 7)'
    Assert-NoStaging

    # --- checksum failures preserve the installation ------------------------
    $mismatch = Copy-Case 'mismatch'
    [System.IO.File]::AppendAllText($mismatch, 'x')
    Expect-InstallFail 'checksum mismatch rejected' $mismatch
    Expect-Unchanged 'checksum mismatch leaves the binary' $before
    Assert-NoStaging

    $missing = Copy-Case 'missing'
    Remove-Item -LiteralPath "$missing.sha256"
    Expect-InstallFail 'missing checksum rejected' $missing
    Expect-Unchanged 'missing checksum leaves the binary' $before
    Assert-NoStaging

    $freshMissing = Copy-Case 'missing-fresh'
    Remove-Item -LiteralPath "$freshMissing.sha256"
    $freshLocal = Join-Path $work 'FreshLocalAppData'
    New-Item -ItemType Directory -Path $freshLocal | Out-Null
    $env:LOCALAPPDATA = $freshLocal
    $code = Invoke-Installer 'pwsh' $freshMissing
    if ($code -eq 0) { Fail 'missing checksum rejected in a fresh LOCALAPPDATA' }
    if (Test-Path -LiteralPath (Join-Path $freshLocal 'Programs\Riltra\bin\riltra.exe')) {
        Fail 'missing checksum installed something in a fresh LOCALAPPDATA'
    }
    Pass 'missing checksum installs nothing in a fresh LOCALAPPDATA'
    $env:LOCALAPPDATA = $fakeLocal

    $malformed = Copy-Case 'malformed'
    Set-Content -LiteralPath "$malformed.sha256" -Value ('not-a-checksum  ' + (Split-Path -Leaf $malformed)) -Encoding Ascii
    Expect-InstallFail 'malformed checksum rejected' $malformed
    Expect-Unchanged 'malformed checksum leaves the binary' $before
    Assert-NoStaging

    # --- unsupported architecture -------------------------------------------
    $env:PROCESSOR_ARCHITEW6432 = 'ARM64'
    $armErr = Join-Path $work 'arm64.err'
    & pwsh -NoProfile -ExecutionPolicy Bypass -File $installer -ArchivePath $archive 2>$armErr | Out-Null
    $code = $LASTEXITCODE
    Remove-Item Env:PROCESSOR_ARCHITEW6432 -ErrorAction SilentlyContinue
    if ($code -eq 0) { Fail 'ARM64 rejected' }
    $message = Get-Content -LiteralPath $armErr -Raw
    if ($message -notmatch 'unsupported platform Windows/ARM64' -or $message -notmatch 'supported: Windows x64') {
        Fail 'ARM64 names the detected platform and the supported list'
    }
    Pass 'ARM64 rejected'
    Expect-Unchanged 'ARM64 leaves the binary' $before

    # --- existing unrelated executable --------------------------------------
    $otherLocal = Join-Path $work 'OtherLocalAppData'
    New-Item -ItemType Directory -Path (Join-Path $otherLocal 'Programs\Riltra\bin') -Force | Out-Null
    $foreign = Join-Path $otherLocal 'Programs\Riltra\bin\riltra.exe'
    Set-Content -LiteralPath $foreign -Value 'foreign' -Encoding Ascii
    $env:LOCALAPPDATA = $otherLocal
    $code = Invoke-Installer 'pwsh' $archive
    if ($code -eq 0) { Fail 'foreign executable is not replaced' }
    if ((Get-Content -LiteralPath $foreign -Raw).Trim() -ne 'foreign') { Fail 'foreign executable changed' }
    Pass 'foreign executable is not replaced'
    $env:LOCALAPPDATA = $fakeLocal

    Set-Content -LiteralPath (Join-Path $installDir 'notes.txt') -Value 'foreign' -Encoding Ascii
    $code = Invoke-Installer 'pwsh' $archive
    if ($code -eq 0) { Fail 'foreign payload file blocks reinstall' }
    Expect-Unchanged 'foreign payload file leaves the binary' $before
    if (-not (Test-Path -LiteralPath (Join-Path $installDir 'notes.txt'))) { Fail 'foreign payload file was removed' }
    Pass 'foreign payload file is unchanged'
    Remove-Item -LiteralPath (Join-Path $installDir 'notes.txt')

    # --- unsafe archive ------------------------------------------------------
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $evilDir = Join-Path $work 'cases\evil'
    New-Item -ItemType Directory -Path $evilDir -Force | Out-Null
    $evil = Join-Path $evilDir 'riltra-evil-windows-x64.zip'
    $zip = [System.IO.Compression.ZipFile]::Open($evil, [System.IO.Compression.ZipArchiveMode]::Create)
    try {
        $entry = $zip.CreateEntry('../escape.txt')
        $stream = $entry.Open()
        try { $bytes = [System.Text.Encoding]::ASCII.GetBytes('escape'); $stream.Write($bytes, 0, $bytes.Length) }
        finally { $stream.Dispose() }
    }
    finally { $zip.Dispose() }
    $evilHash = (Get-FileHash -LiteralPath $evil -Algorithm SHA256).Hash.ToLowerInvariant()
    Set-Content -LiteralPath "$evil.sha256" -Value ("$evilHash  $(Split-Path -Leaf $evil)") -Encoding Ascii
    Expect-InstallFail 'path traversal archive rejected' $evil
    if (Test-Path -LiteralPath (Join-Path $work 'escape.txt')) { Fail 'traversal archive wrote outside staging' }
    Pass 'traversal archive wrote nothing outside staging'
    Expect-Unchanged 'unsafe archive leaves the binary' $before

    # --- static no-elevation check ------------------------------------------
    $text = Get-Content -LiteralPath $installer -Raw
    if ($text -match 'RunAs|Start-Process[^\r\n]*-Verb|HKEY_LOCAL_MACHINE|HKLM|Program Files|SetEnvironmentVariable|\bsetx\b|/usr/local|\bsudo\b') {
        Fail 'installer contains an elevation or system-wide write'
    }
    Pass 'installer has no elevation or system-wide write'

    # --- clean-install smoke on the installed binary ------------------------
    $git = Get-Command git -ErrorAction SilentlyContinue
    if ($null -eq $git) { Fail 'clean-install smoke (git not available)' }
    $gitBash = Join-Path (Split-Path -Parent (Split-Path -Parent $git.Source)) 'bin\bash.exe'
    if (-not (Test-Path -LiteralPath $gitBash -PathType Leaf)) { Fail 'clean-install smoke (Git Bash not found)' }
    $env:RILTRA_TEST_PATH = Join-Path $root 'release\smoke.sh'
    $smoke = (& $gitBash -lc 'cygpath -u "$RILTRA_TEST_PATH"').Trim()
    $env:RILTRA_TEST_PATH = $installDir
    $installed = (& $gitBash -lc 'cygpath -u "$RILTRA_TEST_PATH"').Trim()
    Remove-Item Env:RILTRA_TEST_PATH
    & $gitBash $smoke $installed
    if ($LASTEXITCODE -ne 0) { Fail 'clean-install smoke' }
    Pass 'installed CLI/MCP smoke'

    Write-Output 'test-install: all checks passed'
}
finally {
    $env:LOCALAPPDATA = $oldLocal
    Remove-Item Env:PROCESSOR_ARCHITEW6432 -ErrorAction SilentlyContinue
    if (Test-Path -LiteralPath $work) { Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue }
}
