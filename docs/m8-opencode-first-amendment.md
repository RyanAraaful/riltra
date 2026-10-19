# M8 OpenCode-first amendment (owner decision, 2026-10-19)

Status: **OWNER APPROVED — documentation and governance amendment only.**
OpenCode-first through M10: OpenCode is the required agent integration for M8
and for MVP release readiness. Claude Code, Codex, and other AI agent
integrations are deferred to a separate post-MVP interoperability roadmap.
This decision partially supersedes the original M8 design
(`docs/m8-design-gate.md`) and the external blueprint **only regarding the
client-count requirement**; nothing else is revised, and no historical
approval or result is erased (the 2026-10-18 owner GO, D1–D7, and every prior
gate verdict remain on record).

- Approved baseline of record:
  `15a05f35b55e151e76d3b7ef3fefd5a33d0547de` (branch `m8-unified-cli-mcp`).
- Authority: owner decision 2026-10-19 ("OpenCode-first through M10").
- Nature: governance/docs only — no code, tests, dependencies, schemas,
  contracts, benchmarks, or releases change.
- Architecture constraint (unchanged): standard MCP over stdio; no
  OpenCode-specific coupling.

## 1. Revised acceptance criteria

### M8

- OpenCode is the required integration client.
- A real OpenCode MCP smoke must pass (the actual client against `riltra mcp`,
  not a mock).
- CLI/MCP correctness (G1), evidence (G2), token-efficiency (G3), error paths
  (G4), parity (G5), security (`m8-design-gate.md` §4), regression (G7), and
  dependency-governance (G8) requirements remain as defined.
- Claude Code and Codex are **not** prerequisites for M8 acceptance.
- Final M8 acceptance must explicitly consider the reviewer-adjudicated G1
  exception and the documented G3 limitation (see §3 below).

### G6

- Original definition (`m8-design-gate.md` §6): both selected clients complete
  the smoke corpus.
- Revised G6 for M8: real OpenCode interoperability smoke must pass.
- Claude Code remains **untested — not PASS**. The historical original G6
  **BLOCKED** result (CP7-A) is preserved as recorded; the revised scope is a
  later owner decision, not a reinterpretation of that result.

### G-MCP and M9 release readiness

- For MVP release readiness, verified OpenCode interoperability is required
  rather than two clients.
- All other applicable release gates remain mandatory and are not weakened:
  correctness, freshness, security, dependency licensing, native
  cross-platform behavior, and regression tests.

### M10

- Competitive benchmarking and public launch objectives are retained.
- Claude Code/Codex benchmarks are **not** automatically added to M10.
- No claim of competitor superiority without measured evidence.

### Post-MVP roadmap

- A future, independent interoperability objective will cover Claude Code,
  Codex, and potentially other MCP clients.
- No implementation, schedule, or acceptance criteria for that roadmap are
  authorized by this amendment.

## 2. Superseded clauses (client count only)

| Document / clause | Original requirement | Status |
|---|---|---|
| `m8-design-gate.md` §5 — interop client candidates | OpenCode + Claude Code CLI, versions pinned at CP7 | Superseded re client count: OpenCode only |
| `m8-design-gate.md` §6 — G6 | Both selected clients complete the smoke corpus | Revised: real OpenCode smoke required |
| `m8-design-gate.md` §7 — CP7 | Run "both clients" (A/B + scripted reference) | Revised re client count: OpenCode client run required |
| `m8-design-gate.md` §8 — D5 (approved 2026-10-18) | Approve the two interop clients before CP7 | Superseded re client count only; benchmark fairness protocol unchanged |
| Blueprint M8 PASS | Minimum two interoperable agent clients | Superseded re client count: OpenCode required |
| Blueprint G-MCP | stdio smoke on a minimum of two compatible test clients | Revised re client count: verified OpenCode smoke; JSON-RPC validity and no-stdout-logs requirements unchanged |
| Blueprint M9 PASS | Applicable release gates, including G-MCP | Client-count part revised accordingly; all other gates unchanged |
| Blueprint backlog #14 (MCP stdio facade) | Evidence note "2 clients smoke" | Superseded re client count: OpenCode smoke |

Everything not listed above is unchanged, including all unrelated roadmap
goals. All other blueprint M8/G-MCP/M9 clauses (JSON-RPC validity, no stdout
logs, schema validation, source correctness, token benchmark, and the
G-LIC..G-SEC gates) remain in force.

## 3. Historical benchmark preservation (CP7-A)

No CP7-A artifact may be edited by or after this amendment: frozen scores, raw
sessions, token rules, corpus, goldens, and summaries stay byte-identical. The
following distinctions are recorded and must not be conflated:

- **Frozen G1 automated score: FAIL** — CLI 16/16 versus MCP 15/16 on the
  frozen rubric. This is **not** rewritten to PASS.
- **Reviewer adjudication:** the one MCP failure (`potential-vs-calls`) was a
  rubric false negative; the MCP answer was substantively correct (see
  `docs/pilots/cp7a/reviewer-adjudication.md`).
- **Frozen G3: PASS** with the documented token-rule timing deviation; the
  labeled pilot-excluded sensitivity does not replace the primary metric.
- **Original G6: BLOCKED** — OpenCode subtest PASS; Claude Code unavailable.
- **Revised OpenCode-only interoperability requirement: supported by CP7-A
  smoke evidence** — the OpenCode interop subtest passed against the real
  server (see `docs/pilots/cp7a/README.md`).

**Final M8 acceptance must explicitly consider the reviewer-adjudicated G1
exception and the documented G3 limitation.** Nothing in this amendment
converts either into an unqualified PASS.

## 4. Blueprint synchronization

- The blueprint (`OPEN_SOURCE_CODE_INTELLIGENCE_KNOWLEDGE_BLUEPRINT.md`) is
  **not version-controlled in this repository** (ignored by name in
  `.gitignore`; the maintained private copy lives outside the repo, per
  `docs/m4-validation.md`).
- **External synchronization completed 2026-10-19:** the private blueprint was
  synchronized to this amendment (all byte-identical private copies brought to
  the same state) — M8, G-MCP, and M9 client-count clauses, backlog #14
  evidence note, and a revision note citing this document and decision commit
  `68d00c9`. No other clause changed. SHA-256 after synchronization:
  `1316CDEB87DC70BA4803D48B9F5788940142862911E10C2AC6B490CCA070196A`.
  The blueprint remains private and untracked; no private path or blueprint
  content is recorded in this repository.

## 5. Restrictions carried by this decision

No Rust production code or tests, no new tools or dependencies, no benchmark
rerun, no MCP schema or CLI contract change, no CP7-A evidence edits, no
Claude Code or Codex implementation, no automatic M9 start, no merge to
`master`, no tag, no release. This document changes governance only.

## References

- `docs/m8-design-gate.md` — original M8 design: gates G1–G8, checkpoints
  CP1–CP7, decisions D1–D7 (Revision 3 note points here).
- `docs/pilots/cp7a/README.md` — CP7-A results (G1–G8) and interop evidence.
- `docs/pilots/cp7a/reviewer-adjudication.md` — post-review adjudication
  record.
- `.gitignore` — blueprint ignored by name (not version-controlled here).
- `docs/m4-validation.md` — note that the private blueprint copy lives outside
  the repository.
