# M9 E7 Slice 2 — occurrence evidence and design decision

**Scope: measurement and design refinement only.** No production code, no schema,
no graph change, no CLI/MCP contract change. The runtime is untouched by this
task: the only artefacts are this document and the evidence under
`docs/pilots/e7-slice2-evidence/`.

- Branch: `m9-multirepo-release-candidate`
- Starting HEAD (= remote): `4ea339853681f70be207794a031355e647167b8f`
  (the E7 Slice 1 implementation; verified local == remote, ancestry confirmed,
  working tree clean apart from the untracked owner-local `AGENTS.md`)
- Pilot pin (unchanged): MMSP `d62f06a576d8e76b705c8ecc57fc380f7ebc9c44`,
  tree `c5482ea00481675e0aa921538f560947b68dca12`
- Design of record: `docs/m9-e7-reference-resolution-design.md` (§4.2, §4.3, §7)
- Slice 1 record: `docs/m9-e7-slice1-implementation.md`
- Acceptance source: `docs/m9-cp1-pilot-spec.md` §B.4 and §D.4

**Summary of what the measurement found.** The identifier
`max_triangulation_angle` occurs **9 times in C++ code** in the pinned tree
(4 files): 1 declaration, 1 string literal, and **7 code uses**. Under the
design's **stated** rule (a macro argument is one whose macro is defined in
the same **translation unit**) the design captures **6 of the 7 uses**; only
the *plain identifier* — an implicit member access — is missed. Under the
design's **implementation sketch** (a *per-file* macro-name pre-pass) it
captures **4 of the 7**: the two `CHECK_OPTION_LE` arguments are invisible to
a per-file scan, because that macro is defined in an *included* header. The
two readings differ because the design's rule and its implementation sketch
disagree — that disagreement is itself a finding (§3.1) and an OWNER/design
decision (§8.5). The include-scoped variant, as specified, would lose
**2 more** (the behavioural read in `patch_match.cc`, which reaches the
declaration's header only *transitively*). One frozen CP1 site — the
source-image filter at `patch_match.cc:443-447` — is **not reachable by name
at all**: it reads the local `max_triangulation_angle_rad`.

---

## 1. Baseline and measurement method

### 1.1 Isolation

| Requirement | How it was met |
|---|---|
| disposable snapshot, outside both repositories | `%TEMP%\opencode\m9-e7-slice1\mmsp-pin`, reused from the Slice 1 task after it was re-verified: `HEAD = d62f06a…`, `tree = c5482ea…`, `git status --porcelain` empty |
| cloned from the public remote | `git init` + `git remote add origin https://github.com/RyanAraaful/mmsp.git` + `git fetch --depth 1 origin d62f06a…` + `git checkout FETCH_HEAD` (the Slice 1 task) |
| the original MMSP clone | never read, never written, never cloned from |
| no destructive git, no `Assert-SnapshotPath` | only `git init/fetch/checkout/show/rev-parse/status`; no checkout/reset/clean of any working tree; the one file touched during the Slice 1 incremental experiment was restored byte-exactly and `git status` is empty again |
| no `$home` | the isolated home is `%TEMP%\opencode\m9-e7-slice1\home`; the variable is named explicitly and the actual path was printed before any Riltra run. **No Riltra run happened in this task at all** (the Slice 1 task had a `$home`/`$HOME` collision and corrected it — `docs/m9-e7-slice1-implementation.md` §10) |
| no build, no CMake, no CUDA, no photogrammetry | source and AST inspection only; no compiler was invoked on MMSP |
| no dependency added to the repository | the AST probe is a **separate cargo project** that was run outside the repository; it declares its own `[workspace]`, pins the same grammar versions the product pins (`=0.25.10`, `=0.23.4`), and its `Cargo.lock` is archived with it. Nothing was added to the repository's `Cargo.toml` |

### 1.2 The probe

The probe (`docs/pilots/e7-slice2-evidence/probe/`) parses one C++ file with
the product's own grammar and reports, for every occurrence of a given
identifier: the line, the column, the **AST node kind**, the **syntactic
role**, the enclosing function and class, and the local macro names of the
translation unit. It is read-only and prints tab-separated records. It was
run **outside** the repository and is archived here with its own
`[workspace]`, its pinned versions and its `Cargo.lock`, so it cannot be
built by the repository's CI and cannot change the product's dependency
graph. The grammar is the one the product uses, so the node shapes are the
shapes the product sees (`crates/cli/src/syntax.rs:46` pins `cpp:0.23.4`).

The role rules the probe applies are a **subset** of the design's rules,
checked against the AST rather than against the text:

| Rule | Test the probe applies | Difference from the design |
|---|---|---|
| declaration | the identifier is the `declarator` of a `field_declaration` | — |
| member access | the identifier is the `field` of a `field_expression` (operator `.`, `->`, `.*`) | — |
| address-of binding | that `field_expression` is the operand of a `pointer_expression` with operator `&` | the probe accepts any `&` operand, the design requires a `field_expression` operand; **no measured occurrence is affected** |
| macro argument | the identifier is an argument of a `call_expression` whose callee is a `preproc_function_def` **of the same file** | the design's rule says "of the same **translation unit**" — the probe implements the per-file reading only (§3.1) |
| ordinary call argument | as above, but the callee is not a local macro | this is where a macro from an included header lands |
| plain identifier | any other use (the parent node is a comparison, an assignment, …) | — |
| string literal | the name occurs inside a `string_literal` node | — |
| static member access | **not implemented by the probe** | the design has the role; no measured occurrence is a `Type::member` access, so nothing is lost here |

Two properties of the probe's output are **heuristics, not AST facts**, and
are labelled as such wherever they are used: the *enclosing class* column is
derived lexically from the qualified function name (the probe treats a `mvs`
scope as a namespace), and the *enclosing function* column is the
declarator chain's text. The **role** and the **node kind** are exact.

### 1.3 Universe

The identifier occurs on **45 lines** in the whole tree. 36 of them are in
file types Riltra does not index (MMSP documentation, `.ini` configuration,
PowerShell scripts, one `.patch` file) — no content from those files is
reproduced here. The **C/C++ occurrence universe is the 9 measured
occurrences**; none of them is in a `.cu`/`.cuh` (unsupported) or a `.inl`
(not indexed).

The archive: `docs/pilots/e7-slice2-evidence/occurrence-inventory.txt` (the
probe output), `include-scope.txt` (the include measurement),
`supporting-anchors.txt` (the search counts, the macro-definition anchors, the
include anchors and the direct-includer list) and `probe/` (the probe source,
its pinned manifest and its lock file).

## 2. Occurrence inventory

Every occurrence, with its syntactic evidence. The `enclosing` column is the
enclosing function's declarator text — for a member function, its qualified
name — from which the class is **derived lexically** (`PatchMatchOptions::Check`
→ `PatchMatchOptions`). The function name is exact; the class is a derivation,
not an AST class node (§1.2).

| # | path:line:col | AST node | role | detail | enclosing |
|---|---|---|---|---|---|
| 1 | `colmap/src/colmap/mvs/patch_match_options.h:56:10` | `field_identifier` | **declaration** | the member of `struct PatchMatchOptions` | — (inside the class) |
| 2 | `colmap/src/colmap/mvs/patch_match_options.cc:58:15` | `identifier` | **macro argument** | callee `PrintOption`, a macro **defined in this file** (`:43`) | `PatchMatchOptions::Print` |
| 3 | `colmap/src/colmap/mvs/patch_match_options.cc:91:7` | `identifier` | **plain identifier** | inside a `binary_expression` (`>= 0.0f`) — an *implicit* member access inside a member function | `PatchMatchOptions::Check` |
| 4 | `colmap/src/colmap/mvs/patch_match_options.cc:92:21` | `identifier` | **ordinary call argument** | callee `CHECK_OPTION_LE` — a macro, but **not defined in this file** (it is in `colmap/src/colmap/util/logging.h:61`, included at `:32`) | `PatchMatchOptions::Check` |
| 5 | `colmap/src/colmap/mvs/patch_match_options.cc:93:46` | `identifier` | **ordinary call argument** | as #4 (the second argument) | `PatchMatchOptions::Check` |
| 6 | `colmap/src/colmap/mvs/patch_match.cc:362:16` | `field_identifier` | **member access** | operator `.` on `options_` | `PatchMatchController::ReadProblems` |
| 7 | `colmap/src/colmap/mvs/patch_match.cc:365:31` | `field_identifier` | **member access** | operator `.` on `options_` | `PatchMatchController::ReadProblems` |
| 8 | `colmap/src/colmap/controllers/option_manager.cc:1018:20` | `string_literal` | **string literal** | `"PatchMatchStereo.max_triangulation_angle"` — the CLI option name, not a code reference | — |
| 9 | `colmap/src/colmap/controllers/option_manager.cc:1019:41` | `field_identifier` | **address-of binding** | operator `&` of a `->` member access on `patch_match_stereo` | `OptionManager::AddPatchMatchStereoOptions` |

**Note on #4/#5.** The table reports the *probe's* role, and the probe applies
the per-file rule (the design's implementation sketch, `:473`), so it labels
them "ordinary call argument". Under the design's **stated** rule they are
macro arguments, because `CHECK_OPTION_LE` is defined in `logging.h`, which
this translation unit includes. Both readings are recorded; §3.1 computes the
coverage under each, and the choice between them is an OWNER decision (§8.5).

### 2.1 Three reference tiers, plus the non-reference case

| Tier | Occurrences | What it means |
|---|---|---|
| **verified syntax observation** | 1, 6, 7, 9 | the AST proves the *role* and the location: a field declaration, two member accesses (`.`), one address-of binding (`&` of a `->` member access). Nothing is claimed about *which* declaration a use refers to |
| **heuristic candidate** | 2, 3, 4, 5 | #3: a plain identifier inside a member function of the declaring class — *likely* an implicit member access; a local or a global could shadow it. #2, #4, #5: arguments of a call whose callee *is a macro by textual evidence* — #2 from a definition in the same file, #4/#5 from a definition in a directly included header. No preprocessor ran, so the macro-ness is evidence, not proof |
| **unresolved semantic binding** | every occurrence, in relation to a declaration | which declaration an occurrence refers to. **Not available**: it needs name resolution, type checking and macro expansion (E1) |
| **not a code reference** | 8 | a string literal: the CLI option *name*. It is evidence that the option is exposed, not a use of the member |

### 2.2 The macro facts, stated precisely

- `PrintOption` is defined **four times** in the pilot (CP1 §B.4): `fusion.cc:78`,
  `patch_match_options.cc:43`, `patch_match.cc:45`, `texture_mapping.cc:1034`,
  with **different bodies** (`(option)` vs `option`, `std::endl` or not). The
  site at `patch_match_options.cc:58` is in the same translation unit as the
  definition at `:43` ✓ → the same-TU rule classifies it.
- `CHECK_OPTION_LE` is defined **once**, in `colmap/src/colmap/util/logging.h:61`,
  and `patch_match_options.cc` includes that header **directly** at `:32`.
  The macro is therefore **defined in the translation unit** (the header is
  part of it), though **not in the file**. The design's stated rule ("a macro
  defined in the same translation unit") classifies occurrences 4 and 5 as
  macro arguments; its implementation sketch (a **per-file** pre-pass,
  design `:473`) does not (§3.1).
- Without preprocessing, what can be decided: the callee is an *identifier*
  that is not a known function definition in the repository, and a macro of
  that name is defined in a file that this translation unit includes
  (directly, in the measured case). That is strong textual evidence but still
  **not a proof** — a conditional include, an include-root difference or a
  later `#undef` would change it, and the compiler, not the text, decides. The
  measured case has no `#undef` of the name in the file, and the include is
  unconditional.

### 2.3 The frozen CP1 sites, checked one by one

| CP1 §B.4 site | measured occurrence(s) | reachable by name? |
|---|---|---|
| `patch_match_options.h:56` (declaration) | #1 | yes (Slice 1 already indexes it) |
| `option_manager.cc:1018-1019` (CLI binding) | #8 (the string), #9 (the binding) | the binding yes; `:1018` is the option name, not a code reference |
| `patch_match_options.cc:58` (print) | #2 | yes |
| `patch_match_options.cc:91-93` (validation) | #3 (at `:91`), #4 (`:92`), #5 (`:93`) | yes in principle: #4/#5 are macro arguments under the design's **stated** rule (the macro is in the TU via `logging.h`); **#3 is not** — it is a plain identifier (the `if` guard) that neither the rule nor the per-file pre-pass classifies. The design's §4.2 row treats all of `:91-93` as macro arguments; the AST shows `:91` is not inside a call at all |
| `patch_match.cc:361-366` (behavioural read) | #6 (`:362`), #7 (`:365`) | **yes** — the field name *does* occur inside the frozen range (this resolves the uncertainty the design recorded: it is reachable) |
| `patch_match.cc:443-447` (source-image filter) | **none** | **no** — the filter reads the local `max_triangulation_angle_rad` (`:363-366`), a different identifier. No name-based method reaches it; only data flow (or a semantic provider) does. **This supersedes the design §7 wording** that the filter site is "reachable only through the copy at `:361-366`": the *file* is reached through those occurrences (#6/#7), the *filter site* is not — it contains no occurrence of the field's name |

## 3. Coverage of the Slice 2 design, computed from the measurement

### 3.1 The design: its rule and its implementation sketch disagree

The design states the macro-argument role as *"an `identifier` argument of a
`call_expression` whose callee is a **macro defined in the same translation
unit** (`preproc_function_def`)"* (`docs/m9-e7-reference-resolution-design.md:346`)
and sketches the implementation as *"a **per-file** macro-name pre-pass over
`preproc_function_def`"* (`:473`). A per-file pre-pass cannot see a macro
that is defined in an included header — but such a macro **is** defined in
the translation unit, because the header is part of it. The measurement
resolves that disagreement in favour of the **rule**:

- `CHECK_OPTION_LE` is defined at `colmap/src/colmap/util/logging.h:61`;
- `patch_match_options.cc` includes that header **directly** at `:32`;
- therefore the macro is defined in that translation unit, and occurrences
  #4 and #5 **are** macro arguments under the design's stated rule.

| reading of the design | code uses captured | which | code uses missed |
|---|---|---|---|
| the **stated rule** ("same translation unit") | **6 of 7** | #2, #4, #5, #6, #7, #9 | #3 (the plain identifier) |
| the **per-file pre-pass** (the implementation sketch) | **4 of 7** | #2, #6, #7, #9 | #3, #4, #5 |
| the stated rule **plus** the `implicit-member-access` tier of §5.1 | **7 of 7** | all | none |

Under both readings the *only* occurrence the design's role set cannot
classify is **#3**, the plain identifier at `patch_match_options.cc:91` — an
implicit member access inside a member function. So the design needs **one**
new role tier, not two; the second apparent gap (#4, #5) is an **implementation**
requirement (how the macro names of the *included* files are obtained), not a
new role.

The design's own text already flagged this: §9.2 item 3 asks how "a macro defined
in one TU and used in another" behaves — the measurement shows the pilot's case
is the *same* TU through an include, and the design's §4.2 conditional ("the
second one only if that macro is defined in the same TU (unknown; §9.2)") is now
**resolved in the affirmative** for #4/#5.

One further correction to the design's §4.2 row: it presents `:91-93` as one
macro-argument site (`CHECK_OPTION_LE(min_triangulation_angle,
max_triangulation_angle)`). The AST shows **three** occurrences there, and only
two are inside a call: `:91` is the `if (max_triangulation_angle >= 0.0f)`
guard — a **plain identifier**, not an argument of anything. The design
deliberately excludes plain identifiers (`:350-352`: "ambiguous between a
local, a global and a member"), so the measurement is not a surprise there —
but it means the design's own listing of `:91-93` as a macro site is imprecise,
and the excluded occurrence is the one that gates the whole validation block.

### 3.2 The scope variants, measured

The design offers an *include-scoped* scope (the anchor's file plus the files
whose include text resolves to the anchor's header) and a *repository-wide
name-scoped* scope. Measured on this identifier. **Universe: the 9 C/C++
occurrences** (1 declaration, 1 string literal, 7 code uses) — the same
denominator in every row:

| scope | occurrences inside the scope | what it loses |
|---|---|---|
| include-scoped, **direct** includes only (what the E8 rooted resolution does) | **7 of 9** — #1 (the header itself, the anchor's file), #2, #3, #4, #5 (in `patch_match_options.cc`, which includes the header at `:30`), #8 and #9 (in `option_manager.cc`, which includes it at `:52`) | **#6 and #7** (the behavioural read): `patch_match.cc` does **not** include `colmap/mvs/patch_match_options.h`; it reaches it **transitively** (`patch_match.cc:30` → `patch_match.h:35` → the header) |
| include-scoped, **transitive** include chain | **9 of 9** | nothing, for this identifier — but the chain is a *candidate* chain, not the compiler's include graph |
| repository-wide, **exact** identifier | **9 of 9** | nothing for this identifier: the exact name occurs in 4 files only, and there is exactly **one** declaration of it |

The **code uses** (7) captured by the roles, per scope:

| scope | stated rule | per-file pre-pass |
|---|---|---|
| repository-wide | 6 of 7 | 4 of 7 |
| include-scoped, direct includes | 4 of 7 (#2, #4, #5, #9) | 2 of 7 (#2, #9) |
| include-scoped, transitive chain | 6 of 7 | 4 of 7 |

So the include-scoped variant, as specified, **loses a relevant location**
under every reading — and it loses the behaviourally most interesting one.
The repository-wide variant loses nothing here; the price is that in general
it returns every same-named member in the tree (measured on another name:
`value` has **27** distinct declarations across first-party and third-party
files, e.g. `colmap/src/colmap/controllers/base_option_manager.h:139:17`,
`colmap/src/thirdparty/PoissonRecon/Geometry.h:634:79`,
`colmap/src/thirdparty/VLFeat/mathop.h:55:38`), and without a binding those
are indistinguishable from the one we want. **The material false-positive
risk of the repository-wide scope is therefore not "wrong files" but "wrong
declarations of the same name"** — and for `max_triangulation_angle` it is
measured at zero.

### 3.3 Can the data distinguish the role without a semantic binding?

**Yes for the role, no for the binding.** The AST gives the role exactly
(§2) and the enclosing function exactly; the enclosing class is derived
lexically from the qualified function name (§1.2), which is reliable for the
measured occurrences but is a derivation, not a scope-resolution proof. What
the syntax cannot give is the *declaration* an occurrence belongs to: that
needs name resolution, type checking, macro expansion and include-path-dependent
visibility — the E1 boundary, which stays unauthorized.

### 3.4 Which part of CP1 D4-C5 could be met

D4-C5 asks the affected set to include the declaration, the print and
validation sites, the read and filter sites, and the CLI binding. The frozen
text is stricter than a generic list: CP1 §B.4 says Riltra "must report: the
declaration site, the option binding, the read site and the **filtering
site**" (`docs/m9-cp1-pilot-spec.md:472-473`), and accepts an `UNKNOWN` verdict
"with the affected files listed" (`:474-475`).

| CP1 site | reachable with the design **as written** | reachable with the **refined** design (§5.1) |
|---|---|---|
| declaration `patch_match_options.h:56` | yes (Slice 1) | yes |
| print `patch_match_options.cc:58` | yes | yes |
| validation `patch_match_options.cc:91-93` | **partly**: #4/#5 yes under the stated rule (no under the per-file pre-pass); **#3 no** under both | yes, as candidates: #3 becomes `implicit-member-access`, #4/#5 stay macro arguments with include-based visibility |
| read `patch_match.cc:361-366` | only if the scope follows the **transitive** include | yes |
| filter `patch_match.cc:443-447` | **no** | **no** — the site uses the local `max_triangulation_angle_rad`; no name-based method reaches it |
| CLI binding `option_manager.cc:1018-1019` | the binding yes (`:1019`); `:1018` is the option-name string | same |

**D4-C5 cannot be met in full by any name-based method**, because one frozen
site is not a use of the member's name. The refined design can reach **5 of
the 6** listed locations (all but the filter site) — and only with a
transitive include scope, the include-based macro visibility, and the one new
role tier. D4-C5 therefore remains **UNMET** on the position this document
takes; that position is the owner's to change (§8.3). D1-C6 and D2-C6 are
untouched: no code changed here, so they stand exactly as the CP2 record
leaves them.

## 4. False positives and false negatives

| Kind | Case | Measured effect |
|---|---|---|
| false negative | the filter site uses a local variable | `patch_match.cc:443-447` is unreachable by name (1 frozen site) |
| false negative | a macro defined in an included header, under a **per-file** macro pre-pass | #4, #5 missed (2 of 7 uses) under the implementation sketch — **not** under the design's stated same-TU rule |
| false negative | an implicit member access | #3 missed because plain identifiers are excluded (1 of 7 uses) |
| false negative | a direct-include scope | #6, #7 lost (2 of 7 uses) |
| false negative | a use in a `.cu`/`.inl` file | none for this identifier; in general a real blind spot (CUDA unsupported, `.inl` not indexed) |
| false negative | a use through a renamed or aliased member | not measurable here; a name-based method cannot see it at all |
| false positive | a same-named member of another type | measured **0** for this identifier; **27 declarations** for `value` in the same tree — the general risk, and the reason the *declaration* must stay a separate fact from the *occurrences* |
| false positive | a local variable or a global shadowing the member | cannot be excluded by syntax — this is why #3 stays a candidate |
| false positive | **volume** of the `implicit-member-access` tier | **not measured**: every plain identifier matching a field's name inside any member function would become a candidate. In this file the class derivation is exact, but the general volume is unknown — a Tier-2 role must therefore carry its ambiguity and must not be counted as a use |
| false positive | a string literal | measurable and separable: #8 is a `string_literal` node, not an identifier — it must not be reported as a use |

## 5. Design comparison

### Alternative 1 — `POTENTIAL_REFERENCE` in the existing graph

| | |
|---|---|
| gains | the occurrences travel through the existing graph machinery; `callers` at the field anchor would list them; `impact` would list them as candidate paths (the verdict would still be `UNKNOWN`: `impact_verdict` returns it whenever any edge is not `Exact`, lacks an endpoint, or carries an unresolved reason — `edges.rs:1377-1406`) |
| what it additionally requires | the traversal must actually **thread** the new kind (the impact/trace walk selects edges by kind — a kind it does not know is invisible), and the verdict rule above must be *left* to return `UNKNOWN` for a candidate edge rather than widened to call it proven. Neither is a storage change; both are behaviour changes in the query path, which is why this alternative is larger than Alternative 2 in everything but storage |
| costs | a new edge kind in a graph whose consumers read edges as relations; the **meaning of `callers` changes** (from call sites to call sites + reference candidates), so the MCP tool description ("Call sites that reference the anchor symbol", `mcp.rs:270`) and the CLI contract would need an update — a contract change; the impact path set for a field changes from empty to non-empty, which is exactly what CP1 D4-C5 records as the *unmet* gate, so the change must be reconciled with the frozen record; the occurrence facts must be stored anyway (a new table), so the storage cost is identical to Alternative 2 — only the *presentation* differs |
| honesty risk | a candidate that looks like an edge in a relation graph is the shape of the failure CP1 §D.0 forbids ("a heuristic edge is presented as a verified `CALLS` edge" — not identical, but the same class of misreading: a consumer that filters by edge kind, not by precision) |
| identity limits | unchanged: the edge still cannot say *which* declaration it belongs to; the `unresolved` reason carries that |

### Alternative 2 — separate reference occurrences (facts, not edges)

| | |
|---|---|
| gains | the occurrences stay **facts with roles** ("member access", "macro argument (candidate)", …) and are never presented as relations; the graph's meaning, `callers` and `impact` keep their current meaning; the verdict stays `UNKNOWN` and completeness stays `partial`; the affected locations can be listed as locations with their roles and the "not bound" caveat — which is precisely the shape CP1 §B.4 accepts ("an impact verdict of `UNKNOWN` with the affected files listed is acceptable") |
| costs | the occurrences need a surface: a query path and a place in a response. Any surface is a contract delta (a new tool, or a new section in an existing response). The occurrences are **not traversable**: `trace`/`impact` cannot walk them, so workflow 4 stays partial — which is honest, not a defect |
| honesty risk | the lowest of the three: nothing unbindable enters the relation graph; a consumer that reads "affected locations" sees roles and an explicit "not bound" statement |
| identity limits | the same as Alternative 1: no binding |

### Alternative 3 — semantic binding (capability boundary, not authorized)

Binding an occurrence to a declaration needs a C++ semantic provider:
name resolution, overload resolution, macro expansion and include-path
visibility. That is **E1**, deferred by D-M9-2, and it needs a compilation
database that MMSP does not have (design gate §2.3: no `compile_commands.json`,
no `CMAKE_EXPORT_COMPILE_COMMANDS`, no `.clangd`). **Nothing in this
measurement changes that.** It remains the only path to a *bound* reference,
and it stays unauthorized.

### 5.1 The design refinement the measurement forces

Whatever surface is chosen, the design needs these corrections — each one is
traced to a measured occurrence, not to a preference:

1. **One new role tier: `implicit-member-access`.** A plain identifier inside a
   member function whose class is derivable from the qualified function name
   (occurrence #3). It must be reported as a **candidate with its ambiguity
   stated**, never counted as a proven use: a local or a global can shadow it,
   and the tier's volume in a large tree is **not measured** (§4).
2. **The macro role needs an implementation decision, not a new role.** The
   stated rule ("defined in the same translation unit") already classifies
   #4/#5; the per-file pre-pass does not. Implementing the stated rule needs
   the macro names of the *included* files — either by parsing the headers
   whose include text resolves to a local file (the E8 machinery), or by a
   repository-wide macro-name set. Both are **candidates** (a conditional
   include or an include-root difference would change visibility), so the role
   must carry that precision. If the owner prefers the per-file pre-pass, the
   design must say that #4/#5 are out of scope — not that they do not exist.
3. **The scope must follow the include chain**, at least transitively for the
   header chain (otherwise the behavioural read is lost — measured: a direct-only
   scope drops `patch_match.cc`).
4. **A string literal is not a use** (occurrence #8) and must never be
   reported as one; it is evidence of the CLI option *name*.
5. **The design's §4.2 listing of `:91-93` as one macro site must be corrected**
   to three occurrences with different roles (§3.1).

## 6. Compatibility and output-honesty risks

| Risk | Assessment |
|---|---|
| the meaning of `callers` changes | only under Alternative 1. Under Alternative 2 the graph and its queries are untouched |
| the impact path set changes | only under Alternative 1 (from empty to candidate paths). Under Alternative 2 the verdict stays `UNKNOWN` and the path set stays empty; the affected locations are listed as locations |
| a new fact kind and a schema version | **both** alternatives need somewhere to store the occurrences → a new table and a `SCHEMA_VERSION` 4 → 5 (a schema change is explicitly outside this task's authorization and is listed as an OWNER decision) |
| the MCP tool catalog | neither alternative needs a new tool *in principle*; under Alternative 2 the occurrences need *some* surface, and that surface is a contract decision (a new tool, or a new section in an existing response). Adding a tool would change the frozen twelve-tool catalog and the CP2 grader's exact-set check → an OWNER decision |
| the "not bound" statement must travel with every occurrence list | otherwise a location list reads as a relation list — this is the honesty invariant that both alternatives must keep |
| `PARSER_VERSION` | any extraction change bumps it (1 → 2 → 3) and forces a rebuild of every existing index; a schema change additionally invalidates the existing databases (`schema_outdated`) |
| the frozen CP1/CP2/CP3 evidence | untouched by this task; a future implementation must not retroactively alter the recorded pilot results |

## 7. Recommended minimum design

**Recommendation: Alternative 2 — store the occurrences as facts with roles
and present them as locations with an explicit "not bound" statement, not as
graph edges.** The reasons are the measurement's, not the complexity's:

1. **The data is a location list, not a relation.** Every one of the 7 code
   uses is a *name occurrence* whose binding is unprovable. Putting an
   unbindable candidate into a relation graph changes what `callers` means and
   risks exactly the misreading CP1 §D.0 forbids.
2. **It keeps CP1 D4-H4 literally true** (see §8): no edge is claimed for the
   member read, the address-of binding or the macro arguments — because no
   edge is claimed at all. Alternative 1 would claim them.
3. **The frozen acceptance for workflow 4 already accepts this shape**: CP1
   §B.4 accepts "an impact verdict of `UNKNOWN` with the affected files
   listed" — a location list with an `UNKNOWN` verdict.
4. **It is not smaller in storage** (both need the same table), and it is
   smaller in *behavioural* change: the graph, `callers`, `callees`, `trace`,
   `impact`, the MCP catalog and the CLI contract stay as they are, except
   for the surface the occurrences are shown in — which is an OWNER decision,
   not a design fact.

The **role taxonomy** and the **scope** must be refined as in §5.1. With the
refinement, the design classifies **7 of 7** code uses — 3 with a verified role
(#6, #7, #9) and 4 as candidates (#2, #3, #4, #5); without it, it classifies
6 of 7 under the stated rule and 4 of 7 under the per-file pre-pass — and,
with a direct-only include scope, loses the behavioural read under either
reading. Under Alternative 2 none of these candidates is a graph edge, so none
of them is a claimed relation.

**What this recommendation does not claim.** It does not claim that D4-C5 can
be met: the filter site at `patch_match.cc:443-447` is not a use of the
member's name, and no name-based method reaches it. It does not claim a bound
reference: that is E1.

## 8. CP1 D4-H4 — open OWNER decision, not resolved here

CP1 §D.4, honesty item 4 (verbatim, `docs/m9-cp1-pilot-spec.md:827-829`):

> 4. The two macro sites are not resolved to function definitions, and no edge
> is claimed for the member read, the address-of binding, or the macro
> arguments.

The CP2 re-evaluation records it as **D4-H4 PASS** (`docs/m9-cp2-reevaluation.md:367`),
with the pilot's empty affected set, and D4-C5 as **UNMET** (reason
`affected-set-empty`, `:371`).

The consequences, stated and **not decided here**:

| if the occurrences become | consequence for CP1 D4-H4 | consequence for `impact` / the honesty gate |
|---|---|---|
| `POTENTIAL_REFERENCE` **edges** (Alternative 1) | a **literal conflict**: edges *are* claimed for the member read, the address-of binding and the macro arguments. The criterion was written for a tool that had no member-reference capability; a new capability makes the literal text untrue. Reconciling it needs an OWNER decision (a recorded interpretation, or a re-scoped criterion) — the frozen text cannot be edited by the implementer | the verdict stays `UNKNOWN` and completeness `partial` (the existing rules force it), but the affected-path set becomes non-empty; the honesty gate's "no fabricated relation" is met only if every candidate carries its role, its precision and its unresolved reason |
| **separate occurrence facts** (Alternative 2) | **no literal conflict**: no edge is claimed, so the criterion's text stays true. What changes is the *answer's* content: the locations become reportable, each with its role | the impact path set stays empty and the verdict stays `UNKNOWN`; the locations are listed next to it, each with its role and the "not bound" caveat. A location list is **not** a graph affected-set (the frozen text asks for the affected set to *contain* the sites), so the answer must not present the list as one |

**What needs the OWNER's decision (and is not decided in this document):**

1. whether a *labeled, unbinding* reference candidate may be reported as a
   **graph edge** (which would require an explicit interpretation of D4-H4),
   or whether reference candidates must stay **outside the edge graph**;
2. if they stay outside, **where** they are surfaced: a new MCP tool (which
   would change the frozen twelve-tool catalog and the CP2 grader's exact-set
   check), a new section in an existing response, or the CLI only;
3. whether the CP1 §D.4 **capability** criterion D4-C5 is re-scoped to "the
   affected set contains the reachable locations, with the filter site
   explicitly out of reach because it uses a local variable" — or whether
   D4-C5 stays as it is (UNMET), which is the position this document takes;
4. whether the schema change (a new table, `SCHEMA_VERSION` 4 → 5) and the
   `PARSER_VERSION` bump are accepted for a capability that cannot meet
   D4-C5 in full;
5. **which macro-visibility rule the design adopts**: the stated same-TU rule
   (which classifies #4/#5, but needs the macro names of included files and
   therefore a candidate precision for include-based visibility) or the
   per-file pre-pass (which cannot see them and must record #4/#5 as out of
   scope). The two readings disagree inside the design itself (§3.1), and the
   choice changes both the coverage and the implementation size;
6. whether the `implicit-member-access` candidate tier is acceptable at all,
   given that its false-positive volume is **not measured** (§4).

## 9. Proposed implementation scope (NOT authorized here)

Recorded so the owner can judge it; **no code was written**:

**Open design questions the implementation must settle first** (none of them
is authorized here):

- **How the macro names of included files are obtained** — by parsing the
  headers an include text resolves to (the E8 rooted resolution), or by a
  repository-wide macro-name set. The measured consequence of *not* doing it:
  #4/#5 are missed (§3.1).
- **How a conditional include is handled.** The measured include is
  unconditional; a conditional one would make visibility a candidate at best.
  The honest default is to mark the occurrence's precision, not to decide the
  condition.
- **Where the enclosing class of an `implicit-member-access` candidate comes
  from.** The probe derives it lexically from the qualified function name; the
  product would need a column or a derivation with the same caveat, or the
  tier cannot be scoped to the declaring class.
- **The candidate volume.** Unmeasured (§4): it must be measured on a real
  tree before the tier is exposed, or the tier must be opt-in.

| Component | Change | Note |
|---|---|---|
| `crates/cli/src/syntax.rs` | extract the occurrence roles (§5.1) into a new fact collection; the taxonomy and the tiers are as measured | `PARSER_VERSION` 2 → 3 |
| `crates/cli/src/store.rs` | a new table for the occurrences (the name must not be a SQL keyword) + `SCHEMA_VERSION` 4 → 5 + the two hand-written lists (copy, promote) that fail silently | the traps named in the design §2.7 |
| `crates/cli/src/index.rs` | ingest + counters + the parity test | the parity test must carry the new facts |
| `crates/cli/src/store.rs` (query) | a name-scoped occurrence lookup, and the **transitive** include-chain scope | the scope refinement is forced by the measurement |
| `crates/cli/src/response.rs` | where the occurrences are shown, with the role, the precision and the "not bound" statement | the surface is an OWNER decision (§8.2) |
| `crates/cli/src/mcp.rs` | **no new tool** unless the owner decides otherwise | a new tool would change the frozen catalog |
| `crates/cli/src/edges.rs` | **no change** under Alternative 2 | `POTENTIAL_REFERENCE` is not recommended |
| tests | the role taxonomy, the tiers, the scope, the parity, the honesty statements | — |

Scope estimate: 4–6 production files, one new table, one new fact collection,
one query path, one include-informed macro-name source (the largest unknown of
the four open questions), no graph change, no new dependency, no MCP tool
(unless decided otherwise).

## 10. Verification limits

- The measurement covers **one identifier** in **one repository** at **one
  pin**. It is a measurement of `max_triangulation_angle`, not of the general
  behaviour of the design: the false-positive rates for other names are
  inferred from one measured name (`value`, 27 declarations), and the
  false-negative behaviour of a *renamed* or *aliased* member is **not
  measured** (it is not measurable by name).
- The occurrence roles come from the AST of one grammar version
  (`tree-sitter-cpp 0.23.4`, the pinned one). A different grammar could
  classify the same source differently.
- The include chain is **textual**: `patch_match.cc` → `patch_match.h` → the
  header. That is a candidate chain, not the compiler's include graph; a
  conditional include or an include-root difference would change it. The
  measured chain has no conditional include on either hop.
- **The candidate volume of the `implicit-member-access` tier is not
  measured.** One occurrence of one field in one file is not a distribution;
  the tier's false-positive rate in a large tree is unknown.
- **The design's rule and its implementation sketch disagree** (§3.1). This
  document records the disagreement and the measurement that resolves it in
  favour of the rule; it does not amend the design — that is an OWNER decision
  (§8.5). A reader who takes the implementation sketch as the design will get
  a different coverage number, and both numbers are stated here.
- The probe was **not** run over the whole tree: the universe was established
  by an exact-identifier text search (45 lines, 9 of them C/C++) and the probe
  was run on the 4 files that contain code occurrences. A C/C++ occurrence in
  a file type the search did not cover (e.g. a `.ipp` header) would be missed;
  the search covered `.h .hh .hpp .hxx .cc .cpp .cxx .c .cu .cuh .inl` plus
  a whole-tree pass over every file type.
- **No production code, schema, graph, CLI/MCP contract or frozen evidence
  was changed by this task**: the only artefacts are this document and
  `docs/pilots/e7-slice2-evidence/`. `cargo test --locked` was **not** re-run
  for this documentation-only change; the last full run was 723 passed /
  0 failed at the Slice 1 commit `4ea3398`, and this task changes no code.
- The MMSP snapshot was **not** re-indexed and the CP2 benchmark was **not**
  repeated; the numbers quoted from CP2 are the frozen ones.
