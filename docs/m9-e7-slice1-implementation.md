# M9 E7 Slice 1 — implementation record: C++ data members as `field` symbols

**Scope: E7 Slice 1 only.** Slice 1 makes a C++ data-member *declaration*
findable as an indexed symbol of kind `field`. It does **not** find uses,
references, bindings or impact — those stay Slice 2 work, and the CP1
capability gates D1-C6, D2-C6 and D4-C5 stay **UNMET**.

- Branch: `m9-multirepo-release-candidate`
- Baseline commit: `b73dd360cd7295707248d6bb88fd29aaac2a7c53` (the E7 Phase A
  design, verified local == remote before the work started)
- Pilot pin (unchanged): MMSP `d62f06a576d8e76b705c8ecc57fc380f7ebc9c44`,
  tree `c5482ea00481675e0aa921538f560947b68dca12`
- Design of record: `docs/m9-e7-reference-resolution-design.md` (§4.1, §4.2)
- Acceptance source: `docs/m9-cp1-pilot-spec.md` §B.4, §D.1–D.5

---

## 1. Baseline and final commit

| | |
|---|---|
| Starting HEAD (= remote) | `b73dd360cd7295707248d6bb88fd29aaac2a7c53` |
| Final commit | the DCO-signed commit that carries this record (see the final report) |
| Working tree before the work | clean apart from the untracked owner-local `AGENTS.md`, which was not read, modified or committed |
| `SCHEMA_VERSION` | **4 — unchanged** |
| `PARSER_VERSION` | 1 → **2** (`store.rs:16`) |

## 2. Implementation changes

| File | Change | Why |
|---|---|---|
| `crates/cli/src/syntax.rs` | `push_field` + `field_identifier_of` + `inner_declarator`; the C++ arm of the walker gains `"field_declaration" => push_field(...)` | the extraction itself: a data member becomes a `Fact` of kind `field` in the existing `FileFacts.definitions` collection. No new `Fact` type, no new collection, no new framework |
| `crates/cli/src/store.rs` | `PARSER_VERSION` 1 → 2 | the extraction changed, so an index written by the previous parser must not be reused silently (the contract at `store.rs:13-15`) |
| `crates/cli/src/index.rs` | two one-line adjustments: the syntax anchor fallback skips `kind == "field"` (`index.rs:1455`) and the stale byte-offset recovery reads the symbol set of the previous parser version (`index.rs:1276`); the parity test's repository gains a C++ file with a data member | a field symbol must not become the anchor of a position, and must not change how a stale position is resolved (both would change `callers`/`callees`/`trace`/`impact`); the parity check must carry the field facts through the incremental copy path |
| `crates/cli/src/syntax.rs` (tests) | 8 unit tests | the extraction rules (AC1, AC2, AC3) |
| `crates/cli/tests/index.rs` | 4 CLI-level tests | search finds the field, methods and function-pointer members are not fields, the anchor does not move, the previous parser version is rebuilt |
| `crates/cli/tests/golden/**` | 89 files updated | 88 carry only the config fingerprint (a mechanical consequence of the parser-version bump — the fingerprint embeds `PARSER_VERSION`, `index.rs:70-85`); 1 (`m2-c-util.json`) gains two hand-verified field rows (§8) |
| `docs/pilots/e7-slice1/raw/mmsp-measurement.txt` | new | the raw MMSP measurement: the snapshot identity, the search result, the index counters and the timings |

**Not changed** (per the authorization): no `reference_occurrences` table, no
`POTENTIAL_REFERENCE` edge kind, no `code_edges` or impact-traversal change, no
occurrence role, no member-reference binding, no new CLI command, no new MCP
tool (the catalog stays at twelve), no dependency, no schema version, no
CUDA/`.inl`/E1/E9/E12 work, no change to the frozen CP1/CP2/CP3 evidence.

## 3. RED → GREEN evidence

The tests were written before the implementation.

**RED (extraction, before the change)** — the five positive cases failed with
an empty field set:

```
test syntax::tests::a_cpp_field_with_an_initializer_is_a_field ... FAILED
  assertion `left == right` failed: []
test syntax::tests::extracts_a_cpp_data_member_as_a_field ... FAILED
  assertion `left == right` failed: []
test syntax::tests::multiple_cpp_declarators_are_separate_fields ... FAILED
test syntax::tests::pointer_reference_and_array_members_are_fields ... FAILED
test syntax::tests::same_named_cpp_fields_stay_separate_symbols ... FAILED
```

**GREEN (after the change)** — `cargo test -p riltra --bin riltra syntax::tests::`
→ `28 passed; 0 failed`, including the two exclusion cases, which are
meaningful only once fields exist:

```
test syntax::tests::a_cpp_function_pointer_member_is_not_a_field ... ok
test syntax::tests::a_cpp_method_declaration_is_not_a_field ... ok
test syntax::tests::a_cpp_field_with_an_initializer_is_a_field ... ok
test syntax::tests::extracts_a_cpp_data_member_as_a_field ... ok
test syntax::tests::multiple_cpp_declarators_are_separate_fields ... ok
test syntax::tests::pointer_reference_and_array_members_are_fields ... ok
test syntax::tests::qualified_and_derived_cpp_members_are_fields ... ok
test syntax::tests::same_named_cpp_fields_stay_separate_symbols ... ok
```

**RED (anchor compatibility)** — the characterization test, written *before*
the anchor adjustment, failed exactly as it was meant to: with the field
symbols in place the anchor of a position inside the member became the member
(`"span":[34,57]`) instead of the enclosing class (`"span":[0,68]`):

```
---- a_data_member_does_not_change_the_anchor_inside_its_class stdout ----
the anchor must stay the class, not the data member:
  ..."anchor":{..."span":[34,57],"endpoint":{"type":"syntax_def",...
```

**GREEN** after the one-line adjustment in `resolve_edge_anchor`:
`25 passed; 0 failed` for the `index` suite.

## 4. Parser-version transition

`PARSER_VERSION` 1 → 2 (`store.rs:15`). The two mechanisms that depend on it
already existed and are generic:

- **Freshness**: `evaluate_snapshot` pushes the reason `parser-changed` when
  the stored snapshot's parser version differs (`index.rs:2786`).
- **Rebuild**: the refresh path falls back to a full rebuild with
  `fallback_reason: "parser-changed"` (`index.rs:2572`).

The CLI-level test `an_index_from_the_previous_parser_version_is_rebuilt`
proves the transition end to end: it fabricates `parser_version = 1` in the
snapshot row, then asserts `index status` → `"freshness":"stale"` with
`parser-changed`, `index` → `"mode":"fallback-full"` with `parser-changed`,
and the field is searchable afterwards. **An index from the previous parser
version is therefore never silently reused as an index that already has field
facts.** No snapshot rule and no digest rule changed.

## 5. Field extraction coverage

| Form | Indexed as a field? | Reason |
|---|---|---|
| `double max_triangulation_angle = -1.0f;` | **yes** | the declarator ends in a `field_identifier` |
| `int value = target();` (initializer) | **yes** | same; the initializer's call is still recorded as a call candidate (unchanged) |
| `int a, b;` (multiple declarators) | **yes — two symbols** | `field_declaration.declarator` is a multiple field; each declarator yields its own row |
| `double* p;` / `double& r;` / `double arr[4];` | **yes** | pointer, reference and array declarators wrap the identifier and are walked through (the walk is bounded at six wrappers, like the neighbouring `c_name_node` helper; a deeper chain is skipped, which loses a symbol rather than inventing one) |
| `int process(int x);` (method declaration) | **no** | the declarator is a `function_declarator` — a method, not a data member |
| `int (*target)();` (function-pointer member) | **no** | the declarator chain passes through a `function_declarator`; the ambiguous form is skipped rather than guessed |
| `virtual int target() = 0;` (pure virtual) | **no** | same as a method declaration |
| a namespace-scope variable | **no** | it is a `declaration` node, not a `field_declaration`; unchanged behaviour |
| a C struct member in a `.c` file | **no** | the C walker is untouched; only the C++ arm gained the case. **A `.h` header parses as C++ by policy, so C struct members in headers ARE indexed** (see §8) |
| a member of a class in a `.cu` file | **no** | CUDA stays unsupported (`syntax.rs:77`) |

