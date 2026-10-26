# M9 amendment: pre-release acceptance alignment (2026-10-26)

Dated amendment. It separates core acceptance from agent integration, corrects the G-MCP and G-TOK scope, and
records the M9 status at the shipped commit `b311bb6`. The frozen design gate ([m9-design-gate.md](m9-design-gate.md))
and the CP4 record ([m9-cp4-evidence.md](m9-cp4-evidence.md)) keep their original text. Each has a dated
revision note pointing here. Historical archives are unchanged.

Executor: Claude Code only. OpenCode and Codex were not run. No agent tested or measured another agent.
No production code was changed. No push, merge, tag or publish.

## 1. Owner decisions in this amendment

- **D-1.** Core acceptance and agent integration are separate groups. Their tests are not mixed. Core acceptance is
  repository validation with no agent and no API key. Agent integration is judged per supported AI coding agent,
  on that agent's own evidence.
- **D-2.** No cross-agent testing or benchmarking. Codex is NOT TESTED. That is not a failure.
- **D-3.** The G-MCP core criterion no longer requires an OpenCode session. The OpenCode session is one agent
  integration criterion.
- **D-4.** Token measurement (G-TOK) is per agent. It is not a core gate.
- **D-5.** The four C++ capability gaps (D1-C6, D2-C6, D3-C5, D4-C5) stay UNMET. Dataflow, graph expansion, CUDA
  and C++ semantic providers are not reopened in M9.
- **D-6.** Core validation requires no OpenCode API key.

Decisions D-1 to D-6 are the owner's direction for this task, dated 2026-10-26.

## 2. Core acceptance and agent integration

| Group | Scope | Needs an agent? | Needs an API key? |
|---|---|---|---|
| Core | correctness (G-SYN, G-GRF honesty and safety, G-SEM, G-INC), freshness (G-FSH), MCP protocol and contract (G-MCP core), knowledge (G-KNW), security (G-SEC), regression (tests, fmt, clippy), CI, distribution artifacts (G-LIC, G-NAT, G-CLI) | no | no |
| Agent integration | each supported AI coding agent: real use of the Riltra tools, workflow success, token efficiency (G-TOK), agent behaviour on Riltra tools | yes | depends on the agent |

## 3. Status legend

- **PASS**: evidence exists for the stated criterion at the stated commit.
- **FAIL**: evidence shows incorrect behaviour.
- **UNMET**: the criterion asks for a capability Riltra does not have.
- **UNAVAILABLE**: evidence could not be obtained. The cause is named.
- **DEFERRED**: an owner decision defers the item.
- **NOT TESTED**: no run was made. This is not a failure.

Earlier records say "NOT YET VERIFIED". For a gate, read it as UNAVAILABLE when the evidence is missing and
DEFERRED when an owner decision defers it. The earlier wording stays in the records.

## 4. G-MCP: core criterion and agent integration

### 4.1 Core MCP acceptance (repository validation)

Evidence at `b311bb6`, shipped binary built with `cargo build --locked` (debug):

- Integration tests: `cargo test --locked`, 783 passed, 0 failed. The MCP target (`tests/mcp.rs`) ran in that
  suite. Log: [pilots/m9-grf-transparency/raw/tests/cuda-full-test.log](pilots/m9-grf-transparency/raw/tests/cuda-full-test.log).
- Deterministic stdio smoke, `docs/pilots/cp2/method/mcp-smoke.cjs`, on the pinned pilot index: 12 tools
  advertised; stdout 6 of 6 lines parsed as JSON; 4 calls, 3 answered, 1 classified error (`not_registered`).
  Evidence: [raw/core/mcp/smoke/](pilots/m9-preprelease/raw/core/mcp/smoke/).
- Scripted D5 tool sequence, `docs/pilots/cp2/method/d5-mcp-sequence.cjs`: 9 calls; 8 answered; the last is the
  intended `not_registered` check. Evidence: [raw/core/mcp/d5/](pilots/m9-preprelease/raw/core/mcp/d5/).
- Read-only check: the SHA-256 of the index database file is the same before and after one read-only smoke run.
  The WAL and shared-memory sidecars created by reads are the accepted behaviour recorded in
  [pilots/m9-rc/README.md](pilots/m9-rc/README.md). Evidence: [raw/core/mcp/db-digest-readonly-smoke.txt](pilots/m9-preprelease/raw/core/mcp/db-digest-readonly-smoke.txt).

**Status: PASS (core MCP acceptance, `b311bb6`).** The same checks run again on the candidate index in §8.

### 4.2 Agent integration, per agent

