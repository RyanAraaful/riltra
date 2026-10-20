# M9 design gate — multi-repo release readiness (MMSP-first pilot)

**Status: APPROVED — CP1 EXECUTED, AWAIT OWNER GO FOR CP2.** (Revision 3,
2026-10-20: synchronized with `origin/master` `a5f67e6`; M9 objective restated
as a usable GitNexus alternative; CP1 pilot spec frozen in
[`m9-cp1-pilot-spec.md`](m9-cp1-pilot-spec.md). Revision 2, 2026-10-19: F1–F4
corrections, G-TOK reconciliation policy, and owner decisions D-M9-1..D-M9-6
recorded in §6.)

This document is a design proposal. It authorizes no implementation, no
checkpoint execution beyond CP1, no benchmark run, no MMSP modification, no
packaging, no merge, no tag, and no release. CP1 was executed on 2026-10-20
under an explicit OWNER GO and produced
[`m9-cp1-pilot-spec.md`](m9-cp1-pilot-spec.md) (documentation and ground truth
only — no code, no measurement). Each remaining checkpoint (CP2–CP4) requires
its own OWNER GO. The `v0.1.0` release itself requires a separate, explicit
OWNER GO.

- Date: 2026-10-19 (Revision 3: 2026-10-20)
- Branch: `m9-multirepo-release-candidate`, created explicitly from
  `origin/master`
- **Current baseline (Revision 3):** `origin/master` ==
  `a5f67e6544f14d77774dfddf49e78ff10acd4966` (PR #6 merged; merge commit
  `a5f67e6`); the M9 branch was synchronized with it by merge commit
  `96a57f8d95686cbe061d1f74780db9c373508e8b` (parents `a097e2d` + `a5f67e6`,
  DCO-signed). The M9 branch HEAD after synchronization is `96a57f8`.
