# M9 CP2 (re-evaluation) — MMSP pilot execution with the clangd MCP tool

**Status: CP2 RE-EVALUATION EXECUTED — AWAIT INDEPENDENT AUDIT AND OWNER GO.**
A second, independent execution of the frozen CP2 procedure on 2026-10-21,
after the `riltra_code_semantic` integration (commit `1b162b5`) and after
the first CP2 run (2026-10-20, `docs/m9-cp2-evidence.md`).

**This document does not replace the first CP2 evidence.** The first run's
evidence and harness (`docs/pilots/cp2/`) are **frozen and untouched**; this
document records a *second* execution, its differences, and the parts the
first run could not produce (the agent side of workflow 5, and the twelfth
MCP tool).

- Branch: `m9-multirepo-release-candidate`
- Starting HEAD: `1b162b547202c8e3627e3d7c8de9b00ce3824f00`
- Pilot: MMSP `RyanAraaful/mmsp` at `d62f06a576d8e76b705c8ecc57fc380f7ebc9c44`
  (owner decision D-M9-1, unchanged), tree
  `c5482ea00481675e0aa921538f560947b68dca12`
- Design gate: `docs/m9-design-gate.md` Revision 3; CP1 spec:
  `docs/m9-cp1-pilot-spec.md` (frozen, not reopened)
- First CP2 evidence: `docs/m9-cp2-evidence.md`; archive: `docs/pilots/cp2/`
- **CP3 has already been executed** (`docs/m9-cp3-evidence.md`, verdict
  "PASS against its own scope"); its commits — including the **E8 fix**
  `54c7d26 fix(edges): widen the C/C++ caller candidate scope by unique
  include suffix` — are **ancestors of this run's HEAD** (verified with
  `git merge-base --is-ancestor`). This re-evaluation therefore measures the
  **post-CP3** code: E8 is already fixed here, E9 is already profiled by CP3,
  and the agent side of workflow 5 was already measured once by CP3.
- This run's evidence: `docs/pilots/cp2-reeval/`
- CP4, packaging, merge, tag and release were **not** started.

Status vocabulary: **PASS / FAIL / BLOCKED / UNMET / UNAVAILABLE**. A correct
`UNKNOWN`, `partial` or `POTENTIAL_CALL` is **evidence of honest behaviour,
not evidence that a capability requirement has passed**.

## At a glance

