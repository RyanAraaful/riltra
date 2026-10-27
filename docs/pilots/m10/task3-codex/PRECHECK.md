# M10 Task 3 Codex Precheck

Status: **STOP — precheck has an unexpected tool-exposure blocker; no benchmark sessions launched.**

## Authentication and bounded isolation deviation

The dedicated benchmark `CODEX_HOME` was reused and checked with `codex login status`. The command exited 0 and reported an active ChatGPT login. No credential file was opened, copied, displayed, or linked.

The owner explicitly approved one authenticated `CODEX_HOME` shared only by the eight Task 3 sessions. This is a bounded deviation from frozen §3.4, not full per-session profile isolation. Each run uses a distinct `HOME`, `USERPROFILE`, `APPDATA`, `LOCALAPPDATA`, workspace snapshot, and Arm R `RILTRA_HOME`.

## Checks completed

- Codex CLI version: `0.164.0`.
- `codex exec --help` advertises `--ephemeral`, `--ignore-user-config`, and `--ignore-rules`.
- `codex mcp list --json` under the shared benchmark profile reports one configured server, `riltra`.
- Frozen inputs and the valid earlier freeze/A4 preflight results were not modified in this turn.
- The coordinator's earlier preparation search read a Claude transcript in Task 2 evidence. This is recorded as a preparation deviation; no transcript content was passed into a benchmark session.

## Isolation interpretation

Frozen §3.4 requires procedural isolation: new session profile/home, a fresh pinned snapshot plus its frozen prompt, separate Arm R `RILTRA_HOME`, and no prior evidence or operator-machine instructions injected. It does not require OS-enforced read confinement.

Windows `workspace-write` permits full-filesystem reads. This limitation is documented; no OS-level read isolation is claimed and no split-filesystem gate is added. Sessions remain prohibited from reading paths outside their assigned snapshot. Transcript review must inspect every model-issued file/search/shell action, resolve paths against the run snapshot, and record any out-of-snapshot read as a protocol deviation for owner assessment. Frozen-procedure tool paths and caches remain separately recorded as operational inputs.

## Checks and probes

- The owner reports that the sandbox probe from ordinary host PowerShell returned `SANDBOX_OK`; the coordinator did not repeat a sandbox command from its constrained shell.
- `codex login status` on the shared benchmark profile exited 0. No credential file was opened, copied, displayed, or linked.
- Codex CLI `0.164.0` supports `--ephemeral`, `--ignore-user-config`, `--ignore-rules`, and `--skip-git-repo-check`.
- The benchmark profile contains one configured Riltra MCP server with exactly 11 `enabled_tools`; `riltra_code_semantic` is excluded. Context7 confirms Codex applies `enabled_tools` as an allowlist.
- Before host `codex exec` probes, the benchmark profile had no skills/plugins/memories/session store/instruction files. After the probes, `skills/.system` appeared with five Codex system-skill folders and `plugins/cache/openai-curated-remote` appeared. `memories`, `sessions`, and profile instruction files remain absent. These unexpected skill/plugin assets invalidate the clean-profile check.
- Eight per-run environment/workspace directories are distinct. Each pinned snapshot lacks `.git`, `docs/pilots/m10`, `AGENTS.md`, skills/rules, prior Claude evidence, and expected-answer paths.
- Arm B's configured server is disabled by `-c mcp_servers.riltra.enabled=false`; `codex mcp list --json` reports `enabled=false`. Its run environment has no `RILTRA_HOME` and no Riltra CLI on `PATH`.
- The first host `codex exec` probes failed because frozen archive snapshots have no `.git` and Codex rejected an untrusted working directory. Adding `--skip-git-repo-check` preserved the snapshot and allowed the harmless shell probe in both arms to print `CODEX_PRECHECK_OK`. Probe transcripts contain only the requested echo command, with no file-read items.
- The first Arm B probe used `--ignore-user-config` and reported a different native-tool list than Arm R. That comparison is not accepted as parity evidence. The planned common invocation keeps the same profile and Codex settings in both arms, disabling only Riltra for B; runtime visibility and native parity under that invocation remain unverified.
- A corrected Arm R probe called `riltra_health` once. Its response listed all 11 permitted Riltra tools and no semantic tool. It also listed coordinator-only tools, including Context7 and other MCP integrations, under `other_mcp_tools`, and coordinator management/image/time tools under `native_tools`. These tools were not called. This unexpected tool catalog blocks the precheck; Arm B and native-tool parity have not been runtime-verified with the common final configuration.
- Both host probes emitted a model-refresh timeout warning. The Windows shell also warned that a sandbox-specific temporary directory under the per-run `LOCALAPPDATA` path was missing; the echo still exited 0. Record this as an environment limitation for review.

## Evidence state

- Benchmark sessions: `0/8`.
- Strict-format and factual grading, Riltra adoption, task success, timings, and token usage: not measured.
- Benchmark evidence manifest and repository security scan: not produced; no A1–A4 benchmark outputs exist. Harmless precheck probe transcripts are kept separately under `precheck-probes/`.
- No commit or push was made.

## Earlier coordinator sandbox diagnosis

Date: 2026-10-26.

- Host version/build: Windows `10.0.26300.0`, build `26300.9711`, read from the OS version API and Windows current-version registry values. `Get-CimInstance Win32_OperatingSystem` returned `Access denied`.
- Codex CLI: `0.164.0`. The benchmark profile does not set `windows.sandbox`; the prior default Windows sandbox probe failed with `CreateRestrictedToken failed: 87`.
- The coordinator-nested MXC probe was run once from the prepared R-A1 environment with the authenticated benchmark `CODEX_HOME` and the requested workspace permission profile:

  ```text
  codex -c windows.sandbox=mxc sandbox --include-managed-config --permission-profile :workspace -- cmd.exe /d /c echo MXC_OK
  ```

- Result in the coordinator shell: exit `1`, `failed to prepare MXC sandbox: native MXC is unavailable on this executor`. The later owner-run host probe returned `SANDBOX_OK`; this earlier result is limited to the nested coordinator environment.
- No fallback backend was attempted. No frozen inputs, methodology, source, or Task 2 evidence were changed.

## Current precheck outcome

Date: 2026-10-26.

- **Precheck remains FAIL:** the Arm R probe reported coordinator-only tools, and host startup populated the shared benchmark profile with unexpected skill/plugin assets. Procedural isolation and the Windows full-read limitation are recorded separately; the latter is not a new mandatory gate. Arm B effective visibility and native-tool parity under the common configuration remain unverified. Do not start A1–A4 until these tool/profile exposures are reviewed.
- Benchmark sessions: `0/8`. No commit or push was made.
