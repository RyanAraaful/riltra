# M10 Task 4 — OpenCode eight-session exploratory pilot

Date: 2026-10-27 (UTC). Repository HEAD `38e92a46a339441d9f941b8dd292ea25f4c92a10`.
Frozen inputs: `docs/pilots/m10/freeze` (freeze commit `6b800fba6625e89cbd9428f988c6e9a1877b893a`,
pin commit `7f86891c0942cc1d7efd6ec29c6c0b6b1606e727`, tree `b2bd50be9ec829fd122635696a2cb95c29fe2bcc`).

This is one first-attempt run per slot, R-A1…R-A4 and B-A1…B-A4. No slot was retried or
repeated. The runset completed all eight slots, but provider quota was exhausted partway
through: three sessions produced answers and five returned `Insufficient Balance` before any
agent turn. The five are reported as UNAVAILABLE, not FAIL.

## Run directories

Directory names follow the freeze convention `<arm>-<task>-a1`. Run identity
(`A1-R`, `A2-B`, ...) is unchanged in metadata, grades, and provenance.

| run identity | directory |
|---|---|
| A1-R | `runs/R-A1-a1/` |
| A1-B | `runs/B-A1-a1/` |
| A2-R | `runs/R-A2-a1/` |
| A2-B | `runs/B-A2-a1/` |
| A3-R | `runs/R-A3-a1/` |
| A3-B | `runs/B-A3-a1/` |
| A4-R | `runs/R-A4-a1/` |
| A4-B | `runs/B-A4-a1/` |

## Result

| run | status | wall s | input | output | reasoning | cache read | cost USD | native tool calls | Riltra MCP | Riltra CLI | changed files | strict | factual | task |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| A1-R | completed | 77.112 | 9,785 | 1,153 | 4,212 | 70,272 | 0.018626784 | 10 | 0 | 0 | 0 | FAIL | PASS | PASS |
| A1-B | completed | 36.254 | 4,657 | 1,010 | 1,108 | 22,272 | 0.007757244 | 4 | 0 | 0 | 0 | FAIL | PASS | PASS |
| A2-R | completed | 47.832 | 11,973 | 2,003 | 2,836 | 52,224 | 0.018632328 | 21 | 0 | 0 | 0 | FAIL | PASS | PASS |
| A2-B | UNAVAILABLE | 1.418 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | UNAVAILABLE | UNAVAILABLE | UNAVAILABLE |
| A3-R | UNAVAILABLE | 9.658 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | UNAVAILABLE | UNAVAILABLE | UNAVAILABLE |
| A3-B | UNAVAILABLE | 1.606 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | UNAVAILABLE | UNAVAILABLE | UNAVAILABLE |
| A4-R | UNAVAILABLE | 2.859 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | UNAVAILABLE | UNAVAILABLE | UNAVAILABLE |
| A4-B | UNAVAILABLE | 1.267 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | UNAVAILABLE | UNAVAILABLE | UNAVAILABLE |

Completed sessions total 161.198 s, 26,415 input tokens, 4,166 output tokens, 8,156 reasoning
tokens, 144,768 cache-read tokens, and 0.045016356 USD provider-reported cost. The five UNAVAILABLE
runs used zero tokens and cost nothing.

Grading definitions:

- **strict** — literal PROCEDURE section 7 diff. For A1/A2, all answer lines sorted equal the
  expected lines; for A3, the first line equals and the tail sorted equals. A preamble, heading,
  blank separator, or fence line is an extra line and fails.
- **factual** — the requested content lines match the expected lines exactly and are true against
  the pinned source (A1 offsets, A2 call sites, A3 post-edit value and readers, A4 claims).
- **task** — correct outcome with no forbidden edits. A1–A3 require an unchanged workspace;
  A4 requires an A4 test fix confined to `escape()` with the test module unchanged.
- **UNAVAILABLE** — infrastructure failure before any agent turn. This is neither a model failure
  nor a task failure.

Answer style, verbatim:

- A1-R: preamble `All five definitions found. Results:`, then a blank line and a fenced block
  containing the five correct lines.