The fact points at the **name** (the identifier), not at the type: the span
and the byte range cover the identifier, so the **indexed symbol and the
search hit** are at the name. This is what makes the CP1 anchor
`patch_match_options.h:56:10` reproducible. The **edge anchor** of
`callers`/`callees`/`trace`/`impact` is a different thing and is deliberately
*not* the member: in Slice 1 it stays the enclosing definition (§2).

No symbol is created when the declaration form cannot be interpreted
reliably: an unknown declarator kind yields nothing.

## 6. MMSP search result at the pin

**Setup.** A disposable isolated snapshot was created **from the public
remote** (`https://github.com/RyanAraaful/mmsp.git`), not from the local MMSP
clone: `git init` + `git fetch --depth 1 origin d62f06a…` + `git checkout
FETCH_HEAD`, in `%TEMP%\opencode\m9-e7-slice1\mmsp-pin`, with an isolated
`RILTRA_HOME` in `%TEMP%\opencode\m9-e7-slice1\home`. Verified before the
run:

| | |
|---|---|
| HEAD | `d62f06a576d8e76b705c8ecc57fc380f7ebc9c44` ✓ |
| tree | `c5482ea00481675e0aa921538f560947b68dca12` ✓ |
| `git status --porcelain` | empty ✓ |
| tracked files | 2 277 — the CP1 frozen inventory (`docs/m9-cp1-pilot-spec.md` §A.3) ✓ |
| the original MMSP clone | never read, never written, never cloned from |

**Before.** The frozen CP2 capture is the reference: `riltra_code_search
max_triangulation_angle` → **0 hits** — "the data member is not an indexed
symbol (reported, not guessed)" (`docs/m9-cp2-reevaluation.md:512`), and the
CP3 OpenCode session could not name the member at all
(`docs/m9-cp3-evidence.md:567-571`).

**After** (full rebuild, `mode: "full"`, `fallback_reason: "cold-start"`):

```
$ riltra search --repo mmsp-pin --symbol max_triangulation_angle
{"repo":"mmsp-pin", ... "freshness":"fresh","reasons":[],
 "query":{"symbol":"max_triangulation_angle","limit":50,"offset":0},
 "total":1,
 "results":[{"path":"colmap/src/colmap/mvs/patch_match_options.h",
             "kind":"field","name":"max_triangulation_angle",
             "span":{"start_line":56,"start_col":10,"end_line":56,"end_col":33},
             "bytes":[2460,2483],
             "provenance":{"provider":"syntax","grammar":"cpp:0.23.4","language":"cpp"},
             "snapshot":1}],"truncated":false}
```

The hit is the **frozen CP1 anchor** of §B.4: the declaration of
`max_triangulation_angle` in `colmap/src/colmap/mvs/patch_match_options.h` at
**line 56, column 10** — the position of the identifier — with the
repository identity, the freshness verdict and the provenance.

**Field facts added.** The index counters against the frozen CP2 capture at
the same pin (the raw command output is archived in
`docs/pilots/e7-slice1/raw/mmsp-measurement.txt`):

| Counter | CP2 (frozen) | Slice 1 | Change |
|---|---|---|---|
| `files.definitions` | 12 105 | **17 213** | **+5 108 field symbols** |
| `files.imports` | 6 680 | 6 680 | unchanged |
| `files.calls` | 91 025 | 91 025 | unchanged |
| `files.errors` | 1 991 | 1 991 | unchanged |
| `files.processed` | 1 438 | 1 438 | unchanged |

Only the definition counter moved, and it moved by exactly the number of
extracted data members.

