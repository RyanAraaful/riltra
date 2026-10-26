# M9 C++ semantic index spike, Option A (2026-10-25)

**Status: stopped at the go/no-go gate. Postponed by owner decision on 2026-10-25; no further installs or downloads. No semantic evidence reached Riltra. All four capability gates remain UNMET.**

Basis: [m9-cpp-semantic-index-decision-2026-10-25.md](m9-cpp-semantic-index-decision-2026-10-25.md) §5–§7. Owner approval: experiment only, one translation unit (`patch_match.cc`) without CUDA/HIP macros, no production code change, no push.

## 1. Summary

- The scip-clang dry run exited 0, but the translation unit stopped at its first standard-library header. The index is partial. The indexer still reported `num errored TUs: 0`.
- The SCIP import failed with exit 3. The project root written by the WSL run cannot be resolved by Riltra on Windows.
- Two items need new owner approval: dependency packages and a path bridge (§5).
- Incident: the `.git` metadata of both pinned MMSP bench snapshots lost `refs/` and `objects/info/` at 23:25:21. The cause is not identified. Both snapshots were rebuilt from the intact clone (§4).

## 2. Approvals used

| Item | Status | Details |
|---|---|---|
| WSL + distro | Installed | Ubuntu 26.04.1 LTS (`resolute`), x86_64, glibc 2.43. `wsl --install -d Ubuntu --no-launch`, started elevated. Distro download size not recorded. |
| scip-clang Linux | Downloaded | Asset `scip-clang-x86_64-linux`, release `v0.4.1` (published 2026-10-22), 149,130,216 bytes, sha256 `a434d26a079fd80e1996ba79276cb1ef86b2b2ad28b13e24386285db32c59046` (matches). Source: GitHub `sourcegraph/scip-clang` release. Apache-2.0; external tool, not a repository dependency. |
| Compilation database | Created | One entry: `clang++ -std=c++17 -I<WSL-WORK>/mmsp/colmap/src -c colmap/src/colmap/mvs/patch_match.cc`. No CUDA/HIP macros. Outside the repository and the snapshot. |
| Scratch RILTRA_HOME | Used | Temp folder of the user, `riltra-home`. Repository registered as `mmsp`. Default registry not touched. |
| C++ dependencies and compiler | **Not approved** | §5. |
| Time and disk budget | **Not approved** | §5. |

## 3. Results

**Working copy.** `<WSL-WORK>/mmsp`: 2277 files, 26 MB, without `.git`. Its per-file SHA-256 list is identical to the snapshot's (§4).

**scip-clang dry run (one translation unit).**
- Exit 0, indexing 0.2 s, index 226,405 bytes, sha256 `73fb399d4349976093d6db5a8f413e22421316043cbfc6a089df18b6e4c53976`.
- Diagnostics:
  - `driver: scip-clang needs to be invoke 'clang++' ... couldn't find 'clang++' on PATH`
  - `colmap/src/colmap/mvs/mat.h:32:10: fatal error: 'algorithm' file not found`
  - `Finished indexing 1 translation units ... num errored TUs: 0`
- Interpretation: clang stopped at its first fatal error, so the definitions in `patch_match.cc` were never parsed. The exit code and `num errored TUs: 0` are not valid evidence. Next acceptance criterion: zero fatal diagnostics for the unit.

**Include closure** (script, without evaluating `#if`): 37 internal files, no unresolved local include. External includes: `Eigen/Core`, `Eigen/Geometry`, `Eigen/StdVector`, `glog/logging.h`, `boost/preprocessor.hpp`, and four `boost/unordered/*` headers. CUDA/HIP headers also appear because the script does not evaluate `#if`. Their macro guards (memo §3) are not verified here.

**Riltra index of the snapshot** (cold, scratch RILTRA_HOME): exit 0, wall 114 s (stopwatch, includes startup), discovered 2277, processed 1475 (unsupported 798, binary 4), definitions 18,785, imports 6,680, calls 99,720, `files.errors` 2,387, fallback `cold-start`. The memory baseline on the same host is 97–99 s; this run was slower. The cause was not investigated.

**SCIP import.** `riltra semantic import --repo mmsp --scip <file>` exited 3 with `error: project_root does not resolve: 'file:///<WSL-WORK>/mmsp'`. `semantic status` shows `imports: []`. No state changed.
- Root cause: scip-clang writes the WSL working directory as `project_root`. `uri_path` (`crates/cli/src/semantic.rs:240-268`) strips the scheme and returns a path without a drive letter. Windows resolves that against the active drive, and the path does not exist there. The gate fails before any candidate is checked.

**Gates.** D1-C6 and D3 were not re-evaluated because no new semantic edge was imported. All four gates remain UNMET with no verdict change. Grader `node --test docs/pilots/cp2/method/grade.test.cjs`: 52 of 52 pass.