| Agent | Evidence | Status |
|---|---|---|
| OpenCode | Historical CP7-A, 2026-10-18, on the M8 surface (eleven tools), OpenCode CLI v2.0.32. Revised G6 PASS (real OpenCode smoke). G4 and G5 PASS. G1 FAIL as frozen: MCP arm 15 of 16, reviewer adjudication says the miss is a rubric false negative. The frozen result is kept ([pilots/cp7a/reviewer-adjudication.md](pilots/cp7a/reviewer-adjudication.md)). | Historical PASS on the M8 surface. Not an M9 result. |
| OpenCode | M9 surface, 2026-10-26: the session could not start. The API key is expired (preflight, [pilots/m9-consolidation/raw/g-tok-preflight.txt](pilots/m9-consolidation/raw/g-tok-preflight.txt)). No session ran. | UNAVAILABLE (expired credential) |
| Claude Code | No Claude Code evidence existed in the repository. M8 recorded it as untested, and CP7-B was deferred to post-MVP. This task ran a smoke of Claude Code's own MCP client calls: health, repo list, repo status, one search, one classified error ([raw/agents/claude-code/session-smoke.json](pilots/m9-preprelease/raw/agents/claude-code/session-smoke.json)). The server binary of that session is not identified, and the registered repository is the original MMSP clone at `fa5f74d`. | smoke PASS for the observed calls. No workflow or token claim. |
| Codex | None. Not run. | NOT TESTED |

The Claude Code smoke was a single session. It is not a benchmark and was not repeated.

## 5. G-TOK: per agent, not a core gate

- **OpenCode, historical**: CP7-A G3 under the archived rule, 16,764.5 (MCP) against 24,134.7 (CLI) gross tokens
  per completed task, 30.5 % lower. A documented timing deviation in the token rule is recorded in
  [pilots/cp7a/reviewer-adjudication.md](pilots/cp7a/reviewer-adjudication.md) §2. It is historical evidence on the
  M8 surface, not an M9 release PASS.
- **OpenCode, M9 surface**: UNAVAILABLE. The preflight of 2026-10-26 failed on an expired API key. No session ran
  and no token was counted. An authentication failure is not a Riltra correctness failure.
- **Claude Code**: DEFERRED. No token measurement exists in the repository. None was run here. A measurement needs
  an owner GO and the frozen token rule.
- **Codex**: NOT TESTED.

The CP7-A archive and the failed preflight are kept as recorded. They are not turned into a PASS or into a new
measurement.

## 6. Per-agent status summary

| Agent | MCP integration (M9) | Token efficiency (G-TOK) | Basis |
|---|---|---|---|
| OpenCode | UNAVAILABLE (expired API key). Historical PASS on the M8 surface. | UNAVAILABLE (expired API key). Historical CP7-A G3 on the M8 surface. | §4.2, §5 |
| Claude Code | smoke PASS (single session) | DEFERRED | §4.2, §5 |
| Codex | NOT TESTED | NOT TESTED | §4.2, §5 |

## 7. Documents corrected, and what was kept

| Document | Change | Frozen text kept? |
|---|---|---|
| `docs/m9-consolidation.md` | Rewritten around the core and agent split. Neutral terms in the general sections. Earlier table at commit `67f0f2b`. | not frozen (dated working record) |
| `docs/m9-design-gate.md` | Dated revision notes under the §4 matrix, G-MCP and G-TOK. Each restates the requirement neutrally. | yes, the requirement text is unchanged |
| `docs/m9-cp4-evidence.md` | Dated revision notes in §3.2 and §8. | yes |
| `docs/m9-preprelease-acceptance-amendment.md` | New, this file. | not applicable |
| `docs/pilots/m9-preprelease/` | New evidence: core checks, Claude Code smoke. | not applicable |

Not changed, on purpose:

- `README.md`: general text already says "AI coding agents". The OpenCode configuration example is agent-specific
  configuration and stays.
- `docs/m9-task6-final-verification.md`, `docs/pilots/cp7a/`, `docs/pilots/m9-rc/`, `docs/m8-*`: dated records and
  frozen archives.
- `CLAUDE.md` (untracked in this checkout, so not in this commit): it says "eleven read-only tools". The README and
  the design gate describe twelve tools, eleven read-only and one provider-backed. Owner to decide whether the file
  is committed and what it says.

Agent names stay in evidence, configuration, test results and agent-specific sections. Requirements that do not
need a specific agent use "supported AI coding agent".

## 8. Core gate status at `b311bb6`

Status per §3. The "Still needed" column lists what remains for the pre-release candidate.

