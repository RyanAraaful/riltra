# M8 owner acceptance and milestone closure

**Status: OWNER ACCEPTED — M8 CLOSED.**

This document is the authoritative record of the owner's M8 acceptance
decision and the formal closure of the M8 milestone. It changes no
measurement, criterion, or historical record.

- Owner authorization date: 2026-10-19
- Branch: `m8-unified-cli-mcp`
- Accepted milestone state (branch head at the owner decision):
  `7bfa36cb271d1dc9106a0760eff2d4894804ff6d` — the independently reviewed
  final acceptance review commit, on top of baseline
  `9debe751a2e257cb99fe8dfeddadd81ba4de40ab`. The accepted state spans the
  full M8 line from `master` baseline
  `cb40aea1581683857dd55eff7ea304c311313f31`: design gate (D1–D7), CP1–CP6,
  CP7-A, the OpenCode-first amendment, the blueprint synchronization record,
  and the final acceptance review.
- Process: the final acceptance review at `7bfa36c` was independently audited
  and adjudged fit for owner acceptance; this record is the owner decision
  that review requested. The review document itself is unchanged except for a
  post-review pointer to this record.

## 1. Owner decisions accepted

1. **M8 ACCEPTED** with the documented G1 adjudicated exception.
2. Frozen **G1 stays FAIL** (CLI 16/16, MCP 15/16); the exception is accepted
   because the single failure was a verified rubric false negative.
3. **G3 accepted as PASS with the documented methodological limitation**: the
   frozen primary result (30.538% fewer gross tokens per completed task for
   MCP) is retained, and the 31.351% pilot-excluded sensitivity is not a
   replacement metric.
4. **Revised G6 PASS** based on the real OpenCode MCP smoke; historical
   **G6 remains BLOCKED** as recorded.
5. **CP7-B deferred to post-MVP.** Claude Code and Codex remain **untested —
   not PASS**.
6. The known upstream limitation **`rust-sdk#1353`** is accepted for M8
   closure; it is not stated to be fixed and not stated to be risk-free.
7. No claim of Riltra superiority over GitNexus is made without a valid
   comparative benchmark.
8. **M8 is formally CLOSED** on the evidence and this acceptance decision.

## 2. Checkpoint closure (CP1–CP7)

| Checkpoint | Status |
|---|---|
| CP1–CP6 | CLOSED — evidence and CI-green commits per the final acceptance review (§2) |
| CP7-A | ACCEPTED / CLOSED via reviewer adjudication (commit `15a05f3`) |
| CP7-B | Deferred to post-MVP by the amendment — **not PASS** |

## 3. Gate results (historical results preserved)

| Gate | Historical result | Accepted interpretation |
|---|---|---|
| G1 | **FAIL (frozen)**: A 16/16, B 15/16 | FAIL stands; the single miss is an adjudicated rubric false negative — exception accepted by the owner |
| G2 | PASS — 0 unresolved citations across 32 answers | as recorded |
| G3 | PASS (frozen rule) — 16,764.5 vs 24,134.7 (−30.538%) | as recorded, with the documented token-rule timing deviation and the non-primary sensitivity |
| G4 | PASS — CP6 adversarial suite + live structured-error smoke | as recorded |
| G5 | PASS — 21/21 MCP-vs-CLI deep-equal; 19/19 CLI goldens byte-identical | as recorded |
| G6 | Historical: **BLOCKED** (Claude Code unavailable) | BLOCKED preserved; the revised OpenCode-only requirement is PASS on the real OpenCode MCP smoke |
| G7 | PASS — 634/0 local across 8 suites; CI green on Windows/Ubuntu/macOS | as recorded |
| G8 | PASS — licensing entries and audit test; no CP7-A dependency change | as recorded |

## 4. Exceptions and limitations carried by this acceptance

- **G1 adjudicated exception** — the frozen automated FAIL is the recorded
  gate result and is not rewritten to PASS.
- **G3 methodological limitation** — the token rule was frozen after the
  `repo-list` pilot; the primary metric is retained; the sensitivity is
  labeled and non-primary.
- **Interop scope** — revised G6 satisfied by OpenCode; historical G6 BLOCKED
  preserved; Claude Code and Codex untested and deferred.
- **Upstream SDK limitation** — `rust-sdk#1353` (rmcp 3.5.1 non-cancel-safe
  receive on the invalid-request path) remains unresolved; this closure does
  not address it.
- **No competitor-superiority claim** without a valid comparative benchmark.

## 5. What "M8 CLOSED" does not mean

- The MVP is not complete and not released; M8 is one milestone.
- M9 has not started and is not authorized.
- No merge to `master`, no tag (including `v0.1.0`), and no release is
  authorized by this record.
- Claude Code / Codex implementation and any post-MVP interoperability
  roadmap remain unauthorized.
- No acceptance criterion, benchmark corpus, rubric, golden, or historical
  result was changed by this acceptance.

## 6. References

- `docs/m8-final-acceptance-review.md` — final evidence review; its
  recommendation was accepted by the owner, and a post-review pointer to this
  record was added without changing its content or verdict.
- `docs/m8-opencode-first-amendment.md` — OpenCode-first owner decision
  (2026-10-19).
- `docs/m8-design-gate.md` — original M8 requirements (G1–G8, CP1–CP7).
- `docs/pilots/cp7a/README.md` and `docs/pilots/cp7a/reviewer-adjudication.md`
  — frozen CP7-A results and the adjudication record.
