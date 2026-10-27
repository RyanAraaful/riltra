# M10 Task 3 — Independent Codex pilot

## Scope and status

Eight first-attempt slots were launched in Codex CLI on 2026-10-26 UTC: R-A1 through R-A4 and B-A1 through B-A4. Seven reached an agent turn; B-A1 was rejected by Codex configuration parsing before a turn. The final launcher status is STOPPED because R-A4 and B-A4 successfully read operator skill files outside their assigned snapshots. This confirms two isolation violations. The pilot is complete as exploratory evidence, but is protocol-contaminated and must not be represented as a clean benchmark or a successful launcher run. Evidence: `grading.json`, the A4 `filesystem-adjudication.json` files, the A4 `filesystem-audit.json` files, and `skill-content-redaction.json`.

The owner-approved deviation used one dedicated authenticated CODEX_HOME shared across runs. Each run had its own HOME, USERPROFILE, APPDATA, LOCALAPPDATA, workspace snapshot, and Arm R RILTRA_HOME. Windows workspace-write allowed filesystem reads outside the snapshot; isolation was procedural, not OS-enforced. No OS read isolation is claimed. Evidence: `environment.json`, per-run `provenance.json`, and the A4 filesystem adjudications.

Frozen source: 7f86891c0942cc1d7efd6ec29c6c0b6b1606e727. Frozen methodology: 6b800fba6625e89cbd9428f988c6e9a1877b893a. Benchmark model: gpt-6-luna. Codex CLI: 0.164.0. OS: Windows 11 Pro, build 26300.

## Outcomes

Strict-format outcomes are separate from factual and task outcomes. Counts follow the frozen acceptance criteria; no cross-client ranking or statistical claim is made.

| Run | Strict | Task outcome | Factual assessment | Wall seconds | Tokens I/C/O/R | Native searches | Agent Riltra MCP/CLI | Protocol |
|---|---|---|---|---:|---|---:|---:|---|
| R-A1-a1 | PASS | PASS | PASS | UNAVAILABLE | 34259/27136/416/67 | 2 | 0/0 | PASS; completed turn reconciled after postprocessing interruption |
| B-A1-a1 | UNAVAILABLE | UNAVAILABLE | UNAVAILABLE | UNAVAILABLE | UNAVAILABLE | 0 | 0/0 | Codex rejected Arm B config before agent turn |
| R-A2-a1 | PASS | PASS | PASS | 17.515 | 23114/18944/405/45 | 1 | 0/0 | PASS; completed turn reconciled after postprocessing interruption |
| B-A2-a1 | UNAVAILABLE | UNAVAILABLE | No task facts; infrastructure report accurate | 16.444 | 45188/40192/276/59 | 0 successful; 2 failed attempts | 0/0 | Native executor failure; no retry |
| R-A3-a1 | PASS | PASS | PASS | 9.177 | 22442/18944/73/0 | 1 | 0/0 | PASS |
| B-A3-a1 | PASS | PASS | PASS | 9.351 | 22805/18944/180/0 | 1 | 0/0 | Attempted a missing in-snapshot skill path; no content returned |
| R-A4-a1 | FAIL | FAIL | Accurate infrastructure report | 28.023 | 112117/97024/1162/124 | 2 | 0/0 | FAIL; 2 operator skill files read outside snapshot |
| B-A4-a1 | FAIL | FAIL | Accurate infrastructure report | 44.951 | 247721/202880/1217/123 | 1 | 0/0 | FAIL; 1 operator skill file read outside snapshot |

Tokens are input/cached-input/output/reasoning-output as reported by Codex. UNAVAILABLE values were not inferred. Strict and task outcomes are adjudicated in `grading.json`; per-run tokens, elapsed times, timestamps, snapshot/prompt/patch hashes, and configuration are in `runs/<run>/provenance.json` (or the named run reconciliation for startup/postprocessing failures). Tool events and sanitized transcripts are stored alongside provenance.

