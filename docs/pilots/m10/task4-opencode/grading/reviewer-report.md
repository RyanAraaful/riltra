# M10 Task 4 — independent reviewer report

Date: 2026-10-27 (UTC). Scope: read-only regrade and STOP-condition audit of the eight-session
OpenCode pilot. The reviewer worked from raw evidence (SQLite session databases, per-run
transcripts, tool-call logs, server logs, setup manifests, cargo logs) and did not trust
`grades.json`. No file was modified by the review.

**Verdict: PARTIALLY UNAVAILABLE** — 3 of 8 runs analyzable; 5 killed by provider quota before the
first agent turn. The runset is not compromised.

## Per-run table

| run | exit | prompt delivery | strict | factual | task | native tools | reviewer verdict |
|---|---|---|---|---|---|---|---|
| A1-R | 0 | PASS | FAIL | PASS | PASS | 10 | agrees with `grades.json` |
| A1-B | 0 | PASS | FAIL | PASS | PASS | 4 | agrees |
| A2-R | 0 | PASS | FAIL | PASS | PASS | 21 | agrees |
| A2-B | 1 | PASS | UNAVAILABLE | UNAVAILABLE | UNAVAILABLE | 0 | agrees |
| A3-R | 1 | PASS | UNAVAILABLE | UNAVAILABLE | UNAVAILABLE | 0 | agrees |
| A3-B | 1 | PASS | UNAVAILABLE | UNAVAILABLE | UNAVAILABLE | 0 | agrees |
| A4-R | 1 | PASS | UNAVAILABLE | UNAVAILABLE | UNAVAILABLE | 0 | agrees |
| A4-B | 1 | PASS | UNAVAILABLE | UNAVAILABLE | UNAVAILABLE | 0 | agrees |

Tool breakdown: A1-R shell 4 / glob 1 / grep 1 / read 4; A1-B grep 2 / shell 2; A2-R grep 2 /
read 19; the five UNAVAILABLE runs 0. Riltra MCP 0 and Riltra CLI 0 everywhere.

## Strict grading (independent, PROCEDURE section 7 literal)

All three completed answers FAIL, correctly:

- `grading/answers/A1-R.txt:1` preamble `All five definitions found. Results:`; `:2` blank;
  `:3` opening fence; `:9` closing fence.
- `grading/answers/A1-B.txt` has the same shape: `:1` preamble
  `All five definitions found and byte offsets verified.`, `:2` blank, fences at `:3` and `:9`.
- `grading/answers/A2-R.txt:1` preamble; fences at `:3`/`:28`; and trailing prose
  "Excluded matches and why" from `:30` onward, after the answer block.

The content lines themselves are correct; the failure is output format only. `PROCEDURE.md`
section 7 (lines 99–124) permits answer lines only.

## Factual grading (verified against truth, not only against the expected file)

- A1: all five byte ranges exact — `canonical_root` 1704-1718, `contains` 2180-2188,
  `resolve_recorded_root` 8095-8116, `escape` 356-362, `grammar_versions` 1289-1305. Each slice
  equals the name and lands on the `fn` line in `snapshot-pin`.
- A2: 24 lines exactly equal the expected set, no duplicates, all genuine call sites; independent
  grep confirms completeness (17 `canonical_root` + 7 `resolve_recorded_root`).
- A3 expected answer is true: `MAX_JSON_DEPTH: usize = 128` at line 267 of the A3-patched copy;
  the only readers are at lines 345 and 361.
- A4 expected answer is true: the `'\n'` match arm is absent and the test module is untouched.

## Prompt delivery — 8/8 PASS

Captured 567/567/463/463/430/430/443/443 chars = frozen file bytes + 2 (one wrapping quote pair
each). After stripping the pair and normalizing LF, all eight equal the frozen prompt; the
dequoted SHA-256 equals the frozen file hash. Evidence: `grading/prompt-delivery.json` (lines
1–82) and `runs/<run>/profile/../profile/data/opencode/opencode.db` in the raw run root.

## Workspace change — PASS (no agent edits)

Full SHA-256 recompute of every file against `logs/setup/workspace-pre-session.tsv`: 0 added,
0 removed, 0 modified across all eight runs. A1/A2 workspaces equal `snapshot-pin`; A3 differs
only in `crates/cli/src/knowledge.rs`; A4 differs only in `crates/cli/src/json.rs` (single removed
`'\n'` arm). All differences come from the frozen setup patches.

## A4 cargo — PASS (defect reproduces, no fix)

`grading/cargo-a4/A4-R.json` and `A4-B.json`: command
`cargo clean -p riltra && cargo test --locked --bin riltra json::tests`; clean step exit 0; test
exit 101; result `FAILED. 3 passed; 1 failed`; a fresh `Compiling riltra` line from the
`cargo-copies` staging. `json.rs:13` still has `'\r'` and the newline arm is still missing.

## STOP-condition audit — NONE

- Tool paths were all relative under `crates/cli/src`; shell commands were limited to `ls -la`,
  Python version checks, and inline relative-path PowerShell. No out-of-workspace reads.
- Operator service pid 10556 on port 28894 was never used. Per-run servers bound ports
  29100–29107 with pids 23000, 20128, 20052, 16152, 15700, 1504, 23224, 13904; each
  `api-info.json` pid matches `session.json` `server_pid`; `benchmark-preflight.txt` shows the
  ports were free before the run.
- Config R vs B: identical after removing `mcp` and `permission.riltra_riltra_code_semantic`
  (R has both, B neither); only the per-run `RILTRA_HOME` value differs.
- Skills: exactly `opencode`/`OpenCode` plus `report`/`Report` on both arms
  (`api-before-skill.json`, `api-after-skill.json`).
- Model: `deepseek/deepseek-v4-pro` in all eight sessions and run metadata.
- No retry: 8 progress lines, 8 summary entries, 1 session, 1 user message and 1 run-log entry
  per run, `resume_attempts=0`.

## Five UNAVAILABLE runs — confirmed infrastructure, not agent failure

All show `finish=error`, `provider.quota`, HTTP 402 `Insufficient Balance`, zero tokens and cost,
empty content, a single session, and no retry. Request IDs match across `session.json`,
`grades.json` (lines 404, 507, 610, 727, 858) and the server stderr.

## Grading discrepancies vs `grading/grades.json`

**None.** Every field — strict/factual/task, prompt delivery, tool counts, changed files,
UNAVAILABLE status — matches the independent recomputation.

Non-semantic notes, not discrepancies:

- Answer files on disk are +8/+34 chars versus the DB text because of LF→CRLF expansion; the
  dequoted, normalized content is identical. The DB is authoritative.
- The A4 test-binary hash differs between the setup preflight and the reviewer rebuild, expected
  after `cargo clean -p riltra`; the test outcome is identical (exit 101).

## Comparable-analysis limit

Only A1-R vs A1-B (paired) and A2-R (single arm) support graded conclusions. A3 and A4 have zero
usable agent answers, so their expected answers were verified true but agent correctness is
untestable until provider balance is restored. A3/A4 arm comparison remains blocked.

Reviewer coverage note: server logs were spot-checked rather than re-read line by line; no
material discrepancy was found.
