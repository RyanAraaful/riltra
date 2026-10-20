# M9 CP2 — MMSP pilot validation and targeted correctness fix

**Status: CP2 EXECUTED — AWAIT INDEPENDENT AUDIT AND OWNER GO.** Executed
2026-10-20 on `m9-multirepo-release-candidate`, under the owner directive
"Proceed to CP2 after independent CP1 PASS".

CP2 validates Riltra against the frozen MMSP pilot, grades the five CP1
workflows for **honesty** and **capability** separately, and fixes one
reproducible defect. One further defect is **measured, specified and
escalated** instead of implemented, because closing it changes the import
graph rather than a single fact (§9, E8).

- Branch: `m9-multirepo-release-candidate`
- Pilot: MMSP `RyanAraaful/mmsp` at `d62f06a576d8e76b705c8ecc57fc380f7ebc9c44` (owner decision D-M9-1, unchanged)
- Design gate: [`m9-design-gate.md`](m9-design-gate.md) Revision 3
- CP1 spec: [`m9-cp1-pilot-spec.md`](m9-cp1-pilot-spec.md) (corrected)
- Raw evidence: [`pilots/cp2/`](pilots/cp2/) (redacted; no machine-local path, no credential)
- CP3 was **not** started.

Status vocabulary: **PASS / FAIL / BLOCKED / UNMET / NOT YET VERIFIED**.
A correct `UNKNOWN`, `partial` or `POTENTIAL_CALL` is **evidence of honest
behaviour, not evidence that a capability requirement has passed**.

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
| **post-fix 1** | debug | **155.8 s** | — | 50 462 720 | **91 957** |
| **post-fix 2** | debug | **156.8 s** | — | 50 462 720 | **91 957** |
| **post-fix (release)** | release | **46.7 s** | 121.1 MiB | 50 462 720 | 91 957 |

Two independent cold runs (D-M9-3) were made after the fix; a third
independent run measured the indexer's own peak working set (the harness
otherwise measures the child shell). Wall-time variance on this host is
large — an independent debug rebuild in the same session took 336.5 s under
contention — so the numbers are reported per run, never as a single mean.

### 2.2 Processing counters (identical in every run, pre- and post-fix)

| Counter | Value | CP1 cross-check |
|---|---|---|
| `files.discovered` | **2 277** | = CP1 §A.3 tracked files at the pin |
| `files.processed` | **1 438** | = CP1 §A.5 "Riltra-parsed extensions" |
| `files.definitions` | 12 105 | — |
| `files.imports` | 6 680 | — |
| `files.calls` | **91 957** (89 819 pre-fix) | +2 138 from the §9 defect fix |
| `files.errors` | 1 991 | — |
| `files.skipped_binary` | 4 | — |
| `files.unsupported` | 835 | = (489 CUDA + 350 other) − 4 binary |
| `files.skipped_ignored/large/symlink/escape/unreadable` | 0 | — |

### 2.3 First-party vs third-party split (read from the index)

| | Files | Definitions | Imports | Calls | Parse errors | Files with errors |
|---|---|---|---|---|---|---|
| First-party | 813 | 8 015 | 4 491 | 74 770 | 593 | **55 of 813** |
| Third-party | 625 | 4 090 | 2 189 | 15 049 | 1 398 | 96 of 625 |

