param([Parameter(Mandatory)][string]$HomeDir, [Parameter(Mandatory)][string]$RawDir, [Parameter(Mandatory)][string]$Label)
function Find-Items($node, [System.Collections.Generic.List[object]]$acc) {
  if ($null -eq $node) { return }
  if ($node -is [System.Management.Automation.PSCustomObject]) {
    $names = @($node.PSObject.Properties.Name)
    if (($names -contains 'span') -and $node.span -and (@($node.span.PSObject.Properties.Name) -contains 'byte_start') -and ($names -contains 'path')) { $acc.Add($node) }
    foreach ($p in $node.PSObject.Properties) { Find-Items $p.Value $acc }
  } elseif ($node -is [System.Collections.IEnumerable] -and $node -isnot [string]) { foreach ($x in $node) { Find-Items $x $acc } }
}
$rows = @()
foreach ($f in 'rust-analyzer-definition.json','rust-analyzer-references.json','scip-typescript-query.json','ts-definition-from-callsite.json') {
  $p = Join-Path $RawDir $f
  if (-not (Test-Path -LiteralPath $p)) { $rows += [pscustomobject]@{ label=$Label; file=$f; status='MISSING' }; continue }
  $json = Get-Content -LiteralPath $p -Raw | ConvertFrom-Json
  $repo = [string]$json.repo
  $items = [System.Collections.Generic.List[object]]::new(); Find-Items $json $items
  if ($items.Count -eq 0) { $rows += [pscustomobject]@{ label=$Label; file=$f; repo=$repo; status='NO-SPANS'; diag=(@($json.server_diagnostics) -join ' | ') }; continue }
  foreach ($it in $items) {
    $fx = Join-Path $HomeDir ('fixture-' + $repo + '\' + ($it.path -replace '/', '\'))
    $bytes = [System.IO.File]::ReadAllBytes($fx)
    $s = [int]$it.span.byte_start; $e = [int]$it.span.byte_end
    $slice = [System.Text.Encoding]::UTF8.GetString($bytes, $s, $e - $s)
    $rows += [pscustomobject]@{ label=$Label; file=$f; repo=$repo; path=[string]$it.path; byte_start=$s; byte_end=$e; slice=$slice; roles=$it.roles; verification=[string]$it.verification; precision=[string]$it.precision; grounded=($slice -eq 'greet') }
  }
}
$rows