| Gate | Status | Basis | Still needed |
|---|---|---|---|
| G-LIC | PASS (candidate, this host) | SBOM and licence record ([m9-cp4-evidence.md](m9-cp4-evidence.md) §5). Dependency manifests unchanged since `f45ddb8`. | SBOM drift check on the final commit (release workflow) |
| G-NAT | PASS at `f45ddb8` (three CI runners, run 40185383581). UNAVAILABLE for the final commit. | CP4 §4.1 | CI and release workflow on the final commit. macOS ARM64 not covered (owner decision, §10). |
| G-CLI | PASS (candidate) | CP4 §5. Local suite 783 passed at `b311bb6`. | CI on the final commit |
| G-MCP core | PASS (`b311bb6`) | §4.1, and the candidate check in §8.1 | repeat in CI on the final commit |
| G-SYN | PASS (candidate, `b311bb6`) | §8.1 and §8.2: source-grounded checks, deterministic repeats, exact unsupported counts, first-party errors recorded | none for the gate's acceptance checks. The §2.6 anchors were hand-verified in earlier milestones; two were re-read here. |
| G-SEM | PASS | [m9-gsem-amendment.md](m9-gsem-amendment.md), 2026-10-25 | none in core |
| G-GRF, correctness and safety | PASS | Task 6 record ([m9-task6-final-verification.md](m9-task6-final-verification.md)); candidate grading (§8.1): honesty 19 PASS | none in core |
| G-GRF, capability | UNMET (D1-C6, D2-C6, D3-C5, D4-C5) | kept UNMET (D-5) | owner decision: block the pre-release or record as known limitation |
| G-FSH | PASS (pilot) | CP4 §2 | not repeated on the final commit |
| G-INC | PASS (pilot) | CP4 §3 | not repeated after the parser version change (`PARSER_VERSION` 7). Owner decides whether to repeat on the final commit. |
| G-KNW | PASS (release-level isolated round trip, `b311bb6`: 73 checks, 0 failed) | [raw/core/g-knw-round-trip.txt](pilots/m9-preprelease/raw/core/g-knw-round-trip.txt); isolated-home acceptance at `f9eed48` ([m9-gknw-amendment.md](m9-gknw-amendment.md)) | host only, as recorded |
| G-SEC | PASS (candidate) | `security/scan.sh tree` and `range 8861246..b311bb6`: no leaks, 2026-10-26. Isolation and non-mutation per CP4 §3 and §5. | CI security job on the final commit |
| G-TOK | not a core gate | §5 | — |
| G-PAR | not an M9 gate (M10 target) | design gate §4 | — |

### 8.1 Candidate checks on the index built at `b311bb6`

Built from the pinned snapshot `d62f06a` (byte copy, `core.autocrlf=false`, tracked-content hash checked) with
the `b311bb6` binary (SHA-256 `6f5ec846…`). Evidence: [raw/core/candidate/](pilots/m9-preprelease/raw/core/candidate/).

- **Two cold builds** (`cold-index.ps1`): both exit 0. Counters: discovered 2,277; processed 1,475; definitions
  18,785; imports 6,680; calls 99,720; errors 2,387; skipped binary 4; unsupported 798
  ([logs/cold-index-runs.txt](pilots/m9-preprelease/raw/core/candidate/logs/cold-index-runs.txt)).
- **Fact tables** identical between the two builds and to the G-SYN baseline, with the same database size
  (60,370,944 B): files 1,475; symbols 18,785; imports 6,680; calls 99,720; edges 106,905
  ([logs/digest-vs-gsyn.json](pilots/m9-preprelease/raw/core/candidate/logs/digest-vs-gsyn.json)).
- **Frozen queries** (22): 22 of 22 identical to the `b311bb6` probe after removing volatile metadata and the bench
  root ([queries/compare-vs-b311bb6.json](pilots/m9-preprelease/raw/core/candidate/queries/compare-vs-b311bb6.json)).
  Against the G-SYN run, 21 of 22 are identical. The one difference is `d3-2-trace-to-cuda`, the intended CUDA
  refusal text ([queries/compare-vs-gsyn.json](pilots/m9-preprelease/raw/core/candidate/queries/compare-vs-gsyn.json)).
- **MCP** on the candidate home: scripted smoke, 12 tools, stdout 6 of 6 JSON lines, classified error
  `not_registered`; D5 sequence, 9 calls, 8 answered, the last one the intended classified error
  ([mcp/](pilots/m9-preprelease/raw/core/candidate/mcp/)).
