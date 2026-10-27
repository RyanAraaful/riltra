# M10 Task 2: Claude Code pilot evidence

Status: **run complete; owner GO for commit; committed on `m10-evidence-distribution` (see git log).** Eight first attempts (one per client-arm-task cell) ran on the frozen commit `6b800fba6625e89cbd9428f988c6e9a1877b893a` against the pinned snapshot `7f86891c0942cc1d7efd6ec29c6c0b6b1606e727`. No retries were run. Every result is kept. Run-level statuses only (design gate §6). No G-PAR status, no superiority claim, no Codex or OpenCode work, no confirmatory runs. Pushed to `origin/m10-evidence-distribution`. No PR, merge, tag, or release. Audit of this evidence: [AUDIT.md](AUDIT.md).

## Key findings

- **Arm R made zero Riltra calls.** In all four Arm R runs there were zero Riltra MCP calls and zero `riltra` CLI invocations (raw transcripts; [AUDIT.md](AUDIT.md) §B). Riltra was connected and its 11 tools were listed, but no run used it. This pilot gives no evidence on Riltra's effect on accuracy, time, or tokens. Any Arm R versus Arm B difference is not a Riltra effect.
- **Strict-format failures are separate from factual correctness.** The frozen status (§1, "Frozen") is the strict diff against the expectation. The factual column is a descriptive comparison of the answer's locations with the same expected set. Three of the strict failures (B-A1, R-A2, and the two A3 runs) contain the correct locations.
- **A4 was graded twice.** Grader v1 gave a false FAIL in both A4 runs. Grader v2 gives PASS. Both are kept (D6, [AUDIT.md](AUDIT.md) §D).
- **Two Arm R runs left Windows cache files** under a literal `%SystemDrive%` folder inside their own snapshot. The source and the other runs are unaffected (D7, [AUDIT.md](AUDIT.md) §F).

## 1. Results (pilot, run level)

"Frozen" is the strict diff against the frozen expectation (PROCEDURE §7). "Factual" is descriptive: it compares the answer's location lines with the same expected set. Preamble lines and separator style are not counted in "Factual". "Factual" is not a grade and adds no rule.

| Run | Arm | Task | Frozen (final) | Frozen (initial) | Factual | Strict-format note |
|---|---|---|---|---|---|---|
| R-A1-a1 | R | A1 | PASS | PASS | 5/5 locations match | none |
| B-A1-a1 | B | A1 | FAIL | FAIL | 5/5 match | one preamble line `All verified. Here are the results:` and a blank line |
| R-A2-a1 | R | A2 | FAIL | FAIL | 24/24 match | one preamble line `All confirmed as genuine calls. Here are the results:` and a blank line |
| B-A2-a1 | B | A2 | PASS | PASS | 24/24 match | none |
| R-A3-a1 | R | A3 | FAIL | FAIL | value `128` and 2/2 readers match after separator normalisation | readers use backslashes (`crates\cli\src\knowledge.rs`); the expected form uses `/` |
| B-A3-a1 | B | A3 | FAIL | FAIL | same as R-A3 | same as R-A3 |
| R-A4-a1 | R | A4 | PASS | FAIL (grader v1) | reviewer test passes; change confined to `escape()` | not applicable: A4 is graded by the reviewer test, not by an answer format. Three non-source files added in the workspace (D7) |
| B-A4-a1 | B | A4 | PASS | FAIL (grader v1) | reviewer test passes; change confined to `escape()` | not applicable (as above) |

One attempt per cell; no retries. "Initial" is the grade before the grader correction (D6). Both grades are in each run folder (`grade-initial.txt`, `grade.txt`).

## 2. Measurements (provider-reported where marked)

From `metrics.tsv`. Times are wall clock per session unless noted. Tokens are client-reported `modelUsage` values, summed over models (D10).

