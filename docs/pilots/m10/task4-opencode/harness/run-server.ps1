param(
    [Parameter(Mandatory = $true)][string]$SessionRoot,
    [Parameter(Mandatory = $true)][ValidateSet('R', 'B')][string]$Arm,
    [Parameter(Mandatory = $true)][int]$Port,
    [string]$ServerPassword = 'm10-precheck-local',
    [switch]$DebugConfig,
    [switch]$DebugPaths
)

$ErrorActionPreference = 'Stop'
$root = '<ROOT>'
$profileDir = Join-Path $SessionRoot 'profile'
$workspace = Join-Path $SessionRoot 'workspace'

$envMap = @{
    HOME                                 = (Join-Path $profileDir 'home')
    USERPROFILE                          = (Join-Path $profileDir 'home')
    XDG_CONFIG_HOME                      = (Join-Path $profileDir 'config')
    XDG_DATA_HOME                        = (Join-Path $profileDir 'data')
    XDG_STATE_HOME                       = (Join-Path $profileDir 'state')
    XDG_CACHE_HOME                       = (Join-Path $profileDir 'cache')
    TMP                                  = (Join-Path $profileDir 'tmp')
    TEMP                                 = (Join-Path $profileDir 'tmp')
    SystemRoot                           = $env:SystemRoot
    windir                               = $env:windir
    COMSPEC                              = $env:COMSPEC
    PATHEXT                              = $env:PATHEXT
    NUMBER_OF_PROCESSORS                 = $env:NUMBER_OF_PROCESSORS
    PROCESSOR_ARCHITECTURE               = $env:PROCESSOR_ARCHITECTURE
    OS                                   = 'Windows_NT'
    OPENCODE_CONFIG_PROJECT_DISABLE      = '1'
    OPENCODE_DISABLE_CLAUDE_CODE         = '1'
    OPENCODE_DISABLE_CLAUDE_CODE_SKILLS  = '1'
    OPENCODE_DISABLE_AUTOUPDATE          = '1'
    OPENCODE_LOG_LEVEL                   = 'debug'
    OPENCODE_SERVER_PASSWORD             = $ServerPassword
    CARGO_TARGET_DIR                     = '<USERPROFILE>\AppData\Local\Temp\opencode\m10-cache'
    CARGO_HOME                           = '<USERPROFILE>\.cargo'
    RUSTUP_HOME                          = '<USERPROFILE>\.rustup'
    GIT_CONFIG_GLOBAL                    = (Join-Path $profileDir 'gitconfig')
    GIT_CONFIG_NOSYSTEM                  = '1'
    GIT_TERMINAL_PROMPT                  = '0'
    NO_COLOR                             = '1'
}
if ($Arm -eq 'R') {
    $envMap.RILTRA_HOME = Join-Path $SessionRoot 'riltra-home'
    $envMap.PATH = (@(
        (Join-Path $root 'bin'),
        'C:\Program Files\Git\cmd',
        '<USERPROFILE>\.cargo\bin',
        '<OPERATOR_HOME>\Tools\PowerShell\7',
        'C:\Windows\System32\WindowsPowerShell\v1.0',
        'C:\Windows\System32',
        'C:\Windows'
    ) -join ';')
}
else {
    $envMap.PATH = (@(
        'C:\Program Files\Git\cmd',
        '<USERPROFILE>\.cargo\bin',
        '<OPERATOR_HOME>\Tools\PowerShell\7',
        'C:\Windows\System32\WindowsPowerShell\v1.0',
        'C:\Windows\System32',
        'C:\Windows'
    ) -join ';')
}
$auth = Get-Content '<USERPROFILE>\.local\share\opencode\auth.json' -Raw | ConvertFrom-Json
$envMap.DEEPSEEK_API_KEY = $auth.deepseek.key
Set-Content -Path (Join-Path $profileDir 'gitconfig') -Value '' -Encoding utf8

$psi = [System.Diagnostics.ProcessStartInfo]::new()
$psi.FileName = '<USERPROFILE>\AppData\Roaming\npm\node_modules\@opencode\cli\bin\opencode.exe'
$psi.WorkingDirectory = $workspace
$psi.UseShellExecute = $false
$psi.Environment.Clear()
foreach ($kv in $envMap.GetEnumerator()) { $psi.Environment[$kv.Key] = [string]$kv.Value }
$ocArgs = if ($DebugConfig) { @('debug', 'config') } elseif ($DebugPaths) { @('debug', 'paths') } else { @('serve', '--hostname', '127.0.0.1', '--port', [string]$Port, '--log-level', 'debug', '--print-logs') }
foreach ($arg in $ocArgs) {
    $psi.ArgumentList.Add($arg)
}
$p = [System.Diagnostics.Process]::Start($psi)
if (-not $DebugConfig -and -not $DebugPaths) {
    $serverLogs = Join-Path $SessionRoot 'logs\server'
    New-Item -ItemType Directory -Force -Path $serverLogs | Out-Null
    $serverMeta = [ordered]@{
        executable             = $psi.FileName
        invocation             = 'ProcessStartInfo.ArgumentList; direct executable, no cmd.exe shim'
        args                   = $ocArgs
        hostname               = '127.0.0.1'
        port                   = $Port
        start_utc              = (Get-Date).ToUniversalTime().ToString('o')
        server_password_sha256 = [BitConverter]::ToString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($ServerPassword))).Replace('-','').ToLowerInvariant()
    }
    $serverMeta | ConvertTo-Json -Depth 5 | Set-Content -Path (Join-Path $serverLogs 'server-meta.json') -Encoding utf8
}
Write-Output "server pid=$($p.Id) port=$Port"
$p.WaitForExit()
exit $p.ExitCode
