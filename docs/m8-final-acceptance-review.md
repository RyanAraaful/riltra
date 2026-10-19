# M8 final acceptance review

**Status: REVIEW COMPLETE — PENDING OWNER DECISION.**

This document is the final evidence review requested for M8 after the owner's
OpenCode-first decision of 2026-10-19. It renders a gate-by-gate assessment and
a recommendation only: it does not close M8, does not merge, tag, or release
anything, and does not start M9. The acceptance decision belongs to the owner.

- Review date: 2026-10-19
- Reviewed baseline: `9debe751a2e257cb99fe8dfeddadd81ba4de40ab`
  (`m8-unified-cli-mcp`, equal to `origin/m8-unified-cli-mcp` at review time)
- `origin/master` unchanged at `cb40aea1581683857dd55eff7ea304c311313f31`
- Change produced by this review: this file only
- No code, benchmark, corpus, rubric, golden, schema, or dependency change; no
  benchmark rerun; no frozen score is reinterpreted as a new score

> **Post-review resolution (added 2026-10-19, after this review was written):**
> the owner accepted M8 with the documented G1 exception and G3 limitation —
> see [`m8-owner-acceptance.md`](m8-owner-acceptance.md)
> (**OWNER ACCEPTED — M8 CLOSED**). The review below is unchanged.

## 1. Scope, method, and precedence

Precedence applied:

1. `docs/m8-design-gate.md` (Revisions 1–3) defines the historical M8
   requirements (gates G1–G8, checkpoints CP1–CP7, decisions D1–D7).
2. `docs/m8-opencode-first-amendment.md` (owner decision, 2026-10-19)
   supersedes **only the client-count clauses**; all other requirements and
   every historical result remain in force.
3. Historical frozen results are read exactly as recorded and are not
   recalculated.

Evidence examined (raw evidence was spot-checked; report summaries were not
taken as substitutes where a material claim could be checked directly):

- `docs/pilots/cp7a/README.md` — frozen G1–G8 report and method;
  `docs/pilots/cp7a/summary.md` — frozen aggregate and per-task table;
  `docs/pilots/cp7a/reviewer-adjudication.md` — post-review adjudication.
- Frozen archive integrity: `corpus/corpus.json` SHA-256 `D9F33A76…C824` and
  `method/token-rules.md` SHA-256 `F14C339C…7C56` still equal the archived
  values, and both files were last modified only by `6449a42`, the archive
  commit; the review worktree contains no archive modification. The frozen
  archive is byte-unchanged.
- Raw re-count of all 32 run score records under `raw/runs/tasks`: Arm A
  16/16 correct, Arm B 15/16 correct, 32/32 `evidence_valid`; the only
  `required_failures` entry in the entire archive is the G1 regex on
  `potential-vs-calls` (Arm B).
- `reference/compare.json` raw counts: 21 comparison documents, 21/21
  `mcp_vs_cli` true, 19/19 `cli_vs_golden` true, 11 tools,
  `server_alive_after_error: true`.
- `gate/verify.json`: 57/57 historical integrity checks of the original
  archive; `gate/gate-test.log`: 634 passed / 0 failed across 8 suites.
- MCP protocol references were checked proportionately via Context7 against
  the MCP specification (rev 2025-11-25): stdio carries JSON-RPC messages on
  stdout and nothing else may be written there, stderr is the logging
  channel, and errors use either JSON-RPC protocol errors or tool-execution
  errors (`isError`). This is a protocol-reference check only — it is not a
  substitute for the recorded test evidence.
- No benchmark was rerun; no frozen artifact, golden, session, or config was
  edited; no source code changed.

## 2. Checkpoint summary (CP1–CP7)

Each checkpoint landed as tested, DCO-signed commits with the required
verification, per design gate §7. Run IDs resolve to
`https://github.com/RyanAraaful/riltra/actions/runs/<id>`.

