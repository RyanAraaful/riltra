# M10 Task 6 - consolidation and owner decision input

**Date:** 2026-10-27
**Branch:** `m10-evidence-distribution`
**Audit snapshot:** `e4cba6a891aed5ce7a77e508f6ef6c1184a9f657` — the branch state examined before this consolidation was committed; it is not the current branch HEAD.
**Status:** archival consolidation after owner audit and CI verification. This document was committed and pushed to `origin/m10-evidence-distribution` after the audit snapshot. No new benchmark, no source/API/schema change, no frozen-evidence edit. No merge, tag, release, or Phase B.
**Scope:** `docs/m10-design-gate.md` section 10; read-only consolidation over M10 Tasks 2 to 5.

## 1. Method and boundaries

- Read all six local governance documents: `00_START_HERE.md` through
  `05_DISTRIBUTION_AND_INSTALLATION.md`. They were not modified.
- Reviewed the committed Task 2 to 5 evidence, its commit history, and the
  design gate. Every status below is copied from that evidence; no run is
  re-graded and no outcome is promoted.
- Re-verified evidence integrity read-only: `SHA256SUMS.txt` manifests, the
  branch/remote SHA, worktree state, source/Cargo diffs, frozen-evidence diffs,
  tags/releases, and GitHub Actions run metadata. Manifest verification is an
  integrity check, not a new measurement.
- Did not run an agent session, benchmark, installer, `dist`, release workflow,
  or Phase B download.
- Separated the four questions: engine correctness, agent benchmark, installer
  Phase A, and release readiness.

## 2. Baseline and evidence verification

### 2.1 Branch state

| Item | Value |
|---|---|
| Repository | `RyanAraaful/riltra` |
| M10 base | `7f86891c0942cc1d7efd6ec29c6c0b6b1606e727` (`origin/master`) |
| Freeze commit | `6b800fba6625e89cbd9428f988c6e9a1877b893a` |
| Branch | `m10-evidence-distribution` |
| Audit snapshot (pre-Task 6) | `e4cba6a891aed5ce7a77e508f6ef6c1184a9f657` |
| Remote at audit snapshot | `origin/m10-evidence-distribution` at the same SHA |
| `origin/master` | `7f86891c0942cc1d7efd6ec29c6c0b6b1606e727`, unchanged |
| Worktree | only `?? CLAUDE.md`; the six governance documents are ignored |
| Tags / GitHub Releases | none |
| Merge state | branch is not merged; only `origin/m10-evidence-distribution` contains this branch history |

Task commits reviewed:

| Task | Commit | Content |
|---|---|---|
| 2 | `dbeb237` | Claude Code pilot evidence |
| 3 | `38e92a4` | Codex pilot evidence |
| 4 | `aa348ec` | OpenCode pilot evidence |
| 5 | `60f17e7` | Option B installers and Phase A CI |
| 5 | `e4cba6a` | hosted Phase A evidence in the decision record |
| 6 | this document | consolidation committed after the audit snapshot; see branch history |

### 2.2 Local governance protection

`git check-ignore -v` resolves all six paths to `.git/info/exclude` lines 54 to 59:

| Path | Exclude line |
|---|---|
| `docs/riltra-guide/00_START_HERE.md` | 54 |
| `docs/riltra-guide/01_MASTER_DESIGN.md` | 55 |
| `docs/riltra-guide/02_PRODUCT_SPEC.md` | 56 |
| `docs/riltra-guide/03_BENCHMARK_STANDARD.md` | 57 |
| `docs/riltra-guide/04_SCOPE_AND_GOVERNANCE.md` | 58 |
| `docs/riltra-guide/05_DISTRIBUTION_AND_INSTALLATION.md` | 59 |

`git ls-files -- <the six paths>` returns zero entries. `git diff --cached --name-only`
is empty. `CLAUDE.md` remains untracked and was not touched. None of the six
documents appears in a commit or in staging.

### 2.3 Evidence integrity

