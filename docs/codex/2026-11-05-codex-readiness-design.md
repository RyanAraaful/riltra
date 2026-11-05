# Riltra Codex readiness design

**Date:** 2026-11-05
**Status:** authorized readiness program.
**Starting point:** the repository includes merge `af91bec` (T5b, relationship-proof scopes). Treat T5b as already merged; do not re-run or re-grade it, and do not claim this plan validates it.

## Purpose

Measure Riltra's readiness on real, private Python, Rust, and C++ repositories. Check source relationships, uncertainty, freshness, and CLI/MCP parity; record actionable defects and prepare a focused upgrade PR. The result is a separate readiness decision for each language, grounded in the tested corpus and conditions.

The program consists of 12 task categories, four per language, and 12 independent Luna sessions, one for each task. It reuses existing fixtures and harnesses for setup and assertions while adding the named private repositories as read-only corpus inputs. It does not expand language scope or make performance or superiority claims.

## Scope and invariants

- **Languages and corpus:** Python, Rust, and C++ only. The required private corpus is one Python application (`PY-A`), one Rust application (`RS-A`), and two C++ applications (`CPP-A`, `CPP-B`). These are pseudonyms for public reports. Keep the mapping to actual repositories in access-controlled task records only.
- **Local-first:** default core queries run offline. Provider comparisons are authorized when useful, but remain explicit opt-in per task and run only against the trusted, isolated, read-only corpus. Record provider version, configuration, diagnostics, and any project-code execution. A provider run does not establish offline behavior.
- **Read-only source:** pin each repository revision and mount or copy it as immutable input. Do not edit, stage, or push to source repositories. Build/index state and outputs live separately in isolated scratch state.
- **Privacy and publication:** never publish source snapshots or raw CLI/MCP responses, provider indexes, logs, transcripts, credentials, private repository identifiers, or local paths. Keep raw evidence in access-controlled local storage. Public PRs may contain sanitized methods and derived results labeled only with pseudonyms; remove source text and details that could identify the repositories. Sanitize and review summaries before publication.
- **Revision privacy:** retain exact private repository revisions in access-controlled local records. Public summaries must not include private commit hashes, tags, branch names, URLs, or other revision identifiers; use a non-reversible label such as `PY-A / run-1` instead.
- **Blind oracle:** before querying Riltra or starting a Luna session, an independent reviewer records expected locations, relationship kinds, identities where provable, uncertainty boundaries, and the pinned revision. The reviewer sees neither Riltra outputs nor prior answers for the selected case. Record blinding status and exposure; contaminated cases are replaced or marked exploratory and cannot support a readiness pass.
- **12 Luna sessions:** run one independent session per task against the task's frozen input and prompt. Do not share oracle answers or other sessions' outputs before each session completes. Record model/session metadata and tool-use evidence privately; publish only sanitized findings.
- **Truthfulness:** candidates remain candidates; unverified evidence remains `Unverified`; partial coverage remains partial; stale evidence remains stale; unresolved or unsupported behavior is stated; `UNKNOWN` stays unknown. Empty output is not proof of absence. A task pass is limited to its assertions, repository revision, configuration, and platform.
- **Contracts and fixes:** preserve CLI, MCP, JSON, persisted-data, path-confinement, offline-core, and read-only MCP contracts. Trace discrepancies through callers and failure paths. The program authorizes narrow, evidence-backed fixes and regression coverage; fixes must not silently broaden a contract.
- **No premature claim:** this design and plan do not assert any task has run, any test has passed, or any language is ready. Readiness and release are separate decisions.

## Evaluation unit and outcomes

A task is one frozen case family, one pinned repository or fixture revision, and one set of assertions. Use the existing M6 corpus, language fixtures, semantic fixtures, and harnesses where they apply. Run equivalent inputs through CLI and MCP when both surfaces support the operation. Report per-task and per-language results; do not average unlike outcomes.

Before querying, freeze the oracle privately. Record Riltra revision, binary digest, platform/toolchain, source alias and revision, configuration, scratch-home identity, Luna session metadata, and provider/version if opted in. Keep failed attempts. A second reviewer checks results against the oracle and current contract. Compare parsed CLI/MCP contract fields; explain documented transport differences.

For semantic providers, retain provenance, diagnostics, skipped documents, and whether project code executed. Exit code zero alone is not evidence of correctness. Provider execution stays isolated from the default offline run and must not write to the private source repository.

A task is **PASS** only if every in-scope expected location and relationship in the frozen oracle is actually returned with the expected kind and honest certainty, no contradictory or extra fact appears, and all other frozen assertions pass. Omission of a known expected location is **FAIL** when the query is supported; if the query cannot be performed, mark it **UNAVAILABLE**, never PASS. Use **FAIL** for a contradicted contract and **INCONCLUSIVE** if the oracle or evidence cannot distinguish outcomes. State cause for every non-PASS outcome. Capability absence is not automatically a correctness failure, but it remains visible in the language readiness verdict.

## Issues, upgrade PR, and approval gates

Record each actionable finding with the issue form in the plan. The form contains pseudonyms and sanitized reproduction details only. Link private raw evidence by an access-controlled evidence ID or hash; do not attach it to public issues or PRs.

Prepare a separate upgrade PR containing only fixes tied to reproduced issue forms, regression evidence, and compatibility review. If no code fix is warranted, record that disposition and keep the evidence-only work separate. Do not merge without explicit user approval. Do not publish a release or release claim without explicit user approval. No additional approval is required to execute this authorized program, use an opted-in provider in the trusted scope, or prepare fixes and the upgrade PR.

## Readiness decision

The final report gives a distinct **READY**, **NOT READY**, or **INCONCLUSIVE** verdict for Python, Rust, and C++, with corpus pseudonyms, denominator, task/session coverage, unresolved issues, and limits. READY requires every applicable task for that language to pass, the 4 corresponding Luna sessions to be reviewed, CLI/MCP parity to pass where applicable, and no unresolved high-impact correctness or trust-boundary issue. Unavailable or contaminated evidence cannot be counted as a pass. The cross-language program status is the set of these three decisions, not a pooled score.

Program completion requires: all 12 task executions and 12 Luna sessions reviewed; issue forms for actionable findings; a per-language readiness decision with evidence and limits; and an upgrade PR or a documented no-fix disposition. Merely creating 12 records is not completion. Merge and release remain gated by explicit user approval.
