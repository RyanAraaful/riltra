# M9 E7 Slice 2 — minimum implementation specification (frozen)

**Status: specification only. Implementation is NOT authorized by this document.**
No production Rust, schema, database, version constant, CLI/MCP contract, graph
edge or frozen acceptance record is changed by this task. The only artefact is
this file.

- Branch: `m9-multirepo-release-candidate`
- Baseline HEAD (= remote): `a5b3be032ad53a5cff25d7ebe22bd6b2c222e1f2`
  (local == remote, ancestry of `4ea3398` confirmed, tree clean apart from the
  untracked owner-local `AGENTS.md`)
- Design direction fixed by the OWNER: **Alternative 2 — separate reference
  occurrence facts**, never graph edges
- Design of record: `docs/m9-e7-reference-resolution-design.md` (superseded
  where §1.4 says so)
- Measurement: `docs/m9-e7-slice2-evidence-and-decision.md` +
  `docs/pilots/e7-slice2-evidence/` (not re-measured here)
- Acceptance source: `docs/m9-cp1-pilot-spec.md` §B.4 and §D.4;
  `docs/m9-cp2-reevaluation.md` §4.4; `docs/m9-cp3-evidence.md` §9
- Pilot pin: MMSP `d62f06a576d8e76b705c8ecc57fc380f7ebc9c44`,
  tree `c5482ea00481675e0aa921538f560947b68dca12`

What this document freezes: the occurrence taxonomy, the retrieval surface, the
scope and macro policy, the storage shape, the version bumps, the invariants and
the acceptance matrix. What it does **not** decide: the OWNER decisions in §10,
each of which changes a contract the pilot record already froze.

---

## 1. Baseline and evidence

### 1.1 Frozen measurement used (not re-measured)

Nine C/C++ occurrences of `max_triangulation_angle` in the pinned tree, in four
files. The inventory and its AST evidence are
`docs/pilots/e7-slice2-evidence/occurrence-inventory.txt` and
`docs/m9-e7-slice2-evidence-and-decision.md` §2. This spec only *uses* them:

| # | location | role per evidence | emitted by Slice 2? |
|---|---|---|---|
| 1 | `colmap/src/colmap/mvs/patch_match_options.h:56:10` | declaration (`field_identifier`) | no — Slice 1 `field` symbol, already indexed |
| 2 | `colmap/src/colmap/mvs/patch_match_options.cc:58:15` | macro argument, macro defined in the same file | **yes**, `macro-argument`, candidate |
| 3 | `colmap/src/colmap/mvs/patch_match_options.cc:91:7` | plain identifier (implicit member) | **no** — reserved tier, §3.4 |
| 4 | `colmap/src/colmap/mvs/patch_match_options.cc:92:21` | macro argument, macro defined in an included header | **no** — §5.3 |
| 5 | `colmap/src/colmap/mvs/patch_match_options.cc:93:46` | same as #4 | **no** — §5.3 |
| 6 | `colmap/src/colmap/mvs/patch_match.cc:362:16` | member access (`.`) | **yes**, `member-access`, observed |
| 7 | `colmap/src/colmap/mvs/patch_match.cc:365:31` | member access (`.`) | **yes**, `member-access`, observed |
| 8 | `colmap/src/colmap/controllers/option_manager.cc:1018:20` | string literal | never a code reference |
| 9 | `colmap/src/colmap/controllers/option_manager.cc:1019:41` | address-of binding (`&` of `->`) | **yes**, `address-of-binding`, observed |

Result: **4 of the 7 code uses** and **4 of the 9 occurrences** are emitted; #1
stays a declaration, #8 is excluded by rule, #3/#4/#5 are excluded with a
measured reason (§5.3, §3.4). No number here is recomputed by this task.

The frozen CP1 filter site `patch_match.cc:443-447` reads the local
`max_triangulation_angle_rad`; no name-based occurrence lookup reaches it. That
stays true after this slice and is asserted again in P2 (§8.5).

### 1.2 Source anchors this specification relies on

| Area | Anchor (verified against the tree at the baseline HEAD) |
|---|---|
| fact model | `crates/cli/src/syntax.rs:100-127` — `Fact { kind, name, text, span, byte_start, byte_end, callee, tcallee }`, `FileFacts { language, bytes, definitions, imports, calls, errors }`; one constructor at `:713-734` |
| existing C++ facts | `push_field` `syntax.rs:418-436` (`kind: "field"`), dispatch `syntax.rs:648-709`, direct-init call `:518-560` |
| grammar versions | `syntax.rs:45-46` — `"c:0.24.2"`, `"cpp:0.23.4"` |
| schema | `crates/cli/src/store.rs:158-249` (`SCHEMA_SQL`), `SCHEMA_VERSION = 4` `:12`, `PARSER_VERSION = 2` `:16` |
| migration | `store.rs:256-313` — `SCHEMA_SQL` runs first, then per-version steps, then `PRAGMA user_version = 4` `:302` |
| read path | `store.rs:114-151` — `IndexRead::Outdated(v)` for `v < SCHEMA_VERSION` |
| fact writes | `insert_symbol` `store.rs:1451`, `insert_import` `:1478`, `insert_call` `:1498` |
| promotion | `store.rs:1087-1126` — DELETE list `:1104-1111`, positional INSERT list `:1112-1119` |
| incremental copy | `store.rs:1228-1270` — table list `:1234-1238`, `counts = [0u64; 3]` `:1233`, `CopiedTotals` struct `:1158-1163`, constructor `:1264-1269` |
| symbol query | `store.rs:1999-2058` (`search_symbols`), `Hit` `:981-995` |
| response | `crates/cli/src/response.rs:38-49` (`SearchResponse`), `:51-72` (`search_hit_json`), `:73-93` (`search_response_json`, one `format!`) |
| CLI | `crates/cli/src/args.rs:296-324` (`search` parsing), `crates/cli/src/main.rs:302-425` (`cmd_search`, three response constructors at `:349`, `:390`, `:414`), `main.rs:564` (dispatch), `main.rs:48` (usage) |
| MCP | `crates/cli/src/mcp.rs:36-48` (`CodeSearchInput`, `deny_unknown_fields`), `:251-266` (tool), `:569-673` (`code_search_doc`), `:677-701` (`search_doc`), `:1372-1385` (twelve-tool catalogue) |
| staleness / rebuild | `crates/cli/src/index.rs:2530` (`build`), `:2571-2572` (parser changed → `fallback_full(..., "parser-changed")`), `:2785-2786` (freshness reason `parser-changed`), `:3742` (`parser_version_mismatch_is_stale`) |
| parity test | `index.rs:3059-3125+` (`incremental_matches_independent_full_rebuild`) |
| contract tests | `crates/cli/tests/mcp.rs:184-212` (exact twelve-tool set), `:284-300` (`additionalProperties:false` per tool), `crates/cli/tests/index.rs:470-485` (search golden spans), `:641` (`cli_search_parity_full_vs_incremental`) |

