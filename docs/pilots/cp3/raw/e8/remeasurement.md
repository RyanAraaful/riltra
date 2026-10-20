# E8 re-measurement — CP3 phase 3

Both runs answer the **same** 28-query set against the **same** pilot index
(MMSP `d62f06a`, 1 438 indexed files). The E8 correction is a query-side
change — the caller candidate scope — so the index is not rebuilt and the
diff isolates the change exactly.

Machine provenance: `../queries/diff.json`, `../queries/pairing.json`,
`../queries/before/`, `../queries/after/`.

## Headline

| Measure | Value |
|---|---|
| queries run | 28 |
| queries whose answer changed | **4** |
| queries byte-identical | **24** |
| candidate rows added | **5** |
| candidate rows removed | 0 |
| **`CALLS` rows added** | **0** |
| **`REFERENCES` rows added** | **0** |
| any other edge kind added | 0 |

Every changed query is a `callers` query whose anchor is a C++ header. The
`callees`, `trace`, `impact`, `search` and status queries are byte-identical:
the correction cannot reach them, because `caller_candidate_paths` feeds only
the `callers` candidate branch of `store::EdgeSelection::sql`.

## The 5 added candidates, each verified against the source

| Anchor (header) | Added candidate caller | Verification |
|---|---|---|
| `colmap/src/colmap/mvs/patch_match.h` (`PatchMatchController`) | `colmap/src/colmap/controllers/automatic_reconstruction.cc` | the file `#include "colmap/mvs/patch_match.h"` and constructs `mvs::PatchMatchController patch_match_controller(...)` at `:431` |
| `colmap/src/colmap/mvs/patch_match_options.h` (`PatchMatchOptions`) | `colmap/src/colmap/controllers/option_manager.cc` | the file `#include "colmap/mvs/patch_match_options.h"` and constructs `mvs::PatchMatchOptions` at `:85` — the owner CP1 B.3 verified as the bound object's owner |
| `colmap/src/colmap/mvs/patch_match_options.h` (`PatchMatchOptions`) | `colmap/src/pycolmap/pipeline/mvs.cc` | the file `#include "colmap/mvs/patch_match_options.h"` and references `mvs::PatchMatchOptions` |

The `PatchMatchController` row appears in three queries (`d2-1`, `e8-4`,
`p2`) because three anchors resolve to the same definition.

Every added row is `kind: POTENTIAL_CALL`, `precision: Heuristic`,
`provider: syntax`, `unresolved: dynamic`, `callee: null`. **No row was
promoted into a proven edge.**

**Incorrect candidate additions: 0.**

## Expected relations still missing — and why

CP1 B.2 records **three** external construction sites of
`PatchMatchController`. One is now surfaced. The other two remain missing
for reasons that are **not** E8:

| CP1 site | Form | Why it is still missing | Class |
|---|---|---|---|
| `colmap/src/colmap/exe/mvs.cc:271` — `mvs::PatchMatchController controller(options, …)` | direct initialization whose argument list the grammar reads as a parameter-type-list | the syntax layer records **no** call fact for the most-vexing-parse form (only name lookup can decide it) | **E12** — owner-gated, not E8 |
| `colmap/src/colmap/ui/dense_reconstruction_widget.cc:397-398` — `std::make_shared<mvs::PatchMatchController>(…)` | the name appears as a **template argument**; the callee is `make_shared` | the callee terminal is `make_shared`, not `PatchMatchController`; resolving the argument needs a name lookup | **E1 / E7** — owner-gated, not E8 |

**E8 closes none of E1, E7 or E12.** The E8 correction only makes the
candidate scope non-empty for translation units that include the anchor's
header; what happens inside those units is still bounded by the syntax
layer.

## What the improved candidate set is, and is not

An improved candidate set is **not** semantic resolution. The five added
rows are *candidates*: the tool reports that a translation unit includes the
anchor's header and mentions the anchor's name, and it says so with
`precision: Heuristic`, `unresolved: dynamic` and a `null` callee. It does
not claim that the call refers to that definition.

## Cost

The correction adds one pass over the snapshot's import rows per `callers`
query, plus at most one path scan per distinct literal that can name the
anchor. The pairing table (`../queries/pairing.json`) shows no systematic
latency change on the changed queries; the single-run timings on this host
are dominated by the fixed per-invocation cost that CP2 recorded as E9 and
that phase 6 profiles separately.
