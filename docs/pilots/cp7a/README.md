# CP7-A: OpenCode CLI vs MCP benchmark (Riltra M8)

Status: **measured, frozen, and partially gated — not an M8 acceptance**.
The OpenCode A/B measurement and all local gates are complete. The Claude Code
interop subtest (CP7-B) is externally blocked, so M8 acceptance is not claimed
and this branch must not be merged, tagged, or released on the strength of this
report.

## Result summary

| Gate | Verdict | Evidence |
|---|---|---|
| G1 correctness (MCP arm) | **FAIL (frozen rubric)** | A 16/16, B 15/16; see [G1](#g1--correctness-fail-as-frozen-manual-audit-noted) |
| G2 evidence correctness | **PASS** | 0 unresolved citations across 32 answers |
| G3 tokens per completed task | **PASS** | B 16,764.5 vs A 24,134.7 gross/completed (30.5% lower) |
| G4 error paths | **PASS** | CP6 adversarial suite + live structured-error smoke |
| G5 CLI/MCP parity | **PASS** | 21/21 parsed results deep-equal; 19/19 CLI outputs byte-equal to goldens |
| G6 interop | **BLOCKED** | OpenCode subtest PASS; Claude Code client unavailable |
| G7 no M0–M7 regressions | **PASS (local)** | fmt/check/clippy/test green; CI verification separate |
| G8 dependency governance | **PASS** | no CP7-A dependency change; CP1 allowlist/licensing intact |

## Frozen setup

| Item | Value |
|---|---|
| Model | `opencode/deepseek-v4.1-flash-free` (free tier; no paid call; remote backend revision unverifiable from the client) |
| Harness | OpenCode v2.0.32, primary agent `cp7a-bench`, snapshots disabled |
| Source | Riltra `m8-unified-cli-mcp` at `0fd22a1134274c1e92f741fc0d4e3b4886ceeaf3` (isolated worktree `.cp7a/source`) |
| Corpus | 16 tasks, 12–20 requirement met; SHA-256 `D9F33A765A98723B0ACFD314865C976FB8BD58A680C911932B64C83E8530C824`, frozen `2026-10-18T18:05:37.264Z` |
| Token rule | SHA-256 `F14C339C76DC916D4E3E09C164AB8F28B2DD14844EA691902352720A24AD7C56`, frozen before measured results |
| Measurement window | `2026-10-18T18:18:50.302Z` → `2026-10-18T18:25:53.661Z` |
| Runs | 32 measured sessions (16 tasks × 2 arms), zero retries, `repo-list` pilot included |
| Host | Windows 11 IoT Enterprise LTSC 26100, Intel i5-8250U, 21.3 GiB RAM; rustc/cargo 1.99.1; git 2.45.1.windows.1 |
| Registry | 3 repos (`riltra`, `other`, `empty`); rebuilt from `home-pristine` before every task |

## Method

- **Fair A/B.** One agent definition, one model, one task text, one source
  pin, one `RILTRA_HOME` rebuild, one scorer. The only deliberate difference is
  tool access, enforced by location plugins rather than prompt wording. Both
  arms ran under one global OpenCode config with context7 disabled and one
  `riltra` local stdio server (`target/debug/riltra.exe mcp`,
  `RILTRA_HOME=<home-template>`, `codemode: false`), archived byte-exact at
  `method/global-opencode.cp7a.jsonc` (SHA-256
  `1E9671EC34F536F9E6070F0F791BE466A3ECB025BD2D5EDECE3081552047F93C`,
  matching the `global_config_sha256` frozen in `method/environment.json`
  before measurement). The pre-benchmark original and the two recorded edits
  required to derive the benchmark config are archived as well
  (`method/global-opencode.original-backup.jsonc`); running
  `node harness/verify-global-config.cjs` re-derives it byte-exact.
  - **Arm A (CLI):** `read` + `shell`; shell is allowed only for the wrapper
    `E:\dev\Riltra\.cp7a\bin\riltra-ro.cmd` with a read-only argument
    allowlist (no `;`, `|`, `&`, globs, redirection, or nested shells).
  - **Arm B (MCP):** `read` + the eleven `riltra_riltra_*` MCP tools; no
    `shell`. `read` remains in the tool list only because removing it made the
    free tier return HTTP 403 on the first request; agent permissions deny
    `C:/*` and `E:/*` reads, and the plugin blocks `read` execution.
  - Warm-up session per location (1 attempt each, 0 discovery races) confirmed
    the exact exposure before measurement. Every measured session re-verified
    it (`context.kept`): A `[read, shell]`, B `read` + eleven MCP tools;
    32/32 `exposure_ok`.
- **Isolation enforcement is real, not prompt-only.** The OpenCode location
  plugin deletes every non-listed tool at the `context` hook and rejects
  disallowed executions at `execute.before`. Arm A recorded 2 blocked attempts
  (a `;`-chained command and a wildcard query) that were retried inside the
  allowlist; all 16 A tasks still completed. Arm B recorded 0 blocked attempts.
- **Home reset.** Before every task the live home was rebuilt from
  `home-pristine`; the five main files were hashed before/after
  (`registry.json`, `index/{riltra,other}.db`, `knowledge/{riltra,other}.db`).
  SQLite sidecars are allowed and excluded from hashes. `index/empty.db` and
  `knowledge/empty.db` must never appear. All 32 runs: main files unchanged,
  no `empty` store created.
- **Token rule (frozen).** Provider-reported fields only; no local tokenizer.
  `gross = input + cache.read + cache.write + output`; `reasoning` reported
  separately as `gross_upper`; startup/handshake excluded and reported on its
  own; failed attempts stay in the numerator. Per-message sums were reconciled
  against session totals for all 32 runs (`accounting_ok: true`).
- **Scoring (frozen).** `score.cjs` applies the frozen per-task regex rubrics to
  the final answer and verifies the tool-call evidence patterns; `citations.cjs`
  resolves every cited path/line/record against the tool outputs. Tasks and
  rubrics were never edited after results.
- **Order and retries.** A/B order alternates by task as frozen in the corpus;
  each task ran exactly once per arm; no session was re-run, and no result was
  discarded.

## Measured results

Full per-task table: [`summary.md`](summary.md). Aggregate (see
[`summary.json`](summary.json) / [`measurements.json`](measurements.json)):

| Metric | Arm A (CLI) | Arm B (MCP) |
|---|---:|---:|
| Attempted / completed | 16 / 16 | 16 / 15 |
| Frozen-rubric correct | 16 | 15 (manual audit: 16) |
| Evidence patterns satisfied | 16 | 16 |
| Timeouts / errors / exposure / home failures | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| Gross tokens (all attempts) | 386,155 | 251,468 |
| Gross per completed task | 24,134.7 | 16,764.5 |
| Gross+reasoning per completed | 24,483.6 | 16,810.4 |
| Input / cache-read / output / reasoning | 50,655 / 325,888 / 9,612 / 5,582 | 20,175 / 225,536 / 5,757 / 688 |
| Tool calls | 65 | 19 |
| Turn median | 13,414 ms | 6,687 ms |

Startup is reported separately and excluded from G3: Arm A warm
`gross = 4,510` (input 112, cache-read 4,352, output 46); Arm B warm
`gross = 7,063` (input 20, cache-read 7,040, output 3). Arm B's first request
carries ~7k tokens of cached MCP tool schemas; the A arm's ~4.5k covers `read`
and `shell`. Both were one-attempt, no race.

## G1 — correctness: FAIL as frozen, manual audit noted

The frozen rubric grades B `potential-vs-calls` as incorrect. The B answer
states: *"Caller edges returned: 1 … The single edge"* with kind
`POTENTIAL_CALL`, explicitly not emitted as `CALLS` — which satisfies the
golden (`count 1`, kind `POTENTIAL_CALL`, and no promotion). The frozen regex
`(\b1\b|\bone\b)[^.\n]{0,30}\bedge` did not match the phrasing. Per the frozen
rule, the task, rubric, and score were left untouched; the manual audit would
make B 16/16.

Result: **G1 FAIL as measured** (A 16/16, B 15/16). No threshold, task, or
scoring rule was adjusted. The failure is a rubric false negative, not an MCP
correctness defect.

## G2 — evidence: PASS

All 32 answers cite only resolvable anchors: 0 unresolved path/line/record
citations. Two bare `knowledge.rs` references (A `impact`, B `trace`) are
ambiguous between `crates/cli/src/knowledge.rs` and
`crates/cli/tests/knowledge.rs`; the anchors (199/89) and the tool outputs
resolve to the source file. These are precision notes, not fabricated
references. `POTENTIAL_CALL` edges are never presented as `CALLS`, traversal
results are presented as plausible paths (not execution), stale evidence stays
labelled, and the `Unverified` status is preserved.

## G3 — tokens per completed task: PASS

Using the frozen rule, Arm B uses 30.5% fewer gross tokens per completed task
(16,764.5 vs 24,134.7). On the 15 tasks both arms completed, B uses 36.0%
fewer (15,733.9 vs 24,575.7). `gross_upper` (with reasoning) and input/output
breakdowns agree in direction. All 32 sessions reconcile per-message token sums
with session totals, so the comparison is internally consistent. Caveat: the
counts are provider-reported for a free model whose exact backend revision
cannot be verified from the client.

## G4 — error paths: PASS

CP6's adversarial suite passes in the local gate, covering: structurally
invalid JSON-RPC (`g4_structurally_invalid_jsonrpc_gets_invalid_request`),
invalid requests not killing the server
(`g4_invalid_requests_do_not_kill_the_server`), invalid tool input without
panic (`mcp_invalid_input_is_handled_without_panic`), unknown/missing/retargeted
repos and missing index (`t2_repo_status_unknown_missing_retargeted_and_no_index`),
invalid limits and unknown repos (`t3_code_search_invalid_limit_and_unknown_repo_are_classified`),
missing index without storage creation
(`t3_code_search_missing_index_matches_cli_without_creating_storage`), graph
unknown-repo/missing-index/invalid-input classification
(`t4_graph_unknown_repo_and_missing_index_are_classified`,
`t4_graph_invalid_inputs_are_classified_with_cli_messages`), schema guards for
outdated/ahead/corrupt stores (`t4_graph_schema_guard_outdated_ahead_and_corrupt`,
`f1_future_schema_is_refused_as_schema_ahead`), and missing knowledge stores
(`r1_missing_store_generation_guard_matches_cli`,
`t2_knowledge_search_matches_cli_missing_empty_populated_paging_and_escaping`).
The live smoke adds an unregistered-repo call returning a structured
`not_registered` error with the server still healthy afterwards.

## G5 — parity: PASS

A separate direct MCP stdio probe (pinned binary, `RILTRA_HOME` rebuilt to the
same state) initialized the server, listed 11 tools, and called all eleven
against the frozen registry. A deterministic CLI reference re-ran 19 commands
through the read-only wrapper. Results:

- **21/21** compared documents deep-equal between MCP and CLI (object-key order
  ignored), including both composed tools: `repo_status` = CLI `repo status` +
  `index status`; `health.status`/`health.doctor` = CLI `status`/`doctor`, with
  `repo_count` equal to the CLI repo list count.
- **19/19** CLI stdout captures byte-identical to the frozen goldens.
- Both probes left the five main home files unchanged and created no `empty`
  store; the direct server answered `not_registered` for an unknown repo and
  then served `health` normally.

Raw: [`reference/compare.json`](reference/compare.json),
[`reference/mcp-results.json`](reference/mcp-results.json),
[`reference/cli-results.json`](reference/cli-results.json).

## G6 — interop: BLOCKED (OpenCode subtest PASS)

A separate OpenCode Arm B session (same agent/config, not a corpus task, thus
excluded from G3) called `riltra_riltra_code_callers` with
`repo="does-not-exist"` and then `riltra_riltra_health`:

- the first call surfaced the structured domain error
  `{"kind":"not_registered","message":"unknown repository: 'does-not-exist'"}`
  to the model without a transport crash;
- the health call then returned version `0.1.0`, `doctor.pass: true`,
  `repo_count 3`, all eleven tools, read-only/network-off capabilities;
- the session ended `ok`, exposure was the exact 12-tool set, and the home was
  unchanged.

Claude Code was not available in this environment, so the two-client smoke
corpus cannot be completed and **G6 is BLOCKED**, per the design gate. The
OpenCode half is PASS. Raw: [`g6/`](g6/).

## G7 — no M0–M7 regressions: local PASS

Local gate on the frozen tree (`harness/run-gate.ps1`, logs under `gate/`):

- `cargo fmt --all -- --check` — exit 0, no output;
- `cargo check --locked` — exit 0;
- `cargo clippy --locked --all-targets -- -D warnings` — exit 0;
- `cargo test --locked` — exit 0, **634 passed / 0 failed** across 8 suites.

CI run
[39157182381](https://github.com/RyanAraaful/riltra/actions/runs/39157182381)
on commit `6449a42f16e9863fca2da99dd0ce6842152bb451` (the report commit): all
three jobs green — windows-latest 4m34s, ubuntu-latest 1m13s, macos-latest
1m54s, each running `cargo fmt --all -- --check`,
`cargo clippy --all-targets -- -D warnings`, `cargo check --locked`, and
`cargo test --locked`. No golden or snapshot was modified; the source
worktree was clean at the baseline commit throughout measurement.

## G8 — dependency governance: PASS

CP7-A added no dependency, feature flag, or lockfile change
(`cargo check --locked` green, `Cargo.lock` untouched). The MCP dependency
allowlist amendment and `docs/licensing.md` entries were delivered and reviewed
in CP1; the read-only stdio transport uses the already-approved `rmcp` pin.

## Final integrity verification

An independent post-measurement check (`harness/verify.cjs`; output
`gate/verify.json` and `gate/verify.log`) passed **57/57** checks:

- frozen corpus and token-rule SHA-256s across all copies;
- source pin `0fd22a1…` with a clean worktree, measured binary
  `46678765…126341`, benchmark global config, arm configs, plugins, and
  wrapper all matching the frozen hashes;
- `home-pristine` file hashes and no `empty` store anywhere;
- 32 run dirs, zero retries, exact recomputation of per-arm aggregates from
  raw per-message tokens (A 386,155 / B 251,468 gross; A 16/16, B 15/16
  correct; no timeout, error, exposure, or home mismatch);
- measured isolation exposure sets re-derived from `raw/plugin-events.jsonl`
  joined to session IDs (82 allowed calls, 2 blocked, only the two frozen
  tool sets ever exposed);
- G5 and G6 evidence booleans; and a credential-pattern scan over every
  committed evidence file (0 hits).

## Deviations, limitations, and disclosures

- **G1 false negative** is reported as measured, not repaired.
- **Free-tier model.** No paid call was made. The remote backend revision
  behind `deepseek-v4.1-flash-free` is unverifiable from the client, so token
  counts and behaviour are attributable to that model ID at measurement time,
  not to a pinned backend build.
- **One run per task.** Zero retries is a fairness rule, so no variance or
  confidence interval is available.
- **`read` tool exposure.** Kept for both arms to avoid a free-tier 403;
  execution is denied by permissions and the plugin. One A task attempted a
  blocked command and retried inside the allowlist; no B session attempted a
  denied tool.
- **Composite MCP tools.** `repo_status` and `health` intentionally return
  more than any single CLI command; G5 compares their documented components.
- **Two ambiguous citations** are precision notes, not unresolved evidence.
- **Environment scope.** One Windows machine ran the benchmark; CI covers
  compilation/tests on three OSes but not the benchmark itself.
- **CP7-B.** Claude Code interop remains blocked; M8 acceptance, master merge,
  tagging, and release are out of scope and were not performed.

## Reproduction

All harness scripts are archived here. Paths in the archived scripts are the
absolute paths used on the measurement host (`E:\dev\Riltra\.cp7a`), so
re-running on another machine requires adjusting them first. From the
repository root:

0. Install `method/global-opencode.cp7a.jsonc` as the OpenCode global config
   (SHA-256 `1E9671EC…F93C`) and the arm config from `harness/configs/` as the
   per-location config; both are the exact files used for measurement.
   `node harness/verify-global-config.cjs` re-derives the archived global
   config from the original backup and the two recorded edits.
1. `node corpus/self-test.cjs` — verify the frozen corpus hash and rubric
   self-consistency (expects `failures: 0`).
2. `powershell -File harness/run-paired.ps1` — warm-up plus 32 measured
   sessions (rebuilds the home before each task; writes raw sessions to
   `raw/runs`).
3. Score every run with the frozen scorer:
   ```powershell
   Get-ChildItem raw/runs/tasks -Directory | ForEach-Object {
     foreach ($arm in 'A','B') { node corpus/score.cjs (Join-Path $_.FullName $arm) }
   }
   ```
4. `node harness/citations.cjs` — citation audit across all runs.
5. `node harness/aggregate.cjs` — regenerate `summary.json`/`summary.md`.
6. `node reference/direct-mcp.cjs` and `node reference/cli-compare.cjs` —
   regenerate the G5 parity evidence.
7. `powershell -File harness/g6-smoke.ps1` and `node harness/g6-analyze.cjs` —
   regenerate the G6 structured-error evidence.
8. `powershell -File harness/run-gate.ps1` — local G7 gate (see `gate/`).

## Evidence layout

| Path | Contents |
|---|---|
| `summary.md`, `summary.json`, `measurements.json` | frozen aggregate, per-task table, per-run measurements |
| `raw/runs/tasks/<task>/<arm>/` | per-run prompt, raw messages, session, meta, score, home hashes, plugin events |
| `raw/runs/startup/` | warm-up sessions (excluded from G3) |
| `raw/run.log`, `raw/score-all.log`, `raw/plugin-events.jsonl` | measurement, scoring, and isolation logs |
| `corpus/` | frozen corpus, hash, scorer, self-test, builder |
| `method/` | token rules, environment, fixtures manifest, benchmark-time global config and its original backup |
| `reference/` | direct MCP probe, CLI reference, G5 comparison |
| `g6/` | structured-error smoke raw session and analysis |
| `harness/` | runner, wrapper, plugins, arm configs, gate/verification scripts |
| `gate/` | local G7 fmt/check/clippy/test logs, independent verification report |
| `goldens-raw/` | frozen CLI reference outputs |