| Evidence set | Commit | Manifest result |
|---|---|---|
| Frozen M10 inputs | `6b800fb` | 12 of 12 files PASS |
| Task 2 Claude Code | `dbeb237` | 232 of 232 files PASS |
| Task 3 Codex, top-level | `38e92a4` | 151 of 151 files PASS |
| Task 3 Codex, pilot directory | `38e92a4` | 144 of 144 files PASS |
| Task 4 OpenCode | `aa348ec` | 197 of 197 files PASS |
| Task 5 decision record and CI | `60f17e7`, `e4cba6a` | files present; CI runs below |

The frozen M10 inputs are unchanged since the freeze commit. The M9 frozen
evidence under `docs/pilots/m9-rc-ee7482d/` and `docs/pilots/cp7a/` is unchanged
from the M10 base. No `crates/`, `Cargo.toml`, `Cargo.lock`, or
`.github/workflows/release-candidate.yml` change exists between `7f86891` and
the audit snapshot.

The five Task 2 to 5 commits contain only `docs/`, `release/`, and
`.github/workflows/ci.yml` paths. No product source, public contract, schema,
dependency, or release workflow was changed.

### 2.4 Evidence index and limitations

Primary evidence and manifests:

- Task 2: [README](pilots/m10/task2-claude-code/README.md),
  [AUDIT](pilots/m10/task2-claude-code/AUDIT.md),
  [metrics](pilots/m10/task2-claude-code/metrics.tsv),
  [manifest](pilots/m10/task2-claude-code/SHA256SUMS.txt).
- Task 3: [pilot README](pilots/m10/task3-codex/pilot-20261026T162908300Z/README.md),
  [grading](pilots/m10/task3-codex/pilot-20261026T162908300Z/grading.json),
  [tool-use adjudication](pilots/m10/task3-codex/pilot-20261026T162908300Z/tool-usage-adjudication.json),
  [manifest](pilots/m10/task3-codex/pilot-20261026T162908300Z/SHA256SUMS.txt).
- Task 4: [README](pilots/m10/task4-opencode/README.md),
  [grades](pilots/m10/task4-opencode/grading/grades.md),
  [reviewer report](pilots/m10/task4-opencode/grading/reviewer-report.md),
  [manifest](pilots/m10/task4-opencode/SHA256SUMS.txt).
- Task 5: [decision record](m10-task5-decision.md),
  [install.ps1](../release/install.ps1),
  [install.sh](../release/install.sh),
  [test-install.ps1](../release/test-install.ps1),
  [test-install.sh](../release/test-install.sh),
  [install guide](../release/INSTALL.md),
  [CI workflow](../.github/workflows/ci.yml).
- Engine: [M9 RC amendment](m9-release-candidate-gate-amendment-2026-10-26.md),
  [M9 consolidation](m9-consolidation.md),
  [G-SEM amendment](m9-gsem-amendment.md),
  [G-KNW amendment](m9-gknw-amendment.md),
  [M7 validation](m7-validation.md).
- Design and status rules: [M10 design gate](m10-design-gate.md) sections 5, 6,
  and 10.

Known evidence limits:

| Task | Limit | Effect |
|---|---|---|
| 2 | Raw transcripts stay outside the repository; `RAW-SHA256SUMS.txt` covers 214 raw files. One shared profile (D1). Two Arm R workspaces contain cache files (D7). A4 test-binary SHA-256 was not recorded (D13). | Run statuses stand; full raw reproduction is local-only. |
| 3 | Raw runset is in restricted local storage; nine supplemental audit/adjudication originals are missing, sanitized copies remain. Both A4 runs read operator skill files outside their snapshots. | Exploratory evidence only; not a clean benchmark. |
| 4 | Five slots are UNAVAILABLE; raw SQLite databases are not committed, but their hashes are in provenance. | Three-session sample; no Riltra-effect estimate. |
| 5 | Phase B was not tested; native macOS/Ubuntu evidence is hosted-only; Windows runners are administrators. | Phase A only; no Release or least-privilege claim. |
| Engine | M9 evidence at `ee7482d`; no new engine run in M10. Semantic provider reliability remains an open finding. | Carried status, not a fresh measurement. |

## 3. Engine results carried from M9 and M7

