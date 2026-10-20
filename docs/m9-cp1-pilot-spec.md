# M9 CP1 — MMSP pilot spec: frozen input, source ground truth, acceptance tests

**Status: CP1 EXECUTED — AWAIT OWNER GO FOR CP2.** Executed 2026-10-20 on
`m9-multirepo-release-candidate` at `96a57f8` (merge of `origin/master`
`a5f67e6` into the M9 branch), per the checkpoint plan in
[`m9-design-gate.md`](m9-design-gate.md) §5.

CP1 is documentation and ground-truth preparation. **No production code was
changed, no measurement was executed, and no pilot query was run.** Every
anchor below was read directly from the pinned MMSP source at `d62f06a`;
no Riltra output was used as its own ground truth.

- Branch: `m9-multirepo-release-candidate`
- Design gate: [`m9-design-gate.md`](m9-design-gate.md) (Revision 3)
- Pilot repository: `RyanAraaful/mmsp` (public)
- Pilot pin (owner decision D-M9-1, unchanged): `d62f06a576d8e76b705c8ecc57fc380f7ebc9c44`
- CP2 authorization: **not granted**; this document only freezes the inputs

Status vocabulary used below: **FROZEN** (value fixed and re-verifiable),
**PENDING** (defined now, executed in CP2), **NOT YET VERIFIED** (no
evidence exists yet).

---

## A. Frozen pilot input

### A.1 Snapshot identity (FROZEN)

| Item | Value |
|---|---|
| Remote | `https://github.com/RyanAraaful/mmsp.git` |
| Pinned commit | `d62f06a576d8e76b705c8ecc57fc380f7ebc9c44` |
| Tree object | `c5482ea00481675e0aa921538f560947b68dca12` (identical in the original MMSP clone and in the isolated snapshot) |
| Commit author | `RyanAraaful <terapetralabs@gmail.com>` |
| Commit date | 2026-10-14 02:39:41 +0700 |
| Commit subject | `duel: optimization 2 no-op (quadratic overlap) + 3 verdict (97.7% points parity at 1.58x wall)` |
| Checkout state | detached HEAD at the pin, `git status --porcelain` empty |
| Local clone HEAD (never checked out) | `75b3a3f5e89b080de3356a3fae07d572d0ee9cf8` (docs branch) |

The pin is a real ancestor of the MMSP default branch: `d62f06a` →
`2b1bf42` (squash-merge parent) → `7c1b1ce` (`main` HEAD). The pin is
**intentionally not** the MMSP `main` tip; re-pinning is an owner decision
(D-M9-1).

### A.2 Isolation procedure (FROZEN, reproducible)

The pilot snapshot is an **isolated clone outside both repositories**. The
original MMSP clone is never pulled, checked out, cleaned, stashed or
configured, and no MMSP source, dataset, build output or machine
configuration is read or imported.

```
BENCH_ROOT   = <a benchmark workspace outside both repositories>
SNAPSHOT     = $BENCH_ROOT/mmsp-d62f06a
RILTRA_HOME  = $BENCH_ROOT/home      # index artifacts never land in any MMSP tree
```

`BENCH_ROOT` is a dedicated benchmark workspace outside both repositories (on
the CP1 host: a temporary workspace directory). The concrete host path is
deliberately not recorded in version control, so this document stays free of
local absolute paths; the procedure below fixes *how* the snapshot is created
rather than *where* it lives.

```powershell
git clone https://github.com/RyanAraaful/mmsp.git $SNAPSHOT
git -C $SNAPSHOT checkout --detach d62f06a576d8e76b705c8ecc57fc380f7ebc9c44
```

**No protection override is used, and none is required.** CP1 audited the
earlier claim that a plain `git clone` fails and that
`GIT_CLONE_PROTECTION_ACTIVE=false` is needed. It does not reproduce: on
2026-10-20 a plain `git clone https://github.com/RyanAraaful/mmsp.git`
completed with exit 0 and produced a clean clone (`HEAD` = `7c1b1ce…`,
`git status --porcelain` empty). The same plain clone of
`RyanAraaful/riltra` also succeeded, so the earlier rationale — "a repo with
`core.hooksPath` set breaks a plain clone" — is not reproducible either:
`core.hooksPath` is a **local** config (`.git/config`) and is not propagated
by a clone. The variable is not a documented Git or `gh` setting (checked
against the Git and GitHub CLI manuals on 2026-10-20). The workaround is
therefore removed: weakening clone protection is not a default here, and no
step in this procedure needs it. If a future host does require it, the
override must be scoped to that single command, justified with the exact
error output, and recorded here — never set as a session-wide default.

The snapshot in use was produced by exactly this procedure: a plain clone plus
`checkout --detach` of the pin, with tree object
`c5482ea00481675e0aa921538f560947b68dca12` — byte-identical to the tree the
earlier clone produced and to the tree of the pinned commit in the original
MMSP repository.

A `git worktree` inside MMSP is **not** used, because it writes administrative
metadata into `MMSP/.git`. The isolation invariant is proven, not assumed:
before and after every CP2 run, the original MMSP clone must report an empty
`git status --porcelain` and an unchanged `git rev-parse HEAD`.

The Riltra repository's own frozen CP7-A worktree `.cp7a/source` (detached at
`0fd22a1`) is CP7-A evidence and was not touched by CP1.

### A.3 Source inventory (FROZEN, re-counted 2026-10-20 via `git ls-files` at the pin)

| Item | Value |
|---|---|
| Tracked files | **2 277** |
| `colmap/` | 2 175 |
| `scripts/` | 70 |
| `docs/` | 20 |
| `tools/` | 2 (`tools/metrics.cpp`, `tools/parsecheck.ps1`) |
| Root files/dirs | 10 |

Top extensions: `.h` 817, `.cu` 487, `.cc` 439, `.py` 114, `.ps1` 75,
`.inl` 37, `.svg` 37, `.txt` 34, `.c` 34, `.md` 34, `.rst` 23, `.ts` 18,
`.sh` 17, `.cpp` 14, `.cmake` 12, `.m` 10, `.glsl` 10.

These counts reproduce the design gate §2.2 inventory exactly, confirming the
pin and the clean-clone condition (the M2 walk's 562 `.cu` included 75
build-tree copies under `colmap/build-mx150/` that a clean clone does not
contain).

### A.4 First-party vs third-party split (FROZEN)

| Category | Files |
|---|---|
| Third-party (`colmap/src/thirdparty/**`) | **1 157** |
| — `Symforce-Caspar` | 980 |
| — `PoissonRecon` | 99 |
| — `VLFeat` | 77 |
| — `colmap/src/thirdparty/CMakeLists.txt` | 1 |
| First-party (everything else) | **1 120** |

Third-party content dominates the tree, so **only first-party numbers are
meaningful signals**; every CP2 report must state both numbers.

### A.5 Supported and unsupported inputs (FROZEN, against Riltra's current classifier)

Riltra's classifier (`crates/cli/src/syntax.rs:62-80`, verified against the
current source at `96a57f8`): `.py` → Python; `.js/.mjs/.cjs` → JavaScript;
`.ts/.mts/.cts` → TypeScript; `.tsx` → Tsx; `.rs` → Rust; `.c` → C;
`.h/.hh/.hpp/.hxx` and `.cc/.cpp/.cxx` → C++ (headers parse as C++ **by
policy** — the rationale is the comment at `syntax.rs:58-61`, the mapping is
`syntax.rs:75-76`); `.cu/.cuh` → `CudaExperimental`, "explicitly
experimental/unsupported; never force-parsed" (the variant at `syntax.rs:54`,
the policy comment at `:53`, the mapping at `:77`); everything else →
`UnsupportedExtension` (the variant at `syntax.rs:55`, the catch-all arm at
`:78`).

| Class at the pin | Files | Detail |
|---|---|---|
| Riltra-parsed extensions | **1 438** | `.h` 817, `.cc` 439, `.py` 114, `.c` 34, `.ts` 18, `.cpp` 14, `.js` 2 |
| CUDA, **unsupported** | **489** | 487 `.cu` (482 third-party + 5 first-party) + 2 `.cuh` (both third-party, generated) |
| Other extensions, unsupported | **350** | `.ps1` 75, `.svg` 37, `.inl` 37, `.txt` 34, `.md` 34, `.rst` 23, `.sh` 17, `.cmake` 12, `.m` 10, `.glsl` 10, `.json` 9, … |

