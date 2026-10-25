# M9 E7 Slice 2 — implementation record

**Status: IMPLEMENTED (S1–S3), PILOT ACCEPTANCE PASSED (S4), PERFORMANCE
MEASURED (S5).** This record follows the frozen specification
[`m9-e7-slice2-implementation-spec.md`](m9-e7-slice2-implementation-spec.md)
(design of record: Alternative 2, separate occurrence facts, never graph
edges). The frozen documents are not edited; every difference from them is
listed in §2.

- Branch: `m9-multirepo-release-candidate`
- Authorization: the OWNER instruction of 2026-10-25 ("implement E7 Slice 2,
  focus on finding references", together with "fix the field anchor"). It
  approves D1–D8 of the spec's §10 as recommended and nothing beyond them: no
  push, no merge, no tag, no release.
- Commits (each DCO-signed, each leaving the tree green):

| Stage | Commit | Content |
|---|---|---|
| S1 | `1de6b96` | C++ extraction of `member-access`, `address-of-binding`, `macro-argument` (syntax.rs only) |
| S2 | `726f859` | table `reference_occurrences`, schema 4 → 5, parser 3 → 4, insert / copy / promote, golden `cfg=` digest |
| S3 | `37b9dd3` | `search --occurrences` and the optional `occurrences` property of `riltra_code_search` |
| anchor | `80f82c2` | a data member is an anchor (§6.1) |
| callers | `4bc8ed5` | callers of out-of-line C++ definitions (§6.2) |

## 1. What the feature is

`riltra search --repo R --symbol NAME --occurrences` (and
`riltra_code_search {repo, query, occurrences:true}`) appends one object to the
search document, after `"truncated"`:

```json
"occurrences":{"scope":"repository-wide-exact-name","binding":"unbound",
 "field_declarations":1,"symbol_declarations":1,"total":4,"truncated":false,
 "items":[{"path":"…","role":"member-access","confidence":"observed",
 "span":{…},"bytes":[b,e],"provenance":{"provider":"syntax","grammar":"cpp:0.23.4","language":"cpp"}}]}
```

It is a **name census**, not a binding. `scope` and `binding` are literals that
are present even when the list is empty; `field_declarations` and
`symbol_declarations` expose the ambiguity of the name; the confidence is
derived from the role (`observed` for a member access and the address of a
member, `candidate` for a bare argument of a same-file function-like macro).
Without the flag every document is byte-identical to before.

## 2. Differences from the frozen specification

| # | Spec | Implemented | Why |
|---|---|---|---|
| 1 | `PARSER_VERSION` 2 → 3 | **3 → 4** | the template-callee fix (`f099fde`) took 3 on this branch; the spec's own rule ("whichever lands second takes the next number") |
| 2 | `CopiedTotals` gains an `occurrences` field | not added | no document reports it and no code reads it; the copy still runs (counter slot only), so the incremental/full parity test covers it |
| 3 | fixture file `tests/fixtures/m2/cpp/field-refs.cc` | inline sources in the unit tests | a new file in the M2 fixture tree would change the whole-tree scan goldens; the existing C++ tests use inline sources too |
| 4 | pilot row P3: `value` → `field_declarations:27` | **9** (exact name) | the 27 of the Slice 1 record is a case-insensitive count: 9 fields named `value` plus 18 named `Value`. The census is case-sensitive, as C++ is. The check was rewritten to compare the census with the exact-name subset of the symbol list, an independent query, and a second check records why 27 appeared. The frozen text is not edited; this record is the correction |
| 5 | callers / impact "unchanged" (K2) | two owner-directed fixes (§6) | the same instruction asked to fix the field anchor, and the evidence showed callers of out-of-line methods were always empty |

## 3. Staged verification (RED before GREEN)

| Stage | RED evidence | GREEN |
|---|---|---|
| S1 | 8 of the 14 new syntax tests failed on empty vectors and on the confidence assertion; the remaining 6 are negative guards (object-like macro, string literal, declarations, qualified member, C sources, foreign macro) | `syntax::tests` 46 passed; unit suite 420 passed |
| S2 | the migration, the promotion, the v4 `open_read` and the parity tests failed on the missing table | unit 422, cli 16, edges 147, index 26, knowledge 52, mcp 57, semantic 15, syntax 12 |
| S3 | the census unit test, 8 CLI tests and 2 MCP tests failed (unknown flag, absent section, unknown property) | unit 424, index 35, mcp 58; full suite 759 passed |

Tests added, by spec row:

| Spec row | Test |
|---|---|
| A1, A2, A8 | `a_cpp_member_access_is_an_observed_occurrence_at_the_identifier`, `the_address_of_a_member_is_one_address_of_binding_row`, `a_dereferenced_member_is_a_member_access_not_an_address_of_binding`, `a_bare_argument_of_a_same_file_function_macro_is_a_candidate` |
| A3 | `a_string_literal_is_never_an_occurrence` |
| A4 | `search_occurrences_lists_each_use_with_its_role_and_the_ambiguity_counts` (the declaration counts), the exact-name test |
| A5 | `the_confidence_of_an_occurrence_is_derived_from_its_role`, pilot check `A5` |
| A6, I7 | `an_absent_name_has_an_explicit_empty_occurrence_list` |
| A7 | pilot check `P1-decl` |
| S1 (parity) | `incremental_matches_independent_full_rebuild`, now with the occurrence rows (unchanged file copied, changed file extracted again) |
| S2 (promotion) | `promotion_carries_the_new_occurrences_and_drops_the_previous_ones` |
| S3 (versions) | `a_v4_database_gains_the_empty_occurrence_table_when_it_migrates`, `read_open_refuses_old_and_new_versions_without_migrating` (v4 added), `f1_flipped_v2_v3_refused_unchanged` (v4 added), `an_index_from_the_previous_parser_version_has_no_occurrences_until_it_is_rebuilt` |
| K3 | `mcp_tools_list_advertises_the_twelve_tools` (the property is in `properties`, not in `required`) |
| K4 | `search_without_the_occurrences_flag_has_no_occurrences_member` and the byte-stable `response.rs` search tests |
| K5 | `search_occurrences_is_deterministic`, `occurrence_paging_applies_to_each_list_with_its_own_totals` |
| §4.3 | `search_occurrences_without_an_index_adds_no_member`, `an_old_index_answers_stale_with_no_occurrences_until_it_is_rebuilt` |
| MCP/CLI parity | `code_search_occurrences_is_the_cli_document_and_off_by_default` |

The `cfg=` fingerprint of every golden depends on the parser version. The 88
golden files changed by that digest only (`9193504b411fb110` →
`fc5c6854d048a25d`, 780 lines); reversing the substitution reproduces `HEAD`
byte for byte for every file, and nothing was regenerated.

## 4. Pilot acceptance at the pin (S4)

Setup: MMSP `d62f06a576d8e76b705c8ecc57fc380f7ebc9c44`, tree
`c5482ea00481675e0aa921538f560947b68dca12`, a plain clone of the public
remote in a short bench path (`core.longpaths=true`; the MMSP tree has paths
of up to 152 characters), `git status --porcelain` empty, isolated
`RILTRA_HOME`, debug build. Script:
[`docs/pilots/m9-rc/method/e7-pilot.cjs`](pilots/m9-rc/method/e7-pilot.cjs).

`search --repo mmsp-pin --symbol max_triangulation_angle --occurrences`:

| # | Location | Role | Confidence | Frozen occurrence |
|---|---|---|---|---|
| 1 | `colmap/src/colmap/controllers/option_manager.cc:1019:41` | `address-of-binding` | observed | #9 |
| 2 | `colmap/src/colmap/mvs/patch_match.cc:362:16` | `member-access` | observed | #6 |
| 3 | `colmap/src/colmap/mvs/patch_match.cc:365:31` | `member-access` | observed | #7 |
| 4 | `colmap/src/colmap/mvs/patch_match_options.cc:58:15` | `macro-argument` | candidate | #2 |

`total` 4, `field_declarations` 1, `symbol_declarations` 1. The declaration
(#1) is the `field` search hit at `patch_match_options.h:56:10`; the option-name
string (#8) is absent; #3, #4 and #5 (the plain identifier and the two
`CHECK_OPTION_LE` arguments) are absent by the recorded policy; the filter
site `patch_match.cc:443-447` does not appear. P1–P5 and A5 pass (12 checks,
0 failed; raw output in `docs/pilots/m9-rc/`).

**D4-C5 stays UNMET.** The frozen grader reads the `paths` of the impact
answer, which stays empty by design: 4 of the 7 code uses are reachable by
name as locations, not as graph relations, and the filter site is not
reachable by name at all. Nothing in the CP1 or CP2 records was edited.

## 5. Performance (F1, measured, no threshold)

Debug build, this host (16 logical cores), the pinned snapshot, the frozen CP2
harness for the before/after rows (`docs/pilots/m9-rc/comparison.md`).

| Row | Before (`f65c183`) | After (release-candidate code) |
|---|---|---|
| full index wall time | 98.99 s and 99.25 s | 97.27 s and 98.63 s |
| database bytes | 50,233,344 | 55,627,776 (+5,394,432, +10.7 %) |
| occurrence rows | — | 58,811 (identical in two cold indexes and in the incrementally restored one) |
| `search` p50, five repeats | 390–401 ms | 375–391 ms |
| `search --occurrences` against `search`, same name, same tool | — | +6 ms at p50 (about +1 %) in a separate five-repeat run of the pilot script |

The census is a name index of member accesses, so its size is dominated by
method-call receivers (`x.f()` is a member access of `f`): 58,811 rows for 91,025
call facts. The cold index time did not move beyond the run-to-run spread; the
extra cost is storage, not time. The absolute latencies printed by
`e7-pilot.cjs` include the Node process-spawn overhead and are comparable with
each other only, not with the harness figures above.

## 6. Owner-directed fixes outside the frozen slice

1. **A data member is now an anchor.** `resolve_edge_anchor` skipped `field`
   symbols, so `callers`, `callees`, `trace` and `impact` at
   `patch_match_options.h:56:10` answered for the enclosing struct (proven by
   the owner's MCP run: the anchor span was the struct's bytes). The member's
   symbol spans its name only, so a position on the name anchors the member
   and any other position in the class body still anchors the class. A member
   is not callable: it takes no call by name, so `callers` of a member never
   lists a same-named method call, and the anchor note says what it is and
   where its references are listed (`search --occurrences`).
2. **Callers of out-of-line C++ definitions were always empty.** Two causes,
   both proven on the pilot (the owner's MCP run returned `edges: []` for
   `PatchMatchController::Run`, `ProcessProblem` and `PatchMatch::Run`):
   - the call sites are matched on the called name (`run`), but the
     definition's name is the qualified text (`Widget::run`), so nothing ever
     matched. The match now uses the part after the last `::`;
   - the candidate files were the definition's file and the files that include
     *it*, which for a `.cc` is none. A call is written in files that include
     the header that declares the class, so the headers that the definition's
     file includes and that hold a class or struct of the qualifier's name
     now join the scope (same unique-suffix include rule as before).

   Effect on the pilot (candidates, `POTENTIAL_CALL`, `Heuristic`, callee never
   resolved): `PatchMatch::Problem::Print` 0 → 3 (`patch_match.cc` ×2,
   `exe/mvs.cc` ×1); `PatchMatchController::Run` 0 → 8 (`patch_match.cc` ×2,
   `exe/mvs.cc` ×2 including the real `controller.Run()` at line 277,
   `automatic_reconstruction.cc` ×4). The candidates are matched by name, so
   other classes' `Run` calls are in the set; nothing separates them without
   type information (semantic, E1).

## 7. Limits (carried from the spec, plus what the pilot showed)

L1–L13 of the spec stand. Added by this implementation:

- the census counts method-call receivers as member accesses, so a name that
  is both a field and a method (`value`) lists both kinds of use, and the
  counts must be read with `field_declarations` and `symbol_declarations`;
- `field_declarations` for a common name is small and exact (9 for `value`);
  a name census can still list uses of other types' members of that name;
- the `.inl`, `.cu` and `.cuh` files are not scanned, so their uses are
  absent, not denied.
