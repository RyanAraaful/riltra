# Audit addendum: Task 2 evidence finalization

Date: 2026-10-26. Scope: verification of the eight first-attempt results and their evidence. No benchmark run was repeated. No frozen input, raw transcript, or original outcome was changed. The audit scripts (`verify-transcripts.js`, `verify-redaction.js`, `factual-check.js`) are kept in the scratch root, not committed.

## A. Outcomes preserved

- Outcome files in the repository (`status-initial.txt`, `grade-initial.txt`, `status.txt`, `grade.txt`, `session/answer.txt`) are identical to the raw files: 40 of 40.
- Raw files are not in the repository. `RAW-SHA256SUMS.txt` lists the SHA-256 of the 214 raw files behind the redacted copies. Checked from the scratch root: 214 of 214 OK.

## B. Riltra usage (raw transcripts)

| Run | Riltra MCP calls | Bash commands containing `riltra` | Riltra CLI invocations | Riltra tools in init |
|---|---|---|---|---|
| R-A1 | 0 | 0 | 0 | 11 |
| R-A2 | 0 | 0 | 0 | 11 |
| R-A3 | 0 | 0 | 0 | 11 |
| R-A4 | 0 | 12 | 0 | 11 |
| B-A1 to B-A3 | 0 | 0 | 0 | 0 (no MCP server) |
| B-A4 | 0 | 10 | 0 | 0 (no MCP server) |

The 22 mentions in the A4 runs are cargo commands (`--bin riltra`), build-environment commands, or paths. None starts a riltra executable. Conclusion: Arm R made zero Riltra calls. This pilot provides no evidence about Riltra's effect on accuracy, time, or tokens.

## C. Redaction integrity

- 214 evidence files were compared with their raw counterparts: 71 changed, 513 lines changed, 0 line-count mismatches, 0 changed lines without a path or identifier, and every changed line now holds a placeholder.
- Labels in `REDACTION.tsv`: `scratch`, `scratchpad`, `home`, `user-name`, `project-slug`, `verbatim-prefix`, `placeholder-tail-separator`, `local-pipe`, `local-pipe-field`, `powershell-path`, `posix-local-path`, `windows-local-path`.
- Correction during this audit: the POSIX-path rule first matched a fragment of an opaque base64 signature in one transcript line. The rule now excludes `/` and `+` before the path. All files were redacted again from the raw data, and the results above are from that second run. Outcome files were checked again afterwards (§A). One generic match remains: a fragment inside an opaque base64 thinking signature in `runs/R-A4-a1/transcript.jsonl` line 152. It is not a path and is left unchanged.
- Residual fragments, found after the checks above (which covered changed lines only). Four files kept fragments of local paths in lines the harness had not changed: `runs/R-A4-a1/transcript.jsonl` (scratchpad slug suffix on lines 14, 33, 88, 112; a Windows drive path on line 146 after a JSON `\n`); `runs/B-A4-a1/transcript.jsonl` (slug suffix on lines 14, 41, 76, 124, 139; snapshot folder suffix on line 5); `runs/B-A4-a1/debug.log` (snapshot slug suffix on line 134); `precheck/profile-inventory.txt` (scratch snapshot folder on line 14).
- Cause: the scratchpad input did not match every occurrence of that path, the slug inputs did not list the full names, and the drive rule required a word boundary, so it skipped a drive letter after `\n`.
- Fix: `harness/redact.js` matches a drive letter after a JSON `\n` or `\t` (passes 3 and 4), and the full slug names are inputs. The three harness-produced files were regenerated from raw. The `REDACTION.tsv` count for `R-A4-a1/transcript.jsonl` `windows-local-path` changed from 122 to 124; no other row changed. `precheck/profile-inventory.txt` has no raw copy and no harness run. Its snapshot slug was replaced by `<PROJECT-SLUG>`, the token the harness uses.
- Check after the fix: all 214 raw files were run again through `harness/redact.js` with the same inputs, and all 214 match the committed copies. The residual scan of the whole folder finds no drive-letter path, account name, slug suffix, or run-snapshot slug. The only generic match is the Windows folder name `AppData`, which is kept. The totals above were recomputed and did not change.
- Harness provenance: the `harness/redact.js` committed before this revision was an earlier version (one literal pass and one drive pass; no `PATH_SLUGS` or `USER_NAME` input). It is replaced by the version that produced the evidence. `harness/env.example.sh` lists the inputs as placeholders; their values stay local.

## D. Grader versions and the A4 correction

