# M9 release-candidate gate amendment (2026-10-26)

Dated amendment. It records the release-candidate evidence for commit `ee7482d`, the merge of PR #7 into `master`, and the repeat of G-SEM, G-INC, and G-FSH on that commit. It changes no earlier record: the [design gate](m9-design-gate.md), the [CP4 evidence pack](m9-cp4-evidence.md), the [G-SEM amendment](m9-gsem-amendment.md), and the [pre-release acceptance amendment](m9-preprelease-acceptance-amendment.md) keep their text and conclusions.

Scope: documentation, and the redacted evidence archive in [`pilots/m9-rc-ee7482d/`](pilots/m9-rc-ee7482d/README.md). No change to source, tests, dependencies, CI, the release workflow, or the frozen harness. This file authorizes no tag, GitHub release, publication, or M10 work.

Status terms follow [acceptance amendment §3](m9-preprelease-acceptance-amendment.md).

## 1. Release candidate

| Item | Value |
|---|---|
| Repository | `RyanAraaful/riltra` |
| Branch | `origin/master` |
| Commit | `ee7482dfb12f0e88e3620ada5ce35f74f45041a3` (merge commit) |
| Merge | [PR #7](https://github.com/RyanAraaful/riltra/pull/7), `m9-multirepo-release-candidate` into `master`, merged 2026-10-26T03:13:43Z |
| Parents | `a5f67e6` (previous `master`), `05c7f18` (PR head) |
| Tree | identical to `05c7f18` |
| Version | `0.1.0` (`[workspace.package]` in `Cargo.toml`) |
| Tags, releases | none |

## 2. Workflow results

Both runs succeeded on `ee7482d`.

| Run | Event | Result | Jobs (all success) |
|---|---|---|---|
| [40263559935](https://github.com/RyanAraaful/riltra/actions/runs/40263559935) `release-candidate` | `workflow_dispatch`, `master` | success | `master only`, `package (windows-x64)`, `package (linux-x64)`, `package (macos-x64)` |
| [40263023981](https://github.com/RyanAraaful/riltra/actions/runs/40263023981) `ci` | `push`, `master` | success | `check (windows-latest)`, `check (ubuntu-latest)`, `check (macos-15)`, `security` |

Each package job ran the release build (`cargo build --release --locked`), `release_docs --check` (`no drift`), packaging, the clean-install smoke (`smoke: all checks passed`), and the artifact upload. Package job logs: [linux-x64](https://github.com/RyanAraaful/riltra/actions/runs/40263559935/job/121380953461), [windows-x64](https://github.com/RyanAraaful/riltra/actions/runs/40263559935/job/121380953497), [macos-x64](https://github.com/RyanAraaful/riltra/actions/runs/40263559935/job/121380953516).

The macOS package runs on `macos-15-intel` (x64). macOS ARM64 is not covered.

## 3. Artifacts

| Platform | Artifact | Archive (size) | Archive SHA-256 | GitHub artifact digest |
|---|---|---|---|---|
| Windows x64 | `riltra-windows-x64` | `riltra-0.1.0-windows-x64.zip` (3,200,342 B) | `54387884f151311cedb6378ff5d24c54aa4045b1f5b64e69c2891802fa9ac73e` | `sha256:9c5c785c39ed433906c0d56bfec88d6334804941976ccdb686fa2fdbe6ef5b72` |
| Linux x64 | `riltra-linux-x64` | `riltra-0.1.0-linux-x64.tar.gz` (3,767,655 B) | `e98340fc3e97533a28a8f146e010e374026de5bae5180db83503f1cd27e0520d` | `sha256:62ece3ddacc3cf6709df6a5b1f5f9276e97e9e3baeafee0ab1cbd5ccf69ab2d7` |
| macOS x64 | `riltra-macos-x64` | `riltra-0.1.0-macos-x64.tar.gz` (3,583,201 B) | `65b78d72bd2f7d848768638e8e11c403c081eb944cd91b95e3efb546afcf972c` | `sha256:a29bca078724f938902fd86a3efb39f9da4df6363bcf3e196fc6e945625a1c2f` |

Verified for each artifact:
- It holds its archive and a `.sha256` file, and the recorded SHA-256 matches the archive.
- The archive holds one top-level folder with the binary, `LICENSE`, `NOTICE`, `THIRD_PARTY_LICENSES.txt`, `sbom.cdx.json`, and `INSTALL.md`. The Windows binary is `riltra.exe`.

## 4. Gate status at `ee7482d`

Gate definitions: [design gate §4](m9-design-gate.md). Core scope: [acceptance amendment §2](m9-preprelease-acceptance-amendment.md), decisions D-1 to D-6.

### 4.1 Tested directly at `ee7482d`

| Gate | Status | Evidence |
|---|---|---|
| G-NAT: native archives, smoke, checksums | PASS | Sections 2 and 3. Smoke passed on all three runners. |
| G-LIC: SBOM, notices, licence audit | PASS | `release_docs --check`: no drift on all three runners. CI runs `new_dependencies_are_audited` (`crates/cli/tests/semantic.rs`). |
| G-CLI: stable CLI | PASS | CI `check` on three runners: `cargo fmt --all -- --check`, `clippy -D warnings`, `cargo test --locked`. Archive smoke: `version`, `help`, `status`, `init`, `index`, `search`, `occurrences`, `callers`. |
| G-MCP core: protocol and contract (part) | PASS | CI `cargo test --locked` (includes `crates/cli/tests/mcp.rs`). Archive smoke: stdout is JSON-RPC only, twelve tools advertised, source-grounded call, classified error, empty stderr. |
| G-SEC: security | PASS | CI `security` job (`security/scan.sh tree` in [ci.yml](../.github/workflows/ci.yml)). |
| G-SEM: functional correctness | PASS (attempt 3) | Section 5.1. Provider reliability is an OPEN FINDING. |
| G-INC: incremental correctness | PASS | Section 5.2. |
| G-FSH: freshness | PASS | Section 5.3. |
| Regression and CI | PASS | Run [40263023981](https://github.com/RyanAraaful/riltra/actions/runs/40263023981), all four jobs. |

### 4.2 Carried from earlier commits, implementation unchanged or re-checked

| Gate | Status | Evidence commit | Basis for carrying |
|---|---|---|---|
| G-SYN: source-verified syntax | PASS | `b311bb6`: candidate index, fact digests and span checks ([acceptance amendment §8.1–§8.2](m9-preprelease-acceptance-amendment.md)). Task 6 record `df35536` ([m9-task6-final-verification.md](m9-task6-final-verification.md)). | No fact-producing source changed after `b311bb6`. The only later `crates/` change is the refusal text and check order of `resolve_edge_anchor` (`crates/cli/src/index.rs`, commit `73de417`). The five fact tables of the pinned index, rebuilt at `ee7482d`, equal the archived candidate digests (section 5.2). |
| G-KNW: knowledge | PASS | `b311bb6`: release-level round trip, 73 checks, 0 failed ([g-knw-round-trip.txt](pilots/m9-preprelease/raw/core/g-knw-round-trip.txt)). | No knowledge source changed after `b311bb6`. |
| G-GRF: correctness and safety | PASS (outputs re-checked) | Task 6 record `df35536`. Candidate grading at `b311bb6`. Query probe at `73de417` ([acceptance amendment §15](m9-preprelease-acceptance-amendment.md)). | The probe reran the 22 frozen queries on post-fix code: 22 of 22 outputs identical. `crates/` is identical from `73de417` to `ee7482d`. The grading record is not regenerated. |
| G-LIC: dependency set | PASS | Licence record in CP4 §5, `1e8e5d6` ([m9-cp4-evidence.md](m9-cp4-evidence.md)). | `Cargo.toml`, `crates/cli/Cargo.toml`, and `Cargo.lock` unchanged since `1e8e5d6`. Drift check run in 4.1. |
| G-MCP core: D5 sequence and read-only check | PASS | `b311bb6` ([acceptance amendment §4.1](m9-preprelease-acceptance-amendment.md)). | `crates/cli/src/mcp.rs` unchanged since `b311bb6`. Not repeated at `ee7482d`. |

### 4.3 Unchanged

- G-GRF capability (D1-C6, D2-C6, D3-C5, D4-C5): **UNMET**. Accepted as pre-release known limitations ([acceptance amendment §10](m9-preprelease-acceptance-amendment.md)).
- G-TOK: not a core gate. Per agent, see 4.4.
- G-PAR: not an M9 gate (M10 target, [design gate §4](m9-design-gate.md)).

### 4.4 Agent integration (not core; D-1)

| Agent | Status | Basis |
|---|---|---|
| OpenCode | **UNAVAILABLE**: API key expired; no session on the M9 surface | [acceptance amendment §4.2, §5](m9-preprelease-acceptance-amendment.md); [preflight](pilots/m9-consolidation/raw/g-tok-preflight.txt) |
| Claude Code | Smoke PASS (single session; no workflow or token claim). G-TOK **DEFERRED** | [acceptance amendment §4.2, §5](m9-preprelease-acceptance-amendment.md); [session smoke](pilots/m9-preprelease/raw/agents/claude-code/session-smoke.json) |
| Codex | **NOT TESTED** | [acceptance amendment §4.2](m9-preprelease-acceptance-amendment.md) |

## 5. Repeat evidence on `ee7482d`

The earlier G-SEM, G-INC, and G-FSH records ([G-SEM amendment](m9-gsem-amendment.md), [CP4 §5](m9-cp4-evidence.md)) stay as recorded. This section records the repeat on `ee7482d`.

### 5.1 G-SEM: functional correctness (PASS, attempt 3)

Acceptance test ([design gate, G-SEM](m9-design-gate.md)): two providers from two language families, one of them TypeScript or C/C++; source-grounded definitions and references archived; C++ on MMSP labeled NOT YET VERIFIED with its deferral reason; no syntax candidate promoted under a semantic label.

Procedure: frozen harness [`cp3/method/g-sem.ps1`](pilots/cp3/method/g-sem.ps1), unchanged, run three times. Attempt 2 reused the home of attempt 1 (warm re-run). Attempt 3 used a new home. Each attempt also ran a read-only TypeScript call-site definition query (`semantic query --repo ts --path src/main.ts --line 3 --col 13`, without `--refs`). Outputs: attempt 1 in [`gsem/raw/`](pilots/m9-rc-ee7482d/gsem/raw/), attempt 2 in [`gsem/raw-attempt2/`](pilots/m9-rc-ee7482d/gsem/raw-attempt2/), attempt 3 in [`gsem/raw-attempt3/`](pilots/m9-rc-ee7482d/gsem/raw-attempt3/). Logs: [`g-sem.log`](pilots/m9-rc-ee7482d/evidence/g-sem.log), [`g-sem-attempt2.log`](pilots/m9-rc-ee7482d/evidence/g-sem-attempt2.log), [`g-sem-attempt3.log`](pilots/m9-rc-ee7482d/evidence/g-sem-attempt3.log).

| Attempt | Home | Rust definition | Rust references | TypeScript references | TypeScript definition | Result |
|---|---|---|---|---|---|---|
| 1 | first | none; diagnostic `analysis wait timed out; answers may be empty` | 3, grounded | 2, grounded | 1, grounded | UNAVAILABLE (no Rust definition) |
| 2 | first, warm re-run | none; same diagnostic | 3, grounded | 2, grounded | 1, grounded | UNAVAILABLE (no Rust definition) |
| 3 | second, new | 1, grounded; same diagnostic also present | 3, grounded | 2, grounded | 1, grounded | PASS |

"Grounded" means the reported byte range of the fixture file equals `greet`:
- Rust definition, attempt 3: `src/main.rs`, bytes 3–8, role `definition`.
- Rust references: `src/main.rs`, bytes 96–101, 127–132, and 3–8.
- TypeScript references: `src/main.ts`, bytes 9–14 and 46–51.
- TypeScript definition: `src/greet.ts`, bytes 16–21, role `definition`.

The check is [`check-gsem.ps1`](pilots/m9-rc-ee7482d/evidence/check-gsem.ps1). It is read-only and reads the fixture sources in [`gsem/fixture/`](pilots/m9-rc-ee7482d/gsem/fixture/). Its output for all three attempts is [`gsem-byte-check-all-attempts.json`](pilots/m9-rc-ee7482d/evidence/gsem-byte-check-all-attempts.json).

The Rust references and both TypeScript answers are byte-identical across the three attempts (section 5.4). Only the Rust definition differs.

Provider facts (attempt 3):
- rust-analyzer 1.99.1, LSP subprocess, Rust family. Each answer: `verification: Unverified`, `precision: Heuristic`.
- scip-typescript 0.4.0, SCIP import, TypeScript family. Import: 2 documents, 4 definitions, 6 references, `verification: Unverified`, `fallback_reason: none`.
- No archived semantic answer comes from a syntax provider. No syntax candidate was promoted.
- C++ on MMSP: NOT YET VERIFIED. clangd is deferred by D-M9-2. No C++ provider ran.

Semantic provider reliability: **OPEN FINDING**.
- The Rust definition came back in 1 of 3 attempts. All three Rust outputs carry `analysis wait timed out; answers may be empty`, including attempt 3.
- The harness bounds the Rust analysis wait at 15 s. No attempt shows that the analysis completed.
- Attempt 3 is a functional PASS from one complete run. This amendment does not claim that rust-analyzer is stable.
- The entry `notification: request:workspace/diagnostic/refresh` is a server-to-client request that LSP 3.17 defines to ask the client to refresh diagnostics. It is not, by itself, a failed answer (LSP 3.17 specification, Diagnostics Refresh; checked with Context7).

Harness summary limitation:
- The summary line `rust-analyzer definition bytes=` written by `g-sem.ps1` is empty in all three attempts, including attempt 3, where the definition is present. Its regex expects `"bytes":[a,b]`, while riltra emits `span.byte_start` and `span.byte_end`. The summary is not evidence. The JSON outputs are the primary evidence.
- `rust-analyzer semantic status: null` is the known behavior recorded in the [G-SEM amendment](m9-gsem-amendment.md), finding F4: LSP runs per query, so no status row exists.

Limits: fixture-level providers only. MMSP-scale semantic behavior was not measured.

### 5.2 G-INC: incremental correctness (PASS)

Acceptance test ([design gate, G-INC](m9-design-gate.md)): parity between incremental refresh and full rebuild on the final tree; `reparsed` limited to the changed set; timings and host/toolchain recorded; no speedup claim without measured data.

Procedure: frozen [`cp2/method/cold-index.ps1`](pilots/cp2/method/cold-index.ps1) (two cold runs). The main home is a copy of the first cold index, made with the same helper calls as `run-all.ps1` step 2. Then frozen [`cp2/method/incremental.ps1`](pilots/cp2/method/incremental.ps1): restore the pin, no-op refresh, change one file, incremental refresh, full rebuild of the same changed tree in a fresh home, fact digests of both, query parity, restore. Logs: [`cold-index.log`](pilots/m9-rc-ee7482d/evidence/cold-index.log), [`incremental.log`](pilots/m9-rc-ee7482d/evidence/incremental.log). Helper calls: [`step2-home-main.ps1`](pilots/m9-rc-ee7482d/evidence/step2-home-main.ps1). Outputs: [`bench/cp2/raw/incremental/`](pilots/m9-rc-ee7482d/bench/cp2/raw/incremental/).

The changed file is `colmap/src/colmap/mvs/patch_match_options.h`: `max_triangulation_angle` default `-1.0f` → `60.0f`.

| Check | Result |
|---|---|
| Baseline no-op refresh | `mode=incremental`, `reused=1475`, `reparsed=0`, `status=fresh` |
| One-file incremental refresh | `mode=incremental`, `reused=1474`, `reparsed=1` |
| Fact parity, 5 tables | files, symbols, imports, calls, and edges identical in rows and digests: 1475, 18785, 6680, 99720, and 106905 rows |
| Combined fact digest, changed tree, incremental = oracle | `9548a032eb001d4213a3aa097774f8478c57a4c01e2ce8c21287478d5344b5d9` ([delta-tree.json](pilots/m9-rc-ee7482d/bench/cp2/raw/incremental/delta-tree.json)) |
| Query parity | 12 of 12 identical; exit codes 0 on both sides ([_parity-summary.json](pilots/m9-rc-ee7482d/bench/cp2/raw/incremental/_parity-summary.json)) |
| Restore | snapshot HEAD `d62f06a…`, porcelain 0; main home re-indexed to the pin |
| Archive consistency (pinned tree) | the five table digests of the main home, after restore, equal the archived candidate cold1 digests (`b311bb6` era). Recorded in [pinned-home-main-facts.json](pilots/m9-rc-ee7482d/evidence/pinned-home-main-facts.json), combined digest `f60f2ac3797d017346b95b7cf4d82b94d638af22290513e768f13fc5ed71b9f5` |

Timings: one run per step, this host, debug build. Sources: `wall_s` in [`cold-index.log`](pilots/m9-rc-ee7482d/evidence/cold-index.log), `wall_ms` in [`incremental.log`](pilots/m9-rc-ee7482d/evidence/incremental.log).

| Step | Wall time |
|---|---|
| Cold index, run 1 / run 2 (`cold-index.ps1`, index only) | 123.8 s / 108.6 s |
| No-op refresh (baseline) | 43.4 s |
| One-file incremental refresh | 38.0 s |
| Full rebuild, oracle, fresh home, changed tree | 103.8 s |

These are single measurements. They are not a benchmark, and they support no general performance claim or speedup figure. In this run the one-file incremental refresh took less time than the no-op refresh. That was not investigated.

### 5.3 G-FSH: freshness (PASS)

Acceptance test ([design gate, G-FSH](m9-design-gate.md)): each scenario matches its pre-registered verdict and reasons, or reports `unknown` or `stale` honestly; no silent `fresh`.

Procedure: frozen [`cp2/method/freshness.ps1`](pilots/cp2/method/freshness.ps1) on the pinned snapshot and the main home. Expected outcomes come from the frozen matrix in [CP1 §C](m9-cp1-pilot-spec.md), written before execution. Outputs: [`bench/cp2/raw/freshness/`](pilots/m9-rc-ee7482d/bench/cp2/raw/freshness/). Log: [`freshness.log`](pilots/m9-rc-ee7482d/evidence/freshness.log).

| Scenario | Frozen action | Expected (pre-registered) | Observed `freshness` and `reasons` | Repo head / branch | Result |
|---|---|---|---|---|---|
| F0 | restore pin (baseline) | fresh | fresh; none | `d62f06a…` / none | PASS |
| F1 | change `max_triangulation_angle` default | stale, `content-changed` (`dirty` also expected) | stale; `dirty`, `content-changed` | `d62f06a…` / none | PASS |
| F2 | `git mv` `patch_match_cuda.h` | stale, `file-removed` and `file-added` | stale; `dirty`, `file-added`, `file-removed` | `d62f06a…` / none | PASS |
| F3 | `git rm` `patch_match_options.cc` | stale, `file-removed` | stale; `dirty`, `file-removed` | `d62f06a…` / none | PASS |
| F4 | detached checkout of `2b1bf42…` | stale, `head-changed` | stale; `head-changed` | `2b1bf42…` / none | PASS |
| F5 | `checkout -b m9-freshness-probe` | stale, `branch-changed`, not `head-changed` | stale; `branch-changed` | `d62f06a…` / `m9-freshness-probe` | PASS |

No silent `fresh` was observed in F1–F5. Every stale scenario reported its reasons. The snapshot was restored after each scenario and at the end (porcelain 0). Commit `2b1bf42…` is present in the snapshot, so F4 is a real head change.

### 5.4 Evidence index

Archive: [`pilots/m9-rc-ee7482d/`](pilots/m9-rc-ee7482d/README.md), 111 redacted files. SHA-256 of each archived file: [`SHA256SUMS.txt`](pilots/m9-rc-ee7482d/SHA256SUMS.txt). Redaction record per file, with the SHA-256 before redaction: [`REDACTION.tsv`](pilots/m9-rc-ee7482d/REDACTION.tsv). Text files are LF, as the repository requires.

Not archived: binaries, SQLite databases, the source archive, and build output. The full verification-host set of 191 files stays outside the repository. Its manifest SHA-256 is `e7c1746d151ef53470d96fd72f37096fc53219d4d637ff04b5b2e33ee78f9f44`, which identifies that local set only.

| Evidence (repository path) | SHA-256 (archived) | SHA-256 (before redaction) |
|---|---|---|
| `gsem/raw/rust-analyzer-definition.json` (G-SEM attempt 1) | `6015930d0507d0c9a811dd153a73722410d87ad95a47239b33a94c07775ee0be` | `7211f791d287d9e509ddbb0f2a6952c6fa31c9c1e50a6e129175334fb5c70c0f` |
| `gsem/raw-attempt2/rust-analyzer-definition.json` (attempt 2) | `6015930d0507d0c9a811dd153a73722410d87ad95a47239b33a94c07775ee0be` | `7211f791d287d9e509ddbb0f2a6952c6fa31c9c1e50a6e129175334fb5c70c0f` |
| `gsem/raw-attempt3/rust-analyzer-definition.json` (attempt 3) | `d5b958bf6f9d739cd588b1a6276403bc5b7569675fca94ba73ddf343fda6e220` | `2d4f60a63d545126ee49b6fba723b5f354ef408b784cf77251dc4715bd2aaa1e` |
| `gsem/raw-attempt3/rust-analyzer-references.json` (identical in attempts 1–3) | `4ac6807d0b9d64e6836ac1cf0cd2ac00352a7dac98d725282a2867186f7cc527` | `cd3a43b430db353a3dc335fd820b1a2743c6bda0feeb34bf08dfdf2cccdfd1e4` |
| `gsem/raw-attempt3/scip-typescript-query.json` (identical in attempts 1–3) | `46973c0a5adbcab676de3330ea6f5feafd3ea50719ea3b16a4876d392c329fbe` | `e78e3d4e3989f1a248a514d5f9f35618369c11e013e13d35aa6e60978b20629c` |
| `gsem/raw-attempt3/ts-definition-from-callsite.json` (identical in attempts 1–3) | `82dd5504140f273986002f6adef69f00342cc09dd6ef37b57f9e1ff43c6d1909` | `82dd5504140f273986002f6adef69f00342cc09dd6ef37b57f9e1ff43c6d1909` |
| `bench/cp2/raw/incremental/noop.json` | `29acb7702b0f7d3ad711c6d98f20fdb87d489444f38bfda6edbf1e6e066a2cca` | `2471effdb87822340de220eebe59a6903ff1b8677c11fd8d939e523be28ed857` |
| `bench/cp2/raw/incremental/delta.json` | `fd00b3865506fc204b308b8c88475c6f25c80cba9783857c57aac3def5ec8e78` | `a0fba0dcf738743e177f66cb6d4c0e39cc03b7a52de52cb62602bec7ca423f33` |
| `bench/cp2/raw/incremental/oracle.json` | `b96b9b97b425d30501e8f36fc5ff0aa1ef47a83210e7da9704fc371f10a048c3` | `e4de3809d8709f78c1832cbf9ffce853d43999a2257122bb7b2658736177cefa` |
| `bench/cp2/raw/incremental/delta-inc-facts.json` | `b114607e0d1317932714e0a4d2248131a06593d1f56382cf543d1042cf351784` | `b114607e0d1317932714e0a4d2248131a06593d1f56382cf543d1042cf351784` |
| `bench/cp2/raw/incremental/delta-oracle-facts.json` | `206b9ad255710578175bdb59b979737cfa9093251cbc4daa2881b2e6e0c42843` | `206b9ad255710578175bdb59b979737cfa9093251cbc4daa2881b2e6e0c42843` |
| `bench/cp2/raw/incremental/delta-tree.json` | `f8911f6d679ddfb643dec8ff4da9f24cac9d47c28943d23bb0a19a3d31d6272c` | `3af59960ff4221596df66e590df040482019c04461fa34c4cc65f38bd364cd0f` |
| `bench/cp2/raw/incremental/_parity-summary.json` | `f81d984a6e8d9b0f99430c3e080971f58c083f73a1d815ed2d0628def5cc6b20` | `56391c3eb475c7b8af0334376152351d99f9ffe0f4b810d36147e14e22eddc61` |
| `bench/cp2/raw/freshness/F0-baseline-index-status.json` | `a926f006871eb2e6b7bed44da0fdde0e49edcd27f5a64e31f753881ed0f2ce9b` | `4f2de14c05ef2ab5a41692d09fe0a9795c644c4ae5994ab30090b54fcccd0d5e` |
| `bench/cp2/raw/freshness/F1-modify-index-status.json` | `796e92af35f39e0247017eea17026f81c04e7ff70a44b4010608ab0b392903f6` | `8a71a8f42cbbcacc4e9d24ebbbb6ce4baac612e51ec19652952f6d80f0e39cb2` |
| `bench/cp2/raw/freshness/F2-rename-index-status.json` | `ddb36b23dba00ec3c7fc5fa3cc145e7f48eb2416a10619edb9608bf0eaf155a4` | `43ad1cd564cfbfd30c0067e98376a4133bb7ad699a5e9fb2b23bb300c3b0d755` |
| `bench/cp2/raw/freshness/F3-delete-index-status.json` | `8147fbcdaf2fdf34550e0cbe31a03f8043076f185170e6d3fb8bedd36d32f200` | `733be2ecf62bb2242566fa7e892ed677eb82682a387d262cf9a789646cbdc906` |
| `bench/cp2/raw/freshness/F4-head-change-index-status.json` | `2eef3e2c7d373e838aa5ab9cc831380c0c30a2ca0fe01c0209e36beb537aad6e` | `167122293c62d1d76d353b3b3fba2ff8ad3cd95aca51018893d85a040effbc42` |
| `bench/cp2/raw/freshness/F5-branch-change-index-status.json` | `98ea8379a8f8d0d7b4eeb317e356f8e4fb7b1eb18079cf7c06e7943b728e420b` | `986c12fc1cc1019c170edc30015b120050c32f92e2cf820c63cab88643f86348` |
| `evidence/pinned-home-main-facts.json` (pinned-tree fact tables) | `33537920733d499f6bc2f34a9e92a0bfb291f81d01b8579902664a366a038eb4` | `33537920733d499f6bc2f34a9e92a0bfb291f81d01b8579902664a366a038eb4` |

The Rust references, the TypeScript query, and the TypeScript definition files are byte-identical across attempts 1–3, in the archive and before redaction. The Rust definition files of attempts 1 and 2 are identical; attempt 3 differs. Redaction changed the digests of files that held local paths (placeholder counts in `REDACTION.tsv`); the before-redaction column keeps the earlier digests traceable.

## 6. Identity, evidence location, and test limits

| Item | Value |
|---|---|
| Source | `git archive` of `ee7482dfb12f0e88e3620ada5ce35f74f45041a3`. Tree `db762287a548e6a0c8b398721650898db710917c`. 1670 tracked files, all exported. Archive SHA-256 `BDF868882658DE44467CC3B8B25EF9DB5DF84E16A4B0A58807E3034600A0111E`. `Cargo.lock` SHA-256 `9B9BA4DC0D7E7AD6F653DBBA99B859E6A3FB9030EE73A3AA9858EF4E544EF905`. |
| Binary | `riltra`, debug profile, built with `cargo build --locked -p riltra` from that export into a target directory outside the repository. SHA-256 `36F2BB2CA18C36696AC0CA1DB6BBFE58236B1AD04D72B828AA9724999EB48ABD`. `riltra --version`: `riltra 0.1.0`. Build log: [`build-debug.log`](pilots/m9-rc-ee7482d/evidence/build-debug.log); run summary: [`summary.txt`](pilots/m9-rc-ee7482d/evidence/summary.txt). |
| Toolchain | rustc 1.99.1 (58cf8f9c3 2026-10-13); cargo 1.99.1 (5f94df478 2026-08-27); rustup default `stable-x86_64-pc-windows-msvc`; node v24.19.0; git 2.53.0.windows.2; PowerShell 7.6.9. |
| Providers | rust-analyzer 1.99.1 (58cf8f9c 2026-10-13); scip-typescript 0.4.0 (npm global, installed before this run and not changed during it). |
| Host | Windows 11 Pro; 16 logical cores. |
| Benchmark | MMSP. The original clone was only read (HEAD `fa5f74d940f29d04b0bb192a7c2fb7d31e3ac785`, porcelain 0 before and after). The test snapshot is a separate local clone at the pin `d62f06a576d8e76b705c8ecc57fc380f7ebc9c44` (tree `c5482ea00481675e0aa921538f560947b68dca12`), with `core.autocrlf=false` and `core.longpaths=true`. Porcelain was 0 after each restore. |
| Evidence location | Repository folder [`pilots/m9-rc-ee7482d/`](pilots/m9-rc-ee7482d/README.md): 111 redacted files, listed in section 5.4. The full verification-host set (191 files) is not archived; its manifest SHA-256 is `e7c1746d151ef53470d96fd72f37096fc53219d4d637ff04b5b2e33ee78f9f44`. |

Test limits:
- One host, one debug build, one run per G-INC step. Timings are not portable across hosts.
- G-SEM: fixture-level providers only, no MMSP-scale run. Reliability is an open finding (5.1).
- G-INC: one-file change only. No release profile. The oracle is a full rebuild of the same changed tree.
- G-FSH: scenarios on the pinned snapshot only.
- Harness self-tests were run before the scenarios: fact-digest tests, grader tests, harness-helper tests, and path-safety tests (all exit 0; logs in [`evidence/`](pilots/m9-rc-ee7482d/evidence/), exit codes in [`summary.txt`](pilots/m9-rc-ee7482d/evidence/summary.txt)).
- Not tested here: C++ semantic providers, CUDA, release-profile timing, any OpenCode or agent session, Codex, G-TOK, and macOS ARM64.
- Commands: the frozen scripts were run from the export of `ee7482d`. The additions are a helper that repeats the `run-all.ps1` step 2 calls, an orchestration script, a read-only TypeScript call-site query, and read-only checks (fixture byte ranges and the pinned fact-table comparison). The orchestration script is archived as [`run-verify.ps1`](pilots/m9-rc-ee7482d/evidence/run-verify.ps1), with local roots replaced by placeholders. The helper is [`step2-home-main.ps1`](pilots/m9-rc-ee7482d/evidence/step2-home-main.ps1).
- Archive: redacted. Path strings and line endings are the only changes, and reversing them reproduces each file's verification-host SHA-256. Binaries and databases are not archived. Reproduction needs the `ee7482d` export and the toolchain in this section.

## 7. Limitations and open decisions

- **Open finding: semantic provider reliability.** The Rust definition came back in 1 of 3 attempts (section 5.1). Owner options: keep the functional PASS with this finding recorded, or define a reliability rule (attempt count and acceptance threshold) and repeat. Changing the frozen harness is also an owner decision.
- macOS ARM64 is not covered ([acceptance amendment §10](m9-preprelease-acceptance-amendment.md)).
- Agent integration: OpenCode UNAVAILABLE (expired key), Codex NOT TESTED, Claude Code smoke only, G-TOK DEFERRED (section 4.4).
- Not repeated at `ee7482d` (owner decision): the G-SYN span checks, the G-KNW round trip, and the MCP D5 sequence.
- Warnings from run annotations, not gate failures:
  - Node.js 20 is deprecated. `actions/checkout@v4` (guard and package jobs) and `actions/upload-artifact@v4` (package jobs) run on Node.js 24 ([GitHub changelog](https://github.blog/changelog/2025-09-19-deprecation-of-node-20-on-github-actions-runners/)).
  - `ubuntu-latest` migrates to Ubuntu 26, beginning 2026-10-19 ([actions/runner-images#14748](https://github.com/actions/runner-images/issues/14748)). The run used `ubuntu-latest`.
  - Action and runner-label pinning: not decided.
- `CLAUDE.md` is untracked and was not touched. Its wording is an owner decision.
- Rewriting history with local machine paths: owner-gated, not done.
- `v0.1.0` merge, tag, and publish: separate owner GO.
- M10: owner decision.

## 8. Conclusions

- **Release-candidate workflow: PASS.** Run [40263559935](https://github.com/RyanAraaful/riltra/actions/runs/40263559935) on `ee7482d`: four of four jobs succeeded. Three native archives were produced and verified (section 3).
- **M9 core release gate: PASS within the D-1–D-6 scope.** Basis: 4.1 (direct, including the G-SEM, G-INC, and G-FSH repeats), 4.2 (carried), and section 5.
- **G-SEM functional correctness: PASS (attempt 3).** Accepted by the owner for functional correctness on `ee7482d`.
- **Semantic provider reliability: OPEN FINDING.** rust-analyzer returned the Rust definition in 1 of 3 attempts. Stability is not established.
- **G-INC: PASS.** Five-table fact parity, 12 of 12 query answers identical, one-file delta `reparsed=1`, no-op `reparsed=0`.
- **G-FSH: PASS.** F0–F5 match their pre-registered outcomes. No silent `fresh` was observed.
- **Full agent integration: not verified** (4.4).
- **Public release: NOT AUTHORIZED.** No tag or GitHub release exists. `v0.1.0` needs a separate owner GO.

## 9. Verification of this amendment

- Remote `master`, PR #7, both runs, and the artifact digests were read from GitHub on 2026-10-26.
- Archive SHA-256 values were computed locally and matched against the `.sha256` file in each artifact. Archive contents were listed, not extracted.
- Each "unchanged since" claim is a `git diff` from the evidence commit to `ee7482d` for `crates/`, `Cargo.toml`, and `Cargo.lock`.
- G-SEM: each reported span was checked against the fixture bytes (read-only, [`check-gsem.ps1`](pilots/m9-rc-ee7482d/evidence/check-gsem.ps1)). All grounded items equal `greet`.
- G-INC: fact digests were recomputed with the frozen `db-mutation-check.cjs`. The pinned-tree tables were compared with the archived candidate digests: 5 of 5 identical.
- Full verification-host manifest: 191 files, every SHA-256 re-verified (0 mismatches). An earlier manifest had wrong relative paths; it was regenerated before this amendment, and the evidence files were not changed.
- Archive: 111 files. `sha256sum -c SHA256SUMS.txt` reports OK for every file. Reversing the redaction reproduced the verification-host SHA-256 of every file (0 mismatches). The repository scanner (`security/scan.sh dir`) reports no leaks in the archive. The timings and combined digests in 5.2 match the logs and JSON in the archive.
- Context7: `/github/docs` confirmed the `node20` and `node24` runtime keys. `/sourcegraph/scip-typescript` confirmed `index --cwd --output`. The LSP 3.17 specification confirmed that `workspace/diagnostic/refresh` is a server-to-client request. The forced-runtime rule and the `ubuntu-latest` date are not in Context7; they rest on the run annotations.
- Every repository link in this file resolves to an existing path. The repository scanner (`security/scan.sh dir`) reports no leaks in this file.