**55 first-party files with parse errors reproduces the M2 historical figure
exactly** ("152,543 errors across 5,481 files, of which only 55 are
first-party", `docs/m2-validation.md`): the M3/M4 baseline and this pilot
agree on the meaningful signal even though the tree is much smaller.

---

## 3. Ground-truth correctness — the five CP1 workflows

Grading is mechanical (`pilots/cp2/method/grade.cjs`) against the frozen
CP1 §D criteria. Each workflow carries an **honesty gate** (is the answer
trustworthy?) and a **capability gate** (does the answer cover what a GitNexus
alternative must cover?).

| Workflow | Honesty gate | Capability gate |
|---|---|---|
| 1. Find a symbol and its references | **PASS** (5/5) | **UNMET** |
| 2. Callers and callees | **PASS** (5/5) | **UNMET** |
| 3. Trace a call path | **PASS** (3/3) | **UNMET** |
| 4. Impact of a change | **PASS** (3/3) | **UNMET** |
| 5. Natural-language question via OpenCode/MCP | **PASS** (server side, §7) | **UNMET** |

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

Honesty **PASS**:

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
  to `max_triangulation_angle`.

Capability **UNMET**: the three external `PatchMatchController` construction
sites are not attributed to the class. Two of them
(`exe/mvs.cc:271`, `automatic_reconstruction.cc:431`) are now captured as
call facts by the §9 fix, but the `callers` query still cannot reach them
because the *candidate caller files* for an anchor are only the anchor's own
file plus the files whose **import text mentions the anchor's
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

Honesty **PASS** on the server side: see §7. Capability **UNMET**: the
option's wiring is not discoverable through the tools, for the reasons in
§3.2 and §3.4.

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

| Run | Mode | `reused` | `reparsed` | `fallback_reason` | Wall | Fact counters |
|---|---|---|---|---|---|---|
| no-op refresh | `incremental` | 1 438 | 0 | `none` | 51.9 s | 2 277 / 1 438 / 12 105 / 6 680 / 91 957 / 1 991 |
| one-file delta | `incremental` | 1 437 | 1 | `none` | 52.8 s | identical |
| independent full rebuild (fresh home, same tree) | `full` | 0 | 1 438 | `forced-full` | 336.5 s | identical |

Same-tree fact parity, computed as order-independent digests over the index
database (files with content digests, symbols, imports, calls with spans and
provenance, and the edge set):

| Table | Rows (incremental / oracle) | Digest identical |
|---|---|---|
| `files` | 1 438 / 1 438 | **yes** |
| `symbols` | 12 105 / 12 105 | **yes** |
| `imports` | 6 680 / 6 680 | **yes** |
| `calls` | 91 957 / 91 957 | **yes** |
| `code_edges` | 98 637 / 98 637 | **yes** |

The query outputs of the two homes are identical after normalizing the
volatile snapshot fields (`id`, `nonce`, `revision`, `created_unix`); the
only raw difference is the snapshot creation timestamp.

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

| Query | debug p50 / p95 | release p50 / p95 | vs 500 ms p95 |
|---|---|---|---|
| `search --symbol PatchMatch` | 1 266 / 1 455 ms | 752 / **1 103** ms | missed |
| `search --symbol Run` | 1 382 / 1 443 ms | 765 / **1 049** ms | missed |
| `callers` (Print definition) | 2 553 / 2 739 ms | 1 152 / **1 186** ms | missed |
| `callees` (PatchMatch::Run) | 2 621 / 2 636 ms | 1 116 / **1 149** ms | missed |
| `trace --depth 4` | 2 259 / 2 287 ms | 1 191 / **1 222** ms | missed |
| `impact` (option member) | 2 123 / 2 182 ms | 1 104 / **1 141** ms | missed |
| `index status` | 1 453 / 1 459 ms | 786 / 881 ms | missed |
| `repo status` | 769 / 807 ms | 879 / 1 135 ms | missed |

**Diagnostic finding: the warm-query p95 target is missed by roughly 2.1× to
2.4× even in the release profile.** The cost is dominated by fixed
per-invocation work, not by the graph: `repo status` — which only reads the
git fingerprint — already costs 0.88 s at p50 in release, so process
start-up, the `git` subprocess and the database open dominate every
query. The index itself is not the bottleneck (the cold index is 46.7 s in
release and a no-op refresh is 51.9 s in debug).

No release gate is proposed from these numbers. The options are an owner
decision: (a) accept the current cost and document it, (b) reduce the fixed
per-invocation cost (e.g. avoid the git subprocess when the snapshot
fingerprint can be reused), or (c) restate the §14.3 target for a ~2 000-file
C++ repository. Changing the target is a blueprint decision, not a CP2
decision.

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
`pilots/cp2/raw/mcp/d5-tool-sequence.json`):

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
**BLOCKED / NOT YET VERIFIED**. The **server-side** half is fully verified
by §7.1 and §7.2. CP7-A's own G6 evidence (a real OpenCode session) remains
the last agent-side evidence and is unchanged; it must not be presented as
re-verified by CP2.

---

## 8. What CP2 did **not** change

No source outside the single defect fix, no schema, no dependency, no
scanner, no benchmark, no ruleset, no branch protection, no M8/CP7-A
archive, no MMSP content, no history rewrite, no release tag, and no merge
to `master`. The CP1 design and ground truth were not reopened.

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

