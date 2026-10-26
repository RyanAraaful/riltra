# E8 verified sample — CP3 phase 1

Pilot: MMSP at `d62f06a576d8e76b705c8ecc57fc380f7ebc9c44`
(isolated clone, 2 277 tracked files, 1 438 indexed files).
Include root: `colmap/src` (verbatim evidence in `include-roots.txt`).

Machine provenance: `baseline.json` (produced by `method/e8-baseline.cjs`),
`missing-class.json` (produced by `method/e8-missing-class.cjs`),
`anchors.json` (produced by `method/e8-anchor.cjs`).

## The compiler ground truth

A C++ compiler resolves a **quoted** include in this order:

1. the directory of the including file,
2. each `-I` root, in order.

`method/e8-baseline.cjs` implements exactly that order against the pinned
CMake include root, so "what the compiler would open" is computed, not
assumed. No C++ compiler is installed on the pilot host (`g++`, `clang++`,
`gcc`, `cl`, `cmake` are all absent), so the compiler's own `-H` trace could
not be used; the CMake evidence plus the documented search order is the
strongest available ground truth.

## Headline counts (4 009 quoted C/C++ includes)

| Rule | Resolves |
|---|---|
| `probe_rule` — literal **is** the repository-relative path (what CP2 measured) | **0** |
| `code_rule` — literal resolved **relative to the importer's directory** (what the product actually applies) | **1 353** |
| `suffix_unique` — exactly one indexed path equals the literal or ends with `/` + literal | **2 993** |
| `suffix_ambiguous` | 969 |
| `suffix_none` | 47 |

**CP2's "0 of 4 009" is true of its own probe rule, not of the product.**
The product already resolves 1 353 of the 4 009 (33.7 %) through
same-directory includes. The 2 993 suffix-unique rows therefore contain
**2 609 rows the product does not resolve today**.

## Agreement with the compiler, over the 2 609 NEW candidate rows

| Verdict | Count |
|---|---|
| the suffix match **is** the file the compiler opens | **2 608** |
| the compiler opens a **different** file | 0 |
| the compiler would not open any repository file | **1** |

Of the 2 609 new rows, 2 608 carry a **rooted** literal (contains `/`) and
exactly **one** carries a bare filename. The single failure is that one bare
row. Excluding bare-filename literals from the suffix rule therefore removes
100 % of the observed misleading matches on the pilot while keeping
2 608 / 2 609 (99.96 %) of the new candidates.

## Class 1 — true matches (verified)

Each row: the literal is a component-boundary suffix of the indexed path; the
compiler's search order lands on exactly that file; and the file declares the
symbol the anchor names.

| Importer | Literal | Resolves to | Verification |
|---|---|---|---|
| `colmap/src/colmap/exe/mvs.cc` | `colmap/mvs/patch_match.h` | `colmap/src/colmap/mvs/patch_match.h` | compiler: same; the header declares `class PatchMatch`, `struct Problem`, `void Print() const;`, `class PatchMatchController` |
| `colmap/src/colmap/controllers/automatic_reconstruction.cc` | `colmap/mvs/patch_match.h` | `colmap/src/colmap/mvs/patch_match.h` | compiler: same |
| `colmap/src/colmap/ui/dense_reconstruction_widget.cc` | `colmap/mvs/patch_match.h` | `colmap/src/colmap/mvs/patch_match.h` | compiler: same |
| `colmap/src/colmap/mvs/patch_match.cc` | `colmap/mvs/patch_match_cuda.h` | `colmap/src/colmap/mvs/patch_match_cuda.h` | compiler: same |
| `colmap/src/colmap/mvs/patch_match_options.cc` | `colmap/mvs/patch_match_options.h` | `colmap/src/colmap/mvs/patch_match_options.h` | compiler: same |
| `colmap/benchmark/runtime/bundle_adjustment.cc` | `colmap/estimators/bundle_adjustment.h` | `colmap/src/colmap/estimators/bundle_adjustment.h` | compiler: same |

**Frozen CP1 anchors.** `docs/m9-cp1-pilot-spec.md` B.2 records exactly three
external construction sites of `PatchMatchController`: `exe/mvs.cc:271`,
`controllers/automatic_reconstruction.cc:431`,
`ui/dense_reconstruction_widget.cc:397`. All three translation units
`#include "colmap/mvs/patch_match.h"`, and the compiler opens exactly the
file that declares `PatchMatchController`. The suffix rule therefore predicts
the CP1 ground truth; today's exact rule resolves **none** of these anchors
(`today_exact_rule` is null for every one of them in `anchors.json`).

## Class 2 — incorrect potential match (verified)

| Importer | Literal | Suffix claim | What the compiler opens |
|---|---|---|---|
| `colmap/src/thirdparty/VLFeat/lbp.c` | `string.h` | `colmap/src/colmap/util/string.h` | **nothing in the repository** — the C library's `string.h` |

`lbp.c` includes `"string.h"`; there is no `string.h` next to it, and
`colmap/src/string.h` does not exist, so the compiler falls through to the
system header. The repository happens to contain exactly one file whose path
ends in `/string.h`, so the suffix rule would have claimed it. This is the
**bare-filename shadowing** class, and it is the only misleading match on the
pilot.

## Class 3 — ambiguous paths (stay unresolved)

| Literal | Indexed files that match |
|---|---|
| `shared_indices.h` | `colmap/src/thirdparty/Symforce-Caspar/generated/f32/shared_indices.h`, `…/f64/shared_indices.h` |

The literal names two files in different generated trees. The uniqueness rule
leaves it unresolved, which is honest rather than wrong. 969 rows are
ambiguous on the pilot.

## Class 4 — missing headers (47 rows, two distinct causes)

| Cause | Rows | Example |
|---|---|---|
| present in the tree but **not indexed** (`.inl` is not a recognised extension) | **40** | `colmap/src/thirdparty/PoissonRecon/Array.inl` |
| **absent from the repository** (fetched by CMake at configure time) | **7** | `thirdparty/SiftGPU/SiftGPU.h`, `thirdparty/LSD/lsd.h`, `JPEG/jpeglib.h` |

The 40 `.inl` rows are a **file-extension coverage gap** (G-SYN), not an
include-resolution defect: `crates/cli/src/syntax.rs` recognises `h, hh, hpp,
hxx, cc, cpp, cxx` only. The 7 absent rows are headers the build system
downloads; the repository genuinely does not contain them.

## What this sample does and does not prove

**Proves.** On this pilot, the unique-suffix rule agrees with the compiler's
own resolution on 2 608 of 2 609 new candidate rows, and the one failure is a
bare filename that a conservative rule excludes. The rule reproduces the
frozen CP1 cross-TU ground truth exactly.

**Does not prove.** That a unique path suffix is the file the compiler opens
in general. The pilot has a single project include root, so a literal that
could be resolved through a *different* root never appears here. The rule
stays a **candidate** expansion for that reason: it may widen the search
scope, and it must never establish identity on its own.
