# M9 CP2 — MMSP pilot validation and targeted correctness fix

**Status: CP2 EXECUTED — AUDIT CORRECTIONS APPLIED — AWAIT INDEPENDENT AUDIT
AND OWNER GO.** Executed 2026-10-20 on `m9-multirepo-release-candidate`,
under the owner directive "Proceed to CP2 after independent CP1 PASS".

CP2 validates Riltra against the frozen MMSP pilot, grades the five CP1
workflows for **honesty** and **capability** separately, and fixes one
reproducible defect. One further defect is **measured, specified and
escalated** instead of implemented, because closing it changes the import
graph rather than a single fact (§9, E8).

**Audit corrections (2026-10-20, same branch).** An independent audit found
that the C++ direct-initialization capture asserted a constructor call in
cases where C++ reads the declaration as a function declaration, and that the
grading script turned vacuous evidence into `PASS`. Both are corrected here:
§9.4 (the corrected capture, with compiler and Tree-sitter evidence),
§3 (the corrected grading), §10 (the corrected decision package). The
original CP2 raw evidence is preserved unchanged in
[`pilots/cp2/raw/`](pilots/cp2/raw/); the corrected measurements are in
[`pilots/cp2/raw-audit/`](pilots/cp2/raw-audit/). Where the two differ, the
audit numbers govern.

- Branch: `m9-multirepo-release-candidate`
- Pilot: MMSP `RyanAraaful/mmsp` at `d62f06a576d8e76b705c8ecc57fc380f7ebc9c44` (owner decision D-M9-1, unchanged)
- Design gate: [`m9-design-gate.md`](m9-design-gate.md) Revision 3
- CP1 spec: [`m9-cp1-pilot-spec.md`](m9-cp1-pilot-spec.md) (corrected, not reopened)
- Raw evidence: [`pilots/cp2/`](pilots/cp2/) (redacted; no machine-local path, no credential)
- CP3 was **not** started.

Status vocabulary: **PASS / FAIL / BLOCKED / UNMET / UNAVAILABLE**. A correct
`UNKNOWN`, `partial` or `POTENTIAL_CALL` is **evidence of honest behaviour,
not evidence that a capability requirement has passed**. `UNAVAILABLE` means
the evidence itself could not be obtained; it is not a `PASS`.

---

## 1. Setup, host and isolation

| Item | Value |
|---|---|
| OS | Microsoft Windows 11 IoT Enterprise LTSC, build 26100 |
| CPU / RAM | Intel Core i5-8250U @ 1.60 GHz / 19.9 GiB |
| Filesystem | NTFS (`E:` workspace, `C:` temp) |
| git / rustc / cargo | 2.45.1.windows.1 / 1.99.1 / 1.99.1 |
| gitleaks (repo scanner) | 8.30.1, pinned |
| Build profiles | **debug** (M3/M4-comparable) for the main measurements; **release** for the SLO comparison |
| Pilot snapshot | normal `git clone` + `checkout --detach d62f06a…`, tree `c5482ea00481675e0aa921538f560947b68dca12` |
| `RILTRA_HOME` | a dedicated directory outside both repositories; the registry contains only the pilot repository |
| Original MMSP repository | **never modified**: HEAD `75b3a3f5e89b080de3356a3fae07d572d0ee9cf8`, `git status --porcelain` empty at every check |
| CP7-A frozen archive | untouched (`docs/pilots/cp7a/` not edited) |

The pilot snapshot is an isolated clone, as CP1 §A.2 prescribes: no
protection override, no worktree inside MMSP, no dataset, no build, no
script, and no MMSP configuration was read. The only MMSP content read was
tracked source at the pin.

---

## 2. Cold index and syntax coverage

### 2.1 Measurements

| Run | Profile | Wall | Peak RSS | Index bytes | Calls |
|---|---|---|---|---|---|
| pre-fix 1 | debug | 192.3 s | — | 49 225 728 | 89 819 |
| pre-fix 2 | debug | 166.5 s | — | 49 225 728 | 89 819 |
| pre-fix 3 | debug | 185.0 s | 121.6 MiB | 49 225 728 | 89 819 |
| post-fix (first capture, superseded) | debug | 155.8 s / 156.8 s | — | 50 462 720 | 91 957 |
| post-fix (release, first capture, superseded) | release | 46.7 s | 121.1 MiB | 50 462 720 | 91 957 |
| **corrected capture (audit)** | debug | **160.0 s** | — | 50 238 592 | **91 025** |
| **corrected capture (audit)** | release | **23.8 s** | 120.3 MiB | 50 238 592 | **91 025** |

Two independent cold runs (D-M9-3) were made after the first fix; a third
independent run measured the indexer's own peak working set (the harness
otherwise measures the child shell). Wall-time variance on this host is
large — an independent debug rebuild in the same session took 336.5 s under
contention, and the release cold index measured 46.7 s under contention but
**23.8 s on an idle host** — so the numbers are reported per run with their
load condition, never as a single mean.

The audit re-ran the cold index with the corrected capture (§9.4). The
call count falls from 91 957 to **91 025**: the 932 facts that the first
capture asserted from the most-vexing-parse form are withdrawn, because C++
reads those declarations as function declarations unless a name lookup proves
otherwise. Every other counter is unchanged. The corrected index is smaller
by 224 128 bytes, consistent with the withdrawn facts.

### 2.2 Processing counters (identical in every run, pre- and post-fix)