- A1-B: preamble `All five definitions found and byte offsets verified.`, then a blank line and
  the same fenced block.
- A2-R: preamble `I've verified each match against the source. …`, a blank line and a fenced block
  with all 24 correct lines, then an "Excluded matches and why" list after the block.

Under the Task 2 precedent, preamble, blank separator, fence, and trailing prose are extra lines,
so strict FAILs while the requested content is exactly correct. No completed session used Riltra
MCP or the Riltra CLI.

## Method

- Same direct executable for both arms:
  `node_modules/@opencode/cli/bin/opencode.exe` (OpenCode 2.1.4), SHA-256
  `b12b0613e9b4a491e5b12d90a5e9bbae6d261fe9688c82bf73125a036963d399`.
- Both arms used a pre-warmed `opencode serve` on a private `127.0.0.1` port and then
  `opencode run --server`; the CLI was invoked directly with `ProcessStartInfo.ArgumentList`
  (no `cmd.exe` shim).
- Model `deepseek/deepseek-v4-pro` on every session. Provider-reported tokens and cost are in
  the table and in `provenance.json`.
- Eight independent profiles, workspaces, ports (29100–29107), random passwords, and sessions.
  Each workspace is a copy of the 1,785-file pinned snapshot; A3/A4 then receive the frozen patch.
  Pre-session manifests: A1/A2 both arms equal the snapshot manifest
  `66e955fc3bfd78140dcc849f0b73b48a0da2d9240625859d82e53fbd1588ef48`; A3-R and A3-B share
  `12c523d4ff7ac4e472a71d60e61d49f08b3c57bc3ac447a2aa137939bc2cad08`; A4-R and A4-B share
  `3d2ec36983ae72da992167907144fe48917e2644eba67f1b0b26451b91f948a2`.
- Arm R: the Riltra MCP server (11 model-visible tools) plus the Riltra CLI on `PATH`.
  Arm B: no Riltra MCP, no CLI, no `RILTRA_HOME`. No agent was told to use Riltra.
- Frozen prompts, patches, and expected answers were used unchanged; their SHA-256 values are
  in `provenance.json` and `freeze/SHA256SUMS.txt`.
- Setup order followed PROCEDURE section 4: A1/A2 index the unedited snapshot; A3 indexes first,
  then applies the A3 edit (agent starts stale); A4 applies the defect first, then indexes.
  A4 also ran the section 6 cargo pre-step before the session.
- Workspace change accounting uses `logs/setup/workspace-pre-session.tsv`, recorded after setup,
  versus `workspace-after.tsv`. See the deviation about the launcher's earlier baseline.

## Protocol status and accepted conditions

The owner accepted three conditions for this exploratory pilot, recorded here explicitly:

1. **Client-side semantic denial.** The Riltra MCP server advertises 12 tools at the JSON-RPC
   level, but OpenCode exposes 11 Riltra tools to the model and rejects a raw
   `riltra_riltra_code_semantic` call. Precheck evidence: `evidence/precheck-provenance.json`
   (`mcp_protocol_tools_advertised=12`; catalog 23 = 12 native + 11 Riltra; semantic present
   false) and the captured request body. The benchmark config uses the same MCP block and the
   same `"riltra_riltra_code_semantic": "deny"` permission.
2. **Two built-in OpenCode skills** (`OpenCode`, `Report`) are present identically on both arms
   and in all eight runs (`protocol.json`, `skills_by_run`). No operator skill was loaded.
3. **One pair of wrapping double quotes.** The positional-argument path delivers the prompt
   wrapped in one pair of double quotes on all eight runs. After removing that pair and
   normalizing CRLF/CR to LF, the captured text equals the frozen prompt on all eight runs.
   Literal byte identity is not claimed.

Other protocol checks:

- Config equality: for each task, the R and B configs are equal after removing the Riltra MCP
  block and the semantic deny permission (`protocol.json`, `config_equality`).
