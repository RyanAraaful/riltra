# M10 Task 4 precheck report

Date: 2026-10-27 (UTC)
Repository: `<REPO>`
Branch: `m10-evidence-distribution`
HEAD: `38e92a46a339441d9f941b8dd292ea25f4c92a10`
Freeze commit: `6b800fba6625e89cbd9428f988c6e9a1877b893a`
Pin commit/tree: `7f86891c0942cc1d7efd6ec29c6c0b6b1606e727` / `b2bd50be9ec829fd122635696a2cb95c29fe2bcc`

## Verdict

Conditional GO. The Arm R and Arm B contracts can be met on this host, but four
conditions must be accepted and recorded before A1-A4:

1. The Riltra MCP server still advertises 12 tools at the JSON-RPC protocol
   level. OpenCode's model catalog exposes 11; `riltra_riltra_code_semantic` is
   absent and a forced raw call is rejected. If the design gate requires the
   server itself to register only 11, this precheck is a STOP, because that
   needs a source change.
2. The benchmark must use a pre-warmed `opencode serve` and `run --server`.
   A cold `opencode run --standalone` sends its first provider request before
   MCP connects, so the first turn has no Riltra tools.
3. Startup equivalence is not yet demonstrated for Arm B. The executed Arm R
   probes used `run --server http://127.0.0.1:28994`; the executed Arm B
   isolation probe used `run --standalone`. The isolation result is valid, but
   one Arm B probe with the same pre-warmed server method, or an explicit owner
   acceptance checked in the first B session, is required before GO.
4. The captured catalog request exposes two built-in OpenCode skills
   (`opencode`, `report`). No operator skill is present, and the R/B logs show
   no directory-skill source. Built-ins are client-provided and equal across
   arms, but the run is not zero-skill. Record this as an isolation limitation.

If any condition is not acceptable, STOP before A1-A4. See the addendum at the
end of this report for the packaging-review findings behind conditions 3 and 4.

## Verified

- Freeze files: `sha256sum -c SHA256SUMS.txt` in
  `docs/pilots/m10/freeze` passed 12/12. Freeze directory has no diff from the
  freeze commit.
- Pinned snapshot: the Arm R workspace is byte-identical to
  `git archive 7f86891c...` (`diff -r` clean, 1785 files). Arm B workspace is
  byte-identical to Arm R.
- Toolchain: cargo `1.99.1 (5f94df478 2026-08-27)`, rustc `1.99.1
  (58cf8f9c3 2026-10-13)`.
- Client: OpenCode `v2.1.4`, model `deepseek/deepseek-v4-pro`. The OpenCode
  Console key is expired (`api_key_expired`, 401); the owner's DeepSeek key was
  injected into child-process environments only. No credential was copied into
  a profile, the repository, or a log.
- Arm R binary: built once from the pinned tree, SHA-256
  `71D1F84170ADF5A240495A217AE95C4D5C8CDCD7698654EA5ADCB759284240C7`,
  `riltra 0.1.0`. Arm R index: 1785 discovered, 187 processed, 1 known error,
  freshness `fresh`.
- Arm R model catalog: 23 tools = 12 native + 11 Riltra. The 11 are callees,
  callers, impact, search, trace, health, knowledge related/search/show,
  repo list/status. `riltra_riltra_code_semantic` is not in the catalog.
- Forced semantic call: rejected with
  `No tool named "riltra_riltra_code_semantic" is currently available`.
- Arm R runtime: health tool returned `versions.riltra=0.1.0` and
  `capabilities.transport=stdio`; shell probe resolved `riltra.exe` on PATH and
  printed `riltra 0.1.0`. Exit 0 for both.
- Arm B runtime: `Get-Command riltra` empty, `RILTRA_HOME` absent, no
  `riltra-home`, no MCP server configured; shell tool worked. Exit 0.
- Provider accounting: every probe has provider-reported tokens, cache, cost,
  duration, and outcome in `sessions/*/session.json`. No token estimate is
  used.
