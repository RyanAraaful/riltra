param(
    [string]$Root = '<ROOT>',
    [string]$Freeze = '<REPO>\docs\pilots\m10\freeze',
    [int]$BasePort = 29100,
    [switch]$Execute
)

$ErrorActionPreference = 'Stop'
$exe = '<USERPROFILE>\AppData\Roaming\npm\node_modules\@opencode\cli\bin\opencode.exe'
$pwsh = '<OPERATOR_HOME>\Tools\PowerShell\7\pwsh.exe'
$cache = '<USERPROFILE>\AppData\Local\Temp\opencode\m10-cache'
$tasks = @('A1', 'A2', 'A3', 'A4')
$arms = @('R', 'B')
$snapshot = Join-Path $Root 'snapshot-pin'
$riltra = Join-Path $Root 'bin\riltra.exe'
if (-not (Test-Path -LiteralPath $exe)) { throw "missing executable: $exe" }
if (-not (Test-Path -LiteralPath $snapshot)) { throw "missing snapshot: $snapshot" }
if (-not (Test-Path -LiteralPath $riltra)) { throw "missing riltra binary: $riltra" }

$frozen = @{}
Get-Content -LiteralPath (Join-Path $Freeze 'SHA256SUMS.txt') | ForEach-Object {
    if ($_ -match '^([0-9a-f]{64}) \*prompts/(A\d\.txt)$') { $frozen[$Matches[2]] = $Matches[1] }
}

