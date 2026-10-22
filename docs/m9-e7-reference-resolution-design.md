# M9 E7 — reference-level resolution: design and acceptance specification

**Status: DESIGN ONLY (Phase A). No production code, no schema, no dependency, no
MCP/CLI contract change.** This document authorizes nothing; it is the
evidence-driven design and acceptance specification for a *possible* E7
implementation, which needs its own OWNER GO. Nothing in this task changed
Riltra's runtime behaviour: the only files touched are documentation.

- Date: 2026-10-22
- Branch: `m9-multirepo-release-candidate`
- Baseline commit: `b0fea0ccf886366a4353cb1aec56b42fae0332eb` (verified local
  == remote, tree clean apart from the untracked owner-local `AGENTS.md`)
- Pilot pin (unchanged): MMSP `d62f06a576d8e76b705c8ecc57fc380f7ebc9c44`,
  tree `c5482ea00481675e0aa921538f560947b68dca12`
- Scope: E7 Phase A — C++ **data-member** reference resolution and option
  plumbing, evaluated against MMSP as the fixture.

---

## 1. Baseline repository and evidence used

### 1.1 What was read

| Source | Used for |
|---|---|
| `docs/m9-design-gate.md` | M9 objective (§1.2), the honest-verification vocabulary (§1.5), the relationship-graph rules (§1.6), the MMSP inventory and the "no compilation database" fact (§2.3), the deferred C++ provider decision D-M9-2 (§6) |
| `docs/m9-cp1-pilot-spec.md` | §B.1–B.2 (definitions/references, `PatchMatch` cross-TU), §B.4 (**the `max_triangulation_angle` ground truth**), §B.6 (ambiguity table), §B.9 (what a syntax-only index can and cannot attribute), §D.1–D.5 (the frozen acceptance checklist), §E (E7 as a release-blocking gap) |
| `docs/m9-cp2-reevaluation.md` | §7.2 (the measured D.5 tool sequence on the pilot: `riltra_code_search max_triangulation_angle` → **0 hits**; impact → `UNKNOWN`, 0 paths) |
| `docs/m9-cp3-evidence.md` | §7 (the OpenCode agent explicitly refused to answer the wiring question), §8 (the E9 fixed-cost measurement, ≈ 472 ms per warm invocation), §9 (E7 "the highest-value remaining gap"; the `.inl` coverage gap) |
| `docs/m9-cp2-harness-correction.md` | the CP2 harness state (the corrected 12-tool grader, the frozen evidence rules) |
| `crates/cli/src/*` | the implementation facts cited in §2, read directly |

### 1.2 Implementation files that carry the behaviour

| Area | File | Load-bearing places |
|---|---|---|
| Extraction | `crates/cli/src/syntax.rs` | `Language`/extension policy `:14-22`, `:58-80`; `Fact` `:100-117`; `FileFacts` `:119-127`; the per-language walker `:572-632`; C++ arm `:617-625`; `push_def` `:317-329`; `push_c_def` `:331-343`; `push_call` `:411-440`; `terminal_span` `:520-556` |
| Storage | `crates/cli/src/store.rs` | `SCHEMA_VERSION = 4` `:11-12`; DDL `:157-248`; `SymbolRow` `:369-383`; `CallRow` `:400-418`; `copy_mapped_facts` `:1227-1269`; promotion `:1102-1125`; search `:1995-2046`; `EDGE_ORDER` `:1740-1741` |
| Indexing | `crates/cli/src/index.rs` | `IndexSummary` `:26-51`; `ingest_file` `:383-514`; digest reuse `:574-579`; freshness reasons `:2751-2790`; `resolve_edge_anchor` `:1302-1488`; completeness reasons `:2101`, `:2372`, `:2388` |
| Graph | `crates/cli/src/edges.rs` | `EdgeKind` `:15-31`; syntax derivation `:167-269`; `CALLS` construction `:448-466`; `REFERENCES` `:544-561`; budgets `:755-782`; `impact_verdict` `:1377-1406` |
| CLI | `crates/cli/src/args.rs`, `main.rs`, `response.rs` | command surface `args.rs:406-419`; response documents `response.rs:51-262`; exit codes `main.rs:184-217` |
| MCP | `crates/cli/src/mcp.rs` | the twelve tools `:214-415`; the "twelve tools total" contract note `:5`; the semantic tool is the only non-read-only one `:385-393` |

### 1.3 Grammar facts (pinned dependency, not assumed)

`tree-sitter-cpp 0.23.4` — the dependency is pinned by `Cargo.lock`
(`tree-sitter-cpp 0.23.4`; the manifest range is `crates/cli/Cargo.toml:18`,
`"0.23"`), and the grammar label the harness reports is `syntax.rs:46`
(`Language::Cpp => "cpp:0.23.4"`). The node shapes below were read from the
crate's `src/node-types.json` in the Cargo registry:

| Node | Fields |
|---|---|
| `field_declaration` | `type` (required, `type_specifier`), `declarator` (multiple, `_field_declarator`), `default_value` (multiple) |
| `_field_declarator` | subtypes: `field_identifier`, **`function_declarator`**, `pointer_declarator`, `reference_declarator`, `array_declarator`, `attributed_declarator`, `parenthesized_declarator`, `template_method`, `operator_name` |
| `field_expression` | `argument` (required, expression), `field` (required: `dependent_name` \| `destructor_name` \| `field_identifier` \| `qualified_identifier` \| `template_method`), `operator` (required: `.` \| `->` \| `.*`) |
| `pointer_expression` | present (the `&` address-of form) |
| `preproc_function_def` | present (a macro definition) |

Two consequences follow directly and are load-bearing for §4:

1. **An in-class method declaration is also a `field_declaration`** (its
   declarator is a `function_declarator`). Any extraction that treats every
   `field_declaration` as a data member would turn every in-class method
   declaration into a bogus "field" symbol. The exclusion is specified in
   §4.1.
2. The four constructs E7 needs are **present in the grammar** and are simply
   not matched by Riltra's walker. No grammar change, no dependency change and
   no compilation database are required to *see* them.

---

## 2. Root-cause analysis

Confidence labels: **HIGH** = read from the source and/or measured in the
pilot; **MEDIUM** = read from the source, not exercised end-to-end; **LOW** =
hypothesis that the implementation phase must measure.

### 2.1 Why the field declaration is not a searchable symbol

`search` matches **definition rows only**. The doc comment at
`store.rs:1995-1997` states it: *"Search definitions by name: FTS5 prefix
matches first, then LIKE substring matches… Calls and imports are never symbol
hits."* The FTS index covers `symbols.name` alone (`store.rs:218-226`), and the
LIKE pass filters `s.name` (`store.rs:2025-2031`).

A C++ data member is a `field_declaration` node, and the C++ arm of the walker
(`syntax.rs:617-625`) matches only `function_definition`, `struct_specifier`,
`class_specifier`, `preproc_include`, `call_expression` and `declaration`
(direct-initialisation only). There is **no `field_declaration` arm**, so the
declaration of `max_triangulation_angle` produces **no `symbols` row at all**,
and therefore no search hit.