- **Grading** of the 22 candidate outputs with the candidate smoke ([grading/grading.json](pilots/m9-preprelease/raw/core/candidate/grading/grading.json)):
  honesty 19 PASS (D5-H1 and D5-H2 now PASS on the server side); capability 4 UNMET (D1-C6, D2-C6, D3-C5, D4-C5);
  evidence 2 UNAVAILABLE (D3-E1 `cuda-unsupported`, D5-E1 `session-not-run`: no agent session). The D5 agent side
  is not verified.

### 8.2 Release-candidate scan report (G-SYN)

This is the release-candidate scan report that the G-SYN gate asks for (CP3 §11 wording). It covers the
candidate index and the pinned snapshot, and is archived under [raw/core/candidate/](pilots/m9-preprelease/raw/core/candidate/).

- **Deterministic repeats:** two candidate builds have identical fact digests, and two `riltra scan` runs are
  byte-identical ([scan/scan-repeat.txt](pilots/m9-preprelease/raw/core/candidate/scan/scan-repeat.txt)).
- **Partition, exact:** ok 1,475; unsupported 798 (309 `unsupported-extension`, 489 `cuda-experimental`); skipped 4
  (`binary-or-non-utf8`). The 489 `.cu` and `.cuh` files are all `cuda-experimental`, with no index rows. The 37 `.inl`
  files are indexed ([scan/scan-summary.json](pilots/m9-preprelease/raw/core/candidate/scan/scan-summary.json)).
- **Source-grounded spans** (`docs/pilots/m9-task6/method/span-check.cjs`,
  [span/span-check.json](pilots/m9-preprelease/raw/core/candidate/span/span-check.json)): files, 1,475 digests match,
  0 byte or missing-source mismatches. Symbols, 18,785 rows: 0 line mismatches, 0 name mismatches on the 17,878 named
  rows (907 unnamed). Imports, 6,680: 0 line, 0 name mismatches. Calls, 99,720: 0 line, 0 name mismatches.
- **First-party errors recorded:** 2,387 parse errors in 179 files.
- **Sample anchors read from the candidate source:** `colmap/benchmark/reconstruction/evaluation/geometry.py:34`
  is `def normalize_vec(`, and `colmap/src/colmap/mvs/patch_match.cc:118` is `void PatchMatch::Problem::Print() const {`.
  Both match the query outputs. The full §2.6 set was hand-verified in earlier milestones; this task re-read two.
- **Read-only:** after all runs, both pinned snapshot copies are unchanged (HEAD `d62f06a`, porcelain 0, tracked-content
  hash `ddb1699c…`). The original MMSP clone is unchanged (HEAD `fa5f74d`, porcelain 0).

## 9. Known limitations to carry into the pre-release notes

- A `--to` or anchor target with an unsupported extension (for example `.md`) answers
  `anchor unresolved: source-changed` when it has no index row. Proven on the candidate index: `trace --to README.md:1:1`
  exits 3 with that message. This is the misclassification fixed for `.cu` and `.cuh` in `b311bb6`, but not yet for other
  extensions. Fix not done here, because it changes production code (§11, step 1).
- `riltra_health` reports `capabilities.network: true` and `read_only: false`. `status.networking` is `disabled`. The
  flags describe the server as a whole, which can start a subprocess through `riltra_code_semantic`. The README
  explains this. Not a change in this task.
- macOS: x64 only in CI. ARM64 is not covered.
- The `.cu` and `.cuh` files are refused as `cuda-experimental`. The trace does not name the CUDA boundary in its
  path. C++ capability gaps stay UNMET.
- Snapshot materialization: a clone made with the system Git configuration (`core.autocrlf=true`) changes the
  working-tree bytes (CRLF). The fact tables then differ from the pinned LF snapshot, so that clone is not evidence.
  The candidate run in §8 uses a byte copy of the pinned snapshot, with `core.autocrlf=false`, checked by tracked
  content hash.

## 10. Officially deferred items