### 1.3 Library contracts verified with Context7 (not from memory)

| Contract | Verified method |
|---|---|
| tree-sitter Rust `Node` API | `child_by_field_name`, `byte_range`/`start_byte`/`end_byte`, `start_position`/`end_position`, `parent()` — the extraction rules below use only these |
| rmcp `#[tool]` input schema | the derive uses `Parameters<T>`'s `JsonSchema` when no `input_schema` is supplied, so adding an optional field changes the generated `inputSchema` (declared in §4.5) |
| rusqlite DDL / queries | `execute_batch` for `SCHEMA_SQL`, `prepare` + `query_map` + `params!` for the queries in §6.6 |

### 1.4 Deviations from the design of record (explicit, traced)

1. **§4.3 of the design is superseded.** `POTENTIAL_REFERENCE`, the widened
   `callers` predicate, the `impact` threading and the `unresolved` reason
   vocabulary are **not** built. Alternative 2 stores facts and adds no edge.
   `crates/cli/src/edges.rs` is untouched by this slice.
2. **`static-member-access` is dropped.** The design (§4.2) has four roles and
   this is one of them; the OWNER's frozen taxonomy replaces it with
   `implicit-member-access` (an evidence addition, not a design role), and no
   measured occurrence is a `Type::member` access. The `field_expression` rule
   below requires a `field_identifier`, so a `qualified_identifier` field is
   never recorded. This is a recorded coverage limit (§9 L4), not an implicit
   merge. The design's operator list (`.`/`->`) is widened to **any**
   `field_expression` with a `field_identifier`, matching the evidence probe
   (`docs/m9-e7-slice2-evidence-and-decision.md:70`, which includes `.*`).