Measured on the pilot: `riltra_code_search max_triangulation_angle` → **0
hits** ("the data member is not an indexed symbol (reported, not guessed)",
`docs/m9-cp2-reevaluation.md:512`). The agent side repeats it: *"I cannot
answer how `max_triangulation_angle` is wired from the command line into
source-image selection, and cannot state its default"*
(`docs/m9-cp3-evidence.md:567-571`).

**Confidence: HIGH.**

### 2.2 What tree-sitter captures today

| Construct | Captured? | Evidence |
|---|---|---|
| Field declaration (`double max_triangulation_angle = -1.0f;`) | **No** | no `field_declaration` arm in `syntax.rs:572-632`; the node exists in the grammar (§1.3) |
| Member expression (`options_.max_triangulation_angle`) | **No** | no `field_expression` arm; a member *read* is not a `call_expression`, so it produces no `calls` row either |
| Address-of binding (`&patch_match_stereo->max_triangulation_angle`) | **No** | no `pointer_expression` arm; the inner `field_expression` is ignored as well |
| Macro argument (`PrintOption(max_triangulation_angle)`) | **No** for the argument | the invocation is an ordinary `call_expression` → one call candidate named `PrintOption` (`syntax.rs:411-440`); the *argument* identifier matches no arm, so the field name is recorded nowhere. The macro **definition** is not extracted either (no `preproc_function_def` arm), so a macro invocation is currently indistinguishable from a function call in Riltra's facts. The macro **body** is opaque by grammar construction (`preproc_arg` is a single token). |

Consequence: the whole option-plumbing chain of CP1 §B.4 is invisible except
for the *call candidates* that happen to name the macros.

**Confidence: HIGH** (source), **MEDIUM** for the "no arm exists" claim about
`pointer_expression`/`field_expression` in other languages (grep over
`crates/cli/src` finds no occurrence of those node names outside the grammar
crate).

### 2.3 At which stage the information is lost

**Extraction — and only extraction.** The other layers faithfully propagate
the absence:

| Layer | State | Evidence |
|---|---|---|
| Extraction | **loss point** | `FileFacts` has exactly three fact collections: `definitions`, `imports`, `calls` (`syntax.rs:119-127`); `Fact.kind` is documented as only `"definition" \| "import" \| "call"` (`syntax.rs:102`) |
| Indexed facts | nothing to lose | the code-index schema has `meta, snapshots, files, symbols, imports, calls, code_edges, symbols_fts, semantic_imports, semantic_facts` (`store.rs:157-248`) — **no reference table**; `symbols` has no member/field kind; `calls` holds only call candidates |
| Graph resolution | cannot recover | syntax derivation builds only `IMPORTS` and `POTENTIAL_CALL` (`edges.rs:181-267`), with `callee: None` for syntax candidates; `CALLS`/`REFERENCES` require a semantic provider (`edges.rs:448-466`, `:544-561`), and the pilot has none (D-M9-2) |
| Query | cannot recover | `search` reads `symbols` only (§2.1); `callers`/`callees` are **position-anchored** to an identity endpoint (`index.rs:1302-1488`), and with no field symbol the anchor falls back to "nearest enclosing symbol" with the uncertainty `syntax-only anchor: no semantic identity at this position` (`index.rs:1486`), inserted as the reason `syntax-anchor-uncertain` at `index.rs:2098` and `:2379` — i.e. the enclosing `struct`/`class`, whose edges are never produced (`edges.rs:70-87` requires an owner of kind `function`/`method`) |

This is exactly the CP1 §B.9 conclusion, re-derived from the current source:
*"a member read, an address-of binding and a macro argument are not call
expressions. The symbol `max_triangulation_angle` therefore has **no** reverse
edge in a syntax-only index"* (`docs/m9-cp1-pilot-spec.md:592-598`).

**Confidence: HIGH.**

### 2.4 Can a field reference be recorded without a compilation database?

**Yes for the occurrence, no for the binding.** A `field_declaration` node and
a `field_expression` node are both in the AST with byte offsets, line/column
and the identifier text; recording them needs no compiler, no
`compile_commands.json` and no include resolution (MMSP has none — design gate
§2.3). What is *not* available without a compiler is the binding: which
declaration a use refers to (member vs. global, shadowing, overloads, macro
expansion, template instantiation, include-path-dependent visibility).

**Confidence: HIGH.**

### 2.5 What relationship confidence is provable without a C++ semantic provider

| Claim | Provable? | Why |
|---|---|---|
| "a field named `X` is declared at L1, inside class `C`" | **yes, syntactic** | the `field_declaration` node carries the declarator and the enclosing `class_specifier` |
| "the identifier `X` occurs at L2 in file F, in role *member access* / *address-of binding* / *macro argument*" | **yes, syntactic** | the node kinds and their fields (§1.3) |
| "L2 refers to the declaration at L1" | **no** | requires name resolution + type checking; needs a C++ semantic provider (E1, D-M9-2 deferred) |
| "no other field named `X` exists" | **no** | same-name fields on other types are indistinguishable without types |

Therefore the honest relation is a **candidate**: "an occurrence of the name
`X` that *may* refer to a declaration named `X`". It must carry the existing
honest vocabulary — `precision: "Heuristic"`, `provider: "syntax"` and an
`unresolved` reason — exactly as `POTENTIAL_CALL` does today
(`edges.rs:253-266`). The `verification` value (`"Unverified"`/`"Stale"`) is
**derived at query time** from freshness and the provider verdict
(`index.rs:1784-1792`), not stored on the edge.

**Confidence: HIGH.**

### 2.6 Showing affected locations without claiming complete impact

The existing surface can carry it honestly, with **no new command and no new
MCP tool** — but not for free, and the two surfaces differ:

- **`callers` / `riltra_code_callers` can surface the candidates today.** Its
  name branch (`store.rs:1789`) matches an edge whose
  `edge_kind = 'POTENTIAL_CALL' AND callee_kind = 'unknown' AND callee_name = ?9`
  and whose caller file is in the candidate path set; `index.rs:2011-2027`
  passes the anchor's display name and `caller_candidate_paths(...)`
  (`index.rs:1615-1676`) into that query. A reference candidate therefore
  matches **iff** it stores the occurrence's name in `callee_name` — a
  requirement §4.3 states explicitly.
- **`impact` / `riltra_code_impact` cannot surface them as they stand.**
  `traverse_impact` builds its `EdgeSelection` with `candidate_name: None` and
  `candidate_paths: &[]` (`edges.rs:1261-1262`; the same defaults are in the
  other selection builder at `edges.rs:1022-1023`), so the name branch can
  never match there and the traversal returns no reference paths. Surfacing
  the candidates through `impact` therefore requires threading the anchor's
  name and the candidate path set into `traverse_impact` — a component in
  §6, not a free consequence of the edge kind.
- The **verdict** is unaffected either way: `impact_verdict`
  (`edges.rs:1377-1406`) returns `UNKNOWN` for any edge that is not
  `CALLS`/`REFERENCES` with `precision == "Exact"` and no `unresolved`
  reason, so a new heuristic edge kind keeps the verdict `UNKNOWN`
  **automatically**, with no change to the verdict logic.
- Completeness stays `partial` and keeps its reasons
  (`index.rs:2101`, `:2372`, `:2388`), including
  `empty-result-not-coverage-proof`.
- CP1 §B.4 already accepts exactly this shape: *"An impact verdict of `UNKNOWN`
  with the affected files listed is **acceptable** (M6 policy forbids SAFE/LOW
  verdicts)"* (`docs/m9-cp1-pilot-spec.md:472-475`).

