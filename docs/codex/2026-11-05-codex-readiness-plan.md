# Riltra Codex readiness pilot plan

**Date:** 2026-11-05
**Status:** authorized; task execution and results are not yet recorded here.
**Design:** [Codex readiness design](2026-11-05-codex-readiness-design.md)

## Corpus and shared protocol

The required corpus is four pinned private repositories, represented in all public records only by pseudonyms: `PY-A` (Python), `RS-A` (Rust), `CPP-A` and `CPP-B` (C++). Keep the pseudonym mapping, actual identifiers, source, and raw outputs in access-controlled local records only. Each source repository is an immutable, read-only input; use isolated copies or read-only mounts, with all Riltra state and outputs elsewhere. The same corpus inputs support four tasks per language.

1. Start from the reviewed base containing T5b merge `af91bec`; treat it as already merged, with no new validation claim. Record the exact Riltra revision, source revision, binary digest, platform/toolchain, and configuration in access-controlled local records. Public documents use pseudonyms and non-reversible run labels only; never publish private commit hashes, tags, branch names, URLs, or other revision identifiers.
2. Reuse the existing M6 fixtures, semantic fixtures, goldens, tests, and harness setup. Use those as controls, then run the planned cases against the real private corpus. Do not add languages or a benchmark framework.
3. Before any query or Luna session, have an independent reviewer freeze the oracle: expected locations and relationship kinds, identity only where supported, uncertainty, and checked source revision. The reviewer must not see Riltra results or prior answers. Every selected task must have at least one positive in-scope expected location. Replace contaminated cases or mark them exploratory; they cannot pass readiness.
4. Execute one independent Luna session per task (12 total, four per language). Freeze the prompt and tool access per task. Do not expose the oracle or other session outputs before a session completes. Preserve model/session metadata and tool-use traces privately.
5. Run core checks offline. Provider comparisons are authorized as explicit opt-ins within the trusted isolated scope when they answer a task question. Record provider/tool version, configuration, diagnostics, skipped documents, and whether project code ran. Never let a provider modify the source repository.
6. Capture commands, exit codes, parsed results, freshness, verification, completeness, unresolved reasons, truncation, and paging privately. Compare CLI and MCP contract fields on equivalent inputs wherever supported. Grade every frozen expected location: it must actually appear with the expected relationship kind and honest certainty for a PASS. If a supported query omits any expected location, it fails; an empty output alone never establishes PASS. Keep all failed attempts; explain any unavailable or excluded case.
7. Use the issue form below for each actionable finding. Open separate issue forms per defect; link private evidence only by access-controlled ID/hash. Prepare a distinct upgrade PR for reproduced, in-scope fixes and regression coverage. Keep raw evidence out of issues, PRs, and release materials. Sanitize public summaries and use only corpus pseudonyms.
8. Request explicit user approval before merging the upgrade PR or publishing a release. No additional approval gate applies to executing the program, running an opted-in provider in the trusted scope, or preparing fixes.

## Python: `PY-A`

| ID | Task category and procedure | Acceptance criteria |
|---|---|---|
| T1 | **Definition identity and collisions.** Choose a real-repository anchor with same-name definitions in different scopes/files; use an existing M6 fixture as a control. Freeze declaration locations, then query search/definition/callers as supported. | Every result maps to the pinned source. Distinct definitions remain distinct; no name-only candidate becomes a proven binding. |
| T2 | **Calls, references, imports, and aliases.** Freeze direct calls and non-call references, including imports/aliases, for selected real-repository anchors. Compare callers and search outputs with the oracle. | Locations and relationship kinds match the oracle. Imports/references are not silently promoted to calls or resolved identities; candidate and unresolved labels are accurate. |
| T3 | **Freshness and incremental change.** In an isolated copy, record baseline, edit, rename, and delete transitions using existing freshness harness patterns. Compare incremental with full-index facts on the same final tree. | Freshness and deleted-file behavior match the contract; fact and relevant query results agree between incremental and full. Mode counters may differ. Stale results are never labeled fresh. |
| T4 | **CLI/MCP parity and failure paths.** Compare found, empty, missing-index, stale, and paged Python cases over both surfaces where supported. | Parsed facts, ordering, certainty, freshness, completeness, and classified errors agree under the existing contract. Transport-only differences are explained; source remains untouched. |

## Rust: `RS-A`