**Riltra binary.** Built from `e911052`. `git diff --stat e911052 HEAD -- crates` is empty, so the crates match HEAD.

## 4. Incident: damaged `.git` metadata in the pinned bench snapshots

- **Found.** `git status` on the pinned snapshot failed with `fatal: not a git repository`. `.git/refs/` and `.git/objects/info/` were missing. The `.git` and `objects` directories had mtime 23:25:21. The second bench snapshot (`rc-after`) had the same change at 23:25.
- **Working tree intact.** The 2277 tracked files of the damaged snapshot matched the WSL copy per file (SHA-256 list identical, 2277 of 2277). The second snapshot matched the same list.
- **Ruled out.**
  - Riltra `init`, `index`, and a failing `semantic import` on a throwaway clone: `refs/` was present after each step.
  - Riltra production code: no `git worktree`, `gc`, `prune`, or repository-root delete. The `worktree` and `remove_dir_all` hits are in test code. Production deletes only staging and backup files under the Riltra home.
  - Grader tests: they write only under the OS temp directory.
- **Not ruled out.** An unidentified process at 23:25:21. It coincides with the second import attempt and the grader run, but neither reproduced it.
- **Action taken.** Both snapshots were rebuilt with `git clone --no-hardlinks` from the local MMSP clone (`fa5f74d`, intact), then `git checkout --detach d62f06a`. The original config values were set (`autocrlf=false`, `filemode=false`, `symlinks=false`, `ignorecase=true`, `longpaths=true`). Result: HEAD `d62f06a`, `git status --porcelain` = 0, per-file SHA-256 identical to the pre-incident list. The damaged copies were renamed `mmsp-d62f06a.damaged-git-refs`, not deleted. The rebuilt `.git` holds the full history of the clone; the original was a shallow fetch.
- **Follow-up.** Check `.git/refs` before and after each step that touches a snapshot. Investigate the trigger if the damage recurs.

## 5. Approvals requested before continuing

1. **Ubuntu `resolute` packages in WSL:** `clang` (scip-clang needs `clang++`), `g++` (C++ standard library headers), `libeigen3-dev`, `libgoogle-glog-dev`, `libboost-dev`. Sizes are unknown: the WSL apt index is empty, and packages.ubuntu.com returned HTTP 500 on two attempts. Sizes appear in `apt-get install -s` after `apt-get update`, which downloads the package index. Proposed stop condition: download above 500 MB.
2. **Path bridge for the import**, choose one:
   - A (proposed): `subst` a drive letter to a scratch folder, then a junction `<alias>\root\m9-spike\mmsp` to the snapshot. The SCIP artifact is not modified. Before importing, the per-file SHA-256 list of the WSL copy must equal the snapshot's. Reversible: `subst <drive>: /D` and remove the junction.
   - B: rewrite `project_root` in the SCIP metadata. Not recommended: it changes the artifact's provenance claim.
   - C: build Riltra for Linux inside WSL. Cleanest and heaviest (Rust toolchain, crates, a C compiler, new downloads).
3. **Budget (proposed):** download at most 500 MB; WSL disk at most 5 GB; indexing one unit at most 10 minutes; index at most 64 MiB; scratch under the user's temp folder.

## 6. Go/no-go

No-go for now. Two conditions from the approval are met: the index does not yet provide valid semantic evidence, and continuing needs more scope (dependency packages). No installs were made after the dry run.

## 7. Corrections to the memo

- Memo §3 says clang is needed only for CUDA. The dry run shows the scip-clang driver needs `clang++` for its resource directory on ordinary C++ too. Recorded in memo §11.
- The memo did not record the SCIP root gate. This is a new blocker, recorded in §3 and §5 here and in memo §11.

## 8. Limits

- One translation unit, no CUDA/HIP, as scoped.
- These results are not evidence for any capability gate, and no gate changed.
- No push, merge, or release. No production code change.
- The matching clang version is undecided. The README says "Clang 16 or newer" and also "based on Clang 21".

## 9. Scratch artifacts (outside the repository, not committed)

- `<WSL-WORK>`: working copy of MMSP, compilation database, `run1/index.scip` (sha256 in §3), dry-run log.
- `<SCRATCH>`: `riltra-home`, a copy of `run1/index.scip`, the rebuilt snapshots, and the renamed damaged copies.

## 10. Sources and checks

- scip-clang README (WebFetch, 2026-10-25): `--compdb-path` flag, output `index.scip`, run from the project root. The README says clang is required only for CUDA. The dry run contradicts this.
- `scip-clang --help` and `--help-all` on the v0.4.1 binary.
- Context7 `/websites/clang_llvm`, generic query on the compilation database format: matches `directory`, `file`, and `arguments` (preferred), with relative paths resolved against `directory`.
- Context7 does not index scip-clang. The tool's source is the official README (WebFetch).
- Ubuntu package sizes not retrieved: packages.ubuntu.com returned HTTP 500.