Counts: strict format 4 PASS, 2 FAIL, 2 UNAVAILABLE; task outcome 4 PASS, 2 FAIL, 2 UNAVAILABLE. Factual answers passed for R-A1, R-A2, R-A3, and B-A3. B-A2 gave an accurate infrastructure report but no task facts. Both A4 runs accurately reported inability to complete; the requested code change and passing reviewer test were absent. These assessments come from `grading.json`, with exact-answer line counts and per-run final messages available in the run folders.

## Riltra use

Agents made zero Riltra MCP calls and zero actual Riltra CLI invocations. No fallback followed a Riltra failure because agents did not choose Riltra. Arm R harness setup ran the CLI 3, 3, 4, and 3 times for A1–A4 respectively to initialize/index the fresh snapshot; Arm B setup used it zero times. A raw launcher metric counted Cargo commands mentioning --bin riltra as CLI calls in A4; `tool-usage-adjudication.json` corrects those false positives without rewriting raw run records. Per-run tool calls and setup logs provide the underlying events.

Arm R launcher configuration contained the approved 11-tool allowlist and excluded riltra_code_semantic. The precheck did not establish a complete runtime catalog, and the agents made no MCP calls, so this evidence does not claim independently observed full tool exposure. Arm B configuration disabled Riltra MCP and omitted its CLI from PATH.

## A4 verification

Both snapshots began from the frozen defect. Harness cargo clean succeeded; the pre-session reviewer test compiled and reported 3 passed, 1 failed, with `escapes_quote_backslash_and_newline` failing. After each agent turn, the reviewer again cleaned and compiled the test; it still reported 3 passed, 1 failed. The final target source file matched the frozen defective baseline byte-for-byte. Agents did not produce a successful source edit. The full directory diff was unavailable because Git for Windows reported a missing overlong path in the reviewer baseline; the target-file comparison and test results remain available. Evidence: each A4 `a4-verification-reconciliation.json`, `grading.json`, and reviewer stdout/stderr under `runs/<run>/harness/`.

## Evidence map

- `grading.json` — independent strict, factual, and task grading.
- `tool-usage-adjudication.json` — corrected agent versus harness Riltra usage counts.
- `environment.json` — frozen pins, model, host platform, and accepted isolation limits.
- `runs/` — sanitized transcripts, tool events, filesystem audits, per-run provenance, and A4 verification logs.
- `skill-content-redaction.json` — records operator skill bodies removed from repository transcript copies. The original raw runset is retained separately under `%LOCALAPPDATA%\Riltra\M10\Task3Raw\first-attempts-20261026T162908300Z`; the archive contains only sanitized copies.
- `redaction-index.json` — raw and sanitized artifact hashes plus redaction records.
- `final-review.json` — final evidence, integrity, privacy, and packaging checks.
- `deviations.md` — deviations, infrastructure interruptions, and grading boundaries.
- `SHA256SUMS.txt` — manifest for this pilot evidence directory, excluding the manifest itself.
- `security-scan.txt` — final repository security scanner result.

The coordinator's preparation-time read of a Claude transcript is recorded in the historical top-level `PRECHECK.md`. Recorded benchmark tool evidence contains no references to Claude/Task 2 evidence or frozen expected-answer locations; filesystem marker counts are retained in each run's access adjudication. Because Windows did not enforce read confinement, absence of a recorded reference is an observation about captured actions, not an OS guarantee.

The shared review package contains sanitized evidence only. The raw launcher runset is a separate local copy with owner-only ACL observed and no broad group ACL; it is not included in the ZIP. Its 18,005 files matched the source copy by SHA-256. The redaction index also references nine original supplemental filesystem audit/adjudication files (six audits and three adjudications) that were not found in that raw runset or the task scratch; only their sanitized review copies remain. Raw agent transcripts and tool-call records remain in the verified raw runset. Evidence: `redaction-index.json`, `final-review.json`, and the sanitized A4 access records. No source, frozen input, methodology, or Task 2 evidence was modified. No rerun, commit, or push was made during final review.