**Same-named declarations.** `max_triangulation_angle` has exactly **one**
declaration (`total: 1`). The name `value` has **27** distinct declarations
across different types and files (for example
`colmap/src/colmap/controllers/base_option_manager.h:139:17`,
`colmap/src/thirdparty/PoissonRecon/Geometry.h:634:79`,
`colmap/src/thirdparty/VLFeat/mathop.h:55:38`); they stay **separate symbol
rows** with distinct locations and are never merged (AC3). The count is
exact: the search is a prefix+substring match, and the 27 are the results
whose `kind` is `field` **and** whose `name` is exactly `value`. The raw
command output is archived in
`docs/pilots/e7-slice1/raw/mmsp-measurement.txt`.

## 7. Incremental/full parity

**In the test suite.** The existing parity guard
(`index.rs:incremental_matches_independent_full_rebuild`) compares the fact
tables of an incrementally built index and of an independent full rebuild.
Its repository now contains a C++ file with a data member, so the check
covers the field facts and — because the file is unchanged across the
stages — the **incremental copy path** that carries them. The guard asserts
`assert_eq!(read(&home_b), read(&home_c), "incremental must equal full")` and
the `symbols` dump it compares includes the `kind` column, so a field row is
compared like any other fact.

**On the pilot snapshot.** After a one-file change (a comment appended to
`colmap/src/colmap/mvs/patch_match_options.cc`):

```
mode "incremental", reused 1437, reparsed 1, added 0, removed 0
files.definitions 17 213   files.imports 6 680   files.calls 91 025   files.errors 1 991
riltra search --repo mmsp-pin --symbol max_triangulation_angle
  → total 1, kind field, colmap/src/colmap/mvs/patch_match_options.h:56:10
```

The 1 437 reused files carried their field facts through the copy path
(the definition counter is unchanged), and the field is still found at the
frozen anchor after the refresh. The snapshot was restored afterwards (it is
disposable).

## 8. Regression and golden changes

| Golden set | Files | Change | Why |
|---|---|---|---|
| `tests/golden/m6-corpus/**` | 79 | the `cfg=` fingerprint only | the config fingerprint embeds `PARSER_VERSION` (`index.rs:70-85`), so the parser bump changes the revision string in every response |
| `tests/golden/m6-*.json` | 9 | the `cfg=` fingerprint only | the same |
| `tests/golden/m2-c-util.json` | 1 | **two new field rows** | `c/util.h` parses as C++ (the header policy) and holds `int x;` / `int y;` |

The golden diff was **verified mechanically**: across the 88 fingerprint
files, the number of changed lines that are *not* the fingerprint is **0** —
no behavioural change is hidden by the regeneration. The two field rows in
`m2-c-util.json` were verified by hand against the source (line 7, column 9,
bytes 124–125 for `x`; line 8, column 9, bytes 135–136 for `y`; the `struct
Point` row is byte-for-byte unchanged).

The **frozen** CP7-A raw evidence (`docs/pilots/cp7a/goldens-raw/`) still
carries the historical fingerprint and was **not** modified: it records the
historical run, and no test compares against it.

Other regressions checked: all existing goldens pass unchanged; the E8
caller-scope regression still passes; the MCP tool count is still twelve and
the MCP **contract** is unchanged — the `code_search` response for a data
member now carries `kind: "field"`, which is the feature, not a contract
change; the CLI JSON ordering and exit codes are unchanged.

## 9. Performance (measured, one host)

| Measurement | Value | Context |
|---|---|---|
| cold full index of the pinned snapshot | **166.1 s** | debug profile, one host, `mode: "full"`, `fallback_reason: "forced-full"` |
| one-file incremental refresh | **66.0 s** | debug profile, `reused 1437`, `reparsed 1` |
| index store size | 50 663 424 B | CP2 recorded 49 762 304 B for the same pin |

The command output behind these numbers is archived in
`docs/pilots/e7-slice1/raw/mmsp-measurement.txt`.

These are **single-run, single-host** numbers from a disposable snapshot on
the machine that produced this record. They are **not** comparable with the
M3/M4 or CP2 figures (different host, different run, different machine
state) and **no speed-up or overhead claim is made**: the CP2 baseline was
220.75 s / 216.72 s / 229.46 s / 222.05 s for the cold index and 70.2 s for
the incremental (`docs/m9-cp2-reevaluation.md:125-128`, `:170`), and the
slice-1 numbers sit inside that spread. The extraction cost of the new arm
is therefore **not separately measured**; the whole-tree delta is inside the
run-to-run spread of the baseline.

