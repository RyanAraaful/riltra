# Deviations and limitations

## Isolation and protocol

- Owner approved one dedicated authenticated benchmark CODEX_HOME shared across eight runs. This is a bounded deviation from frozen design §3.4. Credentials were not opened, copied, or included in evidence.
- Windows workspace-write provided procedural isolation only and allowed filesystem reads outside the assigned snapshot. No OS-level read isolation is claimed.
- The coordinator's preparation search read a Claude transcript before the benchmark. This remains recorded in the historical PRECHECK.md. No transcript text or frozen expected-answer content was injected into an agent session.
- R-A4 read two external operator skill files; B-A4 read one. Their contents were returned into the raw tool transcripts, so both runs are marked protocol failures and contaminated. Repository transcript copies redact the skill bodies while retaining commands, result status, and access classification. Raw copies remain in local scratch.
- B-A3 attempted to read .agents/skills/graphify/SKILL.md inside its assigned snapshot. The path was absent and the shell returned no content. The launcher's broad path-string detector initially stopped on this, then an additive adjudication corrected the false positive. The attempted missing-path access remains recorded as a deviation.
- Captured tool actions showed no access to known Task 2 Claude evidence or frozen expected-answer paths. Full-filesystem reads were technically possible, so this does not prove OS-level confinement.

## Startup, processing, and environment

- B-A1 Codex configuration parsing failed before an agent turn because of the disabled MCP server configuration under ignore-user-config. This first attempt remains UNAVAILABLE; it was not retried.
- R-A1 and R-A2 agent turns completed, but launcher postprocessing was interrupted. Their original logs remain; additive reconciliations recover turn/tool/token evidence where available. Missing process exit/time fields remain UNAVAILABLE.
- B-A2's native executor rejected both command attempts because native MXC was unavailable in the nested executor. Its response accurately reported the infrastructure failure; no task facts are available, and the run was not retried.
- The launcher previously read optional process metadata such as TimedOut without checking the returned object's shape. The scratch launcher now checks all required process fields first; missing metadata records an infrastructure failure with unavailable measurements and stops without retry. PowerShell 7.6.6 parser and a synthetic missing-TimedOut check passed. This repair occurred after the eight first attempts and does not change their results.
- A4 reviewer command compiled the frozen defect and reported 3 passed, 1 failed in both final snapshots. The requested fix did not land. Full directory diff output was unavailable due a Git for Windows overlong/missing path; the relevant target file matched the frozen baseline byte-for-byte.
- The raw launcher Riltra CLI counter falsely counted Cargo --bin riltra commands in A4. The additive tool-usage adjudication corrects counts; historical raw logs were not rewritten.

## Grading and limits

- First-attempt outcomes are preserved. No session was rerun to improve a score.
- Strict format, task success, and factual correctness are reported separately in grading.json.
- Pilot results do not support statistical superiority or cross-client ranking.
- Token counts come only from provider-reported JSONL usage. Missing values remain UNAVAILABLE.
- Some agent process completion metadata is unavailable for R-A1 and B-A1; reconciliations preserve the limits.
- The status stopped_restricted_external_reference is a real protocol stop, not a successful completion. It must not be changed to PASS or cleared to permit another benchmark run.
