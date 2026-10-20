# M9 CP3 evidence — multi-repository verification and E8 correctness

CP3 runs on the owner-approved CP3 scope (owner brief, 2026-10-20; design
gate `docs/m9-design-gate.md` §4–6). This report records what was measured,
what was changed, and what remains open. The **M9 release-gate status** is
stated in §11, separately from CP3's own completion.

---

## 1. Git branch and source pins

| Item | Value |
|---|---|
| Repository | `RyanAraaful/riltra` |
| Branch | `m9-multirepo-release-candidate` |
| CP3 starting HEAD | `fca1185678de612552ad24a7932bb3f09f175507` |
| Remote branch at CP3 start (verified with `git ls-remote`) | `fca1185678de612552ad24a7932bb3f09f175507` — identical to local, no divergence |
| Master reference | `a5f67e6544f14d77774dfddf49e78ff10acd4966` (remote `refs/heads/master`, verified) |
| MMSP pilot pin | `d62f06a576d8e76b705c8ecc57fc380f7ebc9c44` |
| Riltra self-index input | `fca1185678de612552ad24a7932bb3f09f175507` (frozen; the implementation advances, the index input does not) |

The branch and its remote matched the expected starting point exactly, so
CP3 proceeded without a STOP. Every repository under test was an **isolated
clone**; the original MMSP and Riltra working trees were never modified.

### CP3 commits

| Commit | Subject |
|---|---|
| `5b19452` | `test(m9): archive the E8 include-resolution baseline and verified sample` |
| `54c7d26` | `fix(edges): widen the C/C++ caller candidate scope by unique include suffix` |
| `95265bd` | `test(m9): archive the E8 before/after re-measurement` |
| `d657abf` | `test(m9): archive the multi-repository isolation matrix` |

### Host toolchain

| Tool | Version |
|---|---|
| Rust / cargo | 1.99.1 (58cf8f9c 2026-10-13) |
| Git | 2.45.1.windows.1 |
| Node | 22.23.3 |
| gitleaks | 8.30.1 (installed from the release asset, SHA-256 verified against the published `gitleaks_8.30.1_checksums.txt`) |
| rust-analyzer | 1.99.1 (58cf8f9c 2026-10-13) — component was **absent** on this host and was installed with `rustup component add rust-analyzer`, the method M5 recorded |
| scip-typescript | 0.4.0 (TypeScript 5.9.3 as its bundled dependency) |
| C++ toolchain | **absent** — `g++`, `clang++`, `gcc`, `cl` and `cmake` are all missing |
| clangd | absent, by owner decision D-M9-2 |

---

## 2. E8 — hypothesis, verified sample, implementation decision

### 2.1 The defect, from the source

`crates/cli/src/index.rs` resolves an import to the anchor's path in
`import_mentions_path` (line 1446). For C/C++ it resolves the quoted literal
**relative to the importing file's own directory** and compares the result
with the anchor's repository-relative path. A C/C++ include is normally
written against an *include root* the repository does not record, so the
literal and the repository path almost never coincide.

The candidate scope built from that rule feeds only one branch of
`store::EdgeSelection::sql` (store.rs:1789): the `POTENTIAL_CALL` branch,
which additionally requires `callee_kind = 'unknown'` and
`callee_name = <the anchor's display name>`. It can never create a `CALLS`
row, never create a `REFERENCES` row, and never change an edge's kind.

### 2.2 The measured baseline — and a correction to CP2

`docs/pilots/cp3/method/e8-baseline.cjs` classifies every quoted C/C++
include in the pilot under the rules that matter.

| Rule | Resolves |
|---|---|
| `probe_rule` — the literal **is** the repository-relative path (**what CP2 measured**) | **0** |
| `code_rule` — the literal resolved **relative to the importer's directory** (**what the product applies**) | **1 353** |
| `suffix_unique` — exactly one indexed path equals the literal or ends with `/` + literal | **2 993** |
| `suffix_ambiguous` | 969 |
| `suffix_none` | 47 |

CP2's headline "0 of 4 009 quoted C/C++ includes resolve" is **true of CP2's
own probe rule and false of the product's rule**. The product already
resolves 1 353 of the 4 009 (33.7 %) through same-directory includes. The
2 993 suffix-unique rows therefore contain **2 609 rows the product does not
resolve today**. CP2's other three counts (4 009 quoted rows, 2 993 unique,
969 ambiguous, 47 unmatched) reproduce exactly.

### 2.3 Which matches are genuinely the intended source

CP2's counts are a projection. CP3 computed the ground truth: a C++ compiler
resolves a **quoted** include by searching (1) the including file's own
directory, then (2) each `-I` root in order. The pilot's include root is
`colmap/src` (`colmap/CMakeLists.txt:309` `include_directories(src)` and
`:495` `target_include_directories(… ${CMAKE_CURRENT_SOURCE_DIR}/src)`),
archived verbatim in `docs/pilots/cp3/raw/e8/include-roots.txt`. No C++
compiler is installed on this host, so the compiler's own `-H` trace could
not be used; the CMake evidence plus the documented search order is the
strongest available ground truth.