| Checkpoint | Deliverable | Commits | CI runs | Review status |
|---|---|---|---|---|
| CP1 | rmcp/Tokio/schemars pin, licensing entries, `riltra mcp` serves initialize/tools-list | `7d4889a` | 39101348245 ✓ | evidence complete |
| CP2 | shared response assembly (single code path for CLI/MCP results) | `40a3168` | 39122699178 ✓ | evidence complete |
| CP3 | repository + search tools and the read-only store guard | `80d9212`, fixes `43a5ee6`, `303f7bc` | 39126027926 ✗ (macOS) → 39126602676 ✓, 39128252876 ✓ | evidence complete; transient CI failure resolved inside the checkpoint |
| CP4 | graph tools (callers/callees/trace/impact) over the read-only seam | `6e1b73c` | 39131971616 ✓ | evidence complete |
| CP5 | knowledge tools (search/show/related) with generation guard | `226c88c`, `a505271`, `3cc6ed3` | 39136262650 ✓, 39138074540 ✓ | evidence complete |
| CP6 | health tool and classified hard error paths | `c61ea50`, `0fd22a1` | 39141380344 ✓, 39145695907 ✓ | evidence complete |
| CP7-A | benchmark, OpenCode interop, archived docs | `6449a42`, `4f475d6`, `15a05f3` | 39157182381 ✓, 39157599429 ✓, 39173787615 ✓ | accepted and closed via adjudication |
| CP7-B | Claude Code interop subtest | — | — | deferred by the amendment — **not PASS** |

Notes:

- Design commits `6c9ecd4` (design gate) and `53a8d1b` (F1–F4 corrections)
  are CI-green; the governance commits after CP7-A are `68d00c9` (amendment,
  39196063479 ✓) and `9debe75` (blueprint synchronization record,
  39198230076 ✓).
- `a505271` is an intermediate fix commit with no isolated CI run; it was
  carried by the `226c88c` run (it is an ancestor of that commit).
- CP1–CP6 closure basis: each required deliverable and its verification tests
  are present in the recorded commits, and every checkpoint's final state
  passed all three CI platform jobs. The repository keeps no separate
  per-checkpoint acceptance ledger; this review found no evidence
  contradicting closure.
- CP7-A closure: the measurement was accepted through the adjudication record
  (`docs/pilots/cp7a/reviewer-adjudication.md`, committed as `15a05f3`).

## 3. Gate matrix (G1–G8)

| Gate | Current requirement | Evidence | Historical outcome | Review conclusion |
|---|---|---|---|---|
| G1 | MCP task correctness at least equal to the CLI baseline on the frozen corpus; no task regresses (design gate §6) | CP7-A README §G1; `summary.md`; adjudication §1; raw `raw/runs/tasks/potential-vs-calls/{A,B}/score.json` | **FAIL (frozen)** — A 16/16, B 15/16; lone miss adjudicated a rubric false negative | Frozen FAIL preserved; acceptance contingent on the owner accepting the documented exception (§4) |
| G2 | every cited location/record resolves; zero fabricated references | CP7-A README §G2; `summary.md`; `gate/verify.json` 57/57 | PASS — 0 unresolved citations across 32 answers | PASS; no summary-vs-raw contradiction found |
| G3 | tokens per completed task no worse than CLI under the frozen rule | CP7-A README §G3; adjudication §2; `summary.json` aggregates | PASS — B 16,764.5 vs A 24,134.7 (30.538% lower); timing deviation recorded | PASS under the archived rule with the documented limitation (§5) |
| G4 | classified structured errors for every mapped branch; no panics, no hangs | CP7-A README §G4 (named adversarial tests); `crates/cli/tests/mcp.rs`; `0fd22a1`; `g6/` smoke; `reference/compare.json` | PASS — CP6 adversarial suite plus live structured-error smoke | PASS |
| G5 | parsed MCP result document equals parsed CLI document; CLI goldens byte-stable | CP7-A README §G5; `reference/compare.json` (re-counted: 21/21, 19/19) | PASS — 21/21 deep-equal; 19/19 byte-identical | PASS |
| G6 | revised: real OpenCode interoperability smoke required; historical: two selected clients | CP7-A README §G6; amendment §1–2; `g6/` raw smoke | Historical BLOCKED; OpenCode subtest PASS | Revised requirement met by recorded evidence; historical BLOCKED preserved; Claude Code untested — not PASS (§6) |
| G7 | no M0–M7 regressions; fmt/clippy/check/test and goldens on all three CI platforms | CP7-A README §G7; `gate/gate-test.log` (634/0, 8 suites); CI run table (§2) | PASS | PASS; no unresolved critical regression |
| G8 | audit allowlist amendment and `licensing.md` entries merged with the dependency checkpoint | `docs/licensing.md` CP1/CP3 sections; `crates/cli/tests/semantic.rs:7`; no CP7-A dependency change | PASS | PASS; no unresolved licensing blocker for M8 (a full M9 release licensing audit is out of scope) |