- Cargo preflight: `cargo test --locked --bin riltra json::tests` passed
  (4 passed, 0 failed, 425 filtered), exit 0. Shared target directory
  `...\m10-cache`, outside every workspace. No source changed.
- Isolation: separate HOME/XDG/TMP per arm, only the isolated
  `profile\config\opencode\opencode.json` loaded, zero skills, no operator
  plugins or MCP servers. `debug paths` shows every path inside the per-arm
  profile.

## Findings that changed the method

- The `permissions` array (action/resource/effect) documented for MCP servers
  did not hide the tool in OpenCode 2.1.4: the request still contained all 24
  tools. The `permission` map (`{"riltra_riltra_code_semantic": "deny"}`) is
  the form that removes it from the model catalog. This is the configuration
  now used in `config/R-opencode.json`.
- A cold `run --standalone` loses the MCP tools on the first provider request.
  The final Arm R probe used a pre-warmed `serve` on 127.0.0.1:28994; its first
  request had the Riltra tools and the model called health directly.
- `opencode debug config` without an explicit service port can attach to the
  operator's managed service (port 28894). It is not used for benchmark
  evidence. The benchmark uses `--standalone` or an explicit `--server`.

## Not exercised

A1-A4 sessions, the frozen A3/A4 edits, the A4 reviewer test, grading, commit,
push, merge, release, and Task 5. No repository source, frozen input, or grading
file was changed. `CLAUDE.md` remains untracked and untouched.

## Redaction and final scan

- Bundled artifacts replace local absolute paths with `<USERPROFILE>`,
  `<REPO>`, and `<TOOLCHAIN>`. Raw originals remain in the precheck session
  directories outside this bundle.
- `gitleaks 8.30.1 dir evidence` reports no leaks after redaction. The scan
  covers secrets, credentials, and local absolute paths.
- Final repository state: HEAD, branch, and pin unchanged; `git diff HEAD` is
  0 lines, freeze diff is 0 lines, and the only status entry is untracked
  `CLAUDE.md` (operator file, untouched).

## Evidence map

- `commands/` - freeze hashes, git state, snapshot diff, toolchain, isolation
  paths, MCP tool list, cargo log, secret scan.
- `config/` - exact Arm R and Arm B OpenCode profiles.
- `sessions/R-server-health/` - real-model MCP health probe, raw JSONL,
  session totals, run metadata.
- `sessions/R-cli/` - real-model CLI-on-PATH probe.
- `sessions/B-isolation2/` - real-model Arm B isolation probe.
- `methodology/capture-requests.jsonl` - raw provider request bodies captured
  at a local OpenAI-compatible endpoint, including the 23-tool catalog.
- `methodology/capture-health-transcript.jsonl` and
  `methodology/capture-semantic-transcript.jsonl` - harness dispatch tests.
- `provenance.json` - machine-readable identity and accounting.
- `hashes/SHA256SUMS.txt` - hashes of this bundle.

## Residual risks

- The index error is the known B-16 finding, not a new defect.
- OpenCode still watches `C:\ProgramData` for location events, although no
  system config was loaded. `OPENCODE_CONFIG_PROJECT_DISABLE=1` did not stop
  project location detection; the snapshot has no project config.
- The local capture endpoint replaces DeepSeek only for the catalog capture;
  it does not exercise provider authentication. The real-model probes do.
- The semantic tool's server-side registration remains visible in the health
  payload. The deny is enforced by the client tool catalog and dispatch, not by
  the server.

## Addendum: packaging review (2026-10-27)

These findings were made while collecting the review bundle, after the first
report was written. They do not change the raw session transcripts.

### Startup equivalence (condition 3)

The Arm R health and CLI probes used
`opencode run --server http://127.0.0.1:28994`; the pre-warmed server starts on
127.0.0.1:28994 and logs `mcp connected server=riltra tools=12` before the first
model request. The Arm B isolation probe used `opencode run --standalone`
(`sessions/B-isolation2/run-meta.json`). Both probes used isolated profiles and
workspaces, but the executed startup method is not the same. The Arm B isolation
content is still valid: no Riltra CLI, no `RILTRA_HOME`, no index, no MCP, and
the shell tool worked. Before A1-A4, run one Arm B probe through the same
`serve` and `run --server` method on a separate port, or record an explicit
owner acceptance and verify it in the first B session.

