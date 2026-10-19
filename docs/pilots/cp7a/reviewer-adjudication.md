# CP7-A retrospective reviewer adjudication record

Status: **CP7-A measurement accepted — adjudication addendum recorded**.
This file is a **retrospective reviewer adjudication record**, added after the
CP7-A submission was reviewed. It is not part of the frozen benchmark archive:
it changes no corpus, rubric, golden, raw session, score, or aggregate, and it
reports no new measurement or rerun. The original archive — including the
`gate/verify.json` 57/57 integrity check — remains historical evidence of the
original run only.

- Recorded: 2026-10-19
- Reviewed baseline: `4f475d6d56804558cb11218ec5a1f3378e36b684`
- Benchmark source SHA: `0fd22a1134274c1e92f741fc0d4e3b4886ceeaf3`
- Scope: G1 false-negative adjudication; G3 measurement-protocol deviation and
  pilot-excluded sensitivity; preservation of all other findings.
- Unchanged: CP7-B remains BLOCKED; no M8 acceptance, master merge, tag, or
  release is authorized.

## 1. G1 — `potential-vs-calls` false negative

**Frozen automated result (unchanged):** G1 FAIL — Arm A 16/16, Arm B 15/16.
The MCP arm's single miss is the task `potential-vs-calls`.

- Question: "At `c/main.c:6:1` in repository `other`, how many caller edges are
  returned, what kind are they, and are any emitted as `CALLS`?"
- Frozen golden: caller edges = **1**; kind = **`POTENTIAL_CALL`**; edges
  emitted as `CALLS` = **0**.

The MCP answer states (verbatim excerpts from the frozen run):

> **Caller edges returned:** 1 (`completeness.covered` = 1, `total` = 1)
>
> **Kind:** `POTENTIAL_CALL` (provider `syntax`, precision `Heuristic`)
>
> **Emitted as `CALLS`?** No. The single edge is emitted as `POTENTIAL_CALL`,
> not promoted to `CALLS` (heuristic edges stay potential per the tool
> contract).

**Frozen rubric.** `corpus/corpus.json` requires these regular expressions
(applied case-insensitively by `corpus/score.cjs`, `scoreTask`):

1. `(?:\b1\b|\bone\b)[^.\n]{0,30}\bedge` — **did not match**
2. `POTENTIAL_CALL` — matched
3. `(?:no|none|not|never)[^.\n]{0,40}CALLS` — matched

The frozen scorer record (`raw/runs/tasks/potential-vs-calls/B/score.json`)
shows `correct: false`, `evidence_valid: true`, and `required_failures` listing
the first pattern (JSON-escaped there as
`"(?:\\b1\\b|\\bone\\b)[^.\n]{0,30}\\bedge"`).

**Why it failed.** The first pattern expects the count token (`1` or `one`)
within 30 characters before the word `edge`. The answer puts the count after
the noun ("Caller edges returned: 1") and later says "The single edge" without
`1` or `one`. The mismatch is wording and token ordering, not semantics: the
count, the kind, and the not-promoted-to-`CALLS` statement are present and
consistent with the golden. The Arm B evidence patterns were satisfied (the
`riltra_riltra_code_callers` call returned `callee_name "add"`,
`kind "POTENTIAL_CALL"`, `c/main.c`).

**Reviewer adjudication.** The MCP answer is substantively correct; under this
targeted manual adjudication all 16 MCP answers appear correct. The frozen
automated score is **not** changed: G1 remains **FAIL** under the frozen
rubric, the regex was not modified, no historical result was regenerated, and
no task or rubric was rewritten.

Automated score (frozen) = B 15/16, G1 FAIL. Retrospective reviewer assessment
= the lone miss is a rubric false negative and the MCP answers are
substantively 16/16. Both statements are recorded as distinct; the frozen
automated outcome remains the official gate result.

References:

- corpus: `corpus/corpus.json` (task `potential-vs-calls`: `golden`,
  `required`, `evidence`); scorer: `corpus/score.cjs` (`scoreTask`)
- Arm B: `raw/runs/tasks/potential-vs-calls/B/score.json` (score + full frozen
  answer), `raw/runs/tasks/potential-vs-calls/B/messages.json` (raw session
  `ses_eafbf67bfffeLiq36Sx4fMdgj1`),
  `raw/runs/tasks/potential-vs-calls/B/meta.json`
- Arm A: `raw/runs/tasks/potential-vs-calls/A/score.json` (`correct: true`;
  session `ses_eafbf4339ffeb8XZtpI73DZjrj`)
