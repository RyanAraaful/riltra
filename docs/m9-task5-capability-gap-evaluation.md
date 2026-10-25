# M9 Task 5 — C++ capability gap evaluation

**Status: EVALUATION ONLY.** This document records why the four M9 capability
gates D1-C6, D2-C6, D3-C5 and D4-C5 stay UNMET at HEAD `e911052`. It changes no
production code, no frozen benchmark, no grader and no acceptance criterion,
and it flips no gate. Every fix below is a recommendation that waits for an
owner decision. No clangd run, no compilation database and no external indexer
was created for MMSP.

## 1. Scope and evidence basis

- Riltra branch `m9-multirepo-release-candidate`, HEAD `e911052`. Debug binary
  built from that HEAD with `cargo build --locked -p riltra`, outside the
  repository.
- MMSP frozen snapshot `d62f06a` (tree `c5482ea`), taken from the bench clone
  pinned for CP2. `git status --porcelain` was empty before and after. The
  local MMSP clone was read only through `git show` and `git grep`.
- Index: `riltra init --repo <mmsp-snapshot> --name mmsp-pin`, then
  `riltra index --repo mmsp-pin --full`, both under a scratch `RILTRA_HOME`.
  Result: 2,277 tracked files discovered, 1,475 processed, snapshot `fresh`,
  `dirty: false`.
- Queries: the CP2 set in `docs/pilots/cp2/method/queries.ps1`, run against
  that index, plus `search --occurrences`. The harness `run-all.ps1` was not
  used, so the grader itself was not executed. Its predicates in
  `docs/pilots/cp2/method/grade.cjs` were applied by hand to the HEAD outputs.
- MMSP line numbers are relative to the MMSP root. Riltra file:line references
  are relative to this repository.
- Context7: `/tree-sitter/tree-sitter-cpp` and `/scip-code/scip`. Queries were
  generic and named no MMSP symbol or path. The lookup does not select the
  pinned grammar version (tree-sitter-cpp 0.23.4, `Cargo.lock`), so the
  tree-sitter examples come from the grammar's current corpus. Both sources
  agree with the Riltra code comments and tests cited below. No contradiction
  was found.

## 2. Matrix