## 4. G1 — adjudicated exception evaluation

Verified basis of the adjudication:

- Golden (task `potential-vs-calls`): caller edges = **1**, kind =
  **`POTENTIAL_CALL`**, emitted as `CALLS` = **0**.
- Arm B's frozen answer states verbatim: "**Caller edges returned:** 1",
  "**Kind:** `POTENTIAL_CALL`", "**Emitted as `CALLS`?** No."
- The frozen scorer matched the `POTENTIAL_CALL` and
  `(?:no|none|not|never)[^.\n]{0,40}CALLS` patterns but not
  `(?:\b1\b|\bone\b)[^.\n]{0,30}\bedge`. The raw score record shows
  `correct: false`, `evidence_valid: true`, with that pattern listed in
  `required_failures`; Arm A's identical-semantics answer recorded
  `correct: true`. The raw re-count confirms this is the archive's only miss.
- Cause: count-token ordering and wording ("Caller edges returned: 1", "The
  single edge"), not a semantic mismatch; the tool-call evidence patterns were
  satisfied.

Assessment:

- The automated and substantive statements are kept distinct: the frozen
  automated score is **FAIL** (official gate result), while the retrospective
  reviewer assessment is that the MCP answers are substantively 16/16. Nothing
  was rewritten, and no regex, task, or historical result was modified.
- The exception is narrow, isolated, and traceable to raw evidence; it does
  not indicate an MCP product defect, and the other 31 runs passed the same
  automated rubric.
- Sufficiency: the documented exception is sufficient to recommend acceptance
  **with an explicitly recorded G1 exception**, provided the owner accepts it.
  It is not sufficient to record "G1 PASS"; an accepted exception must never
  be rendered as a changed gate result.
- No rubric repair and no benchmark rerun are proposed; either would change
  frozen history and is outside this review's scope.

## 5. G3 — methodological deviation evaluation

- **Deviation.** The token-accounting rule (`method/token-rules.md`) was
  frozen after the `repo-list` pilot runs had been recorded; the rule's own
  header states this, and the pilot timestamps are recorded in the
  adjudication. The remaining measured tasks followed the archived rule.
- **No manipulation found.** The archived rule hash matches
  `summary.json → token_rule_sha256`; both arms were measured under the same
  rule; all 32 sessions reconcile per-message token sums with session totals
  (`accounting_ok: true`).
- **Primary frozen metric preserved (this remains the G3 result).**
  A: 386,155 / 16 = 24,134.7; B: 251,468 / 15 = 16,764.5 gross tokens per
  completed task — 30.538% lower for MCP.
- **Sensitivity (label only, not a new score).** Excluding the pilot:
  A: 369,702 / 15 = 24,646.8; B: 236,878 / 14 = 16,919.9 — 31.351% lower.
  This calculation must not replace the primary metric.
- **Limits.** One run per task (no variance or confidence interval), a
  free-tier model ID whose backend revision is unverifiable from the client,
  a single Windows host, and provider-reported tokens. The result describes
  this corpus, harness, host, and model only; no universal efficiency claim
  and no competitor-superiority claim is made or implied.
- **Conclusion.** G3 stands as PASS under the archived rule; the deviation and
  sensitivity are recorded limitations for the acceptance decision. No metric
  was recomputed in this review.

## 6. G6 — revised and historical status

- **Historical G6: BLOCKED.** Under the original two-client definition,
  OpenCode passed the subtest and Claude Code was unavailable, so the original
  gate could not pass. This result is preserved as recorded.
- **Revised G6 (owner amendment, 2026-10-19): PASS on available evidence.**
  The required real OpenCode MCP smoke is recorded in `g6/`: an OpenCode
  session called `riltra_riltra_code_callers` with an unknown repository — the
  structured `not_registered` error surfaced to the model without a transport
  crash — then `riltra_riltra_health`, which returned version `0.1.0`,
  `doctor.pass: true`, `repo_count` 3, all eleven tools, and read-only /
  network-off capabilities; the session ended `ok`, exposure was the exact
  12-tool set, and the home was unchanged. In addition, the 32-session CP7-A
  measurement itself ran OpenCode as the client against the real
  `riltra mcp` server.
- **Claude Code and Codex remain untested and must not be recorded as PASS.**
  The second client is deferred to a post-MVP interoperability roadmap by the
  same amendment; no such implementation is authorized here.

## 7. Material findings and outstanding blockers

No material technical blocker and no product defect was found. Items by
classification:

1. **Test/rubric false negative** — the G1 lone miss; adjudicated and recorded
   (§4). Owner decision required.
2. **Methodological limitation** — the G3 token-rule timing deviation and the
   labeled pilot-excluded sensitivity (§5). The amendment requires final
   acceptance to consider it explicitly.
3. **Scope supersession (owner decision)** — the second interop client / CP7-B
   deferral. Not a defect; must be acknowledged at acceptance.
4. **Upstream SDK limitation** — `rust-sdk#1353` (rmcp 3.5.1 non-cancel-safe
   receive on the invalid-request path; referenced in
   `crates/cli/tests/mcp.rs`) remains unresolved and alters nothing in this
   review.
5. **Precision limitation (evidence audit)** — two ambiguous bare `knowledge.rs`
   citations; their anchors resolve to the source file. Not fabricated
   references.
6. **Environment scope** — the benchmark ran on one Windows host; CI covers
   compilation and tests on three OSes but not the benchmark itself.
7. **Closed process notes (transparency)** — the CP3 head initially failed CI
   on macOS (`39126027926`, four store/index read-open tests) and was fixed
   within CP3 by `43a5ee6` and `303f7bc`; all later runs are green. The
   intermediate commit `a505271` carries no isolated CI run and was covered by
   the `226c88c` run.

## 8. Recommendation

**READY FOR OWNER ACCEPTANCE** — no material technical blocker was found.

Recommended terms of acceptance (all explicit):

- accept M8 with the **adjudicated G1 exception**: frozen G1 FAIL remains the
  recorded automated result; the exception is documented, not converted into
  a PASS;
- accept the **G3 documented limitation**: frozen primary metric unchanged;
  the pilot-excluded sensitivity remains labeled and non-primary;
- acknowledge **CP7-B deferral** and that Claude Code / Codex are untested
  (not PASS), per the amendment;
- no competitor-superiority claim is made.

This recommendation is not acceptance, closure, merge, tag, or release. If the
owner accepts, the decision should be recorded as "accepted with the
adjudicated G1 exception and the documented G3 limitation".

## 9. Decisions still needed from the owner

1. Accept or reject the documented G1 adjudicated exception (and confirm the
   "FAIL recorded, exception accepted" wording).
2. Acknowledge the G3 deviation and sensitivity as a documented limitation of
   the accepted result.
3. Confirm the CP7-B deferral and the untested status of Claude Code / Codex
   at acceptance time.
4. Acknowledge `rust-sdk#1353` as an accepted upstream limitation (optional
   tracking beyond M8).
5. Decide M8 closure and any subsequent action. Closure, merge to `master`,
   tagging, and release each require a separate explicit owner GO; M9 is not
   authorized by this review.

## 10. Status limitations

- This document does not close M8; M8 remains open pending the owner decision.
- M9 has not started and is not authorized.
- Nothing is merged to `master`, tagged, or released.
- This review is documentation-only; all historical records remain the
  authoritative source and were not modified.