This section is a consolidation of existing evidence, not a new engine run. The
M9 release-candidate core gate is **PASS within the D-1 to D-6 scope at
`ee7482d`**; four C++ capability criteria remain **UNMET** and were accepted by
the owner as pre-release limitations on 2026-10-26. The exact gate record is in
`docs/m9-release-candidate-gate-amendment-2026-10-26.md` and
`docs/m9-consolidation.md`.

| Area | Status at the cited evidence | Evidence | Limit carried forward |
|---|---|---|---|
| Native CLI, registry, JSON, health | PASS (M9 RC scope) | M9 RC amendment section 4.1; CLI tests | Git is needed for fingerprint; pre-1.0 contracts are bounded |
| Syntax, definitions, spans | PASS (M9 G-SYN scope) | M2/M6/M9 G-SYN; 18,785 symbols (17,878 named), 99,720 syntax calls, 0 span mismatch in the recorded check | Name/spans are not semantic identity; the statistic is not a universal guarantee |
| Occurrences and references | CANDIDATE; C++ `D1-C6` UNMET | M6, M9 E7/CP1 | C++ name census is not bound references; real-repository semantic scale is unverified |
| Callers and callees | CANDIDATE; C++ `D2-C6` UNMET | M6, M9 CP1/CP4 | Resolved targets are limited; name match can be ambiguous |
| Trace | CANDIDATE; C++ `D3-C5` UNMET | M6, M9 G-GRF | Static path, not proof of runtime execution; indirect chains incomplete |
| Impact | Honesty PASS; C++ `D4-C5` UNMET | M9 G-GRF, CP1 D.4 | Positive affected-set capability is UNMET; no false `SAFE` in the tested corpus only |
| Freshness and incremental | PASS (M9 RC) | G-FSH F0 to F5; G-INC parity 12/12 | Timings are host-specific; one-file incremental delta only |
| Semantic provider | Functional PASS; reliability OPEN FINDING | M9 G-SEM attempt 3; TypeScript fixture; Rust definition returned in 1 of 3 attempts | Results remain `Unverified`; `Current` is not proven; C++ real-project provider not verified; CUDA unsupported |
| Knowledge | PASS in tested scope | M7 565 tests; M9 73-check round trip | Structural backup verification is not authentication; offline/concurrency limits remain |
| MCP | Core PASS | 12 tools advertised; G-MCP transport/contract/error/stdout/read-only checks | 11 read-only tools plus one provider-backed subprocess tool; a client may expose 11 |
| Cross-platform binaries | M9 RC native smoke PASS on three x64 targets | M9 RC run 40263559935; Task 5 Phase A | No published release; ARM64 out of scope; installers are Phase A only |

No engine measurement was added in Task 6. The M10 agent pilots do not change
any engine status.

**Component PASS is not an end-to-end product claim.** The M7 G-KNW PASS covers
the knowledge store in its tested scope; the M9 engine gates cover code
intelligence components in their recorded scopes. Neither proves the combined
**what/impact/why** workflow: no end-to-end user workflow that starts from a
source fact, follows a proven relation, and reaches an engineering
decision/revision has been validated. That integration claim remains **NOT
PROVEN** in M10. This does not downgrade M7 G-KNW or any M9 gate; it records
that their scope is component-level.

## 4. Agent benchmark: M10 Tasks 2 to 4

### 4.1 Totals

The frozen pilot has 24 slots: 3 clients x 2 arms x 4 tasks, one first attempt
per slot. Eighteen slots reached an agent turn. **Zero agent Riltra MCP calls
and zero agent `riltra` CLI invocations were observed in any arm R run.** The
Task 3 harness setup ran the CLI to build/index snapshots; those setup calls are
not agent calls.

| Client | Slots | Reached turn | Final run status | Agent Riltra calls | Protocol qualification |
|---|---:|---:|---|---:|---|
| Claude Code (Task 2) | 8 | 8 | 4 PASS, 4 FAIL | 0 | One shared authenticated profile instead of a new profile per session (D1) |
| Codex (Task 3) | 8 | 7 | 4 PASS, 2 FAIL, 2 UNAVAILABLE | 0 | Contaminated: both A4 runs read operator skill files outside their snapshots |
| OpenCode (Task 4) | 8 | 3 | 3 completed; 5 UNAVAILABLE | 0 | Partially unavailable; provider HTTP 402 before five turns |
| **Total** | **24** | **18** | — | **0** | No confirmatory phase, no G-PAR |