The contract deltas this needs (a new `kind` value, the occurrence role, a
`scope` member and a new fact counter) are listed as ONE OWNER decision in
§10.5.

**Confidence: HIGH** (for the verdict and completeness behaviour, read from
the source).

### 2.7 Consequences for schema, incremental indexing, freshness, query cost, compatibility

| Area | Consequence | Evidence |
|---|---|---|
| Schema | a new fact kind needs a new table → `SCHEMA_VERSION` 4 → 5 | the DDL is one string (`store.rs:157-248`); `migrate()` runs it on every open **of an older database** (`store.rs:255-312`, with an early return at `:264-266` when the version already matches); the read path refuses a version mismatch (`store.rs:145-149`, surfaced at `mcp.rs:544-553`) |
| **Extraction version** | **any change to `syntax.rs` extraction requires `PARSER_VERSION` 1 → 2** (`store.rs:13-15`: *"Bump whenever `syntax.rs` extraction changes; freshness treats a mismatch as stale"*). Without the bump, an incremental run over an existing v4 database **reuses** the unchanged files' old facts (`index.rs:574-579`) and the new facts never appear; with the bump, the run falls back to a full rebuild with the reason `parser-changed` (`index.rs:2561-2562`). This is a **third** silent-failure item, next to the two lists below | `store.rs:13-15`, `index.rs:574-579`, `index.rs:2561-2562` |
| Incremental indexing | the new table must be in **two** hand-written lists, and both fail **silently** if missed: the reused-facts copy (`store.rs:1227-1269`) and the promotion `INSERT … SELECT` (`store.rs:1102-1125`) | a table missing from the copy list loses all reused facts; missing from the promote list is dropped from the live DB after every build |
| Parity | the parity guard test enumerates the fact tables and columns explicitly (`index.rs:3049-3234`, dumps at `:3083`, `:3119`, `:3153`) and must be extended | a new table that is not dumped is not parity-checked |
| Freshness | **a new fact table does not change any freshness verdict**: the content identity is a digest over `files(path, digest)` alone (`store.rs:961`), and the verdict compares the snapshot row, the live git fingerprint, the parser/grammar/config fingerprints and the file content diff (`index.rs:2751-2790`) — none of which knows about a fact table. The *parser-version* change, however, does change a verdict: it forces `parser-changed` and a full rebuild (see the row above) | `store.rs:961`, `index.rs:2751-2790`, `index.rs:2561-2562` |
| Query cost | one extra indexed name lookup per candidate query; the *fixed* per-invocation cost (≈ 472 ms: git fingerprint ≈ 272 ms + DB open ≈ 192 ms) dominates the graph query (≈ 287 ms) | `docs/m9-cp3-evidence.md:592-617` — the marginal cost of one more lookup is expected to be small, **not measured** |
| Extraction cost | the per-file walk gains three node arms and one macro-name pre-pass; the cost is **not measured** | the full MMSP index was ≈ 210–219 s on the M3/M4 host (design gate §2.5); any delta is unmeasured until the implementation phase measures it |
| Backward compatibility | existing pilot indexes become `schema_outdated` → the MCP read path classifies them and the index must be rebuilt; the CP2/CP3 evidence stays valid **as historical evidence for schema v4** | `store.rs:145-149`, `mcp.rs:544-553` |
| Counters | the published fact counters (definitions/imports/calls) are in-memory (`index.rs:26-51`, `main.rs:227-256`); a new counter is **a document delta** in the fixed-key `index` JSON, not a free addition | the M2 numbers (170 586 definitions, 64 954 imports, 562 384 call candidates) stay valid because they describe schema v4 |

---

## 3. Option evaluation

### Option A — syntax-level reference candidates

*Add the minimum extraction for field declarations and syntactically recognisable
field uses; state every ambiguous relation as a candidate, never as a verified
semantic reference.*

