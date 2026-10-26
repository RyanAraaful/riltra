# The twelfth MCP tool (riltra_code_semantic) on the CP2 pilot, over the
# real stdio transport:
#   - the tool catalog is twelve tools, and the new one is in it;
#   - without a configured provider the tool returns a classified refusal and
#     the eleven other tools are unaffected;
#   - the check order holds: a bad path is refused before the provider is
#     looked up;
#   - stdout carries JSON-RPC only.
param(
    [string]$Bin = '<RILTRA_BIN>',
    [string]$HomeDir = "$env:TEMP\opencode\bench\home-main",
    [string]$Out = "$env:TEMP\opencode\bench\cp2\raw\semantic-tool"
)
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force -Path $Out | Out-Null

$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $Bin
$psi.Arguments = 'mcp'
$psi.RedirectStandardInput = $true
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError = $true
$psi.UseShellExecute = $false
$psi.EnvironmentVariables['RILTRA_HOME'] = $HomeDir
# No RILTRA_LSP_SERVER: the provider is operator-configured, and the pilot
# has no provider configured on purpose.
$p = [System.Diagnostics.Process]::Start($psi)

$lines = New-Object System.Collections.ArrayList
function Send-Json($text) { $p.StandardInput.WriteLine($text); $p.StandardInput.Flush() }
function Read-Json() {
    $line = $p.StandardOutput.ReadLine()
    [void]$lines.Add($line)
    $line | ConvertFrom-Json
}

Send-Json '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"cp2-semantic","version":"0"}}}'
$init = Read-Json
Send-Json '{"jsonrpc":"2.0","method":"notifications/initialized"}'
Send-Json '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'
$list = Read-Json
$names = @($list.result.tools | ForEach-Object { $_.name } | Sort-Object)

Send-Json '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"riltra_code_semantic","arguments":{"repo":"mmsp-pin","path":"colmap/src/colmap/mvs/patch_match.cc","line":118,"col":27,"operation":"definition"}}}'
$noProvider = Read-Json
Send-Json '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"riltra_code_semantic","arguments":{"repo":"mmsp-pin","path":"../escape.cc","line":118,"col":27,"operation":"definition"}}}'
$badPath = Read-Json
Send-Json '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"riltra_code_search","arguments":{"repo":"mmsp-pin","query":"PatchMatch"}}}'
$search = Read-Json
Send-Json '{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"riltra_code_semantic","arguments":{"repo":"mmsp-pin","path":"colmap/src/colmap/mvs/patch_match.cc","line":118,"col":27,"operation":"guess"}}}'
$badOperation = Read-Json

$p.StandardInput.Close()
Start-Sleep -Milliseconds 300
$stderr = ''
if ($p.HasExited) { $stderr = $p.StandardError.ReadToEnd() }
if (-not $p.HasExited) { $p.Kill() }

$report = [ordered]@{
    tool_count              = $names.Count
    tools                   = $names
    semantic_tool_present   = ($names -contains 'riltra_code_semantic')
    no_provider             = $noProvider.result
    bad_path                = $badPath.result
    unknown_operation       = $badOperation.result
    search_works            = @($search.result.structuredContent.results).Count
    stdout_lines            = $lines.Count
    stdout_non_jsonrpc      = @($lines | Where-Object { ($_ | ConvertFrom-Json).jsonrpc -ne '2.0' }).Count
    stderr                  = $stderr
}
$report | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $Out 'semantic-tool.json')
$lines | Set-Content (Join-Path $Out 'stdout.txt')
Write-Host ("tools: {0} (the semantic tool present: {1})" -f $names.Count, $report.semantic_tool_present)
Write-Host ("no provider  -> {0}" -f ($noProvider.result.structuredContent | ConvertTo-Json -Compress -Depth 5))
Write-Host ("bad path     -> {0}" -f ($badPath.result.structuredContent | ConvertTo-Json -Compress -Depth 5))
Write-Host ("bad operation-> {0}" -f ($badOperation.result.structuredContent | ConvertTo-Json -Compress -Depth 5))
Write-Host ("search hits  -> {0}" -f $report.search_works)
Write-Host ("stdout lines -> {0}, non-JSON-RPC: {1}" -f $report.stdout_lines, $report.stdout_non_jsonrpc)