- Both versions are kept: `harness/grade-v1.sh` (version 1, as first run) and `harness/grade.sh` (version 2). Per run: `grade-initial.txt` and `status-initial.txt` (version 1), `grade.txt` and `status.txt` (version 2).
- Reproduction from the raw data: version 1 reproduces `grade-initial.txt` in 8 of 8 runs; version 2 reproduces `grade.txt` in 8 of 8 runs. For A1 to A3, versions 1 and 2 print identical output (6 of 6).
- Cause of the false FAIL: `sha256sum` writes binary-mode names with a leading `*`. Version 1 read the field as `*./crates/cli/src/json.rs`, so its changed-file check could not match `./crates/cli/src/json.rs`.
- What version 2 changed: the A4 branch only. The prefix is stripped, and changes are split per file into modified, removed, and added.
- Check classification (no new frozen rule):
  - Frozen: `reviewer-exit-0-and-4-passed` and `test-code-unchanged` (gate §3.2 pass rule, C4). `change-confined-to-escape` (gate §3.2 text: "The diff touches only escape").
  - Auxiliary, not in the frozen text: `modified-tracked-files-only-json.rs` (carried from version 1), `no-removed-files`, `no-added-source-files`.
  - Effect: every check passes for both A4 runs. The frozen pass rule alone also gives PASS. No status depends on an auxiliary check.
- Independent reviewer evidence (A4): after `cargo clean -p riltra`, the reviewer ran the frozen command in the harness shell. Both runs: exit 0, one `Compiling riltra` line (fresh build), `test result: ok. 4 passed; 0 failed` (`reviewer/test.log`, `reviewer/test.exit`). The pre-session defect test: exit 101, `3 passed; 1 failed` (`cargo/pre-test.log`).
- Known gap: the test-binary SHA-256 required by PROCEDURE §6 was not recorded (D13). It is not reconstructed.

## E. Helper file (R-A1)

- Location: the run's workspace root, `runs/R-A1-a1/snapshot/offset_calc.js` in the scratch root. Not in the repository, not in a source directory, not in another run.
- Sequence (raw transcript): calls #6 and #7 were Bash attempts with inline Python and Node that errored. #8 (Write) created the file. #9 ran `node offset_calc.js`. #10 (`rm offset_calc.js`) removed it.
- Content: reads the five named definitions and prints their byte offsets. The answer matches the frozen set (5 of 5). The offsets came from the agent's own computation; the Riltra tools were not used.
- Classification: agent-side deviation from the prompt sentence "Do not modify any file". Not a harness or procedure deviation: the harness creates no files in the snapshot. No frozen rule covers it, so no status changed. The snapshot-end manifest has no trace of the file.
- The file name appears in `runs/R-A1-a1/transcript.jsonl`, `session/tools.json`, and `debug.log`, because those files record the calls.

## F. `%SystemDrive%` cache side effect

- Where: `snapshot/%SystemDrive%/ProgramData/Microsoft/Windows/Caches/` in R-A1-a1 and R-A4-a1 only. Three files per run: `cversions.2.db` (16,384 bytes), `{6AF0698E-…}.2.ver0x0000000000000001.db` (308,840 bytes), `{DDF571F2-…}.2.ver0x0000000000000001.db` (662,488 bytes).
- When: R-A1 files 28 s after session start (session 08:25:13 to 08:26:08 UTC). R-A4 files 133 s after session start (session 08:30:57 to 08:36:17 UTC). Both during the agent session; the reviewer ran after it.
- Source not modified: the snapshot compared with the pinned tree (`git archive` of `7f86891`) differs only by the `%SystemDrive%` entry in R-A1. In R-A4 it also differs in `crates/cli/src/json.rs`, the agent's `escape()` fix (D6). B-A1 is identical to the pinned tree. B-A4 differs only in `json.rs`.
- No contamination: each run has its own snapshot. The entries appear in no other run: not in the B runs, not in R-A2, not in R-A3. The pinned archive has no such entry, and neither the repository nor the scratch root contains the folder.
- Cause: not isolated. A standalone `riltra mcp` kept alive for 90 s, with no client, created nothing.
- Effect: no answer, grade, or reviewer result depends on these files.

## G. Verification commands

Run at finalization. The results are in the final report.

- Evidence manifest: `(cd docs/pilots/m10/task2-claude-code && sha256sum -c SHA256SUMS.txt)`.
- Raw list, from the scratch root: `sha256sum -c <repository>/docs/pilots/m10/task2-claude-code/RAW-SHA256SUMS.txt`.
- Repository scanner: `bash security/scan.sh dir docs/pilots/m10/task2-claude-code`.
- Frozen inputs: `(cd docs/pilots/m10/freeze && sha256sum -c SHA256SUMS.txt)` and `bash docs/pilots/m10/freeze/verify/verify-freeze.sh`.
- Repository state: `git status --porcelain`, and `git diff --quiet HEAD -- docs/pilots/m10/freeze docs/m10-design-gate.md docs/pilots/m9-preprelease docs/pilots/cp7a crates`.

## H. Commit scope (committed after owner GO)

- Path: `docs/pilots/m10/task2-claude-code/` only. Excludes the untracked `CLAUDE.md`. No frozen input, source file, or M9 evidence changes.
- Commit with `git commit -s` (DCO). No `Co-Authored-By` line.