| Question | Answer |
|---|---|
| The snapshot identity | **matches CP1 exactly** — `d62f06a…`, tree `c5482ea…`, 2 277 tracked files |
| The original MMSP clone | **unchanged** (HEAD, tracked tree, porcelain, `.git` metadata) |
| The cold index | 229.46 s / 222.05 s (debug, the archived pair — §3.1; the first pair, 220.75 s / 216.72 s, is console only) — the counters are identical to the first CP2's corrected capture, 91 025 calls |
| The incremental vs full-rebuild parity | **PASS** — identical fact digests and twelve identical query answers |
| The freshness scenarios F1–F5 | **all five match** their pre-registered verdicts and reasons |
| The five workflows | 17 honesty PASS, 1 UNAVAILABLE, **1 FAIL (a stale check in the frozen grader)**, 4 capability UNMET graded (D5's capability gate is not graded; CP1 pre-registers it UNMET) |
| The MCP surface | **12 tools**, stdout pure, the classified error clean, the OD-7 health flags as approved |
| The twelfth tool on the pilot | a **classified refusal** (`provider_unavailable`) — the tree has no compilation database |
| The agent-side OpenCode session | **PASS** (the first CP2 could not produce it: a provider 403) |
| The defects found | **three harness** defects that block or degrade a clean run (the copy step, the multi-line prompt delivery, the global-config assumption — the last caused the one isolation violation), one **grader** check that is stale, the first CP2's E1/E7/E9/E12 reproduced, **E8 fixed in CP3** and active here, and one **isolation violation** (a first agent session read the original clone; its state is unchanged) |
| What is **not** claimed | G-SEM stays `NOT YET VERIFIED` for C++; M9 is not release-ready; no implementation was made |

---

## 1. Environment, snapshot and isolation

| Item | Value |
|---|---|
| OS | Microsoft Windows 11 IoT Enterprise LTSC, build 26100 |
| CPU / RAM | Intel Core i5-8250U @ 1.60 GHz, 8 logical cores / 19.9 GiB (OS-reported) |
| Filesystem | NTFS (`C:` temp workspace) |
| git / rustc / cargo | 2.45.1.windows.1 / 1.99.1 (58cf8f9c3 2026-10-13) / 1.99.1 (5f94df478 2026-08-27) |
| node | v22.23.3 (the harness's `node:sqlite` grader needs 22+) |
| Build profile | **debug** (CP1 §A.6's frozen profile; the M3/M4-comparable one) |
| Benchmark workspace | a temporary directory outside both repositories; `RILTRA_HOME` in its own subdirectory, never inside a repository tree |
| Pilot snapshot | a plain `git clone` + `checkout --detach` of the pin (CP1 §A.2, no protection override) |
| Snapshot identity | HEAD `d62f06a…`, tree `c5482ea00481675e0aa921538f560947b68dca12` — **identical to CP1's frozen value**; 2 277 tracked files (CP1 §A.3) |
| Original MMSP clone | `<MMSP_ROOT>` (the original clone, never pulled, checked out, cleaned, stashed or configured) |

**One divergence from the CP1 record, observed before the run:** the original
MMSP clone is **not** `git status --porcelain` empty as CP1 recorded. It has
two **untracked** entries — `.claude/` and `.tmp_render/` — and no modified
tracked file, so the tracked tree is unchanged. The clone's HEAD and the
pinned commit's tree are exactly as CP1 froze them
(`75b3a3f…` and `c5482ea…`). The entries are recorded before and after.

**One isolation violation, disclosed.** The first agent session (§7.4)
registered a language server of its own, pointed at the **original clone**,
and started an index build there — so the clone was **read**, which CP1
§A.2 forbids. The clone's **state** is unchanged (HEAD, the tracked tree,
the porcelain and the `.git` metadata are byte-identical to the "before"
record, `raw/isolation.json`), and the build wrote to the agent's scratch
home, not to the clone — but the rule was broken once, and the re-run
registers the pilot's server instead.

---

## 2. Method

### 2.1 The measure

The frozen harness's scripts measure with their own `Stopwatch` calls
(`cold-index.ps1:25-29`; the step runner executes the frozen steps
unchanged), and `cold-index-3.ps1:19-49` polls `PeakWorkingSet64` of the
**Riltra process** directly — that is where the 120.5 MiB comes from.
`measure.ps1` is present in the frozen method directory, but no step of this
run invokes it (it would measure a `pwsh` child, not Riltra).

The raw evidence of this run is archived in
`docs/pilots/cp2-reeval/raw/` (redacted with the frozen
`method/redact.cjs`, so no machine-local path and no credential is
committed). The frozen first-run archive (`docs/pilots/cp2/raw/`,
`raw-audit/`) is **not** touched.

### 2.2 What was measured, and what was not

Measured: the cold full index (twice, independent), the no-op incremental
refresh, the one-file incremental refresh, the independent full rebuild of the
same final tree, the fact parity between them, the warm-query latency (≥5
repetitions per query, p50/p95), the five workflows with their mechanical
grading, the freshness scenarios F1–F5, the MCP stdio smoke, the D.5 tool
sequence, a real OpenCode session, and the isolation assertions.

Not measured: a photogrammetry pipeline, a dataset, an MMSP build, a
compilation database for MMSP (the tree has none, and creating one needs a
separate owner decision — D-M9-2), CUDA semantics, and any C++ *semantic*
answer on MMSP (the tree has no provider configured, and the semantic
provider's own prerequisites are absent — §7.3).

## 3. Benchmark indexing

### 3.1 The cold full index (two independent runs, D-M9-3)

| Run | Profile | Wall | `diff` | `ingest` | `verify` | `promote` | Index bytes | Calls | Archived |
|---|---|---|---|---|---|---|---|---|---|
| cold 1 | debug | 220.75 s | 41 ms | 153 507 ms | 1 057 ms | 1 374 ms | 49 762 304 | 91 025 | console only |
| cold 2 | debug | 216.72 s | 39 ms | 150 499 ms | 1 053 ms | 1 430 ms | 49 762 304 | 91 025 | console only |
| cold 1 (repeat) | debug | **229.46 s** | 41 ms | 159 179 ms | 1 228 ms | 1 188 ms | 49 762 304 | 91 025 | `raw/cold-index-debug.txt` |
| cold 2 (repeat) | debug | **222.05 s** | 40 ms | 151 391 ms | 1 160 ms | 1 356 ms | 49 762 304 | 91 025 | `raw/cold-index-debug.txt` |

All four runs are the **same** procedure on the **same** pinned tree, in
independent homes, and they agree on every counter and on the index size.
The **repeat** pair is the archived one (`raw/cold-index-debug.txt`); the
first pair's wall times and stage timings come from the harness's console
output and are **not** archived (§10.1). A third cold index with the
indexer's own peak working set is in §3.4.

**The processing counters are identical in both runs and identical to the
corrected first CP2 capture** (`docs/m9-cp2-evidence.md` §2.2):

| Counter | Value | CP1 cross-check |
|---|---|---|
| `files.discovered` | **2 277** | = CP1 §A.3 tracked files at the pin |
| `files.processed` | **1 438** | = CP1 §A.5 "Riltra-parsed extensions" (817 `.h` + 439 `.cc` + 114 `.py` + 34 `.c` + 18 `.ts` + 14 `.cpp` + 2 `.js`) |
| `files.definitions` | 12 105 | = the first CP2's corrected capture |
| `files.imports` | 6 680 | = the first CP2's corrected capture |
| `files.calls` | **91 025** | = the first CP2's **corrected** capture (the pre-fix count was 89 819) |
| `files.errors` | 1 991 | = the first CP2's corrected capture |
| `files.skipped_binary` | 4 | — |
| `files.unsupported` | 835 | = (489 CUDA + 350 other) − 4 binary; **every `.cu`/`.cuh` is counted unsupported and none is parsed** |
| `skipped_ignored / large / symlink / escape / unreadable` | 0 | — |

**The first-party vs third-party split** (CP1 §A.4's rule: everything
under `colmap/src/thirdparty/` is third-party) is measured from the index
read-only (`raw/first-party-split.json`): of the **1 438 indexed** files,
**813 are first-party** and **625 are third-party** — the same two numbers
the first CP2 recorded. The per-class *fact* counts (8 015 / 4 090
definitions, 4 491 / 2 189 imports, 75 900 / 15 125 calls, **55 of 813**
first-party files with parse errors) are the first CP2's, not re-derived
here; the report does not claim they were re-measured.

**The index size differs from the first run's corrected capture** (49 762 304
here vs 50 238 592 there) although the **facts are identical** (the same
counters, and the fact digests match, §5). The difference is SQLite's
internal layout of the same content, not a different fact set.

### 3.2 The no-op and the one-file incremental refresh

| Run | Mode | Wall | `reused` | `reparsed` | `added` | `removed` | Facts |
|---|---|---|---|---|---|---|---|
| baseline (no change) | incremental | 70.2 s | **1 438** | **0** | 0 | 0 | unchanged (12 105 / 6 680 / 91 025 / 1 991) |
| one-file change | incremental | 65.3 s | **1 437** | **1** | 0 | 0 | unchanged (12 105 / 6 680 / 91 025 / 1 991) |

The one-file change is `colmap/src/colmap/mvs/patch_match_options.h` (one
line), and the refresh re-parsed **exactly one file** — the M4 precedent's
behaviour (11 764 reused / 1 reparsed).

### 3.3 The independent full rebuild of the same final tree (the oracle)

| Run | Mode | Wall | `reused` | `reparsed` | Index bytes | Facts |
|---|---|---|---|---|---|---|
| oracle | full | 212.3 s | 0 | 1 438 | 49 762 304 | unchanged (12 105 / 6 680 / 91 025 / 1 991) |

The oracle is a `--full` rebuild into a **fresh home** of the **same
one-file-changed tree** (the incremental run's tree identity, its changed
file and its SHA-256 are archived). The operational counters differ by mode
(`reused` 1 437 vs 0) — exactly the split CP1 §A.7 point 4 prescribes — and
the **fact** counters are the same. §5 is the parity verdict.

### 3.4 The peak working set and the load condition

| Measure | This run | The first CP2 (corrected capture) |
|---|---|---|
| cold index, debug | **229.46 s / 222.05 s** (the archived pair) | 160.0 s |
| cold index, **release**, the indexer's own peak RSS | **37.4 s, 120.5 MiB** (126 312 448 B) | 23.8–26.1 s, **120.3 MiB** |
| no-op refresh, debug | 70.2 s | 48–52 s |
| warm query p95, debug | 0.33–1.46 s (the eleven queries that missed the target: 0.64–1.46 s) | 0.52–1.09 s (release) |
| the index size | 49 762 304 B | 50 238 592 B (the same facts — §3.1) |

**The indexer's peak working set reproduces the first CP2 to 0.2 MiB**
(120.5 MiB here, 120.3 MiB there) — the same indexer code path, the same
tree, the same host, an independent run. (The binaries are not identical:
this run is the post-CP3 code, with the MCP-tool addition and CP3's E8
fix.)

**The wall times are reported per run with their load condition, never as a
single mean** — the first CP2 established that this host's variance is large
(an independent debug rebuild took 336.5 s under contention, and the release
cold index measured 46.7 s under contention but 23.8 s idle). This run's
debug cold index (**229.46 s / 222.05 s**, the archived pair) is **above**
the first run's idle range (160.0 s) and inside its contention range; the
release cold index measured 37.4 s here against 23.8–26.1 s there. The
tree, the indexer and the host are the same; the **binary is not** (this run
is the post-CP3 code, with the MCP-tool addition and CP3's E8 fix), and the
host's state is the difference that the numbers show. (The first pair's wall
times, 220.75 s / 216.72 s, are console-only and not archived — §10.1.)

### 3.5 The warm-query latency (≥5 repetitions per query, debug)

| Query | p50 | p95 | max | vs the 500 ms p95 target |
|---|---|---|---|---|
| `search --symbol PatchMatch` | 648.2 ms | 649.3 ms | 654.0 ms | missed, 1.30× |
| `search --symbol Run` | 656.2 ms | 658.5 ms | 662.3 ms | missed, 1.32× |
| `search --symbol Print` | 648.4 ms | 650.4 ms | 658.0 ms | missed, 1.30× |
| `search --symbol normalize_vec` | 651.7 ms | 665.1 ms | 665.8 ms | missed, 1.33× |
| `search --symbol max-triang` | 645.2 ms | 648.4 ms | 649.6 ms | missed, 1.30× |
| `callers` (Print definition) | 1 249.0 ms | 1 261.0 ms | 1 288.4 ms | missed, 2.52× |
| `callees` (PatchMatch::Run) | 1 191.8 ms | 1 192.5 ms | 1 206.7 ms | missed, 2.39× |
| `callees` (CtrlRun) | 1 417.7 ms | 1 464.5 ms | 1 584.7 ms | missed, 2.93× |
| `trace --depth 4` | 1 224.7 ms | 1 228.4 ms | 1 229.2 ms | missed, 2.46× |
| `impact` (option member) | 1 003.5 ms | 1 004.6 ms | 1 068.8 ms | missed, 2.01× |
| `index status` | 634.5 ms | 636.2 ms | 637.2 ms | missed, 1.27× |
| `repo status` | 328.6 ms | 331.3 ms | 337.9 ms | met |

**The blueprint's 500 ms p95 target is missed by 1.27×–2.93× in the debug
profile.** This is the first CP2's **E9**, reproduced. **CP3 has since
profiled it** (`docs/m9-cp3-evidence.md` §9): the fixed cost is ~472 ms of a
~760 ms invocation and is dominated by the `git` fingerprint subprocess
(~272 ms) — so the cause is no longer a hypothesis, and this run's numbers
(the same shape: `repo status` 0.33 s at p50, `index status` 0.63 s, the
graph queries 1.0–1.4 s) are consistent with CP3's profile. **The profile
was not repeated here**, so this run contributes the reproduction, not a new
measurement of the split. The target is a diagnostic, not a release gate
(blueprint §14.3), and changing it is a blueprint decision, not a CP2
decision.



The procedure is the one frozen in CP1 §A.2 and §A.7 and implemented by the
**frozen CP2 harness** (`docs/pilots/cp2/method/`), re-run as-is:

1. the harness self-tests (the grader and the digest comparison must be able
   to fail; the path-safety guard must hold);
2. two independent cold full indexes (D-M9-3);
3. the workflow queries of CP1 §D and their mechanical grading;
4. the freshness scenarios F1–F5 of CP1 §C;
5. the one-file delta, the independent full rebuild of the same tree, the fact
   digests of both and the query parity;
6. the warm-query timings;
7. the MCP stdio smoke, the D.5 tool sequence and a real OpenCode session;
8. the isolation assertions.

**What this run adds** to the first CP2: the twelfth MCP tool
(`riltra_code_semantic`) on the pilot, and the **agent side of workflow 5**,
which the first run could not obtain (a provider 403 — §7.4).

## 4. The five workflows — honesty and capability, graded mechanically

The grader is the frozen `docs/pilots/cp2/method/grade.cjs` with its
regression tests (`grade.test.cjs`, 21 tests — the file declares 21; their
output for this run is **not archived**, §10.1). It grades the **same** CP1
§D criteria, and it refuses to turn missing evidence into a pass: a parse
error is `UNAVAILABLE`, an empty edge list is not evidence for the label
check, a capability gap is `UNMET` and a missing capability evidence is
`UNAVAILABLE` — never mixed.

| Workflow | Honesty gate | Capability gate | Evidence |
|---|---|---|---|
| 1. Find a symbol and its references | **PASS** (5/5) | **UNMET** | — |
| 2. Callers and callees | **UNAVAILABLE** (4 PASS, 1 UNAVAILABLE) | **UNMET** | — |
| 3. Trace a call path | **PASS** (3/3) | **UNMET** | **UNAVAILABLE** (`cuda-unsupported`) |
| 4. Impact of a change | **PASS** (3/3) | **UNMET** | — |
| 5. Natural-language question via OpenCode/MCP | **FAIL** (stale grader check — §4) | **UNMET** (pre-registered; the gate has no grader check — §4.5) | **PASS** (the agent-side session ran) |

**Totals over the 25 graded checks: 17 honesty PASS, 1 honesty UNAVAILABLE,
1 honesty FAIL, 4 capability UNMET, 1 evidence UNAVAILABLE, 1 evidence
PASS.** The D5 **capability** gate has no grader check (`n/a` in
`raw/grading.json`); CP1 §D.5.6 pre-registers it as **UNMET**, and §4.5
states it that way. **The honesty gate is not fully met**: `D5-H1` is a FAIL
and `D2-H5` is UNAVAILABLE. The one **FAIL** is a **stale check in the
frozen grader**, not a product defect: `grade.cjs:384` requires
`toolCount === 11`, the surface has been **12** since the twelfth tool was
merged, so the check cannot pass for the current server. The facts the
check tests all hold (the stdout is pure JSON-RPC, stderr is empty, the
calls answered, and the answer carries the repository identity) — they are
verified in §7.1. The grader was **not** modified, and the acceptance
criteria were not lowered. The per-check verdicts and their reasons are in
`docs/pilots/cp2-reeval/raw/grading.json`; the query answers they grade are
in `raw/queries/`.

### 4.1 Workflow 1 — find a symbol and its references

- **D1-H1 PASS**: the search for `Print` resolves the requested definition
  and invents none — the answer carries `PatchMatch::Problem::Print` at
  `patch_match.cc:118` and the other two `Print` methods at their own
  locations.
- **D1-H3 PASS**: `normalize_vec` resolves to the Python definition
  (`colmap/benchmark/reconstruction/evaluation/geometry.py:34`) and to **no**
  C++ definition.
- **D1-H4 PASS**: the bare name `Run` returns multiple distinct identities
  with distinct locations, not one merged symbol.
- **D1-H5 PASS**: the answer states the repository (`mmsp-pin`) and a
  freshness verdict (`fresh`).
- **D1-H2 PASS**: every returned reference edge is a candidate
  (`POTENTIAL_CALL`, `precision: Heuristic`, `verification: Unverified`),
  never a proven `CALLS` edge.
- **D1-C6 UNMET** (`reference-set-empty-or-unresolved`): the reference set
  returns **no edges** at all, and therefore does **not** resolve the C++
  method identity — it cannot separate `patch_match.cc:667` from `:668` and
  `fusion.cc:144`. The capability gate stays as CP1 §D.1.6 states it; the
  pilot does not meet it. **Not lowered.**

### 4.2 Workflow 2 — callers and callees

- **D2-H1, H2, H3, H4 PASS**: the answer states what it could not resolve
  and labels every reported edge as a candidate; the callee set contains the
  `PatchMatchCuda` construction and the `Run` call; the `ThreadPool`
  dispatch to `ProcessProblem` is never a proven `CALLS` edge; the
  `CreateWorkspace` references stay inside `patch_match.cc`.
- **D2-H5 UNAVAILABLE** (`no-reference-returned`): the query reports **no**
  edge for this anchor, so there is no reported edge to inspect. The
  criterion is neither confirmed nor refuted — the same honest `UNAVAILABLE`
  the first CP2 recorded, for the same reason.
- **D2-C6 UNMET** (`construction-sites-not-attributed`): the three external
  construction sites are **not all** attributed to the class
  `PatchMatchController` — **1 of 3 is** (the edge at
  `colmap/src/colmap/controllers/automatic_reconstruction.cc`, a
  `POTENTIAL_CALL` candidate). The first CP2 attributed **0** of 3, because
  it ran before CP3's E8 fix (`54c7d26`, the unique-include-suffix rule,
  which is in this run's code). The two still missing are the sites CP3
  names: **`exe/mvs.cc:271`** (the **E12** most-vexing-parse form — C++ reads
  it as a function declaration, so no call fact exists) and
  **`ui/dense_reconstruction_widget.cc:397-398`** (**E1/E7** — the callee
  terminal is `make_shared`, not the class). Both translation units' includes
  **do** resolve under CP3's suffix rule (`docs/m9-cp3-evidence.md` §2.4);
  the two sites are missing for the reasons above, not for an unresolved
  include. The capability gate stays as CP1 §D.2.5 states it; the pilot does
  not meet it. **Not lowered.**

### 4.3 Workflow 3 — trace a call path

- **D3-H2/H3/H4 PASS**: the plausibility disclaimer is present, the path
  does not claim a device-side definition, and the CUDA destination query
  **fails cleanly** instead of inventing a path (`d3-2-trace-to-cuda` exits
  3 with a classified error).
- **D3-C5 UNMET** (`trace-does-not-extend-beyond-the-first-hop`): the path
  does not span the frozen four-hop chain.
- **D3-E1 UNAVAILABLE** (`cuda-unsupported`): the CUDA terminus cannot be
  verified. The **tool's own message** is `error: anchor unresolved:
  source-changed; reindex before syntax fallback`; the **grader** separates
  that *observed* reason from the *actual cause*, `cuda-unsupported` (the
  `.cu` file is not indexed). The grader's classification is recorded here
  because the grader is the frozen mechanical record; the tool's literal text
  is in `raw/queries/d3-2-trace-to-cuda.json`.

### 4.4 Workflow 4 — impact of a change

- **D4-H2/H3/H4 PASS**: the verdict is exactly `UNKNOWN` (never
  `SAFE`/`LOW`), the completeness is `partial` with
  `graph-completeness-unproven` and the unresolved reasons, and the two
  macro sites are not resolved to function definitions.
- **D4-C5 UNMET** (`affected-set-empty`): the affected set does not contain
  the declaration, print, validation, read/filter and CLI-binding sites —
  the expected syntax-only shape (the first CP2's gap E1/E7: a member read,
  an address-of binding and a macro argument are not call expressions, so the
  member has no incoming edge at all).

### 4.5 What the five workflows mean together

**The honesty gate is met wherever the evidence exists to judge it** — 17 of
the 18 judgeable honesty checks pass; `D2-H5` is `UNAVAILABLE` (no edge is
returned, so there is no edge to inspect) and `D5-H1` is the one FAIL (the
stale grader check, §7.4). **Four CP1 capability gates are UNMET as measured here**; the fifth (D5) has
no grader check and stays the **pre-registered** UNMET — the first CP2 could
not assess it at all (it recorded `UNAVAILABLE`, the agent side blocked).
The four measured gaps are not lowered: the gap is measured, not assumed.

---

## 5. Incremental vs full rebuild — the parity contract

CP1 §A.7's five-point contract, applied to the pair built in §3.2/§3.3 (the
same one-file-changed tree, indexed incrementally and rebuilt from scratch
into a fresh home):

| Contract point | Result |
|---|---|
| 1. the same source tree and the same configuration | **held** — the changed tree's identity is archived in `raw/incremental/delta-tree.json` (the HEAD, the porcelain and the changed file's content digest) and both indexes are built from it |
| 2. fact parity | **held** — the fact digests are **identical** (below) and all twelve query answers are identical |
| 3. the total fact counters on the same basis | **held** — 1 438 / 12 105 / 6 680 / 91 025 / 1 991 in both |
| 4. the operational counters per mode, not forced equal | **held** — `reused` 1 437 (incremental) vs 0 (full), `reparsed` 1 vs 1 438, `fallback_reason` `none` vs `forced-full`; the difference is the mode, and it is explained |
| 5. any unexplained fact difference is a FAIL | **none** — the fact digests match exactly |

The fact digest (the harness's order-independent digest, with only the
volatile snapshot metadata normalized):

```
files     rows    1438 vs    1438  identical = True
symbols   rows   12105 vs   12105  identical = True
imports   rows    6680 vs    6680  identical = True
calls     rows   91025 vs   91025  identical = True
edges     rows   97705 vs   97705  identical = True
incremental combined = 2389cd32e702b479e48b0b3a9ca5e452aa379406058b063b79e619dfcc18756c
oracle      combined = 2389cd32e702b479e48b0b3a9ca5e452aa379406058b063b79e619dfcc18756c
IDENTICAL: files,symbols,imports,calls,edges
```

And the **query parity** on the same tree — twelve answers compared after
the volatile-metadata normalization, all identical:

`search --symbol PatchMatch` · `search --symbol Run` ·
`search --symbol normalize_vec` · `search --symbol max_triangulation_angle`
· `callers` at the `Print` definition · `callees` at `PatchMatch::Run` ·
`callees` at `PatchMatchController::Run` · `callers` at `CreateWorkspace` ·
`callers` at the Python `normalize_vec` · `trace --depth 4` · `impact` at the
option member · `index status`.

**The parity verdict is PASS.** The harness's own regression tests
(`fact-digest.test.cjs`, 16 tests) prove that this comparison *can* fail — a
content change, a changed edge set, a different row count, an empty digest,
a missing digest, indexes from different source trees and a missing
source-tree record all fail it — so the PASS is not vacuous.

---

## 6. Freshness F1–F5 and the isolation evidence

The scenarios are CP1 §C's, with their **pre-registered** expectations, run
inside the disposable snapshot only; the snapshot is restored to the pin
between scenarios.

| # | Scenario | Pre-registered expectation | Measured | |
|---|---|---|---|---|
| F0 | baseline (restored pin) | — | `fresh`, no reasons | — |
| F1 | file modification (the default `-1.0f` → `60.0f` in `patch_match_options.h`) | `stale`, reason `content-changed`, `dirty` also expected | `stale`, reasons **`dirty`, `content-changed`** | **matches** |
| F2 | file rename (`patch_match_cuda.h` → `patch_match_cuda_renamed.h`) | `stale`, reasons `file-removed` **and** `file-added` | `stale`, reasons **`dirty`, `file-added`, `file-removed`** | **matches** |
| F3 | file deletion (`git rm patch_match_options.cc`) | `stale`, reason `file-removed` | `stale`, reasons **`dirty`, `file-removed`** | **matches** |
| F4 | HEAD change (`checkout --detach 2b1bf42…`) | `stale`, reason `head-changed` | `stale`, reasons **`head-changed`** (no `dirty`: the tree is clean) | **matches** |
| F5 | branch change (`checkout -b m9-freshness-probe`, the same commit) | `stale`, reason `branch-changed`, **not** `head-changed` | `stale`, reasons **`branch-changed`** (the HEAD is still `d62f06a…`) | **matches** |

**No silent-stale:** every scenario that changed an input reported `stale`
with a reason; the one scenario that changed nothing reported `fresh`. No
scenario reported `fresh` while its inputs differed — the STOP condition of
the design gate's G-FSH was not triggered.

**The isolation evidence** (recorded before and after the whole run):

| Assertion | Before | After |
|---|---|---|
| the snapshot's HEAD | `d62f06a576d8e76b705c8ecc57fc380f7ebc9c44` | `d62f06a…` (restored after F5 and after the incremental step) |
| the snapshot's tree | `c5482ea00481675e0aa921538f560947b68dca12` | `c5482ea…` |
| the snapshot's `git status --porcelain` | 0 lines | **0 lines** |
| the original MMSP clone's HEAD | `75b3a3f5e89b080de3356a3fae07d572d0ee9cf8` | `75b3a3f…` |
| the original MMSP clone's `git status --porcelain` | 2 lines (`.claude/`, `.tmp_render/` — untracked, pre-existing) | **2 lines — the same two** |

**No cross-repository contamination:** the registry of the pilot home holds
exactly one repository (`mmsp-pin`), every answer carries
`"repo":"mmsp-pin"` and the snapshot's root, and the MCP smoke's
`riltra_repo_list` returns only the pilot.

**The unsupported inputs stay unsupported:** 835 files are counted
unsupported (489 CUDA + 350 other extensions − 4 binary), the CUDA
terminus of the trace is refused (the tool's own message is
`error: anchor unresolved: source-changed; reindex before syntax fallback`;
the frozen grader classifies the cause as `cuda-unsupported`), and **no
CUDA definition, reference or call is claimed anywhere** in the answers.

**No heuristic is promoted:** every reported edge in the graded answers is a
`POTENTIAL_CALL` candidate with `precision: Heuristic` and
`verification: Unverified`; the impact verdict is exactly `UNKNOWN`. D1-H2
and D2-H3 grade exactly those two properties, and both **PASS**.

---

## 7. The MCP surface and the OpenCode interoperability

### 7.1 The tool catalog — twelve tools

The MCP stdio smoke (the frozen `mcp-smoke.cjs`, over the real transport):

| Check | Result |
|---|---|
| the tool surface | **12** — `riltra_code_callees`, `riltra_code_callers`, `riltra_code_impact`, `riltra_code_search`, **`riltra_code_semantic`**, `riltra_code_trace`, `riltra_health`, `riltra_knowledge_related`, `riltra_knowledge_search`, `riltra_knowledge_show`, `riltra_repo_list`, `riltra_repo_status` |
| stdout purity | 6 of 6 lines parsed as JSON-RPC, **0 non-JSON lines** |
| stderr | empty (0 bytes) |
| a source-grounded query | `riltra_code_search normalize_vec` returned the expected document (freshness `fresh`, `repo: mmsp-pin`, the Python definition at `geometry.py:34`) |
| repository selection | `riltra_repo_list` returns exactly the pilot repository; `riltra_repo_status` (in the D.5 sequence, `raw/mcp/d5-tool-sequence.json`, and in the `d0-2` query) reports HEAD `d62f06a…`, dirty `false` |
| a classified error | an unknown repository → `{"kind":"not_registered","message":"unknown repository: 'does-not-exist'"}` with `isError: true`, transport healthy |
| the health document | `read_only: false`, `mutations: true`, `network: true`, the twelve tool names — the owner's OD-7 flags, as approved |
| **the store's read side** | `home_unchanged: **false**` — the store's **content** digest is unchanged, but the read creates the `-wal`/`-shm` sidecars (the main database file's hash is unchanged; `raw/mcp/home-digest-after.txt` adds the two sidecars). The first CP2 recorded `home_unchanged: true`; the difference is the read-side WAL artifact, the accepted L2 limitation (`docs/m8-design-gate.md:221-227`). The store was **not** migrated and no data changed |

### 7.2 The D.5 tool sequence (scripted, over the real MCP transport)

The natural-language question of CP1 §D.5 — *"how is the
max_triangulation_angle option wired from the command line into the
source-image selection, and which files would I need to touch to change its
default?"* — was turned into the tool sequence an agent would run and
executed against the live MCP server (nine calls, archived verbatim in
`raw/mcp/d5-tool-sequence.json`):

| Step | Tool | Result |
|---|---|---|
| 1 | `riltra_code_search max_triangulation_angle` | 0 hits — the data member is not an indexed symbol (reported, not guessed) |
| 2 | `riltra_code_search PatchMatchOptions` | the struct in `option_manager.h:59`, the struct in `patch_match_options.h:37`, and its `Print`/`Check` methods |
| 3 | `riltra_code_callers` at the member | **2 candidate edges** (`callee_name: PatchMatchOptions`); the member's read/filter sites are not among them, and the anchor uncertainty is reported |
| 4 | `riltra_code_callers` at `option_manager.cc:1019:41` | 0 edges, the anchor uncertainty reported |
| 5 | `riltra_code_impact` at the member | `UNKNOWN`, 0 paths, `partial`, the unresolved reasons, the disclaimer present |
| 6 | `riltra_code_callees` at `ReadProblems` | 30+ candidates, all heuristic |
| 7 | `riltra_repo_list` | the pilot repository only |
| 8 | `riltra_repo_status` | HEAD `d62f06a…`, dirty `false` |
| 9 | `riltra_code_callers` on an unknown repo | classified error, `isError: true` |

An agent using these tools learns the option's *type* and its `Print`/`Check`
methods, and correctly reports that the member's callers and its impact are
**not** resolvable — the honest answer, and the capability gap of §4.2/§4.4.

### 7.3 The twelfth tool on the pilot

`riltra_code_semantic` was exercised on the pilot over the real transport
(`raw/semantic-tool/semantic-tool.json`):

| Check | Result |
|---|---|
| the tool is in the catalog | **yes** — 12 tools, `riltra_code_semantic` present |
| **no provider configured** | `{"kind":"provider_unavailable","message":"no language server configured; set RILTRA_LSP_SERVER to an absolute path"}` — a **classified refusal**, and the eleven other tools are unaffected |
| the check order | a path that escapes the repository is refused `invalid_input` ("non-canonical document path: '../escape.cc'") **before** the provider is looked up |
| an unknown `operation` | refused by the SDK's schema layer, before the tool |
| the other tools still answer | `riltra_code_search` returned the pilot's document — the re-run's counter reads **29** hits and the raw stdout agrees; the first run's counter read 0 while its stdout showed results, and that anomaly is **not explained** (§10.1) |
| stdout purity | 6 lines, **0 non-JSON-RPC** |

**This is the honest answer for the pilot**: the tree has **no compilation
database** and no provider is configured, so the semantic tool cannot answer
a C++ question on MMSP — and it says so, with a classified refusal, instead
of returning an empty answer that could be read as "the symbol does not
exist". Producing a compilation database for MMSP is a separate owner
decision (D-M9-2) and was **not** done here.

### 7.4 A real OpenCode session

The first CP2 could not run the agent side: the OpenCode free tier refused
nested CLI sessions (provider 403) and the verdict was `UNAVAILABLE`. **The
block did not reproduce with `opencode/minimax-m3.1-flash-free`** — the
session below ran to completion — but the cause of the 403 is **not
established**, so it is not called "model-specific": it is the same
environmental block CP3 recorded as "no longer blocked — measured in CP3".

Two harness defects had to be worked around in the agent session (the
multi-line prompt through a generated `.cmd`, and the global-config
expectation; neither is a product defect, and the frozen harness was not
modified — §8.1 and §7.4):

- the frozen `opencode-session.ps1` passes a **multi-line** prompt through a
  generated `.cmd`, and the prompt never reaches the agent ("No question
  came through in your message"); the run then answers with a generic
  orientation instead of the D.5 question;
- the frozen script expects the Riltra MCP server to be registered in the
  **global** OpenCode configuration. It is not, so the agent registered a
  server **of its own** — pointed at the **original MMSP clone**, and it
  started an index build there. The clone's *state* is unchanged (HEAD, the
  tracked tree, the porcelain and the `.git` metadata are byte-identical to
  the "before" record), but the clone was **read**, which the CP1 isolation
  rule forbids. The re-run below registers the pilot's server through a
  **project-local** configuration; the run did not observe the agent
  registering another server.

**The re-run** (the frozen prompt, delivered as one argument, against the
pilot's server):

| Measure | Result |
|---|---|
| the session | **exit 0**, 355.6 s, 462 542 B of stdout, **0 B of stderr** |
| the tools it used | 22 `riltra_*` matches, **12 distinct tools** (10 of the 22 are doubled-prefix artifacts of the archived script's regex) |
| Task 1 (the D.5 question) | answered from tool results, with repository-relative `path:line` citations throughout (`cited_sources: true`; the archived summary counts 21 of them, counted from the archived answer text) |
| Task 2 (the unknown repository) | one call, no retry, the classified error quoted exactly: `{"kind":"not_registered","message":"unknown repository: 'mmsp-nonexistent-repo'"}` |
| the repository and the freshness | both stated (`mmsp-pin`; `index_status.freshness: "fresh"`, and every call's `freshness: "fresh"`) |
| the limits it stated | "`max_triangulation_angle` could not be located … it is simply invisible to Riltra's name-indexed `code_search` and to `code_semantic` (LSP unconfigured)"; "The call graph returned `verification: "Unverified"` with `completeness: "partial"` throughout … treat the wiring map above as structurally accurate but not exhaustively proven" |

**The agent-side verdict is PASS** (`D5-E1`, the evidence gate) and the
**source-citation check is PASS** (`D5-H3`) — the first CP2 could produce
neither, because the free tier refused nested sessions. The honesty layer is
what the criterion asks for: the agent **states the limits it hit** ("no
indexed symbol anywhere selects 'source images' by triangulation angle", "I
cannot name its files", "not exhaustively proven"), it does **not** claim a
C++ semantic resolution, and it quotes the tools' own output.

**What the answer also shows**: the twelfth tool is *usable* by an agent —
the agent called it and reported, correctly, that it could not answer because
no provider is configured.


## 8. Defects and observations (collected, not fixed)

Nothing was fixed in this task: the brief is evaluation-only, and every
defect below is **collected as evidence** for the owner's decision.

### 8.1 A defect in the frozen CP2 harness (new, and it blocks a clean run)

`docs/pilots/cp2/method/run-all.ps1` step 2 does:

```powershell
$homeMain = Reset-BenchDir (Join-Path $bench 'home-main') -Root $bench
Copy-Item (Join-Path $bench 'home-cold1') $homeMain -Recurse -Force
```

`Reset-BenchDir` **creates** the directory (`common.ps1:113`), and
`Copy-Item -Recurse` into an existing directory copies the *source folder
into* it — so the registry lands at `home-main/home-cold1/registry.json` and
every subsequent command answers `unknown repository: 'mmsp-pin'` with exit
3. In this run that made the first `run-all.ps1` attempt produce **no usable
evidence**: the workflow queries, the freshness scenarios and the incremental
step all failed, and the grader correctly refused to turn the missing
evidence into a pass (16 honesty `UNAVAILABLE`).

The harness was **not modified** (it is frozen evidence). The run was
repeated with the same frozen steps in the same order, using the **verified
cold run 1 home directly** instead of the broken copy
(`docs/pilots/cp2-reeval/method/run-steps.ps1`). Everything else — the
self-tests, the queries, the grader, the freshness scenarios, the
incremental step, the parity comparison, the timings, the MCP smoke — is the
frozen harness, unchanged.

**Owner decision:** the copy step needs a one-line fix
(`Copy-Item "$bench/home-cold1/*" $homeMain -Recurse -Force`, or
`Copy-Item $src $homeMain -Recurse -Force` *before* the directory is
created). CP2 must not change the harness without an owner GO, so the fix is
**not** applied here; the re-evaluation works around it and the workaround is
archived.

### 8.2 The product gaps carried over from the first CP2 (unchanged)

| # | Gap | Class | This run's evidence |
|---|---|---|---|
| E1 | C++ semantics absent: no provider and no compilation database in MMSP | unsupported capability (owner-gated, D-M9-2) | unchanged: 91 025 calls are all `POTENTIAL_CALL` candidates; the twelfth MCP tool returns the classified refusal `provider_unavailable` on the pilot (§7.3). CP3's §9 records the same status |
| E7 | reference-level resolution absent (a data member, a member read and a macro argument produce no edge) | unsupported capability (owner-gated) | unchanged: D2-C6 and D4-C5 are `UNMET`, D3-C5 is `UNMET`. CP3's §9 calls E7 "the highest-value remaining gap" and asks for a decision |
| E8 | C/C++ include-root resolution | **implementation defect — fixed in CP3** (Option A, narrowed) | the fix (`54c7d26`, the unique-include-suffix rule) is an **ancestor of this run's HEAD** and is in the measured code: D2-C6 now attributes **1 of 3** construction sites, against **0** in the first CP2's pre-fix run. **This is the one gap that changed** |
| E9 | the warm-query p95 target (≤ 500 ms) is missed by 1.27×–2.93× in the debug profile | diagnostic, not a gate | reproduced; **CP3 has profiled it** (the fixed cost ≈ 472 ms of a ≈ 760 ms invocation, dominated by the `git` fingerprint subprocess ≈ 272 ms) — the profile was not repeated here (§3.5) |
| E12 | the C++ most-vexing-parse form cannot be decided from syntax | unsupported capability (needs the name lookup of E1/E7) | unchanged: the call count is the corrected 91 025. CP3's §9 adds that one of the three `PatchMatchController` construction sites is unreachable for this reason alone — the two sites D2-C6 still misses |
| E10 | the OpenCode free tier refused nested CLI sessions in the first CP2 (provider 403) | environmental block | **not reproduced** with `opencode/minimax-m3.1-flash-free` (§7.4); the cause is not established, and CP3 already measured the agent side once |
| — | **`.inl` headers are not indexed** | G-SYN coverage gap, found in CP3 | not re-measured here; it is CP3's finding (40 of the pilot's 47 unmatched includes are `.inl` files) |

### 8.3 New observations from this run

| # | Observation | Evidence |
|---|---|---|
| N1 | The original MMSP clone is **not** `git status --porcelain` empty as CP1 recorded: it has two untracked entries (`.claude/`, `.tmp_render/`). No tracked file is modified, and its HEAD and the pinned tree are exactly as CP1 froze them. The entries pre-date this run (they were present in the "before" record). | §1, §6 |
| N2 | The index **file size** differs from the first CP2's corrected capture (49 762 304 vs 50 238 592 bytes) although the **facts are identical** (the same counters and the same fact digests). The difference is SQLite's internal layout of the same content. | §3.1, §5 |
| N3 | The `run-all.ps1` copy defect (8.1) is a **harness** defect, not a product defect: the product answers every query correctly once the home is right. | 8.1 |
| N4 | The warm-query figures on this host are **load-sensitive**: the same query differs by up to 36 % between runs on an idle host (the first CP2's measurement, `docs/m9-cp2-evidence.md` §6; this run does not repeat the variance study). No single mean is quoted. | §3.4, §3.5 |
| N5 | The **twelfth tool** is usable by an agent and honest when it cannot answer: the agent called it and reported that it could not resolve the member because no provider is configured. | §7.3, §7.4 |
| N6 | The frozen **grader** is stale in one check (`toolCount === 11`) and the frozen **harness** has three defects that block or degrade a clean run (§8.1's copy step and §7.4's prompt delivery and global-config assumption). All three are harness-side; the product answers correctly. | §7.4, §8.1 |

### 8.4 The internal review

The report and the evidence were audited by an independent reviewer
(`opencode/deepseek-v4.1-flash-free#high`), read-only, in five rounds. The
rounds are recorded here because the corrections they forced are part of the
evidence's quality:

| Round | Verdict | What it changed |
|---|---|---|
| 1 | **NO-GO** (7 High, 11 Medium, 9 Low) | the honesty totals hid the FAIL; the report was stale relative to **CP3** (which had already run, with E8 fixed); a machine-local path was committed; D2-C6's attribution was wrong; the smoke's `home_unchanged: false` was omitted; the wall times were not archived; D5's capability gate was mislabelled |
| 2 | **NO-GO** (3 High, 7 Medium, 3 Low) | the archive carried an **unredacted** machine-local path (the JSON-escaped form escaped the redaction); §10.1 both denied an archived artefact and declared two unarchived measurements archived; the D2-C6/E12 attribution still swapped the two sites |
| 3 | **NO-GO** (2 High, 1 Medium, 3 Low) | the archived repeat pair's `verify`/`promote` timings were mis-transcribed; §4.5 overstated the capability baseline; §2.1 misdescribed the measurement method |
| 4 | **NO-GO** on documentation only | the remaining structure and cross-references (the missing §8 heading, two stale cross-references, the unarchived pair quoted as "this run") |
| 5 | **the two corrected passages re-read against the raw evidence; nothing blocks the commit** | — |

**The reviewer's final verdict, quoted verbatim:**

> **Independent re-read (final round): the corrected §3.1 repeat pair —
> `verify` 1 228 ms / 1 160 ms and `promote` 1 188 ms / 1 356 ms at
> `docs/m9-cp2-reevaluation.md:127-128` — matches
> `docs/pilots/cp2-reeval/raw/cold-index-debug.txt:5,18` exactly, and §4.5's
> "Four CP1 capability gates are UNMET as measured here; the fifth (D5) has
> no grader check and stays the pre-registered UNMET" matches
> `docs/pilots/cp2-reeval/raw/grading.json:11-13,81-84`. §2.1's method
> description also matches the frozen scripts (`cold-index.ps1:25-29`,
> `cold-index-3.ps1:19-49`; `measure.ps1` is invoked by no step of this
> run).**
>
> **No finding in either corrected passage blocks the commit: the two High
> documentation defects from the previous round are closed against the raw
> evidence.**

The reviewer's own limits, recorded: it could not run `git`, `cargo` or
`node --test`; it re-read only the corrected passages in the last round; and
the first cold pair's wall times remain unverifiable by design (console
only, §10.1).

---

## 9. Recommendation for the owner — no implementation


The minimum that would move the gates, in the order the evidence supports.
**Nothing here is implemented**; each item is a proposal for the owner's
decision. **CP3 has already decided some of them** — this list is what
remains *after* CP3, not a re-run of CP3's list.

1. **Fix the CP2 harness's copy step** (8.1) — one line, harness-only, no
   product behaviour. Without it the next CP2 run produces no evidence. The
   grader's stale `toolCount === 11` check needs the same treatment.
2. **Decide E7** (reference-level resolution) — CP3's §9 calls it "the
   highest-value remaining gap", and this run's evidence agrees: the member's
   read and filter sites produce no edge, so `impact` on a field is empty
   and the agent could not name the files it would have to touch.
3. **Decide E9** (the warm-query cost) — **CP3 has already profiled it** (the
   `git` fingerprint subprocess, ≈272 ms of a ≈760 ms invocation). The
   remaining decision is whether to reduce the fixed cost (for example by
   reusing the snapshot fingerprint) or to restate the blueprint's 500 ms
   target for a ~2 000-file C++ repository. The target is a blueprint
   decision, not a CP2 decision.
4. **Decide E12 and E1** (the C++ semantic scope) — the twelfth MCP tool
   is implemented and works, but the pilot tree has **no compilation
   database**, so it answers `provider_unavailable` there. Any C++ semantic
   claim on MMSP needs a compilation database, which is a separate owner
   decision (D-M9-2) and must not be produced by Riltra.
5. **Decide the `.inl` coverage gap** (CP3's finding) — a one-line
   classifier change with an index-content consequence, deliberately left to
   the owner.
6. **Re-baseline the release-readiness matrix** for the pinned snapshot once
   1–5 are decided. Until then **G-SEM stays `NOT YET VERIFIED` for C++**,
   and this run adds a second agent-side measurement to CP3's — the gate is
   *measurable*, not yet re-verified for the candidate.

---

## 10. The evidence index

| Path | Content |
|---|---|
| `docs/pilots/cp2-reeval/raw/queries/` | the CP1 §D workflow queries, with their answers and exit codes |
| `docs/pilots/cp2-reeval/raw/grading.json` | the mechanical grading of the five workflows (honesty, capability, evidence) |
| `docs/pilots/cp2-reeval/raw/freshness/` | the F0–F5 freshness verdicts and the repository state at each scenario |
| `docs/pilots/cp2-reeval/raw/incremental/` | the delta pair, both fact digests, the tree identity and the twelve query-parity pairs |
| `docs/pilots/cp2-reeval/raw/perf/` | the warm-query timings (5 repetitions per query) |
| `docs/pilots/cp2-reeval/raw/mcp/` | the MCP stdio smoke, the D.5 tool sequence and the OpenCode session |
| `docs/pilots/cp2-reeval/raw/semantic-tool/` | the twelfth MCP tool on the pilot: the catalog, the classified refusal, the check order |
| `docs/pilots/cp2-reeval/raw/cold-index*` | the cold-index measurements (the two debug runs and the release run with the indexer's own peak working set) |
| `docs/pilots/cp2-reeval/raw/first-party-split.json` | the first-party vs third-party file split, read from the index |
| `docs/pilots/cp2-reeval/raw/isolation.json` | the before/after isolation record of the original clone and the snapshot |
| `docs/pilots/cp2-reeval/method/` | the three scripts this run adds: the step runner, the twelfth-tool check and the agent session; plus the first-party split probe |

### 10.1 What is **not** archived

Stated explicitly, because a report that hides its gaps is the failure mode
this pilot exists to prevent:

- **The first cold pair's wall times and stage timings** (220.75 s / 216.72 s
  with the `ingest`/`verify`/`promote` breakdown) and the **incremental and
  oracle wall times** (70.2 s, 65.3 s, 212.3 s) come from the harness's
  console output and are **not archived**; the archived repeat pair is in
  `raw/cold-index-debug.txt` and the harness's own JSON records carry the
  counters and `timings_ms`, not a wall field.
- The **first `run-all.ps1` attempt** (the failed run described in 8.1) is
  summarized in `raw/first-attempt.json` — its grading summary (16 honesty
  `UNAVAILABLE`, 1 PASS, 4 capability `UNAVAILABLE`, 2 evidence
  `UNAVAILABLE`) and the outcome; the **full grader JSON of that attempt is
  not archived**, only the final grading is in `raw/grading.json`.
- The **agent session's full transcript** (462 542 B of model output) was
  **dropped** as reproducible noise; the summary and the answer text are
  archived instead. The summary's `citations_found` (21) was counted from
  the archived answer text with the same regex the archived script uses, and
  the `citations_source` field records that.
- The **per-class fact counts** (first-party vs third-party) are the first
  CP2's, not re-measured here (§3.1); only the file split is.
- The **E9 profile** is CP3's, not repeated here (§3.5).
- The **`search_works` counter of the first twelfth-tool run** read 0 while
  its stdout showed results; the counter is **not explained**. The re-run
  reads **29** and the raw stdout agrees, and the re-run is the archived one
  — the first run's raw stdout was overwritten by the re-run and is **not**
  archived.
- The **harness self-test output** (`grade.test.cjs`, `fact-digest.test.cjs`,
  `test-path-safety.ps1`) is **not archived**; the report cites the test
  *counts* the files declare, and the path-safety result is quoted from the
  run's console output.

**Every archived path is redacted** with the frozen
`docs/pilots/cp2/method/redact.cjs` (the benchmark workspace → `<CP2_BENCH>`,
the user profile → `<USERPROFILE>`, the Riltra checkout → `<RILTRA_ROOT>`,
the MMSP checkout → `<MMSP_ROOT>`). No credential, token or key is present,
and no large benchmark data is committed: the evidence is JSON documents and
measurements, not index copies.

**The frozen first-run archive is untouched**: `docs/pilots/cp2/` and
`docs/m9-cp2-evidence.md` were read, never written. The one file this run
came closest to was the harness's `measure.ps1` and `freshness.ps1` — the
copies made in error were restored from git before the run, and
`git status` shows the archive unmodified.




