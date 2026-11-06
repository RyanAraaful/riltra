# Documentation index

Where things are, and which documents are guidance and which are historical record.

## Start here

| Read | For |
|---|---|
| [README](../README.md) | What Riltra is, how to install it, the commands, MCP setup, and its limits. |
| [Agent setup guide](guide/agent-setup.md) | Connecting an AI coding agent: which tool to call, and how to read the labels. |
| [Roadmap](../ROADMAP.md) | What is planned and what is not. |
| [Hybrid architecture roadmap](hybrid-architecture-roadmap.md) | Owner-adopted architecture direction and acceptance gates for planned stages. |
| [Changelog](../CHANGELOG.md) | What changed in each release. |
| [Installing Riltra](../release/INSTALL.md) | Installing from a release archive. |
| [Release candidate packaging](../release/README.md) | How archives are built, checked and verified. |
| [Security policy](../SECURITY.md) | Reporting a vulnerability, and what the core does and does not do. |
| [Contributing](../CONTRIBUTING.md) | Rules for contributions. |

## Reference and procedures

- [Dependency and license inventory](licensing.md)
- [Recording a Why review](riltra-why-review-workflow.md)
- [Code binding compatibility](riltra-binding-compatibility.md)

## Historical records

Everything below is a dated record of a milestone, a study or an audit: what was decided and measured at the time. These records are corrected by dated amendment documents, not rewritten, so an older record can be superseded by a later one. Start with the closure or consolidation document of a milestone, then read its amendments.

Most records are in English. The ones marked "(Indonesian)" are in Indonesian.

### Milestones M0 to M7: verification and validation

- [M0 verification report](m0-verification.md)
- [M1 verification report](m1-verification.md)
- [M2 validation summary](m2-validation.md)
- [M3 validation summary](m3-validation.md)
- [M4 validation summary](m4-validation.md)
- [M5 validation](m5-validation.md)
- [M6 validation: real providers, cross-platform CI and closure](m6-validation.md)
- [M6 Task 4: callers and callees](m6-task4-validation.md)
- [M6 Task 5: bounded static trace](m6-task5-validation.md)
- [M6 Task 6: conservative impact analysis](m6-task6-validation.md)
- [M6 Task 7: multilingual golden corpus](m6-task7-validation.md)
- [M6 Task 7 correction: persisted graph proof compatibility](m6-task7-correction-validation.md)
- [M7 validation: native structured knowledge graph and closure](m7-validation.md)

### Milestone M8: unified CLI and MCP interface

- [M8 design gate](m8-design-gate.md)
- [M8 OpenCode-first amendment](m8-opencode-first-amendment.md)
- [M8 final acceptance review](m8-final-acceptance-review.md)
- [M8 owner acceptance and milestone closure](m8-owner-acceptance.md)

### Milestone M9: multi-repository release readiness

Gates, amendments and final verification:

- [M9 design gate](m9-design-gate.md)
- [M9 consolidation: gate status at the shipped commit](m9-consolidation.md)
- [M9 pre-release acceptance alignment](m9-preprelease-acceptance-amendment.md)
- [M9 release-candidate gate amendment](m9-release-candidate-gate-amendment-2026-10-26.md)
- [M9 amendment: G-KNW release-level round trip](m9-gknw-amendment.md)
- [M9 amendment: G-SEM re-verification and `.inl` recovery review](m9-gsem-amendment.md)
- [M9 amendment: multi-line import span end (G-SYN)](m9-gsyn-import-span-amendment.md)
- [M9 amendment: `.inl` coverage (G-SYN)](m9-inl-amendment.md)
- [M9 amendment: CUDA refusal classification and device-side predicate (G-GRF)](m9-grf-transparency-amendment.md)
- [M9 grader correction and dated amendment](m9-grader-correction-amendment-2026-10-25.md)
- [M9 CP2 harness correction](m9-cp2-harness-correction.md)
- [M9 Task 5: C++ capability gap evaluation](m9-task5-capability-gap-evaluation.md)
- [M9 Task 6: final G-SYN and G-GRF verification](m9-task6-final-verification.md)

Checkpoint evidence:

- [M9 CP1: pilot spec](m9-cp1-pilot-spec.md)
- [M9 CP2: pilot validation and targeted correctness fix](m9-cp2-evidence.md)
- [M9 CP2 re-evaluation](m9-cp2-reevaluation.md)
- [M9 CP3: multi-repository verification and E8 correctness](m9-cp3-evidence.md)
- [M9 CP4: release-candidate evidence pack](m9-cp4-evidence.md)

Semantic providers, the C++ semantic index and reference resolution:

- [M9 clangd MCP integration design (proposal)](m9-clangd-mcp-integration-design.md)
- [M9 clangd MCP integration: implementation and evidence](m9-clangd-mcp-implementation.md)
- [M9 clangd semantic provider pilot](m9-clangd-pilot.md)
- [M9 LSP correctness hardening](m9-lsp-hardening.md)
- [M9 C++ semantic index: scope and integration cost](m9-cpp-semantic-index-decision-2026-10-25.md)
- [M9 C++ semantic index spike, Option A](m9-cpp-semantic-index-spike-2026-10-25.md)
- [M9 E7 reference-level resolution: design and acceptance specification](m9-e7-reference-resolution-design.md)
- [M9 E7 Slice 1: C++ data members as `field` symbols](m9-e7-slice1-implementation.md)
- [M9 E7 Slice 2: minimum implementation specification](m9-e7-slice2-implementation-spec.md)
- [M9 E7 Slice 2: occurrence evidence and design decision](m9-e7-slice2-evidence-and-decision.md)
- [M9 E7 Slice 2: implementation record](m9-e7-implementation.md)

### Milestone M10: evidence and distribution

- [M10 design gate](m10-design-gate.md)
- [M10 Task 5: distribution decision record, Option B installers](m10-task5-decision.md)
- [M10 consolidation and owner decision input](m10-consolidation.md)
- [M10 amendment: Phase B installer checks against the published 0.1.0 release](m10-phase-b-amendment-2026-11-04.md)

### What, Impact and Why: correctness studies

Oldest first, by the date in the file name; the implementation record carries its own run date.

- [What, Impact and Why follow-up](riltra-wiy-followup-2026-10-29.md)
- [What, Impact and Why: implementation and evidence](riltra-what-impact-why-implementation.md)
- [E0-S: Rust semantic provider reliability](riltra-e0s-semantic-reliability-2026-10-29.md)
- [What, Impact, Why: research and current evidence](riltra-wiy-research-2026-10-31.md)
- [Impact evidence: current contract and presentation design](riltra-impact-evidence-design-2026-10-31.md)
- [Rust semantic feasibility against the E2 call-site oracle](riltra-rust-semantic-feasibility-2026-10-31.md)
- [What, Impact, Why correctness evaluation plan](riltra-wiy-correctness-plan-2026-10-31.md)
- [What, Impact, Why: partial correctness evaluation](riltra-wiy-correctness-evaluation-2026-10-31.md)
- [Full repository Rust semantic recheck](riltra-wiy-full-project-recheck-2026-10-31.md)
- [Correctness follow-up, 2 November 2026 (Indonesian)](riltra-correctness-followup-2026-11-02.md)
- [Multilanguage evaluation plan (Indonesian)](riltra-multilanguage-evaluation-plan-2026-11-03.md)
- [Multilanguage evaluation baseline, 3 November 2026 (Indonesian)](riltra-multilanguage-baseline-2026-11-03.md)
- [Rust alias correctness: real producer, CLI and MCP, agent pilot (Indonesian)](riltra-rust-alias-correctness-2026-11-03.md)

### Audits

- [Windows WAL concurrency audit](audits/2026-10-19-windows-wal-concurrency.md)

## Evidence data

The directories under [`pilots/`](pilots/) hold the raw evidence behind the records above: captured command output, logs, manifests and fixtures. They are data, not documentation, and they are not edited; corrections are made in dated amendments (see [M10 consolidation, section 11](m10-consolidation.md#11-audit-notes-and-non-blocking-discrepancies)). Two entry points that the README links:

- [CP7-A: OpenCode CLI vs MCP benchmark](pilots/cp7a/README.md)
- [Multilanguage correctness report, interim results, 3 November 2026 (Indonesian)](pilots/multilanguage-correctness-2026-11-03/laporan.md)