| Gate | Frozen expectation | Actual at HEAD | Proven root cause | Existing components enough? | Minimum recommendation |
|---|---|---|---|---|---|
| D1-C6 method identity | Callers of `PatchMatch::Problem::Print` separate `patch_match.cc:667` (the one true reference) from `:668` and `fusion.cc:144` (B.1, D.1 #6). | Three name-level candidates: `patch_match.cc:667`, `patch_match.cc:668`, and `exe/mvs.cc:198` (`options.mesh_texture_mapping->Print()`, another receiver). `fusion.cc:144` is absent. No identity is separated. | Call facts keep only the terminal name. Candidates match by name inside include-linked files. The syntax layer does not know receiver types. | Partly. Stored semantic evidence exists (SCIP import), but MMSP has no C++ index, and building one needs owner approval. | Owner approves a C++ semantic index for MMSP (compilation database and external indexer, outside the MMSP tree). Import it through the existing SCIP path and re-evaluate. |
| D2-C6 constructor callers | Three construction sites are attributed to `PatchMatchController`, not to `make_shared` or `make_unique` (B.2, D.2 #5). | Two of three: `automatic_reconstruction.cc:431` (candidate) and `dense_reconstruction_widget.cc:397-398` (factory candidate). `exe/mvs.cc:271` is absent. One real site outside the frozen pathspec, `pycolmap/pipeline/mvs.cc:39`, is attributed. | `exe/mvs.cc:271` has five plain-identifier arguments. The grammar reads that shape as a parameter list, and Riltra refuses it by design (gap E12). The site also sits in a CUDA-only `#else` branch. | Partly. A syntax-only fix needs a bounded function-scope binding rule, which is a new small capability. A semantic fix needs a compilation database with CUDA macros. | Owner chooses: (a) a bounded local-binding rule in `direct_init_parts`, candidate only, or (b) the semantic route with a CUDA-enabled database. D2-C6 stays UNMET under the frozen wording either way. |
| D3-C5 multi-hop trace | Four nodes, three edges: `PatchMatchController::Run` → `ProcessProblem` (indirect, via `ThreadPool::AddTask`) → `PatchMatch::Run` → `PatchMatchCuda::Run` (CUDA, unsupported) (B.3, D.3 #5). | 23 paths from `CtrlRun` (`patch_match.cc:233:28`). Each has one edge and ends in `unresolved-target`. Maximum hops: 1. `ProcessProblem` is absent. The CUDA `--to` probe exits 3 (`source-changed`). | Trace advances only over `CALLS` edges with a resolved callee (`edges.rs:1223-1240`). Syntax emits only `POTENTIAL_CALL` with `callee: null`. MMSP has no semantic provider, so there are zero `CALLS` edges. Hop 1 is an address-of argument, not a call fact. | Partly. Traversal and SCIP import exist. Hop 1 needs a new labelled candidate class and a traversal change. CUDA stays unsupported. | (1) Semantic index, as for D1. (2) Owner decides an indirect-dispatch candidate hop that is labelled, never `CALLS`, and follows only a unique target. (3) The CUDA terminus stays labelled unsupported. |
| D4-C5 impact | The affected set includes the six frozen locations (declaration, print, validation, read, filter, CLI binding) with verdict `UNKNOWN` (B.4, D.4 #5). | Verdict `UNKNOWN`, completeness `partial`, reason `graph-completeness-unproven`, `paths: []` (the expected pilot shape). Occurrence rows cover four of six locations (§6). | Impact walks proven edges only (`edges.rs:1427-1449`). Syntax derives no edges from occurrence rows (`edges.rs:283-391`). By-name matching is off for data members (`index.rs:2158`). Macro rows exist only for macros defined in the same file (`syntax.rs:587-600`). The filter site reads a local variable. | Partly. Occurrence rows (S3) and the impact answer exist. Listing them as candidates is reuse. Validation needs include-based macro visibility (E7 territory). The filter needs local dataflow, which is out of scope. | Owner decides whether a candidate-only affected listing counts. If it does, join the existing occurrence rows into the impact answer, labelled as candidates, with the verdict unchanged. The gate stays UNMET until the owner closes or narrows the two missing sites. |

## 3. D1-C6 — C++ method identity

**Expected (frozen).** `riltra callers` at `patch_match.cc:118:27` should show
`patch_match.cc:667` as the one true reference and separate it from
`patch_match.cc:668` (`PatchMatchOptions::Print`) and `fusion.cc:144`
(`StereoFusionOptions::Print`). The honesty gate requires all three sites to
appear as `POTENTIAL_CALL` candidates labelled with the name `Print` (B.1).

**Actual (HEAD).** `callers` returns three candidate edges, all with reason
`dynamic`:

- `colmap/src/colmap/mvs/patch_match.cc` lines 667 and 668 (byte spans 26058
  and 26077).
- `colmap/src/colmap/exe/mvs.cc` line 198, `options.mesh_texture_mapping->Print();`.
  This is a name-only false candidate with a different receiver.
- `fusion.cc:144` does not appear.

Nothing separates identities.

**Root cause (proven).**

1. A call fact stores the full callee text and a terminal span, which is the
   name after the last `.`, `::` or `->` (`syntax.rs:608-622`). The receiver
   text is stored, but nothing binds it to a type.
2. `derive_syntax_edges` makes every such call a candidate with `callee: null`.
   A receiver call gets the reason `dynamic` (`edges.rs:157-161`, `:360-361`).
3. The candidate scope for `callers` is the anchor file, the declaring header,
   and every file that includes either (`index.rs:1665-1696`). `fusion.cc`
   includes `fusion.h`, not `patch_match.h`, so `fusion.cc:144` never enters
   the set. That scope is correct for identity, and it also means the
   name-level set that B.1 requires is incomplete.
4. Mapping `problem`, `patch_match_options` and `options_` to declared types
   needs name and type resolution. That would be a new resolver, which the
   Task 5 constraints prohibit.

**Can existing components fix it?** Only with semantic evidence. The SCIP
grammar encodes scope in the symbol string (`<type>#` and `<method>().`
descriptors), so `PatchMatch#Problem#Print().` and `PatchMatchOptions#Print().`
differ when an indexer emits them (Context7 `/scip-code/scip`, `docs/scip.md`).
Riltra stores SCIP import rows as edges. The `lsp-subprocess` provider
yields zero stored edges (`edges.rs:396-400`), so it cannot feed `callers` at
HEAD. MMSP has no compilation database for its colmap tree (design gate E1).

**Minimum recommendation.** Owner approves (a) a compilation database for the
colmap tree at `d62f06a`, generated outside the MMSP tree, and (b) one external
SCIP indexer run. Import the result through the existing SCIP path and re-run
the D1 queries. Do not mark D1-C6 PASS from the import until the owner rules on
finding G1 (§7).

**Risks and dependencies.** The database depends on build flags (CUDA, optional
dependencies). colmap's CMake uses `FetchContent`, which downloads at configure
time. The SCIP import must keep `Exact` only when the import holds a definition
(commit `e911052`). Freshness must bind the import to the snapshot.

## 4. D2-C6 — constructor callers of `PatchMatchController`

**Expected (frozen).** Three construction sites: `exe/mvs.cc:271`,
`controllers/automatic_reconstruction.cc:431` and
`ui/dense_reconstruction_widget.cc:397-398`. All three must be attributed to the
class, not to the factory template (B.2, D.2 #5).

**Actual (HEAD).** Class callers (`p2`) return three candidates:

- `controllers/automatic_reconstruction.cc:431`, reason `dynamic`.
- `ui/dense_reconstruction_widget.cc:397-398`, reason `factory-template-argument`
  (commit `c869fdf`).
- `pycolmap/pipeline/mvs.cc:39`, reason `factory-template-argument`. This site
  lies outside the frozen pathspec (G5).

`exe/mvs.cc:271` is absent. Two of the three frozen sites are attributed,
which matches the task brief.

**Root cause for `exe/mvs.cc:271` (proven).**

1. *Source.* The construction is at lines 271-277, in the `#else` branch of
   `#if !defined(COLMAP_CUDA_ENABLED) && !defined(COLMAP_HIP_ENABLED)`
   (lines 260-263, `#endif` at 278). Its arguments are `options`,
   `workspace_path`, `workspace_format_lower`, `pmvs_option_name` and
   `config_path`. They are all plain identifiers. Four are parameters of
   `RunPatchMatchStereoImpl` (lines 255-259), and `workspace_format_lower` is a
   local (line 264). C++ resolves them as variables, so the statement is a
   variable definition with a constructor call.
2. *Grammar and refusal.* Context7 (`/tree-sitter/tree-sitter-cpp`, corpus
   `ambiguities.txt`) shows that `T1 a(T2 *b);` parses as a `function_declarator`
   and `T7 f(g.h);` parses as an `init_declarator` with an `argument_list`. Plain
   identifiers are the ambiguous case. `direct_init_parts` (`syntax.rs:651-672`)
   accepts only the `argument_list` form. The unit test
   `cpp_most_vexing_parse_is_ambiguous_and_not_a_call` (`syntax.rs:1210-1222`)
   pins `Widget w(options, problem);` as not a call. The refusal is therefore
   deliberate, and the code comment defers this case to name lookup (gap E12).
   The test result is in §8.
3. *Contrast.* `automatic_reconstruction.cc:431-432` passes
   `*option_manager_.patch_match_stereo, dense_path, "COLMAP", ""`. The
   non-identifier arguments select `argument_list`, so that site is attributed.
   The mechanism is the argument shape, not the file.

**Ruled out.**

- *Include scope.* `exe/mvs.cc:38` includes `colmap/mvs/patch_match.h` directly,
  so the file is in the candidate scope.
- *Inactive preprocessor branch alone.* `automatic_reconstruction.cc:429-431`
  sits inside `#if defined(COLMAP_CUDA_ENABLED)` and is still attributed. The
  syntax layer parses inactive branches, so the CUDA guard does not explain the
  syntax miss.

**Semantic caveat.** The guard does matter for a semantic provider. The clangd
pilot reconstruction did not define `COLMAP_CUDA_ENABLED` or
`COLMAP_HIP_ENABLED` (pilot §9, note 5), so a provider over that database cannot
see `exe/mvs.cc:271`. The pilot attributes the absence to the LSP answer shape
(pilot §6.3) and does not record this guard. Both causes need separate tests
before a semantic result for this site is read.

**Can existing components fix it?** For the syntax route, `direct_init_parts` and
the body walk exist. What is missing is the enclosing function's binding table
(parameters and earlier locals). A rule could record a constructor-call
candidate when every argument is an identifier bound as a parameter or local
before the statement. Anything else keeps the refusal. The nearest binding
decides, and an unknown name refuses. This is narrow name lookup, which the
code defers on purpose, so it is an owner decision.

**Minimum recommendation.** Owner chooses one: (a) the bounded binding rule,
candidate only, with tests for shadowing and for an identifier that names a
type; or (b) the semantic route with a CUDA-enabled database (§3). Under either
choice D2-C6 stays UNMET, because the frozen wording requires identity
resolution (B.9, D.2 #5). The grader's predicate accepts file presence only, so
it would pass on candidate attribution alone (G2).

## 5. D3-C5 — multi-hop trace and indirect dispatch

**Expected (frozen).** `trace` from `CtrlRun` at depth 4 should span the chain
`Run` → `ProcessProblem` → `PatchMatch::Run` → `PatchMatchCuda::Run`. Hop 1 is
the indirect `ThreadPool::AddTask` dispatch. Hop 3 ends in CUDA, which is
unsupported (B.3, D.3 #5).

**Actual (HEAD).** `trace --depth 4` returns 23 paths. Each path has one edge and
ends in `unresolved-target`. The maximum hop count is 1, and `ProcessProblem`
never appears. The CUDA `--to` probe exits 3 with
`anchor unresolved: source-changed; reindex before syntax fallback`, matching
the frozen expectation.

**Root cause, per hop.**

- *Trace rule.* The trace advances only over an edge whose kind is `CALLS` and
  whose callee is resolved (`edges.rs:1223-1240`). Every other edge ends the
  path with `unresolved-target` (`edges.rs:1225-1239`). Syntax emits only
  `POTENTIAL_CALL` edges with `callee: null` (`edges.rs:375-388`). MMSP has no
  semantic provider, so the number of `CALLS` edges is zero and every path has
  length 1.
- *Hop 1* (`thread_pool_->AddTask(&PatchMatchController::ProcessProblem, …)`,
  `patch_match.cc:268, 292, 309`). The member function is passed by address.
  Only `call_expression` and direct-init constructs become call facts
  (`syntax.rs:608-622`, `:678-700`). The reference is recorded as an
  address-of occurrence (`syntax.rs:535-540`), and nothing traverses it.
- *Hop 2* (`patch_match.Run()`, `patch_match.cc:671`). This is a receiver call
  with the terminal name `Run`. Its candidate reason is `dynamic`, so the target
  is not resolved to `PatchMatch::Run`. Resolving it needs the receiver's type.
- *Hop 3* (`patch_match_cuda_->Run()`, `patch_match.cc:197`). The same shape. The
  definition is at `patch_match_cuda.cu:1326`, which Riltra does not parse.

**Boundaries, kept separate.**

- *Syntax.* Call facts exist for hops 2 and 3, but the name is ambiguous. Hop 1
  exists only as an address-of occurrence, and the trace does not follow it.
- *Semantic.* A C++ provider (§3) could supply `CALLS` for hop 2 and a
  declaration target for hop 3 (`patch_match_cuda.h:54`). It cannot supply a
  `CALLS` edge for hop 1, because B.2 and B.3 treat that as a FAIL. Even a
  `REFERENCES` edge for hop 1 would not be followed by the current trace.
  Hop 1 therefore needs a new labelled candidate class.
- *CUDA.* `patch_match_cuda.cu` is not indexed, so hop 3 to hop 4 can only end
  as a labelled unsupported terminus. The `--to` probe already shows that
  behaviour.

**Can existing components fix it?** Partly. Traversal, the `TRACE_MAX_*` budgets,
path output and SCIP import exist. Hop 1 needs three things: a candidate built
from the existing address-of occurrence, a traversal rule that follows only
labelled candidates with a unique target by qualified name, and a kind that is
never `CALLS`. Together these change the trace contract, so they are an owner
decision.

**Minimum recommendation.** (1) The semantic index from §3. (2) Owner decides
the indirect-dispatch candidate contract. (3) The CUDA terminus stays labelled
unsupported.

**Risks.** `Run` has four identities in `mvs/` (B.6 row 1). Following
candidates without a unique-target rule would multiply fabricated paths. The
budgets must still bound the search. The grader counts edges, not whether they
form the frozen chain (G3).

## 6. D4-C5 — impact of `max_triangulation_angle`

**Expected (frozen).** The affected set includes the declaration
(`patch_match_options.h:56`), the print (`patch_match_options.cc:58`), the
validation (`patch_match_options.cc:91-93`), the read (`patch_match.cc:361-366`),
the filter (`patch_match.cc:443-447`) and the CLI binding
(`option_manager.cc:1018-1019`). The verdict must be `UNKNOWN`. D.4 also states
that the expected pilot result is an empty path set.

**Actual (HEAD).**

- `search --symbol max_triangulation_angle` returns one field,
  `patch_match_options.h:56:10`.
- `impact` on that member returns verdict `UNKNOWN`, completeness `partial`, the
  reason `graph-completeness-unproven`, the disclaimer
  `plausible-impact-not-execution`, and `paths: []`. The anchor note says the
  member has no call edges and points to `search --occurrences`.
- `search --occurrences` (scope `repository-wide-exact-name`, binding `unbound`,
  four items):

| Location | Role | Confidence | Frozen site |
|---|---|---|---|
| `controllers/option_manager.cc:1019:41` | address-of-binding | observed | CLI binding, found |
| `mvs/patch_match.cc:362:16` | member-access | observed | read, found |
| `mvs/patch_match.cc:365:31` | member-access | observed | read, found |
| `mvs/patch_match_options.cc:58:15` | macro-argument | candidate | print, found |

Not found: the validation at `patch_match_options.cc:92-93`. Its macro
`CHECK_OPTION_LE` is defined in `colmap/util/logging.h:61`, outside this file. The
filter at `patch_match.cc:444-445` compares the local `max_triangulation_angle_rad`,
so no member-name method reaches it.

**Found references versus unproven dependencies.**

- *Found.* The four occurrence rows above, plus the declaration returned by
  `search`. These are syntax-level reference facts.
- *Not proven.* Any dependency edge from those sites to the member's impact set.
  Impact returns proven edges only (`edges.rs:1427-1449`). Syntax derives no edge
  from an occurrence row (`edges.rs:283-391`). By-name matching is off for data
  members (`index.rs:2158`; the comment says a data member takes no call by name).

**Root cause of the empty affected set (proven).**

1. Occurrence rows are not edges, and impact consumes edges only.
2. Data-member callers are never matched by name (`index.rs:2158`), so the rows
   never become candidate edges.
3. Macro arguments are recorded only for macros defined in the same file
   (`syntax.rs:559-582`, `:587-600`). `CHECK_OPTION_LE` is defined in a header,
   so the validation sites are invisible.
4. The filter site uses a local variable. No member-name method can reach it.
   E7 slice 2 (`docs/m9-e7-slice2-evidence-and-decision.md` §3.4) reaches the same
   conclusion.

**Can existing components fix it?** Partly. The occurrence query (S3) and the
impact answer exist. Listing the occurrence rows as labelled candidates reuses
them and adds no parser. Validation needs include-based macro visibility, which
is E7 design territory and is not repeated here. The filter needs local dataflow,
which is out of scope.

**Risks.** `binding: unbound` means the rows match repository-wide by exact name,
so a same-named field in another struct would appear as a candidate. Each row
must carry `unproven-relation`. The grader's predicate `paths.length > 0` would
pass on a single unrelated row (G4).

**Minimum recommendation.** Owner decides whether a candidate-only affected
listing counts toward D4-C5. If it does, join the occurrence rows into the impact
answer as labelled candidates, and keep the verdict `UNKNOWN`. D4-C5 stays UNMET
until the owner closes or narrows the validation and filter sites.

## 7. Grader and criteria findings (not changed; owner decision)

- **G1. D1-C6 predicate cannot match by construction.** The predicate
  (`grade.cjs:205-213`) reads `caller_span[0]`. In the JSON, `caller_span` is a
  byte offset (`7210`, `26058`, `26077`), but the predicate compares it with line
  numbers (`667`, `668`, `144`). Even with line numbers, it requires
  `fusion.cc:144` among the callers of `PatchMatch::Problem::Print`, which an
  identity-resolved answer would exclude. The predicate therefore rewards a
  name-level candidate set and cannot test identity.
- **G2. D2-C6 predicate checks presence only** (`grade.cjs:297-309`). The frozen
  wording requires identity resolution (B.9, D.2 #5). Candidate attribution
  satisfies the predicate but not the wording.
- **G3. D3-C5 predicate counts edges** (`grade.cjs:327-330`). It passes on any
  path with three edges and does not check the frozen chain.
- **G4. D4-C5 predicate counts paths** (`grade.cjs:385-387`). It passes on any
  non-empty affected set and does not check the six frozen sites.
- **G5. Frozen source truth is incomplete.** B.1 and B.2 cover only `.`-calls in
  `colmap/src/colmap/*`. Real sites outside that scope are `exe/mvs.cc:198`
  (`->Print()`) and `pycolmap/pipeline/mvs.cc:39` (factory construction). The
  "complete set of three" claim holds only for `.` calls. Record this in a dated
  amendment. Do not edit the frozen text.
- **G6. D2-H2 changed since the archived run.** The archived m9-rc run records
  FAIL (`expected-callee-missing`). Commit `c869fdf` adds the factory-argument
  candidate. On the HEAD outputs, both `PatchMatchCuda` (factory candidate) and
  `Run` (`dynamic`) appear as callees of `PatchMatch::Run`, so the predicate
  gives PASS. This evaluation does not change that result. The project memory
  entry that lists the factory candidate as an open owner decision is stale.
- **G7. Clangd pilot attribution is incomplete.** Pilot §6.3 attributes the
  `exe/mvs.cc:271` absence to the LSP answer shape. It does not record the CUDA
  guard described in §4. The attribution needs a new check with the macros
  defined.

## 8. Verification

- Targeted unit tests at HEAD: `cargo test --locked --bin riltra syntax::tests::cpp_`.
  Result: 18 passed, 0 failed. The set includes `cpp_most_vexing_parse_is_ambiguous_and_not_a_call`,
  `cpp_direct_initialization_is_a_call` and `cpp_template_type_direct_initialization_is_a_call`.
- MMSP snapshot: `git status --porcelain` empty before and after, HEAD `d62f06a`.
  The local MMSP clone was read only and is unchanged.
- No production code changed. No clangd, compilation database or indexer was run.
- The grader was not executed. Its predicates were applied by hand (§7).

## 9. Recommended order of resolution (owner decisions, no implementation yet)

1. **Settle G1 to G5 first.** Use a dated amendment or an owner ruling to align
   the predicates and the frozen source truth with the criteria. This prevents a
   false PASS from any later candidate fix.
2. **Approve a C++ semantic index for MMSP colmap.** Use a compilation database
   outside the tree and one external SCIP indexer, in a scratch location, and
   decide the CUDA macro setting. This unlocks D1-C6, the hop 2 and hop 3 part of
   D3-C5, and the semantic case for `exe/mvs.cc:271`.
3. **D2-C6 `exe/mvs.cc:271`.** Choose the bounded binding rule (syntax, candidate
   only) or rely on step 2. The rule is independent of step 2.
4. **D4-C5.** Decide whether a candidate-only affected listing counts. If it does,
   implement the occurrence join. This is independent of step 2.
5. **D3-C5 hop 1.** Decide the indirect-dispatch candidate contract after step 2.

Dependencies: step 2 is needed by D1-C6 and by the D3-C5 hops 2 and 3. Steps 3
and 4 do not depend on step 2. Step 5 depends on step 2. The E7 decisions for
the D4-C5 validation and filter sites remain the owner's.

## 10. Reproduction

Set `RILTRA_HOME` to a scratch directory first, as the repository rules require.
Replace the placeholders with local paths. Do not commit local paths.

```bash
riltra init --repo <mmsp-snapshot> --name mmsp-pin
riltra index --repo mmsp-pin --full
riltra callers --repo mmsp-pin --path colmap/src/colmap/mvs/patch_match.cc --line 118 --col 27
riltra callees --repo mmsp-pin --path colmap/src/colmap/mvs/patch_match.cc --line 191 --col 18
riltra trace --repo mmsp-pin --path colmap/src/colmap/mvs/patch_match.cc --line 233 --col 28 --depth 4
riltra impact --repo mmsp-pin --path colmap/src/colmap/mvs/patch_match_options.h --line 56 --col 10
riltra search --repo mmsp-pin --symbol max_triangulation_angle --occurrences
```

## 11. Limits

- One host, debug build. Timings are not part of this evaluation.
- The Context7 tree-sitter examples come from the grammar's current corpus. They
  are not pinned to 0.23.4. The pinned behaviour is covered by Riltra's own test
  (§8).
- The SCIP disambiguator format depends on the indexer. No indexer was verified.
- The clangd conclusions are taken from the pilot record. No new clangd run was
  made.