$plan = @()
$index = 0
foreach ($task in $tasks) {
    foreach ($arm in $arms) {
        $prompt = Join-Path $Freeze ('prompts\' + $task + '.txt')
        if (-not (Test-Path -LiteralPath $prompt)) { throw "missing frozen prompt: $prompt" }
        $sha = (Get-FileHash -LiteralPath $prompt -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($frozen[$task + '.txt'] -ne $sha) { throw "frozen prompt hash mismatch: $prompt" }
        $plan += [pscustomobject]@{
            run          = "$task-$arm"
            arm          = $arm
            task         = $task
            port         = $BasePort + $index
            session_root = Join-Path $Root ('sessions\bench\' + $task + '-' + $arm)
            prompt       = $prompt
            prompt_sha256 = $sha
            server_exe   = $exe
            client_exe   = $exe
            startup      = 'opencode serve (pre-warmed) + opencode run --server'
            password     = 'random per run, never written'
        }
        $index += 1
    }
}

if (-not $Execute) {
    $plan | ConvertTo-Json -Depth 5
    Write-Output ("plan_only=true runs={0} r={1} b={2} unique_ports={3}" -f $plan.Count, @($plan | Where-Object arm -eq 'R').Count, @($plan | Where-Object arm -eq 'B').Count, @($plan.port | Sort-Object -Unique).Count)
    return
}

function Wait-Server {
    param([int]$Port, [hashtable]$Headers, [int]$Seconds = 180)
    for ($i = 0; $i -lt ($Seconds * 2); $i++) {
        Start-Sleep -Milliseconds 500
        try {
            $r = Invoke-WebRequest -Uri "http://127.0.0.1:$Port/api/info" -Headers $Headers -TimeoutSec 3 -ErrorAction Stop
            if ($r.StatusCode -eq 200) { return $r.Content }
        } catch {}
    }
    throw "server on port $Port did not become ready"
}

function Save-Api {
    param([int]$Port, [hashtable]$Headers, [string]$Directory, [string]$Location, [string]$Prefix = 'api')
    $loc = [uri]::EscapeDataString($Location)
    foreach ($ep in @(
        @('config', '/api/config'),
        @('skill', '/api/skill?location%5Bdirectory%5D=' + $loc),
        @('plugin', '/api/plugin?location%5Bdirectory%5D=' + $loc),
        @('mcp', '/api/mcp?location%5Bdirectory%5D=' + $loc),
        @('session', '/api/session?location%5Bdirectory%5D=' + $loc)
    )) {
        $r = Invoke-WebRequest -Uri ("http://127.0.0.1:$Port" + $ep[1]) -Headers $Headers -TimeoutSec 15 -ErrorAction Stop
        [IO.File]::WriteAllText((Join-Path $Directory ($Prefix + '-' + $ep[0] + '.json')), $r.Content, (New-Object System.Text.UTF8Encoding($false)))
    }
}

function Write-Manifest {
    param([string]$Workspace, [string]$Path)
    $lines = Get-ChildItem -LiteralPath $Workspace -Recurse -File -Force |
        ForEach-Object {
            $rel = $_.FullName.Substring($Workspace.Length + 1).Replace('\', '/')
            (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() + '  ' + $rel
        } | Sort-Object
    [IO.File]::WriteAllText($Path, (($lines -join "`n") + "`n"), (New-Object System.Text.UTF8Encoding($false)))
    return [ordered]@{
        path   = $Path
        files  = @($lines).Count
        sha256 = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
    }
}

function Invoke-RiltraStep {
    param([string]$RiltraHomeDir, [string[]]$Arguments, [string]$LogFile)
    $prev = $env:RILTRA_HOME
    $env:RILTRA_HOME = $RiltraHomeDir
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $out = (& $riltra @Arguments 2>&1 | Out-String)
    $code = $LASTEXITCODE
    $sw.Stop()
    if ($null -eq $prev) { Remove-Item Env:RILTRA_HOME -ErrorAction SilentlyContinue } else { $env:RILTRA_HOME = $prev }
    $rec = [ordered]@{
        step       = $Arguments[0]
        command    = ('riltra ' + ($Arguments -join ' '))
        exit_code  = $code
        duration_s = [Math]::Round($sw.Elapsed.TotalSeconds, 3)
        output     = $out.Trim()
    }
    Add-Content -LiteralPath $LogFile -Value ($rec | ConvertTo-Json -Compress -Depth 5) -Encoding utf8
    if ($code -ne 0) { throw "riltra $($Arguments[0]) failed with exit $code`: $out" }
    return $rec
}

function Invoke-CargoPreflight {
    param([string]$Workspace, [string]$LogFile)
    $prevTarget = $env:CARGO_TARGET_DIR
    $prevFlags = $env:RUSTFLAGS
    $env:CARGO_TARGET_DIR = $cache
    $env:RUSTFLAGS = ''
    Push-Location $Workspace
    try {
        $clean = (& cargo clean -p riltra 2>&1 | Out-String)
        $cleanExit = $LASTEXITCODE
        $sw = [Diagnostics.Stopwatch]::StartNew()
        $test = (& cargo test --locked --bin riltra json::tests 2>&1 | Out-String)
        $testExit = $LASTEXITCODE
        $sw.Stop()
    }
    finally {
        Pop-Location
        if ($null -eq $prevTarget) { Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue } else { $env:CARGO_TARGET_DIR = $prevTarget }
        if ($null -eq $prevFlags) { Remove-Item Env:RUSTFLAGS -ErrorAction SilentlyContinue } else { $env:RUSTFLAGS = $prevFlags }
    }
    $compile = @($test -split "`r?`n" | Where-Object { $_ -match 'Compiling riltra' })
    $resultLine = @($test -split "`r?`n" | Where-Object { $_ -match '^test result:' }) | Select-Object -Last 1
    $bin = Get-ChildItem -LiteralPath (Join-Path $cache 'debug\deps') -Filter 'riltra-*.exe' -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1
    $rec = [ordered]@{
        command              = 'cargo clean -p riltra && cargo test --locked --bin riltra json::tests'
        cargo_target_dir     = $cache
        rustflags            = ''
        clean_exit_code      = $cleanExit
        test_exit_code       = $testExit
        duration_s           = [Math]::Round($sw.Elapsed.TotalSeconds, 3)
        compiling_line       = $compile
        result_line          = $resultLine
        test_binary_path     = $(if ($bin) { $bin.FullName } else { $null })
        test_binary_sha256   = $(if ($bin) { (Get-FileHash -LiteralPath $bin.FullName -Algorithm SHA256).Hash.ToLowerInvariant() } else { $null })
        output_tail          = (@($test -split "`r?`n") | Select-Object -Last 25) -join "`n"
    }
    [IO.File]::WriteAllText($LogFile, ($rec | ConvertTo-Json -Depth 6), (New-Object System.Text.UTF8Encoding($false)))
    return $rec
}

$results = @()
$progress = Join-Path $Root 'export\benchmark-progress.jsonl'
New-Item -ItemType Directory -Force -Path (Join-Path $Root 'export') | Out-Null

foreach ($spec in $plan) {
    if (Test-Path -LiteralPath $spec.session_root) { throw "session already exists: $($spec.session_root)" }
    if (Get-NetTCPConnection -State Listen -LocalPort $spec.port -ErrorAction SilentlyContinue) { throw "port in use: $($spec.port)" }
    $opListener = Get-NetTCPConnection -State Listen -LocalPort 28894 -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($opListener) { Write-Output "operator service observed on 28894 pid=$($opListener.OwningProcess) (not used)" }

    $session = $spec.session_root
    $profile = Join-Path $session 'profile'
    $workspace = Join-Path $session 'workspace'
    foreach ($d in @('profile\home', 'profile\config\opencode', 'profile\data', 'profile\state', 'profile\cache', 'profile\tmp', 'workspace', 'logs\server', 'logs\run', 'logs\setup')) {
        New-Item -ItemType Directory -Force -Path (Join-Path $session $d) | Out-Null
    }
    Copy-Item -Path (Join-Path $snapshot '*') -Destination $workspace -Recurse -Force
    Copy-Item -LiteralPath $spec.prompt -Destination (Join-Path $session 'prompt.txt')

    $setupLog = Join-Path $session 'logs\setup\riltra.jsonl'
    $setup = [ordered]@{
        run             = $spec.run
        arm             = $spec.arm
        task            = $spec.task
        snapshot_copy   = 'git archive pin 7f86891c, verified 1785 files'
        patch           = $null
        patch_sha256    = $null
        patch_exit_code = $null
        riltra_steps    = @()
        index_status    = $null
        cargo_preflight = $null
    }
    $manifestBefore = Write-Manifest -Workspace $workspace -Path (Join-Path $session 'logs\setup\workspace-before.tsv')
    $setup['workspace_before'] = $manifestBefore

    if ($spec.task -eq 'A3' -or $spec.task -eq 'A4') {
        $patchName = if ($spec.task -eq 'A3') { 'a3-edit.patch' } else { 'a4-defect.patch' }
        $patchPath = Join-Path $Freeze ('edits\' + $patchName)
        $setup.patch = $patchPath
        $setup.patch_sha256 = (Get-FileHash -LiteralPath $patchPath -Algorithm SHA256).Hash.ToLowerInvariant()
    }

    if ($spec.arm -eq 'R') {
        New-Item -ItemType Directory -Force -Path (Join-Path $session 'riltra-home') | Out-Null
        $riltraHome = Join-Path $session 'riltra-home'
        $rc = Invoke-RiltraStep -RiltraHomeDir $riltraHome -Arguments @('init', '--repo', $workspace, '--name', 'm10') -LogFile $setupLog
        $setup.riltra_steps += $rc
        if ($spec.task -ne 'A4') {
            $rc = Invoke-RiltraStep -RiltraHomeDir $riltraHome -Arguments @('index', '--repo', 'm10') -LogFile $setupLog
            $setup.riltra_steps += $rc
        }
    }

    if ($setup.patch) {
        Push-Location $workspace
        $patchOut = (& git apply $setup.patch 2>&1 | Out-String)
        $setup.patch_exit_code = $LASTEXITCODE
        Pop-Location
        if ($setup.patch_exit_code -ne 0) { throw "patch failed: $($setup.patch)`: $patchOut" }
    }

    if ($spec.arm -eq 'R') {
        if ($spec.task -eq 'A4') {
            $rc = Invoke-RiltraStep -RiltraHomeDir $riltraHome -Arguments @('index', '--repo', 'm10') -LogFile $setupLog
            $setup.riltra_steps += $rc
        }
        $rc = Invoke-RiltraStep -RiltraHomeDir $riltraHome -Arguments @('index', 'status', '--repo', 'm10') -LogFile $setupLog
        $setup.riltra_steps += $rc
        $setup.index_status = $rc.output
    }

    if ($spec.task -eq 'A4') {
        $setup.cargo_preflight = Invoke-CargoPreflight -Workspace $workspace -LogFile (Join-Path $session 'logs\setup\cargo-prestep.json')
        Write-Output ("run={0} cargo_preflight_exit={1} result={2}" -f $spec.run, $setup.cargo_preflight.test_exit_code, $setup.cargo_preflight.result_line)
    }

    $manifestPre = Write-Manifest -Workspace $workspace -Path (Join-Path $session 'logs\setup\workspace-pre-session.tsv')
    $setup['workspace_pre_session'] = $manifestPre
    [IO.File]::WriteAllText((Join-Path $session 'logs\setup\setup.json'), ($setup | ConvertTo-Json -Depth 8), (New-Object System.Text.UTF8Encoding($false)))

    $deny = [ordered]@{
        '<REPO>/**'                                                                      = 'deny'
        '<USERPROFILE>/.config/opencode/**'                                          = 'deny'
        '<USERPROFILE>/.local/share/opencode/**'                                     = 'deny'
        '<ROOT>/evidence/**'  = 'deny'
        '<ROOT>/export/**'    = 'deny'
        '<ROOT>/sessions/**'  = 'deny'
        '<ROOT>/pin/**'       = 'deny'
        '<ROOT>/snapshot-pin/**' = 'deny'
    }
    $config = [ordered]@{
        '$schema'  = 'https://opencode.ai/config.json'
        model      = 'deepseek/deepseek-v4-pro'
        update     = 'disable'
        snapshots  = $false
        lsp        = $false
        formatter  = $false
        permission = [ordered]@{ external_directory = $deny }
    }
    if ($spec.arm -eq 'R') {
        $config['mcp'] = [ordered]@{
            servers = [ordered]@{
                riltra = [ordered]@{
                    type        = 'local'
                    command     = @($riltra, 'mcp')
                    environment = [ordered]@{ RILTRA_HOME = (Join-Path $session 'riltra-home') }
                    codemode    = $false
                }
            }
        }
        $config.permission['riltra_riltra_code_semantic'] = 'deny'
    }
    $configPath = Join-Path $profile 'config\opencode\opencode.json'
    [IO.File]::WriteAllText($configPath, ($config | ConvertTo-Json -Depth 10), (New-Object System.Text.UTF8Encoding($false)))

    $password = [Convert]::ToBase64String([Security.Cryptography.RandomNumberGenerator]::GetBytes(32)).Replace('+', 'A').Replace('/', 'B').Replace('=', '')
    $auth = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes("opencode:$password"))
    $headers = @{ Authorization = "Basic $auth" }
    $serverUrl = "http://127.0.0.1:$($spec.port)"
    $srv = $null
    $runMeta = $null
    $sessionInfo = $null
    try {
        $serverArg = '-NoProfile -File "' + (Join-Path $Root 'run-server.ps1') + '" -SessionRoot "' + $session + '" -Arm ' + $spec.arm + ' -Port ' + $spec.port + ' -ServerPassword "' + $password + '"'
        $srv = Start-Process -FilePath $pwsh -ArgumentList $serverArg -PassThru -WindowStyle Hidden `
            -RedirectStandardOutput (Join-Path $session 'logs\server\stdout.log') `
            -RedirectStandardError (Join-Path $session 'logs\server\stderr.log')
        $info = Wait-Server -Port $spec.port -Headers $headers
        $serverPid = ($info | ConvertFrom-Json).pid
        $listener = Get-NetTCPConnection -State Listen -LocalPort $spec.port -ErrorAction SilentlyContinue | Select-Object -First 1
        if (-not $listener -or $listener.OwningProcess -ne $serverPid) { throw "port $($spec.port) not owned by probe server $serverPid" }
        [IO.File]::WriteAllText((Join-Path $session 'logs\server\api-info.json'), $info, (New-Object System.Text.UTF8Encoding($false)))
        Save-Api -Port $spec.port -Headers $headers -Directory (Join-Path $session 'logs\server') -Location $workspace -Prefix 'api-before'

        $runArg = '-NoProfile -File "' + (Join-Path $Root 'run-precheck.ps1') + '" -SessionRoot "' + $session + '" -Arm ' + $spec.arm + ' -PromptFile "' + (Join-Path $session 'prompt.txt') + '" -Title ' + $spec.task + '-' + $spec.arm + ' -ServerUrl ' + $serverUrl + ' -ServerPassword "' + $password + '"'
        $run = Start-Process -FilePath $pwsh -ArgumentList $runArg -PassThru -WindowStyle Hidden `
            -RedirectStandardOutput (Join-Path $session 'logs\run\stdout.log') `
            -RedirectStandardError (Join-Path $session 'logs\run\stderr.log')
        $finished = $run.WaitForExit(3600000)
        if (-not $finished) { throw "run $($spec.run) exceeded 3600 s" }
        $runMeta = Get-Content -LiteralPath (Join-Path $session ('logs\' + $spec.task + '-' + $spec.arm + '\run-meta.json')) -Raw | ConvertFrom-Json

        Save-Api -Port $spec.port -Headers $headers -Directory (Join-Path $session 'logs\server') -Location $workspace -Prefix 'api-after'
        $sessionList = Get-Content -LiteralPath (Join-Path $session 'logs\server\api-after-session.json') -Raw | ConvertFrom-Json
        $sessionInfo = @($sessionList.data) | Where-Object { $_.location.directory -eq $workspace } | Select-Object -First 1
        if (-not $sessionInfo) { $sessionInfo = @($sessionList.data) | Select-Object -First 1 }

        $manifestAfter = Write-Manifest -Workspace $workspace -Path (Join-Path $session 'logs\setup\workspace-after.tsv')
        $before = Get-Content -LiteralPath $manifestBefore.path
        $after = Get-Content -LiteralPath $manifestAfter.path
        $diff = Compare-Object $before $after
        $changed = @($diff | ForEach-Object { ($_.InputObject -split '  ', 2)[1] } | Sort-Object -Unique)
        $diffLines = @($diff | ForEach-Object { "$($_.SideIndicator) $($_.InputObject)" })
        [IO.File]::WriteAllText((Join-Path $session 'logs\setup\workspace-diff.txt'), (($diffLines -join "`n") + "`n"), (New-Object System.Text.UTF8Encoding($false)))
        $workspaceSummary = [ordered]@{
            files_before        = $manifestPre.files
            files_after         = $manifestAfter.files
            changed_files       = $changed
            changed_count       = $changed.Count
            diff_lines          = $diffLines.Count
            manifest_after_path = $manifestAfter.path
            manifest_after_sha  = $manifestAfter.sha256
        }
        [IO.File]::WriteAllText((Join-Path $session 'logs\setup\workspace-summary.json'), ($workspaceSummary | ConvertTo-Json -Depth 5), (New-Object System.Text.UTF8Encoding($false)))

        $summary = [ordered]@{
            run            = $spec.run
            arm            = $spec.arm
            task           = $spec.task
            port           = $spec.port
            server_url     = $serverUrl
            server_pid     = $serverPid
            session_id     = $sessionInfo.id
            model          = $sessionInfo.model
            outcome        = $sessionInfo.outcome
            cost           = $sessionInfo.cost
            tokens         = $sessionInfo.tokens
            prompt_sha256  = $spec.prompt_sha256
            config_sha256  = (Get-FileHash -LiteralPath $configPath -Algorithm SHA256).Hash.ToLowerInvariant()
            executable     = $exe
            executable_sha256 = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant()
            riltra_binary_sha256 = $(if ($spec.arm -eq 'R') { (Get-FileHash -LiteralPath $riltra -Algorithm SHA256).Hash.ToLowerInvariant() } else { $null })
            startup        = $spec.startup
            exit_code      = $runMeta.exit_code
            duration_s     = $runMeta.duration_s
            start_utc      = $runMeta.start_utc
            end_utc        = $runMeta.end_utc
            patch          = $setup.patch
            patch_sha256   = $setup.patch_sha256
            index_status   = $setup.index_status
            cargo_preflight = $setup.cargo_preflight
            workspace      = $workspaceSummary
        }
        [IO.File]::WriteAllText((Join-Path $session 'session.json'), ($summary | ConvertTo-Json -Depth 8), (New-Object System.Text.UTF8Encoding($false)))
        $results += $summary
        Add-Content -LiteralPath $progress -Value ($summary | ConvertTo-Json -Compress -Depth 8) -Encoding utf8
        Write-Output ("run={0} exit={1} duration_s={2} session={3} cost={4}" -f $spec.run, $runMeta.exit_code, $runMeta.duration_s, $sessionInfo.id, $sessionInfo.cost)
    }
    finally {
        if ($srv) {
            $infoPath = Join-Path $session 'logs\server\api-info.json'
            if (Test-Path -LiteralPath $infoPath) {
                $sp = (Get-Content -LiteralPath $infoPath -Raw | ConvertFrom-Json).pid
                Stop-Process -Id $sp -Force -ErrorAction SilentlyContinue
            }
            else {
                $listener = Get-NetTCPConnection -State Listen -LocalPort $spec.port -ErrorAction SilentlyContinue | Select-Object -First 1
                if ($listener) { Stop-Process -Id $listener.OwningProcess -Force -ErrorAction SilentlyContinue }
            }
            Stop-Process -Id $srv.Id -Force -ErrorAction SilentlyContinue
        }
        for ($i = 0; $i -lt 40; $i++) {
            if (-not (Get-NetTCPConnection -State Listen -LocalPort $spec.port -ErrorAction SilentlyContinue)) { break }
            Start-Sleep -Milliseconds 250
        }
        if (Get-NetTCPConnection -State Listen -LocalPort $spec.port -ErrorAction SilentlyContinue) {
            throw "port $($spec.port) still listening after run $($spec.run)"
        }
    }
}

[IO.File]::WriteAllText((Join-Path $Root 'export\benchmark-summary.json'), ($results | ConvertTo-Json -Depth 8), (New-Object System.Text.UTF8Encoding($false)))
Write-Output ("benchmark_done=true runs={0}" -f $results.Count)