The five first-party CUDA translation units (all unsupported, all in `mvs/` or
`feature/`): `colmap/src/colmap/feature/sift_gpu_native.cu`,
`colmap/src/colmap/mvs/gpu_mat_prng.cu`, `colmap/src/colmap/mvs/gpu_mat_ref_image.cu`,
`colmap/src/colmap/mvs/gpu_mat_test.cu`, `colmap/src/colmap/mvs/patch_match_cuda.cu`.

**Expected CP2 reporting:** every `.cu`/`.cuh` file is reported with an
unsupported status and a reason; **no CUDA definition, reference or call may
be claimed**, and no coverage percentage may be presented for those 489
files.

### A.6 Toolchain and host (FROZEN for the CP1 host; CP2 records its own)

| Item | Value |
|---|---|
| OS | Microsoft Windows 11 IoT Enterprise LTSC, build 26100 |
| CPU / RAM | Intel Core i5-8250U @ 1.60 GHz / 19.9 GiB (OS-reported `TotalPhysicalMemory`; the design gate's CP7-A record quotes 21.3 GiB for the same CPU — two measurements of the same host, not two hosts) |
| Filesystem | NTFS |
| git | 2.45.1.windows.1 |
| rustc / cargo | 1.99.1 (58cf8f9c3 2026-10-13) / 1.99.1 (5f94df478 2026-08-27) |
| bash (hooks) | GNU bash 5.2.26(1) (msys) |
| gitleaks (repo scanner, not the pilot) | 8.30.1, pinned |
| Build profile | debug, as in the M3/M4 precedent (CP2 may add one release-profile cold index only with owner approval) |

The MMSP tree has **no compilation database** and **no `.clangd` file**: no
`compile_commands.json` at the root or in `build/`, no
`CMAKE_EXPORT_COMPILE_COMMANDS` anywhere in the tree. A C++ semantic provider
therefore cannot be configured without a separate owner-approved proposal
(D-M9-2, unchanged).

### A.7 Measurement procedure (FROZEN for CP2, not executed here)

Fixed inputs: the snapshot of §A.2 at the pinned SHA, `RILTRA_HOME` outside
every repository tree, a registry containing only the snapshot.

```powershell
$env:RILTRA_HOME = "$BENCH_ROOT/home"
riltra init  --repo $SNAPSHOT --name mmsp-pin
riltra index --repo mmsp-pin --full        # cold index, >= 2 runs (D-M9-3)
riltra index --repo mmsp-pin               # no-op refresh
riltra index status --repo mmsp-pin
```

Per run, record: wall time, `mode`/`reason`, `reused`/`reparsed`/`added`/`removed`
counters, peak RSS, index size in bytes. Warm queries use the frozen query
list of §B with ≥5 repetitions each, reporting p50/p95 and the exact
commands. Every measured command is bracketed by the isolation assertions of
§A.2.

**Incremental vs full-rebuild parity contract.** A one-file delta is measured
on the snapshot, then an independent `--full` rebuild of the **same final
tree** is built into a **fresh home** and compared. The contract is:

1. Both runs use the same source tree and the same configuration.
2. **Fact parity is required.** The indexed facts must be equivalent:
   definitions, imports, call candidates (names, kinds, spans, byte offsets,
   digests, provenance) and the relevant query outputs (search, callers,
   callees, trace, impact) after volatile-field normalization. This mirrors
   the M4 parity gate (`docs/m4-validation.md`: "files, definitions, imports,
   unresolved call candidates (names, kinds, spans, byte offsets, digests,
   provenance) — plus CLI search-output parity after volatile-field
   normalization") and the in-repo guard test
   `incremental_matches_independent_full_rebuild`
   (`crates/cli/src/index.rs:2812-2997`, "incremental must equal full").
3. The **total fact counters** are compared on the same basis — e.g.
   `files.definitions`, `files.imports`, `files.calls`, `files.errors`,
   `files.processed`, `files.discovered`.
4. The **operational counters** (`mode`, `reused`, `reparsed`, `added`,
   `removed`, `fallback_reason`) are recorded and validated **according to
   each mode**, not forced to be identical. A full rebuild re-parses
   everything (`mode:"full"`, `reused:0`, `reparsed` = files processed —
   `index.rs:340-363`); an incremental run reuses the unchanged files
   (`mode:"incremental"`, `reparsed` = changed files only —
   `index.rs:664-709`). The M4 evidence records exactly this split: the
   delta reported `reused` 11,764 / `reparsed` 1 while the full oracle
   re-parsed the tree, and the *fact* counters were the ones that were
   byte-identical (170,586 / 64,954 / 562,384 / 152,543).
5. Any **unexplained fact difference** is a FAIL. An operational-counter
   difference that is explained by the mode is not a difference at all.

**Freshness scenarios (§C) run inside the snapshot only**, and the snapshot is
restored to the pinned SHA between scenarios.

---

## B. Manually verified source ground truth (FROZEN)

Every anchor below was read in the pinned source. "Column" is the 1-based
column of the identifier, matching the `--path/--line/--col` anchor grammar of
the CLI and MCP tools.

### B.1 Definitions and references — representative C++ symbols

**Anchor B1 — `PatchMatch::Problem::Print` (C++ method, member of a nested struct)**

| | |
|---|---|
| Declaration | `colmap/src/colmap/mvs/patch_match.h:74:10` — `void Print() const;` inside `struct Problem` (`:57`) inside `class PatchMatch` (`:55`), namespace `colmap::mvs` |
| Definition | `colmap/src/colmap/mvs/patch_match.cc:118:27` — `void PatchMatch::Problem::Print() const {` |
| References | **exactly one** call site: `colmap/src/colmap/mvs/patch_match.cc:667:11` — `problem.Print();` inside `PatchMatchController::ProcessProblem` (defined `:533`) |

Expected factual answer: the symbol is a `const` member function of the
nested `Problem` struct of `colmap::mvs::PatchMatch`; it has one
declaration, one out-of-line definition, and exactly one call site in the
same translation unit.

Evidence: the source lines quoted above; the definition body is
`patch_match.cc:118-128` and ends at `:128`. The **complete** set of
`.Print()` call sites in `colmap/src/colmap` is three, established by
`git grep -n "\.Print()" -- 'colmap/src/colmap/*'` over the pinned tree,
which returns exactly `fusion.cc:144`, `patch_match.cc:667` and
`patch_match.cc:668`: `patch_match.cc:667` (this symbol),
`patch_match.cc:668` (`patch_match_options.Print();` →
`PatchMatchOptions::Print`, defined `patch_match_options.cc:45`) and
`fusion.cc:144` (`options_.Print();` → `StereoFusionOptions::Print`,
declared `fusion.h:100`, called from `StereoFusion::Run()` at
`fusion.cc:137`). The latter two are **different** symbols and must not be
conflated with `PatchMatch::Problem::Print`.

Riltra must report: the definition with its exact span.

- **Required (honesty gate).** The three call sites are reported as
  *candidates* whose terminal name is `Print`, each labeled
  `POTENTIAL_CALL` / `precision:"Heuristic"` / `verification:"Unverified"`
  — never as three proven references to `PatchMatch::Problem::Print`.
  Riltra's syntax layer matches by terminal name and has no C++ overload
  resolution (§B.9), so all three sites are indistinguishable to it.
- **Expected limitation.** A pilot answer that lists all three call sites
  as `Print` candidates, and states that the method identity cannot be
  resolved without a semantic provider, is the **correct** answer. So is an
  answer that names only `patch_match.cc:667` **provided** it does not
  claim the other two do not call anything named `Print`.
- **FAIL conditions.** Reporting `fusion.cc:144` or `patch_match.cc:668`
  as a reference to `PatchMatch::Problem::Print` (wrong citation /
  fabricated relation); reporting a `POTENTIAL_CALL` as a verified `CALLS`
  edge; or presenting the candidate set as a complete, resolved reference
  list for one specific method.

**Anchor B2 — `CreateWorkspace` (C++ function with internal linkage)**

| | |
|---|---|
| Definition | `colmap/src/colmap/mvs/patch_match.cc:54:28` — `std::unique_ptr<Workspace> CreateWorkspace(` |
| Linkage | inside an **anonymous namespace** opened at `patch_match.cc:50` and closed at `:111` → internal linkage, not declared in any header |
| References | exactly three call sites, all in the same file: `:255:18` (pyramid coarse workspace), `:278:18` (full-res workspace + pyramid seeds), `:323:18` (`PatchMatchController::ReadWorkspace`, defined `:320`) |

Expected factual answer: `CreateWorkspace` is a file-local helper of
`colmap/src/colmap/mvs/patch_match.cc`; it has no external visibility and
exactly three call sites, all in that file. A sibling helper
`UpsamplePyramidSeeds` (`patch_match.cc:80`) lives in the same anonymous
namespace and is called once, at `:284`.

Evidence: the anonymous-namespace block spans `patch_match.cc:50-111`; the
three call sites were enumerated by `git grep -n "CreateWorkspace"` over the
pinned tree, which returned only `patch_match.cc:54, 255, 278, 323` — the
definition plus exactly three call sites, no occurrence anywhere else in the
repository. The sibling helper `UpsamplePyramidSeeds` is defined at
`patch_match.cc:80` and, by the same enumeration, called exactly once, at
`patch_match.cc:284`.

Riltra must report: the definition and the three references. It must **not**
present `CreateWorkspace` as a cross-file or public symbol, and must not
attribute any reference outside `patch_match.cc`. If a cross-file reference
is reported, that is a **FAIL** (fabricated relation). If the symbol is
absent from the index because the extractor skips anonymous namespaces, that
is an **acceptable, honestly-labeled** limitation — but it must be reported as
unsupported/unknown, not silently dropped.

**Anchor B3 — `normalize_vec` (Python; the design gate listed it beside C++ symbols, it is not one)**

| | |
|---|---|
| Definition | **1** — `colmap/benchmark/reconstruction/evaluation/geometry.py:34:5` — `def normalize_vec(vec: npt.NDArray[np.floating], eps: float = 1e-10) -> npt.NDArray[np.floating]:` |
| Body | `geometry.py:37` — `return vec / max(eps, float(np.linalg.norm(vec)))` |
| Call expressions | **5**, on **4** source lines — `geometry.py:43:31` (`normalize_vec(vec1)`) and `geometry.py:43:52` (`normalize_vec(vec2)`), both on the same line inside `vec_angular_dist_deg`; `colmap/benchmark/reconstruction/evaluation/geometry_test.py:38:22`, `:44:22`, `:51:22` |
| Import references | **2** — `colmap/benchmark/reconstruction/evaluation/geometry_test.py:32:23` (`from .geometry import normalize_vec, vec_angular_dist_deg`) and `colmap/benchmark/reconstruction/evaluation/utils.py:57:23` (the same import, re-exported with `# noqa: F401`) |

Expected factual answer: a module-level Python function in
`colmap/benchmark/reconstruction/evaluation/geometry.py`, defined at line 34,
with **5 call expressions** on **4 lines** and **2 import references** — that
is **7 reference facts** on **6 source lines**, plus the definition line.

The three counts are different quantities and must not be conflated:

| Quantity | Value | Meaning |
|---|---|---|
| Textual occurrences of `normalize_vec` | **8** | Every occurrence of the identifier in the tree, spread over the 7 source lines of the table above (1 definition + 4 call lines + 2 import lines) |
| Source-line locations | **7** | 1 definition + 4 call lines + 2 import lines |
| Call expressions | **5** | 4 call lines, because `geometry.py:43` contains **two** calls |
| Import references | **2** | `geometry_test.py:32`, `utils.py:57` |
| Reference facts (calls + imports) | **7** | 5 + 2 |

`1 definition + 5 calls + 2 imports = 8` textual occurrences, so every
occurrence in the tree is accounted for by exactly one of those three roles.

Evidence: read directly from the pinned files.
`git grep -n "normalize_vec"` over the whole tree returns exactly the 7 lines
listed above, and counting the identifier per line yields the 8 occurrences
(1 + 2 + 1 + 1 + 1 + 1 + 1). **There is no C++ `normalize_vec` anywhere in
the repository** — the identifier exists only in these three Python files
under `colmap/benchmark/reconstruction/evaluation/`.

Riltra must report: the definition with the file/line, and the references
including the import sites.

- **Required (honesty gate).** The reference count is **7 facts**: 5 call
  facts (`POTENTIAL_CALL`, heuristic) and 2 import facts. The two calls on
  `geometry.py:43` are two distinct facts with distinct spans
  (`43:31` and `43:52`) that happen to share one line; a tool that reports
  one fact for that line has under-counted and is wrong.
- **Expected limitation.** A pilot answer that reports 5 call candidates and
  2 import references over 6 lines, and states that Python import
  relationships are recorded as import facts rather than resolved
  definitions, is the **correct** answer.
- **FAIL conditions.** Reporting a C++ `normalize_vec` (fabricated
  definition); reporting 4 call facts where 5 call expressions exist
  (under-count caused by the two calls on one line); or reporting the
  identifier's total occurrence count (8) as the reference count.

### B.2 Cross-file callers and callees around `PatchMatch`

`PatchMatchController` (constructor defined `patch_match.cc:219`) is
constructed in three other translation units:

| Caller site | Anchor | Context |
|---|---|---|
| `colmap/src/colmap/exe/mvs.cc:271` | `mvs::PatchMatchController controller(options, workspace_path, workspace_format_lower, pmvs_option_name, config_path);` | CLI entry point; `controller.Run();` at `:277` |
| `colmap/src/colmap/controllers/automatic_reconstruction.cc:431` | `mvs::PatchMatchController patch_match_controller(` | automatic reconstruction pipeline |
| `colmap/src/colmap/ui/dense_reconstruction_widget.cc:397-398` | `std::make_unique<ControllerThread<mvs::PatchMatchController>>(std::make_shared<mvs::PatchMatchController>(...))` | Qt GUI |

Callee chain inside `colmap/src/colmap/mvs/patch_match.cc`:

| Hop | Callee | Definition | Call site |
|---|---|---|---|
| 1 | `PatchMatchController::Run()` | `:233:28` | `exe/mvs.cc:277` (and the two other TUs) |
| 2 | `PatchMatchController::ProcessProblem()` | `:533:28` | **indirect** — `thread_pool_->AddTask(&PatchMatchController::ProcessProblem, this, …)` at `:268`, `:292`, `:309` |
| 3 | `PatchMatch::Run()` | `:191` | `:671:15` — `patch_match.Run();` |
| 4 | `PatchMatchCuda::Run()` | `colmap/src/colmap/mvs/patch_match_cuda.cu:1326` (**CUDA, unsupported**) | `:197:22` — `patch_match_cuda_->Run();` |

Expected factual answer: `PatchMatchController` has three external
construction sites; `PatchMatch::Run()` has exactly one call site
(`patch_match.cc:671`); `PatchMatchController::ProcessProblem` is dispatched
through a `ThreadPool` member-function pointer, not called directly;
`PatchMatchCuda::Run()` is called from `patch_match.cc` but **defined** in an
unsupported `.cu` file.

Evidence: the table above was read from the pinned sources; the
`PatchMatchController` constructions were enumerated with
`git grep -rn "PatchMatchController" -- 'colmap/src/colmap/*'`, which also
shows the declaration in `patch_match.h:129` and the class comment in
`colmap/src/colmap/controllers/undistorters.h:40`.

Riltra must report: the three external construction sites as references to the
class; the `ProcessProblem` dispatch as a **candidate/heuristic edge at
best** — the `&PatchMatchController::ProcessProblem` token is an address-of
reference, not a call, so a syntax-level tool cannot observe the invocation.
Reporting hop 2 as a verified `CALLS` edge is a **FAIL** (fabricated
relation). Reporting it as `POTENTIAL_CALL`, or omitting it while stating the
indirect-dispatch limitation, is a PASS. Hop 4 must be reported with the
definition site in the unsupported `.cu` file labeled as such, never as a
resolved C++ definition.

### B.3 Trace path in `mvs/`

The frozen trace chain (source order, each hop verified):

```
PatchMatchController::Run()            patch_match.cc:233
  └─(ThreadPool::AddTask, :309)→ PatchMatchController::ProcessProblem()   patch_match.cc:533
       └─(:671)→ PatchMatch::Run()                                          patch_match.cc:191
            └─(:197)→ PatchMatchCuda::Run()      patch_match_cuda.cu:1326  [UNSUPPORTED: .cu]
```

Expected factual answer: a 4-hop chain from `PatchMatchController::Run` to
`PatchMatchCuda::Run`, where hop 1→2 is an indirect thread-pool dispatch and
hop 3→4 terminates in a CUDA translation unit that Riltra does not parse.

Evidence: `patch_match.cc:233` (`Run`), `:268/:292/:309` (three
`AddTask(&PatchMatchController::ProcessProblem, …)` dispatches), `:533`
(`ProcessProblem` definition), `:670-671` (`PatchMatch patch_match(...);
patch_match.Run();`), `:191` (`PatchMatch::Run` definition), `:196-197`
(`std::make_unique<PatchMatchCuda>(options_, problem_);
patch_match_cuda_->Run();`), and `patch_match_cuda.cu:1326`
(`void PatchMatchCuda::Run() {`).

Riltra must report: a path that ends at the CUDA boundary, with the
unsupported terminus labeled and the plausibility disclaimer
(`plausible-path-not-execution`) present. A path that claims to reach a
device-side definition, or that omits the disclaimer, is a **FAIL**. If the
thread-pool hop is missing, that is acceptable **only** if the answer says
the dispatch is indirect; silently dropping the hop while presenting a
complete-looking chain is a **FAIL** (fabricated path).

### B.4 Impact analysis — `max_triangulation_angle`

The full option-plumbing chain (every hop read in the pinned source):

| Role | Location | Anchor |
|---|---|---|
| Declaration (member of `struct PatchMatchOptions`, `patch_match_options.h:37`) | `colmap/src/colmap/mvs/patch_match_options.h` | `:56:10` — `double max_triangulation_angle = -1.0f;` |
| CLI option binding | `colmap/src/colmap/controllers/option_manager.cc` | `:1018-1019` — `AddDefaultOption("PatchMatchStereo.max_triangulation_angle", &patch_match_stereo->max_triangulation_angle);` |
| Owner of the bound object | `colmap/src/colmap/controllers/option_manager.cc` | `:85` — `patch_match_stereo = std::make_shared<mvs::PatchMatchOptions>();` |
| Print | `colmap/src/colmap/mvs/patch_match_options.cc` | `:58:15` — `PrintOption(max_triangulation_angle);` in `PatchMatchOptions::Print()` (`:45`) |
| Validation | `colmap/src/colmap/mvs/patch_match_options.cc` | `:91-93` in `PatchMatchOptions::Check()` (`:77`) — `CHECK_OPTION_LE(max_triangulation_angle, 180.0f)` and `CHECK_OPTION_LE(min_triangulation_angle, max_triangulation_angle)` |
| Behavioral read | `colmap/src/colmap/mvs/patch_match.cc` | `:361-366` in `PatchMatchController::ReadProblems()` (`:342`) — `has_max_triangulation_angle`, `max_triangulation_angle_rad` |
| Behavioral effect | `colmap/src/colmap/mvs/patch_match.cc` | `:443-447` — the source-image filter `angle >= min_triangulation_angle_rad && (!has_max_triangulation_angle || angle <= max_triangulation_angle_rad)` |

Expected factual answer: changing the default or the value of
`max_triangulation_angle` affects the **automatic (`__auto__`) source-image
selection** in `PatchMatchController::ReadProblems` only. A negative value
disables the upper bound (the option is documented as "A negative value
disables the upper bound", `patch_match_options.h:52-56`). It does **not**
change the geometric consistency term, the NCC likelihood, or the
photometric cost.

Two subtleties that a correct answer must get right: the CLI option name is
`PatchMatchStereo.max_triangulation_angle` while the C++ type is
`mvs::PatchMatchOptions` (the "PatchMatchStereo." prefix is a CLI naming
convention, not a type), and the option is only consulted when
`max_triangulation_angle >= 0`.

Riltra must report: the declaration site, the option binding, the read site
and the filtering site, with the "affects `__auto__` source selection only"
qualification. An impact verdict of `UNKNOWN` with the affected files listed
is **acceptable** (M6 policy forbids SAFE/LOW verdicts). An impact answer
that claims a small or safe blast radius, that omits
`option_manager.cc:1018-1019`, or that claims the geometric consistency path
is affected, is a **FAIL** (false-safe impact result). The two macro sites
must not be resolved to function definitions:
`patch_match_options.cc:58` is `PrintOption(max_triangulation_angle);` and
`patch_match_options.cc:92` is `CHECK_OPTION_LE(max_triangulation_angle,
180.0f);`. Both are **macro invocations**, not calls to a function named after
the option, and both macros are opaque to a syntax extractor.

**Macro hazard (re-verified at CP1).** The macro `PrintOption` is defined
**four times** in the pilot, each in a different translation unit with a
different body — `fusion.cc:78`, `patch_match.cc:45`,
`patch_match_options.cc:43`, `texture_mapping.cc:1034` (established by
`git grep -n "#define PrintOption"`, which returns exactly these four). The
site that matters for this option is `patch_match_options.cc:58`, whose macro
is defined at **`patch_match_options.cc:43`** — *not* at `patch_match.cc:45`,
which is a different macro with a different body in a different file.
Resolving either macro invocation to a function definition is a **FAIL**
(fabricated relation).

### B.5 Header and C++/CUDA boundaries

| Item | Location | Fact |
|---|---|---|
| C++ TU includes the CUDA header | `colmap/src/colmap/mvs/patch_match.cc:34` | `#include "colmap/mvs/patch_match_cuda.h"` — parseable C++ header |
| CUDA class declaration | `colmap/src/colmap/mvs/patch_match_cuda.h:49` | `class PatchMatchCuda { … }`, `Run()` declared at `:54:8` |
| CUDA definitions | `colmap/src/colmap/mvs/patch_match_cuda.cu` | `Run()` `:1326`, `RunWithWindowSizeAndStep()` `:1416`, `ComputeCudaConfig()` `:1567`, `BindRefImageTexture()` `:1584`, `InitRefImage()` `:1597`, `InitSourceImages()` `:1614`, `InitTransforms()` `:1724`, `InitWorkspaceMemory()` `:1840`, `Rotate()` `:1904` |
| Device-side identifiers | `patch_match_cuda.cu` | `PhotoConsistencyCostComputer` `:454`, `ComputeNCCProb` `:741`, `SweepFromTopToBottom` kernel `:945` — **all three exist only in the `.cu` file**; none appears in any header |
| Header guard asymmetry | `colmap/src/colmap/mvs/patch_match.h:36` and `:127` (the only two `#ifndef __CUDACC__` guards in the file, closed at `:39` and `:159`) | The first guard hides `colmap/util/base_controller.h` and `colmap/util/threading.h`; the second hides the whole `PatchMatchController` class — from an nvcc compilation |

Expected factual answer: the C++/CUDA boundary is crossed only in the
declaration direction. The C++ side can see `PatchMatchCuda`'s public
interface (from the header) but can never see its implementation, and the
device-side identifiers are invisible from C++ entirely. The same header
yields a **different** symbol set depending on the compiler, because the
`__CUDACC__` guards hide `PatchMatchController` from CUDA translation units.

Riltra must report: the header edge `patch_match.cc → patch_match_cuda.h` as
an import, the `PatchMatchCuda` declaration as a C++ definition, and the
`.cu` file as unsupported with no definitions extracted from it. Any CUDA
definition, any device-side identifier (`SweepFromTopToBottom`,
`ComputeNCCProb`, `PhotoConsistencyCostComputer`) presented as a resolved
Riltra definition, or any claim of CUDA coverage, is a **FAIL** (fabricated
relation / unsupported claim).

### B.6 Ambiguous and unresolved relationships

| # | Situation | Ground truth | Required Riltra behavior |
|---|---|---|---|
| 1 | Bare name `Run` | **Four** distinct `Run()` methods in `mvs/`: `StereoFusion::Run` (`fusion.h:118`), `PatchMatch::Run` (`patch_match.h:84`), `PatchMatchController::Run` (`patch_match.h:136`), `PatchMatchCuda::Run` (`patch_match_cuda.h:54`) | Multiple distinct candidate identities with distinct locations. Returning one arbitrary `Run` as "the" `Run`, or merging them into one symbol, is a **FAIL** (wrong citation) |
| 2 | `CreateWorkspace` | Internal linkage, TU-local (§B.1/B2) | No cross-file attribution; reference set exactly 3, all in `patch_match.cc` |
| 3 | `&PatchMatchController::ProcessProblem` | Address-of a member function; the invocation happens inside `ThreadPool` | At most a `POTENTIAL_CALL` candidate; never `CALLS` |
| 4 | `PrintOption(max_triangulation_angle)` | Macro expansion; the macro body is opaque to a syntax extractor | Either not reported as a call, or reported as an unresolved macro reference with the macro labeled. Resolving it to a function definition is a **FAIL** |
| 5 | Three distinct `Print()` methods share one bare name | `PatchMatch::Problem::Print` (`patch_match.h:74`), `PatchMatchOptions::Print` (`patch_match_options.cc:45`), `StereoFusionOptions::Print` (`fusion.h:100`) | Each call site must resolve to its own method: `patch_match.cc:667`, `patch_match.cc:668`, `fusion.cc:144` respectively. Merging them into one symbol, or mis-attributing a call site, is a **FAIL** (wrong citation) |
| 6 | Template-heavy code | `colmap/src/colmap/estimators/cost_functions/reprojection_error.h` declares **13** templates, at lines 50, 61, 147, 216, 260, 311, 343, 388, 425, 432, 441, 448 and 455 (the last one a template-template parameter, `template <template <typename> class CostFunctor, typename... Args>`) | Template identity preserved (parameterization visible); call targets inside templates reported as candidates; no claim of overload resolution |
| 7 | Anonymous-namespace helper | `UpsamplePyramidSeeds` (`patch_match.cc:80`), one call site `:284` | Definition reported with file-local scope; no external attribution |

For every one of these, the honest-but-partial answer is a PASS; the
unreliable answer (a confident, wrong or fabricated relation) is a FAIL. The
distinguishing property is whether the answer's claims are supported by the
source it cites.

### B.7 Header include edges (representative `mvs/` case)

`colmap/src/colmap/mvs/patch_match.h` includes
`colmap/mvs/depth_map.h`, `colmap/mvs/image.h`, `colmap/mvs/normal_map.h`,
`colmap/mvs/patch_match_options.h`, and — under `#ifndef __CUDACC__` —
`colmap/util/base_controller.h` and `colmap/util/threading.h`.
`colmap/src/colmap/mvs/patch_match.cc` includes `colmap/mvs/patch_match.h`,
`colmap/math/math.h`, `colmap/mvs/consistency_graph.h`,
`colmap/mvs/patch_match_cuda.h`, `colmap/mvs/workspace.h`,
`colmap/util/cuda.h`, `colmap/util/file.h`, `colmap/util/hash_containers.h`,
`colmap/util/misc.h`, `colmap/util/threading.h`.

Expected factual answer: the include graph of these two files, exactly as
listed — **9** `#include` directives in the header, of which **6** are
project includes (4 unconditional, 2 under the `__CUDACC__` guard) and 3 are
standard-library; and **12** in the implementation unit, of which **10** are
project includes and 2 are standard-library.

Riltra must report the include edges with the target paths; include
resolution is deliberately conservative (design gate §4 G-SYN "Gaps"), so an
unresolved or conservatively-resolved include must be **labeled**, not
guessed. Reporting an include edge that does not exist in the source, or
resolving an include to a wrong file, is a **FAIL**.

### B.8 Evidence completeness

Every answer produced against this corpus must carry, for each cited fact: the
repository identity, the repository-relative path, the line and column, the
freshness verdict with its reasons, and the edge kind (`CALLS` vs
`POTENTIAL_CALL`, `Unverified`/`Stale` vs semantic verification). A citation
that does not resolve to the expected construct in §B is a FAIL, and an
answer without a freshness verdict is a FAIL (no silent freshness).

### B.9 What a syntax-only index can and cannot attribute

This section is the root cause behind the acceptance criteria in §D, and it
is read from Riltra's own current source, not assumed.

**What the syntax layer records.** Definitions, imports, and **calls** —
and a call is recorded only from a call expression
(`crates/cli/src/syntax.rs:400-429` `push_call`, dispatched from the
`call_expression` arms at `syntax.rs:499-538`). A call fact stores the whole
callee text plus a *terminal* span: the identifier after the last `.`, `::`
or `->` (`syntax.rs:436-469`).

**What edges are derived.** `derive_syntax_edges`
(`crates/cli/src/edges.rs:167-205`) builds only two kinds of syntax edge:
`IMPORTS` (precision `Exact`) and `POTENTIAL_CALL` candidates
(`precision:"Heuristic"`, `verification:"Unverified"`, `callee: null`,
`unresolved:"no-overlap"` when the target is not proven to overlap).
A `CALLS` or `REFERENCES` edge requires a semantic provider
(`scip-import` / `lsp-subprocess`); the MMSP pilot has **none** (D-M9-2,
§E1).

**Consequence 1 — data members have no incoming edges.** A member *read*
(`options_.max_triangulation_angle`), an address-of binding
(`&patch_match_stereo->max_triangulation_angle`) and a macro argument
(`PrintOption(max_triangulation_angle)`) are not call expressions. The
symbol `max_triangulation_angle` therefore has **no** reverse edge in a
syntax-only index, and an impact anchored at its declaration has no path to
traverse.

**Consequence 2 — call sites are matched by terminal name.** `Print`,
`Run` and `PatchMatchController` are matched as bare terminal names, so
distinct methods that share a name, overloads, and types passed as template
arguments are indistinguishable to the syntax layer. The `callee` of a
syntax edge is `null`; the target is never resolved to a definition.

**Consequence 3 — the honest verdict is `UNKNOWN`.** `impact_verdict`
(`crates/cli/src/edges.rs:1377-1406`) returns `UNKNOWN` for any non-`Exact`
or unresolved edge, `HIGH` only for proven `CALLS`, `MEDIUM` only for proven
`REFERENCES`, and never `SAFE`/`LOW`.

**Frozen evidence of that shape (CP7-A, do not edit).**
`docs/pilots/cp7a/goldens-raw/impact-validate-id.json` records
`"paths":[]`, `"covered":0,"total":0`, `verdict:"UNKNOWN"`,
`completeness.status:"partial"` and the unresolved reasons
`insufficient-impact-evidence`, `syntax-anchor-uncertain`,
`empty-result-not-coverage-proof`, `unresolved_target`;
`docs/pilots/cp7a/goldens-raw/callers-validate-id.json` records four
`POTENTIAL_CALL` edges with `precision:"Heuristic"`,
`verification:"Unverified"`, `callee: null`, `unresolved:"no-overlap"`.

**Therefore, for the pilot:** the honest answer to workflow 4 is an empty
or partial impact with an `UNKNOWN` verdict, and the honest answer to
workflows 1–2 is a name-matched candidate set that cannot separate overloads
or member references. These are the **expected** pilot outputs. They are the
evidence for the gaps in §E1 and §E7, not tool defects, and not grounds for
lowering the release requirement: the release requirement is stated
separately in §D as the *capability gate*, and §D records that the
capability gate is currently **UNMET** on a syntax-only pilot.

---

## C. Freshness scenario matrix (pre-registered, PENDING execution in CP2)

Each scenario runs **inside the snapshot only**; the snapshot is restored to
`d62f06a` between scenarios. Expected verdicts are written before execution
and are never adjusted afterwards.

| # | Scenario | Exact action on the snapshot | Pre-registered expected verdict and reasons | Riltra must NOT |
|---|---|---|---|---|
| F1 | File modification | Edit `colmap/src/colmap/mvs/patch_match_options.h` (change the default of `max_triangulation_angle` from `-1.0f` to `60.0f`) | `stale` with reason `content-changed`; `dirty` is also expected because the working tree is modified | report `fresh`, or report the old default as current |
| F2 | File rename | `git mv colmap/src/colmap/mvs/patch_match_cuda.h colmap/src/colmap/mvs/patch_match_cuda_renamed.h` | `stale` with reasons `file-removed` **and** `file-added` | silently keep the old path resolvable as fresh |
| F3 | File deletion | `git rm colmap/src/colmap/mvs/patch_match_options.cc` | `stale` with reason `file-removed` | report the deleted file's symbols as available |
| F4 | HEAD change | `git checkout --detach 2b1bf425f46939c6007d182a2009065334a8e78e` (a later MMSP commit, present in the snapshot clone) | `stale` with reason `head-changed` | report `fresh` |
| F5 | Branch change | `git checkout -b m9-freshness-probe` — the HEAD commit stays `d62f06a`, only the branch name changes | `stale` with reason `branch-changed` (and **not** `head-changed`, because the commit is unchanged) | report `fresh`, or report `head-changed` for a pure branch switch |

The reason vocabulary and its semantics are fixed by the current source
(`crates/cli/src/index.rs:2628-2643` for `root-moved`/`dirty`/`head-changed`/
`branch-changed`, and `:2490-2539` for `content-changed`/`file-added`/
`file-removed`): the full set is `dirty`, `head-changed`, `branch-changed`,
`parser-changed`, `grammar-changed`, `config-changed`, `file-added`,
`file-removed`, `content-changed`, `changed-during-index`, `root-moved`;
anything unverifiable is `unknown`. A scenario that reports `fresh` while the
inputs differ is an immediate **STOP** for that run (design gate §4 G-FSH STOP
condition).

---

## D. GitNexus-replacement acceptance checklist (FROZEN, executed in CP2)

Riltra may be described as a practical GitNexus alternative for `v0.1.0` only
if it demonstrably completes the five workflows below **within its
documented support scope**. Parity with GitNexus's UI, Cypher, automated
rename, embeddings or its full MCP tool set is **not** required. A natural
language question is interpreted by OpenCode using the Riltra MCP tools; no
integrated LLM is required inside Riltra.

### D.0 Common failure conditions (apply to all five workflows)

Every workflow is judged in two layers.

- **Honesty gate** — what the answer must do to be *trustworthy*: no
  fabrication, correct repository identity, resolvable citations, freshness
  stated, verified calls distinguishable from candidates. This layer is
  judgeable on the syntax-only pilot; it is the PASS/FAIL that CP2 records.
- **Capability gate** — what the answer must *cover* to count as a practical
  GitNexus alternative for that workflow. **This layer is not lowered.** It
  is the release requirement, and on the syntax-only pilot it is expected to
  be **UNMET** where the coverage needs C++ identity or member-reference
  resolution (§B.9, §E1, §E7). CP2 records the unmet gate as *evidence for
  the owner decision*; an unmet gate never becomes a PASS, and it is never
  "fixed" by reporting less.

A workflow **FAILS the honesty gate** if any of the following occurs:

- a relation is reported that the cited source does not contain
  (fabricated relation);
- an impact or trace result is presented as safe/complete when the evidence
  is partial (false-safe impact);
- a citation points to a line/column that does not contain the cited
  construct (wrong citation);
- a result mixes facts from two different repositories, or omits the
  repository identity (cross-repository contamination);
- a heuristic edge is presented as a verified `CALLS` edge;
- freshness is not stated, or `fresh` is reported while the inputs differ.

Common honesty-gate requirements for every workflow: the answer names the
repository it answered for; every cited fact resolves to the pinned source;
a freshness verdict with reasons is present; verified calls and
`POTENTIAL_CALL` candidates are distinguishable; and the answer states what
it could not resolve instead of implying completeness.

### D.1 Workflow 1 — Find a symbol and its references

*Representative user question:* "Where is `PatchMatch::Problem::Print`
defined in the MMSP project, and where is it used?"

Expected operations: `riltra search --repo mmsp-pin --symbol Print` (or
`riltra_code_search` with `query`), then `riltra callers --repo mmsp-pin
--path colmap/src/colmap/mvs/patch_match.cc --line 118 --col 27` (or
`riltra_code_callers`).

Objective criteria:

*Honesty gate (PASS required).*

1. The search returns the definition at `patch_match.h:74` and/or
   `patch_match.cc:118:27`, with a resolvable anchor.
2. The reference candidates are labeled `POTENTIAL_CALL` / heuristic; no
   candidate is presented as a proven reference to one specific method.
3. A search for `normalize_vec` returns the Python definition
   `colmap/benchmark/reconstruction/evaluation/geometry.py:34`; no C++
   `normalize_vec` is reported. Its reference set is **7 facts** — 5 call
   facts (4 lines, because `geometry.py:43` holds two calls) and 2 import
   facts — and not the 8 textual occurrences (§B.1 B3).
4. A search for the bare name `Run` returns **multiple** distinct `Run`
   identities with distinct locations, not one merged symbol.
5. The repository identity and a freshness verdict are present.

*Capability gate (release requirement; expected **UNMET** on the pilot).*

6. The answer distinguishes `patch_match.cc:667` from `patch_match.cc:668`
   and `fusion.cc:144` — i.e. it resolves the C++ method identity of the
   single true reference (§B.1). §B.9 explains why a syntax-only index
   cannot: it matches the terminal name `Print`, so all three sites are
   indistinguishable to it (gap E7). The gate stays as stated; the pilot is
   expected not to meet it, and workflow 1 does **not** count as a
   demonstrated GitNexus replacement until it is met.

### D.2 Workflow 2 — Identify callers and callees

*Representative user question:* "Who calls `PatchMatchController`, and what
does `PatchMatch::Run` call?"

Expected operations: `riltra callers` / `riltra_code_callers` anchored at
`colmap/src/colmap/mvs/patch_match.h:136:8` (the `PatchMatchController::Run`
declaration) and at `colmap/src/colmap/mvs/patch_match.cc:191` (`PatchMatch::Run`
definition); `riltra callees` / `riltra_code_callees` at the same anchors.

Objective criteria:

*Honesty gate (PASS required).*

1. The caller candidates of `PatchMatchController` include the two direct
   construction calls `exe/mvs.cc:271` and
   `controllers/automatic_reconstruction.cc:431`, and the candidate set
   contains no site outside the source truth of §B.2.
2. The callee candidates of `PatchMatch::Run` include the `PatchMatchCuda`
   construction and the `patch_match_cuda_->Run()` call at
   `patch_match.cc:196-197`.
3. The `ThreadPool` dispatch to `ProcessProblem` is either absent or labeled
   `POTENTIAL_CALL`; a verified `CALLS` edge here is a FAIL.
4. Every reported edge carries a file/line/column that resolves to the
   construct in §B.2, and every edge is labeled as a candidate.

*Capability gate (release requirement; expected **UNMET** on the pilot).*

5. The caller set is complete **and** identity-resolved: all three
   construction sites — including the factory-template site
   `ui/dense_reconstruction_widget.cc:397-398` — are attributed to the class
   `PatchMatchController` rather than to the `make_shared` / `make_unique`
   template, and each is distinguished from the same-named symbols in
   §B.6. §B.9 explains why a syntax-only index cannot do this (the callee of
   a syntax edge is `null`; the target is never resolved to a definition):
   gap E7. The gate stays as stated; the pilot is expected not to meet it.

### D.3 Workflow 3 — Trace a call path

*Representative user question:* "Trace the path from
`PatchMatchController::Run` down to the CUDA implementation."

Expected operations: `riltra trace --repo mmsp-pin --path
colmap/src/colmap/mvs/patch_match.cc --line 233 --col 28 --depth 4` (or
`riltra_code_trace` with `depth`), optionally with `--to` anchored at
`colmap/src/colmap/mvs/patch_match_cuda.cu:1326`.

Objective criteria:

*Honesty gate (PASS required).*

1. The returned path is a **subset** of the frozen chain in §B.3; every hop
   that is present resolves to the cited construct.
2. The plausibility disclaimer `plausible-path-not-execution` is present.
3. The path does not claim to reach a device-side definition; the CUDA
   terminus is labeled unsupported.
4. If the thread-pool hop is missing, the answer states that the dispatch is
   indirect — a complete-looking chain with a silently dropped hop is a FAIL.

*Capability gate (release requirement; expected **UNMET** on the pilot).*

5. The path spans the full chain of §B.3, including the indirect
   thread-pool dispatch. §B.9: the dispatch is an address-of reference, not
   a call, so a syntax-only index cannot traverse it, and the CUDA terminus
   lives in an unsupported file (gaps E4, E2). The gate stays as stated; the
   pilot is expected to return a shorter path.

### D.4 Workflow 4 — Analyze the impact of a change

*Representative user question:* "If I change the default of
`max_triangulation_angle`, what is affected?"

Expected operations: `riltra impact --repo mmsp-pin --path
colmap/src/colmap/mvs/patch_match_options.h --line 56 --col 10` (or
`riltra_code_impact`), cross-checked with `riltra callers` at
`colmap/src/colmap/controllers/option_manager.cc:1019:41`.

Objective criteria:

*Honesty gate (PASS required).*

1. The impact is anchored at the declared member
   `patch_match_options.h:56:10` and the anchor resolves.
2. The verdict is `UNKNOWN` — the expected syntax-only outcome (§B.9,
   `edges.rs:1377-1406`) — and never `SAFE`/`LOW` and never "no impact".
3. `completeness.status` is `partial` with the `graph-completeness-unproven`
   reason, the completeness disclaimer is present, and the unresolved
   reasons are reported.
4. The two macro sites are not resolved to function definitions, and no edge
   is claimed for the member read, the address-of binding, or the macro
   arguments.

*Capability gate (release requirement; expected **UNMET** on the pilot).*

5. The affected set includes `patch_match_options.h:56`,
   `patch_match_options.cc` (print at `:58`, validation at `:91-93`),
   `patch_match.cc` (read at `:361-366`, filter at `:443-447`) and the CLI
   binding `option_manager.cc:1018-1019`.
   §B.9 explains why this needs a reference-level graph and not a call
   graph: a member read, an address-of binding and a macro argument are not
   call expressions, so the member has no incoming edge at all (gaps E1,
   E7). **The expected pilot result is an empty path set** — the same shape
   as the frozen CP7-A golden
   `docs/pilots/cp7a/goldens-raw/impact-validate-id.json` (`"paths":[]`,
   `"covered":0,"total":0`). This gate is therefore expected to be
   **UNMET**; CP2 records the evidence, and workflow 4 does **not** count as
   a demonstrated GitNexus replacement until a reference-level capability
   exists. **The requirement is not lowered — it is recorded as unmet.**

### D.5 Workflow 5 — Natural-language code question through OpenCode

*Representative user question (asked in an OpenCode session, with Riltra
registered as an MCP server):* "In the MMSP repository, how is the maximum
triangulation angle option wired from the command line into the source-image
selection, and which files would I need to touch to change its default?"

Expected operations: a real OpenCode session against `riltra mcp`, using
`riltra_code_search`, `riltra_code_callers`/`riltra_code_callees`,
`riltra_code_trace`/`riltra_code_impact` and `riltra_repo_list` /
`riltra_repo_status` on the `mmsp-pin` repository.

Objective criteria:

*Honesty gate (PASS required).*

1. Every claim in the answer is one the tools actually support, and the
   answer states the limits it hit (no C++ identity resolution, no member
   reference graph, CUDA unsupported). A confident, complete-sounding
   answer that the tools cannot back is a FAIL.
2. Every fact carries a resolvable citation and a freshness verdict.
3. The answer does not assert C++ semantic resolution, CUDA support, or a
   complete call graph.
4. Transport health: the MCP session initializes, exposes the 11 read-only
   tools, and carries only JSON-RPC on stdout; a classified error (e.g. an
   unknown repository) is surfaced without a transport crash; no read created
   or migrated a store.
5. No cross-repository contamination: the answer's facts come only from
   `mmsp-pin`, and the repository identity is stated.

*Capability gate (release requirement; expected **UNMET** on the pilot).*

6. The answer carries the full chain of §B.4 — declaration, CLI binding,
   read site, filter site — and the "affects `__auto__` source selection
   only" qualification. This inherits the capability gate of D.4: the
   syntax-only tools cannot attribute the read and filter sites to the
   member, so the session is expected to answer with the declaration, the
   binding-as-candidate, and an explicit statement of what it could not
   resolve. The gate stays as stated.

### D.6 Release-readiness matrix (status at CP1, before any CP2 measurement)

| Workflow | Implemented (capability exists) | Verified by evidence | Unverified / blocked | Honesty gate (pilot) | Capability gate (release) |
|---|---|---|---|---|---|
| 1. Find symbol + references | yes — `search`, `callers` (M2/M3/M6) | CP7-A goldens on the M6 corpus | MMSP-scale accuracy, C++ identity | judgeable | **UNMET** (E7) |
| 2. Callers / callees | yes — `callers`, `callees` (M6) | CP7-A G2 zero unresolved citations | cross-TU C++ resolution at scale; indirect-dispatch honesty | judgeable | **UNMET** (E4, E7) |
| 3. Trace | yes — bounded `trace` (M6) | CP7-A fixture traces | the 4-hop `mvs/` chain; CUDA terminus labeling | judgeable | **UNMET** (E2, E4) |
| 4. Impact | yes — conservative `impact` (M6) | CP7-A `impact` fixture | `max_triangulation_angle` blast radius; `UNKNOWN`-first behavior on the pilot | judgeable | **UNMET** (E1, E7) |
| 5. NL question via OpenCode | yes — `riltra mcp`, 11 tools (M8/CP7-A) | CP7-A revised G6 (real OpenCode session) | the five pilot questions end-to-end; natural-language framing is OpenCode's, not Riltra's | judgeable | **UNMET** (E1, E7) |

---

## E. Functional gaps that could block GitNexus replacement

These are recorded as **release-blocking** only where the evidence supports
it. Each requires an owner decision; none may be silently downgraded.

| # | Gap | Evidence | Severity for `v0.1.0` |
|---|---|---|---|
| E1 | **C++ semantics are absent.** No provider covers C++; clangd is deferred by D-M9-2 and no compilation database exists in MMSP. All C++ answers are syntax-level candidates. | Design gate §1.5/§2.3/§4 G-SEM; §A.6 | **Release-blocking for the "verified caller" reading of workflows 2–5.** The honesty gate still passes on a syntax-only pilot, but every capability gate that needs resolved C++ edges is **UNMET** (§D). If MMSP's real-world use requires resolved C++ call edges, this is the gap that decides `v0.1.0` |
| E2 | **CUDA is unsupported by policy** (489 files at the pin, including all five first-party `.cu`). | §A.5; `syntax.rs:53-54`, `:77` | **Release-blocking for any workflow that must reason about the photogrammetry CUDA kernels.** The `mvs/` call chain terminates in CUDA (§B.3) |
| E3 | **Macro interiors and templates are opaque** — `PrintOption` (defined 4× in the pilot), `CHECK_OPTION_LE`, the 13 template sites in `reprojection_error.h`. | §B.4, §B.6 #4, #6 | Not release-blocking, but it caps accuracy: some edges can only be candidates |
| E4 | **Indirect dispatch is invisible** to a syntax-level extractor (`ThreadPool::AddTask(&Class::Method, …)`). | §B.2, §B.3 | Not release-blocking; it constrains how complete a trace may honestly be |
| E5 | **The Riltra branch's own history carries unsanitized local paths.** Measured on the synchronized M9 branch (`96a57f8`, isolated clone, `.gitleaksignore` deleted, `--log-opts=--all`): **1 068 path findings in 9 commits / 184 files, 0 credential-rule findings** (959 `local-absolute-path`, 94 `windows-user-path`, 15 `unix-home-path`). The 9 affected commits are `6449a42`, `cabfb9fc`, `2538e69`, `4746fff6`, `02151230`, `9953b0cf`, `b7f41157`, `bcbd1c6a`, `d1af7577` — 8 were already in the M9 branch, and `2538e69` arrived with the synchronization merge. The two `.gitleaksignore` pins are keyed to `2538e69` and suppress exactly 2 of those findings. The rewrite is not performed and not authorized (owner-gated; the `protect-master` ruleset blocks the non-fast-forward push). | `security/history-rewrite-plan.md`; `.gitleaksignore`; measured 2026-10-20 on the merged branch | Not a functional gap, but it blocks a clean `v0.1.0` provenance story and must stay in the release notes as an open risk |
| E6 | **No release artifacts exist yet**: no native per-OS archives, checksums, SBOM, or updated `NOTICE`. | Design gate §4 G-LIC/G-NAT | Release-blocking for publication, not for CP1/CP2 |
| E7 | **Reference-level (not call-level) resolution is absent.** A data member such as `max_triangulation_angle` has **no incoming edge at all** in a syntax-only index: a member read, an address-of binding and a macro argument are not call expressions, and a syntax edge's `callee` is never resolved to a definition. The impact workflow therefore returns an empty path set on the pilot — the same shape as the frozen CP7-A golden (`"paths":[]`, `"covered":0`). | §B.9; `crates/cli/src/edges.rs:167-205`; `docs/pilots/cp7a/goldens-raw/impact-validate-id.json` | **Release-blocking for workflow 4** (and for the D.5 capability gate): D.4's capability gate is **UNMET** and the expected pilot output is empty. Closing it needs either a C++ semantic provider (D-M9-2, deferred) or a new reference-extraction capability — each with its own evidence and its own OWNER GO |

No new semantic provider, SCIP producer, language support, repository, LLM,
embedding store, graph database, daemon or benchmark is required by CP1, and
none was added. MMSP was not modified.

---

## F. Recommended CP2 execution scope, prioritized by evidence

Ordered by what the evidence above actually requires, highest value first.

1. **Cold index on the frozen snapshot** (§A.7), ≥2 runs, recording time,
   counters, RSS and index size — the only number every later workflow
   depends on. Also records the first-party vs third-party error split that
   §A.4 says is the only meaningful signal.
2. **Workflow 1 + 2 against anchors B1, B2 and §B.6** — definition/reference
   accuracy, cross-TU callers, and the honesty checks (`POTENTIAL_CALL`
   separation, macro and anonymous-namespace handling). These are the cheapest
   cases with the sharpest FAIL conditions. Record **both** gates: the
   honesty-gate result and whether the capability gate was met.
3. **Workflow 3 against the frozen chain in §B.3** — trace, including the
   CUDA terminus and the indirect-dispatch limitation.
4. **Workflow 4 against `max_triangulation_angle` (§B.4)** — impact, with the
   `UNKNOWN`-first and `partial` completeness checks. The **expected** pilot
   result is an empty path set; the run must confirm that the answer is
   honest about it, and the unmet capability gate is recorded as evidence.
5. **Freshness scenarios F1–F5 (§C)** — pre-registered verdicts, executed
   unchanged.
6. **Workflow 5: a real OpenCode session against the pilot index** — the
   amended G-MCP evidence.
7. **One-file delta and full-rebuild parity** (§A.7) — incremental
   correctness, G-INC.
8. **Isolation assertions after every measured command** (§A.2) — the
   MMSP-untouched invariant, G-SEC.
9. **Owner decision package** — one table listing, per workflow, the
   honesty-gate result and the capability-gate result, and, for every
   unmet capability gate, the measured evidence and the options (a C++
   semantic provider under D-M9-2, a new reference-extraction capability, or
   an explicit scope decision that the workflow stays syntax-level). The
   package is what lets the owner decide whether `v0.1.0` may be described
   as a practical GitNexus alternative; it does not decide it.

CP2 is not authorized by this document. It needs its own OWNER GO.

---

## G. Verification log for this document

- Every anchor in §B was read in the isolated snapshot at
  `d62f06a576d8e76b705c8ecc57fc380f7ebc9c44`; the file, line and column of
  each anchor are recorded inline so a reviewer can re-read the same byte
  range.
- Inventory numbers in §A.3–A.5 were recomputed with `git ls-files` at the
  pin and reproduce design gate §2.2 exactly.
- The Riltra capability statements used above (extension classification,
  freshness reason vocabulary, CLI/MCP operation surface) were read from the
  current Riltra source at `96a57f8`:
  `crates/cli/src/syntax.rs:44-80`, the freshness reason constants, and
  `crates/cli/src/args.rs` / `crates/cli/src/mcp.rs` (11 tools:
  `riltra_code_search`, `riltra_code_callers`, `riltra_code_callees`,
  `riltra_code_trace`, `riltra_code_impact`, `riltra_repo_list`,
  `riltra_repo_status`, `riltra_knowledge_show`, `riltra_knowledge_search`,
  `riltra_knowledge_related`, `riltra_health`).
- No Riltra query was executed against the pilot to produce this document,
  and no Riltra output was used as ground truth.
- The E5 history-exposure figure was measured on the **synchronized M9
  branch** (`96a57f8`) in an isolated clone with `.gitleaksignore` deleted and
  `--log-opts=--all`: 1 068 findings, 9 commits, 184 files, 0
  credential-rule findings, 135 commits scanned. The commit list and the
  "8 already present, 1 added by the merge" split were confirmed with
  `git merge-base --is-ancestor` against the pre-merge M9 tip `a097e2d`.
  The clone and the scan ran outside the working tree; the Riltra working
  tree was not modified.
- An independent review pass (2026-10-20) was executed against the two
  documents. Its findings were validated one by one against the pinned
  source and the current Riltra source before anything was changed:
  - **Applied** — the `PrintOption` macro was attributed to the wrong file:
    the site `patch_match_options.cc:58` uses the macro defined at
    `patch_match_options.cc:43`, not `patch_match.cc:45`. The review also
    surfaced that `PrintOption` is defined in **four** translation units
    (`fusion.cc:78`, `patch_match.cc:45`, `patch_match_options.cc:43`,
    `texture_mapping.cc:1034`) with different bodies; that hazard is now
    part of the ground truth (§B.4).
  - **Applied** — the acceptance criteria in §D required coverage that a
    syntax-only index cannot produce (a member read, an address-of binding
    and a macro argument are not call expressions). The criteria were split
    into an honesty gate and a capability gate, the capability gate is
    explicitly recorded as **UNMET** on the pilot, and the root cause is now
    stated in the new §B.9 with the frozen CP7-A goldens as evidence. The
    release requirement is **not lowered** — it is recorded as unmet and
    carried into §E1/§E7.
  - **Applied** — small precision fixes: the `syntax.rs` variant/arm line
    references, the explicit CUDA arithmetic, the RAM figure's provenance,
    and the enumeration commands behind the "exactly N" claims.
  - **Rejected** — the reported column errors at `patch_match.cc:118:27`
    and `:667:11`. The columns were re-derived independently
    (`[regex]::Match` on the raw lines): `118:27` (indent 0) and `667:11`
    (indent 2) are correct; the file is 2-space indented, not 4.
  - **Not independently verifiable by the review** (its sandbox denied
    access outside the repository): the MMSP source anchors themselves. They
    were re-read directly in the isolated snapshot for this revision —
    including the `PrintOption` definitions, the `.Print()` and `Run` call
    sites, and the columns of the `patch_match.cc` anchors.
- A targeted CP1 correction round (2026-10-20), on owner instruction, with
  the evidence for each change:
  - **Python ground truth (§B.1 B3, §D.1).** The `normalize_vec` counts were
    conflated. Re-measured in the pinned snapshot: **1** definition
    (`geometry.py:34:5`), **5** call expressions on **4** source lines
    (`geometry.py:43:31` and `geometry.py:43:52` are two calls on the *same*
    line; `geometry_test.py:38:22`, `:44:22`, `:51:22`), **2** import
    references (`geometry_test.py:32:23`, `utils.py:57:23`) — i.e. **8**
    textual occurrences over **7** source lines in 3 files, and **7**
    reference facts. The previous text said "four call sites … six
    locations". The three quantities are now tabulated separately, and the
    acceptance expectation states the reference count as 7 facts, with
    under-counting the two-calls-on-one-line case named as a FAIL condition.
    Evidence: `git grep -n "normalize_vec"` over the whole pinned tree
    (7 lines) plus a per-line occurrence count (8).
  - **Incremental/full parity contract (§A.7, and design gate §3.2).** The
    requirement that a full rebuild produce "byte-identical counters" to the
    incremental run was wrong: `mode`, `reused` and `reparsed` are
    mode-specific by construction — a full build sets `reused: 0` and
    `reparsed` = files processed (`crates/cli/src/index.rs:340-363`), an
    incremental run reuses the unchanged files (`:664-709`). The M4 evidence
    that the requirement cited is a *fact* parity gate
    (`docs/m4-validation.md`: "files, definitions, imports, unresolved call
    candidates (names, kinds, spans, byte offsets, digests, provenance) —
    plus CLI search-output parity after volatile-field normalization. Any
    unexplained fact difference would be FAIL"), and the byte-identical
    counters recorded there are the fact counters (170,586 / 64,954 /
    562,384 / 152,543), not the operational ones. The same holds for the
    in-repo guard test `incremental_matches_independent_full_rebuild`
    (`crates/cli/src/index.rs:2812-2997`, "incremental must equal full"),
    which compares files, symbols, imports and calls. Both documents now
    state the five-point contract: same tree and configuration; fact parity
    (definitions, imports, call candidates, spans, digests, relevant query
    results); total fact counters compared on the same basis; operational
    counters recorded and validated per mode; any unexplained fact
    difference is a FAIL. `docs/m4-validation.md` and the frozen M8/CP7-A
    evidence were not modified.
  - **Isolated-clone procedure (§A.2).** The `GIT_CLONE_PROTECTION_ACTIVE=false`
    override was audited and removed. A plain
    `git clone https://github.com/RyanAraaful/mmsp.git` completes with exit 0
    and yields a clean clone (`HEAD` = `7c1b1ce…`, porcelain empty), and a
    plain clone of `RyanAraaful/riltra` also succeeds — so the recorded
    rationale ("a repo with `core.hooksPath` set breaks a plain clone") does
    not reproduce; `core.hooksPath` is a local config and is not propagated
    by a clone. The variable is not a documented Git or `gh` setting. The
    procedure now uses a normal clone, and states that any future need for
    an override must be scoped to the single command and justified with the
    exact error output. The MMSP snapshot remains an isolated clone outside
    both repositories, and the original MMSP repository was not modified.