- a C++ `declaration` is now examined for the direct-initialization shape,
  and when it is a **local** declaration (inside a function or lambda body,
  where C++ does not allow a function declaration) the constructor call is
  recorded as a call fact: the call range runs from the called type to the
  end of the argument list, the callee is the type, and the terminal is the
  type's last name part;
- the owner proof records the same range, so the new calls get a proven
  enclosing definition like every other call.

Guards, each with a regression test: a primitive type (`int x(5);`) is not a
call; a namespace-scope prototype is not a call; a class-member declaration
is not a call; a declaration inside a `#if/#else` branch is a call.

The fix adds **no** dependency, **no** schema change, **no** edge kind and
**no** promotion: the new facts are `POTENTIAL_CALL` / `precision:"Heuristic"`
/ `verification:"Unverified"`, exactly like every other syntax call, and the
callee endpoint stays unresolved.

### 9.3 Effect (measured)

| | Before | After |
|---|---|---|
| `files.calls` on the pilot | 89 819 | **91 957** (+2 138) |
| Other fact classes | — | unchanged (definitions 12 105, imports 6 680, errors 1 991, files 1 438) |
| `exe/mvs.cc:271` | absent | `mvs::PatchMatchController` call fact |
| `automatic_reconstruction.cc:431` | absent | `mvs::PatchMatchController` call fact |
| `patch_match.cc:670` | absent | `PatchMatch` call fact |
| Regression tests | — | **7** new unit tests in `syntax.rs` (3 fail before the fix) |
| `cargo test --locked` | 641 passed | **647 passed / 0 failed** (8 suites) |
| `cargo fmt --check`, `cargo clippy -D warnings` | clean | clean |

The M2/M6 goldens are **byte-unchanged**: the frozen C++ fixtures contain
no direct-initialization form, so no golden was edited.

The construction sites are now *present* as candidates. They are still not
*attributed* to the class, because the candidate-caller scope of a C++
include is empty (E8) and the third site is a template instantiation (E7).

---

## 10. Decision package

### 10.1 Per-workflow gates

| Workflow | Honesty | Capability | Evidence |
|---|---|---|---|
| 1. Find a symbol + references | **PASS** | **UNMET** | §3.1; `raw/queries/d1-*.json` |
| 2. Callers / callees | **PASS** | **UNMET** | §3.2; `raw/queries/d2-*.json` |
| 3. Trace | **PASS** | **UNMET** | §3.3; `raw/queries/d3-*.json` |
| 4. Impact | **PASS** | **UNMET** | §3.4; `raw/queries/d4-*.json` |
| 5. NL question via OpenCode/MCP | **PASS** (server) | **UNMET** | §7; agent side BLOCKED |
| Freshness F0–F5 | **PASS** (5/5) | n/a | §4; `raw/freshness/` |
| Incremental parity | **PASS** | n/a | §5; `raw/incremental/` |
| MCP surface / purity / error / isolation | **PASS** | n/a | §7.1; `raw/mcp/` |
| OpenCode session (agent side) | **BLOCKED** | **NOT YET VERIFIED** | §7.3 |
| Warm-query SLO (diagnostic) | — | **MISSED** (2.1–2.4×) | §6 |

**Riltra is not yet a practical GitNexus alternative for these workflows on
a C++ repository of this size.** The five workflows are *honest* — nothing
is fabricated — but three of the five capability gates are unmet, and the
gap is now measured rather than assumed.

### 10.2 Gap classification