| Item | Decision | Source |
|---|---|---|
| Four C++ capability gates (D1-C6, D2-C6, D3-C5, D4-C5) | stay UNMET; no further development in M9 (D-5) | this amendment; [m9-task5-capability-gap-evaluation.md](m9-task5-capability-gap-evaluation.md) |
| C++ semantic provider and clangd on MMSP | D-M9-2 deferred; C++ index spike postponed by the owner on 2026-10-25 | [m9-cpp-semantic-index-decision-2026-10-25.md](m9-cpp-semantic-index-decision-2026-10-25.md) |
| CUDA (`.cu`, `.cuh`) | unsupported by policy; no CUDA coverage claim | [m9-inl-amendment.md](m9-inl-amendment.md), G-SYN gate |
| Dataflow, graph expansion | outside M9 (D-5) | this amendment |
| Claude Code and Codex integration beyond the smoke | post-MVP interoperability | [m8-owner-acceptance.md](m8-owner-acceptance.md) (CP7-B deferral) |
| Claude Code token measurement | DEFERRED until owner GO | §5 |
| G-PAR | M10 target | design gate §4 |
| macOS ARM64 coverage | not covered | CP3 and CP4 limitations |
| v0.1.0 merge, tag, publish | separate owner GO | [m9-consolidation.md](m9-consolidation.md) |
| Rewriting history that carries local machine paths | owner-gated | `security/history-rewrite-plan.md` |

## 11. Minimum recommendation to finish pre-release

In the normal pre-release order:

1. **Close the proven correctness issue in §9** (unsupported extensions answer `source-changed`). Small change in
   `resolve_edge_anchor`, RED and GREEN tests as for `.cu`. Needs an owner GO for a production change.
2. **Finish core evidence.** The G-SYN scan report in §8.2 is done. Decide whether G-INC is repeated on the final commit.
3. **Regression, security and CI on the final candidate.** The branch is pushed at `13f083a` (owner yes, 2026-10-26).
   Check CI run `40257287230`. Then run the release-candidate workflow on the final commit. A branch push does not
   trigger it (only `workflow_dispatch` or a tag `v*-rc*`), so that choice is an owner decision. Also the SBOM drift check.
4. **Audit documents, known limitations and distribution artifacts.** Rebuild the release archives on the final commit.
   The archives built at `f45ddb8` do not contain the `b53d412` and `b311bb6` changes. Carry §9 into the release
   notes. Decide the CLAUDE.md wording.
5. **Owner decisions.** Whether the four C++ UNMET criteria block the pre-release, or are recorded as known limitations.
   macOS ARM64 coverage. Agent integration for the release: OpenCode needs a new key for a session on the M9 surface,
   Claude Code has a smoke only, Codex is not tested. History rewrite. The v0.1.0 GO.

## 12. Blockers toward pre-release (actual)

- The release-candidate workflow has not run on the final commit. A branch push does not trigger it. Owner decision
  needed on how it runs (tag `v*-rc*`, or `workflow_dispatch`). CI for `13f083a` is queued (run `40257287230`).
- The OpenCode API key is expired. No OpenCode session on the M9 surface, so OpenCode agent integration is
  UNAVAILABLE. This does not block core acceptance.
- The four C++ capability criteria are UNMET. Owner decision needed on whether they block.
- The §9 correctness issue needs an owner GO for a production change.
- The archives built at `f45ddb8` are stale for the code changes after it.

## 13. Traceability of earlier statuses

| Earlier status (record) | Current status (this amendment) |
|---|---|
| G-MCP "NOT YET VERIFIED", blocked by the expired key ([m9-cp4-evidence.md](m9-cp4-evidence.md) §5, `67f0f2b`) | core MCP PASS at `b311bb6` (§4.1). OpenCode M9 session UNAVAILABLE (§4.2). |
| G-TOK "NOT YET VERIFIED" ([m9-cp4-evidence.md](m9-cp4-evidence.md) §8) | per agent: OpenCode UNAVAILABLE on M9, Claude Code DEFERRED, Codex NOT TESTED (§5). CP7-A G3 kept as historical. |
| G-KNW "PASS for the isolated-home acceptance test; release round trip not run" ([m9-consolidation.md](m9-consolidation.md) at `67f0f2b`) | PASS, release-level round trip at `b311bb6` (§8) |
| G-SYN "NOT YET VERIFIED (formal): archived RC scan report not produced" | §8.2 |
| G-GRF "NOT YET VERIFIED" ([m9-cp4-evidence.md](m9-cp4-evidence.md) §5) | correctness and safety PASS; capability UNMET (§8) |

## 14. Evidence

- [pilots/m9-preprelease/](pilots/m9-preprelease/): core checks, Claude Code smoke, candidate checks. Local paths
  replaced with placeholders (`<BENCH>`, `<RILTRA_ROOT>`, `<DEV_ROOT>`).
- Related records: [m9-consolidation.md](m9-consolidation.md), [m9-grf-transparency-amendment.md](m9-grf-transparency-amendment.md),
  [m9-gsyn-import-span-amendment.md](m9-gsyn-import-span-amendment.md), [m9-task6-final-verification.md](m9-task6-final-verification.md).