| Counter | Value | CP1 cross-check |
|---|---|---|
| `files.discovered` | **2 277** | = CP1 §A.3 tracked files at the pin |
| `files.processed` | **1 438** | = CP1 §A.5 "Riltra-parsed extensions" |
| `files.definitions` | 12 105 | — |
| `files.imports` | 6 680 | — |
| `files.calls` | **91 025** (89 819 pre-fix) | +1 206 from the §9 defect fix (the sound direct-initialization form only) |
| `files.errors` | 1 991 | — |
| `files.skipped_binary` | 4 | — |
| `files.unsupported` | 835 | = (489 CUDA + 350 other) − 4 binary |
| `files.skipped_ignored/large/symlink/escape/unreadable` | 0 | — |

### 2.3 First-party vs third-party split (read from the index)

| | Files | Definitions | Imports | Calls | Parse errors | Files with errors |
|---|---|---|---|---|---|---|
| First-party | 813 | 8 015 | 4 491 | 75 900 | 593 | **55 of 813** |
| Third-party | 625 | 4 090 | 2 189 | 15 125 | 1 398 | 96 of 625 |

**55 first-party files with parse errors reproduces the M2 historical figure
exactly** ("152,543 errors across 5,481 files, of which only 55 are
first-party", `docs/m2-validation.md`): the M3/M4 baseline and this pilot
agree on the meaningful signal even though the tree is much smaller.

---

## 3. Ground-truth correctness — the five CP1 workflows

Grading is mechanical (`pilots/cp2/method/grade.cjs`) against the frozen
CP1 §D criteria. Each workflow carries an
**honesty gate** (is the answer trustworthy?), a **capability gate** (does the
answer cover what a GitNexus alternative must cover?) and, where the evidence
itself is missing, an **evidence verdict**.

The audit corrected the grader (`pilots/cp2/method/grade.cjs`, with
`grade.test.cjs` as its regression test). The corrections:

- a parse error is **never** a `PASS`; the verdict becomes `UNAVAILABLE` and
  records the reason the tool gave;
- a **missing** output or an **unclassifiable** error is not a clean
  failure either: the check is `UNAVAILABLE`, not `PASS`;
- an error is classified by its **actual** reason. The CUDA terminus is
  unavailable because CUDA sources are not indexed (`cuda-unsupported`), not
  because the source changed (`source-changed` is what the tool's own message
  says);
- an impact verdict must be **exactly** `UNKNOWN`, not merely "not SAFE";
- an empty array is **never** proof of completeness: a criterion that
  inspects the **reported edges** needs at least one edge, otherwise the
  verdict is `UNAVAILABLE` with the reason `no-reference-returned`. A
  criterion that inspects the **whole answer** (the impact answer, whose
  empty path set CP1 §D.4.5 pre-registers as the expected pilot shape) is
  judged on that answer;
- a capability gap is `UNMET`, an unavailable piece of evidence is
  `UNAVAILABLE`, and the two are never mixed.

| Workflow | Honesty gate | Capability gate | Evidence |
|---|---|---|---|
| 1. Find a symbol and its references | **PASS** (5/5) | **UNMET** | — |
| 2. Callers and callees | **UNAVAILABLE** (4 PASS, 1 UNAVAILABLE) | **UNMET** | — |
| 3. Trace a call path | **PASS** (3/3) | **UNMET** | **UNAVAILABLE** (cuda-unsupported) |
| 4. Impact of a change | **PASS** (3/3) | **UNMET** | — |
| 5. Natural-language question via OpenCode/MCP | **PASS** (server side, §7) | **UNAVAILABLE** (agent side blocked) | **UNAVAILABLE** (provider-auth-403) |

Totals over the 24 checks: **17 honesty PASS, 1 honesty UNAVAILABLE, 4
capability UNMET, 2 evidence UNAVAILABLE** — and **no check is a vacuous
`PASS`**.

The one honesty check that is `UNAVAILABLE` is D2-H5 ("every reported edge
carries a resolvable location and is labeled as a candidate"): the query
reports **no** edge at all, so no reported edge can be inspected. The
criterion is neither confirmed nor refuted, and the earlier `PASS` was
vacuous. The underlying behaviour is unchanged and correct (no edge names the
option), but the honest verdict for *that criterion alone* is `UNAVAILABLE`.

### 3.1 Workflow 1 — find a symbol and its references

Honesty **PASS**:

- `search --symbol normalize_vec` returns exactly the Python definition
  `colmap/benchmark/reconstruction/evaluation/geometry.py:34` and **no C++
  result** — the CP1 §B.1 B3 anchor is met, and the CP1 requirement to
  distinguish 5 call expressions, 4 lines and 2 import references is
  measurable: the `callers` query at that anchor returns exactly **5**
  `POTENTIAL_CALL` candidates, two of them on `geometry.py:43` (spans
  `1984-2003` and `2005-2024`, i.e. the two calls on one line).
- `search --symbol Run` returns multiple distinct identities with distinct
  locations, not one merged symbol.
- `search --symbol max_triangulation_angle` returns **0 results**: a data
  member is not an indexed symbol. Reported, not guessed.
- Every answer carries the repository identity and a freshness verdict.

Capability **UNMET**: the reference set of `PatchMatch::Problem::Print` cannot
be resolved to the C++ method. The anchor at `patch_match.cc:118:27`
returns **0** edges although three `.Print()` call sites exist
(`patch_match.cc:667`, `:668`, `fusion.cc:144`) — the call facts are in the
index (4 `POTENTIAL_CALL` edges named `Print`), but the answer cannot
separate the three *different* `Print` methods. Cause: the syntax layer
matches the terminal name only and a syntax edge never resolves a callee
endpoint (`callee: null`); C++ identity needs a semantic provider (E1/E7).

### 3.2 Workflow 2 — callers and callees

Honesty **PASS** (4 of the 5 checks; the fifth is `UNAVAILABLE`, see §3):

- `callees` at `patch_match.cc:191:18` (`PatchMatch::Run`) returns exactly
  the 4 expected candidates, including the `PatchMatchCuda` construction and
  the `patch_match_cuda_->Run()` call at `patch_match.cc:196-197` — the
  CP1 §B.2 expectation.
- `callers` at `patch_match.cc:54:28` (`CreateWorkspace`) returns exactly
  the **3** expected call sites, all inside `patch_match.cc` — the CP1
  §B.1 B2 expectation, and no cross-file attribution.
- The `ThreadPool` dispatch to `ProcessProblem` is never presented as a
  proven `CALLS` edge.
- The `option_manager.cc:1019` address-of site is not presented as a call
  to `max_triangulation_angle` — but the query returns **no** edge, so the
  criterion is `UNAVAILABLE` rather than a vacuous `PASS`.

Capability **UNMET**: the three external `PatchMatchController` construction
sites are not attributed to the class. The corrected capture (§9.4) records
the *sound* direct-initialization calls only, and all three construction
sites use the most-vexing-parse form (`PatchMatch patch_match(patch_match_options, problem);`
— the arguments are identifiers, so only a name lookup can say whether they
are types or variables). The `callers` query could not reach them in any
case, because the *candidate caller files* for an anchor are only the
anchor's own file plus the files whose **import text mentions the anchor's
repository-relative path** — and MMSP's C++ includes are rooted at
`colmap/src/`, so that match never succeeds (E8). The third site
(`dense_reconstruction_widget.cc:397-398`) is a `make_shared`/`make_unique`
template instantiation whose callee name is the factory template, not the
class; attributing it needs template-argument resolution (E7).

### 3.3 Workflow 3 — trace a call path

Honesty **PASS**: the plausibility disclaimer `plausible-path-not-execution`
is present, the path does not claim a device-side definition, and the
query aimed at the CUDA terminus fails cleanly
(`error: anchor unresolved: source-changed; reindex before syntax fallback`,
exit 3) instead of inventing a path.

Evidence **UNAVAILABLE** for the CUDA terminus, with the real reason: Riltra
does not index CUDA sources (835 unsupported files, 489 of them CUDA), so the
`.cu` anchor can never resolve. The tool's own message says `source-changed`;
the actual cause is `cuda-unsupported`. The two are recorded separately.

Capability **UNMET**: the trace returns 21 **single-edge** paths (the direct
callees of the anchor) and cannot extend to the 4-hop frozen chain
(`PatchMatchController::Run` → `ProcessProblem` → `PatchMatch::Run` →
`PatchMatchCuda::Run`). Cause: a syntax edge has no resolved callee
endpoint, so the traversal cannot continue past the first hop, and the
`ThreadPool` dispatch is an address-of reference, not a call (E7).

### 3.4 Workflow 4 — impact of a change

Honesty **PASS**: the anchor resolves to
`colmap/src/colmap/mvs/patch_match_options.h:56:10`, the verdict is
`UNKNOWN` (never `SAFE`/`LOW`, never "no impact"), `completeness.status` is
`partial` with `graph-completeness-unproven`, the disclaimer
`plausible-impact-not-execution` is present, and the two macro sites are
not resolved to function definitions. The 12 unresolved reasons are
reported (`insufficient-impact-evidence`, `empty-result-not-coverage-proof`,
`unresolved_target`, …).

Capability **UNMET**: the affected set is empty. The member read, the
address-of binding and the macro arguments are not call expressions, so the
member has no incoming edge at all (E7). **The empty path set is reported as
`UNKNOWN` with the reason, and is explicitly not evidence that changing the
symbol is safe.**

### 3.5 Workflow 5 — natural-language question via OpenCode/MCP

Honesty **PASS** on the **server side** only: the MCP surface answers, the
repository identity is present, and an unknown repository returns a
classified error (§7). The **agent side is BLOCKED**: the OpenCode free tier
refuses a nested CLI session, so no LLM ever interpreted the question
through the tools (§7.3). The capability verdict is therefore
`UNAVAILABLE`, not `UNMET`: the evidence was never produced. What the tools
can and cannot show is in §3.2 and §3.4.

---

## 4. Freshness (CP1 §C scenarios, executed unchanged)

All five pre-registered scenarios were executed on the isolated snapshot and
restored between scenarios. **No expectation was adjusted after seeing a
result.**

| Scenario | Action | Pre-registered expectation | Measured | Verdict |
|---|---|---|---|---|
| F0 | baseline at the pin | `fresh` | `fresh`, reasons `[]` | PASS |
| F1 | edit `patch_match_options.h` (default `-1.0f` → `60.0f`) | `stale`, `content-changed` (+`dirty`) | `stale`, reasons `["dirty","content-changed"]` | PASS |
| F2 | `git mv patch_match_cuda.h → patch_match_cuda_renamed.h` | `stale`, `file-removed` + `file-added` | `stale`, reasons `["dirty","file-added","file-removed"]` | PASS |
| F3 | `git rm patch_match_options.cc` | `stale`, `file-removed` | `stale`, reasons `["dirty","file-removed"]` | PASS |
| F4 | `git checkout --detach 2b1bf42…` | `stale`, `head-changed` | `stale`, reasons `["head-changed"]` | PASS |
| F5 | `git checkout -b m9-freshness-probe` (same commit) | `stale`, `branch-changed` (and **not** `head-changed`) | `stale`, reasons `["branch-changed"]` | PASS |

**No silent-fresh was observed in any scenario.** The snapshot was restored
to the pin afterwards (HEAD `d62f06a…`, porcelain 0).

---

## 5. Incremental correctness (CP1 §A.7 five-point contract)

The audit re-ran the whole step with the corrected capture:

| Run | Mode | `reused` | `reparsed` | `fallback_reason` | Wall | Fact counters |
|---|---|---|---|---|---|---|
| no-op refresh | `incremental` | 1 438 | 0 | `none` | 52.1 s | 2 277 / 1 438 / 12 105 / 6 680 / 91 025 / 1 991 |
| one-file delta | `incremental` | 1 437 | 1 | `none` | 65.0 s | identical |
| independent full rebuild (fresh home, same tree) | `full` | 0 | 1 438 | `forced-full` | 166.5 s | identical |

Same-tree fact parity, computed as order-independent digests over the index
database (files with content digests, symbols, imports, calls with spans and
provenance, and the edge set):

| Table | Rows (incremental / oracle) | Digest identical |
|---|---|---|
| `files` | 1 438 / 1 438 | **yes** |
| `symbols` | 12 105 / 12 105 | **yes** |
| `imports` | 6 680 / 6 680 | **yes** |
| `calls` | 91 025 / 91 025 | **yes** |
| `code_edges` | 97 705 / 97 705 | **yes** |

The 12 query outputs of the two homes are identical after normalizing the
volatile snapshot metadata (`id`, `created_unix`, `nonce`, `revision`,
`added_unix`); the only raw difference is the snapshot creation timestamp.
The comparison **aborts** if a query exits non-zero, prints nothing, or
differs — the harness can no longer report a match it did not measure
(§11.1).

**Contract result:** same tree and configuration, fact parity, equal total
fact counters, operational counters validated per mode, and no unexplained
fact difference. The operational counters are *not* expected to be equal: a
full build re-parses everything, an incremental run reuses the unchanged
files. This is the CP1 §A.7 contract, and it is met.

---

## 6. Query performance versus the blueprint SLO (diagnostic, not a gate)

Blueprint §14.3 proposes **p95 ≤ 500 ms** for warm indexed
symbol/relationship queries on a small/medium fixture set, explicitly as a
"target awal yang dapat direvisi" and explicitly **not** a release gate.
The pilot here is the ~2 000-file MMSP repository, at the upper end of that
range. Five repetitions per query, both profiles, on the same host.

The CP2 release numbers were measured while the host was busy; the audit
re-measured them on an idle host with the corrected binary, which is **not
comparable** to the debug column of the same table either. Both are given,
with the load condition, because the difference is the measurement itself:

| Query | release p50 / p95 (idle host, audit) | release p50 / p95 (CP2, under load) | vs 500 ms p95 (audit) |
|---|---|---|---|
| `search --symbol PatchMatch` | 536 / **544** ms | 752 / 1 103 ms | missed by 1.09× |
| `search --symbol Run` | 534 / **537** ms | 765 / 1 049 ms | missed by 1.07× |
| `callers` (Print definition) | 856 / **1 001** ms | 1 152 / 1 186 ms | missed by 2.00× |
| `callees` (PatchMatch::Run) | 897 / **989** ms | 1 116 / 1 149 ms | missed by 1.98× |
| `trace --depth 4` | 884 / **1 007** ms | 1 191 / 1 222 ms | missed by 2.01× |
| `impact` (option member) | 723 / **738** ms | 1 104 / 1 141 ms | missed by 1.48× |
| `index status` | 517 / **522** ms | 786 / 881 ms | missed by 1.04× |
| `repo status` | 309 / **315** ms | 879 / 1 135 ms | met |

**Diagnostic finding: the warm-query p95 target is missed by 1.04× to 2.01×
even in the release profile on an idle host.** The earlier CP2 figures
(2.1×–2.4×) were inflated by host contention and must not be quoted as the
product's cost.

The cause is **a hypothesis, not a measured fact**. The measurements are
consistent with a large fixed per-invocation cost: `repo status`, which only
reads the git fingerprint, costs 0.31 s at p50, and `index status` 0.52 s,
while the graph queries cost 0.72–0.90 s at p50. That *suggests* process
start-up, the `git` subprocess and the database open dominate, rather than
the graph traversal — but the audit did **not** profile the query path, so
the split between the fixed cost and the traversal is unmeasured. What is
measured: the cold index takes 23.8 s in release, a no-op refresh 52.1 s in
debug, and every warm query costs 0.54–1.01 s at p95 in release.

No release gate is proposed from these numbers. The options are an owner
decision: (a) accept the current cost and document it, (b) profile the
per-invocation cost and then reduce it (e.g. avoid the git subprocess when
the snapshot fingerprint can be reused), or (c) restate the §14.3 target for
a ~2 000-file C++ repository. Changing the target is a blueprint decision,
not a CP2 decision.

---

## 7. OpenCode MCP interoperability

### 7.1 Direct MCP stdio smoke (the objective part)

| Check | Result |
|---|---|
| Tool surface | **exactly 11** — `riltra_code_search`, `riltra_code_callers`, `riltra_code_callees`, `riltra_code_trace`, `riltra_code_impact`, `riltra_repo_list`, `riltra_repo_status`, `riltra_knowledge_show`, `riltra_knowledge_search`, `riltra_knowledge_related`, `riltra_health` |
| stdout purity | 6 of 6 lines parsed as JSON-RPC, **0 non-JSON lines** |
| stderr | empty |
| Source-grounded query | `riltra_code_search` on the pilot returned the expected document (freshness `fresh`, repository identity present) |
| Repository selection | `riltra_repo_list` returns exactly the pilot repository; `riltra_repo_status` reports HEAD `d62f06a…`, dirty `false` |
| Classified error | unknown repository → `{"kind":"not_registered","message":"unknown repository: 'does-not-exist'"}` with `isError: true`, transport healthy |
| Non-mutating read | the store's **content** digest is unchanged across the session; the index file's *bytes* change on the first open because a WAL checkpoint rewrites the file — the accepted read-side effect already recorded in the design gate (G-SEC gaps) |

### 7.2 The D.5 tool sequence (scripted, over the real MCP transport)

The natural-language question — *"how is the max_triangulation_angle option
wired from the command line into the source-image selection, and which files
would I need to touch to change its default?"* — was turned into the tool
sequence an agent would run, and that sequence was executed against the live
MCP server (9 calls, archived verbatim in
`pilots/cp2/raw/mcp/d5-tool-sequence.json` for the CP2 run and in
`pilots/cp2/raw-audit/mcp/d5-tool-sequence.json` for the audit run, with
identical results):

| Step | Tool | Result |
|---|---|---|
| 1 | `riltra_code_search` `max_triangulation_angle` | `total: 0` — the data member is not an indexed symbol (reported, not guessed) |
| 2 | `riltra_code_search` `PatchMatchOptions` | 4 hits: the struct in `option_manager.h:59`, the struct in `patch_match_options.h:37`, and the `Print`/`Check` methods |
| 3 | `riltra_code_callers` at the member | 0 edges, anchor uncertainty reported |
| 4 | `riltra_code_callers` at `option_manager.cc:1019:41` | 0 edges, anchor uncertainty reported |
| 5 | `riltra_code_impact` at the member | `UNKNOWN`, 0 paths, `partial`, 12 unresolved reasons, disclaimer present |
| 6 | `riltra_code_callees` at `ReadProblems` | 30+ candidates, all heuristic |
| 7 | `riltra_repo_list` | the pilot repository only |
| 8 | `riltra_repo_status` | HEAD `d62f06a…`, dirty `false` |
| 9 | `riltra_code_callers` on an unknown repo | classified error, `isError: true` |

An agent using these tools would learn the option's *type* and its
`Print`/`Check` methods, and would correctly report that the member's
callers and its impact are **not** resolvable — the honest answer, and the
capability gap of §3.2/§3.4.

### 7.3 A real OpenCode session — **BLOCKED**

The OpenCode CLI was invoked exactly as the frozen CP7-A harness invokes it
(`opencode run --format json --model opencode/deepseek-v4.1-flash-free`,
same project-config pattern, MCP server registered under `mcp.servers`, the
agent restricted to the MCP tools with filesystem reads denied).

Every attempt failed at the provider, before any model call:

```
{"type":"error","error":{"type":"provider.auth",
 "message":"OpenCode's free tier can only be used from within OpenCode. Download it at https://opencode.ai and keep building.",
 "status":403,"response":{"body":"{\"type\":\"error\",\"error\":{\"type\":\"FreeTierError\",...}}"}}}
```

The same 403 occurs with the CP7-A model and with the default model, and
with a trivial prompt, so it is a provider-side free-tier restriction on
nested CLI sessions in this environment — **not** a Riltra defect and not a
model choice. Consequently the **agent-side** half of workflow 5 (an LLM
interpreting the natural-language question through the MCP tools) is
**BLOCKED**, and its verdict is **UNAVAILABLE** — the evidence was never
produced, which is not the same as a failure and not the same as a pass.
The audit re-ran the session and recorded the outcome in a machine-readable
summary (`raw-audit/mcp/opencode-session.json`:
`{"exit_code":1,"reason":"provider-auth-403",...}`); the harness exits with
the provider's code and prints `BLOCKED: the OpenCode session failed`. The
**server-side** half is fully verified by §7.1 and §7.2. CP7-A's own G6
evidence (a real OpenCode session) remains the last agent-side evidence and
is unchanged; it must not be presented as re-verified by CP2.

---

## 8. What CP2 did **not** change

No source outside the single defect fix, no schema, no dependency, no
scanner, no benchmark corpus, no ruleset, no branch protection, no M8/CP7-A
archive, no MMSP content, no history rewrite, no release tag, and no merge
to `master`. The CP1 design and ground truth were not reopened.

The audit changed the **benchmark harness** — the measurement scripts under
`pilots/cp2/method/`, which are evidence artefacts, not product code: the
path guard, the grader and the parameterization (§11.1). It changed no
acceptance criterion and no CP1 ground truth, and it lowered no gate: the
corrected grading is *stricter* than the CP2 one (§3).

---

## 9. Defect fixed: C++ direct-initialization constructor calls

### 9.1 Evidence (the defect)

On the frozen pilot, the C++ construction sites of `PatchMatchController`
were **absent from the index**: `exe/mvs.cc:271`,
`automatic_reconstruction.cc:431` and `patch_match.cc:670` produced no call
fact, so the class's construction sites were invisible to every query.

Root cause (tree-sitter-cpp 0.23.4, verified by dumping the parse tree):
C++ direct initialization has **no `call_expression` node**. The grammar
models it as

```
declaration
  type_identifier / qualified_identifier      <- the called type
  init_declarator
    identifier                               <- the variable
    argument_list                            <- the arguments
```

and, when every argument looks like a type, C++'s *most-vexing-parse* rule
applies and the same statement is parsed as a `function_declarator`
(a prototype) instead. Riltra's visitor matched only `call_expression`, so
both forms were invisible.

### 9.2 The fix (minimal, inside the existing architecture)

`crates/cli/src/syntax.rs` only:

- a C++ `declaration` is now examined for the direct-initialization shape:
  when the declarator is an `init_declarator` whose value is an
  `argument_list`, the constructor call is recorded as a call fact — the
  call range runs from the called type to the end of the argument list, the
  callee is the type, and the terminal is the type's last name part;
- the owner proof records the same range, so the new calls get a proven
  enclosing definition like every other call.

The fix adds **no** dependency, **no** schema change, **no** edge kind and
**no** promotion: the new facts are `POTENTIAL_CALL` / `precision:"Heuristic"`
/ `verification:"Unverified"`, exactly like every other syntax call, and the
callee endpoint stays unresolved.

### 9.3 Effect (measured)

| | Before | After (corrected) |
|---|---|---|
| `files.calls` on the pilot | 89 819 | **91 025** (+1 206) |
| Other fact classes | — | unchanged (definitions 12 105, imports 6 680, errors 1 991, files 1 438) |
| Regression tests | — | **13** C++ unit tests in `syntax.rs` |
| `cargo test --locked` | 648 passed | **654 passed / 0 failed** (8 suites) |
| `cargo fmt --check`, `cargo clippy -D warnings` | clean | clean |

The M2/M6 goldens are **byte-unchanged**: the frozen C++ fixtures contain
no direct-initialization form, so no golden was edited.

### 9.4 Audit correction — the most-vexing-parse form is not a constructor call

The first version of this fix also recorded the `function_declarator` form
(`T v(a, b);` with type-looking arguments) as a constructor call whenever the
declaration was inside a function body, on the premise that "a function
declaration is not legal C++ inside a block". **That premise is false, and
the capture produced false facts.** The audit verified the C++ semantics
with a compiler (MSVC 19.34, `/std:c++20`) and the grammar with Tree-sitter:

| Source | C++ reading (compiler-verified) | Tree-sitter shape | First capture | Corrected |
|---|---|---|---|---|
| `Widget w(1);` | a variable, i.e. a constructor call | `init_declarator` → `argument_list(number_literal)` | call ✓ | call ✓ |
| `Widget w(int);` | **a function declaration** (`w` has type `Widget(int)`) | `function_declarator` → `parameter_list(primitive_type)` | call ✗ | not a call ✓ |
| `Widget w();` | **a function declaration** (`Widget(void)`) | `function_declarator` → empty `parameter_list` | call ✗ | not a call ✓ |
| `Widget w(T());` | **a function declaration** (`Widget(T(void))`) | `function_declarator` → `parameter_declaration` + `abstract_function_declarator` | call ✗ | not a call ✓ |
| `Widget w(options, problem);` | **ambiguous**: a function declaration if the names denote types, a variable if they denote variables | `function_declarator` → `parameter_list(type_identifier…)` | call ✗ | not a call (ambiguity reported, §9.5) |
| `void helper(int);` in a block | legal C++ (a block-scope function declaration) | `function_declarator` | not a call ✓ | not a call ✓ |
| `Widget w;` | a variable | plain `identifier` | not a call ✓ | not a call ✓ |
| `Widget w{1};` | a variable (a constructor call) | `init_declarator` → `initializer_list` | not a call (missed) | not a call (missed) |

The compiler check: taking the address of the declared name compiles only for
a variable; `Widget w(int);` yields `error C2440: cannot convert from
'Widget (__cdecl *)(int)' to 'Widget *'`, i.e. `w` is a function.

**The rule the corrected code implements**: only the `init_declarator` form
with an `argument_list` value is a constructor call, because there the
arguments are *expressions* and C++ therefore reads the declaration as a
variable definition. The `function_declarator` form is never asserted:

- when a parameter is syntactically a type (`int`, `T()`), the declaration is
  a function declaration (compiler-verified above);
- when the parameters are plain identifiers, only a **name lookup** can tell a
  variable definition from a function declaration — that is a semantic
  question, and Riltra has no semantic provider (E1/E7).

Effect of the correction on the pilot: the facts asserted from the
ambiguous form (**932**) are withdrawn and the form with expression
arguments is kept (**+1 206** over the pre-fix baseline). The three MMSP
construction sites named in §9.1 use the ambiguous form, so they are **not**
captured — the audit withdraws the earlier claim that they are "now present
as candidates".

The corrected code also removes the scope test it no longer needs: a
block-scope function declaration is legal C++, so the scope never settled
the question.

### 9.5 The ambiguity is reported, not resolved

Where the syntax cannot decide, Riltra records **no** constructor call and
says so. The evidence is the absence of the fact plus the anchor uncertainty
the queries already report (`syntax-only anchor: no semantic identity at this
position`); the audit did not add a new field, a new edge kind or a new
label, so the schema and the edge taxonomy are unchanged. Closing this gap
needs a name lookup — i.e. the semantic provider that E1/E7 already mark as
owner-gated.

### 9.6 Known limits of the retained form

The retained form is the right *shape*; three limits are recorded in the
code and here, and none of them produces a wrong fact:

| Limit | Behaviour | Why |
|---|---|---|
| a **typedef** is indistinguishable from a class | `size_t n(0);` is recorded as a call to `size_t` although C++ may perform a functional cast | a name lookup would settle it (gap E12) |
| a **multi-declarator** declaration | `Widget a(1), b(2);` records the fact for the first declarator only | the grammar attaches one declarator field; the omission is conservative |
| a **template type** | `std::vector<int> v(10);` is recorded, with the terminal `vector<int>` | the template arguments stay in the terminal (the known terminal-mangling limit) |

The first two are over-approximations and omissions of the same class as
E12; the third is already recorded in the pilot's template-call measurement.

---

## 10. Decision package

### 10.1 Per-workflow gates

| Workflow | Honesty | Capability | Evidence |
|---|---|---|---|
| 1. Find a symbol + references | **PASS** (5/5) | **UNMET** | §3.1; `raw-audit/queries/d1-*.json` |
| 2. Callers / callees | **UNAVAILABLE** (4 PASS, 1 UNAVAILABLE) | **UNMET** | §3.2; `raw-audit/queries/d2-*.json` |
| 3. Trace | **PASS** (3/3) | **UNMET** | **UNAVAILABLE** (cuda-unsupported); §3.3 |
| 4. Impact | **PASS** (3/3) | **UNMET** | §3.4; `raw-audit/queries/d4-*.json` |
| 5. NL question via OpenCode/MCP | **PASS** (server side only) | **UNAVAILABLE** | **UNAVAILABLE** (provider-auth-403); §7 |
| Freshness F0–F5 | **PASS** (5/5) | n/a | §4; `raw/freshness/` |
| Incremental parity | **PASS** | n/a | §5; `raw-audit/incremental/` |
| MCP surface / purity / error / isolation | **PASS** | n/a | §7.1; `raw-audit/mcp/` |
| OpenCode session (agent side) | **BLOCKED** | **UNAVAILABLE** | §7.3 |
| Warm-query SLO (diagnostic) | — | **MISSED** (1.04–2.01×, idle host) | §6 |

**Riltra is not yet a practical GitNexus alternative for these workflows on
a C++ repository of this size.** Nothing is fabricated — every answer reports
what it can support and what it cannot — but **all four capability gates are
unmet**, the agent side of workflow 5 could not be measured at all, and one
honesty criterion has no evidence to inspect. The gap is now measured rather
than assumed.

### 10.2 Gap classification

| # | Gap | Class | Severity |
|---|---|---|---|
| E1 | C++ semantics absent: no provider, no compilation database in MMSP | **unsupported capability** (owner-gated: clangd deferred by D-M9-2) | release-blocking for resolved C++ call edges |
| E7 | Reference-level resolution absent: a data member, a member read and a macro argument produce no edge, so `impact` on a field is empty, a trace cannot exceed one hop, and the ambiguous most-vexing-parse form cannot be decided (a name lookup is needed) | **unsupported capability** (owner-gated) | release-blocking for workflow 4 |
| E8 | C/C++ include-root resolution: the candidate-caller scope of an anchor is built from import texts that mention the anchor's *repository-relative* path, but MMSP's includes are rooted at `colmap/src/`. Measured: **0 of 4 009** quoted C/C++ includes resolve; a unique-suffix rule *would* match **2 993** of them, 969 are ambiguous and 47 have no match | **implementation defect** — diagnosed, measured, **not fixed here** (see 10.3) | release-blocking for cross-TU C++ callers (workflow 2) |
| E9 | The warm-query p95 target (§14.3, ≤ 500 ms) is missed by 2.1–2.4× in the release profile. The **cause is a hypothesis**: the measurements are consistent with a large fixed per-invocation cost (process start, the `git` subprocess, the database open) rather than with the graph, but the query path was **not profiled**, so the split is unmeasured | **diagnostic**, not a gate | owner decision: profile first, then either the target or the fixed cost |
| E10 | The OpenCode free tier refuses nested CLI sessions in this environment, so the agent side of workflow 5 could not be measured | **environmental block** | G-MCP re-verification deferred; CP7-A G6 remains the last agent-side evidence |
| E11 | The repository's own history carries unsanitized local paths (1 068 findings in 9 commits / 184 files, 0 credential-rule findings), inherited from CP1 | **open risk**, unchanged | rewrite still owner-gated |
| E12 | The C++ most-vexing-parse form cannot be decided from syntax: `T v(a, b);` is a function declaration when `a`/`b` denote types and a variable when they denote variables. Riltra therefore records **no** constructor call for that form (§9.4, §9.5) | **unsupported capability** (needs the name lookup of E1/E7) | the affected constructor calls are missing, not wrong |

### 10.3 E8 — the smallest defensible options (owner decision required)

CP2 deliberately did **not** implement E8. The fix is small and would
clearly improve the product, but it changes the **import graph** of every
C++ repository, not a single fact, and CP1's E7 already classified
reference-graph work as owner-gated.

The numbers below are a **diagnostic projection, not a promise of correct
resolution**: they count how many quoted includes a unique-suffix rule
*would* match on this pilot, and they say nothing about whether the matched
file is the file the compiler would open. The audit did not implement the
rule and did not measure the resulting query answers, so the effect on the
workflow gates is **unmeasured**.

- **Option A — unique-suffix include resolution** (smallest): resolve a
  quoted C/C++ include against the indexed files by path suffix when exactly
  one file matches; leave ambiguous (969) and unmatched (47) includes
  unresolved. Projection on the pilot: 2 993 of the 4 009 quoted includes
  would gain a candidate file, so a header anchor's candidate caller set
  would become non-empty for the first time. Risk: a suffix match can pick
  the wrong file when the tree contains same-suffix paths; the uniqueness
  rule bounds this, and the pilot's 969 ambiguous cases stay unresolved
  (honest, not wrong). **Whether this improves the workflow answers is not
  measured.**
- **Option B — read the include roots from the build system** (correct, more
  work): parse `target_include_directories` from CMake, so `colmap/mvs/…`
  resolves against the real root rather than a suffix heuristic. Needs a
  build-system reader and its own evidence; it is the option that would also
  serve a future C++ semantic provider.
- **Option C — accept the syntax-only scope for C++** (no change): document
  that cross-TU C++ callers are unavailable and that the workflows are
  limited to same-file relations plus Python-style import scopes.

Any option leaves E1/E7/E12 open: even with the include graph resolved, the
tool still cannot say *which* `Print` method a call refers to, and it still
cannot decide the most-vexing-parse form without a name lookup.

### 10.4 Recommended CP3 scope (not started; needs its own OWNER GO)

1. Decide E8 (A, B or C) and, if A or B, implement it with regression
   tests and re-measure the affected workflows.
2. Re-measure the workflows that E8 unblocks (2 and 3 first) and re-grade the
   capability gates.
3. Decide E9: profile the per-invocation cost first, then either restate the
   §14.3 warm-query target for a ~2 000-file C++ repository or reduce the
   fixed cost.
4. Re-run the agent-side G-MCP smoke when the OpenCode free tier allows a
   nested session, or on a host with a paid provider.
5. Re-baseline the release-readiness matrix for the pinned MMSP snapshot
   (CP3 also covers Riltra self-indexing, the M6 corpus adequacy check and
   the remaining gate evidence).

---

## 11. Evidence index

| Artefact | Content |
|---|---|
| `pilots/cp2/README.md` | the archive overview and how to reproduce the run |
| `pilots/cp2/method/README.md` | how to run the harness, its parameters and its safety rules |
| `pilots/cp2/method/` | the harness: the path-safety helpers and their tests, the step scripts, the grader and its tests, the digest implementation, the redaction |
| `pilots/cp2/raw/` | **the original CP2 run, preserved unchanged** (queries, freshness, incremental, perf, MCP, cold index, the first grading) |
| `pilots/cp2/raw-audit/` | **the audit's corrected run** (queries against the corrected index, the corrected grading, the incremental parity, the MCP smoke, the agent-side session summary) |
| `pilots/cp2/raw/cold-index*.txt` | the cold-index runs (debug and release) |
| `pilots/cp2/raw-audit/grading.json` | the corrected grading: 17 honesty PASS, 1 honesty UNAVAILABLE, 4 capability UNMET, 2 evidence UNAVAILABLE |

All archived evidence is redacted: machine-local paths are replaced with
`<CP2_BENCH>`, `<USERPROFILE>`, `<RILTRA_ROOT>` and `<MMSP_ROOT>`. No
credential, token or key is present. The redaction rule set is
`pilots/cp2/method/redact.cjs`, which takes the paths to redact as
arguments, so it carries no machine-local path of its own.

### 11.1 The superseded claim in the original harness

The original `run-all.ps1` compared two fact digests with
`if ($inc -eq $orc) { "FACT PARITY: IDENTICAL" }`. The digest script it
called crashed (`node:sqlite` returns row objects, not arrays), both sides
came back empty, and the comparison reported **IDENTICAL** for two empty
strings. The audit re-measured the parity with a working, order-independent
digest (`method/fact-digest.cjs`): the five fact tables are byte-identical
between the incremental home and an independent full rebuild (§5). The
superseded line stays in the preserved `raw/cp2-suite.txt` transcript; the
harness no longer contains the pattern, and `incremental.ps1` now aborts when
a query errors, prints nothing, differs, when the one-file delta did not
change the tree, or when an index run exits non-zero.

### 11.2 The review round

An independent review of the audit diff found three defects that the audit
itself had introduced; all three are corrected and covered by regression
tests:

| Finding | Correction |
|---|---|
| the path guard resolved only the **leaf** component, so a path *through* an intermediate junction escaped the benchmark root | the path is now resolved component by component; the test covers a path through an intermediate junction |
| the grader awarded a `PASS` to the CUDA check when the output was **missing** or **unclassifiable** | only an observed failure passes; a missing or unclassifiable output is `UNAVAILABLE` |
| two checks accepted less than their criterion: the callee check did not require the `Run` call, and the trace check compared against a hop count the criterion did not state | the callee check now requires both names, and the trace criterion states the frozen chain as 4 nodes / 3 edges, matching CP1 §B.3 |

The review also reported that a template type is not captured; the claim was
checked against the grammar and **refuted** — `std::vector<int> v(10);` is
recorded, with the terminal `vector<int>`. The two limits that remain (a
typedef is indistinguishable from a class, and a multi-declarator
declaration records its first declarator only) are recorded in §9.6 and in
the code.