- **Historical original baseline (preserved):** `cc963ae…`
  (`cc963aec1e55ed65cc0628d65309c754177ce270`, PR #3 merged) — the commit the
  M9 branch was created from and the baseline of the M0–M8 evidence below.
- **Objective (Revision 3, owner directive):** deliver `v0.1.0` as a usable,
  local-first alternative to GitNexus for the core code-intelligence workflows
  developers and OpenCode agents need — find a symbol and its references,
  identify callers/callees, trace a call path, analyze the impact of a change,
  and answer natural-language code questions through OpenCode using Riltra MCP
  tools — within the documented support scope. Parity with GitNexus's UI,
  Cypher, automated rename, embeddings and its full MCP tool set is **not**
  required for `v0.1.0`; a full C++/CUDA semantic capability is **not**
  claimed. The objective, the workflow acceptance checklist and the
  release-blocking functional gaps are frozen in
  [`m9-cp1-pilot-spec.md`](m9-cp1-pilot-spec.md) §D and §E.
- Primary pilot (owner decision, fixed): MMSP `RyanAraaful/mmsp`, default
  branch `main`, commit `d62f06a576d8e76b705c8ecc57fc380f7ebc9c44`
- Secondary pilot: Riltra self-indexing — **internal validation only**, never
  independent cross-repository evidence
- Governance applied: [`m8-opencode-first-amendment.md`](m8-opencode-first-amendment.md)
  (OpenCode-first through M10; the G-MCP client-count requirement is revised to
  a real OpenCode interoperability smoke; all other gates, including
  G-LIC..G-SEC, remain in force). The private blueprint is not version
  controlled in this repository and no blueprint text is copied here — the
  same handling as the amendment §4.

Status vocabulary for gates: **PASS / FAIL / BLOCKED / NOT YET VERIFIED**.
Release gates below are evaluated against the eventual release candidate, not
against milestone history. A gate is never PASS until its M9 acceptance test
ran on the release candidate and its evidence is archived. One MMSP pilot is
not sufficient evidence for any entire release gate.

## 1. Verified current state (baseline `cc963ae`; synchronized with `a5f67e6` on 2026-10-20)

### 1.1 Repository and branch state

- **Current (Revision 3, 2026-10-20):** `origin/master` == `a5f67e6…`
  (PR #6 MERGED, merge commit `a5f67e6…`); `m9-multirepo-release-candidate`
  synchronized with `origin/master` by merge commit `96a57f8…` (two parents:
  `a097e2d…` M9 tip + `a5f67e6…` master tip), DCO-signed, no rebase, no
  force-push, no branch deletion. The merge was conflict-free: the M9 branch
  changed only `docs/m9-design-gate.md`, which `master` did not touch.
- **Historical (Revision 2, 2026-10-19):** `origin/master` == expected
  `cc963ae…`; local `master` equal; PR #3 MERGED (GitHub API: state `MERGED`,
  merge commit `cc963ae…`).
- `m9-multirepo-release-candidate` created from `origin/master`; `HEAD` ==
  `cc963ae…`; no history divergence; working tree carries only the
  pre-existing untracked `AGENTS.md` (never staged; not ignored).
- Separate frozen worktree `.cp7a/source` (detached) holds CP7-A evidence and
  must not be touched.

### 1.2 CLI surface and output contract

Commands (`crates/cli/src/args.rs`): `init`, `repo list`, `repo status`,
`doctor`, `scan`, `index [--full]`, `index status`, `search`,
`callers`/`callees`, `trace`, `impact`, `semantic import|status|query
[--lsp-server]`, `knowledge create|show|update|history|search|link|unlink|
related|export|import|backup|restore|verify`, `mcp`, `status`, `--help`,
`--version`. Exit codes `0` / `2` (usage) / `3` (repository/data); JSON is
hand-built, deterministic, key-ordered; goldens under `crates/cli/tests/golden/`
enforce byte stability.

### 1.3 Language and syntax support (current, verified in source)

- Parsed: Python (`.py`), JavaScript (`.js/.mjs/.cjs`), TypeScript
  (`.ts/.mts/.cts`, `.tsx`), Rust (`.rs`), C (`.c`), C++ (`.h/.hh/.hpp/.hxx`,
  `.cc/.cpp/.cxx`). Headers parse as C++ by policy (`syntax.rs:58–60`).
- **CUDA extension support is split and must be reported separately:**
  - extension classification: `.cu/.cuh` → `FileKind::CudaExperimental`
    (`syntax.rs:77`)
  - syntax parsing: **none** — "explicitly experimental/unsupported; never
    force-parsed" (`syntax.rs:53–54`); scan reports `status:"unsupported"` with
    reason `cuda-experimental` (`scan.rs:109`)
  - semantic resolution: **none** — no CUDA-capable provider exists; no CUDA
    semantic claim may be made from this pilot
- Unknown extensions are reported as `unsupported-extension`, never guessed;
  binary/non-UTF-8/oversized files are skipped with explicit reasons.

### 1.4 Durable index, freshness, incremental

- One SQLite (bundled, WAL) database per repository under `<RILTRA_HOME>`;
  code-index schema v4 with snapshot nonce/revision identity; knowledge store
  schema v2; stores are promoted atomically through a staging file.
- Freshness is evaluated against live state with named reasons (`dirty`,
  `head-changed`, `branch-changed`, `parser-changed`, `grammar-changed`,
  `config-changed`, `file-added`, `file-removed`, `content-changed`,
  `changed-during-index`, `root-moved`); anything unverifiable is `unknown`;
  nothing is silently `fresh` (M3 audit closures).
- Incremental refresh reuses digest-verified per-file facts and parses only
  changed files; explicit fallback reasons (`cold-start`, `forced-full`,
  `db-unreadable`, `no-snapshot`, `root-moved`, `parser-changed`,
  `grammar-changed`, `config-changed`, `head-changed`, `branch-changed`,
  `revision-changed`); output carries `mode`, `reused`, `reparsed`, `added`,
  `removed`, `timings_ms` (M4).

### 1.5 Semantic providers

- Provider registry: `scip-import` and `lsp-subprocess` only
  (`semantic.rs:208–221`); unknown providers are explicit unsupported errors.
- Verification is honest by rule: `Unverified` / `Stale`; `Current` is
  unreachable without independent producer-to-source proof
  (`semantic.rs:406–421`).
- LSP subprocess trust boundary: explicit absolute `--lsp-server` opt-in, no
  auto-install, no shell, timeout (30 s), message caps (8 MB / 10k), stderr
  drained, kill + bounded reap, URI containment; `RILTRA_LSP_DEBUG=1` for
  audits (M5).

### 1.6 Relationship graph (callers / callees / trace / impact)

- Syntax facts are candidates only: `POTENTIAL_CALL` is never promoted to
  `CALLS`; semantic facts carry provider/artifact/revision evidence.
- Trace/impact are bounded and conservative: depth 1–6 (default 3), ≤500
  identities, ≤3000 examined edges, ≤1000 paths, cooperative 2000 ms budget;
  disclaimers `plausible-path-not-execution` / `plausible-impact-not-execution`;
  impact verdicts are `UNKNOWN` / `HIGH` / `MEDIUM` (no SAFE/LOW); all impact
  responses are `completeness.status:"partial"` with
  `graph-completeness-unproven` (M6).

### 1.7 Knowledge store

- Revisioned records with generation guards; deterministic canonical JSONL
  export; import; backup + verify; fail-closed restore (M7, closing G-BACK /
  G-PORT / G-PROV); knowledge MCP reads are non-migrating and non-creating.

### 1.8 MCP interface (OpenCode-first)

- `riltra mcp` serves 11 read-only tools over local stdio; reads never create
  or migrate a store (`no-index` / `schema_outdated` / `schema_ahead` /
  `store_missing` classification); CLI/MCP parity is parser-defined and was
  proven 21/21 in CP7-A; the real-OpenCode interop smoke passed (revised G6);
  historical two-client G6 remains BLOCKED/untested (Claude Code deferred to
  post-MVP by the amendment).

### 1.9 Verification assets and CI

- Integration suites: `cli`, `syntax`, `index`, `edges`, `semantic`,
  `knowledge`, `mcp` plus unit tests; CP7-A local gate: 634 passed / 0 failed
  (8 suites); golden fixtures for every language plus the M6 multilingual
  corpus (79 source/manifest pairs, 494 frozen responses across py/ts/rs/c/cpp).
- CI (`.github/workflows/ci.yml`): `windows-latest`, `ubuntu-latest` and
  macOS 15 running fmt / clippy / `check --locked` / `test --locked` +
  CLI help/version/status smoke on every push and PR. The macOS job name is
  `check (macos-15)`; its **runner** is `macos-15-intel`, so the macOS CI is
  **x64** and **macOS ARM64 is not covered** (see the G-NAT correction).
- No packaging, release, SBOM, or benchmark workflow exists yet.

### 1.10 Licensing and security evidence present

- `docs/licensing.md` inventories every M2–M8 addition (tree-sitter + 6
  grammars, rusqlite/bundled SQLite, protobuf reader, serde, rmcp/Tokio/
  schemars) with pinned versions and permissive licenses; the dependency-audit
  test (`crates/cli/tests/semantic.rs:7`) enforces deliberate allowlisting.
- Root `NOTICE` is a placeholder ("no bundled third-party source at this
  stage") that still requires the release-time third-party append.
- Windows WAL concurrency audit (`docs/audits/2026-10-19-windows-wal-concurrency.md`,
  diagnostic) is closed by the accepted minimal fix `d1ce68f` (retry
  `PermissionDenied` ≤3 inspections, 25 ms, Windows-only; persistent denials
  stay fail-closed).

## 2. MMSP as primary pilot (read-only assessment)

### 2.1 Repository state and pinned snapshot strategy

Recorded 2026-10-19, read-only (Git metadata + tracked files only; no builds,
no scripts, no datasets, no `env.local.ps1`):

| Item | Value |
|---|---|
| Remote | `https://github.com/RyanAraaful/mmsp.git` |
| Local HEAD | `d62f06a576d8e76b705c8ecc57fc380f7ebc9c44` |
| Remote HEAD / `origin/main` | identical `d62f06a…` (`git ls-remote` verified) |
| Branch | `main` |
| Working tree | clean (`status --short` empty) |
| Divergence vs `origin/main` | `0 0` |

The local clone must stay byte-untouched through M9. Benchmark runs use an
**isolated pinned snapshot outside both repositories**: a fresh clone of the
pinned commit under a dedicated benchmark workspace (e.g.
`<BENCH_ROOT>\mmsp-<shortsha>`), created with ordinary clone/checkout; MMSP's
own clone is never pulled, checked out, cleaned, stashed, or configured. A
`git worktree` inside MMSP is not used because it would add administrative
metadata to `MMSP/.git`; the isolated clone keeps the "MMSP unchanged"
invariant provable by `status --short` + `rev-parse HEAD` before and after
every run. `RILTRA_HOME` points to `<BENCH_ROOT>\home`, outside MMSP, so
indexing artifacts never land in any MMSP working tree. No MMSP source is
imported into Riltra; no MMSP dataset, build output, or machine configuration
is opened.

**Revision 3 note (2026-10-20):** the table above is the 2026-10-19 record and
is kept as history. The MMSP default branch has since advanced
(`origin/main` == `7c1b1ceb1b067268c9e6b1b2c353f9044a2e3546`) and the local
MMSP clone now sits on a documentation branch (`75b3a3f…`); the local clone was
neither pulled nor checked out for M9. **The pilot pin is unchanged and stays
`d62f06a…`** (owner decision D-M9-1) — a re-pin is an owner decision. The
current snapshot identity, the concrete `BENCH_ROOT` and the frozen inventory
are recorded in [`m9-cp1-pilot-spec.md`](m9-cp1-pilot-spec.md) §A, and were
re-counted at the pin and found to reproduce §2.2 exactly.

The local tree currently contains ignored build/smoke directories
(`colmap/build-mx150/` — including 75 build-tree `.cu` copies — plus
`smoke-test/`, `smoke-test-gpu/`, `.opencode/`). They are recorded for
inventory completeness only, are never read or used, and a clean pinned clone
contains none of them.

### 2.2 Source inventory and language distribution (tracked files)

- 2,277 tracked files: `colmap/` 2,175; `scripts/` 70; `docs/` 20; `tools/` 2;
  root docs/config 10.
- Extensions (top): `.h` 817, `.cu` 487, `.cc` 439, `.py` 114, `.ps1` 75,
  `.inl` 37, `.c` 34, `.cpp` 14 (plus `.cmake` 12, `.cuh` 2, smaller sets).
- `colmap/src/colmap/` modules (files): estimators 109, ui 102, scene 74,
  util 62, mvs 56, controllers 49, geometry 35, feature 34, optim 29.
- CUDA inventory (tracked files, recomputed 2026-10-19 via `git ls-files` at
  `d62f06a`; classification proven by tree paths): total `.cu` **487** =
  third-party **482** + first-party **5**; total `.cuh` **2** = third-party
  **2** + first-party **0**; combined `.cu + .cuh` = **489**. All 482
  third-party `.cu` live under `colmap/src/thirdparty/Symforce-Caspar/`
  (Symforce-Caspar 980 tracked files; the other vendored trees, PoissonRecon
  99 / VLFeat 77, contain no tracked `.cu`). The two third-party `.cuh` are
  generated headers (`Symforce-Caspar/generated/f32/memops.cuh`,
  `.../generated/f64/memops.cuh`). The five first-party CUDA translation
  units: `feature/sift_gpu_native.cu`, `mvs/gpu_mat_prng.cu`,
  `mvs/gpu_mat_ref_image.cu`, `mvs/gpu_mat_test.cu`,
  `mvs/patch_match_cuda.cu`.
- Ignored/untracked build-tree copies are a separate inventory category: 75
  `.cu` copies under `colmap/build-mx150/` — never read or used, and absent
  from a clean pinned clone. The M2 walk's reported 562 `.cu` reconciles as
  487 tracked + 75 build-tree copies (verified via `git ls-files` 2026-10-19);
  a clean snapshot clone carries only the 489 tracked `.cu/.cuh` files.
- `.cu/.cuh` remain **unsupported by Riltra by policy** (see §1.3) — this
  inventory correction does not change that: the pilot must still expect
  **labeled unsupported output** for every tracked `.cu/.cuh` file and must
  not imply CUDA syntax or semantic support.

### 2.3 Build system and compilation database status

- CMake (`colmap/CMakeLists.txt`): `CUDA_ENABLED` default ON with a dedicated
  `colmap_mvs_cuda` target (3 first-party `.cu` + test); HIP alternative
  exists; `SIFTGPU_ENABLED` default OFF (non-permissive license; never
  executed); vcpkg manifest features keyed off the options.
- **No compilation database exists or is configured**: no
  `compile_commands.json` at the repository root or `build/` (checked), no
  `CMAKE_EXPORT_COMPILE_COMMANDS` anywhere in the tree, no `.clangd` file.
  `.clang-tidy` exists (warnings-as-errors profile).
- Consequence for G-SEM: a future C++ semantic provider (e.g. clangd) would
  need a generated compilation database; CMake only exports it with
  Makefile/Ninja generators (Context7-verified: `CMAKE_EXPORT_COMPILE_COMMANDS`,
  CMake ≥3.5, ignored on other generators), and clangd discovers it in parent
  directories and `build/` subdirectories (Context7-verified). This is
  recorded for a potential later proposal only: **D-M9-2 is deferred** — no
  clangd configuration or integration happens in M9 baseline work, and any
  future attempt needs its own evidence plus OWNER GO (§4, G-SEM). Nothing is
  installed or generated in this design task.

### 2.4 Documentation and licensing surface (tracked)

- `README.md`: COLMAP-derived drone photogrammetry engine; V1 scope ends at
  the fused dense cloud; upstream base `bd1fcf6`.
- `MODS.md` (source of truth: modifiers, benchmark tables — photogrammetry
  evidence only, not Riltra evidence); `CONTRIBUTING.md` (scope `mvs/`,
  evidence-before-claims, red-green tests, benchmark reporting contract);
  `LICENSE` (Apache-2.0 for original work); `THIRD_PARTY_NOTICES.md`
  (BSD-3-Clause components incl. COLMAP base; one MPL-2.0 component with a
  source-availability duty; SiftGPU and LSD purged, never shipped).
- The MMSP licensing file records MMSP's own obligations; M9 evaluates Riltra,
  and Riltra bundles nothing from MMSP.

### 2.5 Prior Riltra–MMSP evidence (all at the same commit `d62f06a`)

MMSP has already served as read-only Tier B evidence for earlier milestones;
M9 continues from the identical commit, which keeps the baseline continuous.

- **M2 (syntax, full tree):** 15,176 files discovered; 11,765 processed;
  170,586 definitions; 64,954 imports; 562,384 unresolved call candidates;
  152,543 errors across 5,481 files of which only 55 are first-party;
  2,628 unsupported (all 562 `.cu/.cuh` as `cuda-experimental`); languages
  cpp=11,394 / python=299 / c=52 / ts=18 / js=2; two runs byte-identical;
  debug-build full scans ~179–366 s, peak ~222–325 MB RSS.
- **M3 (full index):** 219.1 s / 214.9 s builds, peak 338–357 MB RSS, database
  81,281,024 bytes; reopen + `index status` ~2.5 s; hand-verified searches hit
  exact spans (`PatchMatch::Problem::Print`, `normalize_vec`,
  `CreateWorkspace`); a mid-validation stale flip was reported honestly.
- **M4 (incremental):** full ~210.8 s; no-op refresh 15.3–15.8 s (~13.6×);
  one-file delta at full scale 15.3 s (~13.5×, `reparsed` 1, `reused` 11,764);
  parity vs independent full rebuild byte-identical (counters + facts);
  isolated 18,057-file / 6.6 GB copy used — MMSP itself porcelain-0 throughout.
- **M5:** MMSP + SCIP at scale explicitly out of scope (semantic pilots used
  small fixtures only).

### 2.6 Candidate ground-truth pool (to be frozen at CP1, verified by eye)

| # | Case | Value |
|---|---|---|
| 1 | Definition lookup: `PatchMatch::Problem::Print`, `normalize_vec`, `CreateWorkspace` | Continuity with M3 hand-verified anchors |
| 2 | Callers/callees across translation units in `mvs/` (`PatchMatch::Run` call sites; `patch_match.cc` ↔ `patch_match_cuda.h/.cu`) | Definition/caller accuracy on real C++ |
| 3 | Header dependencies: `#include "colmap/…"` edges for mvs files | Include graph correctness |
| 4 | Class methods / namespaces / overloads / templates (sample chosen at CP1, e.g. `PatchMatch` methods and one template-heavy estimator) | C++ identity handling |
| 5 | C++/CUDA boundary: calls from `patch_match.cc` into `patch_match_cuda.cu` symbols (`PhotoConsistencyCostComputer`, `ComputeNCCProb`, `SweepFromTopToBottom` — per `MODS.md`, re-verify at CP1) | Cross-language expectation: `.cu` side labeled unsupported; declarations in headers still resolvable from C++ |
| 6 | API/header impact: `max_triangulation_angle` option plumbing (`patch_match_options.h` → `.cc` → `option_manager.cc` → `patch_match.cc`) | Impact and provenance on a real change surface |
| 7 | Ambiguous/unresolved counts and budget cuts on frozen queries | Transparency labeling |
| 8 | Freshness: comment edit, rename, delete, branch switch on the isolated copy | FSH scenario matrix |
| 9 | Trace: one small `mvs/` chain (chosen at CP1) | Trace semantics + evidence |
| 10 | Evidence completeness: every citation resolves; partial labels preserved | Source-grounded claims |

### 2.7 Limits that must stay labeled (not PASS)

- CUDA (`.cu/.cuh`): syntax and semantic support **absent**; every pilot report
  states unsupported counts, never coverage.
- C++ semantic: no provider has ever been pointed at C++/MMSP (M5 scope);
  clangd is deferred (D-M9-2), so C++ semantics stay NOT YET VERIFIED in M9
  and must never be claimed from syntax output; a future attempt requires a
  separate evidence-backed proposal plus OWNER GO.
- Scale ambiguity: MMSP has heavy third-party/generated content; first-party
  error counts are the meaningful signal (55 at M2) and are re-measured, not
  extrapolated.
- Benchmark environment: single-host figures describe that host/config only;
  no universal or competitive claim may be made.

## 3. Proposed benchmark methodology (executable at CP2)

### 3.1 Fixed inputs (freeze before measuring)

- Pinned Git commit `d62f06a…` (or an owner-approved successor) and the
  isolated-snapshot creation procedure from §2.1; MMSP clone re-verified
  `clean` + `rev-parse` equal before and after every run.
- Git worktree state of the snapshot: `status --porcelain` empty at creation;
  edits only ever occur inside the snapshot copy and are reverted/removed
  before any timing run (no mixed state).
- Source file count and language distribution re-counted at CP1 (§2.2 table
  frozen into the CP1 spec).
- Host: OS build, CPU, RAM, filesystem (e.g. NTFS), power setting; last
  recorded CP7-A host (Windows 11 IoT Enterprise LTSC 26100, i5-8250U, 21.3 GiB)
  is a reference point only — CP2 records its actual host.
- Toolchain: rustc/cargo/git versions; build profile recorded per run (M3/M4
  used a debug binary; CP2 repeats that profile for comparability and may add
  one release-profile cold index if the owner wants it).
- Semantic provider availability: record available provider tooling and
  versions (informational only; nothing installed or run; clangd is deferred
  per D-M9-2).
- `RILTRA_HOME` isolated at `<BENCH_ROOT>\home`; registry contains only the
  snapshot repository.

### 3.2 Measurements (each repeated; variance reported)

- Cold index (full): wall time, mode/reason, peak RSS, resulting index size in
  bytes; at least 2 runs (M4 precedent), both reported.
- No-op incremental refresh: time, `reused`/`reparsed` counters.
- One-file incremental delta: time, `reparsed` = 1, fact parity vs the
  pre-edit state.
- Full rebuild comparison after the delta: independent `--full` on the same
  final tree, same configuration, built into a fresh home. **Fact parity is
  the requirement**: definitions, imports, call candidates (names, kinds,
  spans, byte offsets, digests, provenance) and the relevant query outputs
  must match after volatile-field normalization, and the total fact counters
  must be equal on the same basis. The operational counters (`mode`,
  `reused`, `reparsed`) are recorded and validated **per mode** — a full
  rebuild re-parses everything, an incremental run reuses the unchanged
  files — they are not expected to be identical. Any unexplained fact
  difference is a FAIL.
- Warm queries: frozen query list (search, index status, callers, callees,
  trace, impact) with ≥5 repetitions each; report p50/p95 and the exact
  commands; freshness-check cost included and noted.
- Peak memory per phase where observable; index size after each build.
- Isolation assertions: MMSP porcelain-0 and no new files under the MMSP tree
  before/after every measured command; reads create nothing (§G-SEC).

### 3.3 Correctness protocol

- Ground truth frozen at CP1 from §2.6, every anchor hand-verified by reading
  the source; expectations written before measurement.
- Every cited span resolves to the expected construct; `POTENTIAL_CALL` vs
  `CALLS` and `Unverified`/`Stale` labels are checked, never promoted.
- Unsupported, ambiguous, unresolved, and budget-cut counts recorded per
  query set.
- No self-scoring: results are compared against the frozen expectations by a
  human-readable table, not by the tool under test.

### 3.4 Freshness scenarios (pre-registered expectations)

Comment edit in a tracked file; file rename; file delete; branch switch and
HEAD change on the snapshot; each step records the freshness verdict + reasons
and the expected value written in advance. The snapshot is restored (checkout
the pinned SHA) between scenarios.

### 3.5 Explicit exclusions

- No MMSP build, no CMake configure, no CUDA, no photogrammetry pipeline, no
  dataset access, no kernel timing, no ablation scripts.
- No photogrammetry benchmark result may be used as Riltra performance
  evidence; measurements are Riltra timings on MMSP source only.
- No source modifications inside MMSP; no benchmark numbers without archived
  raw logs; no claims beyond the recorded host and configuration.

## 4. M9 acceptance gates

Matrix summary (release-gate status at M9 entry; details per gate below):

| Gate | Focus | Milestone evidence (M0–M8) | Release status at M9 entry |
|---|---|---|---|
| G-LIC | license inventory, SBOM, notices | licensing.md chain; CP7-A G8 PASS | NOT YET VERIFIED |
| G-NAT | native Windows/Linux/macOS | 3-OS CI builds/tests (M1–M8) | NOT YET VERIFIED |
| G-CLI | stable CLI and JSON | goldens; CP7-A G5 PASS | NOT YET VERIFIED |
| G-MCP | OpenCode interoperability | CP7-A revised G6 PASS; G4/G5 PASS | NOT YET VERIFIED |
| G-SYN | source-verified syntax | M2 goldens + MMSP full-tree scan | NOT YET VERIFIED (M9 scope) |
| G-SEM | semantic providers | M5 PASS (rust-analyzer + scip-typescript, two families) | NOT YET VERIFIED |
| G-GRF | graph correctness and safety | M6 PASS (fixtures); CP7-A G2 PASS | NOT YET VERIFIED (MMSP scope) |
| G-FSH | freshness | M3/M4 PASS; MMSP honest staleness | NOT YET VERIFIED (M9 matrix) |
| G-INC | incremental correctness/perf | M4 PASS; MMSP 13.5× delta | NOT YET VERIFIED (M9 procedure) |
| G-KNW | knowledge graph | M7 PASS — CLOSED | NOT YET VERIFIED (release) |
| G-TOK | token/task effectiveness | CP7-A G3 PASS (−30.5%, historical; D-M9-4) | NOT YET VERIFIED (CP4 reuse review) |
| G-SEC | filesystem, isolation, recovery | M1/M7/M8 + WAL audit + fix | NOT YET VERIFIED (release) |
| G-PAR | competitive validity (≥3 of 5 pre-registered dimensions) | — (M10 target) | not an M9 gate |

### G-LIC — License inventory, SBOM, notices

- **Requirement.** Every distributed v0.1.0 artifact has a complete audited
  license inventory; an SBOM exists and matches `Cargo.lock`; `NOTICE` and
  `docs/licensing.md` are accurate; no copyleft / noncommercial / unknown-
  rights content anywhere in the build or runtime path.
- **Existing evidence.** `docs/licensing.md` M2–M8 entries with pinned
  versions; dependency-audit allowlist test; CP7-A G8 PASS (no dependency
  change); `NOTICE` placeholder with explicit release-time append duty.
- **Provable via MMSP.** None — MMSP is an evaluation target; no MMSP content
  enters Riltra. The pilot records that fact as an isolation assertion.
- **Needs Riltra/release artifacts.** SBOM generation (Context7-verified
  `cargo metadata` fields: id/name/version/license/license_file/source),
  lockfile-vs-SBOM consistency check, `NOTICE` append, licensing.md re-check
  at pin time, forbidden-license scan.
- **Gaps.** No SBOM file; stale `NOTICE` text; no release-level license gate
  run. SBOM tool choice deferred (existing tooling preferred; no new build or
  runtime dependency).
- **Acceptance test.** On the release candidate: SBOM generated from the
  locked graph; zero drift vs `Cargo.lock`; every SBOM package has a
  licensing.md entry; `NOTICE` updated; permissive-only audit passes;
  commands + outputs archived.
- **STOP.** Any dependency with a non-allowlisted license, or unresolved
  SBOM/licensing drift.

### G-NAT — Native Windows / Linux / macOS

- **Requirement.** Release binaries build and run natively on Windows, Linux,
  and macOS; a clean-install CLI/MCP smoke passes on each; platform-specific
  behavior is documented (Windows junctions/paths, WAL sidecars; macOS ARM64).
- **Existing evidence.** 3-OS CI fmt/clippy/check/test + CLI smoke on every
  milestone (M6 run 38897944666, M7 39091621405, M8 39157182381 all green);
  platform-split tests (Windows junctions, Unix permissions); macOS CI is
  ARM64.
  **CP3 correction (2026-10-20):** `macos-15` and the earlier `macos-latest`
  are both **ARM64** images, and the ARM64 hosted pool could not supply a
  runner: the macOS job was cancelled after ~15 minutes with **no runner
  assigned and no step executed**, 7 times out of the repository's last 40
  macOS jobs, while the Windows, Ubuntu and security jobs of the same runs
  passed; **no macOS job ever failed a test**. The macOS **runner** is now
  `macos-15-intel` (the same macOS 15 release on the x64 pool); the macOS
  **job name** stays `check (macos-15)` because the `protect-master` ruleset
  requires that status-check context. The move is **verified**: run
  39448700227 ran the macOS job on the `macos-15-intel` runner, it was
  assigned a runner, it executed every step and it passed.
  **macOS ARM64 is therefore no longer
  covered by the CI matrix** and must be restored as a separate,
  non-required workflow once the ARM64 pool recovers, or the native-macOS
  claim must be read as x64-only. Evidence:
  `docs/pilots/cp3/raw/ci/`.
- **Provable via MMSP.** No — this is a Riltra packaging gate; MMSP only runs
  on the pilot host.
- **Needs Riltra/release artifacts.** Release-profile builds per OS; artifact
  naming; clean-install smoke (extract → `--version`/`--help`/`status` →
  `init` + `index` a fixture → MCP initialize); checksums; install notes.
- **Gaps.** No release binary has been built or published; no packaged smoke;
  no checksums or install documentation.
- **Acceptance test.** Three native artifacts produced; each passes the
  clean-install smoke with archived command output; checksums listed.
- **STOP.** A platform fails the smoke, or the build needs unreleased
  OS-specific code paths.

### G-CLI — Stable CLI and JSON

- **Requirement.** Shipped CLI keeps documented commands, exit codes (0/2/3),
  deterministic JSON; goldens stay valid; version reported; changes to
  behavior are deliberate and documented.
- **Existing evidence.** Determinism/exit-code tests (M1); per-language
  goldens (M2); M8 single response-assembly path; CP7-A G5: 19/19 CLI
  captures byte-identical; 634/0 suite; 3-OS CI.
- **Provable via MMSP.** Partially — repeated pilot commands must be
  byte-identical on the frozen snapshot (repeat-run equality), exercising the
  CLI at scale.
- **Needs Riltra/release artifacts.** Release-candidate suite + golden run; a
  short CLI stability/versioning note (pre-1.0 policy) in docs.
- **Gaps.** No written stability policy; no release-candidate run yet.
- **Acceptance test.** `cargo test --locked` and goldens green on the
  candidate; repeated MMSP commands byte-identical; documented policy exists.
- **STOP.** Unjustified golden drift or nondeterministic output.

### G-MCP — OpenCode interoperability (amended scope)

- **Requirement (revised by the owner amendment).** A real OpenCode session
  against the shipped `riltra mcp`: initialize, the 11 tools exposed,
  representative calls, at least one classified error, no stdout pollution,
  read-only guard honored. JSON-RPC validity and no-stdout-logs rules
  unchanged. Claude Code / Codex are post-MVP and explicitly not PASS.
- **Existing evidence.** CP7-A revised G6 PASS (real OpenCode session; error
  surfaced without transport crash; health OK; home unchanged); G5 parity
  21/21; G4 adversarial error paths; frozen archive byte-unchanged; historical
  G6 BLOCKED preserved.
- **Provable via MMSP.** Yes — the M9 interop demonstration runs OpenCode
  queries against the MMSP pilot index (callers/callees/trace/impact),
  in addition to the release-candidate smoke.
- **Needs Riltra/release artifacts.** Release-candidate binary; documented
  OpenCode config; archived smoke transcript.
- **Gaps.** No candidate-level smoke yet; upstream `rust-sdk#1353` remains an
  accepted, unresolved limitation.
- **Acceptance test.** Recorded OpenCode run against the candidate: 11 tools,
  ≥1 successful MMSP query, ≥1 classified error, transport healthy; stdout
  carries only JSON-RPC.
- **STOP.** Transport crash, stdout pollution, or a read tool creating /
  migrating a store.

### G-SYN — Source-verified syntax

- **Requirement.** Scan output is source-grounded (definitions, imports, call
  candidates with spans/byte offsets for supported languages); unsupported
  inputs are labeled, never guessed; errors counted; output deterministic;
  `.cu/.cuh` remain explicitly unsupported.
- **Existing evidence.** M2 per-language goldens + hand-verified MMSP
  full-tree scan at `d62f06a`; byte-identical repeat runs; `.cu` refused as
  `cuda-experimental`; first-party error count 55.
- **Provable via MMSP.** Yes — re-scan the pinned snapshot; hand-verify
  first-party anchors; reconcile unsupported counts against the clean-clone
  inventory (489 tracked `.cu/.cuh`; the M2 walk's 562 included 75
  `colmap/build-mx150/` copies that a clean clone does not have); determinism
  repeats.
- **Needs Riltra/fixtures.** Existing m2/m6 corpora for regression; no new
  languages.
- **Gaps.** Conservative include resolution (not a full resolver); macro
  interiors opaque; C/C++ header ambiguity handled by policy + recovery
  counting; CUDA absent by policy.
- **Acceptance test.** CP2 scan report: deterministic repeats; hand-verified
  anchors from §2.6; unsupported counts exact; first-party errors recorded.
- **STOP.** Fabricated facts, nondeterminism, or any CUDA parse/coverage claim.

### G-SEM — Semantic providers

- **Requirement.** Minimum two semantic providers from different language
  families, at least one of them C/C++ or TypeScript, must carry
  source-grounded definitions/references evidence. Semantic answers come only
  from explicit providers with honest verification (`Unverified`/`Stale`;
  `Current` unreachable), enforced trust boundary, and no claim beyond
  provider coverage. C++ semantics on MMSP are unproven (clangd deferred by
  D-M9-2), CUDA semantics are unsupported, and a syntax graph must never be
  promoted to a verified semantic graph.
- **Existing evidence.** M5 registry + trust boundary + two real-provider
  fixture pilots from different language families: rust-analyzer (Rust) and
  scip-typescript (TypeScript — satisfies the C/C++-or-TypeScript clause)
  with source-grounded definitions/references evidence. MMSP+SCIP at scale
  explicitly out of scope; no C++ provider ever run.
- **Provable via MMSP.** Not for C++ in M9: D-M9-2 defers clangd until
  baseline MMSP correctness evidence demonstrates a justified requirement;
  early CP1/CP2 work focuses on syntax/graph correctness. A proven need later
  requires a separate proposal with evidence and OWNER GO before any clangd
  configuration or integration. Default: syntax-only on MMSP and the C++
  semantic gap recorded as NOT YET VERIFIED.
- **Needs Riltra/release artifacts.** Provider policy at release; tooling
  entries recorded; fallback behavior verified; documented statement that
  MMSP pilot accuracy claims are bounded by provider coverage (no C++ provider
  covers MMSP in M9).
- **Gaps.** C++ semantic unproven on MMSP; CUDA semantic unsupported; semantic
  scale unproven; no compilation database exists in MMSP today.
- **Acceptance test.** The two-provider/two-family minimum (one of them
  TypeScript or C/C++) is re-verified on the release candidate with
  source-grounded definitions/references evidence archived; C++ gaps on MMSP
  are labeled NOT YET VERIFIED with the deferral reason; no syntax candidate
  is promoted under a semantic label. (No clangd acceptance path in M9.)
- **STOP.** Untrusted-server constraints bypassed, silent installs, any
  provider configured without its own OWNER GO, or unverified semantic
  claims.

### G-GRF — Graph correctness and safety

- **Requirement.** callers/callees/trace/impact are source-grounded,
  conservative, bounded, and transparent: no promoted `POTENTIAL_CALL`, no
  fabricated references, explicit disclaimers, budgets enforced, completeness
  `partial` stated.
- **Existing evidence.** M6 tasks 4–6 (+ correction) with the 79-pair
  multilingual corpus and frozen goldens; CP7-A G2 zero unresolved citations
  across 32 answers; conservative `UNKNOWN` impact policy.
- **Provable via MMSP.** Yes — frozen manual ground truth in `mvs/`
  (cross-TU callers/callees, option plumbing impact, one trace chain);
  count ambiguous/unresolved/budget cuts at scale.
- **Needs Riltra/fixtures.** Existing m6 corpus; no new graph engine.
- **Gaps.** C++ macro/template-heavy sites; MMSP-scale accuracy never
  measured; `.cu` boundary observable only from the C++ side.
- **Acceptance test.** Frozen expectations vs results table; every cited span
  resolves; counts for unresolved/ambiguous/cuts; no promotion; budgets
  obeyed on oversized fanout.
- **STOP.** Any promoted potential call, fabricated reference, or
  completeness claim beyond `partial`.

### G-FSH — Freshness

- **Requirement.** Every result carries explicit fresh/stale/unknown with
  reasons; nothing is silently fresh; behavior after edit/rename/delete/
  branch/HEAD changes is documented and tested.
- **Existing evidence.** M3/M4 freshness suites and reason taxonomy; MMSP
  runs demonstrably reported `stale` during churn; dirty semantics tests; MCP
  reads never migrate.
- **Provable via MMSP.** Yes — §3.4 scenario matrix with pre-registered
  expectations on the isolated copy.
- **Needs Riltra/fixtures.** Scenario script (CP1); no code changes expected.
- **Gaps.** M9 matrix unrun; branch-change flow not previously exercised on
  MMSP; expectations must be written first.
- **Acceptance test.** Each scenario matches its pre-registered verdict/reason
  (or reports `unknown`/`stale` honestly); no silent-fresh observed.
- **STOP.** `fresh` reported while inputs differ, or missing reasons.

### G-INC — Incremental correctness and performance

- **Requirement.** Incremental refresh is fact-identical to a full rebuild of
  the same tree, reparses only changed files, falls back explicitly, and is
  measurably faster; numbers are reproducible with recorded host/toolchain.
- **Existing evidence.** M4 parity gate; MMSP numbers (~210.8 s full;
  15.3–15.8 s no-op ≈13.6×; one-file delta 15.3 s ≈13.5×; byte-identical
  counters — the *fact* counters (definitions / imports / call candidates /
  errors), not the operational `reused`/`reparsed` counters, which are
  mode-specific (see §3.2); isolated 18k-file copy; MMSP porcelain 0).
- **Provable via MMSP.** Yes — re-run the M4-style procedure per §3.2 on the
  M9 pin.
- **Needs Riltra/fixtures.** Procedure doc; isolated copy; repetition policy.
- **Gaps.** Prior figures are one host, debug profile; no release-profile
  numbers; no CI benchmark (still none proposed).
- **Acceptance test.** Parity between incremental and full on the final tree;
  `reparsed` limited to the changed set; timings + host/toolchain recorded; no
  speedup claim without measured data.
- **STOP.** Fact divergence, hidden fallback, or extra reparses.

### G-KNW — Knowledge graph

- **Requirement.** Revisioned records with generation guards, deterministic
  export, fail-closed restore, backup + verify; reads are non-mutating; no
  data loss.
- **Existing evidence.** M7 PASS — CLOSED (565-test run; G-BACK/G-PORT/G-PROV
  closures); knowledge MCP read tools with generation guards (M8 CP5).
- **Provable via MMSP.** Partially — optional pilot records documenting MMSP
  benchmark decisions in an isolated M9 home; backup → verify → store loss →
  restore reproducing a byte-identical export.
- **Needs Riltra/release artifacts.** Release-level re-verification of the
  backup/restore flow and docs.
- **Gaps.** None known at capability level; release-level re-check pending.
- **Acceptance test.** Isolated-home backup/restore round trip; deterministic
  export; read tools create/mutate nothing.
- **STOP.** Partial restore, nondeterministic export, or mutating reads.

### G-TOK — Token / task effectiveness

- **Requirement.** Measured tokens per completed task for MCP no worse than
  the CLI arm under frozen rules; no universal or competitive claim.
- **Existing evidence (historical, D-M9-4).** CP7-A G3 PASS, frozen:
  16,764.5 vs 24,134.7 gross per completed task (−30.5%; −36.0% on shared
  tasks); recorded limitations (frozen-rule timing deviation on one pilot
  task, one run per task, free-tier model, single Windows host,
  provider-reported tokens). The CP7-A archive — raw evidence, token rules,
  scores, and rubric — is frozen and must not be edited; it is kept as
  recorded historical evidence.
- **Provable via MMSP.** No — MMSP is a measurement target repository, not an
  agent-efficiency corpus; token measurement is a client experiment in the
  CP7-A style.
- **Release evidence applicability review (CP4).** At CP4, evaluate whether
  changes after M8 materially affect MCP output, task outcomes, or token
  consumption. If the relevant behavior is unchanged, document the rationale
  for relying on CP7-A for release-candidate acceptance. If there is a
  material change, or the evidence is insufficient, mark G-TOK NOT YET
  VERIFIED/BLOCKED and propose a new measurement for OWNER GO. Any approved
  re-run is a new, separately labeled measurement and never replaces CP7-A.
  A milestone-level historical PASS alone is never reported as G-TOK release
  PASS.
- **Gaps.** Single-host/single-model scope; no automation; the CP4
  applicability review itself is still pending.
- **Acceptance test.** Either the CP4 review documents unchanged relevant
  behavior with concrete diff evidence and rationale, or a separately labeled
  frozen-rule measurement exists with archived raw sessions and the CP7-A
  limitations restated.
- **STOP.** Metric substitution, recomputation of frozen results, declaring
  release PASS from milestone history, or extrapolated claims.

### G-SEC — Filesystem, repository isolation, recovery

- **Requirement.** Path confinement (symlink/junction escapes fail);
  repository isolation; read paths create and migrate nothing; corruption and
  recovery behavior explicit; no secrets; WAL sidecar behavior documented;
  Windows transient-denial handling honest and minimal.
- **Existing evidence.** M1 security suite (junctions, symlink retargets,
  chmod, corrupt metadata); M8 read-only store guard; M7 fail-closed restore;
  WAL audit + bounded retry fix `d1ce68f`; adversarial MCP error paths
  (CP6/CP7).
- **Provable via MMSP.** Yes — every pilot run asserts the MMSP clone stays
  porcelain-0 and no files appear under any MMSP tree; `RILTRA_HOME` outside
  the repository; read commands create no stores on a first call.
- **Needs Riltra/release artifacts.** Release security checklist (checksums,
  no secrets, install isolation), documented WAL limitation (M7 L2), and the
  accepted `rust-sdk#1353` note.
- **Gaps.** Release-level verification unrun; WAL sidecar creation by reads
  is an accepted limitation to document.
- **Acceptance test.** Isolation + no-creation assertions archived per run;
  adversarial fixtures (unknown repo, escape paths, corrupt stores) behave as
  classified without panics; no secrets introduced.
- **STOP.** Any write inside MMSP, any store created by a read, or any
  confinement bypass.

### G-PAR — Competitive validity (M10 target, not an M9 gate)

G-PAR is an M10 gate, not an M9 gate. PASS requires superiority in at least
three of five pre-registered measurable dimensions, with no critical
correctness regression: (1) source-grounded correctness; (2) change
freshness; (3) changed-file refresh cost; (4) agent token/task success;
(5) maintainability/deployment burden. M9 does not measure competitors: no
competitor benchmark and no parallel-implementation work is authorized by
this design gate (see §7, "No competitor claims").

## 5. Checkpoint plan (proposal)

Checkpoint count is a proposal. Revise it only against gaps and dependencies
that the evidence actually shows — never to add nominal work. Each checkpoint
requires its own OWNER GO and lands tested, DCO-signed commits with CI green.

- **CP1 — MMSP pilot baseline and corpus specification.**
  Freeze: approved snapshot + isolated-copy procedure, re-counted source
  inventory, the §2.6 ground truth with hand-verified anchors and pre-written
  expectations, the §3.4 freshness matrix, and the §3 measurement procedure.
  No code changes. Deliverable: CP1 pilot spec document. Verification: every
  anchor checked by reading MMSP source; `git diff --check`; CI green.
  STOP if the snapshot differs from the record or an anchor is unverifiable.
  **EXECUTED 2026-10-20** under an explicit OWNER GO; deliverable:
  [`m9-cp1-pilot-spec.md`](m9-cp1-pilot-spec.md) (frozen snapshot, inventory,
  source ground truth, freshness matrix, measurement procedure, workflow
  acceptance checklist, release-readiness matrix, functional gaps). No code
  change, no measurement, no MMSP modification, no pilot query executed.
- **CP2 — MMSP pilot execution and targeted correctness fixes.**
  Execute §3 against the frozen snapshot with archived raw logs; report cold
  index, warm queries (p50/p95), one-file delta, full-rebuild parity, memory
  and index size, correctness vs ground truth, unsupported/ambiguous counts,
  freshness scenarios, and isolation assertions. Fix only defects proven by
  this evidence (minimal, TDD, licensing/audit rules apply). Deliverable: CP2
  benchmark + fix report. Verification: results cross-checked against raw
  logs; parity checks; 3-OS CI green.
  STOP if a fix would require new architecture or new language support.
- **CP3 — Multi-repo and remaining release-gate verification.**
  Riltra self-indexing (labeled internal validation), Python/TS fixture
  adequacy check against the existing 79-pair corpus, and evidence updates
  for G-SYN/G-SEM/G-GRF/G-FSH/G-INC/G-TOK/G-SEC. Additional repositories are
  considered **only** if a real gap is proven, and only with an explicit
  owner add-on decision. Deliverable: CP3 gate-evidence report.
  Verification: every updated status cites archived evidence.
  Simultaneous multi-repo isolation scenario (one isolated `RILTRA_HOME`):
  register the MMSP pinned snapshot and the Riltra pinned snapshot
  **together** and exercise the existing registry/query capability only — no
  new cross-repository relationship feature, no third repository. Acceptance:
  both repositories appear in the registry; search uses the correct target
  repository; callers/callees/trace/impact return no cross-repository
  leakage; freshness verdicts are independent per repository; index/database
  and generation bindings are never swapped between repositories; the
  original MMSP and Riltra working trees stay unchanged; every query result
  carries its repository identity and matching evidence.
- **CP4 — Packaging and release readiness.**
  Native builds + checksums, clean-install CLI/MCP smokes, SBOM + NOTICE +
  licensing re-check, documented language support matrix and known
  limitations, security verification record, final acceptance evidence pack.
  `v0.1.0` release (merge/tag/publish) remains a separate OWNER GO. Deliverable:
  CP4 release-candidate evidence pack.
  Verification: G-LIC/G-NAT/G-CLI/G-MCP/G-SEC acceptance tests run on the
  actual candidate.

## 6. Owner decisions (approved)

Recorded 2026-10-19; explicitly approved by the owner. These decisions
supersede the earlier open-decisions list, change no M8 milestone status, and
leave every M9 release gate at NOT YET VERIFIED until its own acceptance
evidence exists.

| Decision | Approved |
|---|---|
| D-M9-1 — Pilot pin | Pin MMSP `d62f06a…` for CP1 (continuity per §2.5). |
| D-M9-2 — C++ semantic provider | DEFER clangd: baseline syntax/graph correctness first; a later proven need requires a separate evidence-backed proposal + OWNER GO before any configuration or integration. |
| D-M9-3 — Benchmark environment | One host, ≥2 cold-index runs, variance reported (M4 precedent). |
| D-M9-4 — G-TOK evidence | Keep CP7-A G3 as recorded historical evidence, with the CP4 release evidence applicability review (§4, G-TOK); frozen CP7-A archive untouched. |
| D-M9-5 — Additional repositories | No additional repository unless a real gap is proven and the owner approves an add-on. |
| D-M9-6 — Packaging scope | Native per-OS archives + checksums + notices; no new installer or signing infrastructure. |

## 7. Risks and limitations (carried and new)

- **Biggest unproven area:** C++ semantics and CUDA. No provider covers them
  today; `.cu/.cuh` stay labeled unsupported; pilot reports must not imply
  otherwise.
- **Data scope:** MMSP third-party/generated content dominates error counts;
  only first-party numbers are meaningful and are always reported with the
  third-party split.
- **Environment variance:** single development host; debug-profile baselines
  from M3/M4; report scoped claims only.
- **Moving target:** MMSP upstream is actively developed; the pin makes
  results reproducible, and any re-pin is an owner decision.
- **Carried M8 limitations:** frozen G1 exception wording, G3 documented
  deviation, `rust-sdk#1353`, Windows WAL accepted behavior, CP7-B deferral
  (Claude Code/Codex untested — not PASS).
- **Licensing discipline:** any new tool (clangd, SBOM tool) is tooling, not
  bundled code; it still gets recorded, and no GPL/AGPL content may enter the
  build or runtime path.
- **No competitor claims:** nothing in M9 measures competitors; no
  superiority claim may be made without a valid comparative benchmark (M10).

## 8. Evidence references

- `docs/m0-verification.md`, `docs/m1-verification.md` — CLI/registry/security.
- `docs/m2-validation.md` — syntax baseline; MMSP full-tree metrics.
- `docs/m3-validation.md`, `docs/m4-validation.md` — index, freshness,
  incremental; MMSP performance and parity numbers.
- `docs/m5-validation.md` — semantic providers; trust boundary.
- `docs/m6-validation.md`, `docs/m6-task4..7-*.md` — graph, trace, impact,
  multilingual corpus; closure record.
- `docs/m7-validation.md` — knowledge store closure.
- `docs/m8-design-gate.md`, `docs/m8-opencode-first-amendment.md`,
  `docs/m8-final-acceptance-review.md`, `docs/m8-owner-acceptance.md` —
  M8 gates, amendment, acceptance.
- `docs/m9-cp1-pilot-spec.md` — CP1 deliverable: frozen MMSP pilot input,
  manually verified source ground truth, pre-registered freshness matrix,
  frozen measurement procedure, GitNexus-replacement workflow acceptance
  checklist, release-readiness matrix and functional gaps.
- `docs/pilots/cp7a/` — frozen benchmark archive (must not be edited).
- `docs/licensing.md`, `NOTICE` — licensing inventory and release duty.
- `docs/audits/2026-10-19-windows-wal-concurrency.md` — Windows WAL audit;
  fix `d1ce68f`.
- Context7 checks (2026-10-19): tree-sitter-cpp node types
  (`function_definition`, `class_specifier`, `call_expression`); SQLite WAL
  `-wal`/`-shm` sidecar semantics (read-only rules); clangd compilation
  database discovery (parents + `build/`, `compile_commands.json`); CMake
  `CMAKE_EXPORT_COMPILE_COMMANDS` (≥3.5, Makefile/Ninja generators only);
  `git worktree add --detach` vs isolated clone for pinned snapshots; `cargo
  metadata` package fields for SBOM input.
