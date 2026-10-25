# M9 grader correction and dated amendment (2026-10-25)

**Status: grader predicates corrected; no criterion, frozen specification or
historical result changed.** This follows the owner decision of 2026-10-25 to
audit and correct findings G1 to G5 of
[docs/m9-task5-capability-gap-evaluation.md](m9-task5-capability-gap-evaluation.md)
under these limits:

- fix a predicate only where it is proven wrong against the MMSP source;
- keep the frozen specification and the historical results;
- change a criterion only through a dated amendment (this document);
- keep grader bug corrections separate from capability requirement changes;
- do not move a gate from UNMET to PASS without source-grounded evidence;
- implement no clangd, compilation database or graph feature.

All four capability gates (D1-C6, D2-C6, D3-C5, D4-C5) remain UNMET at HEAD
`e911052`. No PASS was introduced.

## 1. Audit of G1 to G5

The classes are the ones the owner asked to keep apart. "Grader bug" means the
predicate does not implement the frozen text it claims to test. "Requirement"
means the frozen text itself would have to change. "Record" means a note on an
existing document.

| ID | Finding | Evidence | Class | Action |
|---|---|---|---|---|
| G1 | D1-C6 compared `caller_span[0]`, a byte offset, with line numbers. It could not match any line. It also required `fusion.cc:144` to be returned, which an identity-resolved answer should not do. | Archived `grading.json` lists sites such as `patch_match.cc:26058`. Line 667 of `patch_match.cc` starts at byte 26058 (checked with the line mapper). Frozen D.1 #6. | Grader bug: unit mismatch. Grader bug: a presence test from the honesty side placed in the capability check. | Corrected. Lines come from the source. The capability needs a proven edge at :667 to the anchored definition. A proven edge at :668 or :144 to it is a FAIL. The B.1 candidate listing is no longer graded (question Q3). |
| G2 | D2-C6 passed when the three files appeared as callers, even as name candidates. | Red test: the old predicate passes on three candidates. Frozen D.2 #5 requires identity resolution. | Grader bug | Corrected. Each site needs a proven edge to the class anchor, on its frozen lines, with the class name on that line. |
| G3 | D3-C5 passed on any path with three edges. | Red test: the old predicate passes on three proven edges that are not the chain. Frozen D.3 #5 and B.3. | Grader bug | Corrected. The predicate checks the frozen chain hop by hop (§2). |
| G4 | D4-C5 passed on any non-empty affected set. | Red test: the old predicate passes on one unrelated row and on a candidate at a frozen site. Frozen D.4 #5 and B.4. | Grader bug | Corrected. Each of the six frozen sites needs a proven row on its lines that names the option. The declaration is covered by the anchor's own line. |
| G5 | The frozen source truth (B.1, B.2) lists only `.`-calls in `colmap/src/colmap/*`. Two real sites fall outside it: `exe/mvs.cc:198` (`->Print()`, another receiver) and `pycolmap/pipeline/mvs.cc:39` (factory construction). | `git grep` at `d62f06a`. | Requirement: the frozen truth is incomplete. | Not changed. Proposal A-1 (§4). |
| G6 | D2-H2 changed between the archived run (FAIL, `expected-callee-missing`) and HEAD (PASS after commit `c869fdf`). | Archived `grading.json`. HEAD outputs under the unchanged predicate. | Historical change | Not changed. The archived FAIL stands as a historical record. HEAD is PASS under the unchanged predicate. The memory note that called this an open decision is corrected. |
| G7 | The clangd pilot attributes the absence of `exe/mvs.cc:271` to the shape of the LSP answer. It does not record the CUDA guard (`exe/mvs.cc:260-278`) or that the pilot's reconstruction did not define the CUDA macros. | Pilot §6.3 and §9, note 5. | Record | Not changed, because the pilot is a historical record. Proposal A-2 (§4). |

## 2. Corrections made

Files: `docs/pilots/cp2/method/grade.cjs` (the four predicates and the source
grounding) and `docs/pilots/cp2/method/grade.test.cjs` (twelve new tests). Nothing
else in the harness changed. The CLI is unchanged, so `run-all.ps1` is unchanged.

**Source grounding.** A line-based check maps a span to its line in the snapshot
named by the search output's `root`. `CP2_SOURCE_ROOT` overrides that root. An
archived output carries a redacted root (for example `<M9_BENCH_AFTER>`), so a
re-grade of an archive must name the snapshot. Without a source, a line-based
check is UNAVAILABLE with reason `source-unavailable`. It never compares a byte
offset with a line number.