The R profile log also records a separate OpenCode process (`run=89f86d92`) at
05:13 that queried `<REPO>` from its location service; it is not one of the
three evidence sessions. Use a unique port and password per run, and verify no
other local client attaches before A1-A4.

### Built-in skills (condition 4)

The catalog capture (`methodology/capture-requests.jsonl`, lines 2, 4, 6-10)
contains two built-in OpenCode skills in the system prompt: `opencode` and
`report`. No operator skill appears. The R/B logs show `skills=[]` for every
directory source. `OPENCODE_DISABLE_CLAUDE_CODE_SKILLS=1` disables Claude Code
skills only; it does not remove OpenCode's built-in skills. Both arms receive
the same client build, so this is equal treatment, but the session is not
zero-skill as originally summarized. Record it or disable the built-ins before
A1-A4.

### What the bundle does not prove

- It does not prove that no configuration is reachable through
  `C:\ProgramData`; it proves only that no system config was loaded in these
  runs. The B/R logs show a watcher subscription, not a config read.
- It does not prove server-side removal of the semantic tool. It proves the
  OpenCode client did not expose or dispatch it.
- It does not prove Arm B pre-warmed-server behavior or token parity. No A1-A4
  prompt was run.

# Adjudication — corrected Arm B pre-warmed probe (2026-10-27)

Status: **PASS**. The owner approved exactly one corrected Arm B probe after attempt 1 was
invalidated by multiline prompt truncation through `opencode.cmd`.

- Invocation: direct `node_modules/@opencode/cli/bin/opencode.exe`
  (SHA-256 `b12b0613e9b4a491e5b12d90a5e9bbae6d261fe9688c82bf73125a036963d399`),
  `serve` pre-warmed on 127.0.0.1:29095, then `run --server`; no `cmd.exe` shim.
- Prompt: 283 bytes, SHA-256 `e218ee932b3920999f55922569a24268c9d32d0784ec1209d31022ff47e791bc`.
  The captured user text is wrapped in one pair of double quotes; dequoted and
  LF-normalized it matches the prompt hash exactly, including the `Command:` paragraph.
- Execution: one `shell` call, exit 0, output matches an independent run
  (workspace path, `19`, `False`, `False`); workspace 1785/1785 files, 0 diff after.
- Native configuration: 12 tools, 0 Riltra, 0 semantic, 0 MCP servers, 0 plugins; only the
  built-in skills `opencode` and `report`.
- Isolation: server PID 20168 child of our wrapper on port 29095; operator service PID 10556
  on 28894 unused; old Arm R precheck PID 19760 on 28994 unused; three established sockets
  all owned by the probe server; ports closed after the run.
- Evidence: `m10-task4-b-probe-corrected-2026-10-27.zip`,
  SHA-256 `1b443ba1b49e494f35dd038f81508bb2bdc48f5aaa6c45c7faa50dbdae80b43e`. Attempt 1 remains `m10-task4-b-probe-attempt1-INVALID-2026-10-27.zip`,
  SHA-256 `358065db107880b060b5b21180d2d27c4a0b18e6745159c2cfa96d8345786bd9`, INVALID.
- Accepted limitations: `riltra_riltra_code_semantic` stays denied client-side (Arm R server
  advertises 12, agent receives 11); built-in skills `opencode`/`report` accepted if
  identical on both arms; positional prompt arrives with one pair of wrapping quotes.
- Benchmark preparation: `run-benchmark.ps1` is plan-only unless `-Execute` is given.
  Its plan shows 8 unique sessions and ports (29100-29107), direct executable on both
  arms, and `serve` + `run --server`; static check in `benchmark-prep/static-checks.txt`.
  No A1-A4 session was run.
- No Riltra source, freeze input, commit, push, or earlier evidence was changed.

# Adjudication - M10 Task 4 OpenCode eight-session first-attempt pilot (2026-10-27)