| Run | Wall s | Client API ms | Index s (Arm R, cold) | Reviewer test s | Tool calls | Riltra calls | Native search | Fallbacks | In tok | Out tok | Cache read | Cache write | Cost USD |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| R-A1 | 55 | 52658 | 38.9 | - | 10 | 0 | 1 | 0 | 1051 | 3429 | 164627 | 6945 | 0.0960 |
| B-A1 | 42 | 38629 | - | - | 8 | 0 | 3 | 0 | 1047 | 3087 | 91998 | 6299 | 0.0754 |
| R-A2 | 31 | 28096 | 38.0 | - | 9 | 0 | 1 | 0 | 1008 | 2625 | 77389 | 6777 | 0.0698 |
| B-A2 | 13 | 11696 | - | - | 2 | 0 | 2 | 0 | 1004 | 995 | 25958 | 2357 | 0.0255 |
| R-A3 | 9 | 6921 | 38.2 | - | 1 | 0 | 1 | 0 | 997 | 257 | 34402 | 1243 | 0.0153 |
| B-A3 | 8 | 6048 | - | - | 1 | 0 | 1 | 0 | 997 | 195 | 25952 | 1182 | 0.0128 |
| R-A4 | 320 | 235164 | 38.0 | 17 | 31 | 0 | 9 | 0 | 1053 | 16747 | 1482399 | 59378 | 0.7025 |
| B-A4 | 217 | 182290 | - | 18 | 26 | 0 | 6 | 0 | 1043 | 11044 | 882976 | 42542 | 0.4582 |
| **Total** | | | | | 88 | 0 | | 0 | 8200 | 38379 | 2785701 | 126723 | 1.4554 |

- Precheck probes are not in the totals: B probe $0.0109, R probe $0.0711 (`precheck/`).
- No run hit its time cap. The A4 pre-session defect test (exit 101) was not timed. Its log is in `runs/*-A4-a1/cargo/pre-test.log`.
- Arm R index: `riltra init` and `index` exit 0 in all four Arm R runs. `index` reports `errors: 1` (B-16 fixture, see §3).

## 3. Preconditions (before the first session)

| Check | Result | Evidence |
|---|---|---|
| Branch and frozen commit | `m10-evidence-distribution` at `6b800fb`; `git status` showed only `?? CLAUDE.md` at run start | run log |
| Freeze manifest (12 files) | OK; `SHA256SUMS.txt` self-hash `1b2cc40bf33102aaa68ec8e8bbe01898b361df4d7f126f67a0e6119c0ce801f4` | `precheck/freeze-manifest-check.txt` |
| Frozen verification script | exit 0 (pins, A1 to A3 derivations, both A4 patches) | `precheck/frozen-verify.txt` |
| Gate and PROCEDURE unchanged | `git diff --quiet 6b800fb` on both: no change | run log |
| Cargo cache strategy | cargo and rustc 1.99.1, as pinned; `verify-freeze.sh --cargo` exit 0; P1 101, P2 0, P3 101, C1 101 (reuse, control), P4 0 | `precheck/cargo-preflight-summary.tsv` |
| Arm R binary | built from the pinned tree; `riltra 0.1.0`; SHA-256 `e16903191d3b905978213a2fe95c5096d9af1ed3807d7d191c52a99537f5c996` (in every R `provenance.txt`) | `precheck/` and runs |
| Authentication, no MCP (arm B) | probe answered `OK`, `is_error: false`, `apiKeySource: none` | `precheck/probe-B-summary.json` |
| Authentication, Arm R config | probe answered `OK`; `riltra` MCP `connected` | `precheck/probe-R-summary.json` |
| Clean profile | 21 entries, names only; no `settings.json`, `CLAUDE.md`, `skills/`, `plugins/`, `agents/`, `commands/`, `hooks/`, or memory. Init: `plugins: []`, `skills: []`, `slash_commands: []`. Debug log: no memory or instruction load | `precheck/profile-inventory.txt`, `precheck/probe-*-debug.log` |
| Configuration hash | `620b014947dc…` before and after every probe and every run (identical) | `runs/*/config-before.sha256`, `config-after.sha256` |
| Arm B has no Riltra | `command -v riltra` empty; init has no `mcp__riltra__*` tool and no MCP server | `precheck/probe-B-init.json` |
| Arm R has Riltra available | `riltra` on PATH; init lists `riltra` connected and 11 riltra tools (not used, see Key findings) | `precheck/probe-R-init.json` |
| Semantic provider | Not used: no LSP provider configured (`mcp` arguments are only `mcp`). The server still advertises 12 tools including `riltra_code_semantic` (`precheck/mcp-tools-advertised.txt`). The client denies it (`--disallowedTools`), so it is absent from init and was never called | `precheck/mcp-tools-advertised.txt` |
| B-16 | Cold index: `errors: 1`, the malformed fixture `crates/cli/tests/fixtures/m2/bad/bad.py`, classified benign in the earlier report. Not a blocker | `runs/R-A1-a1/index/index-initial.json` |