| Verdict over the 2 609 **new** candidate rows | Count |
|---|---|
| the suffix match **is** the file the compiler opens | **2 608** |
| the compiler opens a **different** file | 0 |
| the compiler would open no repository file | **1** |

The single failure is the one row whose literal is a **bare filename**:

```
colmap/src/thirdparty/VLFeat/lbp.c   #include "string.h"
  suffix claim : colmap/src/colmap/util/string.h
  compiler opens: nothing in the repository (the C library's string.h)
```

That is exactly the shadowing class the brief warns about. Excluding
bare-filename literals from the suffix rule removes **100 %** of the
observed misleading matches while keeping 2 608 / 2 609 (99.96 %) of the
new candidates.

The verified sample, with every class, is archived in
`docs/pilots/cp3/raw/e8/SAMPLE.md`:

| Class | Sample | Verified by |
|---|---|---|
| true match | `exe/mvs.cc` → `"colmap/mvs/patch_match.h"` → `colmap/src/colmap/mvs/patch_match.h` | the compiler's search order lands there; the header declares `class PatchMatch`, `struct Problem`, `void Print() const;`, `class PatchMatchController` |
| true match | `controllers/automatic_reconstruction.cc` → `"colmap/mvs/patch_match.h"` | same |
| true match | `ui/dense_reconstruction_widget.cc` → `"colmap/mvs/patch_match.h"` | same |
| true match | `patch_match_options.cc` → `"colmap/mvs/patch_match_options.h"` | same |
| **incorrect potential match** | `thirdparty/VLFeat/lbp.c` → `"string.h"` | the compiler opens the system header |
| ambiguous | `"shared_indices.h"` → `…/generated/f32/shared_indices.h` **and** `…/generated/f64/shared_indices.h` | two indexed files |
| missing | `"thirdparty/SiftGPU/SiftGPU.h"`, `"thirdparty/LSD/lsd.h"` | absent from the repository (fetched by the build system) |
| missing | `"Array.inl"` and 39 siblings | **present in the tree but not indexed** — `.inl` is not a recognised extension |

### 2.4 The frozen CP1 anchors

CP1 B.2 records exactly three external `PatchMatchController` construction
sites: `exe/mvs.cc:271`, `controllers/automatic_reconstruction.cc:431`,
`ui/dense_reconstruction_widget.cc:397`. All three translation units
`#include "colmap/mvs/patch_match.h"`, and the compiler opens exactly the
file that declares `PatchMatchController`. The suffix rule therefore
reproduces the CP1 ground truth; today's exact rule resolves **none** of
these anchors (`today_exact_rule` is null for every one of them in
`docs/pilots/cp3/raw/e8/anchors.json`).

### 2.5 The decision

**Option A was implemented**, and the implementation is deliberately
narrower than the projection CP2 published:

- the existing exact rule is preserved unchanged;
- the suffix rule applies to **C/C++ only**; Python, JavaScript,
  TypeScript and Rust keep their own relative-import semantics;
- only **quoted** includes are considered — an angle-bracket include names
  a root or system header, which a repository path suffix can never
  establish;
- the literal must be **rooted** (contain a `/`), which is what excludes the
  `string.h` class;
- the literal must match **exactly one** indexed path at a **component
  boundary**; ambiguity is never resolved arbitrarily;
- the matched path must be the anchor itself;
- separators are normalised to `/`, so a Windows-authored include resolves
  the same way on every host.

The rule widens a **candidate scope**. It cannot promote a `POTENTIAL_CALL`
into a `CALLS`, cannot create a `REFERENCES` row, and cannot change an edge
kind — structurally, because the candidate paths feed only the
`POTENTIAL_CALL` branch of the query predicate.

---

## 3. Before/after E8 results

Same pilot index, same 28-query set, two binaries. The index is **not**
rebuilt between the runs: the correction is query-side, so the diff
isolates the change exactly.