| ID | Task category and procedure | Acceptance criteria |
|---|---|---|
| T5 | **Direct calls and uncertain ownership.** Freeze ordinary calls and closure/trait-dispatch boundaries from real source; use existing Rust fixtures as controls. | Supported direct calls match the oracle. Unproven ownership or dynamic dispatch is not reported as certain; test-only or name-only matches do not count as production facts. |
| T6 | **Semantic import and provenance.** Run the existing Rust SCIP import path when the provider is opted in. Record producer provenance, diagnostics, omissions, import status, and oracle queries. | Imported rows retain provenance and the `Unverified` ceiling. Exit success alone is insufficient; producer omissions and diagnostics are visible; no unsupported `Verified` or freshness claim appears. |
| T7 | **Incremental, stale, and provider/source transitions.** Use an isolated source copy for edit, rename, delete, and provider artifact mismatch; compare full rebuild facts on the same final tree. | Freshness, partial, and unresolved labels remain truthful. Fact parity holds; operational counters are mode-specific. Failed/interrupted updates cannot leave a false fresh/complete claim. |
| T8 | **CLI/MCP parity, pagination, and impact.** Compare search, callers, and bounded trace/impact results including a classified error. | CLI/MCP agree on facts, order, verification, freshness, completeness, truncation, and `UNKNOWN`. Transport failures preserve uncertainty; source stays untouched. |

## C++: `CPP-A` and `CPP-B`

| ID | Task category and procedure | Acceptance criteria |
|---|---|---|
| T9 | **Translation-unit identity and overloads.** Freeze declarations, overloads, cross-file calls, and header anchors from both private C++ corpora. Run syntax baseline; opt into the existing clangd path in the isolated scope where available. | Distinct overloads and definitions stay distinct. Syntax candidates are not described as semantic resolution. Provider results carry provenance and stay within the current verification contract. |
| T10 | **Headers, macros, templates, and indirect dispatch.** Select real anchors that exercise available header and unsupported/ambiguous constructs; compare with existing fixtures. Oracle observable source facts and uncertainty before query. | Returned spans map to source; macro/template/indirect limits remain explicit. Unsupported or ambiguous edges are not presented as resolved, and omitted edges do not prove absence. |
| T11 | **Provider failure and conservative impact.** In trusted isolation, compare syntax-only and opted-in clangd behavior where available. Include missing/invalid compilation database or provider diagnostics and query impact on incomplete evidence. | Provider failure is surfaced without stronger silent fallback. Incomplete evidence stays partial or `UNKNOWN`; positive complete-impact claims do not exceed contract. Core offline behavior remains provider-free. |
| T12 | **Freshness and CLI/MCP parity.** Make an edit only in the isolated copy, include provider artifact mismatch, then compare stale, unresolved, and paged outputs over CLI/MCP where supported. | Source/provider mismatch is not fresh. Interfaces agree on facts, certainty, freshness, pagination, completeness, and classified errors. Source repositories remain unchanged. |

## Luna session schedule

Run exactly 12 independent sessions, one for each T1–T12, after its blind oracle is frozen. This is four sessions per language. Give each session only its assigned task, pinned isolated corpus view, and permitted tools; do not reveal oracle answers. Retain full prompts, session metadata, tool calls, and raw outputs privately. A reviewer grades each completed session against the precommitted oracle and records whether Riltra tools were used, whether source was independently inspected, and which claims remain uncertain. Session completion or a correct final answer alone is not a task pass.

## Issue form

Use one form per actionable issue. Keep public text sanitized and pseudonymized; private raw details stay in the local evidence store.

```text
Title:
Task ID and language:
Corpus pseudonym and sanitized run label:
Riltra revision / binary digest:
Severity and user-visible consequence:
Contract or documented behavior:
Sanitized reproduction steps:
Frozen expected facts (evidence ID only):
Observed facts and uncertainty labels:
CLI/MCP comparison:
Root cause and affected callers:
Proposed smallest fix:
Regression evidence required:
Private evidence ID/hash (exact pins and raw material stay in access-controlled storage; no attachment):
Public-safe summary reviewed by:
Disposition: fix / accepted limitation / unavailable / inconclusive
```

## Upgrade PR and per-language readiness

Prepare one upgrade PR for fixes supported by reproduced issue forms. Keep changes narrow, add regression evidence, and review compatibility across callers, CLI/MCP, JSON, persistence, offline core, and provider opt-in boundaries. If no fix is justified, document a no-fix disposition and do not invent a code change. The PR and any public summary contain no source excerpt, private identifier, local path, raw output, or credential.

Issue a separate readiness verdict for Python, Rust, and C++. **READY** requires all applicable task assertions for that language to pass, the four corresponding Luna sessions to be reviewed, required CLI/MCP parity to pass, and no unresolved high-impact correctness or trust-boundary defect. Use **NOT READY** where supported evidence shows a blocking issue and **INCONCLUSIVE** where evidence is contaminated or insufficient. Unavailable is never treated as a pass. Report tested denominators and limits; do not pool languages.

The program is not complete merely because 12 records exist. Completion requires execution and review of all 12 tasks and Luna sessions, issue-form disposition for each actionable finding, per-language readiness verdicts with evidence and limits, and an upgrade PR with evidence-backed fixes or a documented no-fix disposition. Require explicit user approval before merge and before any release.
