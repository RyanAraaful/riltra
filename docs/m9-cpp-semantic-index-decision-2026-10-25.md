# M9 C++ semantic index for MMSP: scope and integration cost (2026-10-25)

**Status: decision memo. Not approved, not implemented.** Nothing was installed or
downloaded. No compilation database or index was written, and no repository was
changed. The owner must approve a scope and its cost before any step below runs.

## 1. Question

The evaluation (`docs/m9-task5-capability-gap-evaluation.md` §9, step 2) asks
whether to build a C++ semantic index of the MMSP colmap sources. The index would
let Riltra store proven relations and so resolve the gates that the syntax layer
cannot. Which scope is worth its cost?

## 2. What the index would have to provide

| Gate | Proven relation needed | Source truth |
|---|---|---|
| D1-C6 | An edge at `patch_match.cc:667` to the definition of `PatchMatch::Problem::Print` (`patch_match.cc:118`). | B.1 |
| D3-C5, hop 2 | A `CALLS` edge at `patch_match.cc:671` (`patch_match.Run()`) to the definition at `patch_match.cc:191`. | B.3 |
| D3-C5, hop 3 | A `CALLS` edge at `patch_match.cc:197` to the `PatchMatchCuda::Run` declaration (`patch_match_cuda.h:54`) or its definition (`patch_match_cuda.cu:1326`). | B.3 |
| D3-C5, hop 1 | Stays a candidate. The address-of dispatch at `patch_match.cc:268, 292, 309` is never `CALLS` (amendment Q4). | B.2, B.3 |
| D2-C6 | A proven edge to the class constructor (`patch_match.cc:219`) at each of the three construction sites. These sites are CUDA/HIP-guarded (§3). | B.2, amendment §7.2 |
| D4-C5 | Proven rows at the read, binding, print and validation sites. The filter site needs local dataflow and stays unprovable (amendment Q2). | B.4 |

For D1-C6 and the class-level gates, the SCIP descriptor grammar encodes the
type scope (`<type>#`) and the method suffix (`<method>().`), so
`PatchMatch#Problem#Print().` and `PatchMatchOptions#Print().` are distinct symbols
when the indexer emits them (Context7 `/scip-code/scip`, `docs/scip.md`). Whether a
given indexer emits them is an open point for the spike.

## 3. Facts established (read-only)

**Host.** `scip-clang`, `clangd`, `clang` and `clang++` are not on PATH. `cmake`
(miniconda), `ninja` and `nvcc` 12.4 are present. The CUDA 12.4 headers are present
(`cuda_runtime.h`). WSL is **not installed** (`wsl --install` is required). No
Linux runtime exists on this host.

**Indexer.** Facts from the official README (`https://github.com/sourcegraph/scip-clang`,
fetched 2026-10-25). Context7 does not index scip-clang, so Context7 was not used for
this tool.

- License Apache-2.0. Status Beta.
- Prebuilt binaries for x86_64 Linux (glibc 2.16 or newer) and arm64 macOS only.
  Windows is not listed. Upstream status for Windows is unverified.
- Input: a JSON compilation database (`--compdb-path`, usually `compile_commands.json`).
  Output: `index.scip`.
- Scratch about 2 MB per translation unit; about 2 GB RAM per core.
- CUDA: needs the CUDA SDK and a `clang` on PATH. The README's version statements
  conflict ("Clang 16 or newer" against "based on Clang 21"). A dry run must settle
  which applies.

**Riltra import path (existing components).**

- `riltra semantic import --repo <r> --scip <file> [--source-digest <v>]`
  (`crates/cli/src/args.rs:335-352`).
- Provenance gate (`crates/cli/src/edges.rs:52-64`): a non-ephemeral provider, a
  non-empty artifact digest, a `references` capability, and a concrete text
  encoding. The trust root is `import_file`.
- Budgets (`crates/cli/src/scip.rs:47-55`): 64 MiB of input, 100,000 documents,
  5,000,000 occurrences, 200,000 per document, nesting 32, 512 MiB decoded.
  The size of a colmap-scale index is unknown. An import above 64 MiB aborts, and
  raising it would be a product change.
- The ephemeral `lsp-subprocess` provider yields zero stored edges
  (`crates/cli/src/edges.rs:396-400`). clangd through the existing adapter therefore
  cannot feed `callers`. Persisting LSP facts would be a new pipeline (clangd pilot
  §8.3), which the constraints forbid now.

**CUDA and the translation units.**

- `patch_match.cc` contains no `COLMAP_CUDA`, `COLMAP_HIP` or `__CUDACC__` guard
  (checked: zero matches). It includes `colmap/util/cuda.h` and
  `patch_match_cuda.h`, which includes `cuda_to_hip.h`.
- `cuda_to_hip.h:136-138` includes `<cuda_runtime.h>` and `<curand_kernel.h>` only
  under `COLMAP_CUDA_ENABLED` or `__CUDACC__`. So the D1 and D3 translation unit
  parses without the macro and without CUDA headers. Defining the macro pulls the
  CUDA headers into the parse.
- The D2 construction sites are CUDA/HIP-guarded (amendment §7.2): `exe/mvs.cc`
  260-278, `automatic_reconstruction.cc` 429-440 (CUDA only), `dense_reconstruction_widget.cc`
  394-398 (MVS and CUDA or HIP), `pycolmap/pipeline/mvs.cc` 33-44.
- `COLMAP_CUDA_ENABLED` is appended only when `CUDA_ENABLED AND CUDA_FOUND`
  (`colmap/cmake/FindDependencies.cmake:314, 347, 399-400`). `CUDA_ENABLED` defaults to
  ON (`colmap/CMakeLists.txt:40`).

