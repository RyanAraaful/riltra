# CP2 harness

The scripts that produced the CP2 evidence. Every path is a parameter: no
machine-local path is baked into the harness, and no placeholder has to be
edited before a run.

## Prerequisites

- the `riltra` binary (debug: `<repo>/target/debug/riltra.exe`, release:
  `<repo>/target/release/riltra.exe`);
- a git clone of the pilot repository, checked out at the pinned commit and
  placed **inside the benchmark root**;
- PowerShell 7 (`pwsh`) and Node.js with `node:sqlite` (Node 22+).

## Parameters

Every step script accepts the same switches:

| Switch | Meaning | Default |
|---|---|---|
| `-BenchRoot` | the benchmark workspace | `$env:TEMP/opencode/bench` |
| `-Bin` | the riltra binary | `<repo>/target/debug/riltra.exe` (from the script location) |
| `-Snapshot` | the pilot snapshot | `<BenchRoot>/mmsp-d62f06a` |
| `-MmspRoot` | the **original** pilot repository, verified but never modified | none |
| `-Out` | the raw evidence directory | `<BenchRoot>/cp2/raw` |
| `-HomeDir` | the `RILTRA_HOME` of the pilot index | `<BenchRoot>/home-main` |
| `-ReleaseBin` | the release binary (parity step only) | `<repo>/target/release/riltra.exe` |

## The steps

```powershell
$bench = "$env:TEMP/opencode/bench"
$snap  = "$bench/mmsp-d62f06a"      # a clone, checked out at the pin
$raw   = "$bench/cp2/raw"

# 0. the path-safety guard (no side effects outside a private temp dir)
pwsh -File test-path-safety.ps1

# 1. the whole run
pwsh -File run-all.ps1 -BenchRoot $bench -Snapshot $snap -MmspRoot <original clone>
```

The individual steps, in the order `run-all.ps1` runs them:

| Script | What it does |
|---|---|
| `cold-index.ps1` | two independent cold indexes (D-M9-3) |
| `cold-index-3.ps1` | a third cold index with the indexer's own peak RSS |
| `queries.ps1` | the CP1 section D workflow queries + the class-anchor probes |
| `grade.cjs` | grades the query output against the CP1 section D criteria |
| `grade.test.cjs` | the regression tests of the grader (`node --test`) |
| `freshness.ps1` | the CP1 section C freshness scenarios F0-F5 |
| `incremental.ps1` | the no-op refresh, the one-file delta, the oracle rebuild and the query parity |
| `perf.ps1` | the warm-query timings |
| `parity-release.ps1` | the same-tree fact parity plus the release profile |
| `mcp-smoke.cjs` | the MCP stdio smoke (needs `CP2_RILTRA`, `CP2_HOME`, `CP2_OUT`) |
| `d5-mcp-sequence.cjs` | the D.5 tool sequence over the real MCP transport |
| `opencode-session.ps1` | the agent-side OpenCode session |
| `db-facts.cjs` | read-only fact reports (`facts`, `split`, `digest`) |
| `db-mutation-check.cjs` | the content digest of an index database |
| `fact-digest.cjs` | the shared order-independent digest implementation |
| `db-include-probe.cjs`, `db-probe*.cjs` | the read-only diagnostics behind the decision package |
| `measure.ps1` | the measurement wrapper (wall time, peak RSS) |
| `redact.cjs` | redacts machine-local paths before archiving |
| `common.ps1` | the path-safety helpers and the shared configuration |

## Path safety

`common.ps1` holds every guard. A destructive operation is refused unless
the **resolved** path is strictly inside the benchmark root:

- the containment test is component by component, so a sibling whose name
  starts with the benchmark root's name (`bench-evil` next to `bench`) is
  outside;
- the benchmark root itself is never a mass-delete target;
- junctions and symlinks are resolved first, so a link inside the root that
  points outside it is outside;
- the pilot snapshot may only be mutated through `Restore-Snapshot`, which
  first proves that it is a git repository inside the benchmark root and is
  not the original repository.

`test-path-safety.ps1` is the regression test for those rules.

## Comparisons fail loudly

A comparison never reports a match it did not measure:

- a query that exits non-zero, prints nothing, or cannot be parsed aborts
  the step;
- the fact parity needs a digest from both sides; a missing digest aborts
  the run instead of comparing two empty strings;
- the grader distinguishes `PASS`, `FAIL`, `UNMET` (a capability gap) and
  `UNAVAILABLE` (the evidence could not be obtained). A parse error is
  never a `PASS`, an empty array is never proof of completeness, and an
  impact verdict must be exactly `UNKNOWN`.

## Redaction

`redact.cjs` takes the paths to redact as arguments, so it carries no
machine-local path of its own:

```powershell
node redact.cjs <raw-dir> <archive-dir> `
  "<bench-root>=<CP2_BENCH>" "<riltra-checkout>=<RILTRA_ROOT>" "<mmsp-checkout>=<MMSP_ROOT>"
```

Each mapping also replaces the JSON-escaped forms of the path.
