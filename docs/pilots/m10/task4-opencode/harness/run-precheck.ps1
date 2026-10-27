param(
    [Parameter(Mandatory = $true)][string]$SessionRoot,
    [Parameter(Mandatory = $true)][ValidateSet('R', 'B')][string]$Arm,
    [Parameter(Mandatory = $true)][string]$PromptFile,
    [string]$Title = 'precheck',
    [string]$LogLevel = 'debug',
    [string]$ServerUrl = '',
    [string]$ServerPassword = 'm10-precheck-local'
)

$ErrorActionPreference = 'Stop'
$root = '<ROOT>'
$profileDir = Join-Path $SessionRoot 'profile'
$workspace = Join-Path $SessionRoot 'workspace'
$logs = Join-Path $SessionRoot ('logs\' + $Title)
New-Item -ItemType Directory -Force -Path $logs | Out-Null

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
    OPENCODE_LOG_LEVEL                   = $LogLevel
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

$prompt = Get-Content -Path $PromptFile -Raw
$psi = [System.Diagnostics.ProcessStartInfo]::new()
$psi.FileName = '<USERPROFILE>\AppData\Roaming\npm\node_modules\@opencode\cli\bin\opencode.exe'
$psi.WorkingDirectory = $workspace
$psi.UseShellExecute = $false
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError = $true
$psi.Environment.Clear()
foreach ($kv in $envMap.GetEnumerator()) { $psi.Environment[$kv.Key] = [string]$kv.Value }
$runArgs = @('run')
if ($ServerUrl) { $runArgs += @('--server', $ServerUrl) } else { $runArgs += '--standalone' }
$runArgs += @('--auto', '--format', 'json', '--log-level', $LogLevel, '--print-logs', '--model', 'deepseek/deepseek-v4-pro', '--title', $Title, $prompt)
foreach ($arg in $runArgs) {
    $psi.ArgumentList.Add($arg)
}

$start = (Get-Date).ToUniversalTime()
$p = [System.Diagnostics.Process]::Start($psi)
$stdoutTask = $p.StandardOutput.ReadToEndAsync()
$stderrTask = $p.StandardError.ReadToEndAsync()
$p.WaitForExit()
$end = (Get-Date).ToUniversalTime()
$stdout = $stdoutTask.Result
$stderr = $stderrTask.Result
Set-Content -Path (Join-Path $logs 'transcript.jsonl') -Value $stdout -Encoding utf8
Set-Content -Path (Join-Path $logs 'debug.log') -Value $stderr -Encoding utf8
$meta = [ordered]@{
    arm           = $Arm
    title         = $Title
    model         = 'deepseek/deepseek-v4-pro'
    command       = ('opencode run ' + $(if ($ServerUrl) { '--server ' + $ServerUrl } else { '--standalone' }) + ' --auto --format json --log-level ' + $LogLevel + ' --print-logs')
    executable    = $psi.FileName
    invocation    = 'ProcessStartInfo.ArgumentList; direct executable, no cmd.exe shim'
    server_startup = $(if ($ServerUrl) { 'pre-warmed opencode serve + opencode run --server' } else { 'standalone' })
    run_args      = @($runArgs[0..($runArgs.Count - 2)])
    prompt_bytes  = (Get-Item -LiteralPath $PromptFile).Length
    prompt_length = $prompt.Length
    server_url    = $ServerUrl
    workspace     = $workspace
    start_utc     = $start.ToString('o')
    end_utc       = $end.ToString('o')
    duration_s    = [Math]::Round(($end - $start).TotalSeconds, 3)
    exit_code     = $p.ExitCode
    prompt_sha256 = (Get-FileHash -LiteralPath $PromptFile -Algorithm SHA256).Hash.ToLowerInvariant()
    server_password_sha256 = [BitConverter]::ToString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($ServerPassword))).Replace('-','').ToLowerInvariant()
    env_keys      = ($envMap.Keys | Sort-Object)
    stdout_sha256 = (Get-FileHash (Join-Path $logs 'transcript.jsonl') -Algorithm SHA256).Hash.ToLowerInvariant()
    stderr_sha256 = (Get-FileHash (Join-Path $logs 'debug.log') -Algorithm SHA256).Hash.ToLowerInvariant()
}
$meta | ConvertTo-Json -Depth 5 | Set-Content -Path (Join-Path $logs 'run-meta.json') -Encoding utf8
Write-Output ($meta | ConvertTo-Json -Depth 5)
exit $p.ExitCode