- Isolation: per-run `HOME`, `USERPROFILE`, XDG directories, `TMP`/`TEMP`, Git config, and
  OpenCode config; operator config, plugins, skills, and MCP are not loaded. Both arms disable
  snapshots, LSP, formatters, autoupdate, and Claude Code skill loading.
- Operator service on `127.0.0.1:28894` (pid 10556) was never used. The launcher checked every
  benchmark port and verified the listener pid was the probe server.
- Workspace: 1,785 files before and after every session. The three completed sessions changed
  no file.

## STOP-condition audit

No STOP condition was triggered.

- **Filesystem.** Every tool call and shell command was inspected. The three completed sessions
  stayed inside their own workspace (`ls`, `grep`, `read`, PowerShell byte-offset scripts).
  There were no reads of `<REPO>`, operator config/skills/data, expected answers, other
  runs' transcripts, the evidence directory, or the pinned snapshot. The five UNAVAILABLE runs
  made no tool calls.
- **Operator service.** No benchmark port or session connected to pid 10556 on 28894.
- **Model/provider.** All sessions are `deepseek/deepseek-v4-pro`. The quota error is a provider
  account limit (`Insufficient Balance`, HTTP 402), not a model change.
- **Config.** R and B are equivalent apart from the Riltra block and semantic deny.
- **Prompt.** All eight deliveries match the frozen prompt after the accepted dequote + LF
  normalization. No prompt content changed.

## A4 cargo

The frozen A4 defect was applied before each A4 session, and the section 6 pre-step ran with the
shared `CARGO_TARGET_DIR` and empty `RUSTFLAGS`:

- both A4 runs: exit 101, `test result: FAILED. 3 passed; 1 failed; 0 ignored; 0 measured;
  425 filtered out`, failing `json::tests::escapes_quote_backslash_and_newline`;
- compile line and test-binary SHA-256 are in each run's `logs/setup/cargo-prestep.json`.

Because the provider returned quota errors before any A4 turn, no agent fix exists. The reviewer
step re-ran the same cargo command on each final A4 workspace copy: exit 101 with the same
`3 passed; 1 failed` result. The defect therefore persists in both final workspaces. The test
binary SHA-256 differs between the pre-step and the reviewer copy because the debug build embeds
the absolute source path; the stable signals are the compile line, exit code, and result line.

## Independent review

The independent reviewer regraded the runset from raw evidence (session databases, transcripts,
tool-call logs, setup manifests, cargo logs) and audited the STOP conditions read-only. Verdict:
**PARTIALLY UNAVAILABLE, not compromised**; no grading discrepancy was found. The reviewer
confirmed 8/8 prompt delivery, zero workspace changes by agents, the A4 defect still reproducing,
and no STOP condition. Full report: `grading/reviewer-report.md`.

## Deviations

Accepted methodology conditions (owner, before execution):

1. Client-side semantic tool denial (12 advertised, 11 model-visible, raw semantic call rejected).
2. Two built-in OpenCode skills (`opencode`, `report`) identical on both arms.
3. One pair of wrapping quotes; equality after dequote + LF normalization, not literal identity.

Operational deviations:

4. **Provider quota exhausted.** After A2-R completed, the provider returned
   `AI.Error.QuotaExceeded: Insufficient Balance` (HTTP 402, request ids recorded) for A2-B,
   A3-R, A3-B, A4-R, and A4-B. Those sessions received no assistant content and used zero
   tokens. Per the freeze, they are UNAVAILABLE. No first attempt was retried to improve a score.
5. **Workspace diff baseline.** The launcher wrote its diff baseline before applying the frozen
   A3/A4 patch, so its `workspace-diff.txt` counts the patch itself for A3/A4. Grading recomputes
   from `workspace-pre-session.tsv`, recorded after setup. No agent changed any file; the
   recomputed change count is zero for every run.
6. **A1-R, A1-B, A2-R answer style.** All three add a preamble; both A1 answers and A2-R also
   fence the answer block; A2-R adds trailing exclusion prose. That makes the literal strict diff
   fail. This is a result, not a deviation from the frozen grading; factual and task pass.