- tables: `summary.md`, `summary.json`; report section "G1" in
  [`README.md`](README.md#g1--correctness-fail-as-frozen-manual-audit-noted)

## 2. G3 — measurement-protocol deviation and sensitivity

**Deviation (documented during review).** The token-accounting rule
(`method/token-rules.md`, SHA-256
`F14C339C76DC916D4E3E09C164AB8F28B2DD14844EA691902352720A24AD7C56`) was frozen
after the `repo-list` pilot had already been recorded:

- pilot runs: `2026-10-18T18:18:49.9939081Z` → `2026-10-18T18:19:02.9962096Z`
  (Arm A) and `2026-10-18T18:19:03.0206815Z` → `2026-10-18T18:19:09.0486880Z`
  (Arm B), from `raw/runs/tasks/repo-list/{A,B}/meta.json`;
- the rule file's own header states it was frozen "before any measured task
  beyond the already-recorded `repo-list` pilot".

The remaining measured tasks followed the archived rule. This differs from the
originally approved requirement to freeze measurement rules before all
measured tasks. No evidence reviewed establishes post-hoc manipulation of the
threshold: the archived rule hash matches the hash recorded with the aggregates
(`summary.json` → `token_rule_sha256`), and all 32 sessions reconcile
per-message token sums with session totals (`accounting_ok: true`). The
report's frozen-setup table renders this as "frozen before measured results";
this addendum records the precise timing.

**Frozen primary metric (unchanged; G3 PASS under the archived rule).**

| Arm | Gross tokens (all attempts) | Completed | Gross per completed |
|---|---:|---:|---:|
| A (CLI) | 386,155 | 16 | 24,134.7 |
| B (MCP) | 251,468 | 15 | 16,764.5 |

MCP ≈ **30.5% lower** (30.538% computed from the frozen aggregates).

**Pilot-excluded sensitivity (separately labelled; must not replace the frozen
primary metric).** Removing the `repo-list` pilot from both arms (pilot gross:
A 16,453; B 14,590):

| Arm | Gross tokens | Completed | Gross per completed |
|---|---:|---:|---:|
| A (CLI) | 369,702 | 15 | 24,646.8 |
| B (MCP) | 236,878 | 14 | 16,919.9 |

MCP ≈ **31.4% lower** (31.351% computed; 16,919.8571 unrounded). The
pilot-excluded view is a sensitivity calculation only; it is not the reported
G3 result.

**Scope limits.** One execution per task (zero retries; no variance or
confidence interval is available). The model is the free
`opencode/deepseek-v4.1-flash-free`, no paid call was made, and its exact
remote backend revision could not be verified from the client. These numbers
describe this frozen corpus, harness, host, and model ID only; no universal
efficiency claim is made.

## 3. Other findings preserved

- **G2 PASS** — 0 unresolved citations across 32 answers, within the
  documented evidence-validation scope. Two bare `knowledge.rs` references
  (Arm B `trace-validate-new-to-id`, Arm A `impact-validate-id`) are ambiguous
  between `crates/cli/src/knowledge.rs` and `crates/cli/tests/knowledge.rs`;
  their anchors (199/89) resolve to the source file. They remain precision
  limitations; nothing here claims them fabricated.
- **G4 PASS** — CP6 adversarial suite plus the live structured-error smoke.
- **G5 PASS** — 21/21 MCP-vs-CLI deep-equal; 19/19 CLI outputs byte-identical
  to the frozen goldens.
- **G6 BLOCKED** — OpenCode subtest PASS; Claude Code was unavailable and
  remains untested.
- **G7 PASS** — local fmt/check/clippy/test green (634 passed / 0 failed) with
  CI verification recorded in the report; nothing was re-run for this record.
- **G8 PASS** — no dependency or lockfile change; the approved `rmcp` pin is
  unchanged.
- Upstream **`rust-sdk#1353`** (rmcp 3.5.1 non-cancel-safe receive on the
  invalid-request path; referenced in `crates/cli/tests/mcp.rs`) remains an
  unresolved SDK limitation. This adjudication neither resolves nor alters it.
- Acceptance criteria are unchanged; no threshold was adjusted anywhere.

## 4. Historical evidence protection

Only this file and a minimal link in [`README.md`](README.md) were added by
the adjudication. The following remain byte-identical to the archived
measurement and are not to be edited:

- `corpus/corpus.json`, `corpus/corpus.sha256`, and all frozen
  rubrics/goldens;
- all raw sessions under `raw/`;
- `measurements.json`, `summary.json`, `summary.md`;
- `gate/verify.json` — the historical 57/57 check of the original archive; it
  is not a new independent verification performed after this addendum;
- `method/token-rules.md` and the frozen benchmark configurations and scripts.
