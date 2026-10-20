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

The individual steps. `run-all.ps1` runs the first group in this order; the
rest are standalone (the diagnostics, the release profile and the
self-tests, which `run-all.ps1` also runs as step 0).

| Script | What it does |
|---|---|
| `cold-index.ps1` | two independent cold indexes (D-M9-3) |
| `cold-index-3.ps1` | a third cold index with the indexer's own peak RSS |
| `queries.ps1` | the CP1 section D workflow queries + the class-anchor probes |
| `grade.cjs` | grades the query output against the CP1 section D criteria |
| `grade.test.cjs` | the regression tests of the grader (`node --test`) |
| `freshness.ps1` | the CP1 section C freshness scenarios F0-F5 |
| `incremental.ps1` | the one-file delta, the independent full rebuild of the **same** changed tree, the fact digests of both, the query parity, and only then the restore |
| `perf.ps1` | the warm-query timings |
| `parity-release.ps1` | the same-tree fact parity on the pinned tree plus the release profile |
| `fact-digest.cjs` | the order-independent fact digest and the **comparison** (digests, query answers, the volatile-metadata normalization) |
| `fact-digest.test.cjs` | the regression tests of the comparison (`node --test`) |
| `db-mutation-check.cjs` | the content digest of an index database |
| `db-facts.cjs` | read-only fact reports (`facts`, `split`, `digest`) |
| `db-include-probe.cjs`, `db-probe*.cjs` | the read-only diagnostics behind the decision package |
| `mcp-smoke.cjs` | the MCP stdio smoke (needs `CP2_RILTRA`, `CP2_HOME`, `CP2_OUT`) |
| `d5-mcp-sequence.cjs` | the D.5 tool sequence over the real MCP transport |
| `opencode-session.ps1` | the agent-side OpenCode session |
| `measure.ps1` | the measurement wrapper (wall time, peak RSS) |
| `redact.cjs` | redacts machine-local paths before archiving |
| `common.ps1` | the path-safety helpers and the shared configuration |

## The parity step

`incremental.ps1` proves the parity on **one** source tree — the tree with
the one-file change — and never compares two indexes from different source
states:

1. restore the snapshot to the frozen pin;
2. refresh the baseline index on that tree (a no-op: `reparsed` must be 0);
3. change exactly one file (the porcelain must show exactly one entry);
4. index incrementally on the changed tree (`reparsed` must be 1);
5. build an independent full rebuild (oracle) in a fresh home on the **same**
   changed tree;
6. compute and archive the fact digests of **both** indexes **before** the
   restore, and compare them;
7. compare the query answers of both indexes on the same state;
8. only then restore the snapshot to the pin, verify it, and re-index the
   main home.

The archived pair is `incremental/delta-inc-facts.json` and
`incremental/delta-oracle-facts.json`, with the source tree identity in
`incremental/delta-tree.json` (the commit, the porcelain, the changed file's
content digest and the commit each index recorded).

## Path safety

`common.ps1` holds every guard. A destructive operation is refused unless
the **resolved** path is strictly inside the benchmark root:

- the containment test is component by component, so a sibling whose name
  starts with the benchmark root's name (`bench-evil` next to `bench`) is
  outside;
- the benchmark root itself is never a mass-delete target;
- the path is resolved **component by component**, so a junction or symlink
  at *any* component — not only at the leaf — that points outside the root
  makes the path outside;
- the pilot snapshot may only be mutated through `Restore-Snapshot`, which
  first proves that it is a git repository inside the benchmark root and is
  not the original repository (both paths resolved, so a link cannot pose as
  the snapshot).

`test-path-safety.ps1` is the regression test for those rules: a valid
path, a sibling, a parent, the root, a junction pointing outside, and a path
*through* an intermediate junction.

## Comparisons fail loudly

A comparison never reports a match it did not measure:

- a query that exits non-zero, prints nothing, or cannot be parsed aborts
  the step;
- the incremental step also asserts that the one-file change touched
  exactly one entry, that the baseline refresh was a no-op, that the
  incremental run re-parsed exactly one file, and that the snapshot was
  restored afterwards;
- the fact parity needs a digest from both sides, computed from the **same**
  source tree; a missing, empty or uncomputable digest, a differing digest
  or a differing recorded commit is a FAIL;
- the grader distinguishes `PASS`, `FAIL`, `UNMET` (a capability gap) and
  `UNAVAILABLE` (the evidence could not be obtained), and exits non-zero
  when a check fails. A parse error is never a `PASS`, an empty array is
  never proof of completeness, and an impact verdict must be exactly
  `UNKNOWN`;
- only the **verified** volatile snapshot metadata is normalized: the
  snapshot id, its creation timestamp, its nonce and its revision, and the
  registry's `added_unix`. The normalization is structural for a JSON answer,
  so no broad regex can erase a substantive difference;
- `run-all.ps1` grades **this run's** query directory, not the archived one,
  and re-makes the parity comparison from the archived digest pair of the
  same-state (changed tree) pair.

`fact-digest.test.cjs` is the regression test of the comparison: two
identical indexes pass, a content change the oracle does not have fails, an
empty or missing digest never passes, a missing output or a nonzero exit
never passes, and a substantive difference is never normalized away.

## Redaction

`redact.cjs` takes the paths to redact as arguments, so it carries no
machine-local path of its own:

```powershell
node redact.cjs <raw-dir> <archive-dir> `
  "<bench-root>=<CP2_BENCH>" "<riltra-checkout>=<RILTRA_ROOT>" "<mmsp-checkout>=<MMSP_ROOT>"
```

Each mapping also replaces the JSON-escaped forms of the path.