| # | Gap | Class | Severity |
|---|---|---|---|
| E1 | C++ semantics absent: no provider, no compilation database in MMSP | **unsupported capability** (owner-gated: clangd deferred by D-M9-2) | release-blocking for resolved C++ call edges |
| E7 | Reference-level resolution absent: a data member, a member read and a macro argument produce no edge, so `impact` on a field is empty and a trace cannot exceed one hop | **unsupported capability** (owner-gated) | release-blocking for workflow 4 |
| E8 | C/C++ include-root resolution: the candidate-caller scope of an anchor is built from import texts that mention the anchor's *repository-relative* path, but MMSP's includes are rooted at `colmap/src/`. Measured: **0 of 4 009** quoted C/C++ includes resolve; a unique-suffix rule would resolve **2 993**, 969 are ambiguous and 47 have no match | **implementation defect** — diagnosed, measured, **not fixed here** (see 10.3) | release-blocking for cross-TU C++ callers (workflow 2) |
| E9 | The warm-query p95 target (§14.3, ≤ 500 ms) is missed by 2.1–2.4× in the release profile; the cost is fixed per-invocation work (git fingerprint, process start, database open), not the graph | **diagnostic**, not a gate | owner decision on the target or on the fixed cost |
| E10 | The OpenCode free tier refuses nested CLI sessions in this environment, so the agent side of workflow 5 could not be re-measured | **environmental block** | G-MCP re-verification deferred; CP7-A G6 remains the last agent-side evidence |
| E11 | The repository's own history carries unsanitized local paths (1 068 findings in 9 commits / 184 files, 0 credential-rule findings), inherited from CP1 | **open risk**, unchanged | rewrite still owner-gated |

### 10.3 E8 — the smallest defensible options (owner decision required)

CP2 deliberately did **not** implement E8. The fix is small and would
clearly improve the product, but it changes the **import graph** of every
C++ repository, not a single fact, and CP1's E7 already classified
reference-graph work as owner-gated. The options, with their measured
effect on the pilot:

- **Option A — unique-suffix include resolution** (smallest): resolve a
  quoted C/C++ include against the indexed files by path suffix when exactly
  one file matches; leave ambiguous (969) and unmatched (47) includes
  unresolved. Effect on the pilot: 2 993 of 4 009 quoted includes become
  resolvable, so a header anchor's candidate caller set becomes non-empty
  and the two direct-initialization construction sites become reachable.
  Risk: a suffix match can pick the wrong file when the tree contains
  same-suffix paths; the uniqueness rule bounds this, and the pilot's 969
  ambiguous cases stay unresolved (honest, not wrong).
- **Option B — read the include roots from the build system** (correct, more
  work): parse `target_include_directories` from CMake, so `colmap/mvs/…`
  resolves against the real root rather than a suffix heuristic. Needs a
  build-system reader and its own evidence; it is the option that would also
  serve a future C++ semantic provider.
- **Option C — accept the syntax-only scope for C++** (no change): document
  that cross-TU C++ callers are unavailable and that the workflows are
  limited to same-file relations plus Python-style import scopes.

Any option leaves E1/E7 open: even with the include graph resolved, the
tool still cannot say *which* `Print` method a call refers to.

### 10.4 Recommended CP3 scope (not started; needs its own OWNER GO)

1. Decide E8 (A, B or C) and, if A or B, implement it with regression
   tests and re-measure the affected workflows.
2. Re-measure the workflows that E8 unblocks (2 and 3 first) and re-grade the
   capability gates.
3. Decide E9: restate the §14.3 warm-query target for a ~2 000-file C++
   repository, or reduce the fixed per-invocation cost.
4. Re-run the agent-side G-MCP smoke when the OpenCode free tier allows a
   nested session, or on a host with a paid provider.
5. Re-baseline the release-readiness matrix for the pinned MMSP snapshot
   (CP3 also covers Riltra self-indexing, the M6 corpus adequacy check and
   the remaining gate evidence).

---

## 11. Evidence index

| Artefact | Content |
|---|---|
| `pilots/cp2/README.md` | how to reproduce the run |
| `pilots/cp2/raw/queries/` | the 20 workflow queries, verbatim JSON, plus the class-anchor probes |
| `pilots/cp2/raw/freshness/` | F0–F5 `index status` and `repo status` documents |
| `pilots/cp2/raw/incremental/` | no-op, delta, oracle and the same-tree fact digests |
| `pilots/cp2/raw/perf/` | warm-query timings, debug and release |
| `pilots/cp2/raw/mcp/` | the MCP stdio smoke, the D.5 tool sequence, stdout capture |
| `pilots/cp2/raw/cold-index*.txt` | the cold-index runs (debug and release) |
| `pilots/cp2/method/` | the harness scripts used, and the environment record |

All archived evidence is redacted: machine-local paths are replaced with
`<CP2_BENCH>`, `<USERPROFILE>`, `<RILTRA_ROOT>` and `<MMSP_ROOT>`. No
credential, token or key is present.