## 4. Protocol as executed

- Harness: `harness/`. `lib.sh` holds the isolated session function, `session.sh` runs one cell, `grade.sh` grades (version 2), `grade-v1.sh` is version 1 as first run, `summarize.js` splits the stream-json transcript, `probe.sh` runs precheck, `redact.js` redacts evidence. `env.example.sh` is a placeholder template.
- Session isolation: `CLAUDE_CONFIG_DIR` set to the owner's benchmark profile (D1). `HOME`, `USERPROFILE`, `APPDATA`, and `LOCALAPPDATA` are fresh per run. The environment is cleared (`env -i`). Working directory is the run's snapshot. Flags: `--restricted`, `--tools` and `--allowedTools` set to `Read,Glob,Grep,Edit,Write,Bash` (plus 11 riltra tools in Arm R), `--permission-mode dontAsk --permission-prompts none`, `--strict-mcp-config` (riltra only in Arm R), `--disable-slash-commands`, `--no-session-persistence`, `--disallowedTools mcp__riltra__riltra_code_semantic` (Arm R). The prompt is the frozen `prompts/A#.txt`, passed on stdin unchanged.
- Time caps: 900 s for A1 to A3, 1800 s for A4. None reached.
- Arm R index, per PROCEDURE §4: A1 and A2 on the unedited copy. A3 indexed before the frozen edit; after the edit `index status` = `"freshness":"stale","reasons":["content-changed"]` (`runs/R-A3-a1/index/status-after-edit.json`). A4 indexed after the defect; status fresh.
- Cargo: `cargo clean -p riltra` before every session (PROCEDURE §6, gate §3.4). A4 also runs the frozen test in defect state before the session (exit 101, `3 passed; 1 failed`). After each A4 session, the reviewer runs `cargo clean -p riltra` then the frozen test on the agent's final copy.
- Grading: `grade.sh` implements PROCEDURE §7 (exact diff, sorted for A1/A2, value line plus sorted readers for A3). A4 checks: reviewer exit and result line (frozen pass rule), test module unchanged (frozen), change confined to `escape()` (frozen description), and auxiliary file-level checks (AUDIT §D).
- Transcripts: `summarize.js` counts tool calls by category. A `riltra` call is a Bash command whose first executable is `riltra`. A native search is `Grep`, `Glob`, or a Bash `grep`/`rg`/`find`. A fallback is a native search after a riltra call that errored, returned empty, or reported `"freshness":"stale"`. Fallbacks were 0 in every run.

## 5. Deviations, environment events, and caveats