**Dependencies.** Eigen, glog, Ceres, Boost, FLANN and FreeImage are not installed
(clangd pilot §10). A real configure fetches dependencies over the network
(`FetchContent`), which is not authorized. The reconstructed database of the
pilot defined no macros and no dependency paths, so its answers were partial. The
dense widget is a Qt GUI translation unit (B.2). The `pycolmap` binding is
pybind11-based (not verified on this host).

## 4. Options

| Option | Scope | Unlocks (to be measured) | Does not unlock | Prerequisites | Main costs and risks |
|---|---|---|---|---|---|
| **A. Narrow spike** | `patch_match.cc` and the translation units its identity needs. Reconstructed database, no CUDA or HIP macro. | D1-C6 identity at `:667`; D3 hops 2 and 3 (possibly). | D2 (guarded sites); D4 (filter needs dataflow, validation needs macro expansion). | A Linux runtime (WSL install, or a Linux host); the scip-clang Linux binary; a scratch database; a scratch `RILTRA_HOME`. | WSL install and distribution (a system change and a download). Indexer download. Database construction effort. Indexing time and index size (unknown; measure one unit first). Partial dependencies may leave identity unresolved. |
| **B. MVS subtree with CUDA macro** | A, plus `exe/mvs.cc`, `automatic_reconstruction.cc`, the dense widget, and `pycolmap`. | D2 constructor sites (if the constructor is emitted). | D4 filter. | A, plus CUDA headers (toolkit 12.4 present), include paths in the database, and an indexer that parses the CUDA 12.4 headers. | Qt and pybind11 headers (not installed; unverified). Heavy parse-error load. Index size. Clang version conflict in the README. |
| **C. Real colmap build** | All colmap sources with the real configuration. | The most complete resolution. | — | All dependencies, a network fetch, the CUDA toolkit. | Not estimated. Network fetch is not authorized. Not recommended now. |
| **D. Defer** | None. | Nothing. | All four gates stay UNMET, with the documented limits. | None. | None. |

Rejected: clangd through the existing LSP adapter as a source of stored edges. It
yields zero stored edges (`edges.rs:396-400`). Persisting LSP facts is a new
pipeline, which the constraints forbid.

## 5. Recommendation

1. **Approve Option A as a bounded spike with a go or no-go gate.** Run it only
   after the approvals in §6.
   - **Go** if the indexer runs on one translation unit in the approved Linux
     runtime, the index imports under the budgets in §3, and D1-C6 either gains a
     proven edge at `:667` or the failure is recorded with its cause.
   - **No-go** stops the spike. The result is recorded and Option D stands.
2. **Do not approve B or C** until the spike reports its measured time, index size
   and parse-error count. Each would need its own decision.
3. **Keep the gates UNMET** until the spike produces a source-grounded result. No
   gate changes from a candidate or a partial parse.

## 6. Approvals requested before any action

1. A Linux runtime: install WSL and a distribution (a system change and a
   download), or name a Linux host to use.
2. The scip-clang Linux binary (a download). If it is built from source, its build
   dependencies are a separate approval.
3. A reconstructed compilation database for the Option A translation units, in a
   scratch directory outside the repository and the MMSP tree.
4. A scratch `RILTRA_HOME` for the import. The default registry is not touched. The
   database and the index are not committed.
5. A time and disk budget. The spike stops if one translation unit exceeds the
   agreed time, or if the index exceeds 64 MiB.
6. A licence record. scip-clang is Apache-2.0. If it becomes a repository
   dependency, `docs/licensing.md` needs an entry in the same change (`CLAUDE.md`).
   If it stays an external tool, the spike note records it.

## 7. Acceptance criteria for the spike

- The indexer exits 0 on the database subset and writes `index.scip`. Its time and
  size are recorded.
- `riltra semantic import` exits 0 under the budgets in §3.
- D1-C6 is re-evaluated. It passes only with a proven edge at `:667` to the
  anchored definition, source-grounded. Otherwise it stays UNMET, with the cause.
- No other gate changes unless a proven relation supports it.
- `git status --porcelain` is empty in the MMSP tree before and after, and nothing
  is written to the repository.
- The grader suite stays green.

## 8. Out of scope

- Persisting clangd facts, or any new graph feature.
- Installing the CUDA toolkit or the Qt and pybind11 headers (Option B).
- A full dependency build (Option C).
- Any product budget change, unless the owner decides it after the spike measures
  the index.
- Committing any database, index or external binary.

## 9. Open points

- The indexer's Clang version (the README conflicts) and its Windows status. Check
  the upstream issue tracker during the spike.
- Whether the indexer emits type-scoped descriptors for methods. The spike checks
  this on `PatchMatch::Problem::Print`.
- CUDA headers with the indexer's Clang (Option B only).

## 10. Sources

- scip-clang README, `https://github.com/sourcegraph/scip-clang` (fetched 2026-10-25).
- SCIP descriptor grammar, Context7 `/scip-code/scip`, `docs/scip.md`.

## 11. Revision 2026-10-25 (spike)

Recorded after the dry run. Details and status: [m9-cpp-semantic-index-spike-2026-10-25.md](m9-cpp-semantic-index-spike-2026-10-25.md).

- §3 (indexer): the claim that clang is needed only for CUDA is wrong for this indexer. Its driver needs `clang++` to find the resource directory on ordinary C++ too. The dry run failed on the missing compiler and on the first missing standard header.
- §3 (import): the SCIP project root from a WSL run is a POSIX URI that Riltra cannot resolve on Windows. The import stops with exit 3 before any candidate is checked. A path bridge is needed (owner decision).
- Status: the spike stopped at the go/no-go gate and waits for approval of the packages and the bridge. All four gates remain UNMET.
