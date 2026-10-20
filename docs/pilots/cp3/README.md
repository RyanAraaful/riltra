# M9 CP3 pilot archive

Multi-repository verification and E8 correctness, on the owner-approved CP3
scope. The report is `docs/m9-cp3-evidence.md`; this directory is the
evidence it rests on.

## Inputs (frozen)

| Repository | Pin |
|---|---|
| MMSP pilot | `d62f06a576d8e76b705c8ecc57fc380f7ebc9c44` |
| Riltra self-index input | `fca1185678de612552ad24a7932bb3f09f175507` |

Both were used as **isolated clones** under a bench root. The original
working trees were never modified.

## Layout

```
method/     the harness: every script takes its paths as parameters and
            carries no machine-local path
raw/        archived machine output, redacted
```

## Reproducing

```powershell
$w  = "$env:TEMP\opencode\cp3"
$bin = "$w\riltra-after.exe"     # the candidate binary

# Phase 1 — E8 baseline, verified sample, frozen anchors
node method/e8-baseline.cjs "$w\home\index\mmsp-pin.db" 'colmap/src' > raw/e8/baseline.json
node method/e8-missing-class.cjs raw/e8/baseline.json "$w\raw\mmsp-tree.txt" > raw/e8/missing-class.json
node method/e8-anchor.cjs "$w\home\index\mmsp-pin.db" 'colmap/src' > raw/e8/anchors.json

# Phase 3 — before/after on ONE index (the correction is query-side)
pwsh -File method/queries.ps1 -HomeDir "$w\home" -BinBefore "$w\riltra-before.exe" `
     -BinAfter $bin -Out "$w\raw\queries"
node method/e8-diff.cjs "$w\raw\queries" > raw/queries/diff.json

# Phase 4 — the isolation matrix (one home, two repositories)
pwsh -File method/multirepo.ps1 -HomeDir "$w\home" -BenchRoot $w -Bin $bin `
     -OutDir "$w\raw\multirepo"

# Phase 5 — the two semantic providers
pwsh -File method/g-sem.ps1 -HomeDir "$w\home-sem" -Bin $bin -OutDir "$w\raw\gsem"

# Phase 6 — the E9 stage profile
pwsh -File method/profile.ps1 -HomeDir "$w\home" -Bin $bin -OutDir "$w\raw\perf"
```

The incremental/full parity run and the MCP smoke reuse the CP2 harness
unchanged (`docs/pilots/cp2/method/incremental.ps1`,
`docs/pilots/cp2/method/mcp-smoke.cjs`).

## Redaction

Every archived artefact is passed through `docs/pilots/cp2/method/redact.cjs`
before it is committed. The script takes the paths to redact as arguments,
so it carries no machine-local path of its own. No credential, token or key
is present in this directory.