| Aspect | Assessment |
|---|---|
| Capability actually gained | the **declaration** becomes a searchable symbol; a **name-scoped candidate set** of occurrences (member accesses, address-of bindings, macro arguments) becomes available; `callers`/`impact` at the field anchor can list the affected locations with the verdict `UNKNOWN` |
| What stays unsolved | the binding of an occurrence to a declaration; overloads; macro expansion (the macro body stays opaque); template instantiation; the four different `PrintOption` definitions; `.cu` files (unsupported by policy); `.inl` headers (not indexed — CP3 §9: 40 of the pilot's 47 unmatched includes are `.inl`) |
| Components to change | 1 extraction module (`syntax.rs`), 1 storage module (`store.rs`: new table + version + 2 lists), 1 index module (`index.rs`: ingest + counters + parity test), 1 graph module (`edges.rs`: one new edge kind + reasons), 1 query path (`store.rs`: the name join + an index), 1 response module at most (`response.rs`: a role field), 0 new CLI commands, 0 new MCP tools, plus tests and goldens |
| False positives | same-name fields on other types; a macro argument that is a local variable shadowing the field; a plain identifier that merely shares the name; a `field_expression` whose `field` is a `qualified_identifier` (a static member, e.g. `Type::member`) |
| False negatives | a use whose name differs (`has_max_triangulation_angle`, `max_triangulation_angle_rad` — CP1 §B.4 names exactly these at `patch_match.cc:361-366`); a use in a `.cu` or `.inl` file; a use in a file that does not textually include the anchor's header (see §4.3) |
| Incremental / full parity risk | the two silent-failure lists (§2.7); the parity test must dump the new table |
| Performance impact | extraction cost **not measured**; query cost expected small relative to the ≈ 472 ms fixed cost, **not measured** |

### Option B — enrich the existing index or query without new storage

| Aspect | Assessment |
|---|---|
| Capability actually gained | **none that the evidence supports** |
| Why | the facts do not exist in any stored table (§2.3): `symbols` holds definitions, `calls` holds call candidates, `imports` holds include directives; there is no identifier/occurrence table and no index on `calls.name`. Any "enrichment" can only re-read what was never stored. The one B-variant that does not need storage — resolving candidates by **re-parsing the tree at query time** — violates the harness's own rule that *"Query code never reparses source"* (`edges.rs:89`): the read path may *read* source bytes (it does, for the syntax fallback at `index.rs:1433-1441`), but it must not *parse* them. The cost of a per-query parse is **not measured**; comparing it with the ≈ 210 s full index is not a per-query measurement and is not used as an argument here |
| Components to change | none that would work |
| False positives / negatives | — |
| Incremental / full parity risk | — |
| Performance impact | — |

**Verdict: rejected on evidence, not on complexity.** The information is lost
in extraction, so it must be re-captured there; no amount of query or mapping
enrichment can recreate it.

A B-*variant* is viable and is **folded into Option A**: the **declaration**
side needs **no schema change at all**, because `symbols.kind` is a free-text
column (`store.rs:174-181`) and the FTS index already covers `symbols.name`
(`store.rs:218-226`). A `field` row therefore makes `search` find the
declaration with no DDL and no migration — **but it does need a full re-index**,
because the extraction change bumps `PARSER_VERSION` (§2.7), and an index
built before the bump reuses the old facts and keeps returning 0 hits.

### Option C — a C++ semantic provider (boundary comparison only)

| Aspect | Assessment |
|---|---|
| Capability actually gained | the **binding** — which occurrence refers to which declaration — plus overloads, macro expansion and cross-TU identity: everything Option A cannot do |
| What stays unsolved | the compilation database itself: MMSP has none (design gate §2.3: no `compile_commands.json`, no `CMAKE_EXPORT_COMPILE_COMMANDS`, no `.clangd`), and clangd needs one, which CMake only exports with Makefile/Ninja generators |
| Components to change | a provider, its trust boundary, a compilation-database generation step, a snapshot procedure, a new evidence class |
| Status | **deferred by owner decision D-M9-2** ("DEFER clangd: baseline syntax/graph correctness first; a later proven need requires a separate evidence-backed proposal + OWNER GO") and explicitly out of scope for this task (§9) |
| Consequence | every requirement whose truth depends on the **binding** stays **BLOCKED** under Option A. This is recorded, not worked around |

### 3.1 What only Option C can satisfy

- "this occurrence is *this* declaration" (the D.1-C6 requirement: telling
  `patch_match.cc:667` from `patch_match.cc:668` and `fusion.cc:144` apart,
  `docs/m9-cp1-pilot-spec.md:729-737`);
- distinguishing a member from a same-named global or a shadowing local;
- resolving a macro argument through the macro body (E3);
- any claim of a *verified* C++ reference.

**These are recorded as blocked requirements. No heuristic is allowed to make
them PASS.**

---

## 4. Recommended minimum approach

**Recommendation: Option A, in two slices, with Option B's declaration slice
folded in.** The recommendation is driven by the evidence: the facts are lost
in extraction and the grammar already contains the nodes, so the smallest
change that produces *any* honest reference capability is a small, local
extraction addition plus one new fact table.

### 4.1 Slice 1 — declarations (no schema change)

1. Extract C++ `field_declaration` nodes into a `symbols` row with
   `kind = "field"` and `name` = the field identifier
   (`_field_declarator` → `field_identifier`).
2. **Exclusion (load-bearing).** In the grammar, an in-class *method
   declaration* is also a `field_declaration` whose declarator is a
   `function_declarator` (§1.3). Only declarators that resolve to a
   `field_identifier` become a `field` symbol; a `function_declarator`
   declarator is **not** a data member and must be skipped, otherwise every
   in-class method declaration (e.g. `PatchMatchOptions::Print`) would become
   a bogus field.
3. **Multiple declarators.** `field_declaration.declarator` is *multiple*
   (`int a, b;`): each declarator that resolves to a `field_identifier`
   yields its own symbol row.
4. The enclosing class is **not** stored in slice 1 (no column exists, and
   adding one is a schema change). Consequence, stated openly: two fields
   with the same name on different types are **not** distinguishable in the
   index; the *candidate set is name-scoped* and the number of same-name
   declarations is reported, never merged.
5. **Anchor-identity consequence (a regression, not a side effect).** The
   syntax fallback of `resolve_edge_anchor` picks the narrowest enclosing
   `symbols` row by byte containment, with no kind filter
   (`index.rs:1444-1474`). A new `field` row therefore becomes the anchor for
   any position inside a field declaration, where the anchor used to be the
   enclosing `struct`/`class`. That changes `callers`/`callees`/`impact`
   output at those positions, not only the search results. The implementation
   must either exclude `field` symbols from that fallback or accept the
   changed anchors deliberately and update the affected goldens; both are
   listed in the regression matrix (§7, C1).
6. Effect: **after a full re-index** (the `PARSER_VERSION` bump forces one,
   §2.7), `search max_triangulation_angle` starts returning the declaration
   (it returns 0 hits today, §2.1). The result's `kind` becomes `"field"` — a
   new value in an existing field, i.e. a documented contract delta
   (OWNER decision §10.5).

### 4.2 Slice 2 — occurrences (schema v5)

New fact table `reference_occurrences` (not `references`: that is an SQLite
keyword), mirroring `calls` (snapshot, file, name, role, span, byte offsets,
provider, grammar, language). Roles, all syntactic:

| Role | Source node | Example (CP1 §B.4) |
|---|---|---|
| `member-access` | `field_expression` with a `field_identifier` and the operator `.` or `->` | `options_.max_triangulation_angle` in `patch_match_options.cc` |
| `static-member-access` | `field_expression` whose `field` is a `qualified_identifier` (`Type::member`) | a static member use; kept as a **separate role** so it is never merged with an instance access |
| `address-of-binding` | `pointer_expression` (`&`) whose operand is a `field_expression` | `&patch_match_stereo->max_triangulation_angle` at `option_manager.cc:1018-1019` |
| `macro-argument` | an `identifier` argument of a `call_expression` whose callee is a **macro defined in the same translation unit** (`preproc_function_def`) | `PrintOption(max_triangulation_angle)` at `patch_match_options.cc:58`; `CHECK_OPTION_LE(min_triangulation_angle, max_triangulation_angle)` at `:91-93` — the second one only if that macro is defined in the same TU (unknown; §9.2) |

Deliberate exclusions (and why):

- **Plain identifiers are not occurrences.** A bare `X` in an expression is
  ambiguous between a local, a global and a member; recording it would drown
  the candidate set in noise and would silently merge same-named symbols.
- **Macro bodies stay opaque.** The grammar makes a macro body one token; the
  *expansion* of `PrintOption(x)` is invisible, so a macro-argument occurrence
  is evidence about the **argument**, never about what the macro does with it.
- **A macro invocation stays an unresolved call candidate.** It is not a
  function call, and the four different `PrintOption` definitions are never
  merged. The macro's *own* definition site is not added to the reference set.
- **`.cu`/`.cuh` stay unsupported** (`syntax.rs:77`, `scan.rs:109`), and
  **`.inl` stays out of scope** (CP3 §9) — recorded as a coverage limit, not
  silently ignored.

### 4.3 Derivation and query

1. A new edge kind `POTENTIAL_REFERENCE` is produced by the syntax derivation
   (`edges.rs:167-269` is the place), always with `precision: "Heuristic"`,
   `provider: "syntax"`, `callee: null`, and an `unresolved` reason from a
   **new, small** vocabulary:
   - `unbound-member` — the occurrence's name matches at least one
     declaration, but no binding is proven;
   - `ambiguous:{n}` — `n` declarations share that name (reusing the
     existing `ambiguous:` convention of `edges.rs:219-251`);
   - the existing `no-overlap` — the name matches no declaration at all
     (e.g. a local variable).
2. **No `CALLS` edge is ever produced from a name match.** The existing
   guarantee stands: `CALLS` is constructed only inside the semantic path
   (`edges.rs:448-466`), and no `UPDATE code_edges SET edge_kind` exists in
   production code — the only two such statements are in the test module
   (`edges.rs:1438-1439` starts the tests; `:1515` and `:1525` are inside it).
   Promotion is implemented as delete + insert (`edges.rs:566-572`).
3. **The candidate must carry the name in `callee_name`.** The `callers` name
   branch (`store.rs:1789`) matches on `callee_name = ?9` (the anchor's
   display name); a reference candidate that leaves `callee_name` empty can
   never be found. The edge therefore stores the *occurrence's* name there,
   with `callee_kind = 'unknown'` and the callee's path/byte columns NULL —
   the same shape a syntax call candidate has (`store.rs:1626-1634`).
4. **The `callers` predicate is widened in place**, not extended: the existing
   second disjunct `edge_kind = 'POTENTIAL_CALL'` becomes
   `edge_kind IN ('POTENTIAL_CALL','POTENTIAL_REFERENCE')` (`store.rs:1789`),
   keeping CP3's deliberate anchor-name filter.
5. **`impact` needs one extra change.** `traverse_impact` builds its
   `EdgeSelection` with `candidate_name: None` and `candidate_paths: &[]`
   (`edges.rs:1261-1262`), so the name branch never matches there. To list the
   affected locations through `impact`, the anchor's display name and the
   candidate path set must be threaded into that traversal. Without this,
   `impact` at the field anchor returns no reference paths (the verdict stays
   `UNKNOWN` either way).
6. **The candidate scope is a design choice with a real trade-off:**
   - *include-scoped* (reuse the `callers` machinery): the candidate file set
     is the anchor's file plus the files whose include text resolves to the
     anchor's header (the E8-rooted resolution from CP3). Cheap, reuses the
     existing query, but **misses** a use in a file that does not textually
     include the header.
   - *repository-wide name-scoped*: a use anywhere in the repository with the
     same name. Complete for the name, but the result is a name census, not a
     relation, and it is much noisier.
   Recommendation: **include-scoped** first (it reuses the `callers` contract
   and the E8 work), with the repository-wide variant left as an OWNER choice
   (§10.3).
7. **The scope must be stated in the result.** No existing response field
   carries it (the `callers` document's keys are fixed at `response.rs:208`),
   so the implementation adds an explicit `scope` member — a contract delta
   listed in §6 and §10.5.
8. The **impact verdict logic is not touched**: `impact_verdict` returns
   `UNKNOWN` for any kind that is not `CALLS`/`REFERENCES` with
   `precision == "Exact"` (`edges.rs:1377-1406`), so the affected locations
   are listed while the verdict stays `UNKNOWN` and completeness stays
   `partial`.

### 4.4 What the recommended approach explicitly does **not** do

- It does not produce a verified reference. There is no `CALLS`, no
  `precision: "Exact"`, no `verification: "Current"`.
- It does not bind an occurrence to a declaration. The relation is
  name-scoped, and the report says so.
- It does not change the impact verdict vocabulary, the budgets
  (`edges.rs:755-782`), the exit codes or the MCP tool catalog (twelve tools,
  `mcp.rs:5`).
- It does not add a CLI command or an MCP tool.
- It does not add a generic resolver framework, an include resolver, a
  type system or a "semantic" layer. It adds one fact kind, one table, one
  edge kind and one predicate disjunct.

---

## 5. Invariants and failure modes

### 5.1 Honesty invariants (must hold after any implementation)

| # | Invariant | Where it lives today |
|---|---|---|
| I1 | A verified `CALLS` edge is never produced from a name match | `edges.rs:448-466`; no `UPDATE edge_kind` in production code |
| I2 | `POTENTIAL_CALL` is never promoted; promotion is delete + insert | `edges.rs:566-572` |
| I3 | A macro argument is not a function call; a macro invocation is not resolved to a definition; macro bodies stay opaque | `syntax.rs:411-440` today; the E7 rules in §4.2 |
| I4 | The four `PrintOption` definitions are never merged; the macro used at `patch_match_options.cc:58` is the one defined at `patch_match_options.cc:43` | CP1 §B.4 (`:485-494`) — the design keeps the macro site labeled, not resolved |
| I5 | Same-named fields on different types are never silently merged; the count of same-name declarations is reported | new `ambiguous:{n}` reason, mirroring `edges.rs:219-251` |
| I6 | A reference candidate is distinguishable from a semantic reference | the edge carries `precision: "Heuristic"`, `provider: "syntax"` and an `unresolved` reason, exactly as a `POTENTIAL_CALL` does (`edges.rs:253-266`); `verification` is **not** an edge column — it is derived at query time from freshness (`index.rs:1784-1792`) and reads `"Unverified"` or `"Stale"` |
| I7 | An empty result is never proof of "no use" | `empty-result-not-coverage-proof` (`index.rs:2101`, `:2388`) plus the new `unbound-member` reason |
| I8 | An incomplete impact stays `UNKNOWN`/`partial`; no SAFE/LOW ever appears | `edges.rs:1377-1406`; verdicts are `UNKNOWN`/`HIGH`/`MEDIUM` only |
| I9 | Repository identity, provenance (provider/grammar/language), source anchors (path/line/column/bytes) and the freshness verdict accompany every result | `response.rs:51-262`; freshness `index.rs:2751-2790` |
| I10 | CUDA stays unsupported: no `.cu`/`.cuh` fact, no CUDA coverage claim | `syntax.rs:77`, `scan.rs:109`; CP1 §B.5 |
| I11 | No new edge is created merely to make an acceptance test pass | this document; CP1 §D.0 honesty gate |

### 5.2 Failure modes the implementation must handle

| Failure mode | Required behaviour |
|---|---|
| A field name exists on two types | report both declarations and the candidate set as name-scoped; `ambiguous:{n}`; never pick one |
| The field name is a local variable somewhere | the occurrence is recorded with role `member-access` only when the syntax says so; a bare identifier is not recorded at all |
| The macro is defined in another translation unit | the argument is **not** classified as a macro argument (the macro is not known in this TU) → the occurrence is dropped, not guessed |
| The use is in a `.cu` file | unsupported: no fact, and the coverage report counts it as unsupported |
| The use is in a `.inl` file | not indexed (CP3 §9) → recorded as a coverage limit, not as "no use" |
| The anchor is the field declaration | resolves to the `field` symbol (slice 1) → the `callers` name branch applies; otherwise the syntax fallback reports `syntax-anchor-uncertain` (`index.rs:1486`) |
| The repository has no field with that name | the *declaration* candidate set is empty and each occurrence edge carries `no-overlap` (`edges.rs:250`) — explicitly **not** proof of absence (I7) |
| A stale index | the freshness verdict and its reasons travel with the result (I9) |

---

## 6. Proposed file and component changes (for the implementation phase — not performed here)

| File | Change | Risk |
|---|---|---|
| `crates/cli/src/syntax.rs` | new `Fact` kind(s); `FileFacts.reference_occurrences`; C++ arms for `field_declaration` (excluding `function_declarator` declarators), `field_expression` and `pointer_expression`; a per-file macro-name pre-pass over `preproc_function_def`; **`PARSER_VERSION` 1 → 2** (`store.rs:13-15`) | medium — the walk is shared by all languages; the arms are C++-only |
| `crates/cli/src/store.rs` | new `reference_occurrences` table; `SCHEMA_VERSION` 4 → 5; the table added to `copy_mapped_facts` (`:1227-1269`) **and** to the promotion list (`:1102-1125`); an index on the name column | medium — the two lists fail **silently** when missed |
| `crates/cli/src/index.rs` | `ingest_file` branch (`:383-514`); a counter in `IndexSummary` (`:26-51`) and in the fixed-key `index` JSON (`main.rs:227-256`) — a document delta; the parity test extended to dump the new table (`index.rs:3049-3234`); the anchor-identity decision of §4.1 item 5 | low |
| `crates/cli/src/edges.rs` | `EdgeKind::PotentialReference` + `as_str`; the derivation of the new edges in `derive_syntax_edges`; the new `unresolved` reasons; the `EdgeStats` counters (`:34-40`); **threading the anchor name and candidate paths into `traverse_impact`** (`:1261-1262`) | medium — the graph is shared by `callers`/`callees`/`trace`/`impact` |
| `crates/cli/src/store.rs` (query) | the name-join predicate's `edge_kind` widened in place (`store.rs:1789`); a name lookup for the declarations of a name | medium — the E8 anchor-name filter must be preserved |
| `crates/cli/src/response.rs` | a new evidence key for the occurrence role, and a new `scope` member in the `callers`/`impact` document (`response.rs:208` has no scope field today) | low — a document delta |
| `crates/cli/src/mcp.rs` | **no edit expected** — the existing documents gain the new `kind` value; the tool catalog stays at twelve | low |
| `crates/cli/tests/{syntax,edges,index,mcp}.rs` | the acceptance tests of §7 | low |
| `crates/cli/tests/golden/*` | the M2 goldens gain `field` rows and reference rows where the fixtures contain fields; the anchor-identity change of §4.1 item 5 may change graph goldens | low — deterministic byte stability is a gate (G-CLI) |
| `docs/m9-e7-implementation.md` | the implementation record (phase 2) | — |

**Not touched:** production Rust outside these files; the schema of any other
store; the dependency set; the CLI argument surface; the MCP tool catalog; the
knowledge store; the semantic providers; the LSP/SCIP paths.

---

## 7. Regression and acceptance test matrix

The matrix below is the specification for the **implementation** phase. Each
row states whether the recommended minimum can satisfy it, or whether it needs
E1/E12/C++ semantics.

### A. Positive cases

| # | Case | Expectation | Achievable with the minimum? |
|---|---|---|---|
| A1 | the field declaration is found at the right anchor | a `field` symbol at `colmap/src/colmap/mvs/patch_match_options.h:56:10` with valid line/column/byte offsets, and `search` returns it | **yes** (slice 1) |
| A2 | the CLI binding use is found | an `address-of-binding` occurrence at `colmap/src/colmap/controllers/option_manager.cc:1018-1019` | **yes**, if the field name literally occurs there (to be measured) |
| A3 | the Print and Check uses are identified with the right classification | a `macro-argument` occurrence at `patch_match_options.cc:58` and `:91-93`, with the macro labeled as a macro and **not** resolved to a definition | **yes** for `:58` (the macro is defined in the same file, `patch_match_options.cc:43`, CP1 §B.4); **conditional** for `:91-93` — the definition site of `CHECK_OPTION_LE` is not in the frozen evidence, and the classification requires the macro to be defined in the same TU (§9.2) |
| A4 | the behavioural read is found | an occurrence in `patch_match.cc` — **conditional**: CP1 §B.4 names `has_max_triangulation_angle` and `max_triangulation_angle_rad` at `:361-366`, which are *different* identifiers; the field name occurs there only if the value is copied from the member under that name | **not guaranteed** — to be measured; if the field name does not occur, the site is **not** reachable by any name-based method (E1) |
| A5 | the results point at real source locations | every reported location resolves to the construct in the file (line/column valid, byte offsets exact) | **yes** |
| A6 | the affected locations are listable from the existing surface | `callers` at the field anchor returns the candidate edges; `impact` at the field anchor lists the candidate locations with verdict `UNKNOWN`, `completeness.status: "partial"` | **yes for `callers`** (its name branch matches, provided the candidate stores the name in `callee_name`); **yes for `impact` only if** the anchor name and candidate paths are threaded into `traverse_impact` (`edges.rs:1261-1262`) — that change is in §6; the verdict is `UNKNOWN` in both cases regardless |

### B. Negative and ambiguity cases

| # | Case | Expectation | Achievable with the minimum? |
|---|---|---|---|
| B1 | the same field name on two different types | both declarations reported; the candidate set is name-scoped; `ambiguous:{n}`; **never** merged into one symbol | **yes** (the count is reported; the *distinction* is not) |
| B2 | a macro invocation must not become a verified `CALLS` edge | the macro site stays `POTENTIAL_CALL`/unresolved (or a labeled macro reference); no `CALLS` | **yes** |
| B3 | an ambiguous use must not be labeled certain | every new relation carries `precision: "Heuristic"`, `provider: "syntax"` and an `unresolved` reason; the derived `verification` is `"Unverified"` (or `"Stale"` on a stale index) | **yes** |
| B4 | an unsupported file must not produce fabricated references | no fact from `.cu`/`.cuh`; an unsupported file is reported as unsupported | **yes** (policy unchanged) |
| B5 | no cross-repository mixing | every result carries the repository identity; a candidate set never spans two repositories | **yes** (the index is per repository) |
| B6 | an empty candidate set is not proof of absence | the result carries `empty-result-not-coverage-proof` and the unresolved reasons | **yes** |
| B7 | a `qualified_identifier` field expression (`Type::member`) | kept as the separate `static-member-access` role (§4.2), never merged with an instance access | **yes** — the role distinction is defined in §4.2 |
| B8 | a use in a file that does not include the anchor's header | either outside the candidate scope with the scope stated, or reported — but never silently presented as complete | **yes**, given the scope is stated — and the implementation adds the `scope` member to the result (§4.3 item 7, §6) |

### C. Regression

| # | Case | Expectation |
|---|---|---|
| C1 | `search`, `callers`, `callees`, `trace`, `impact` do not change incorrectly | the existing suites and all goldens stay byte-identical except for the documented new `kind` values and the new rows; the E8 anchor-name filter still holds (CP3's `e8_candidate_scope_keeps_the_anchor_name_filter`) |
| C1b | **anchor identity** at a position inside a field declaration | the syntax fallback picks the narrowest enclosing `symbols` row with no kind filter (`index.rs:1444-1474`), so a new `field` row becomes the anchor where the enclosing `struct`/`class` used to be. The implementation either excludes `field` symbols from that fallback or the changed anchors are documented and the affected goldens updated deliberately |
| C2 | incremental == independent full rebuild | the extended parity test (`index.rs:3049-3234`) compares the new table too; any unexplained difference is a FAIL |
| C3 | deterministic ordering and the CLI/MCP contracts | `EDGE_ORDER` (`store.rs:1740-1741`) already includes `edge_kind`, so a new kind needs no order change; the JSON stays key-ordered; the exit codes are unchanged |
| C4 | performance compared on an equivalent baseline | the same host, the same snapshot, the same queries; the fixed cost and the graph cost are reported separately (as CP3 §8 did); **no speculative claim** |
| C5 | freshness unchanged | the eleven frozen reasons (CP1 §C) behave as before, plus `content-unverifiable` (`index.rs:2786`); a new fact table does not affect the content identity (`store.rs:961`); the `PARSER_VERSION` bump is the one intended change of a verdict (`parser-changed`, `index.rs:2561-2562`) |
| C6 | the CP1 gates are not lowered | CP1 §D is not edited; the §7 matrix states which criteria E7 can and cannot move |

### D. What the matrix says about the frozen CP1 gates

| CP1 criterion | After the recommended minimum |
|---|---|
| D.1-C6 (distinguish `patch_match.cc:667` from `:668` and `fusion.cc:144`) | **still UNMET** — needs the binding (E1) |
| D.2-C6 (the `PatchMatchController` construction sites) | **still UNMET** — E1/E12 |
| D.4 honesty item 4 (**D4-H4**; the frozen CP2 record scores it "D4-H2/H3/H4 PASS" — `docs/m9-cp2-reevaluation.md:367`) | **needs an OWNER interpretation** (§10.2): the new relations are explicitly labeled candidates, and no definition is resolved, but the literal text says "no edge is claimed" |
| D.4-C5 (the affected set includes the anchors) | **partially achievable at best** — it depends on the measured occurrence set (A2–A4). The filter site `patch_match.cc:443-447` uses `max_triangulation_angle_rad`, a *different* identifier, so it is reachable only through the copy at `:361-366`. If the field name does not occur there, D.4-C5 **remains UNMET** |
| D.5-C6 (the agent can carry the whole chain) | inherits D.4-C5 |

**Therefore: the recommended minimum does not, by itself, close E7 as a
release-blocking gap.** It converts "the member is invisible" into "the
member and its name-scoped candidate sites are visible, with the binding still
unproven". Whether that is enough for `v0.1.0` is an owner decision (§10.1).

---

## 8. Scope estimate and the E7 / E1 / E12 boundary

### 8.1 E7 Phase A (this design's proposal, if authorized)

| Item | Count |
|---|---|
| production modules touched | 5 (`syntax.rs`, `store.rs` ×2 areas, `index.rs`, `edges.rs`) + at most 1 response module |
| new database tables | 1 (`reference_occurrences`), i.e. one `SCHEMA_VERSION` bump 4 → 5 and one `PARSER_VERSION` bump 1 → 2 |
| new edge kinds | 1 (`POTENTIAL_REFERENCE`) |
| new CLI commands / MCP tools | **0** |
| new dependencies | **0** |
| new concepts | 1 fact role set (3 roles) and 2 `unresolved` reasons |
| test files | 4 integration suites + the goldens |

This is not a God Class and not a new architecture: it is one fact kind, one
table, one edge kind and one predicate disjunct, all inside the existing
layers.

### 8.2 E1 — C++ semantics (deferred, D-M9-2)

Binding, overload resolution, macro expansion, cross-TU identity, and the
`CALLS`/`REFERENCES` classes that only a provider can produce. **None of it is
touched by E7 Phase A.** E7's output must never be labelled with a semantic
class.

### 8.3 E12 — the most-vexing-parse form

Unaffected by E7: it is a constructor-form problem, not a reference problem
(`docs/m9-cp3-evidence.md:639`).

### 8.4 Also out of E7's scope (recorded, not solved)

- **E3** — macro interiors and templates stay opaque.
- **`.inl` headers** — not indexed (CP3 §9); a use inside one is invisible.
- **E2** — CUDA stays unsupported.
- **E9** — the warm-query fixed cost (≈ 472 ms) is untouched; E7 adds one
  lookup to a command whose cost is dominated by the fixed part.

---

## 9. Risks, unknowns and explicit non-goals

### 9.1 Risks

| # | Risk | Severity | Mitigation |
|---|---|---|---|
| R1 | the new table is missing from the copy or the promotion list → facts are lost **silently** | high | the extended parity test; an explicit review of `store.rs:1102-1125` and `:1227-1269` |
| R2 | false positives: same-name fields on other types, shadowing locals, macro arguments that are not the field | medium | name-scoped reporting, `ambiguous:{n}`, `unbound-member`; the same-name count is always visible |
| R3 | false negatives: a use whose identifier differs (`max_triangulation_angle_rad`), a use in `.cu`/`.inl`, a use in a file outside the include scope | medium | stated as coverage limits; the completeness reasons make the gap visible rather than silent |
| R4 | the search surface changes: names that are fields now return results | medium | documented as a contract delta; OWNER decision (§10.5) |
| R5 | existing pilot indexes become `schema_outdated` | medium | the rebuild is a measured step of the implementation phase; the CP2/CP3 evidence stays valid as historical evidence for schema v4 |
| R6 | the CP1 §D.4 honesty item 4 (D4-H4) literal text is in tension with a candidate relation for the member read | medium | OWNER decision (§10.2) — never an edit of the frozen CP1 text |
| R7 | the extraction cost grows and the full index gets slower | low–medium | **not measured**; the implementation phase must measure the same snapshot with the same host and report the delta, or state "not measured" |
| R8 | the candidate scope (include-scoped) hides legitimate use sites | medium | the scope is stated in the result (the implementation adds a `scope` member, §4.3 item 7); the repository-wide variant is an OWNER choice (§10.3) |
| R9 | the anchor identity at a position inside a field declaration changes (the new `field` symbol becomes the narrowest enclosing symbol) | medium | the explicit decision in §4.1 item 5 and the regression row C1b |
| R10 | a missing `PARSER_VERSION` bump silently keeps the old facts (no `field` rows, 0 hits) | high | the bump is specified in §2.7 and §6 and is asserted by a test |

### 9.2 Unknowns (the implementation phase must measure them)

1. The **actual occurrence set** of `max_triangulation_angle` at the pin —
   specifically whether the field name occurs at
   `patch_match_options.h:56`, `option_manager.cc:1018-1019`,
   `patch_match_options.cc:58` and `:91-93`, `patch_match.cc:361-366` and
   `:443-447`. **This decides A2–A4 and therefore D.4-C5.** It requires a
   disposable isolated snapshot of the pin; the original MMSP clone must stay
   untouched.
2. How many declarations named `max_triangulation_angle` exist in the tree
   (the ambiguity count that the result must report).
3. How many macro-argument occurrences exist per file, and how the
   same-TU macro classification behaves in the pilot (a macro defined in one
   TU and used in another is not classified).
4. The extraction and query cost deltas.

None of these can be answered from the frozen evidence alone; the CP1 text
names *behavioural* sites, and the field's identifier may not occur there.

### 9.3 Explicit non-goals

- No clangd, no compilation database, no `.clangd`, no CMake change.
- No change to production Rust, the schema of any other store, the dependency
  set, the CLI argument surface or the MCP tool catalog — in this task.
- No generic resolver framework, no include resolver, no type checker, no
  "semantic" layer of any kind.
- No `.inl`, no CUDA, no E1/E9/E12 work, no CP4.
- No change to the frozen CP1 ground truth, the CP1 §D acceptance checklist,
  the CP2/CP3 evidence or any historical verdict.
- No claim that a reference candidate is a reference.
- No new trust boundary: the new fact table carries the same provenance
  columns as `calls` (`provider`, `grammar`, `language`) and the same
  file/digest-derived provenance discipline; the implementation adds no
  external process, no network access and no dependency.

---

## 10. Decisions required from the OWNER before implementation

| # | Decision | Options | Recommendation |
|---|---|---|---|
| 1 | **What E7 must deliver for `v0.1.0`**: a name-scoped *candidate* reference graph, or binding-level resolution? | (a) accept a candidate graph and state the coverage limits in the release notes; (b) require binding → E1 (D-M9-2) | **(a)** for `v0.1.0`, with (b) kept as the honest ceiling; the candidate graph is what the evidence supports |
| 2 | **CP1 §D.4 honesty item 4 (D4-H4) tension**: the frozen text says "no edge is claimed for the member read, the address-of binding, or the macro arguments"; a candidate relation does claim one | (a) accept it as consistent with the criterion's intent (no *resolved* edge, verdict `UNKNOWN`) and record an interpretation note in a **new** document; (b) require a separate surface outside `code_edges` | **(a)**, with the interpretation note; the alternative adds a CLI/MCP contract change for no honesty gain |
| 3 | **Candidate scope**: include-scoped (reuse `callers`) or repository-wide name-scoped | (a) include-scoped; (b) repository-wide; (c) both, with the scope stated per result | **(a)** first — it reuses the E8-rooted include work and the existing query; (c) as a later increment |
| 4 | **Schema**: accept a new table (`SCHEMA_VERSION` 4 → 5, one rebuild window, MCP `schema_outdated` in between) or restrict Phase A to slice 1 (declarations only, no schema change) | (a) full A in one step; (b) slice 1 first, slice 2 after the slice-1 evidence | **(b)** — slice 1 is measurable, needs no migration, and after a full re-index it already fixes the measured 0 hits; slice 2 then carries the schema cost with its own evidence. **Either way the extraction change bumps `PARSER_VERSION` and forces a full re-index** |
| 5 | **Contract deltas**: a new `kind` value (`"field"`, `"POTENTIAL_REFERENCE"`), a new occurrence-role key, a new `scope` member in the `callers`/`impact` document, and a new fact counter in the fixed-key `index` JSON | (a) accept the additive deltas; (b) require a new command/tool — which would change the frozen twelve-tool catalog and the CP2 grader's exact-set check | **(a)** — (b) would break the approved twelve-tool contract and the frozen CP2 grader expectation |
| 6 | **Measurement authorization**: a disposable isolated snapshot of the pin to measure the occurrence set and the cost deltas | (a) authorize a disposable snapshot; (b) measure only on the existing pilot workspace | **(a)** — the original clone stays untouched, and §9.2 item 1 cannot be answered without it |
| 7 | **Macro-argument role**: include the `macro-argument` occurrence role (the only way to reach `patch_match_options.cc:58` and `:91-93`) or exclude it | (a) include, with the macro labeled and the body opaque; (b) exclude | **(a)** — CP1 §B.6 #4 explicitly allows "reported as an unresolved macro reference with the macro labeled" |
| 8 | **`.inl` headers**: inside E7 or a separate decision | (a) keep out of E7 and record the coverage limit; (b) include (one-line extension change with an index-content consequence) | **(a)** for E7 Phase A; (b) is a separate decision with its own evidence (CP3 §9) |
| 9 | **The `search` surface**: a name that is a field now returns a `field` result | (a) accept; (b) restrict field results to a dedicated query | **(a)** — it is the fix for the measured 0 hits and it keeps the tool catalog at twelve |
| 10 | **Gate reporting**: E7 Phase A will **not** close D.1-C6, D.2-C6, and may not close D.4-C5 | (a) record them as still UNMET with the measured evidence; (b) lower the gate | **(a)** — the gate is not lowered and no heuristic is added to make a test pass |

---

## 11. Verification of this document

- Every technical claim in §2 and §3 cites a `file:line` in this repository
  or a frozen pilot document; the claims were read directly from the source
  during this task, not taken from a summary.
- The grammar claims in §1.3 were read from the pinned crate's
  `src/node-types.json` in the Cargo registry.
- The measured claims (0 hits, `UNKNOWN`/0 paths, the ≈ 472 ms fixed cost,
  the ≈ 210 s full index) are cited to the frozen pilot evidence, with their
  host and run context preserved.
- No acceptance criterion in §7 is derived from a lower gate: every row that
  the recommended minimum cannot satisfy says so, and the blocked
  requirements are named against E1/E12.

### 11.1 Independent review of this specification

This document was audited by an independent read-only reviewer, which read it
against the source and the frozen pilot documents. Its findings were checked
one by one against the code and **all valid ones were corrected** before
commit:

| Severity | Finding | Disposition |
|---|---|---|
| HIGH | `impact` cannot list the new candidates: `traverse_impact` builds its `EdgeSelection` with `candidate_name: None` and empty candidate paths (`edges.rs:1261-1262`), so the name branch never matches there | **corrected** — §2.6 now separates the two surfaces, §4.3 items 3–5 and §6 add the threading and the `callee_name` requirement, and A6 is no longer an unconditional "yes" |
| HIGH | `PARSER_VERSION` was omitted, so "no rebuild" and "already fixes the 0 hits" were false: an extraction change must bump it (`store.rs:13-15`), otherwise the old facts are reused, and with the bump a full rebuild is forced (`index.rs:2561-2562`) | **corrected** — §2.7 gained the row, §4.1, §6, §7/C5, §8.1, §9/R10 and §10.4 were corrected |
| MEDIUM | the freshness evidence misstated the mechanism (the content identity is a `files` digest; the verdict also reads the snapshot row, the git fingerprint and the parser/grammar/config fingerprints) | **corrected** in §2.7 |
| MEDIUM | `verification` is not an edge column — it is derived at query time and can read `"Stale"` | **corrected** in §2.5, §4.3, §5.1/I6 and §7/B3 |
| MEDIUM | the CP1 §D.4 honesty item is **H4**, not C4 (the frozen CP2 record scores "D4-H2/H3/H4 PASS" and "D4-C5 UNMET") | **corrected** in §7/D and §9/R6 and §10.2 |
| MEDIUM | Option B was rejected on a false "read-only/no-create" rationale; the real rule is *"Query code never reparses source"* (`edges.rs:89`), and the parse cost was unmeasured | **corrected** in §3/Option B |
| MEDIUM | no existing result field carries the candidate scope | **corrected** — §4.3 item 7 and §6 add a `scope` member; §7/B8 and §9/R8 follow |
| MEDIUM | A3 assumed a same-TU definition for `CHECK_OPTION_LE` without evidence | **corrected** — A3 is now conditional, as §9.2 already said |
| MEDIUM | C1 missed the anchor-identity change (a new `field` symbol becomes the narrowest enclosing symbol) | **corrected** — §4.1 item 5 and the new regression row C1b |
| LOW ×10 | citation and wording details (the `syntax-anchor-uncertain` insertion points, `migrate()`'s early return, "widened in place" instead of "one disjunct", `no-overlap` on an occurrence edge, the twelve freshness reasons, the grammar pin in `Cargo.lock`, the module count, the undefined B7 role, `EDGE_ORDER` already covering `edge_kind`, the counter as a document delta) | **corrected** in the cited places |
| suggestion | `references` is an SQLite keyword | **corrected** — the table is named `reference_occurrences` |
| suggestion | an in-class **method declaration** is also a `field_declaration` in the grammar (its declarator is a `function_declarator`), so an unqualified extraction would create bogus field symbols | **corrected** — §1.3 and §4.1 item 2 now specify the exclusion |
| suggestion | multiple declarators (`int a, b;`) | **corrected** — §4.1 item 3 |
| suggestion | security: the new table must stay inside the existing validation that `calls` uses | **corrected** — §9.3 |

**This task changed no runtime behaviour**: no production file, no schema,
no dependency, no CLI/MCP contract, no MMSP file, and no frozen evidence was
modified. The only artefact is this document.
