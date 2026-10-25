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

## 4. Requirement questions for the owner (not applied)

Each question changes a capability requirement, so none is applied. A decision
needs a dated amendment.

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

**Proposals to record, not to apply.**

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
