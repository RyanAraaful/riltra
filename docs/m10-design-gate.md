# M10 design gate: evidence distribution

Status: **proposed, not approved; revision 4.** Prepared 2026-10-26 on branch `m10-evidence-distribution`, created from `origin/master` at `7f86891c0942cc1d7efd6ec29c6c0b6b1606e727` (M9 closure, PR #8). Revision 4 adds the frozen procedure (`docs/pilots/m10/freeze/PROCEDURE.md`) and the A4 isolation preflight (§14). Revision 3 prepared the frozen inputs under `docs/pilots/m10/freeze/`. This document runs no agent benchmark, runs no `dist init`, changes no M9 record, and makes no commit, push, PR, merge, tag, or release. Owner approval of the frozen inputs is required before the first Task 2 session (§3.1). The distribution technology stays provisional until the bounded feasibility assessment (§9.2) is reviewed.

## 1. Frozen baseline and inputs

| Item | Value |
|---|---|
| Repository | `RyanAraaful/riltra` |
| Base commit | `7f86891c0942cc1d7efd6ec29c6c0b6b1606e727` (`origin/master`) |
| Base tree | `b2bd50be9ec829fd122635696a2cb95c29fe2bcc`, 1785 tracked files |
| M9 release candidate | `ee7482dfb12f0e88e3620ada5ce35f74f45041a3` (PR #7 merge); archive in [pilots/m9-rc-ee7482d/](pilots/m9-rc-ee7482d/README.md) |
| M9 gate definitions | [m9-design-gate.md](m9-design-gate.md) §4. G-PAR is an M10 target there |
| M9 legend and decisions D-1 to D-6 | [m9-preprelease-acceptance-amendment.md](m9-preprelease-acceptance-amendment.md) §1, §3 |
| M9 release-candidate evidence | [m9-release-candidate-gate-amendment-2026-10-26.md](m9-release-candidate-gate-amendment-2026-10-26.md) |
| Owner-private blueprint | Referenced by name only. It is gitignored and was not present in the checkout used for this document. Nothing from it is copied here (open item O-1). |

M9 facts this design relies on:
- Core gates PASS at `ee7482d`: G-NAT, G-LIC, G-CLI, G-MCP core, G-SEC, G-SEM, G-INC, G-FSH, regression, and CI. G-GRF capability stays UNMET and is an accepted pre-release limitation.
- G-SEM has an open finding: rust-analyzer returned the Rust definition in 1 of 3 attempts. The default Riltra arm therefore does not register semantic-provider tools (§3.3).
- Agent integration at M9: OpenCode UNAVAILABLE (expired API key; no session ran), Claude Code smoke PASS (single session, no token claim), Codex NOT TESTED.
- The M9 release-candidate workflow (`.github/workflows/release-candidate.yml`) runs only from `master` (guard job). It packages Windows x64, Linux x64, and macOS x64 (`macos-15-intel`). macOS ARM64 is not covered.
- C++ semantics and CUDA are not covered (M9 §7; acceptance amendment D-5).
- D-M9-6 (native archives, checksums, and notices; no new installer or signing infrastructure) is an M9 decision, and its M9 record is unchanged. Owner correction (2026-10-26): cross-platform installation is an M10 requirement, not an M9 violation (§9).
- The M9 acceptance baseline for packaging is `release/package.sh`, `release/smoke.sh`, and the release-candidate workflow. §9.6 maps each condition to M10.

## 2. Task order

| # | Task | Executor | May start when | Output |
|---|---|---|---|---|
| 1 | Branch and design-gate preparation | Claude Code | Now | Branch `m10-evidence-distribution` and this document. No commit, push, PR, tag, or release |
| 2 | Independent Claude Code benchmark (pilot) | Claude Code, isolated session | Owner approves the frozen design (§3.1) | Raw evidence per §7, committed, pushed, reviewed |
| 3 | Independent Codex benchmark (pilot) | Codex, isolated session | Task 2 completed, committed, pushed, and reviewed | Same as Task 2 |
| 4 | Independent OpenCode benchmark (pilot) | OpenCode, isolated session | Task 3 completed, committed, pushed, and reviewed; owner supplies a valid OpenCode credential (O-2) | Same as Task 2 |
| 5 | Cross-platform distribution: bounded feasibility, then implementation | Implementation by OpenCode, as planned. No automatic reassignment (O-2) | Owner explicitly authorizes Task 5. Not gated by Tasks 2 to 4, and not by any release publication (O-13) | Feasibility decision record (§9.2); distribution configuration or scripts; smoke results (§9.4) |
| 6 | Evidence consolidation and owner release decision | Claude Code, read-only over evidence; owner decides | Tasks 2 to 5 completed, or recorded UNAVAILABLE with cause, and reviewed | `docs/m10-consolidation.md` and owner decision (§10) |

Rules:
- **The benchmark chain is strictly sequential.** Task 2, then Task 3, then Task 4. Each agent runs its own pilot independently, then commits, pushes, and receives review. The next agent begins only after that.
- **An unavailable agent stops the chain, not the distribution track.** The unavailable run is recorded as UNAVAILABLE with its evidence (§6). The chain stops for an owner decision. It does not skip to the next agent.
- **Task 5 may proceed after an unavailable agent only with explicit owner approval.** It is not gated by Tasks 2 to 4 in dependency terms, but it starts only on that approval. The approval is per start: a Task 5 authorization does not authorize Phase B (O-12).
- **No automatic reassignment.** Task 5's intended implementer is OpenCode. If OpenCode is unavailable, Task 5 STOPs for an owner decision: the owner supplies a valid credential, or names a different implementer by dated amendment. This design does not choose one.
- **Release authorization is separate.** Pre-release publication is not a prerequisite for designing or implementing distribution. Tags and GitHub Releases need separate owner GO (O-5, O-13).
- Each push needs the owner's explicit yes.
- Task 1 is not an evaluation. The authoring session is not an independent Claude Code run. Task 2 starts in a clean session (§3.4).

## 3. Test design

### 3.1 Corpus, snapshots, and freeze

- **Snapshot S1 (default):** the Riltra repository at `7f86891c0942cc1d7efd6ec29c6c0b6b1606e727`. Every run receives a fresh copy made with `git archive` from that commit. The author's working tree is never used. Arm R uses a Riltra binary built from the same commit with `cargo build --locked`.
- **Snapshot S2 (optional, owner GO):** MMSP at the M9 pin `d62f06a576d8e76b705c8ecc57fc380f7ebc9c44`, as a read-only isolated clone. It is outside the default acceptance: its content is third-party, and it would be sent to model providers. Use requires owner approval (O-3).
- **Freeze:** the benchmark inputs are the files under `docs/pilots/m10/freeze/`: prompts (`prompts/A1.txt` to `A4.txt`), expected answers (`expected/`), frozen edits (`edits/`), the verification script (`verify/verify-freeze.sh`), and the procedure with exact commands and pins (`PROCEDURE.md`). `SHA256SUMS.txt` lists their SHA-256 values. Revision 3 creates them uncommitted. The freeze commit is made after owner approval, pushed, and reviewed like any task commit, and it precedes the first Task 2 session. The acceptance rules (§4 to §6) are frozen in the same commit.
- **Expected answers and derivation:** derived from the pinned commit with `git grep -n -w`, `git show`, and `head -c` / `tail -c` byte slices. Each occurrence was then classified by reading it in context, as in M9 CP1. No Riltra binary, Riltra index, or agent output was used. `verify/verify-freeze.sh` re-derives A1 to A3 and the A4 patches. With `--cargo` it also runs the frozen A4 command on the pinned tree and on the defect copy. The owner reviews the expected answers before the freeze.
- **Frozen harness:** the scripts that create snapshot copies, apply the frozen edits, prepare the Riltra index, and run verification commands are part of the freeze. They are not edited during Tasks 2 to 6.
- **Changes after freeze:** any change to prompts, expectations, edits, rules, or commands needs a dated amendment. Every run it touches is relabeled.

### 3.2 Four agent tasks

The same prompt text (`freeze/prompts/`, Appendix A) goes to every client and both arms. Every target below was checked at the pinned commit by `verify/verify-freeze.sh`: definitions, byte slices, call-site lines, and edit anchors.

| Task | Prompt goal | Frozen answer | Run passes when |
|---|---|---|---|
| A1 Definition | Locate the definition (the `fn` item) of five exact names | `canonical_root` (`crates/cli/src/paths.rs:57`, bytes 1704-1718), `contains` (`paths.rs:72`, 2180-2188), `resolve_recorded_root` (`crates/cli/src/main.rs:184`, 8095-8116), `escape` (`crates/cli/src/json.rs:7`, 356-362), `grammar_versions` (`crates/cli/src/store.rs:27`, 1289-1305). Offsets are 0-based from the start of the file; the end is exclusive | All five exact: file, line, and both offsets |
| A2 Call sites | List every call site of `canonical_root` and of `resolve_recorded_root` in `crates/cli/src`, including test modules. Only call lines are listed | 24 lines in `expected/A2.txt`: 17 for `canonical_root` and 7 for `resolve_recorded_root`. The identifiers also occur as fields, variables, parameters, imports, strings, and comments (many lines). None of those is a call, and none is listed | Every expected line found, no extra line |
| A3 Post-edit truth | Frozen edit (`edits/a3-edit.patch`) in the run's copy: `const MAX_JSON_DEPTH: usize = 64;` at `crates/cli/src/knowledge.rs:267` becomes `= 128;`. Readers: `knowledge.rs:345` and `knowledge.rs:361`. The edited copy compiles (checked) | Post-edit value `128`, and the two reader lines | Exact post-edit answer. A stale value reported as current is a critical failure. Reporting `stale` and refreshing before answering is allowed |
| A4 Fix and test | Frozen defect (`edits/a4-defect.patch`) in the run's copy: delete the arm `'\n' => out.push_str("\\n"),` at `crates/cli/src/json.rs:13` in `escape`. Frozen command: `cargo test --locked --bin riltra json::tests` | Validated: the defect state exits 101 with `3 passed; 1 failed`, and only `json::tests::escapes_quote_backslash_and_newline` (`crates/cli/src/json.rs:30` at the pinned commit) fails. The restored state exits 0 with `4 passed; 0 failed` (`expected/A4.txt`). The diff touches only `escape` | Command exits 0, no test code changed, no regression in `json::tests` |

Shared run rules:
- Prompts are frozen text. The operator reads them verbatim and does not coach, correct, or hint during a run. Any intervention is logged, and the run is labeled ASSISTED and reported apart from unassisted runs.
- Each agent runs its own tests inside its own client. The reviewer reruns only the frozen verification command (A4) or the frozen verification script (A1 to A3) on the archived snapshot copy. The reviewer never grades from the agent's own report.
- Time caps: 15 minutes for A1 to A3; 30 minutes for A4, including build. Build time is logged separately. A timeout is a FAIL for that run, unless the failure log shows a client or infrastructure error. That case is UNAVAILABLE for that run.
- **Pilot (exploratory):** one independent run per client, arm, and task: 3 clients × 2 arms × 4 tasks = **24 runs**. Each run is its own session. Harness timings for D3 are not agent runs and are not counted in the 24.
- **Attempts and retries:** every attempt is a run folder. A retry is allowed only after a logged infrastructure or client error, and the error is logged before the retry starts. Failures, retries, and UNAVAILABLE results are all kept and reported. Counts are shown for first attempts and for all attempts.
- **Pilot status:** pilot results are exploratory and are not evidence of general statistical superiority. From the pilot, only run-level outcomes (PASS, FAIL, or UNAVAILABLE, with cause) are reported.
- **Confirmatory plan** (fixed before any pilot run; executed only on separate owner GO after pilot review): 5 independent attempts per client, arm, and task, 120 runs in total. All attempts are counted. A run may be excluded only for an infrastructure error logged before its outcome is inspected. Confirmatory runs are analyzed with the §5 rules. Pilot runs are reported separately and are not pooled into confirmatory counts. No significance claim is made unless the owner approves a named test by dated amendment before the confirmatory phase starts.

### 3.3 Arms

- **Both arms** have the same native client tools: file reading, file search, text search, editing, and shell. The frozen verification commands run through the shell in both arms.
- **Arm R (Riltra):** the native tools, plus the Riltra MCP server (`riltra mcp`, read-only tools) and the `riltra` CLI on the shell path. The harness prepares the Riltra index as `PROCEDURE.md` §4 orders it: A1 and A2 on the unedited snapshot, A3 before the A3 edit, and A4 after the A4 defect. The Arm R binary is built once from the pinned tree (`PROCEDURE.md` §3). Index build time is logged separately and is not counted in agent tokens or task time. The semantic-provider tool and its LSP provider are not registered (O-10).
- **Arm B (source-search baseline):** the native tools only. No Riltra binary on the path, no Riltra MCP server, no index.
- **Usage recording (Arm R):** Arm R may use native search. This is allowed, not a violation. The run records Riltra MCP calls by tool name, `riltra` CLI calls, native search calls, the tool used for the first search, and every fallback (a native search used after a Riltra call returned an error, an empty result, or a stale status). Usage comes from the transcript (C5).
- **Arm X (external comparator):** none authorized. Nothing is installed automatically. Arm X may be used only after the owner confirms in writing the comparator's license and authorization. Public reports then name it only as "Comparator A". Arm B results are not evidence against an external product (§5).

### 3.4 Isolation

- Each client session starts with a new user profile and home directory. No instruction files, skills, plugins, or memory from the operator's machine are loaded. The client configuration is recorded as a SHA-256 of the sanitized file set.
- Each session's workspace holds only its snapshot copy and its frozen prompt. Earlier evidence folders stay outside the workspace, and the agent never receives them.
- No shared state between clients: separate profiles, workspaces, and `RILTRA_HOME` directories. The dependency cache (`CARGO_TARGET_DIR`) is shared across runs but sits outside every workspace, and its path is recorded. Before each agent session and each reviewer run, the harness runs `cargo clean -p riltra` in the workspace. Control C1 showed that without this step a copy reused another copy's defective test binary (B-14). The Rev 4 preflight passed with this strategy (§14). Per-copy target directories hit LNK1104 on the validation host, and the shared cache avoided it in the observed runs (B-15).
- Provider credentials come from the owner, one per client. They never go into the repository, a prompt, or a transcript. A credential that appears in a log is redacted before commit.
- Start-up check, Task 2 preparation: confirm that each client runs with a clean profile. If one cannot, its runs are UNAVAILABLE for isolation, and the owner decides (B-4).

### 3.5 No cross-agent ranking

Claude Code, Codex, and OpenCode are not ranked against each other (M9 D-1 and D-2). Each client's Arm R and Arm B outcomes are reported separately under the §5 rules.

## 4. Acceptance criteria (same for every client)

| Code | Criterion | Failure |
|---|---|---|
| C1 | Every reported location resolves to the expected construct in the snapshot | Miss or wrong location. Critical in Arm R when Arm B answers that item correctly |
| C2 | No file, symbol, line, or call site appears that is not in the snapshot | Critical |
| C3 | After the frozen edit, no stale fact is stated as current | Critical |
| C4 | A4 command exits 0, no test code changed, no regression in `json::tests` | Regression of a previously passing test is critical |
| C5 | Tool use is recorded from the transcript (§3.3): Riltra MCP calls, `riltra` CLI calls, native search calls, and fallbacks. A claimed tool use that the transcript does not show fails the run. A fallback to native search is allowed and recorded | FAIL for that run, only for an unshown claim |
| C6 | Authentication or client failure is reported as UNAVAILABLE with its log, not as a Riltra correctness failure (M9 rule) | Not a FAIL |

Per run: PASS when every criterion for its task holds. Per cell: passing attempts out of attempts run.

## 5. G-PAR: five dimensions

M9 §4 defines G-PAR. Its rule is kept: PASS requires superiority in at least three of five pre-registered dimensions, with no critical correctness regression. The five dimensions are the pre-registered dimensions of M9 §4: (1) source-grounded correctness, (2) change freshness, (3) changed-file refresh cost, (4) agent token and task success, (5) maintainability and deployment burden. The thresholds below are fixed by this document.

Scope rules:
- **The baseline is not a competitor.** Arm B is a generic source-search baseline. Superiority over Arm B is not evidence of superiority over an external code intelligence product. No public text may claim superiority over an external product on the basis of M10 results.
- **Per-client reporting.** Each client's Arm R and Arm B outcomes are reported separately. No ranking of Claude Code, Codex, or OpenCode is produced, and no token metric is pooled across clients.
- **Shared dimensions.** D3 is a harness measurement without an agent. D5 describes the distributed product. Both are reported once and apply to every client's count.
- **Superiority** for a dimension means that Arm R meets the dimension's rule against Arm B, as stated in the table.

| # | Dimension | Measured by | Rule | Arm R | Arm B |
|---|---|---|---|---|---|
| D1 | Source-grounded correctness | A1 and A2 exact-match rate; fabrication count | Per client: Arm R exact-match rate strictly above Arm B, with zero C2 failures | Measured | Measured |
| D2 | Change freshness | A3 outcome | Per client: Arm R passes A3 in every attempt with no C3 failure, and Arm B fails at least one A3 attempt. Arm B reads files directly, so superiority is unlikely by construction | Measured | Measured |
| D3 | Changed-file refresh cost | Harness timing, no agent: time from the frozen edit to a correct answer (Arm R: incremental refresh plus query; Arm B: re-search plus read). Pilot: one timing per arm. Confirmatory: median of 5 repetitions. Arm R also records the `reparsed` count | Arm R median below Arm B median; `reparsed` equals the changed files; fact parity with a full rebuild | Measured | Measured |
| D4 | Agent token and task success | Completed A1 to A4 attempts and provider-reported tokens | Per client with complete token data: Arm R task success at least Arm B's, and Arm R tokens per completed attempt at least 10 % lower | Measured | Measured |
| D5 | Maintainability and deployment burden | Install steps, manual steps, runtime dependencies, and time to first working query on the hosted runners of §9 | Superiority only against an authorized external comparator. Without one, D5 cannot show superiority, and it is UNAVAILABLE for superiority. The §9 install results are reported as absolute results | Recorded | Not applicable: no install step |

Tokens per completed attempt = all tokens of all attempts in the cell, including failed ones, divided by the number of completed attempts. A cell with no completed attempt fails D4 for that client.

G-PAR outcome, per client:
- **PASS:** at least three of D1 to D5 hold for that client, with no critical correctness regression and complete evidence.
- **FAIL:** every dimension is evaluated and fewer than three hold, or the open dimensions cannot bring the count to three.
- **UNAVAILABLE:** missing evidence could still change the outcome. Name each missing item and its cause.
- A critical correctness regression fails that client's G-PAR, whatever the count.
- **Pilot:** G-PAR is not assessed from the pilot. No dimension or G-PAR status is assigned from pilot runs.
- **Confirmatory:** G-PAR uses the confirmatory plan (§3.2).
- A threshold changed after the first run is an amendment, and affected results are relabeled.
- A FAIL and "no superiority" are allowed, recorded outcomes. This design does not predict them.
- If competitive evidence is incomplete or does not establish superiority, the limitation is reported (§10).

## 6. Status rules

Use the M9 legend (acceptance amendment §3), with these M10 rules:
- **PASS:** evidence exists for the criterion at the stated commit, and every raw log it relies on is archived.
- **FAIL:** the frozen criterion is not met. This includes a comparison where superiority is required and not shown.
- **UNAVAILABLE:** evidence cannot be obtained. The cause is named (expired credential, missing runner label, client not installed, usage not reported). Never recorded as PASS or FAIL.
- **DEFERRED:** only by a dated owner decision.
- **UNMET:** a capability Riltra does not have. Used only for the carried M9 G-GRF items.
- **NOT TESTED** is not used for a required run. A required run that did not happen is UNAVAILABLE with its cause.
- **Pilot runs** carry run-level status only: PASS, FAIL, or UNAVAILABLE, with cause. Dimension and G-PAR status come only from the confirmatory phase (§5).
- **Retries** do not replace earlier results. Each attempt keeps its own status.

## 7. Provenance and raw evidence per run

Each run records these fields in `provenance.json`:
- **Riltra:** source commit, binary SHA-256, build command, `riltra --version`, `rustc --version`, `cargo --version`.
- **Snapshot:** base commit, tree SHA, `git archive` SHA-256, SHA-256 of the frozen edit patch (A3, A4), SHA-256 of the prompt, SHA-256 of the expectations file.
- **Client:** name, CLI version, model identifier as the client reports it, session start and end in UTC, arm, attempt number, sanitized configuration hash, assisted flag.
- **Attempt:** attempt number, `retry_of` if any, and the logged error that justified the retry.
- **Index (Arm R):** commit, index build command, build time, and the `RILTRA_HOME` label.
- **Riltra usage (Arm R):** Riltra MCP calls by tool name, `riltra` CLI calls, native search calls, the first search tool, and fallback events.
- **Host:** OS and build, CPU, RAM. One host runs every agent session. Host names and user paths are replaced with placeholders such as `<HOST>`, `<REPO>`, `<HOME>`.
- **Tokens:** provider-reported input, output, cache-read, and cache-write counts for each session. No local re-estimation. A client that reports no usage makes its token fields UNAVAILABLE.
- **Timing:** wall clock from prompt submission to final answer. Build time for A4 is logged separately. Timings are not compared across hosts.
- **Transcript:** tool calls with arguments, redacted, for criterion C5 and the usage record above.
- **Failures:** verbatim client and tool errors with exit codes, redacted.
- **Limitations:** one paragraph per run: client version, attempt count, assisted flag, and any deviation from the frozen procedure.

## 8. Evidence layout and rules

- Path: `docs/pilots/m10/<task>-<client>/`, with `README.md`, one folder per attempt under `runs/<arm>-<task>-a<attempt>/`, `SHA256SUMS.txt`, and `REDACTION.tsv` when redaction changes a file. Frozen files live in `docs/pilots/m10/freeze/`.
- Append-only. An agent may add folders. It never edits or deletes an earlier folder. A correction is a new dated file that points at the file it corrects.
- Redaction: no absolute local paths, host names, keys, or account identifiers in tracked files (CLAUDE.md). Run `bash security/scan.sh dir <path>` on each evidence folder before commit. Exit 3 means the scanner is missing, and the step fails closed.
- Frozen M9 evidence (`docs/pilots/m9-*`, `docs/pilots/cp7a/`) is not modified.
- Text evidence is LF, as `.gitattributes` requires.
- Every commit in Tasks 2 to 6 carries a DCO sign-off (`git commit -s`).

## 9. Task 5: cross-platform distribution

### 9.1 Requirements (owner, 2026-10-26)

- **Targets:** Windows x64 (`x86_64-pc-windows-msvc`), Linux x64 (`x86_64-unknown-linux-gnu`), and macOS Intel x64 (`x86_64-apple-darwin`). macOS ARM64 is outside the accepted scope until it is separately supported and verified. No ARM64 target is built, published, or claimed.
- **Source:** GitHub Releases is the distribution source.
- **Binary:** a native binary per target. Users need no Rust, Node.js, or Python.
- **Installers:** PowerShell installation on Windows; shell installation on Linux and macOS. Both are generated or managed through one distribution system (option A, §9.2) or provided by minimal scripts (option B, §9.2).
- **User-level install:** no administrative privileges. No `sudo`, no `RunAs`, no machine-wide path, no registry or PATH edit. The installer prints the PATH line the user may add.
- **Unsupported platforms:** an unsupported operating system or architecture makes the installer exit non-zero, name the detected platform and the supported list, and install nothing.
- **Checksum:** SHA-256 verification before installation. A missing checksum or a mismatch exits non-zero before any file is installed.
- **Smoke tests:** installation and CLI/MCP smoke tests on GitHub-hosted runners (§9.4, §9.5).
- **Install location (both options):** Windows `%LOCALAPPDATA%\Programs\Riltra\bin\riltra.exe`; Linux and macOS `$HOME/.local/bin/riltra`. Removal is manual: delete the install directory and the link.
- **Out of scope without separate owner approval:** package managers (Homebrew, winget, Scoop, Chocolatey, apt, npm, and similar), GUI installers (MSI, pkg, dmg), code signing, notarization, auto-update, and any new dependency.

### 9.2 Feasibility check (bounded; Task 5, before any adoption)

Compare two options:
- **Option A:** `dist` (formerly `cargo-dist`), one shared distribution configuration that generates the per-target archives, the shell installer, and the PowerShell installer.
- **Option B:** the existing M9 archives (`release/package.sh`, unchanged) plus two minimal scripts, `install.ps1` and a shared `install.sh`.

Scope bound: the check runs in an isolated copy of the base or RC tree, under the Task 5 authorization. It may build locally with `dist`. It does not run `dist init` against the active repository, change the M9 release workflow, create a tag or release, or add a dependency. Its output is a decision record that the owner reviews before adoption.

Evaluation items, for both options:
1. **M9 workflow compatibility.** The gates that must stay are: release build with `--locked`, `release_docs --check`, archive packaging, and the clean-install smoke (§9.6).
2. **Archive contents.** LICENSE, NOTICE, THIRD_PARTY_LICENSES.txt, sbom.cdx.json, and INSTALL.md must be present in each archive, generated by the same `release_docs` step. Option B keeps the M9 layout: one top-level folder. Option A's `include` option adds listed files, relative to the config file, with no globs, at the archive root. That root-level layout differs from M9. The owner accepts or rejects the difference.
3. **SHA-256 and failure behavior.** A `.sha256` file sits next to each archive, in the format `sha256sum` reads. Test: a copy with one changed byte must fail before installation, leave no install, and remove staging. Option A's default is `checksum = "sha256"`. Context7 documentation says fetching installers can use the checksums to validate downloads. The generated installer's exact check, its order relative to extraction, and its failure output are not confirmed. They must be tested.
4. **Platform support.** The `targets` list names the three targets only. Each target has an explicit runner. Unsupported-platform behavior is tested. Context7 documentation of the shell installer lists caveats about platforms other than Linux and macOS, so explicit handling must be shown, not assumed.
5. **Configuration and maintenance.** Option A adds a dist configuration, a generated CI workflow, a pinned tool version (`cargo-dist-version`), and an upgrade path. `dist init` writes its configuration into `dist-workspace.toml` or into `Cargo.toml` metadata, depending on version. Option B adds two scripts, their tests, and the smoke job.
6. **Dependencies and licensing.** Option A adds the `dist` tool to CI. Third-party indexes report it as dual-licensed Apache-2.0 OR MIT. Confirm this from the project's own license file before adoption. Option B adds no dependency. It uses built-in tools: `curl`, `tar`, `sha256sum` or `shasum -a 256`, and PowerShell built-ins.
7. **Unintended side effects.** Option A's generated release workflow creates the GitHub Release in its host step by default (Context7: `github-release` default `auto`, host phase). Its trigger and its `pr-run-mode` setting must be checked in the isolated copy. Publication must be disabled or gated behind owner GO. The generated workflow must not replace `release-candidate.yml`. Option B adds only a smoke job.
8. **Installed contents.** Option A's shell installer keeps only the binary, per its documented caveats. The notices stay in the archive but not in the install location. Option B installs the whole archive folder. The owner decides whether the installed location must hold the notices.

Decision rule:
- Prefer option A if it meets every requirement in §9.1 and every item above, with equal or lower maintenance complexity than option B.
- Otherwise select option B and record the reasons in the decision record.
- Support for three operating systems is not enough to accept option A.
- If option A is adopted, these conditions apply: `publish-jobs` is empty; no Homebrew, npm, or MSI output; no updater or auto-update feature is enabled; release creation is not performed by the generated workflow until owner GO; the `targets` list excludes ARM64; the generated workflow is reviewed and does not replace `release-candidate.yml`.

### 9.3 Option A configuration intent (for the feasibility check; not applied now)

- `targets`: the three targets in §9.1 only.
- `installers`: `shell` and `powershell`.
- `checksum`: `sha256`.
- `include`: `LICENSE`, `NOTICE`, `release/INSTALL.md`, and the two generated documents, `THIRD_PARTY_LICENSES.txt` and `sbom.cdx.json`. How dist receives the generated files, which the M9 `release_docs --out` step writes, is part of item 2.
- `install-path`: the cascade `["$LOCALAPPDATA/Programs/Riltra/bin", "~/.local/bin"]`. Windows uses the first entry, and POSIX systems fall through to the second, because `LOCALAPPDATA` is unset there.
- Runners: `github-custom-runners` pins `x86_64-apple-darwin` to `macos-15-intel`, and the other two targets to explicit labels (O-7).
- `publish-jobs`: empty.
- `cargo-dist-version`: pinned to the version tested in the feasibility copy.

### 9.4 Test plan (both options; the same smoke for both)

- **Hosted runners:** `windows-2025`, `ubuntu-24.04`, and `macos-15-intel`, as explicit labels. M9 used `windows-latest` and `ubuntu-latest` (`release-candidate.yml`). Pinning changes the image. The change is recorded, and it must not reduce any M9 check (O-7).
- **Phase A (pre-publication; allowed once Task 5 is authorized):** test against existing M9 RC archives, if still retained, or against archives built from the base commit. Option B archives come from `release/package.sh`. Option A archives come from a local build in the isolated copy. No GitHub Release is created or downloaded. Each installer's test hook takes a local archive: `-ArchivePath` (Windows) and `--archive` (POSIX) for option B. For option A, the feasibility check selects a local-source method that does not change installer logic.
- **Phase B (actual Release asset download):** requires separate owner authorization (O-12) and a published pre-release, which is separately authorized (O-13). The same checks run after download from the GitHub Release URL.
- **Per platform:**
  1. **Positive:** install into a fresh test home (`HOME` on POSIX, `LOCALAPPDATA` on Windows). Then run the smoke in §9.5.
  2. **Re-install:** install again over the existing install. Results match step 1.
  3. **Negative, checksum mismatch:** change one byte in a copy of the archive. Expect a non-zero exit, nothing installed, and no staging directory left behind.
  4. **Negative, missing checksum:** remove the `.sha256` copy. Expect a non-zero exit and nothing installed.
  5. **Negative, unsupported platform:** a test-only shim reports an unsupported operating system or architecture. Expect a non-zero exit, a message that names the platform and the supported list, and nothing installed.
  6. **Location:** every installed path lies under the test home. On POSIX, every file is owned by the invoking user.
  7. **Static check on both scripts or generated installers:** no elevation calls (`sudo`, `RunAs`), no machine-wide paths (`/usr/local`, `HKLM`, `Program Files`), and every write goes to the install location or staging.
- **Limitation:** GitHub-hosted Windows runners run as an administrator account. The no-admin property rests on steps 6 and 7, not on an unprivileged account. An unprivileged-account test is a separate owner decision.

### 9.5 Smoke checks (preserved from M9 `release/smoke.sh`)

Run against the installed binary, not the source tree, with an isolated `RILTRA_HOME`:
- **Archive content** (when the archive is available): LICENSE, NOTICE, THIRD_PARTY_LICENSES.txt, sbom.cdx.json, and INSTALL.md are present and non-empty.
- **Binary:** `version` output begins with `riltra `; `help` lists `riltra mcp`; `status` reports networking disabled.
- **Index and search:** `init` registers the fixture; `index` reports `status_at_write` fresh; `search --symbol alpha` finds `alpha`; `search --occurrences` reports binding `unbound` and role `member-access`; `callers` reports a `POTENTIAL_CALL` candidate.
- **MCP stdio:** stdout carries only JSON-RPC lines; twelve `riltra_*` tools are advertised; a source-grounded call returns `alpha`; a classified error returns `not_registered`; stderr is empty.
- **Pass line:** `smoke: all checks passed`, archived with the run.

### 9.6 M9 acceptance baseline and how M10 preserves it

| M9 condition (source) | M10 preservation |
|---|---|
| Release-candidate runs only manually or on a `v*-rc*` tag, and only from `master`. It never creates a release, tag, signature, or installer (`release-candidate.yml` header and guard job) | Unchanged. The workflow is not replaced. Any generated workflow is a separate file and is not wired to tags or publication until owner GO |
| Release build `cargo build --release --locked -p riltra` on every target (package job) | Option B unchanged. Option A must still build release-optimized and `--locked`. Checked in feasibility |
| `release_docs --check` prints `no drift` before packaging (package job) | Preserved in both options as a gate before packaging |
| Archive `riltra-<version>-<platform>.<ext>` holds one folder with the binary, LICENSE, NOTICE, THIRD_PARTY_LICENSES.txt, sbom.cdx.json, and INSTALL.md (`release/package.sh`; `release/smoke.sh` lines 37 to 42) | Option B unchanged. Option A: content equivalence required, and the root-level layout difference is decided at feasibility (§9.2 item 2) |
| `.sha256` next to each archive, in `sha256sum` format (`release/package.sh`) | Option B unchanged. Option A uses `checksum = "sha256"`, same format |
| Clean-install smoke: `version`, `help`, `status`, `init`, `index`, `search`, `occurrences`, `callers` (`release/smoke.sh`) | Both options run the §9.5 smoke against the installed binary |
| MCP stdio: JSON-RPC only on stdout, twelve tools, source-grounded call, classified error, empty stderr (`release/smoke.sh`) | Both options run the same checks (§9.5) |
| Three native targets: Windows x64, Linux x64, macOS x64 on `macos-15-intel`; macOS ARM64 not covered (RC matrix; RC amendment §2) | Both options use the same three targets. ARM64 stays excluded (§9.1) |
| Artifact upload per platform (`upload-artifact` step) | Both options keep per-platform artifacts for Phase A |
| Runner labels `windows-latest`, `ubuntu-latest`, `macos-15-intel` (RC matrix) | Pinning is a change (O-7). Its equivalence is recorded at feasibility |

### 9.7 Release mechanics (Phase B only)

- A pre-release tag `v<version>-rc.N` on a commit already on `master` matches the M9 trigger `v*-rc*`. It is not needed for design, feasibility, or Phase A (O-5).
- A workflow may create a tag or publish a GitHub Release only after separate owner GO. Pre-release is O-13. The `v0.1.0` tag, Release, and publication are §10.
- A new repository dependency needs a `docs/licensing.md` entry in the same PR (CLAUDE.md). If option A is adopted, the `dist` tool is recorded there too.

## 10. Task 6: consolidation and owner decision

- Read-only consolidation in `docs/m10-consolidation.md`: every client, arm, and dimension with its status and links to raw evidence. No new measurement.
- Includes the Task 5 decision record (§9.2) and its review outcome.
- Lists every UNAVAILABLE item, its cause, and the owner action it needs.
- Public reporting:
  - External tools and comparators are not named. An authorized comparator is "Comparator A".
  - Client names may appear in agent-specific sections, as in M9.
  - A superiority claim is made only if the G-PAR rule holds on this corpus. Host, client versions, attempt counts, dates, and limitations appear beside the claim.
  - No performance or accuracy claim beyond the measured corpus, and no claim about other systems.
  - Public release may be considered without superiority claims. The release text then states the limitations: pilot or confirmatory scope, no external-product comparison, and any incomplete competitive evidence.
- The owner decides separately: (a) what the results may say publicly, and (b) the `v0.1.0` tag, Release, and publication. A PASS in this document does not authorize publication.

## 11. Open decisions (owner)

| ID | Question | Default in this document | Blocks |
|---|---|---|---|
| O-1 | The owner-private blueprint is not in this checkout. Does it add M10 requirements? | Not used. This design follows the M9 records and the M10 brief | Nothing, if it adds none |
| O-2 | The OpenCode credential is expired. The owner supplies a new key outside chat, or names a different Task 5 implementer by dated amendment. No automatic reassignment | Task 4 waits. Task 5 STOPs for owner decision | Task 4; Task 5 |
| O-3 | Corpus: S1 only, or add S2 (MMSP) | S1 only | Scale claims |
| O-4 | Cross-platform installation is an M10 requirement, not an M9 violation. Evaluate `dist` first, with the minimal-script fallback (§9.2) | Option A if it passes §9.2 in full; otherwise option B, with reasons recorded | Task 5 technology |
| O-5 | Pre-release publication is not a prerequisite for designing or implementing distribution. Release authorization is separate (O-13) | No tag and no GitHub Release until owner GO | Phase B only |
| O-6 | Intel macOS label. `macos-15-intel` was used at M9; `macos-26-intel` is listed in current GitHub docs. Retirement date for the macOS 15 Intel image was not verified. One secondary search result suggested fall 2027; GitHub's changelog did not confirm it | Record the label used. If no x64 runner exists, the macOS checks are UNAVAILABLE, and the owner decides whether the release proceeds | macOS checks in Phase A and B |
| O-7 | Runner pinning. M9 used `windows-latest` and `ubuntu-latest`. Current GitHub docs map `ubuntu-latest` to the 24.04 image, while the M9 RC amendment records a planned move to Ubuntu 26 from 2026-10-19 | Pin explicit labels (§9.4). The owner confirms the change | Reproducibility; M9 equivalence |
| O-8 | External comparator | None authorized. Arm B only | Any external comparison |
| O-9 | Benchmark size. Pilot: 24 runs (one per client, arm, task). Confirmatory: 120 runs (5 attempts per cell), only after pilot review and owner GO | Pilot on owner GO. Confirmatory not authorized | Confirmatory phase; cost |
| O-10 | Semantic provider in Arm R, given the M9 G-SEM finding | Not registered | D1 scope |
| O-11 | Operator of the Codex and OpenCode sessions | A human reads the frozen prompt verbatim | Tasks 3 and 4 |
| O-12 | Authorization for Phase B (download of a GitHub Release asset) | Not authorized. Phase A only | Phase B |
| O-13 | Authorization for a pre-release tag and public pre-release | Not authorized | Phase B; any RC tag build |
| O-14 | Approval of this revision as the frozen design, and the freeze commit. Distribution technology stays provisional (O-15) | Not approved | Task 2 |
| O-15 | Acceptance of the Task 5 decision record (option A or B) after the bounded feasibility review (§9.2) | Not accepted. Technology provisional | Task 5 adoption |

## 12. Known blockers

- **B-1** The OpenCode credential is expired (O-2). Task 4 is blocked. Task 5 STOPs for an owner decision and has no authorized implementer until then.
- **B-2** Intel macOS runner label availability and retirement (O-6). Blocks the macOS checks.
- **B-3** The M9 release-candidate workflow runs only from `master`. It matters only for Phase B pre-release tags (O-5). Design, feasibility, and Phase A do not need it.
- **B-4** Clean-profile isolation is not verified for any client. Verify during Task 2 preparation.
- **B-5** Some client and provider pairs may not report token usage. Those token fields become UNAVAILABLE.
- **B-6** S2 (MMSP) is third-party content and must not go to a model provider without owner approval. Any external comparator needs a license check and written authorization before it is installed.
- **B-7** Tracked docs, evidence, and installers must hold no absolute local paths. Tests use placeholders. The pre-commit hook and the CI `security` job reject local paths.
- **B-8** The RC archives from M9 run `40263559935` may have expired. Phase A then uses archives built from the base commit, and the evidence states the build source.
- **B-9** Phase B needs network access from the runners to `github.com` and `release-assets.githubusercontent.com`.
- **B-10** Option A's generated release workflow creates the GitHub Release in its host step by default. Its tag trigger is not yet confirmed. Unless disabled or gated, it could publish without owner GO.
- **B-11** Option A's archive layout (root-level files) differs from M9's single top-level folder. Generated files must reach `include` through a step that is not yet designed.
- **B-12** Option A's installer behavior is not confirmed for checksum-before-install, unsupported platforms, or the PowerShell installer. Only the feasibility test can confirm it.
- **B-13** The `dist` license is reported by third-party indexes only (Apache-2.0 OR MIT). Confirm it from the project's own license file before adoption.
- **B-14** A shared target directory let one copy reuse another copy's `riltra` artifact: the base run executed the defect binary. Mitigated by `cargo clean -p riltra` before each session and reviewer run (§3.4). The Rev 4 control C1 reproduced the reuse without the clean step.
- **B-15** Per-copy target directories failed to link on the validation host: `link.exe` returned LNK1104 on the tree-sitter build scripts of the defect copy, while the same step succeeded for the base copy. Cause not diagnosed. The alternative, a shared dependency cache with `cargo clean -p riltra`, passed a cold build and the Rev 4 preflight. The owner accepts LNK1104 as an environmental limitation only on that evidence (§3.4, §14).
- **B-16** The cold Arm R index on the snapshot reported one file error (`errors: 1`). The file is not identified. It does not touch ground truth, but it must be explained before Task 2 Arm R runs.

## 13. Freeze status

| Item | Status |
|---|---|
| Base commit and tree | Fixed by this document (proposed) |
| Task order, arms, criteria, dimensions, status rules, provenance fields | Proposed; frozen in the freeze commit after owner approval |
| Prompts (`freeze/prompts/`) | Prepared and validated, uncommitted. Frozen by the freeze commit after owner approval |
| Frozen edits A3 and A4 (`freeze/edits/`) | Prepared. Both apply cleanly at the pinned commit. The A3 edited copy compiles. A4: defect state exits 101 with the named failure, restored state exits 0 with 4 passed |
| Expected answers A1 to A4 (`freeze/expected/`) and verification script (`freeze/verify/`) | Prepared and validated at the pinned commit. Owner review before the freeze commit |
| SHA-256 list (`freeze/SHA256SUMS.txt`) | Recomputed in Rev 4 for every proposed freeze file. Recomputed again at the freeze commit |
| Procedure (`freeze/PROCEDURE.md`) | Prepared: exact commands for snapshots, both edits, the Arm R binary and index, Arm B, cargo runs, verification, and grading. Pins checked by the verify script |
| A4 isolation preflight | Passed in Rev 4: P1 and P3 defective (exit 101, own build), P2 and P4 restored (exit 0, own build). Control C1 without clean reused the defective binary. Evidence in scratch; redacted copies at the freeze commit |
| Thresholds: 10 %, pilot 24 runs, confirmatory 120 runs (5 per cell), 5 confirmatory harness repetitions for D3, time caps | Proposed; frozen in the freeze commit |
| Distribution technology (option A `dist` or option B minimal scripts) | Provisional until the bounded feasibility assessment (§9.2) is reviewed. Not frozen |
| Installer design and install location (§9.1, §9.3) | Provisional, bound to the technology decision above |
| Installer runner labels | Proposed (§9.4); the labels used are recorded in evidence |

## 14. Revision log

- **Rev 0 (2026-10-26, Task 1):** branch and first design gate.
- **Rev 1 (2026-10-26, owner corrections):**
  1. Minimum distribution: archive-only default replaced by GitHub Releases plus installer scripts; checksum required; user-level installs; smoke on three runners; Phase A and Phase B split.
  2. Task dependencies: benchmark chain strictly sequential; an unavailable agent stops the chain with UNAVAILABLE evidence; Task 5 owner-authorized; no automatic reassignment.
  3. Repetitions: pilot of 24 runs; confirmatory plan of 5 attempts per cell, fixed before pilot outcomes; retries and failures kept.
  4. G-PAR interpretation: five dimensions and the PASS rule kept; per-client reporting; no cross-client ranking or pooling; baseline is not evidence against an external product; release without superiority claims allowed.
  5. Fairness and freeze: both arms keep native tools and Arm R adds Riltra; usage and fallbacks recorded; freeze commit defined; prompts, frozen edits, and expectations specified.
- **Rev 2 (2026-10-26, owner request):**
  1. Revision 1 corrections kept. Pilot of 24 runs, sequential benchmark chain, equivalent native tools, approved freeze before benchmark execution, per-client G-PAR without unsupported superiority claims, and Task 5 only after explicit owner approval. The original 72-run requirement is not reintroduced.
  2. Task 5 is cross-platform distribution (§9.1). `dist` (formerly `cargo-dist`) is the primary candidate, with a bounded feasibility check against the minimal-script fallback (§9.2). Targets are the three Intel and x64 targets only. §2, §9, §11, §12, and §13 changed.
  3. Feasibility criteria cover the eight items in §9.2 and a decision rule that prefers `dist` only with equal or lower maintenance complexity.
  4. The M9 acceptance baseline is mapped to both options (§9.6). `dist init` is not run in this task. Release workflow replacement, tags, and releases are excluded without owner GO.
  5. O-4 and O-5 corrected. Task 5 dependency on OpenCode updated: STOP, no automatic reassignment. The freeze table marks distribution technology as provisional.
- **Rev 3 (2026-10-26, Task 2 preparation):**
  1. Frozen inputs prepared under `docs/pilots/m10/freeze/` (uncommitted) and validated at `7f86891`: prompts, expected answers, two patches, verification script, and SHA-256 list.
  2. A2 lists call sites only. The identifiers occur in many non-call lines (fields, variables, strings), so the excluded-mention list is dropped.
  3. A1 offsets are 0-based and validated. The A3 constant (`MAX_JSON_DEPTH`) and the A4 defect (the `'\n'` arm) are fixed. The A3 edited copy compiles. Both A4 states were run with cargo.
  4. Cargo runs use a dependency cache outside the workspace with `cargo clean -p riltra` before each state (B-14, B-15), not per-copy target directories.
  5. Appendix A now points to the freeze files. The 24-run pilot, sequential chain, equivalent native tools, per-client reporting, G-PAR rules, and the separate Task 5 gate are unchanged.
- **Rev 4 (2026-10-26, freeze preflight):**
  1. Procedure added (`freeze/PROCEDURE.md`): exact commands for clean snapshots, both frozen edits, the Arm R binary from the pinned tree, the Arm R index in task order, Arm B without Riltra, cargo runs, verification, and grading. Pins: commit, tree, `Cargo.lock` SHA-256, toolchain.
  2. Cache strategy fixed for Task 2: a shared dependency cache outside the workspaces, and `cargo clean -p riltra` before each session and reviewer run (§3.4). Preflight, two copies in sequence: P1 defective exit 101; P2 restored exit 0; P3 defective exit 101; C1 control without clean exit 101, reusing P3's binary; P4 restored exit 0. Every run except C1 compiled its own copy.
  3. Arm R index: `index status` reports `stale` with `content-changed` after the A3 edit, with no git metadata. The cold index reported one file error, not yet identified (B-16).
  4. Verification script: pins checked; A4 preflight added (`--cargo`); summary and logs written to `M10_LOG_DIR`. A wrong assumption, that test binaries are byte-identical across runs, was removed. Incremental builds are not byte-identical; the proof is own-build evidence and the outcome.
- Unchanged: corpus S1 and S2, isolation rules except the cache rule in §3.4, status legend, evidence layout, M9 records. Targets are fixed (§3.2).

## Appendix A. Frozen prompt text

The prompts are the files `docs/pilots/m10/freeze/prompts/A1.txt` to `A4.txt`. They are the authoritative text, so this appendix does not repeat them. SHA-256 values are in `docs/pilots/m10/freeze/SHA256SUMS.txt`.