## 10. Residual gaps and Slice 2 readiness

**What Slice 1 does not do** (unchanged from the design):

- no *use* of a data member is found: a member read
  (`options_.max_triangulation_angle`), an address-of binding
  (`&patch_match_stereo->max_triangulation_angle`) and a macro argument
  (`PrintOption(max_triangulation_angle)`) are still invisible;
- no binding: a candidate is never tied to a declaration, and the CP1
  capability gates D1-C6, D2-C6 and **D4-C5 stay UNMET**;
- no function-pointer member is indexed (the ambiguous declarator is
  skipped, deliberately);
- C struct members in `.c` files are not indexed (the C walker is untouched);
- `.inl` headers are still not indexed (CP3 §9) and CUDA stays unsupported.

**Readiness for Slice 2.** The pieces Slice 2 needs are now in place and
proven: the extraction walks C++ class bodies (so a `field_expression` and a
`pointer_expression` are reachable), the fact carries a name, a span and byte
offsets, the index carries a `symbols` row per declaration, the freshness and
rebuild machinery is exercised by the parser-version transition, and the
anchor behaviour is pinned by a characterization test. The open Slice 2
decisions are unchanged from the design: the storage of the occurrences
(a new table and a `SCHEMA_VERSION` bump), the `POTENTIAL_REFERENCE` edge
kind, the candidate scope, and the OWNER interpretation of CP1 §D.4's
honesty item 4 (D4-H4).

**Known limits of this record.** The MMSP measurement used one disposable
snapshot on one host with a debug build; the field-fact count (5 108) and
the anchor are exact, the timings are indicative only. One correction was
made during the measurement itself: a shell variable collision (`$home` vs
the read-only `$HOME`) put the first index in the user's home directory; the
artifacts created there (an `index/` directory and a `registry.json`, both
new, both containing only this measurement) were removed, and the
measurement was repeated in the isolated `RILTRA_HOME`.

---

## 11. Internal review

An independent read-only review of the whole diff raised five findings and
six suggestions. The valid ones were corrected; the dispositions are:

| Finding | Disposition |
|---|---|
| the stale byte-offset recovery reads `symbols` without a kind filter, so a field row could change how a stale position resolves | **corrected** — the query now uses the symbol set of the previous parser version (`index.rs:1276`) |
| the declarator walk is bounded at six wrappers, which the coverage table did not say | **documented** in §5 (the bound matches the neighbouring helper; a deeper chain is skipped, never guessed) |
| three line references in this record were stale | **corrected** (§1, §4) |
| the RED excerpt listed four of the five positive extraction cases | **corrected** (§3) |
| "the anchor of a member is its name" conflated the indexed symbol with the edge anchor | **corrected** (§5) |
| the same-name count was not reproducible as stated | **corrected** — the method is stated and the raw output is archived (§6) |
| the golden-diff and MMSP numbers were not backed by artifacts | **corrected** — the raw measurement is archived in `docs/pilots/e7-slice1/raw/`; the golden diff is verifiable from the commit itself |
| the parser-version test's comment claimed a pre-Slice-1 index that the fixture did not have | **corrected** — the test now removes the field rows and asserts the field is absent before the rebuild and present after it |
| declaration forms (static, mutable, bitfield, anonymous union) and duplicate indexing were untested | **corrected** — `qualified_and_derived_cpp_members_are_fields` covers them and asserts one row per declaration |
| two wording slips in this record ("no new fact kind", "no MCP response changed") | **corrected** (§2, §8) |

The review also confirmed the scope: no `reference_occurrences` table, no
`POTENTIAL_REFERENCE` edge kind, no change to `code_edges` or the impact
traversal, no new CLI command, no new MCP tool (still twelve), no dependency,
`SCHEMA_VERSION` still 4, and the frozen CP1/CP2/CP3 evidence and the
historical verdicts untouched.