### 4.2 Claude Code, Task 2

Evidence: [README](pilots/m10/task2-claude-code/README.md),
[AUDIT](pilots/m10/task2-claude-code/AUDIT.md),
[metrics](pilots/m10/task2-claude-code/metrics.tsv),
[manifest](pilots/m10/task2-claude-code/SHA256SUMS.txt).

| Run | Arm | Task | Final status | Initial status | Factual result | Note |
|---|---|---|---|---|---|---|
| R-A1-a1 | R | A1 | PASS | PASS | 5/5 locations | none |
| B-A1-a1 | B | A1 | FAIL | FAIL | 5/5 locations | strict diff rejected a preamble line |
| R-A2-a1 | R | A2 | FAIL | FAIL | 24/24 call sites | strict diff rejected a preamble line |
| B-A2-a1 | B | A2 | PASS | PASS | 24/24 call sites | none |
| R-A3-a1 | R | A3 | FAIL | FAIL | value `128`; 2/2 readers | strict diff rejected backslash separators |
| B-A3-a1 | B | A3 | FAIL | FAIL | value `128`; 2/2 readers | strict diff rejected backslash separators |
| R-A4-a1 | R | A4 | PASS | FAIL (grader v1) | reviewer test passes; fix confined to `escape()` | both grades retained |
| B-A4-a1 | B | A4 | PASS | FAIL (grader v1) | reviewer test passes; fix confined to `escape()` | both grades retained |

The four strict FAILs are retained. The factual column shows the requested
content was correct in all four. The A4 initial FAIL and final PASS are both
retained; the later grade is explained by the documented grader v2 correction,
not by rewriting the first result.

### 4.3 Codex, Task 3

Evidence:
[pilot README](pilots/m10/task3-codex/pilot-20261026T162908300Z/README.md),
[grading](pilots/m10/task3-codex/pilot-20261026T162908300Z/grading.json),
[tool-use adjudication](pilots/m10/task3-codex/pilot-20261026T162908300Z/tool-usage-adjudication.json),
[manifest](pilots/m10/task3-codex/pilot-20261026T162908300Z/SHA256SUMS.txt).

| Run | Arm | Task | Strict | Task outcome | Protocol status | Note |
|---|---|---|---|---|---|---|
| R-A1-a1 | R | A1 | PASS | PASS | PASS | completed turn reconciled after postprocessing interruption |
| B-A1-a1 | B | A1 | UNAVAILABLE | UNAVAILABLE | PASS | Codex rejected the Arm B config before an agent turn |
| R-A2-a1 | R | A2 | PASS | PASS | PASS | completed turn reconciled after postprocessing interruption |
| B-A2-a1 | B | A2 | UNAVAILABLE | UNAVAILABLE | PASS | native executor failure; no task facts; no retry |
| R-A3-a1 | R | A3 | PASS | PASS | PASS | none |
| B-A3-a1 | B | A3 | PASS | PASS | PASS | attempted a missing in-snapshot skill path; no content returned |
| R-A4-a1 | R | A4 | FAIL | FAIL | FAIL | read two operator skill files outside the snapshot |
| B-A4-a1 | B | A4 | FAIL | FAIL | FAIL | read one operator skill file outside the snapshot |

Counts: strict format and task outcome both 4 PASS, 2 FAIL, 2 UNAVAILABLE.
The pilot is protocol-contaminated and must not be described as a clean
benchmark or a successful launcher run.

### 4.4 OpenCode, Task 4

Evidence: [README](pilots/m10/task4-opencode/README.md),
[grades](pilots/m10/task4-opencode/grading/grades.md),
[reviewer report](pilots/m10/task4-opencode/grading/reviewer-report.md),
[manifest](pilots/m10/task4-opencode/SHA256SUMS.txt).

