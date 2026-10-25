# M9 CP4 — release-candidate evidence pack

**Status: RELEASE CANDIDATE PREPARED. NOT A RELEASE.** This document records
what was built, measured and verified for the M9 release candidate on
2026-10-25, and states each release gate honestly. It authorizes no merge, no
tag, no publication and no signing: the `v0.1.0` release is a separate OWNER GO
(design gate §5, D-M9-6).

- Branch: `m9-multirepo-release-candidate`, pushed at `f45ddb8` on 2026-10-25 under the OWNER's instruction (no force; `master` unchanged); the commit that records the CI results (§1, §4) is local until pushed
- Authorization: the OWNER instruction of 2026-10-25 (integrate the
  template-callee commits, fix the field anchor and implement E7 Slice 2, close
  the proven callers/trace/impact gaps, re-test the five MMSP workflows and
  measure, finish CP4).
- Frozen inputs, not edited: `docs/m9-cp1-pilot-spec.md`, the CP2 harness and
  its archives (`docs/pilots/cp2/`, `docs/pilots/cp2-reeval/`), the CP7-A
  archive, and the E7 Slice 2 specification and evidence documents.
- Pilot: MMSP `d62f06a576d8e76b705c8ecc57fc380f7ebc9c44`, tree
  `c5482ea00481675e0aa921538f560947b68dca12`, an isolated plain clone
  (`core.longpaths=true`), `git status --porcelain` empty before and after every
  run. The original MMSP clone was only read by the harness's isolation
  assertions and stayed unchanged (`HEAD` and porcelain recorded before and
  after).
- Host: Windows 11 Pro, 16 logical cores, `rustc`/`cargo` 1.99.1, `git`
  2.53.0, Node 24.19. Debug profile for the harness (the CP1-frozen profile),
  release profile for the packaged binary. One host, one run each unless a
  count says otherwise: no figure here is a claim beyond this host.

## 1. What is in the candidate

The commits of this work on top of the M9 tip `f65c183`, from `f099fde` to the one that adds these documents, all DCO-signed;
`origin/m9-multirepo-release-candidate` moved `f65c183` → `f45ddb8` (fast-forward, hook scan of the ten commits clean):

| Commit | Content |
|---|---|
| `f099fde`, `54dbc71` | C++ template call terminal names the called function; SCIP evidence pinned (integrated by fast-forward; CI was green on the feature branch) |
| `1de6b96` | E7 Slice 2 S1: C++ name occurrences extracted |
| `726f859` | E7 Slice 2 S2: `reference_occurrences` table, schema 5, parser 4, golden `cfg=` digest |
| `37b9dd3` | E7 Slice 2 S3: `search --occurrences` and the optional `occurrences` MCP property |
| `80f82c2` | a C++ data member is an anchor (`callers`, `callees`, `trace`, `impact`) |
| `4bc8ed5` | callers of out-of-line C++ definitions (called name and declaring-header scope) |
| `13ea855` | release-candidate workflow, packaging and smoke scripts, SBOM, notices, README, licensing inventory |
| the documents commit | CP4 evidence pack, E7 implementation record, the redacted archive `docs/pilots/m9-rc/` |

The full local suite on the final tree: **764 tests, 0 failed** (unit 424, cli
16, edges 147, index 40, knowledge 52, mcp 58, semantic 15, syntax 12), `cargo
fmt --check` and `cargo clippy --all-targets -- -D warnings` clean.