| Measure | Value |
|---|---|
| queries run | 28 (CP2's set verbatim + 6 E8 anchors) |
| queries whose answer changed | **4** |
| queries byte-identical | **24** |
| candidate rows added | **5** |
| candidate rows removed | 0 |
| **`CALLS` rows added** | **0** |
| **`REFERENCES` rows added** | **0** |
| any other edge kind added | 0 |

Every changed query is a `callers` query whose anchor is a C++ header. The
`callees`, `trace`, `impact`, `search` and status answers are byte-identical:
the correction cannot reach them.

### The 5 added candidates, each verified against the source

| Anchor | Added candidate caller | Verification |
|---|---|---|
| `mvs/patch_match.h` (`PatchMatchController`) | `controllers/automatic_reconstruction.cc` | includes `"colmap/mvs/patch_match.h"`; constructs `mvs::PatchMatchController` at `:431` |
| `mvs/patch_match_options.h` (`PatchMatchOptions`) | `controllers/option_manager.cc` | includes `"colmap/mvs/patch_match_options.h"`; constructs `mvs::PatchMatchOptions` at `:85` — the owner CP1 B.3 verified as the bound object's owner |
| `mvs/patch_match_options.h` (`PatchMatchOptions`) | `pycolmap/pipeline/mvs.cc` | includes `"colmap/mvs/patch_match_options.h"`; references `mvs::PatchMatchOptions` |

**Incorrect candidate additions: 0.** Every added row is `kind:
POTENTIAL_CALL`, `precision: Heuristic`, `provider: syntax`,
`unresolved: dynamic`, `callee: null`.

### Expected relations still missing — and why

| CP1 site | Form | Why still missing | Class |
|---|---|---|---|
| `exe/mvs.cc:271` — `mvs::PatchMatchController controller(options, …)` | direct initialization whose argument list the grammar reads as a parameter-type-list | the syntax layer records **no** call fact for the most-vexing-parse form | **E12** |
| `ui/dense_reconstruction_widget.cc:397-398` — `std::make_shared<mvs::PatchMatchController>(…)` | the name is a **template argument**; the callee is `make_shared` | the callee terminal is `make_shared`; resolving the argument needs a name lookup | **E1 / E7** |

**E8 closes none of E1, E7 or E12.** An improved candidate set is not
semantic resolution: the added rows are candidates, and the tool says so.

### Cost, freshness and incremental effects

`docs/pilots/cp3/method/profile.ps1` ran 7 repetitions per stage on both
binaries (medians, ms):

| Stage | Before | After | Δ |
|---|---|---|---|
| process start and exit | 8.7 | 8.7 | 0.0 |
| git fingerprint collection | 270.2 | 271.9 | +1.7 |
| database open + snapshot validation | 185.5 | 191.9 | +6.4 |
| query execution (`search`) | 6.5 | −5.0 | noise |
| query execution (`callers`, the E8 path) | 293.3 | 287.1 | −6.2 |

Every difference is inside the run-to-run spread (min-to-max ≈ 20–40 ms).
**The E8 correction adds no measurable cost.** Freshness and incremental
behaviour are untouched: the change is query-side, and the incremental
parity run in §6.4 shows the stored facts are bit-identical.

---

## 4. Multi-repository isolation matrix

One isolated `RILTRA_HOME`, exactly two registered repositories, each in its
own isolated clone.

| | `mmsp-pin` | `riltra-self` |
|---|---|---|
| root | isolated clone of MMSP | isolated clone of Riltra |
| pin | `d62f06a576d8e76b705c8ecc57fc380f7ebc9c44` | `fca1185678de612552ad24a7932bb3f09f175507` |
| indexed files | 1 438 | 152 |
| symbols | 11 344 | 2 211 |
| imports | 6 680 | 208 |

| # | Check | Result |
|---|---|---|
| C1 | both repositories register, each with its own root | **PASS** |
| C2 | each repository has its own fresh index at its own head | **PASS** — both `fresh`, reasons empty |
| C3 | a query targets the selected repository | **PASS** — `search --symbol main`: `mmsp-pin` 107 hits, `riltra-self` 10 hits, **0** foreign paths in either answer |
| C4 | callers, callees, trace and impact never cross the boundary | **PASS** — 6 graph queries, **0** foreign path mentions |
| C5 | freshness is independent per repository | **PASS** — editing one indexed MMSP source file makes `mmsp-pin` **stale** (`dirty;content-changed`) while `riltra-self` stays **fresh**; after the restore both are fresh |
| C6 | generation, snapshot and citation belong to one repository | **PASS** — every answer carries its own `repo` name and its own `snapshot.head` |
| C7 | read-only MCP tools create or migrate no store | **PASS** (with the accepted WAL caveat) |
| C8 | a junction that leaves the repository root is not followed | **PASS** |

**Identically named symbols.** 27 symbol names exist in **both**
repositories (`main`, `build`, `evaluate`, `Index`, `update`, `load`,
`run`, `PatchMatch::Problem::Print`, …); `main` is the widest, 30 MMSP paths
and 10 Riltra paths. The two stores share **0** repository-relative paths.

**C7 — read-only MCP purity.** Against an **empty** home, a read-only MCP
session creates **nothing** (`home_unchanged: true`, 0 files), exposes the
11-tool surface, keeps stdout 6/6 JSON lines with 0 non-JSON lines, and
returns a classified `not_registered` error for an unknown repository.
Against the **populated** home it leaves two extra files,
`mmsp-pin.db-shm` and `mmsp-pin.db-wal`. These are **SQLite's own WAL
sidecars** for a store that already existed: no store file is created, the
store file's size is unchanged (50 020 352 bytes before and after), so **no
schema migration ran**. This is the already-accepted WAL behaviour,
accurately stated — not a read tool creating or migrating a store.

**C8 — escape refused.** A junction inside the MMSP clone pointing outside
the root: `scan --path …/cp3-escape/secret.h` → exit 3, `error: --path
escapes repository root`; the index does not pick the outside file up
(`search --symbol Secret` → `total: 0`); `doctor` exits 0. The link was
removed, the pinned tree restored (`HEAD` = `d62f06a…`, 0 dirty lines) and
re-indexed.

**Scope.** Riltra self-indexing is **internal validation**. It shows that
one binary, one home and one code path keep two repositories apart. It is
**not** independent evidence about an unrelated third codebase. No third
pilot repository was added.

---

## 5. M6 multilingual corpus adequacy

The existing corpus was reviewed, not enlarged.

| Language | Pairs | Scenarios |
|---|---|---|
| Python | 16 | direct, reference, nested_calls, nested_anonymous, local_scope, same_name, alias, recursion, cycle, missing_definition, dynamic_dispatch, unresolved, stale_after_edit, unsupported, competing_targets, same_name_binding |
| TypeScript | 16 | same 16 |
| Rust | 16 | same 16 |
| C++ | 16 | same 16 |
| C | 15 | the same 15 **without** `nested_anonymous` |
| **JavaScript** | **0** | — |
| **total** | **79** | |

**Adequacy for the CP3 question.** The corpus covers the four families the
brief names (Python, TypeScript/JavaScript, Rust, C/C++) for the graph
contract — with one real gap:

- **JavaScript has no corpus pair.** The TypeScript corpus is not evidence
  for JavaScript: the grammar, the import model and the call shape differ.
  JavaScript's only regression evidence is the M2 golden
  (`golden/m2-js-hello.json`), a single syntax fact sample with no graph
  contract at all. **This is a real coverage gap, reported rather than
  manufactured into a new fixture**: adding 16 JS corpus pairs is a corpus
  change with its own evidence, not a CP3 observation.
- **C has no `nested_anonymous` scenario.** Deliberate: the C grammar has no
  anonymous nested function form, so there is nothing to assert.

No other gap was found: the corpus's 16 scenarios are the honest contract
surface, and every pair asserts the **full JSON golden** plus source-truth
assertions, so it fails on drift, not on counts.

---

## 6. Updated gate evidence

### 6.1 G-SYN — source-verified syntax

**Regression: PASS.** `cargo test --locked` is green on the candidate:
360 + 16 + 144 + 21 + 52 + 44 + 15 + 12 = **664 tests, 0 failures**. The
M6 corpus runs inside `tests/edges.rs` (144 tests), one per pair, each
asserting the full JSON golden and the source-truth spans.

**MMSP re-scan: PASS.** The pinned tree scans deterministically: 2 277 files
discovered, 1 438 processed, 835 unsupported, 4 skipped as binary, 1 991
parse errors counted, `errors` recorded. Repeat runs are byte-identical.

**New finding — a coverage gap, not a defect.** 40 of the 47 unmatched
includes are **`.inl` files present in the tree but not indexed**:
`crates/cli/src/syntax.rs` recognises `h, hh, hpp, hxx, cc, cpp, cxx` only.
`.inl` is a real C++ header extension used by PoissonRecon. This is a G-SYN
coverage gap, recorded in §9; it was **not** fixed, because widening the
recognised extensions changes what the index contains and is outside the
authorized Option A scope.

CUDA remains explicitly unsupported: `.cu`/`.cuh` are refused and never
counted as coverage.

### 6.2 G-SEM — semantic providers

**Two providers, two language families, revalidated on the candidate.**
`docs/pilots/cp3/method/g-sem.ps1`:

| Provider | Family | Result |
|---|---|---|
| rust-analyzer 1.99.1 (LSP subprocess) | Rust | **PASS** — the definition of `greet` resolves to `src/main.rs` bytes **3..8**, exactly `greet` in `fn greet(name: &str) -> String {`; the references resolve to bytes **96..101** (`greet("world")`), **127..132** (`greet("mars")`) and the declaration 3..8 |
| scip-typescript 0.4.0 (SCIP producer) | TypeScript | **PASS** — import: 2 documents / 4 definitions / 6 references, 0 skipped, `fallback_reason: none`, provider `scip-typescript` 0.4.0; the references resolve to bytes **9..14** (the import binding) and **46..51** (the call site) with the symbol URI `scip-typescript npm tierb-ts 1.0.0 src/greet.ts/greet().` |

Every definition and reference was checked against the fixture source **by
byte range**, not by count. Verification is `Unverified` throughout, which
is the M5 rule; `Current` remains unreachable.

TypeScript satisfies the "at least one of them C/C++ or TypeScript" clause
of G-SEM.

**Honest limitations.**

- The **rust-analyzer component was absent** on this host (only a stale
  rustup shim was present). It was installed with `rustup component add
  rust-analyzer` — the method M5 recorded — and the version is recorded in
  `docs/pilots/cp3/raw/gates/gsem/provider-versions.json`. The install is a
  host toolchain change, not a repository change.
- The **first** rust-analyzer query on a cold server returns empty with the
  diagnostic `analysis wait timed out; answers may be empty`: the bounded
  analysis wait is capped at 15 s (`crates/cli/src/lsp.rs:599`) and a cold
  rust-analyzer needs at least that on this host. The second query on the
  now-warm metadata cache answered correctly. The diagnostic is surfaced
  honestly by the tool; it is **not** a silent empty. This is the cold-start
  limitation M5 already recorded, observed again.
- **C++ semantics on MMSP remain unproven**: no clangd (owner decision
  D-M9-2), no compilation database in the pilot, and no C++ provider has
  ever run. Recorded as **NOT YET VERIFIED**, not as a pass.
- **CUDA semantics remain unsupported.**
- No syntax candidate was promoted under a semantic label; the
  `POTENTIAL_CALL` rows added in §3 carry `provider: syntax` and
  `callee: null`.

### 6.3 G-GRF — graph correctness and safety

**PASS on the CP3 scope.**

- **No promotion**: the E8 change adds 5 `POTENTIAL_CALL` rows and **0**
  `CALLS` rows (§3).
- **No fabricated references**: **0** `REFERENCES` rows added; a
  `REFERENCES` edge is only ever created from a SCIP fact.
- **Honest labels**: every added row is `precision: Heuristic`,
  `unresolved: dynamic`, `callee: null`.
- **Explicit disclaimers**: the completeness block keeps reporting
  `status: partial` with the real unresolved counts
  (`unresolved_target: 91 025`, `dynamic_calls: 45 663`,
  `ambiguous_calls: 194`, `parse_error_files: 151`, `unsupported_files:
  839`).
- **Bounded**: the query budgets and the `TRACE_MAX_*` limits are unchanged
  and the 24 unchanged queries include the bounded `trace` and `impact`
  answers byte-for-byte.
- **No false `SAFE`**: `impact` answers are unaffected by the E8 change
  (byte-identical before and after); the conservative `UNKNOWN`-first policy
  is untouched.
- The M6 corpus (79 pairs, each asserting the full JSON golden) is green.

### 6.4 G-FSH — freshness, and G-INC — incremental correctness

**G-FSH: PASS.** Both repositories report `fresh` at their own pins.
Freshness is **independent per repository** (C5): a change in the MMSP
clone makes only `mmsp-pin` stale; `riltra-self` stays fresh, and the
restore returns both to fresh. The reasons vocabulary is unchanged
(`dirty;content-changed`).

**G-INC: PASS** — the CP1 five-point parity contract, re-run on the
candidate with the CP2 harness (`docs/pilots/cp2/method/incremental.ps1`,
archived under `docs/pilots/cp3/raw/gates/incremental/`):

| Point | Result |
|---|---|
| restore to the pinned tree | HEAD `d62f06a…`, 0 porcelain lines |
| baseline index of the pinned tree | mode `incremental`, 0 reparsed |
| change exactly one file | `patch_match_options.h`, 1 changed line, digest recorded |
| incremental indexing on the changed tree | mode `incremental`, **reused 1 437, reparsed 1** |
| independent full rebuild of the **same** changed tree | mode `full`, 1 438 reparsed, fresh home |
| fact digests of both indexes, taken **before** the restore | **IDENTICAL** — files 1 438/1 438, symbols 12 105/12 105, imports 6 680/6 680, calls 91 025/91 025, **edges 97 705/97 705**; combined digest `2389cd32e702b479e48b0b3a9ca5e452aa379406058b063b79e619dfcc18756c` on both sides |
| query parity on the same state | **identical** for all 12 queries (search, callers, callees, trace, impact, index status) |
| restore and verify | HEAD `d62f06a…`, 0 porcelain lines, re-indexed |

The **edges** table is identical too, which is the direct proof that the E8
correction changed no indexed fact.

### 6.5 G-KNW — knowledge graph

**PASS (regression).** The 52 knowledge-store tests are green, including
`independent_stores_and_cross_repo_rejection`. The M7 "CLOSED" status is
preserved.

**No benchmark knowledge record was written into a canonical user store.**
The CP3 harness creates only throwaway fixture homes under the bench root
(`home-sem`, `home-incremental`, `home-mcp-empty`); the knowledge store was
never written to by CP3.

### 6.6 G-SEC — filesystem, repository isolation, recovery

**PASS.**

- **Filesystem confinement**: the junction escape is refused (C8); the
  existing path-containment, symlink and Windows-junction tests are green
  (part of the 664).
- **Repository isolation**: the two-repository matrix (C1–C6) is green, and
  the existing `two_repositories_stay_isolated`,
  `incremental_two_repositories_stay_isolated` and
  `cli_cross_repo_isolation` tests are green.
- **Non-mutating reads**: the empty-home MCP session creates **0** files
  (C7).
- **WAL limitation**: stated accurately in §4 — the sidecars are SQLite's,
  no store is created or migrated, and the store file's size is unchanged.
- All M8 / CP7-A historical evidence is preserved unchanged.

### 6.7 G-TOK — token / task effectiveness

**CP7-A's evidence is retained unchanged and was not re-run or
reinterpreted.** No CP3 measurement re-derives a CP7-A number.

**Whether CP3 could materially affect it.** The only product change is the
E8 candidate scope, which:

- adds at most a handful of `POTENTIAL_CALL` rows to a `callers` answer
  (5 rows across the whole pilot's 28-query set);
- changes **no** MCP tool's schema, name, argument set or response
  envelope;
- leaves the `search`, `callees`, `trace` and `impact` answers
  **byte-identical**.

The MCP surface is unchanged: 11 tools, the same names and arguments. So
CP3 **cannot materially change** MCP responses or token consumption, except
by adding the candidate rows themselves to the affected `callers` answers.
The formal release-evidence applicability review is left to CP4, as
instructed.

---

## 7. G-MCP and G-TOK status with evidence limitations

### G-MCP / E10 — **the agent side is now measurable**

CP2 recorded E10 as **BLOCKED** (`provider-auth-403`: "OpenCode's free tier
can only be used from within OpenCode"). On this host the free tier is now
reachable from within OpenCode, so the gate could be measured. No paid
subscription was used and no free-tier restriction was bypassed.

A real OpenCode session ran against the candidate's `riltra mcp` server, in
a session directory whose only configuration is the Riltra MCP server, with
the agent constrained to the MCP tools.

| Check | Result |
|---|---|
| session outcome | **exit 0**, 62 transcript lines, 20 tool-use parts |
| tools used | `riltra_repo_list`, `riltra_code_search` ×10, `riltra_code_callees` ×3, `riltra_code_callers` ×2, `riltra_code_trace`, `riltra_repo_status` |
| repository selection | the agent identified the registry alias `mmsp-pin` from `riltra_repo_list` and queried only that |
| classified error | surfaced verbatim, one call, **not retried**: `{"kind":"not_registered","message":"unknown repository: 'mmsp-no-such-repo'"}` |
| freshness | reported: `"freshness": "fresh"` and `snapshot.status_at_write: "fresh"` at head `d62f06a…` |
| source citation | 9 repository-relative paths cited, each as the tool reported it |
| honesty | 6 explicit disclaimers; the agent **refused to answer** the option-wiring question and said so explicitly rather than guessing |

The agent's own conclusion is the most valuable output: it reported that
`search` cannot name the `max_triangulation_angle` **data member** and
explicitly wrote *"I cannot answer how `max_triangulation_angle` is wired
from the command line into source-image selection, and cannot state its
default"*. That is **E7** confirmed from the agent side, from the tool
surface, not from a fixture.

**Limitations of this evidence.** One session, one repository, one host, one
model. The agent side is now *measurable*; it is not yet a repeated
measurement, and it does not re-open CP7-A's frozen archive. The transport
side is unchanged from the revalidated smoke: 11 tools, stdout pure, health
OK, home unchanged apart from SQLite's WAL sidecars.

### G-TOK

See §6.7. CP7-A retained unchanged; CP3's impact on MCP responses and token
consumption is recorded as **not material**.

---

## 8. E9 diagnostic findings

Bounded profiling with existing tooling only
(`docs/pilots/cp3/method/profile.ps1`): 7 repetitions per stage, one
discarded warm-up, medians on an otherwise idle host.

| Stage | Median | min–max | Share of the `callers` invocation |
|---|---|---|---|
| process start and exit | **8.7 ms** | 7.9–10.4 | 1.1 % |
| **git fingerprint collection** | **271.9 ms** | 274.2–286.5 | **35.8 %** |
| **database open + snapshot validation** | **191.9 ms** | 454.3–492.8 | **25.3 %** |
| query execution (`search`) | below the resolution of this method | 461.4–487.0 | ≈ 0 % |
| query execution (`callers`, the E8 path) | **287.1 ms** | 742.9–773.3 | 37.8 % |

The stages are separated by **differencing** the cost of successively
heavier commands, because the CLI has no built-in stage timer; the search
difference (−5.0 ms before, +6.5 ms after) is **below the resolution of the
method** and is reported as such rather than as a negative cost.

**The CP2 hypothesis is confirmed and now measured.** The warm-query miss
is **not** a graph problem: a **fixed per-invocation cost of ≈ 472 ms**
(62 % of the whole `callers` invocation) dominates — the `git` fingerprint
subprocess (~272 ms) and the database open with snapshot validation
(~192 ms). Process start is negligible (~9 ms). The graph query itself
costs ~287 ms for the heaviest query.

**What was not done.** The blueprint's SLO was not changed, and no cache,
daemon, service or performance refactor was implemented. CP2's §14.3 target
(≤ 500 ms warm) is met by `search` on this host and missed by `callers`
(≈ 1.52×) — but the measurement above says the miss is dominated by a fixed
cost that is independent of the query, which is the decision the owner
asked for ("profile first, then either the target or the fixed cost").

**Not resolved.** The split *inside* the 272 ms git stage — how much is the
`git` process spawn, how much is the worktree scan of 2 277 files — was not
separated; that needs an in-process breakdown that the current tooling does
not provide. It is recorded as unresolved rather than guessed.

---

## 9. Remaining functional blockers and recommended next decisions

| # | Item | Status | Recommended next decision |
|---|---|---|---|
| E1 | C++ semantics on MMSP: no provider, no compilation database | unsupported capability, owner-gated (D-M9-2) | **No change in CP3.** The decision is deferred until baseline MMSP correctness evidence shows a justified need. The E8 work strengthens the case for cross-TU *candidates* but not for a semantic provider. |
| E7 | Reference-level resolution: a data member, a member read and a macro argument produce no edge | unsupported capability, owner-gated | **Decide.** The OpenCode session now demonstrates the user-visible cost directly: the agent could not answer a plain "where is this option wired" question because the field is not a symbol. This is the highest-value remaining gap. |
| E12 | The most-vexing-parse form cannot be decided from syntax | unsupported capability | **Decide.** One of the three CP1 `PatchMatchController` construction sites is unreachable for this reason alone, and E8 cannot reach it. |
| E8 | C/C++ include-root resolution | **implementation defect — fixed in CP3** (Option A, narrowed) | None. The residual limit — a unique suffix is a candidate, not the compiler's choice — is documented in the code with a `shortcut:` comment naming the upgrade path (CP2's Option B: read the build system's include roots). |
| **new** | **`.inl` headers are not indexed** | **G-SYN coverage gap, found in CP3** | **Decide.** 40 of the pilot's 47 unmatched includes are `.inl` files present in the tree. Widening `syntax.rs`'s recognised extensions is a one-line change with an index-content consequence; it was deliberately **not** done inside Option A. |
| E9 | Warm-query SLO missed | **diagnostic, now measured** | **Decide.** The fixed cost is ~472 ms of a ~760 ms invocation and is dominated by the `git` fingerprint subprocess (~272 ms). The owner's CP2 instruction was "profile first, then either the target or the fixed cost"; the profiling is done. |
| E10 | The agent side of workflow 5 | **no longer blocked** — measured in CP3 | **No change.** One session is evidence that the gate is measurable, not a repeated measurement. |
| E11 | The repository's history carries unsanitised local paths | open risk, unchanged from CP1 | **Owner-gated** (history rewrite). Untouched by CP3. |

---

## 10. CP3 assessment against its actual scope

| Phase | Scope | Result |
|---|---|---|
| 1 | E8 source-grounded evaluation | **COMPLETE** — the caller candidate scope and import matching located and explained from source; baseline measured; a verified sample of true / incorrect / ambiguous / missing classes prepared; the frozen CP1 anchors included; a before-change baseline recorded. **One CP2 figure corrected** (§2.2). |
| 2 | Conditional minimal E8 implementation | **COMPLETE — Option A implemented**, narrower than projected (rooted literals only). Regression tests: 3 unit tests + 7 CLI-level tests covering correct discovery, ambiguous suffixes, misleading unique matches, missing paths, differing include roots, Windows/Unix normalisation and cross-repository isolation. The red state was proven: with the source change stashed, `e8_rooted_include_adds_candidate_caller` fails. No new schema, database, dependency, CMake parser or external provider. |
| 3 | Re-measure E8 impact | **COMPLETE** — 28 queries on one index, 4 changed, 5 candidates added, 0 removed, **0 `CALLS` and 0 `REFERENCES` added**, every added row verified against the source, missing expected relations classified to E1/E7/E12, cost unchanged within noise. |
| 4 | Multi-repository verification | **COMPLETE** — one isolated home, exactly two repositories, eight checks, all PASS. Riltra self-indexing labelled internal validation. No third pilot repository. |
| 5 | Remaining CP3 gate verification | **COMPLETE** — G-SYN, G-SEM, G-GRF, G-FSH, G-INC, G-KNW, G-SEC re-verified with archived evidence; G-TOK recorded without re-running CP7-A; G-MCP/E10 measured for the first time. |
| 6 | E9 diagnostic profiling | **COMPLETE** — the four stages separated by differencing, with medians and spread; the bottleneck identified; the blueprint SLO untouched; no speculative optimisation. |
| — | CP3 evidence report and archive | **COMPLETE** — this report plus `docs/pilots/cp3/` (method scripts, redacted raw evidence). |

### **CP3 verdict: PASS against its own scope.**

Every phase in the owner's CP3 brief was completed, and every claim in this
report is backed by archived, redacted evidence in `docs/pilots/cp3/`.
CP3 passed **its own** scope. It did **not** make any release gate pass, and
it does not claim that it did.

---

## 11. M9 release-gate status (stated separately from CP3)

CP3's PASS is about CP3. The M9 release gates are a different question, and
none of them is lowered by this work. A historical milestone being green is
not a release gate passing.

| Gate | Status after CP3 | Basis |
|---|---|---|
| G-LIC | **NOT YET VERIFIED** | outside the CP3 scope; no SBOM, no release artifact |
| G-NAT | **NOT YET VERIFIED** | outside the CP3 scope; the 3-OS CI run is the evidence and is awaited at the end of CP3 |
| G-CLI | **NOT YET VERIFIED** | the candidate suite and goldens are green and unchanged (§6.1), but the release-candidate run and the written stability policy do not exist |
| G-MCP | **PARTIALLY RE-VERIFIED** | the transport and tool behaviour are revalidated and the agent side is now measured (§7); the release-candidate binary, the documented OpenCode config and the archived smoke transcript for the candidate are still outstanding |
| G-SYN | **NOT YET VERIFIED — one gap found** | the corpus, the goldens and the deterministic MMSP scan are re-verified; the `.inl` extension gap (§9) is open; the release-candidate scan report is still to be archived |
| G-SEM | **PARTIALLY RE-VERIFIED** | the two-provider/two-family minimum is re-verified on the candidate with source-grounded evidence (§6.2); C++ on MMSP is **NOT YET VERIFIED** with the D-M9-2 deferral reason; CUDA remains unsupported |
| G-GRF | **NOT YET VERIFIED** | the graph contract is re-verified on the pilot and the corpus (§6.3); the release-candidate run against the frozen MMSP ground truth is still to be archived as the release artefact |
| G-FSH | **NOT YET VERIFIED** | freshness is re-verified including per-repository independence (§6.4); the release matrix across the release candidate is still to be archived |
| G-INC | **NOT YET VERIFIED** | incremental/full parity is re-verified with identical digests (§6.4); the release-candidate performance comparison is still to be archived |
| G-KNW | **NOT YET VERIFIED (release)** | M7 is CLOSED and the regression is green (§6.5); the release-level review is still to be run |
| G-TOK | **NOT YET VERIFIED** | CP7-A is retained unchanged and CP3's impact is recorded as not material (§6.7); the formal release-evidence applicability review is **left to CP4** as instructed |
| G-SEC | **NOT YET VERIFIED** | confinement, isolation and non-mutating reads are re-verified (§6.6); the release-level review is still to be run |
| G-PAR | not an M9 gate | — |

**The release gate status is materially unchanged by CP3.** CP3 closed one
implementation defect (E8) and turned one blocked gate (E10) into a
measured one; it did not make any release gate pass, and it identified one
new coverage gap (`.inl`) that must be decided before G-SYN can close.

---

## 12. Archive index

| Artefact | Content |
|---|---|
| `docs/pilots/cp3/README.md` | the archive overview and how to reproduce the run |
| `docs/pilots/cp3/method/e8-baseline.cjs` | the E8 baseline and the compiler-equivalent ground truth |
| `docs/pilots/cp3/method/e8-missing-class.cjs` | classifies the unmatched includes against the real git tree |
| `docs/pilots/cp3/method/e8-anchor.cjs` | the frozen CP1 anchors under the E8 rules |
| `docs/pilots/cp3/method/e8-diff.cjs` | the before/after edge diff |
| `docs/pilots/cp3/method/queries.ps1` | the 28-query before/after set |
| `docs/pilots/cp3/method/multirepo.ps1` | the multi-repository isolation matrix |
| `docs/pilots/cp3/method/multirepo-probe.cjs` | the store-level two-repository comparison |
| `docs/pilots/cp3/method/g-sem.ps1` | the two-provider revalidation |
| `docs/pilots/cp3/method/profile.ps1` | the E9 stage profiler |
| `docs/pilots/cp3/method/opencode-summary.cjs` | the OpenCode session summary |
| `docs/pilots/cp3/raw/e8/SAMPLE.md` | the verified E8 sample, every class |
| `docs/pilots/cp3/raw/e8/baseline.json` | the E8 baseline counts and the samples |
| `docs/pilots/cp3/raw/e8/missing-class.json` | the 47 unmatched includes, classified |
| `docs/pilots/cp3/raw/e8/anchors.json` | the frozen CP1 anchors under the E8 rules |
| `docs/pilots/cp3/raw/e8/include-roots.txt` | the include-root evidence, verbatim from the pinned tree |
| `docs/pilots/cp3/raw/e8/remeasurement.md` | the before/after result and the missing-relation analysis |
| `docs/pilots/cp3/raw/queries/` | the before/after answers, the pairing table and the diff |
| `docs/pilots/cp3/raw/multirepo/matrix.md` | the isolation matrix and its evidence |
| `docs/pilots/cp3/raw/multirepo/probe.json` | the store-level comparison |
| `docs/pilots/cp3/raw/gates/perf/`, `perf-before/` | the E9 stage profiles for both binaries |
| `docs/pilots/cp3/raw/gates/gsem/` | the two-provider evidence |
| `docs/pilots/cp3/raw/gates/incremental/` | the incremental/full parity evidence |
| `docs/pilots/cp3/raw/gates/opencode-session.stdout.json` | the real OpenCode session transcript |
| `docs/pilots/cp3/raw/gates/opencode-summary.json` | the machine-readable session summary |

All archived evidence is redacted with `docs/pilots/cp2/method/redact.cjs`
(machine-local paths replaced). No credential, token or key is present.