3. **The same-translation-unit macro rule is not implemented.** Its per-file
   reading is what Slice 2 freezes (§5.3). The design's §4.2 conditional
   ("the second one only if that macro is defined in the same TU") is resolved
   here as: *not emitted*, with the measured cost (#4/#5) stated.
4. **Confidence is derived from the role, not stored.** The design does not
   specify a column; the OWNER requires observed/candidate to be separated.
   §3.3 gives the frozen role → confidence map; no dead column is added for a
   tier that is not emitted (§3.4).
5. **The scope is repo-wide, not the design's recommendation.** The design
   recommends include-scoped/direct and leaves repository-wide as an OWNER
   choice (§4.3 item 6, `:398-409`); the evidence refines the requirement to
   "at least transitive" (`docs/m9-e7-slice2-evidence-and-decision.md:226-228`,
   `:236`, `:277`). This spec takes the design's OWNER choice and rejects
   direct-only as measured lossy; the transitive chain is not adopted because
   it needs an include resolver this task is not authorized to add (§5.1).

### 1.5 Method limits of this document

No MMSP build, no index run, no re-measurement. The occurrence volume of a
member-access census, the index-size delta and the wall-time delta are
**unmeasured** (§9 L9) and are not claimed.

---

## 2. Chosen minimum architecture

### 2.1 Shape

```
C++ source ──parse_source──> FileFacts.reference_occurrences (Vec<Fact>)
                                      │
                              staging DB insert (same transaction, per file)
                                      │
                              reference_occurrences (snapshot-scoped)
                                      │
        riltra search --symbol NAME --occurrences   /   riltra_code_search {query, occurrences:true}
                                      │
                     existing search document + optional "occurrences" object
```

One fact collection, one table, one query, one optional response member. No
edge, no graph traversal, no new tool, no new command, no new dependency.

### 2.2 Retrieval surface — the three options against the required dimensions

Rows are the dimensions the OWNER required; columns are the options.

| Dimension | **A — occurrence section in the existing `search` response** | B — CLI-only occurrence query | C — new MCP tool |
|---|---|---|---|
| entry point in the source | `main.rs:302-425` `cmd_search` and `mcp.rs:569-673` `code_search_doc`; both already funnel into `response.rs:73` `search_response_json` | a new `Command` arm in `args.rs`/`main.rs` and a new usage line | a new `#[tool]` in the existing `#[tool_router]` block (`mcp.rs:214`) |
| response-contract change | one **optional** member, present only when requested; the existing keys, order and bytes are unchanged when it is absent | a new command → a new document (same shape) and a new usage/exit-code surface | a new tool → new document, new annotations, new input schema |
| how OpenCode reaches it | one extra property on `riltra_code_search`; the agent needs no shell | shelling out from the agent; the MCP path cannot see occurrences | a new tool call |
| how a field-name lookup happens | `--symbol <field> --occurrences`; the request already carries the name, but the occurrence lookup applies **exact equality** while the symbol list keeps its FTS-prefix/LIKE semantics (§4.1) | same, in the new command | same |
| identity / freshness / provenance / ambiguity / pagination | inherited: same `repo`/`root`/`snapshot`/`freshness`/`reasons`; same transaction and `snapshot_id`; provenance copied per row; ambiguity as `field_declarations`; the request's `limit`/`offset` window applies to both lists, each with its own `total`/`truncated` | must re-implement all five (or duplicate `cmd_search`) | must re-implement all five in a second document |
| query cost | 3 bounded queries (one name-indexed) **only when requested**; zero when absent | 3 bounded queries, plus a second code path to maintain | same as A plus a second tool round trip |
| compatibility risk | additive and opt-in; the only shared-surface change is the generated `inputSchema` gaining one optional boolean (§4.5) | no MCP risk, but a new CLI public surface and an agent-access gap | breaks the frozen twelve-tool catalogue and `tests/mcp.rs:184-212`; needs an OWNER unfreeze |

**Recommendation: A.** It is the only option that reuses the existing document,
identity and freshness machinery *and* serves the agent path, while leaving
every existing response byte-identical when the new flag is absent. B is not
justified: A does not change the meaning of any existing key and its cost is
three bounded lookups, two of them name-indexed. C is last resort as the OWNER
stated.

### 2.3 The frozen surface (A)

- CLI: `riltra search --repo <name> --symbol <name> [--limit <n>] [--offset <n>] [--occurrences]`
  (`--occurrences` is a bare boolean flag; usage line `main.rs:48` gains
  `[--occurrences]`; the command exits 0 with the same document as today plus
  the section).
- MCP: `riltra_code_search` gains one optional property `occurrences`
  (`Option<bool>`, the established pattern at `mcp.rs:129-133`, so it stays out
  of the generated `required` array and the current four-field calls keep
  validating under `deny_unknown_fields`).
- No other CLI command and no other MCP tool changes.

### 2.4 What explicitly does not change

`edges.rs` in full (no `POTENTIAL_REFERENCE`, no `EdgeKind` change, no verdict
change), `code_edges` content, `callers`, `callees`, `trace`, `impact`
(including `UNKNOWN`/`partial`), the twelve-tool catalogue, the knowledge store,
the semantic providers, exit codes, the `index` JSON (no new counter, §6.7) and
`SCHEMA_VERSION`/`PARSER_VERSION` **in this task** (they change only in the
future implementation, §6.3).

---

## 3. Occurrence taxonomy and confidence

### 3.1 Frozen roles

| Role | AST evidence (tree-sitter C++, pinned `0.23.4`) | Confidence | Emitted in Slice 2 |
|---|---|---|---|
| `member-access` | a `field_expression` whose `field` node is a `field_identifier`; the recorded name is the field identifier's text. Operator (`.`, `->`, `.*`) is irrelevant: any `field_expression` with a `field_identifier` is a member access | `observed` | yes |
| `address-of-binding` | a `pointer_expression` whose operator is `&` and whose operand is a `field_expression` with a `field_identifier`; the recorded name is that identifier's text. **Precedence:** this role replaces `member-access` for the same identifier — exactly one row per field identifier | `observed` | yes |
| `macro-argument` | a bare `identifier` node inside the `argument_list` of a `call_expression` whose callee is a bare `identifier` that names a `preproc_function_def` **defined in the same file**. Object-like macros (`preproc_def`) do not qualify (they are not callable) | `candidate` | yes |
| `implicit-member-access` | a bare `identifier` inside a member-function body whose class is derivable from a qualified declarator | `candidate` | **no** — reserved, §3.4 |

The role strings above are the only values written to storage. They are also
the `Fact.kind` strings (§6.1).

### 3.2 Rules that keep the set exact

- **One row per identifier occurrence.** The walk records a `field_expression`
  field identifier at most once; the address-of rule takes precedence (§3.1);
  a field identifier inside a call argument is a member access, not a
  macro argument (only *bare identifiers* are macro arguments).
- **The callee is never an argument.** `f(x)` records nothing for `f`; only the
  `argument_list` children are considered.
- **Declarations are not occurrences.** A `field_declaration` identifier is
  Slice 1's `field` symbol. A method name, a parameter name and a local
  declarator are not occurrences.
- **String literals are never occurrences.** A `string_literal` node is not an
  identifier; occurrence #8 is evidence of the CLI option name, not a use.
- **A use textually before its `#define` is still a candidate.** The per-file
  pre-pass is order-insensitive; the confidence label covers it.
- **`#undef`, conditional definitions and conditional includes are not
  modelled.** They cannot make the role *observed*; the `candidate` label is
  the honest ceiling (§5.3, §9 L2).
- **Only C++ files produce rows** (§5.4). No `.cu`/`.cuh`, no `.inl`.

### 3.3 Confidence semantics (frozen; no third value)

| Value | Meaning | Roles |
|---|---|---|
| `observed` | the AST proves the role and the location; nothing is claimed about the declaration an occurrence belongs to | `member-access`, `address-of-binding` |
| `candidate` | the role needs an assumption outside the AST: here, that the callee, which is a `#define` in the same file, is visible and expands at this site | `macro-argument` |

`implicit-member-access`, if ever enabled, would be `candidate` (§3.1); it is
not emitted, so the emitted set above is the complete role → confidence map.

`binding` is a property of the **list**, not of a row: every list this slice
emits is `unbound`. A `verified`/`exact` label never appears; producing one
needs a semantic provider (E1, deferred by D-M9-2). No false-positive rate is
claimed for any role: only the pilot's 9 occurrences were inspected (§9 L9).

### 3.4 Reserved role: `implicit-member-access`

The role is frozen and specified above, and **not enabled**. Reasons, in order
of weight:

1. **No bounded extraction rule exists without a declaration binding.** The
   only per-file rule ("a bare identifier in a member function") records a
   large fraction of every identifier in every method body. The pilot's
   `PatchMatchOptions::Check` is *out of line* in the `.cc` while its fields are
   in the header, so a same-file class-scope filter misses the one measured
   case; the remaining honest rule is pure noise. The volume is not measured:
   the tree records 91,025 call facts
   (`docs/m9-e7-slice1-implementation.md:186`), and the bare-identifier count is
   structurally larger, but no number is claimed here.
2. **Volume is unmeasured**, so neither "safe by default" nor "proven
   restriction" can be asserted. The OWNER's instruction was to prefer "not
   enabled by default until measured".
3. **A dead column is avoided.** No `confidence` column is stored; confidence
   is derived from the role. When this tier is enabled it arrives with its own
   schema step (v6) and its own measurement gate.

Enabling it later needs an OWNER decision (§10 D4) and one of: (a) a measured
repo-wide field-name set plus a post-ingest filter, or (b) a semantic provider.
Neither is authorized here.

---

## 4. Retrieval contract

### 4.1 Request

| Surface | Request | Semantics |
|---|---|---|
| CLI | `search --repo R --symbol NAME --occurrences` | `NAME` is the occurrence lookup key **exactly**; the symbol list keeps its existing FTS-prefix/LIKE semantics (unchanged) |
| MCP | `riltra_code_search { "repo": R, "query": NAME, "occurrences": true }` | same |

Absent flag/property = today's behaviour, byte for byte. `limit`/`offset` keep
their existing defaults (50/0) and maximum (1000) and apply to **each** list
independently: the symbol list and the occurrence list are two page windows of
the same request, each with its own `total` and `truncated`.

Rationale for exact name: a prefix or substring match would need a per-item
`name` and would multiply the census. The symbol list already answers "what is
declared like this"; the occurrence list answers "where does this exact name
appear".

The two lists can therefore disagree on purpose: the symbol list keeps its
prefix/substring semantics, so a query for `max_triangulation_angle` can list
the longer symbol `max_triangulation_angle_rad`, while the occurrence list
contains only rows whose name is exactly `max_triangulation_angle`. `scope`
states which rule produced the items; the symbol list is unchanged.

### 4.2 Response (appended only when requested)

The section is appended after `"truncated"`; key order inside it is fixed.

```json
"occurrences":{
  "scope":"repository-wide-exact-name",
  "binding":"unbound",
  "field_declarations":1,
  "symbol_declarations":1,
  "total":4,
  "truncated":true,
  "items":[
    {"path":"colmap/src/colmap/controllers/option_manager.cc","role":"address-of-binding","confidence":"observed",
     "span":{"start_line":1019,"start_col":41,"end_line":1019,"end_col":64},"bytes":[0,0],
     "provenance":{"provider":"syntax","grammar":"cpp:0.23.4","language":"cpp"}},
    {"path":"colmap/src/colmap/mvs/patch_match.cc","role":"member-access","confidence":"observed",
     "span":{"start_line":362,"start_col":16,"end_line":362,"end_col":40},"bytes":[0,0],
     "provenance":{"provider":"syntax","grammar":"cpp:0.23.4","language":"cpp"}}
  ]
}
```

(The `span` end columns and the `[0,0]` byte offsets are illustrative; rows
carry the real values from `Span`. The pilot example's real ordering is
`option_manager.cc`, `patch_match.cc:362`, `patch_match.cc:365`,
`patch_match_options.cc:58`. The excerpt shows two of the four items, so
`truncated` is `true` by the rule below; a page that returns all four has
`truncated:false`.)

Contract facts:

- `scope` is the literal `repository-wide-exact-name`; `binding` is the literal
  `unbound`. Both are always present when the section is present, including
  when `total` is 0.
- `field_declarations` = number of `symbols` rows with `kind='field'` and the
  same exact name; `symbol_declarations` = all `symbols` rows with that name.
  These are the ambiguity counts: 27 field declarations for `value` in the
  pilot tree is what the consumer must see.
- `items` are ordered by `(path, byte_start, role)` — deterministic, matching
  the symbol list's path-then-byte ordering.
- `total` = occurrence rows for the exact name in the snapshot (all roles);
  `truncated` = `offset + items.len() < total`.
- No `name` member per item: the name is the request. No `snapshot`, `repo`,
  `freshness` or `reasons` inside the section: they are the response's and are
  guaranteed to describe the same revision.
- `confidence` is per item; a list may mix observed and candidate.
- `provenance` mirrors `search_hit_json` (`provider`, `grammar`, `language`),
  copied from the row.

### 4.3 Identity, freshness, provenance, ambiguity, pagination

| Property | How it is preserved |
|---|---|
| repository identity | `search` already resolves the repo, root and snapshot; the occurrence queries take the **same** `snapshot.id` inside the **same** read transaction as `search_symbols` (`main.rs:405-413`, `mcp.rs:651-658`) — one revision per response, as `search_response_carries_one_revision_identity` (`tests/index.rs:137`) already requires |
| freshness | unchanged: `evaluate_snapshot` fills `freshness`/`reasons` before either query; a stale snapshot makes the occurrence list stale in the same document (reason `parser-changed` after the version bump) |
| provenance | per row from the extractor: `provider="syntax"`, `grammar="cpp:0.23.4"`, `language="cpp"` (`syntax.rs:45-46`, mirroring `index.rs:445-446`) |
| ambiguity | `field_declarations`/`symbol_declarations` + `binding:"unbound"` + `scope:"repository-wide-exact-name"`. The response never selects one declaration |
| pagination | the response's `limit`/`offset` apply to both lists; each list has `total` and `truncated`; ordering is total and deterministic |
| empty result | the section is still emitted with `total:0`, `truncated:false`, `scope` and `binding`; the response carries the freshness reasons. Nothing in the document asserts "no uses" |
| no index | unchanged `no-index` document, `freshness:"unknown"`, **no** `occurrences` member (the flag cannot manufacture facts). The `root-unreadable` document behaves the same way: it already skips the symbol query and returns `total:0`, so it skips the occurrence query too — no facts are reported when freshness cannot be evaluated |

### 4.4 Contract deltas (each needs OWNER approval, §10 D1)

| # | Delta | Compatibility |
|---|---|---|
| Δ1 | `search`/`riltra_code_search` may carry an additional `occurrences` object | additive; absent unless requested; existing goldens and parity tests unaffected |
| Δ2 | `riltra_code_search`'s generated `inputSchema` gains one optional boolean `occurrences` | the tool **set** is unchanged (twelve); an old client's four-field call still validates; a new client calling an old server would be rejected by `deny_unknown_fields` (single-binary deployment, out of scope) |
| Δ3 | the CLI usage line gains `[--occurrences]` | documentation |
| Δ4 | `SCHEMA_VERSION` 4→5, `PARSER_VERSION` 2→3 | one forced full rebuild per repository (§6.3, §7) |
| Δ5 | `riltra_health.versions.code_index_schema` reads `5` instead of `4` (`mcp.rs:1352`, asserted at `tests/mcp.rs:502`) | an already-exposed version value changes; no tool or key is added. It forces the test sweep below |

`riltra_code_search`'s description text does not change; the new property's doc
comment becomes its schema description.

The version bump is not test-neutral. Verified literals that must move with it:
`tests/mcp.rs:502` (health `code_index_schema`), `:1555`, `:1632` (`newer than
supported v4`), `:1832`, `:2592`, `:2601`, `:2898`, `:2994` (`user_version ==
4`), `:1756-1761` (the 1↔4 flipper), `:2649`/`:2658` (the schema-ahead fixture
sets 5 and expects `schema_ahead` → becomes 6); `store.rs:2971`, `:3005`,
`:3029` (`must migrate to v4`). All are test-only edits inside S2/S3; none
relaxes an assertion, and the full suite must pass before S4.

### 4.5 Query cost

With the flag absent: **zero** new queries. With it present: three bounded
queries per response (declaration count, occurrence count, occurrence page);
only the two occurrence queries are name-indexed, all are `snapshot_id`-scoped
and the page is bounded by the `limit`/`offset` window. An empty
name yields an occurrence section with `total:0` **without touching the table**,
mirroring `search_symbols`'s empty-term early return (`store.rs:2006-2008`).

---

## 5. Scope and macro policy

### 5.1 Scope: repository-wide exact name (recommended) vs include scope

| Scope | Occurrences inside the scope (evidence) | Code uses emitted under this spec's per-file policy | Cost / honesty |
|---|---|---|---|
| repository-wide exact name | 9 of 9 (`docs/m9-e7-slice2-evidence-and-decision.md:228`) | 4 of 7 (`:234`, per-file pre-pass column); the stated-rule ceiling is 6 of 7 (`:234`) | general cost: same-named members of other types. Must be labelled a **census**; `field_declarations` exposes the ambiguity (27 for `value`) |
| include-scoped, direct includes only (what the E8 rooted resolution does today) | 7 of 9 (`:226`) | 2 of 7 (`:235`, per-file) | **must never be claimed complete**: loses #6/#7, the behavioural read in `patch_match.cc`, which reaches the header only through `patch_match.h:35` |
| include-scoped, transitive chain | 9 of 9 (`:227`) | 4 of 7 (`:236`, per-file) | reaches the right files, but needs an include-graph resolver; **not authorized** by this task; the chain is textual, not the compiler's |

**Decision: repository-wide exact name.** It is one indexed equality lookup
(no resolver, no graph, no new pass) and it loses no location the measurement
can see: 9 of 9 are inside the scope (`:228`). The 3 unemitted code uses are
excluded by role policy, not by scope — #4/#5 by §5.3, #3 by §3.4. The honesty
price is fixed in-band: `scope`, `binding`, `field_declarations`. The
direct-include variant is rejected because the frozen acceptance values the
behavioural read (`patch_match.cc:361-366`) and because "complete" could not be
claimed for it (7 of 9 inside the scope, 2 of 7 emitted). The transitive variant
is rejected as out of authorization, not as wrong. The ceiling if D3 is
approved is 6 of 7 (`:234`, stated-rule column); 7 of 7 additionally needs D4.

### 5.2 Naming discipline

The response must never be described as "references of this declaration".
`scope:"repository-wide-exact-name"` + `binding:"unbound"` +
`field_declarations` are the machine-readable form of "this is a name census,
not a binding". `field_declarations:0` means "no field of this name is indexed",
not "no uses". An empty list is not proof (I7).

### 5.3 Macro policy (frozen minimum)

| Case | Slice 2 behaviour |
|---|---|
| `PrintOption(...)` in `patch_match_options.cc` — `#define` in the same file (`:43`) | emits `macro-argument`, `candidate` (occurrence #2) |
| `CHECK_OPTION_LE(...)` — `#define` in `logging.h:61`, included at `:32` | **not emitted** (#4, #5). Proving visibility needs the include graph at extraction time or a repo-wide macro set; neither exists, and adding either is beyond this slice. Documented, not denied |
| a macro from another translation unit | not emitted (unchanged design rule) |
| `#undef` / conditional `#define` / conditional include | not modelled; the `candidate` label is the ceiling |
| macro bodies / expansion | opaque; no expansion engine, no preprocessor, no compiler |
| the macro's own definition site | never an occurrence and never merged with the call site |

**Measured cost of this policy: #4 and #5 (2 of 7 code uses).** The upgrade
path, if the OWNER later authorizes it, is a `macro_definitions` fact plus
direct-include visibility, emitted as `macro-argument`/`candidate` — one more
table and one more copy/promote/parity entry, not a new architecture. It is
**not** part of this specification (§10 D3).

### 5.4 Language and file scope

- **C++ only.** `Language::Cpp` files produce occurrence rows; the C arm is
  untouched. C member accesses are not indexed here (L5). `.h`/`.hh`/`.hpp`/
  `.hxx` are parsed as C++ (`syntax.rs:75`), so a header contributes like any
  other C++ file. Slice 1's field symbols are C++-only for the same reason.
- `.cu`/`.cuh` remain unsupported (`syntax.rs:77`); `.inl` is not indexed
  (CP3 §9). Both are coverage limits, never "no use".
- No `.ipp`/`.tpp` extension is added; that is an E7+ scope question.

---

## 6. Minimal storage and schema (design only — not implemented here)

### 6.1 Extraction facts: reuse `Fact`, add one vector

`Fact.kind` already carries fact sub-kinds (`"definition"`, `"import"`,
`"call"`, `"field"`), so the occurrence role is stored in `kind` — no new
struct:

```rust
// FileFacts
pub reference_occurrences: Vec<Fact>,   // kind = role; name = identifier text;
                                        // text/callee/tcallee = None; span + bytes set
```

One constructor (`parse_source`) and one new recursive parameter (the per-file
macro-name set for C++, empty otherwise) are the whole extraction surface. The
C++ arms are `field_expression`, `pointer_expression` (address-of precedence)
and `call_expression` (bare identifier arguments whose callee is in the
same-file `preproc_function_def` set).

### 6.2 DDL (proposed; schema v5)

```sql
CREATE TABLE IF NOT EXISTS reference_occurrences (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  snapshot_id INTEGER NOT NULL,
  file_id INTEGER NOT NULL,
  name TEXT NOT NULL,
  role TEXT NOT NULL,
  start_line INTEGER NOT NULL,
  start_col INTEGER NOT NULL,
  end_line INTEGER NOT NULL,
  end_col INTEGER NOT NULL,
  byte_start INTEGER NOT NULL,
  byte_end INTEGER NOT NULL,
  provider TEXT NOT NULL,
  grammar TEXT NOT NULL,
  language TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_ref_occ_name
  ON reference_occurrences(snapshot_id, name);
```

- The shape mirrors `symbols` (`store.rs:175-183`) minus `kind` nullability;
  `role` cannot be NULL because the extractor always sets it.
- One index only. No `file_id` index: no query filters by it, and the copy join
  scans by `snapshot_id` exactly as the other fact tables do.
- No `CHECK` constraint: the only writer is the extractor through
  `Fact.kind: &'static str`; the role set is asserted by tests, not by SQL.
- No FTS: the lookup is exact equality.
- No `confidence` column: derived from the role (§3.3). No `scope`/`binding`
  columns: they are response-level constants. No include/class columns: not
  used by this slice.

### 6.3 Versions and migration

| Change | Exact effect |
|---|---|
| `SCHEMA_VERSION` 4→5 | `SCHEMA_SQL` gains the DDL above, so both fresh databases and v1–v4 upgrades get it; `PRAGMA user_version = 4` at `store.rs:302` becomes `{SCHEMA_VERSION}`; a migration failure still rolls back the version marker (`store.rs:306-312`) |
| `PARSER_VERSION` 2→3 | `index` sees `old.parser_version != 3` and takes `fallback_full(..., "parser-changed")` (`index.rs:2571-2572`); freshness reports `parser-changed` (`index.rs:2785-2786`). Existing facts are re-parsed, so occurrence rows are populated, not reused from parser 2 |
| MCP on a v4 database | `open_read` returns `IndexRead::Outdated(4)` (`store.rs:146-149`) → `schema_outdated` (`mcp.rs:606-607`); the search tool never issues the new query against a v4 schema |
| CLI on a v4 database | `migrate` runs `SCHEMA_SQL` in one transaction → empty occurrence table, version 5; until `index` runs, the snapshot is stale with `parser-changed` and the response says so |

### 6.4 Write path

`insert_reference_occurrence(conn, snapshot_id, &ReferenceOccurrenceRow)` in
`store.rs` mirrors `insert_call` (`store.rs:1498`) and is called from a **new**
loop over `facts.reference_occurrences` immediately **after** the existing
`facts.calls` loop in `ingest_file` (`index.rs:470-502`). It is not folded into
the calls loop: the two collections carry different rows and different insert
statements. Same staging transaction, same `file_id`, no new connection, no new
transaction boundary.

### 6.5 Copy and promotion — the three silent-failure points

| Point | Change |
|---|---|
| `copy_mapped_facts` (`store.rs:1234-1238`) | add `("reference_occurrences", "name, role, start_line, start_col, end_line, end_col, byte_start, byte_end, provider, grammar, language")`; `counts = [0u64; 4]` `:1233`; the `CopiedTotals` struct (`:1158-1163`) gains `occurrences`, set in the constructor (`:1264-1269`), carrying `#[allow(dead_code)]` when the read path has no consumer — the same convention `CarriedTotals` already uses (`store.rs:1274`). No JSON counter is added (§6.7) |
| `promote_staging` DELETE list (`store.rs:1104-1111`) | add `DELETE FROM reference_occurrences;` — otherwise rows from the previous snapshot linger in the live database |
| `promote_staging` INSERT list (`store.rs:1114-1116`) | add `INSERT INTO reference_occurrences SELECT * FROM src.reference_occurrences;` — the positional copy requires the identical column order, guaranteed by the shared `SCHEMA_SQL` |

Both promotion lists are hand-written and fail **silently** (design §2.7); the
parity and promotion tests below are the guard.

### 6.6 Query shape

```sql
-- ambiguity counts (one round trip)
SELECT COUNT(*) AS symbols,
       COALESCE(SUM(CASE WHEN kind = 'field' THEN 1 ELSE 0 END), 0) AS fields
FROM symbols WHERE snapshot_id = ?1 AND name = ?2;

-- total
SELECT COUNT(*) FROM reference_occurrences WHERE snapshot_id = ?1 AND name = ?2;

-- page
SELECT f.path, r.role, r.start_line, r.start_col, r.end_line, r.end_col,
       r.byte_start, r.byte_end, r.provider, r.grammar, r.language
FROM reference_occurrences r JOIN files f ON f.id = r.file_id
WHERE r.snapshot_id = ?1 AND r.name = ?2
ORDER BY f.path, r.byte_start, r.role
LIMIT ?3 OFFSET ?4;
```

`ORDER BY` is total (a path, a byte offset and a role are unique per row for a
given name at a given offset; if two roles could ever coincide, the `id`
tiebreaker is available and must be added rather than accepted as
nondeterministic).

### 6.7 Deliberately not added

No confidence column, no macro table, no include table, no FTS index, no
`POTENTIAL_REFERENCE`, no `EdgeKind`, no migration framework, no generic
resolver, no new `IndexSummary` counter and no new key in the `index` JSON
(`main.rs:227-256` stays byte-identical). No new dependency.

---

## 7. Incremental and freshness invariants

| # | Invariant | Where enforced | Proof |
|---|---|---|---|
| I1 | a full build and an incremental build of the same tree produce identical occurrence rows | `copy_mapped_facts` copies only digest-verified files; `reparsed` files are re-extracted | parity test extended with occurrences (§8.3 S1) |
| I2 | promotion never loses occurrences | DELETE + positional INSERT in `promote_staging` | promotion test (§8.3 S2) |
| I3 | promotion never leaves rows from a previous snapshot | the DELETE list includes the table | S2 asserts `SELECT COUNT(*)` and distinct `snapshot_id` |
| I4 | an extraction change is never silently served from old facts | `PARSER_VERSION` 2→3 → `parser-changed` full rebuild | existing `parser_version_mismatch_is_stale` + new assertion that a v2 snapshot yields 0 rows before rebuild and 4 after |
| I5 | an old schema never answers the new query | `IndexRead::Outdated` / `schema_outdated`; CLI migrates before use | existing `migrate_*` tests + new `open_read` v4 → Outdated |
| I6 | repositories never mix | every query filters `snapshot_id`; the copy joins the old snapshot id | `two_repos_stay_isolated_under_incremental` extended with same-named fields |
| I7 | an empty list is not proof of absence | `scope`+`binding`+freshness travel with the list; `total:0` is explicit | §8.2 A6 |
| I8 | the graph rules are untouched | no edit to `edges.rs`, `code_edges`, verdicts or the tool catalogue | §8.4 K1/K2/K3 |

**Existing-database sequence (worked example).** v4 DB, binary v5:

1. MCP `riltra_code_search` → `schema_outdated` (nothing else runs).
2. CLI `search --occurrences` → `store::migrate` (v4→v5, empty table), snapshot
   still has `parser_version=2` → freshness `stale`/`parser-changed`, symbol
   hits present, `occurrences.total=0` and the stale reason in the same
   document.
3. CLI `index` → `parser-changed` → full staged rebuild → promotion → same
   query returns the real rows with `fresh`.

No silent success at any step.

---

## 8. Acceptance and regression matrix

### 8.1 Staged implementation (each stage is independently reviewable)

| Stage | Scope (exact files) | Verification |
|---|---|---|
| S1 extraction | `syntax.rs` only: `FileFacts.reference_occurrences`, the three C++ arms, the same-file macro set; unit tests in the module | `cargo test --locked --bin riltra -- syntax` (the crate has no library target, `Cargo.toml` `[[bin]]`) |
| S2 storage | `store.rs`: DDL, `SCHEMA_VERSION` 5, `PARSER_VERSION` 3, `insert_reference_occurrence`, the three lists; `index.rs`: the insert loop + parity dump; the Δ5 test sweep (§4.4) | `cargo test --locked --bin riltra -- store`, then `-- index`, then the **full** `cargo test --locked` — S2 is the first stage that can break the suite and must leave it green |
| S3 query + surface | `store.rs` query, `response.rs` `SearchResponse`+builder, `args.rs` flag, `main.rs` (3 constructors + usage), `mcp.rs` (field + one doc comment + `search_doc`), tests | `cargo test --locked --test index --test mcp --test cli` |
| S4 pilot | index the pin in an isolated home; assert the 4 rows, the exclusions and the filter site | §8.5 |
| S5 performance + record | measure and write `docs/m9-e7-implementation.md`; no claim beyond the numbers | §8.6 |

Each stage keeps the tree green; S1–S3 must not be squashed into one change.

### 8.2 Correctness

| # | Criterion (OWNER §10) | Test | Expected |
|---|---|---|---|
| A1 | path, line, column and byte offsets correct | new fixture `tests/fixtures/m2/cpp/field-refs.cc` + unit test | each emitted row's `span` and `bytes` address exactly the identifier text in the fixture |
| A2 | role matches the AST | unit test on the same fixture: `a.b`, `p->b`, `&p->b`, `M(b)` | `member-access`, `member-access`, `address-of-binding` (once), `macro-argument` |
| A3 | a string literal is not a code reference | fixture with `"b"` and `M("b")` | no row; the string never appears |
| A4 | same-name declarations are not silently merged | fixture with two classes each declaring `value` | `field_declarations:2`; both occurrences listed; `binding:"unbound"`; no row names a declaration |
| A5 | a candidate is never labelled verified | the macro-argument row | `confidence:"candidate"`; no `verified`/`exact` anywhere in the section |
| A6 | an empty list is not proof of no use | `search --symbol <absent> --occurrences` | section present, `total:0`, `scope`+`binding` present, freshness present, no "no uses" wording |
| A7 | declarations are not occurrences | the pilot's `patch_match_options.h:56` | absent from `items`; present as a `field` search hit |
| A8 | precedence: one row per identifier | `&p->b` | exactly one row (`address-of-binding`), not two |

### 8.3 Storage

| # | Criterion | Test | Expected |
|---|---|---|---|
| S1 | incremental/full fact parity | extend `incremental_matches_independent_full_rebuild` (`index.rs:3059`) with an `OccRow` dump | identical dumps for symbols, imports, calls, **occurrences** |
| S2 | copy and promotion keep occurrences | full build → add a file → incremental build; then a second full build through `promote_staging` | row count and `snapshot_id` set are exactly the new snapshot's; no orphans |
| S3 | parser/schema mismatch is explicit | existing `parser_version_mismatch_is_stale` + new v4 `open_read` test + v2-snapshot rebuild test | `parser-changed`; `Outdated(4)`; 0 rows before, real rows after |
| S4 | repositories never mix | extend `two_repos_stay_isolated_under_incremental` | repo A's occurrences never appear for repo B's snapshot |
| S5 | nothing bypasses validation/isolation | review of the write path | only `insert_reference_occurrence` writes, only inside the staging transaction; no new connection; every read filters `snapshot_id` |

### 8.4 Compatibility

| # | Criterion | Test | Expected |
|---|---|---|---|
| K1 | graph edge kinds unchanged | run `cargo test --locked --test edges`; grep `crates/` for `POTENTIAL_REFERENCE` | zero production hits; all edge tests unchanged |
| K2 | `callers`/`callees`/`trace`/`impact` semantics unchanged | existing suites, untouched; no diff in `edges.rs` | identical output; `impact` still `UNKNOWN`/`partial` |
| K3 | MCP catalogue stays at twelve | `crates/cli/tests/mcp.rs:184-212` unchanged | passes; only `riltra_code_search`'s `inputSchema` gains one optional boolean. Extend `tests/mcp.rs:329-341`: `occurrences` is in `properties`, is **not** in `required`, and `additionalProperties` stays `false` |
| K4 | existing search JSON is byte-identical without the flag | golden/parity tests + a new assertion | no `"occurrences"` substring; `tests/index.rs:470-485` and `:641` unchanged |
| K5 | ordering, pagination, freshness, provenance deterministic | new test: two runs, same page window | identical bytes; `(path, byte_start, role)` order; per-list `total`/`truncated` |

### 8.5 Pilot acceptance (MMSP pin, isolated home, no original clone)

| # | Criterion | Expected |
|---|---|---|
| P1 | the nine frozen occurrences are located or explicitly classified | `items` = #2, #6, #7, #9 only; `total:4`; #1 a search hit; #8 absent; #3/#4/#5 absent with their reasons recorded in the implementation record |
| P2 | the filter site is still unreachable by name | `patch_match.cc:443-447` does not appear for this name; the record says so explicitly |
| P3 | ambiguity is visible | `max_triangulation_angle` → `field_declarations:1`; `value` → `field_declarations:27` (frozen Slice 1 count), never merged |
| P4 | CP1 D4-C5 is **not** flipped | `docs/m9-cp1-pilot-spec.md` and `docs/m9-cp2-reevaluation.md` are unmodified; the implementation record states D4-C5 stays **UNMET**; `impact` still reports `UNKNOWN`/`partial` and gains no occurrence data |
| P5 | D4-H4 stays literally true | no edge is claimed for the member read, the address-of binding or the macro argument (§2.4) |

The pin and tree must be re-verified (`git rev-parse HEAD`, `git rev-parse
HEAD^{tree}`, `git status --porcelain`) before indexing; the original clone is
never read. The isolated registry/home must be the explicit `%TEMP%\opencode\…`
path, not `$HOME`.

### 8.6 Performance (measure, do not claim)

**F1** — measure on the same host, profile, repository state and query set: (a)
index wall time for the full rebuild, (b) database bytes, (c) occurrence row
count, (d) `search --occurrences` latency versus `search` on the same name.
Report the numbers in the implementation record. **No threshold and no
"acceptable" claim is set here**; the OWNER accepts or rejects on the
measurement (D7). The census's size is the known unknown (§9 L9).

### 8.7 Definition of done

All A/S/K/P rows PASS; P4 passes by asserting that D4-C5 remains **UNMET** (it
is not flipped). F1 is reported with its numbers. A row is never removed or
relaxed to make the slice look complete. Any row that cannot pass stops the
implementation (no partial merge of S1–S3).

---

## 9. Explicit limitations

| # | Limit | Consequence |
|---|---|---|
| L1 | no semantic binding: occurrences are a name census | two same-named fields are indistinguishable; 27 declarations for `value` |
| L2 | include-based macro visibility is not implemented | #4/#5 (2 of 7 uses) are absent; not denied, not guessed |
| L3 | the implicit-member tier is reserved, not emitted | #3 (1 of 7 uses) is absent |
| L4 | static member access (`Type::member`) is outside the frozen taxonomy | not emitted; `field_expression` requires a `field_identifier` |
| L5 | C files produce no occurrence rows | a C struct member census is unavailable until the C arm is added |
| L6 | `.cu`/`.cuh` unsupported, `.inl` not indexed, `.ipp`/`.tpp` not scanned | real blind spots for this and any name |
| L7 | renamed/aliased members | invisible to any name-based method (E1) |
| L8 | an empty list is not absence | the response carries `scope`/`binding`/freshness; consumers must not infer "no use" |
| L9 | volume and performance unmeasured | index-size/time impact is unknown until F1; no rate is claimed |
| L10 | occurrences are not traversable | `trace`/`impact`/`callers` never see them; workflow 4 stays partial |
| L11 | the design's §4.3 edge model and `static-member-access` role are not implemented | recorded deviations (§1.4); a reader of the design must read this spec for the frozen subset |
| L12 | text order vs macro order | a use before its `#define` is still a `candidate`; `#undef` is ignored |
| L13 | one grammar version | `tree-sitter-cpp 0.23.4` shapes only; a grammar bump can change roles |

---

## 10. OWNER decisions remaining before implementation

Each is a decision this document cannot take. Recommendation first.

| # | Decision | Recommendation | If rejected |
|---|---|---|---|
| D1 | **Surface**: opt-in `occurrences` section on `search` (A), with Δ1–Δ3 | approve A | B leaves the agent path on shell; C unfreezes the twelve-tool catalogue and `tests/mcp.rs:184-212` |
| D2 | **Scope**: repository-wide exact-name census (with `field_declarations`) | approve | include-scoped direct is measured lossy (2 uses) and cannot be called complete; transitive needs a new resolver (not authorized) |
| D3 | **Macro policy**: same-file only, #4/#5 documented | approve the minimum | the alternative needs a `macro_definitions` fact + include visibility; it gains 2 uses and adds a table + copy/promote/parity surface |
| D4 | **Implicit tier**: reserved, not emitted | approve (no volume data) | enabling it needs a bounded-rule decision and its own volume measurement; a schema v6 |
| D5 | **Schema**: v4→v5 + parser 2→3 + one forced rebuild per repository (Δ4/Δ5, including `riltra_health` reading 5) | approve | without the bump the facts are silently reused (§7 I4); without the test sweep the suite fails |
| D6 | **Pilot acceptance**: 4 of 7 uses, P4 states D4-C5 stays UNMET | approve | any expectation above 4/7 requires D3 and/or D4 first |
| D7 | **Performance gate**: accept F1's measured rows/bytes/time after the fact | approve | a threshold must be chosen before S5 and will be an invented claim until measured |
| D8 | **Language scope**: C++ only | approve | adding the C `field_expression` arm is small but changes the census's meaning (C field declarations are not indexed) |

Approval of D1–D8 is the OWNER GO for S1–S5, and nothing beyond it: no merge, no
tag, no release, no CP4, no CP1/CP2 re-scoring.

---

## Appendix A — what was checked for this document

- Context7: tree-sitter `Node` API (`child_by_field_name`, `byte_range`,
  `start_position`, `parent`), rmcp `#[tool]`/`Parameters<T>` schema
  generation, rusqlite `execute_batch` + prepared statements (table §1.3).
- Source at the baseline HEAD: the anchors in §1.2, including the four response
  constructors, the two promotion lists, the three copy counts, the parity test
  and the catalogue test.
- Frozen evidence: the 9-occurrence inventory and the CP1/CP2 text quoted
  above. **No MMSP measurement was repeated**; no build was run.
- This task changed exactly one file: this document. No test run is claimed for
  a documentation-only change (the last full run was 723 passed / 0 failed at
  `4ea3398`).

## Appendix B — evidence-to-row map (pilot)

| Frozen # | Emitted role | Confidence | Section member | Expected pilot assertion |
|---|---|---|---|---|
| 1 | — (Slice 1 `field`) | — | `results[]` | `search` hit at `patch_match_options.h:56:10` |
| 2 | `macro-argument` | `candidate` | `items[]` | `patch_match_options.cc:58:15` |
| 3 | — (reserved) | — | absent | recorded as L3 |
| 4, 5 | — (include macro) | — | absent | recorded as L2 |
| 6, 7 | `member-access` | `observed` | `items[]` | `patch_match.cc:362:16`, `:365:31` |
| 8 | — (string literal) | — | absent | never a row (rule §3.2) |
| 9 | `address-of-binding` | `observed` | `items[]` | `option_manager.cc:1019:41` |

`total` for `max_triangulation_angle` = 4; `field_declarations` = 1;
`symbol_declarations` = 1; the filter site `patch_match.cc:443-447` is absent
and stays absent (P2).