- **D1. Shared owner profile.** The eight sessions used the same profile (`claude-profile`), as instructed. The gate (§3.4) asks for a new profile per session. Mitigation: fresh HOME, APPDATA, workspace, and `RILTRA_HOME` per run, and a configuration hash that did not change across runs. The profile holds the login; session persistence is off.
- **D2. Riltra was not used in any Arm R run.** Zero riltra MCP calls and zero `riltra` CLI invocations in four runs, so zero fallbacks. The 22 Bash commands in the two A4 runs that contain the string `riltra` are cargo targets (`--bin riltra`), build-environment commands, or paths; none starts a riltra executable ([AUDIT.md](AUDIT.md) §B). Arm R therefore measures native tools with Riltra available. It does not measure Riltra-assisted answers. No coaching was given.
- **D3. Preamble lines fail the frozen strict diff (B-A1, R-A2).** The location lines are correct (Factual column). The frozen rule (PROCEDURE §7, "An extra line in A2 fails") makes these FAIL. They are kept as FAIL. Owner decision: keep the strict rule or amend it (dated amendment, before any confirmatory phase).
- **D4. Backslash path separators fail the frozen strict diff (R-A3, B-A3).** Same file and line as expected; separator style differs. Kept as FAIL. Same owner decision as D3.
- **D5. Agent-side A4 build environment (both arms).** In the agent's shell, `link.exe` resolved to a coreutils `link` (`link: missing operand`). The harness shell, which ran the pre-session test and the reviewer, linked without error. Both agents worked around it by pointing `PATH`, `LIB`, and `INCLUDE` at the Visual Studio 2022 toolchain (R-A4 answer text; B-A4 wrote `run_test.bat` and deleted it). This was agent action, not operator intervention, so no run is marked ASSISTED. The root cause in the client shell was not isolated.
- **D6. Grader v1 false negative; v2 correction.** `sha256sum` writes binary-mode names with a leading `*`. Grader v1 did not strip it, so its changed-file check could not match the path and both A4 runs failed. Grader v2 strips the prefix and checks changes per file. Initial grades are kept as `runs/*/grade-initial.txt` and `status-initial.txt`; v1 is kept as `harness/grade-v1.sh`. Both versions reproduce their recorded outputs from the raw data (AUDIT §D). The A4 status changed from FAIL to PASS. No A1 to A3 status changed. Grader v2 also adds auxiliary file-level checks that are not in the frozen text. Every check passes for both A4 runs, and the frozen pass rule alone also gives PASS, so no status depends on them.
- **D7. Cache files in the workspace of two Arm R runs.** R-A1 and R-A4 snapshots each contain three Windows cache files (16,384, 308,840, and 662,488 bytes) under a literal `%SystemDrive%/ProgramData/Microsoft/Windows/Caches/` folder. They were written 28 s into the R-A1 session and 133 s into the R-A4 session, both during the agent session. No B run and no other Arm R run has them. Verified: the source is unchanged apart from the expected A4 fix, the other runs are unaffected, and neither the repository nor the scratch root has the folder ([AUDIT.md](AUDIT.md) §F). The cause was not isolated. A standalone `riltra mcp` kept alive for 90 s, with no client, created nothing. The files are not source and do not affect any answer or the A4 reviewer result.
- **D8. Helper file in the R-A1 workspace.** The A1 prompt says "Do not modify any file". R-A1 wrote `offset_calc.js` into its workspace root, ran it once, and deleted it (transcript calls #8 to #10). The file is not in the repository, not in any source directory, and not in another run ([AUDIT.md](AUDIT.md) §E). Classification: agent-side deviation from the prompt sentence. It is not a harness or procedure deviation, because the harness creates no files in the snapshot. No frozen rule covers it, so no status changed.
- **D9. Semantic tool advertised.** The server advertises `riltra_code_semantic` (12 tools). The client denies it, and no run called it. Owner decision on whether advertised-but-denied counts as compliant with O-10.
- **D10. Token accounting.** Tokens are the client-reported `modelUsage` values, summed over models. Each session also includes a small auxiliary `claude-haiku-4-5` call (about 1 k input and 20 output tokens). These are included in the totals. No local re-estimation. Context7 (`/anthropics/claude-code`, `TurnUsage` and `ContextApiUsage`) confirms the four token fields per response and binds usage to a model. The `modelUsage` aggregate itself is not documented there, so summing per model is our reading of the client output.
- **D11. Timing.** Index time is one cold full index (38 to 39 s). The A4 pre-session test is not timed. Reviewer test time is in `reviewer/timing.txt`.
- **D12. Model.** All eight sessions report `claude-sonnet-5` in their init event (raw transcripts).
- **D13. Gap: test-binary hash not recorded.** PROCEDURE §6 asks for the test-binary SHA-256 of each A4 run. The harness did not record it. The reviewer log has the binary path and exit code. Later builds overwrite the artifact, so the hash is not reconstructed.
- **Caveats.** One attempt per cell. No statistical claim. Windows host only.

## 6. Owner review items

1. D3 and D4: keep the strict frozen diff (FAIL) or amend the grading rule before the confirmatory phase. The Factual column shows the content is correct.
2. D6: accept grader v2 (including its auxiliary checks as informational), or re-grade with v1 only.
3. D7: investigate the `%SystemDrive%` cache writes in Arm R sessions before any further Arm R run.
4. D5: fix the agent-shell toolchain path, or accept the workaround as recorded.
5. D2: decide whether Arm R must exercise Riltra. The pilot does not, and the gate allows native search only as a fallback.
6. D1: accept the shared profile, or provide a fresh headless credential per session.
7. D9: accept advertised-but-denied semantic tool for O-10, or record it as a deviation.
8. D13: accept the missing test-binary hash, or rerun in a later task (not in this pilot).

## 7. Evidence inventory and verification

- `metrics.tsv`: the table in §2, generated from `runs/*/session/summary.json`, `tools.json`, `timing.txt`, and `reviewer/timing.txt`.
- `runs/<ARM>-<TASK>-a1/`: `provenance.txt`, `status.txt`, `status-initial.txt`, `grade.txt`, `grade-initial.txt`, `config-before.sha256`, `config-after.sha256`, `snapshot-start.sha256`, `snapshot-end.sha256`, `snapshot-changes.diff`, `session/` (timing, summary, tools, init, result, answer), `transcript.jsonl` (redacted stream-json), `debug.log` (redacted), `stderr.log`, `mcp.json` (Arm R), `index/` (Arm R), `cargo/`, `reviewer/` (A4).
- `precheck/`: probe summaries and init events, probe debug logs, cargo preflight, frozen verification output, manifest check, profile inventory, advertised MCP tool names.
- `harness/`: scripts as run (§4), `grade-v1.sh` (grader version 1), and `env.example.sh` as a placeholder template.
- `REDACTION.tsv`: per file, the label and count of each replacement. These files are redacted copies. The raw files stay outside the repository in the scratch root. Labels: `scratch`, `scratchpad`, `home`, `user-name`, `project-slug`, `verbatim-prefix`, `placeholder-tail-separator`, `local-pipe`, `local-pipe-field`, `powershell-path`, `posix-local-path`, `windows-local-path`. Post-redaction token scan of the run files (`accessToken`, `refreshToken`, `sk-ant-`, `Bearer`, JWT-like): 0 hits.
- `RAW-SHA256SUMS.txt`: SHA-256 of the 214 raw files behind these redacted copies. Paths are relative to the scratch root. Check from the scratch root with `sha256sum -c <repository>/docs/pilots/m10/task2-claude-code/RAW-SHA256SUMS.txt`.
- `AUDIT.md`: verification record for the Riltra usage, redaction, graders, helper file, and cache side effect, with the commands used.
- `SHA256SUMS.txt`: hashes of every file in this folder except itself. Verify with `(cd docs/pilots/m10/task2-claude-code && sha256sum -c SHA256SUMS.txt)`.
- Frozen inputs, unchanged: `(cd docs/pilots/m10/freeze && sha256sum -c SHA256SUMS.txt)`, and `bash docs/pilots/m10/freeze/verify/verify-freeze.sh` from the repository root.