| Run | Arm | Task | Strict | Factual | Task | Note |
|---|---|---|---|---|---|---|
| A1-R / `runs/R-A1-a1` | R | A1 | FAIL | PASS | PASS | completed; preamble and fence fail strict diff |
| A1-B / `runs/B-A1-a1` | B | A1 | FAIL | PASS | PASS | completed; preamble and fence fail strict diff |
| A2-R / `runs/R-A2-a1` | R | A2 | FAIL | PASS | PASS | completed; preamble, fence, trailing prose fail strict diff |
| A2-B / `runs/B-A2-a1` | B | A2 | UNAVAILABLE | UNAVAILABLE | UNAVAILABLE | HTTP 402 `Insufficient Balance` before an agent turn |
| A3-R / `runs/R-A3-a1` | R | A3 | UNAVAILABLE | UNAVAILABLE | UNAVAILABLE | HTTP 402 before an agent turn |
| A3-B / `runs/B-A3-a1` | B | A3 | UNAVAILABLE | UNAVAILABLE | UNAVAILABLE | HTTP 402 before an agent turn |
| A4-R / `runs/R-A4-a1` | R | A4 | UNAVAILABLE | UNAVAILABLE | UNAVAILABLE | HTTP 402 before an agent turn |
| A4-B / `runs/B-A4-a1` | B | A4 | UNAVAILABLE | UNAVAILABLE | UNAVAILABLE | HTTP 402 before an agent turn |

Three sessions completed; five used zero tokens and no agent turn. The owner
closed the runset on 2026-10-27 without a retry. The completed sample is three
sessions and cannot estimate a Riltra effect.

### 4.5 Cross-client interpretation

- **No Riltra usage:** all three pilots show zero agent Riltra calls. Any
  difference between Arm R and Arm B is not attributable to Riltra.
- **No cross-client ranking:** Claude Code, Codex, and OpenCode are reported
  separately. No cross-client ranking or pooled token metric is made.
- **Pilot only:** one first attempt per slot, no confirmatory phase. Run-level
  status only; no dimension or G-PAR status is assigned.
- **Strict format versus factual content:** several FAILs are formatting-only.
  They remain FAIL under the frozen rule; the factual column is descriptive and
  does not change a grade.
- **Task 3 is contaminated:** the A4 filesystem reads outside the snapshots
  prevent a clean protocol claim.
- **Task 4 is partially unavailable:** five slots are infrastructure failures,
  not model or engine failures.
- **Historical M8/M9 results are not transferred:** the M8 token observation
  and earlier agent integrations describe older surfaces; they are not current
  M10 evidence.

### 4.6 Historical M8/M9 agent status, not M10 evidence

The frozen M8/M9 records remain authoritative. They are listed here only so
their statuses are not silently promoted into M10 results.

| Surface | Historical status | Limit |
|---|---|---|
| M8 OpenCode MCP/CLI | Real MCP use recorded; CP7-A token observation (~30.5% lower gross tokens/completed on that corpus) | One client, host, model, and corpus; not transferable to M9/M10 |
| M9 OpenCode | UNAVAILABLE: expired API key; no session ran | Not a Riltra correctness failure |
| M9 Claude Code | Smoke PASS: single session, five MCP calls | No workflow or token claim |
| M9 Codex | NOT TESTED | Not converted into an M10 result |

## 5. Installer Task 5: Option B Phase A

### 5.1 Decision and layout

The Task 5 decision record selects **Option B**: the existing M9 archive and
smoke pipeline plus two minimal installers. Option A (`dist 0.33.1`) is recorded
as rejected after the feasibility review, with one static re-check of the
retained generated installer; no Option A run, release, or CI result is claimed.
No new dependency or distribution framework was added.

Evidence: [decision record](m10-task5-decision.md),
[install.ps1](../release/install.ps1), [install.sh](../release/install.sh),
[test-install.ps1](../release/test-install.ps1),
[test-install.sh](../release/test-install.sh),
[install guide](../release/INSTALL.md),
[CI workflow](../.github/workflows/ci.yml).

| Target | Command | Managed payload and documents |
|---|---|---|
| Windows x64 | `%LOCALAPPDATA%\Programs\Riltra\bin\riltra.exe` | same dedicated Riltra tree |
| Linux x64 | `$HOME/.local/bin/riltra` symlink | `$HOME/.local/share/riltra/` |
| macOS Intel x64 | `$HOME/.local/bin/riltra` symlink | `$HOME/.local/share/riltra/` |