## Evidence inventory

- `grading/grades.json`, `grading/grades.md` — deterministic grades and extraction details.
- `grading/answers/` — final assistant text per run.
- `grading/tool-calls/` — every tool call with input, status, output, and exit metadata.
- `grading/audit/` — filesystem audit and shell-command listing per run.
- `grading/cargo-a4/` — reviewer cargo logs and records for A4-R and A4-B.
- `runs/<arm>-<task>-a1>/` — sanitized session summary, raw CLI transcript, run metadata, setup and Riltra
  logs, workspace manifests, server metadata, skills, and config.
- `evidence/precheck-provenance.json`, `evidence/precheck-REPORT.md`,
  `evidence/precheck-mcp-tools.txt` — precheck protocol evidence.
- `protocol.json` — model, startup, ports, config equality, skills, and MCP status.
- `provenance.json` — hashes of the client and Riltra binaries, prompts, expected answers,
  patches, per-run DBs, and grading.
- `freeze/SHA256SUMS.txt` — frozen input hashes.
- `harness/` — the executed launcher, client/server harnesses, and grader.
- `MANIFEST-SHA256.txt` — SHA-256 of every file in this bundle. It covers the sanitized
  copies, not the raw run files; raw artifact hashes are in `provenance.json`.

Redaction: local user, repository, and harness paths are replaced with `<USERPROFILE>`,
`<REPO>`, and `<ROOT>`. Server password digests are removed, and credential-shaped strings are
redacted. Raw OpenCode SQLite databases (about 14 MB each) are not included; their SHA-256 values
are in `provenance.json`, and the extracted prompt and transcript are included.

## Limits

- One attempt per slot. Five slots are UNAVAILABLE, so this runset cannot estimate a Riltra effect
  and the completed sample is three sessions (A1-R, A1-B, A2-R).
- No capability claim. This pilot says nothing about whether Riltra improves or degrades agent
  performance. With two completed paired sessions that agree on content (A1-R, A1-B) and one
  unpaired session (A2-R), and with A3/A4 having no agent answer, the data cannot support either
  a positive or a negative claim. The only defect signal in the runset is the frozen A4 bug, which
  no agent attempted.
- **Owner closure (2026-10-27).** The owner decided not to retry the five UNAVAILABLE slots. The
  status remains PARTIALLY UNAVAILABLE, the eight first attempts are preserved unchanged, and no
  benchmark rerun, provider change, or source/freeze change was made to improve a score.
- No commit, push, merge, tag, release, Task 5 work, source change, or freeze change was made.
- A future attempt needs separate owner approval and a fresh freeze; it is not authorized by this
  pilot.

## Publication package

This directory is the publication copy of the review bundle. It was produced from the
sanitized bundle without reading or changing the raw run roots or the original ZIP.

- Line endings: every text file is LF. The package-local `.gitattributes` (`* -text -eol`)
  disables Git line-ending conversion so committed bytes keep matching `SHA256SUMS.txt`.
- Redaction: `REDACTION.tsv` lists, per file, each redaction class and how many placeholder
  spans it stands for. Classes follow the M10 vocabulary: `scratch` (`<ROOT>`), `home`
  (`<USERPROFILE>`, `<OPERATOR_HOME>`), `repo` (`<REPO>`), and `secret` (`<REDACTED>`,
  `<REDACTED-API-KEY>`). Originals are deliberately not stored.
- Manifest: `SHA256SUMS.txt` covers every file in this directory except itself.
- Raw SQLite session databases are not included. `provenance.json` records their SHA-256
  and `-wal` SHA-256 values; the sanitized prompt, transcript, and answer copies here are
  the audit text.
- The publication ZIP beside this directory is generated from these files and is not
  byte-identical to the internal review ZIP. Its SHA-256 is reported with the hand-off.
- Security scan: `security/scan.sh dir` (gitleaks 8.30.1) over this directory, exit 0,
  zero findings. The scan log and exit code are kept with the harness export, not inside
  this directory, so the manifest stays stable.