**Proven relation.** `isProven` applies the rule that the impact traversal uses
before it follows an edge (`crates/cli/src/edges.rs`, `traverse_impact`): a CALLS
or REFERENCES edge with Exact precision and no unresolved reason.

**Identity.** `sameDefinition` compares a SCIP symbol, or a syntactic definition
span in the same file.

**Predicates and the frozen text they implement:**

- **D1-C6** (D.1 #6). PASS needs a proven edge at `patch_match.cc:667` to the
  anchored definition, with `Print` on that line. FAIL needs a proven edge at
  `:668` or `fusion.cc:144` to the same definition. Otherwise UNMET, with reason
  `identity-not-proven-at-667` or `site-667-not-returned`.
- **D2-C6** (D.2 #5, B.2). Each of the three sites (`exe/mvs.cc:271`,
  `automatic_reconstruction.cc:431`, `dense_reconstruction_widget.cc:397-398`)
  needs a proven edge to the class anchor, on its lines, with `PatchMatchController`
  on that line. Reasons: `construction-sites-not-attributed`, or
  `candidate-only-attribution`.
- **D3-C5** (D.3 #5, B.2, B.3). The first hop is `ProcessProblem`, a candidate
  dispatch from the lines of `AddTask` (268, 292, 309). This is the only candidate
  the frozen text allows, because B.2 and B.3 say it can never be CALLS. The second
  hop is a proven `Run` call from line 671 whose callee starts at line 191 of
  `patch_match.cc`. The third hop is a proven `Run` call from line 197 whose callee
  starts at line 1326 of `patch_match_cuda.cu` or line 54 of `patch_match_cuda.h`.
  Reason `path-not-frozen-chain`. A path shorter than three edges keeps the
  reason `trace-does-not-extend-beyond-the-first-hop`.
- **D4-C5** (D.4 #5, B.4). The declaration is the anchor, covered by its line 56
  and the option token. Print (`patch_match_options.cc:58`), validation
  (`:91-93`), read (`patch_match.cc:361-366`), filter (`:443-447`) and CLI binding
  (`option_manager.cc:1018-1019`) each need a proven row on those lines with the
  token. An empty path set keeps the reason `affected-set-empty`. An incomplete
  set gets `affected-set-incomplete`.

**Effect on verdicts and reasons.**

| Run | D1-C6 | D2-C6 | D3-C5 | D4-C5 |
|---|---|---|---|---|
| HEAD `e911052`, 24 queries (scratch index) | UNMET, reason changed (`reference-set-empty-or-unresolved` to `identity-not-proven-at-667`) | UNMET, same reason | UNMET, same reason | UNMET, same reason |
| Archived `raw-after` with `CP2_SOURCE_ROOT` | UNMET, reason changed | UNMET, same | UNMET, same | UNMET, same |

No verdict changed. The reasons changed only for D1-C6.

## 3. Verification

- `node --test grade.test.cjs` in `docs/pilots/cp2/method`: 45 of 45 pass. The
  33 existing tests are unchanged. The 12 new tests are in the section "source-
  grounded capability predicates".
- Red check: the 12 new tests were run against the grader from `HEAD` before
  this change. Eight fail, and four pass because their verdict did not change
  (the D2 presence case, and the D3 and D4 positive cases). The eight failures
  include the three false PASSes (G2, G3, G4), which the old predicates produce.
  The D1 failures show the unit bug (G1). The old grader returned UNMET with the
  wrong reason, not a false PASS.
- Full HEAD outputs: the old and new graders agree on every check except
  the D1-C6 reason.
- The archived outputs, re-graded with `CP2_SOURCE_ROOT`, agree with the archived
  verdicts for D1 to D4. The D5 rows differ only because the MCP smoke and the
  OpenCode session files are absent from this re-grade. That input is missing,
  not changed.

**Limits.**

- No resolved edge (CALLS or REFERENCES) exists in any archived or HEAD output.
  The proven branches are exercised only by synthetic fixtures. Their endpoint
  fields follow the caller and callee schema, but no real semantic output has
  confirmed that shape.
- D2 uses class-level identity. A constructor endpoint would not match the class
  anchor, so a future semantic output could produce a false negative. Recheck it
  against real output.
- These tests check the grader only. They are not evidence that any gate passes
  on MMSP.

## 4. Requirement questions for the owner (decided 2026-10-25, see §6)

Each question changes a capability requirement or its reading. §6 records the
decision on each one and what the grader applies. §7 records the verification
of A-1 and A-2.

- **Q1 (D4-C5 candidate listing).** B.4 accepts "affected files listed" for the
  honesty side. The corrected predicate requires proven rows instead, because the
  owner rule forbids a PASS from syntax candidates alone. Decide whether
  source-grounded candidate rows may satisfy D4-C5.
- **Q2 (D4-C5 filter site).** The filter (`patch_match.cc:443-447`) reads a local
  copy, `max_triangulation_angle_rad`. No reference-based proof can reach it. With
  Q1 unresolved, D4-C5 cannot pass for that reason. Decide whether the criterion
  stays as written or is re-scoped.
- **Q3 (D1-C6 candidate listing).** B.1 requires the three `Print` sites to appear
  as candidates. That honesty requirement is no longer graded after the correction,
  because the 144 presence test left the capability check. Decide whether to add a
  dated honesty check.
- **Q4 (D3-C5 hops 2 and 3).** The predicate requires the callee of each direct call
  to be the cited definition. This reads B.3 ("every hop that is present resolves
  to the cited construct") as requiring a resolved callee, not only a located
  citation. Confirm the reading.
- **Q5 (D2-C6 class identity).** A constructor endpoint is not the class endpoint.
  Decide whether the class anchor or the constructor counts as the class.

**Proposals (verified in §7; the frozen specification file is unchanged).**

- **A-1 (source truth, G5).** Record in B.1 and B.2 by a dated amendment that
  `exe/mvs.cc:198` (`->Print()`) and `pycolmap/pipeline/mvs.cc:39` are real sites
  outside the frozen pathspecs. Do not edit the frozen text.
- **A-2 (clangd pilot, G7).** Record that `exe/mvs.cc:271` sits in the CUDA-only
  branch, and that the pilot's reconstruction did not define the CUDA macros. A
  new check with the macros defined is needed before the semantic result for that
  site is read.

## 5. Not changed

- `docs/m9-cp1-pilot-spec.md` (the frozen specification), including sections B
  and D.
- The archived `grading.json` files and the raw outputs under `docs/pilots/`.
- `run-all.ps1`, `queries.ps1` and the other harness scripts. The grader keeps its
  CLI.
- The MMSP repository. Its snapshot `d62f06a` and its local clone are untouched.
- No clangd, compilation database, indexer or graph feature was added.

## 6. Decisions on Q1 to Q5 (2026-10-25)

Owner constraints for this round: Q2 does not mean building a dataflow engine,
and Q4 does not mean making the dispatch hop CALLS. The frozen specification is
not changed before its sources are verified (§7).

| Q | Decision | Where it is applied | Reason |
|---|---|---|---|
| Q1 | A candidate row, syntactic or name-level, does not satisfy D4-C5. Only a proven row on the frozen site does. | `grade.cjs`, D4-C5 (in `d0b3f00`) | The owner rule forbids a PASS from syntax candidates alone. Candidate rows stay visible as occurrences, but they are not capability evidence. |
| Q2 | D4-C5 stays UNMET. The relation from the field read to the filter is not provable with the existing capabilities, and no dataflow was built. | Verified (§7.3) | The member occurrences in `patch_match.cc` are at lines 362 and 365. The filter at 443–447 uses the bool `has_max_triangulation_angle` and the local `max_triangulation_angle_rad`. No existing relation links those to the member. |
| Q3 | No new honesty check for the B.1 name-level listing. | Not applied | B.1 reads both ways. Its "Required" sentence asks for all three `Print` sites as candidates. Its "Expected limitation" accepts an answer that names only `:667`, provided it does not claim the others are absent. The owner decides by amendment (A-3). The existing checks still cover the candidate labels (D1-H2) and wrong citations (the D1-C6 FAIL). |
| Q4 | The dispatch hop stays `POTENTIAL_CALL`. Hops 2 and 3 must be proven direct calls whose callee is the cited definition. A `CALLS` edge on the dispatch hop does not satisfy the chain. | `grade.cjs`, D3-C5 (in `d0b3f00`) | B.2 and B.3 say the dispatch is at most a candidate and that a `CALLS` edge there is a FAIL. The existing D2-H3 check catches a proven `ProcessProblem` edge in the callees of `CtrlRun`. |
| Q5 | The class's own constructor counts as the class. `patch_match.h:131` declares it and `patch_match.cc:219` defines it. A proven edge to it attributes the site to the class. | `grade.cjs` (`classOwns`), `grade.test.cjs` (two new tests) | A construction site calls the constructor, so the constructor is the identity a semantic provider would return. Rejecting it would fail a correct answer. A proven edge to another definition in the same file does not count. |

Reason rename in D2-C6: `candidate-only-attribution` becomes
`attribution-not-proven-to-class`. It covers name candidates and proven edges to
other definitions.

Verification: 47 of 47 tests pass. Against the pre-Q5 grader (`d0b3f00`), the
three D2 tests that are new or renamed fail: the constructor acceptance on its
verdict, and the other two on the reason name. Re-grading HEAD and the archived
`raw-after` outputs changes no check. All four gates stay UNMET at HEAD.
D2-H2 is PASS at HEAD.

## 7. Verification of A-1 and A-2 (2026-10-25)

Method: `git grep` over the whole MMSP tree at `d62f06a`, not only
`colmap/src/colmap/*`, and a read of each guard at its exact line. The local MMSP
clone was read only. The frozen specification file was not edited.

### 7.1 A-1, completeness of the source truth

Construction sites of `PatchMatchController` across the whole tree:

- `colmap/src/colmap/controllers/automatic_reconstruction.cc:431` (B.2).
- `colmap/src/colmap/exe/mvs.cc:271` (B.2).
- `colmap/src/colmap/ui/dense_reconstruction_widget.cc:398`, with the
  `make_unique` wrapper on line 397 (B.2 cites 397-398).
- `colmap/src/pycolmap/pipeline/mvs.cc:39`, a `std::make_shared` construction.
  **Not in B.2.** B.2 says three translation units. The repository has four.

Calls to `Print` through `.` or `->`, whole tree: four. The three `.` sites of
B.1 are the complete set of `.` calls: `patch_match.cc:667`, `patch_match.cc:668`
and `fusion.cc:144`. The fourth is `colmap/src/colmap/exe/mvs.cc:198`,
`options.mesh_texture_mapping->Print();`. It is a `->` call on another receiver,
and B.1 does not enumerate `->` calls.

**Result: A-1 is confirmed.** B.2 omits the pycolmap construction. B.1 covers
`.` calls only. Neither point changes a criterion; both change the truth list.
Proposed amendment A-1 adds the pycolmap site to B.2 and the `->Print()` site to
B.1, with the lines above.

### 7.2 A-2, the CUDA condition

| Site | Guard at `d62f06a` | The construction is compiled when |
|---|---|---|
| `exe/mvs.cc:271` | `#if !defined(COLMAP_CUDA_ENABLED) && !defined(COLMAP_HIP_ENABLED)` at 260, `#else` at 263, `#endif` at 278 | CUDA or HIP is defined |
| `automatic_reconstruction.cc:431` | `#if defined(COLMAP_CUDA_ENABLED)` at 429, `#else` at 437, `#endif` at 440 | CUDA is defined; HIP alone is not enough |
| `dense_reconstruction_widget.cc:397-398` | `#if defined(COLMAP_MVS_ENABLED) && (defined(COLMAP_CUDA_ENABLED) || defined(COLMAP_HIP_ENABLED))` at 394-395 | MVS, and CUDA or HIP |
| `pycolmap/pipeline/mvs.cc:39` | `#if defined(COLMAP_CUDA_ENABLED) || defined(COLMAP_HIP_ENABLED)` at 33, `#else` at 44 | CUDA or HIP |

Where the macro comes from: `colmap/cmake/FindDependencies.cmake:347` appends
`COLMAP_CUDA_ENABLED` to the compile definitions inside `if(CUDA_ENABLED AND
CUDA_FOUND)`, opened at line 314. Its `else()` at line 399 sets `CUDA_ENABLED OFF`
(line 400). The option defaults to ON (`colmap/CMakeLists.txt:40`). A build
without a CUDA toolkit defines no CUDA macro.

The pilot's reconstruction, `docs/pilots/clangd/method/colmap-cdb.ps1`, emits no
`-D` defines, and its header says the defines of a real build are not reproduced.
All four construction sites were therefore inactive in the pilot. The pilot's
absence for them is inactive code, not evidence about E12 or about the shape of
the LSP answer alone. This corrects pilot §6.3 (G7).

Consequence for the semantic route: a compilation database must define
`COLMAP_CUDA_ENABLED` or `COLMAP_HIP_ENABLED` before a semantic provider can see
any frozen construction site. Parsing those files may also need the CUDA headers.
That is an owner decision, taken together with the database (evaluation §9).

**Result: A-2 is confirmed.** Proposed amendment A-2 records each guard in B.2 and
in the pilot record, and states that the frozen sites are conditional on CUDA or
HIP. No criterion changes.

### 7.3 Q2 check

`search --occurrences` for the member lists `patch_match.cc` at lines 362 and 365
only. The filter at 443–447 holds no member name: the bool
`has_max_triangulation_angle` (444) and the local `max_triangulation_angle_rad`
(445). No existing capability links that local to the member.

### 7.4 A-3, opened by Q3

B.1 asks for all three `Print` sites as candidates ("Required") and also accepts
an answer that names only `:667` ("Expected limitation"). The owner states which
reading holds. The grader applies neither.

Decided in §8: the "Expected limitation" reading holds. The grader now checks the
labels, the callee rule and the completeness claim (D1-H6).

## 8. A-3 decision (2026-10-25)

**Decision.** B.1 is read through its "Expected limitation". A name-level
candidate list may be incomplete, provided the limitation is stated honestly.
The frozen B.1 text is unchanged. Its "Required" sentence is read in this light,
and no grader check requires all three `Print` sites to be listed.

| Condition | What is checked | Where |
|---|---|---|
| 1. An answer that names only `patch_match.cc:667` may pass the honesty gate, but must not claim that it is every call named `Print`. | A reference set that holds candidates must declare `completeness.status: partial`. A set that declares `complete` fails. | D1-H6 (new) |
| 2. Candidates keep the labels `POTENTIAL_CALL`, `Heuristic` and `Unverified`. | The kind is checked by D1-H2 (no proven `CALLS`). Precision and verification are checked by D1-H6. | D1-H2, D1-H6 |
| 3. `patch_match.cc:668` and `fusion.cc:144` are not misattributed to `PatchMatch::Problem::Print`. | A proven edge from either site to the anchored definition fails D1-C6. A candidate must name no callee, so it cannot carry an identity. | D1-C6 (corrected in `d0b3f00`), D1-H6 |
| 4. D1-C6 still requires a proven method identity. Candidates alone do not pass the capability. | D1-C6 passes only with a proven edge at `:667` to the anchored definition. | D1-C6 (unchanged since `d0b3f00`) |

**Reading.** Honesty measures whether the answer is honest: its labels, its
completeness claim, and whether it misattributes a site. Capability measures
whether the symbol resolution is complete and accurate: a proven identity at
`:667`, and no proven wrong identity at `:668` or `:144`.

**Why a grader change was needed.** The existing checks did not cover conditions
1 and 2 in full. D1-H2 checks only the kind of each edge. Nothing checked the
precision and verification labels, the callee rule, or the completeness claim.
The owner's rule is to change the grader only where the existing checks do not
suffice, so D1-H6 was added. It is additive: no existing check changed.

**D1-H6.** Honesty layer. For every candidate (`POTENTIAL_CALL`) in the reference
queries: the precision must be `Heuristic`, the verification `Unverified`, and
the callee `null`. A reference set that holds candidates must declare
`completeness.status: partial`. With no candidate, the check is UNAVAILABLE
(`no-candidate-returned`).

**Verification.**

- `node --test grade.test.cjs`: 52 of 52 pass (47 before, plus five A-3 tests).
  The A-3 tests cover a partial answer that names only `:667` (passes D1-H6, D1-C6
  stays UNMET), a mislabelled candidate, a candidate that names a callee, a
  set that does not declare itself partial, and an empty candidate set.
- HEAD outputs (24 queries): D1-H6 PASS. It inspected 34 candidates, with none
  mislabelled, none naming a callee, and none undeclared. No existing check
  changed. D1-C6, D2-C6, D3-C5 and D4-C5 stay UNMET.
- Archived `raw-after` outputs, re-graded with `CP2_SOURCE_ROOT`: D1-H6 PASS, and
  no existing check changed.

**Not changed.** The frozen specification, the archived `grading.json` files and
the raw outputs. The capability predicates, which are unchanged since `d0b3f00`
and `9583bcf`.

**Open.** If the owner wants the three-site candidate listing enforced, that is a
new amendment for B.1, not a grader fix.