Each payload carries the native binary, `LICENSE`, `NOTICE`,
`THIRD_PARTY_LICENSES.txt`, `sbom.cdx.json`, `INSTALL.md`, and the
`.riltra-manifest` ownership list.

### 5.2 Phase A evidence

Local Phase A passed on Windows x64 under Windows PowerShell 5.1 and PowerShell
7, and on Linux x64 under WSL Ubuntu, including the installed CLI/MCP smoke.
WSL is a development diagnostic, not a substitute for native Ubuntu.

Hosted Phase A is the `installer-phase-a` matrix in `.github/workflows/ci.yml`,
one job per pinned native runner. Run
[40511229393](https://github.com/RyanAraaful/riltra/actions/runs/40511229393)
on `60f17e7` completed 7 of 7 jobs successfully:

| Job | Result | Duration |
|---|---|---|
| `installer (windows-2025)` | PASS, `test-install: all checks passed` | 4m 8s |
| `installer (ubuntu-24.04)` | PASS, `test-install: all checks passed` | 1m 57s |
| `installer (macos-15-intel)` | PASS, `test-install: all checks passed` | 5m 26s |
| `check (windows-latest)` | PASS | 5m 42s |
| `check (ubuntu-latest)` | PASS | 2m 36s |
| `check (macos-15)` | PASS | 5m 25s |
| `security` | PASS | 21s |

Run [40512511191](https://github.com/RyanAraaful/riltra/actions/runs/40512511191)
on `e4cba6a` (the evidence-record commit) also completed successfully.

Each installer job built the archive from the same commit with
`cargo build --release --locked`, ran `release_docs --check`, packaged with
`release/package.sh`, and covered: positive install, reinstall, checksum
mismatch/missing/malformed with the old install preserved and no staging left,
unsupported OS and ARM64, location/ownership and no-elevation checks, foreign
executable/symlink/payload entries, archive traversal/absolute/symlink entries,
the static no-elevation scan, and the installed CLI/MCP smoke.

### 5.3 Installer status and limits

- **Phase A: PASS** on the local Windows/Linux diagnostics and on all three
  hosted native runners.
- **Phase B: NOT TESTED.** No actual GitHub Release asset download was run.
  Phase B needs separate owner authorization (O-12) and a published
  pre-release (O-13).
- Native macOS and Ubuntu execution exists only on hosted runners; there is no
  local native host for those platforms.
- GitHub-hosted Windows runners run as an administrator. The no-admin property
  rests on location and static checks, not on an unprivileged-account test.
- macOS ARM64 and Linux ARM64 are refused by design; no ARM64 archive is built
  or claimed.
- No code signing, notarization, package manager, GUI installer, auto-update,
  or PATH modification is provided.
- **O-15 remains an owner review item.** The decision record states Option B was
  selected; the local governance document still marks formal acceptance of the
  decision record and POSIX layout as pending. This consolidation records that
  ambiguity rather than declaring it resolved.

**Review outcome:** hosted Phase A is PASS and the decision record is ready for
owner audit. The owner review of the decision record and layout (O-15) is
pending; no reviewer has approved release.

## 6. Release readiness

Three separate things are recorded here: the technical Phase A result, the
authorization state, and the untested Phase B step.

| Area | Technical status | Authorization status | Separate owner decision |
|---|---|---|---|
| Engine core | PASS within the M9 D-1 to D-6 scope; four C++ criteria UNMET | no new release authorization | accept carried limitations |
| Installer Phase A | PASS (run 40511229393) | committed on the branch only; not merged | O-15 acceptance |
| Merge to master | NOT DONE | NOT AUTHORIZED | owner GO |
| Tag and GitHub pre-release | NONE | NOT AUTHORIZED | O-13 |
| Phase B release-asset download | NOT TESTED; requires a published pre-release | NOT AUTHORIZED | O-12, after O-13 |
| Public claims | NOT APPROVED | NOT AUTHORIZED | section 10 owner decision |
| G-PAR | NOT ASSESSED | no claim permitted | O-9 if confirmatory is wanted |
| ARM64 | OUT OF SCOPE | not requested | separate support decision |

**Sequencing.** Phase B is a validation of downloading the asset from a
*published* pre-release. It is therefore downstream of O-13, not a precondition
for the first pre-release: publishing the first pre-release provides the asset
that Phase B is meant to test. Phase B must pass before any final/stable
publication decision, but its absence does not make the first pre-release
impossible.

**Readiness verdict:** the Phase A installer checks are technically PASS. No
merge, tag, pre-release, or public-claim authorization has been granted, and
Phase B has not run. The project is **not ready for a final/stable release**; a
first pre-release remains an owner decision (O-13) and Phase B follows it. A
PASS in this document does not authorize publication.

## 7. G-PAR: five dimensions, not assessed

The design gate keeps the M9 G-PAR rule: PASS requires superiority in at least
three of five pre-registered dimensions with no critical correctness
regression. The pilot rule is explicit: **G-PAR is not assessed from pilot
runs.** The confirmatory plan of 120 runs was not executed.

| Dimension | Status | Reason |
|---|---|---|
| D1 source-grounded correctness | NOT ASSESSED | pilot only; zero Riltra calls; no confirmatory comparison |
| D2 change freshness | NOT ASSESSED | A3 evidence exists, but no confirmatory run and no Riltra use |
| D3 changed-file refresh cost | NOT ASSESSED | no confirmatory harness timing; no 5-repetition median |
| D4 agent token and task success | NOT ASSESSED | provider token data exists, but no confirmatory phase and no Riltra effect |
| D5 maintainability and deployment burden | NOT ASSESSED for superiority | Phase A install results are absolute PASS; no authorized external comparator |

No dimension is marked PASS, FAIL, or UNAVAILABLE for G-PAR. No superiority
claim is made. The M8 token observation is not transferable to this corpus.

## 8. Status register

### 8.1 FAIL retained

| Task | Run(s) | Status | Scope |
|---|---|---|---|
| 2 | B-A1-a1, R-A2-a1, R-A3-a1, B-A3-a1 | FAIL | strict frozen diff; factual content correct |
| 2 | R-A4-a1, B-A4-a1 | initial FAIL; final PASS | both grades retained; grader v2 correction documented |
| 3 | R-A4-a1, B-A4-a1 | FAIL | task outcome and protocol isolation violation |
| 4 | A1-R, A1-B, A2-R | strict FAIL | factual and task PASS; three completed sessions |

### 8.2 UNAVAILABLE retained

| Task | Run(s) | Cause | Owner action |
|---|---|---|---|
| 3 | B-A1-a1 | Codex config parser rejected Arm B before an agent turn | accept exploratory gap, or authorize a new clean run with a dated GO |
| 3 | B-A2-a1 | native executor failure before task work | same |
| 4 | A2-B, A3-R, A3-B, A4-R, A4-B | provider `Insufficient Balance` (HTTP 402) before an agent turn | owner closed without retry on 2026-10-27; reopen only with a new GO |
| 5 | Phase B | no authorized published pre-release and no Phase B download | O-12 and O-13 |

### 8.3 UNMET retained

| Capability | Status | Treatment |
|---|---|---|
| `D1-C6` C++ method identity/references | UNMET | accepted pre-release limitation, 2026-10-26 |
| `D2-C6` C++ external construction-site attribution | UNMET | accepted pre-release limitation |
| `D3-C5` complete multi-hop chain including indirect dispatch | UNMET | accepted pre-release limitation |
| `D4-C5` C++ affected set | UNMET | accepted pre-release limitation |

The four UNMET capabilities are not relabeled by M10 or by this consolidation.

### 8.4 Not assessed or out of scope

- **G-PAR D1 to D5:** NOT ASSESSED; confirmatory phase not run.
- **macOS ARM64 and Linux ARM64:** OUT OF SCOPE; refused by the installers.
- **Code signing, notarization, package managers, GUI installers, auto-update:**
  OUT OF SCOPE without separate approval.
- **CUDA semantic, Go/Java/C# core support, C++ real-project semantic
  provider:** OUT OF SCOPE or unverified, as recorded in the M9/M10 evidence.
- **M9 Codex integration:** historical NOT TESTED; not converted into an M10
  result.
- **M10 confirmatory 120-run phase:** NOT RUN; requires owner GO.

## 9. Owner decision queue

| ID | Decision needed | Why it matters | Blocks |
|---|---|---|---|
| O-15 | Accept the Option B decision record and the POSIX payload/symlink layout | Formalizes what is already implemented and tested | Task 5 adoption record |
| O-9 | Authorize the confirmatory 120-run phase, or accept the pilot limitation | G-PAR cannot be assessed without it | G-PAR, superiority claims |
| O-12 | Authorize Phase B Release-asset download | Phase A used local archives only | Phase B |
| O-13 | Authorize a pre-release tag and public pre-release | No tag or GitHub Release exists | Phase B and publication |
| O-7 | Confirm the pinned runner labels are an acceptable M9-equivalent change | Phase A used `windows-2025`, `ubuntu-24.04`, `macos-15-intel` | Reproducibility record |
| Task 2 review | Keep or amend the strict-format rule and accept grader v2 | Four strict FAILs and the A4 initial FAIL are owner-visible | Confirmatory design |
| Task 3 review | Accept the contaminated pilot or authorize a clean rerun | A4 read operator files outside the snapshots | Any clean Codex claim |
| Section 10 | Approve public wording and any release text | G-PAR is not assessed and no external comparator exists | Publication |

Merge, tag, release, and Phase B each require a separate explicit owner GO.
None is granted by this consolidation.

## 10. Public reporting boundary

If the owner later authorizes public reporting:

- Do not name external tools or comparators. An authorized comparator is
  "Comparator A".
- Client names may appear in client-specific sections, as in M9.
- Do not claim superiority. G-PAR was not assessed; there is no confirmatory
  result and no external comparison.
- Put host, client version, attempt count, date, and limitations beside every
  result.
- Make no performance or accuracy claim beyond the measured corpus, and no
  claim about other systems.
- Release text must state the pilot or confirmatory scope, the absence of an
  external-product comparison, and any incomplete competitive evidence.
- The owner decides separately: (a) what the results may say publicly, and
  (b) the `v0.1.0` tag, Release, and publication.

## 11. Audit notes and non-blocking discrepancies

1. **O-15 wording differs between records.** `docs/m10-task5-decision.md`
   states Option B was selected and Option A rejected; the local governance
   document still marks O-15 acceptance pending. Treat formal acceptance as an
   owner action, not as proven by this consolidation.
2. **Task 4 manifest name.** The Task 4 README refers to
   `MANIFEST-SHA256.txt`; the committed file is `SHA256SUMS.txt`, and its 197
   entries pass. This is a documentation-only naming mismatch in frozen
   evidence; it was not edited.
3. **Task 2 protocol deviations.** The shared profile (D1), the strict-format
   failures (D3/D4), and the grader v2 correction (D6) are recorded in the Task
   2 README and AUDIT. They remain owner review items for any confirmatory
   design.
4. **Task 3 contamination.** The A4 filesystem reads outside the snapshots
   prevent a clean protocol claim. The pilot is retained as exploratory
   evidence, not as a clean benchmark.
5. **Windows elevation limitation.** Hosted Windows runners are administrator
   accounts; the no-admin property is supported by location and static checks,
   not by an unprivileged-account execution.

## 12. Conclusion

This document satisfies the Task 6 consolidation scope: it separates engine,
agent benchmark, installer Phase A, and release readiness; preserves every
historical FAIL, UNMET, UNAVAILABLE, and the unassessed G-PAR state; includes
the Task 5 decision record and the hosted Phase A evidence; and lists every
missing item with its cause and owner action. It records two limits explicitly:
the end-to-end what/impact/why workflow is not proven even though M7 G-KNW and
the M9 component gates PASS in their tested scopes, and Phase B follows the
first pre-release rather than blocking it.

It adds no measurement and changes no source, API, schema, or frozen evidence.
It is ready for owner audit, not for a final/stable release. The document is
committed on `m10-evidence-distribution`; merge, tag, pre-release, release, and
Phase B remain separate owner decisions.