Status: **PARTIALLY UNAVAILABLE**. Three of eight slots produced an agent answer; five were
stopped by provider quota before the first turn. No slot was retried, so the runset cannot
estimate a Riltra effect.

- Pre-flight: HEAD `38e92a46a339441d9f941b8dd292ea25f4c92a10`; freeze `SHA256SUMS.txt` 12/12 OK;
  pin tree `b2bd50be9ec829fd122635696a2cb95c29fe2bcc`; snapshot 1,785 files; `Cargo.lock`
  `9b9ba4dc…`; both frozen patches apply; cargo/rustc 1.99.1.
- Invocation: same direct `node_modules/@opencode/cli/bin/opencode.exe`
  (SHA-256 `b12b0613e9b4a491e5b12d90a5e9bbae6d261fe9688c82bf73125a036963d399`, OpenCode 2.1.4),
  pre-warmed `serve` + `run --server` on both arms, model `deepseek/deepseek-v4-pro`, eight
  independent profiles/workspaces/ports 29100-29107/passwords/sessions.
- Prompt delivery: 8/8 match the frozen prompt after removing one wrapping quote pair and
  normalizing LF. Literal byte identity is not claimed.
- Completed: A1-R 77.112 s / 9,785 in / 1,153 out / 0.018626784 USD; A1-B 36.254 s / 4,657 /
  1,010 / 0.007757244; A2-R 47.832 s / 11,973 / 2,003 / 0.018632328. Totals: 161.198 s,
  26,415 input, 4,166 output, 8,156 reasoning, 144,768 cache-read, 0.045016356 USD.
- UNAVAILABLE (`AI.Error.QuotaExceeded: Insufficient Balance`, HTTP 402, zero tokens and cost, no
  agent turn): A2-B, A3-R, A3-B, A4-R, A4-B. Request ids: `75c247b0-…`, `b9340396-…`,
  `8dc329a5-…`, `960da790-…`, `10ffb2db-…`.
- Grades (Task 2 precedent): all three completed runs strict FAIL on output format only
  (preamble; both A1 answers and A2-R also fence the block; A2-R also adds trailing prose),
  factual PASS (A1 5/5 byte slices verified; A2 24/24 call sites), task PASS with zero agent
  workspace changes. Five UNAVAILABLE.
- A4: the frozen defect was present before each A4 session (exit 101,
  `3 passed; 1 failed`, `json::tests::escapes_quote_backslash_and_newline`) and the reviewer
  rerun on both final workspaces reproduced it. No agent fix exists.
- STOP audit: no forbidden filesystem access, no operator-service connection (pid 10556 on
  28894 unused), no model/provider change, no native-config divergence, no prompt change, no
  operator skill loaded. Riltra MCP calls 0 and Riltra CLI calls 0 in every completed session.
- Independent reviewer: regraded from raw evidence; verdict PARTIALLY UNAVAILABLE, not
  compromised; no grading discrepancy. Report in the bundle at `grading/reviewer-report.md`.
- Deviations: provider quota exhaustion (no retry); the launcher wrote its workspace diff
  baseline before the frozen A3/A4 patch, so grading recomputes from
  `workspace-pre-session.tsv` (recorded after setup) - every run has zero agent changes;
  answer style caused the three strict FAILs.
- Bundle: `m10-task4-opencode-pilot-20261027.zip`, 195 entries, 194 manifest hashes verified.
  gitleaks: zero findings; no residual local path or credential-shaped string. The ZIP SHA-256 is
  recorded in `pilot-bundle-sha256.txt` beside the ZIP (a hash cannot be embedded in a file
  inside the bundle it hashes).
- **Owner closure (2026-10-27):** the owner declined to retry the five quota-stopped slots.
  Status stays PARTIALLY UNAVAILABLE; all eight first attempts are preserved unchanged. No
  capability conclusion is drawn: three completed sessions, identical A1 content on both arms,
  and no A3/A4 agent answer cannot establish a Riltra effect in either direction.
- No Riltra source, freeze input, commit, push, Task 5 work, merge, tag, or release was made.
  A future attempt requires separate owner approval and a fresh freeze.