**CI on the pushed tip `f45ddb8`** ([run 40185232654](https://github.com/RyanAraaful/riltra/actions/runs/40185232654), event `push`): all four jobs green.

| Job | Runner | Result |
|---|---|---|
| `check (windows-latest)` | `windows-latest` | fmt, clippy, `cargo check --locked`, `cargo test --locked` **764 passed, 0 failed**, CLI smoke |
| `check (ubuntu-latest)` | `ubuntu-latest` | same steps, **761 passed, 0 failed** |
| `check (macos-15)` | `macos-15-intel` (x64) | same steps, **761 passed, 0 failed** |
| `security` | `ubuntu-latest` | pinned gitleaks 8.30.1: pushed commits, committed tree and scanner regression tests all pass |

Windows runs 3 more tests than Ubuntu and macOS: 8 tests are `cfg(windows)` and 5 are `cfg(unix)` or `cfg(not(windows))`. 764 equals the
local Windows total. The previous reference was run 40169969953 at `54dbc71` (731 tests on Windows, 728 elsewhere, the same gap).

Dependencies: `Cargo.toml`, `crates/cli/Cargo.toml` and `Cargo.lock` are
byte-identical to `origin/master`, so no dependency entered the build or the
runtime path.

## 2. Before and after on the pilot

"Before" is the M9 tip `f65c183`, "after" is `4bc8ed5`; both ran the frozen CP2
harness on the same pinned snapshot, on this host, in the debug profile
(`docs/pilots/m9-rc/comparison.md`, raw evidence in `raw-before/` and
`raw-after/`).

| Measure | Before | After | Change |
|---|---|---|---|
| cold index, run 1 / run 2 | 98.99 s / 99.25 s | 97.27 s / 98.63 s | −1.2 % (inside the run-to-run spread) |
| database bytes | 50,233,344 | 55,627,776 | +10.7 % (58,811 occurrence rows) |
| refresh, nothing changed | 35.2 s | 36.7 s | +4.4 % |
| refresh, one file changed (`reparsed` 1, `reused` 1437) | 36.8 s | 36.6 s | −0.4 % |
| independent full rebuild | 98.2 s | 94.9 s | −3.3 % |
| warm `search`, p50 (5 queries) | 390–401 ms | 375–391 ms | −1.7 % to −6.1 % |
| warm `callers` of a qualified method, p50 | 709.6 ms | 807.2 ms | +13.8 % (it now answers 3 edges, was 0) |
| warm callees, trace, impact, index status, repo status, p50 | 156–781 ms | within ±3 % | no change |

- **Incremental/full parity: identical.** Files, symbols, imports, calls and
  edges digests are identical and the twelve parity queries return identical
  bytes, before and after. The frozen digest does not cover the new table, so
  `docs/pilots/m9-rc/method/occurrence-parity.cjs` compares it: 58,811 rows
  with the same SHA-256 in two independent cold indexes and in the index that
  was refreshed incrementally back to the pin.
- **Freshness F1–F5 match their pre-registered verdicts** in both runs:
  F0 `fresh`, F1 `dirty,content-changed`, F2 `dirty,file-added,file-removed`,
  F3 `dirty,file-removed`, F4 `head-changed`, F5 `branch-changed` (all
  `stale`). The first pass of F4 was invalid in both runs (the depth-1 snapshot
  lacked the later commit, so nothing changed and the verdict was `fresh`);
  both were corrected and re-run, and the first pass is kept next to the valid
  one.
- **Release profile** (informational, one run): cold index 29.2 s, no-op refresh
  17.9 s of which the four timed phases account for 4.4 s. About 13.5 s of a
  no-op refresh is spent outside the timed phases; `git` is not the cause
  (`status` 0.07 s). This fixed cost predates this work and is not profiled
  here; it is the reason a no-op refresh is only 1.6× faster than a cold index
  on a repository of this size.

The honest reading: the indexing and refresh cost did not change, storage grew
by 10.7 %, the warm queries did not change except the one that now does more
work, and the gain is in what the answers contain (§3), not in speed.

## 3. The five workflows

Graded by the frozen grader (`grade.cjs`, not modified):

| Layer | Before | After |
|---|---|---|
| honesty | 17 PASS, 1 UNAVAILABLE | **17 PASS, 1 FAIL** |
| capability | 4 UNMET | 4 UNMET |
| evidence | 2 UNAVAILABLE | 2 UNAVAILABLE |

Two checks changed:

- **D2-H5 UNAVAILABLE → PASS.** `callers` at the option binding in
  `option_manager.cc` now returns two candidate edges (it returned none), each
  with a resolvable location and a `POTENTIAL_CALL` label.
- **D2-H2 PASS → FAIL (a regression).** The criterion is that the callee set of
  `PatchMatch::Run` contains the `PatchMatchCuda` construction. The
  template-callee fix (`f099fde`, integrated in this work) makes the terminal
  `make_unique`, so the edge no longer carries the type argument and the
  grader, which reads callee names, finds no `PatchMatchCuda`. The earlier PASS
  was an artifact of the previous terminal text (`make_unique<PatchMatchCuda>`;
  for a qualified argument the old terminal was `PatchMatchController>`).
  **The grader was not changed and the criterion was not lowered.** Restoring
  its intent needs a labelled "type constructed through a factory template"
  candidate that never becomes a SCIP `CALLS`; that is a design decision (§7).

What the workflow queries return:

| Query | Before | After |
|---|---|---|
| callers of `PatchMatch::Problem::Print` (definition) | 0 edges | 3 candidates (`patch_match.cc` ×2, `exe/mvs.cc` ×1) |
| callers at the option binding (`AddPatchMatchStereoOptions`) | 0 edges | 2 candidates (`option_manager.cc`, `exe/mvs.cc`) |
| callers of `PatchMatchController::Run` | 0 edges | 8 candidates, including the real `controller.Run()` in `exe/mvs.cc:277` |
| `impact` at `patch_match_options.h:56:10` | `UNKNOWN`, anchored at the struct (bytes 1696–5794) | `UNKNOWN`, anchored at the member (bytes 2405–2428) |
| occurrences of `max_triangulation_angle` | not available | 4 locations (§3.1) |
| callers of the class, callees, trace | unchanged | unchanged (1 edge; 21 and 21 one-hop paths) |

**The four capability gates stay UNMET, before and after**: D1-C6 (method
identity), D2-C6 (the three construction sites attributed to the class: one of
three, as before), D3-C5 (no path beyond the first hop) and D4-C5 (an empty
affected set). None of them can be met without type identity, which a syntax
index cannot give; §7 lists what would.

### 3.1 Find references: the E7 Slice 2 pilot

`search --repo mmsp-pin --symbol max_triangulation_angle --occurrences` returns
exactly the four locations the frozen evidence predicted (spec row P1), with
`field_declarations` 1 and `symbol_declarations` 1:

| Location | Role | Confidence |
|---|---|---|
| `option_manager.cc:1019:41` | `address-of-binding` | observed |
| `patch_match.cc:362:16` | `member-access` | observed |
| `patch_match.cc:365:31` | `member-access` | observed |
| `patch_match_options.cc:58:15` | `macro-argument` | candidate |

The declaration (`patch_match_options.h:56:10`) is the `field` search hit; the
option-name string, the plain identifier at `:91` and the two `CHECK_OPTION_LE`
arguments are absent by the recorded policy; the filter site (`:443-447`,
which reads a local) does not appear. The pilot script reports 12 PASS and 0
FAIL on the final binary (`raw-after/e7-pilot.json`); details and one
correction of the frozen spec (the `value` count is 9, not 27) are in
`docs/m9-e7-implementation.md`.

### 3.2 Workflow 5 through MCP

- The MCP stdio smoke on the candidate: 12 tools advertised, stdout carries
  only JSON-RPC (6 of 6 lines parsed), a classified error for an unknown
  repository (`not_registered`), no transport failure.
- The scripted D5 tool sequence over the real MCP transport answered all nine
  calls (the one `is_error` is the intended unknown-repository check).
- **A real OpenCode session did not run.** `opencode run` returned "This API
  key has expired" before it reached the MCP server (`mcp-server-not-reached`,
  `raw-after/mcp/opencode-session.json`). Renewing the key is the owner's; I did
  not touch credentials. The agent-side evidence of the candidate is therefore
  **UNAVAILABLE**, and G-MCP is not marked passed on it (§5).

## 4. The release candidate artifacts

Built and checked on this Windows host, then built and checked again on the three CI runners (§4.1).

| Item | Result |
|---|---|
| release build | `cargo build --release --locked`, 90.7 s, binary 13,910,016 bytes |
| archive | `riltra-0.1.0-windows-x64.zip`, 3,193,112 bytes, SHA-256 `4a792d60a944c28dc6646dc2eb6d8779ce296b65cc1dfbad6347ad0edde17477` (verified with `sha256sum -c`). A rebuilt archive has another digest: `Compress-Archive` stores timestamps, so the archive is not byte-reproducible, the contents are |
| archive contents | `riltra.exe`, `LICENSE`, `NOTICE`, `THIRD_PARTY_LICENSES.txt`, `sbom.cdx.json`, `INSTALL.md` |
| clean-install smoke | `release/smoke.sh` on the unpacked archive: 15 checks, all `ok` (version, help, status, init, index, search, occurrences, callers as candidates, JSON-RPC-only stdout, twelve tools, a source-grounded call, a classified error, empty stderr, notices present) |
| SBOM | CycloneDX 1.5, 109 third-party components (110 with the root), 110 dependency entries, generated from `cargo metadata --locked` by `crates/cli/examples/release_docs.rs`; committed as `release/sbom.cdx.json` |
| drift check | `release_docs --check` passes: the packages of `cargo metadata` equal those of `Cargo.lock` in both directions, every package has a license and a checksum, every package has a row in `docs/licensing.md`, and the committed SBOM equals a fresh one. It fails (exit 1) when a licensing row is removed (checked) |
| licenses | 68 `MIT OR Apache-2.0`, 29 `MIT`, 5 `Apache-2.0 OR MIT`, 2 `Unlicense OR MIT`, 2 `Apache-2.0`, 1 `Zlib`, 1 `MIT OR Apache-2.0 OR LGPL-2.1-or-later` (`r-efi`, the MIT alternative is used), 1 `(MIT OR Apache-2.0) AND Unicode-3.0`: permissive only; the existing `new_dependencies_are_audited` test enforces the allowlist |
| license texts | `THIRD_PARTY_LICENSES.txt` carries each crate's own license and notice files; seven crates publish none (`r-efi`, `rmcp`, `rmcp-macros`, `rsqlite-vfs`, `tree-sitter`, `tree-sitter-cpp`, `tree-sitter-typescript`), so the canonical text of the declared license is supplied with the crate's declared authors as holders |
| workflow | `.github/workflows/release-candidate.yml`: manual or `v*-rc*` tag, Windows, Linux and macOS x64; build, drift check, package, smoke of the archive, upload as an artifact. It creates no release, tag, signature or installer. It ran once, from a throwaway branch (§4.1) |

The cargo metadata step downloads the manifests of the packages that this host
does not build (other platforms), checksummed by `Cargo.lock`; no build script
runs.

### 4.1 The workflow on the three runners

`workflow_dispatch` returned HTTP 404 (`workflow release-candidate.yml not found
on the default branch`): GitHub dispatches only workflow files that exist on
`master`, and this one exists only on the M9 branch. With the OWNER's choice
of a throwaway branch, `rc-run/m9` (M9 plus one commit, `e8ec1d3`, that adds a
`push` trigger for `rc-run/**`) was pushed, the workflow ran once, and the
branch was deleted from the remote and locally. That commit is not part of M9
and must not be merged; the built sources are those of `f45ddb8`.

[Run 40185383581](https://github.com/RyanAraaful/riltra/actions/runs/40185383581): **three jobs, all steps green**
(release build, `release_docs --check` printing `no drift`, package, clean-install
smoke of the unpacked archive with every check `ok` including twelve tools, a
JSON-RPC-only stdout and a classified error, artifact upload).

| Platform | Runner | Archive | SHA-256 | Artifact size |
|---|---|---|---|---|
| windows-x64 | `windows-latest` | `riltra-0.1.0-windows-x64.zip` | `9d5df8db061d0076be52d4b5959bd60350b989433f55f58ed025d74b51cacb0e` | 3,136,075 B |
| linux-x64 | `ubuntu-latest` | `riltra-0.1.0-linux-x64.tar.gz` | `03972f111e98f1bc9107e451e9af554f83701f66cc0a5468c092a9f604ce5e67` | 3,677,420 B |
| macos-x64 | `macos-15-intel` | `riltra-0.1.0-macos-x64.tar.gz` | `51d8b2928004a5bc0e802b8c85435fe0d13c1cf9dd6f5c2e8020bdd46871c5f7` | 3,495,634 B |

The step output of the three jobs (drift check, package, smoke) is archived, with
runner paths replaced, in `docs/pilots/m9-rc/raw-ci/release-candidate-run-40185383581.txt`.
The artifacts stay in the workflow run for the retention period; they were not
downloaded. The same drift check that passed on this host (Windows) passed on
Linux and macOS, so the committed SBOM is identical across platforms. macOS
ARM64 is not built.

## 5. Release gates (design gate §4), stated separately

| Gate | Status | Basis |
|---|---|---|
| G-LIC | **PASS** (on the candidate, this host) | SBOM generated from the locked graph, zero drift against `Cargo.lock`, every package in `docs/licensing.md`, `NOTICE` updated, permissive-only audit passes, commands and outputs archived. The SBOM is a superset of what one platform compiles |
| G-NAT | **PASS** (acceptance test; macOS is x64 only) | three native archives were produced by the CI runners, each passed the clean-install smoke with archived command output (`docs/pilots/m9-rc/raw-ci/`), checksums are listed (§4.1), and the three-OS CI is green with 764/761/761 tests. **macOS ARM64 is not covered** (CP3; `README.md` known limitations). The requirement also asks that platform behavior be documented: macOS ARM64 is in the README, while Windows junctions and WAL sidecars are recorded in `docs/audits/2026-10-19-windows-wal-concurrency.md` and the M8 gate, not in the README or `INSTALL.md` |
| G-CLI | **PASS** (candidate, this host) | the suite and goldens are green (764/764); eight pilot commands run twice return identical bytes; the pre-1.0 stability policy is written in `README.md` |
| G-MCP | **NOT YET VERIFIED** | transport, 12 tools, pure stdout, classified error, non-mutating store content: pass on the candidate (smoke and scripted D5). The real OpenCode run is **blocked** by an expired OpenCode API key (§3.2) |
| G-SYN | **NOT YET VERIFIED** | extraction, determinism and the new occurrence rows verified; the `.inl` coverage gap found in CP3 is still undecided |
| G-SEM | **NOT YET VERIFIED** | the two-family minimum is covered by the suite on the candidate; C++ on MMSP stays NOT YET VERIFIED with the D-M9-2 deferral reason; CUDA unsupported |
| G-GRF | **NOT YET VERIFIED** | the frozen MMSP ground truth was run on the candidate (§3): no promotion, no fabrication, completeness `partial`; but D2-H2 fails since the template-callee change and needs a decision |
| G-FSH | **PASS** (candidate, pilot) | F1–F5 match their pre-registered verdicts and reasons (§2) |
| G-INC | **PASS** (candidate, pilot) | parity identical including the occurrence table; one file reparsed; timings and host recorded; no speedup claimed beyond the measured 2.6× (debug) of a refresh over a rebuild |
| G-KNW | **NOT YET VERIFIED (release)** | the knowledge suite (52 tests: backup, verify, restore, export, non-mutating reads) is green on the candidate; the release-level round trip on an isolated home was not separately run |
| G-TOK | **NOT YET VERIFIED** | §8 |
| G-SEC | **PASS** (candidate; accepted limit below) | isolation and non-mutation hold (porcelain 0 and `HEAD` unchanged for the snapshot and the original clone; the only home change on a read is the accepted WAL sidecars, identical to the CP2 record); the repository's own scan (`security/scan.sh`, gitleaks 8.30.1) over the commits since `f65c183` and over the tree reports no leaks, and the pre-commit hook scanned every commit; the CI `security` job (pinned gitleaks: pushed commits, tree, scanner regression tests) is green on the pushed tip. The accepted limit stays: a read opens the store's `-wal`/`-shm` sidecars (M7 limitation L2, recorded in `docs/m8-design-gate.md` and the WAL audit, not in the README) |

"PASS" above means the acceptance test of the gate ran on the candidate with
archived evidence. A gate is never marked passed on milestone history alone.

## 6. Known limitations of the candidate

- Syntax-only answers are name-matched candidates; `trace` ends at the anchor
  and `impact` is `UNKNOWN`, by design, until a semantic provider backs them.
- `callers` of a C++ method lists every same-named call in its scope; without
  type information it cannot separate `controller.Run()` from `fuser.Run()`.
- `search --occurrences` is a repository-wide census of an exact name, it
  counts method-call receivers as member accesses, and an empty list is not
  proof of absence. `.inl`, `.cu`, `.cuh` and C sources are not covered.
- The timings are one host and one profile; a no-op refresh has an unprofiled
  fixed cost of about 13.5 s in release on this host.
- The repository history carries machine-local paths from early commits (E5/E11,
  owner-gated).

## 7. What needs the owner

1. **Done on 2026-10-25:** the branch was pushed and the CI and the
   release-candidate workflow ran green (§1, §4.1). To make the workflow
   dispatchable it has to be merged to `master` (a pull request: not done, not
   authorized here); until then it runs from a throwaway branch or an `-rc` tag.
   This documentation commit is local until pushed.
2. **Renew the OpenCode API key** so the real agent-side session can run
   against the candidate (G-MCP, D5-E1).
3. **D2-H2 (decided 2026-10-25, see §10):** the owner chose the labelled
   candidate. `make_unique<T>` and `make_shared<T>` keep their callee names; the
   constructed type is a `POTENTIAL_CALL` candidate with the reason
   `factory-template-argument`, never `CALLS`. The grader's D2-H2 check passes on
   this candidate. The 2026-10-25 record of the regression in §3 is unchanged.
4. **C++ semantics on MMSP.** The type-identity gates (D1-C6, D2-C6) and a
   real answer for D3/D4 need a semantic provider. The clangd pilot
   (`docs/m9-clangd-pilot.md`) showed `references` on the colmap sources and the
   twelfth tool already ships; MMSP has no compilation database, so it needs a
   downloaded clangd and a reconstructed database, which this work did not do
   (explicit permission needed, D-M9-2).
5. **A new G-TOK measurement** under frozen rules (§8).
6. **`.inl` indexing** (a one-line classification change with an index-content
   consequence) and any new occurrence role beyond the frozen four, for example
   type references (class names in declarations, template arguments, `new T`).

## 8. G-TOK applicability review (design gate §4, decision D-M9-4)

The CP7-A measurement (16,764.5 vs 24,134.7 gross tokens per completed task,
−30.5 %) was taken on the eleven-tool MCP surface of M8. The question is
whether what changed since then materially affects MCP output, task outcomes
or token consumption.

| Change since M8 | Effect on what an agent sees |
|---|---|
| a twelfth tool, `riltra_code_semantic` | `tools/list` grows by one tool and its schema; every session pays for it |
| `riltra_health` capability flags | different values in a document an agent may read |
| E8 caller scope (`54c7d26`) | `callers` returns more candidate edges |
| E7 Slice 1 field symbols | `search` returns `field` hits that did not exist |
| template terminal (`f099fde`) | callee names change (`make_unique<T>` → `make_unique`) |
| E7 Slice 2 and the callers fix | new opt-in section; `callers` of C++ methods goes from empty to candidate lists |

Output size and content change materially for C++ repositories, and the tool
catalogue itself grew. **Verdict: the CP7-A figure cannot be relied on for the
release candidate.** G-TOK stays **NOT YET VERIFIED**. A new measurement, run
under frozen rules and labelled separately (it never replaces CP7-A), needs the
owner's GO; it is proposed, not run.

## 9. Evidence index

| Artefact | Content |
|---|---|
| `docs/pilots/m9-rc/` | README, `comparison.md`, `raw-before/`, `raw-after/` (redacted), `method/` (`e7-pilot.cjs`, `occurrence-parity.cjs`, `run-rest.ps1`, `compare.cjs`) |
| `docs/m9-e7-implementation.md` | the E7 Slice 2 implementation record |
| `release/` | `README.md`, `INSTALL.md`, `package.sh`, `smoke.sh`, `sbom.cdx.json` |
| `crates/cli/examples/release_docs.rs` | SBOM, third-party licenses and the drift check |
| `.github/workflows/release-candidate.yml` | the release-candidate workflow (not yet run) |
| `docs/licensing.md` | the complete package inventory appended on 2026-10-25 |

## 10. Amendment 2026-10-25: D2-H2 resolved by a labelled candidate

Dated amendment. §3 keeps the record of the regression (D2-H2 PASS → FAIL).
This section records the change that followed and does not rewrite that record.

**Decision.** `make_unique` and `make_shared` stay the callee names. The type
argument of such a call is a labelled candidate: `POTENTIAL_CALL` at the
argument's own span, `unresolved` = `factory-template-argument`, never `CALLS`.
Only these two names qualify, and only a plain qualified argument
(`mvs::PatchMatchController`). A generic `make<T>` and other argument shapes get
no candidate. The callee name `make_unique<T>` is not restored.

**Change.** `crates/cli/src/edges.rs` derives the candidate from the stored
callee text. `crates/cli/src/index.rs` reads its spans from the evidence when no
call row sits at its span. `crates/cli/src/store.rs`: `PARSER_VERSION` 4 → 5, so
an index written by parser 4 reads as stale. The `cfg=` digest in the 88 golden
files changes from `fc5c6854d048a25d` to `1a5c2768082c1072`; the substitution was
checked by reversal against HEAD. One regression test changes:
`cli_cpp_template_call_names_the_function_not_the_type_argument` now expects
three edges and one candidate.

**Verification.**
- `cargo test --locked`: all targets pass. `cargo clippy --all-targets -- -D warnings`: exit 0.
  `cargo fmt --all -- --check`: clean.
- New tests: `cli_factory_template_argument_is_a_labelled_candidate`;
  `cli_factory_template_argument_stays_a_candidate_under_scip` (no `CALLS` under
  SCIP evidence); unit `edges_factory_argument_names_the_constructed_type_only`.
- Frozen harness `docs/pilots/cp2/method/run-all.ps1`, unchanged, on the pinned
  snapshot `d62f06a` (tree `c5482ea`), debug build: completed with no failed
  check; isolation assertions pass (snapshot and original clone porcelain 0).
- Grader: honesty 18 PASS (the 2026-10-25 run before this change: 17 PASS, 1 FAIL);
  capability 4 UNMET (D1-C6, D2-C6, D3-C5, D4-C5); evidence 2 UNAVAILABLE
  (D3-E1, D5-E1).
- D2-H2 callees of `PatchMatch::Run`: `make_unique` (`POTENTIAL_CALL`, `dynamic`,
  callee span `std::make_unique<PatchMatchCuda>`) and `PatchMatchCuda`
  (`POTENTIAL_CALL`, `factory-template-argument`, span `PatchMatchCuda`). No
  `CALLS` edge in any query output of the run.
- D2-C6 stays UNMET. Class callers gain `dense_reconstruction_widget.cc`, so two
  of the three expected sites are attributed. `exe/mvs.cc` is still not recorded:
  its direct initialization is not recorded by the syntax layer, as before.

**Side effects (recorded, not extended in this task).** Callers of
`PatchMatchController` gain `pycolmap/pipeline/mvs.cc`, a factory site outside
the three expected sites. Callees of `PatchMatchController::Run` gain two
`ThreadPool` candidates (`std::make_unique<ThreadPool>`, lines 240 and 241).
`trace --depth 4` from `PatchMatchController::Run` goes from 21 to 23 paths.

**Limits.** The D5-E1 reason changed from `mcp-server-not-reached` to
`session-not-run`: this pass did not run the OpenCode session (its API key had
expired). Byte offsets differ from the M9 runs because the new snapshot clone
uses CRLF line endings. Compare semantic fields (names, kinds, reasons, grades),
not byte offsets or database sizes, across these runs. The raw run output is not
tracked in this repository.
