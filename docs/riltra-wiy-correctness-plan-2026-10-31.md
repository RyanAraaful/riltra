# What, Impact, Why Correctness Evaluation Plan

> For agentic workers: use the project-authorized implementation workflow and applicable Superpowers skills. This is an evaluation plan; an implementation plan for a demonstrated defect must be written separately. Checkboxes track work, not permission to change contracts.

**Goal:** Establish what the existing Windows CLI and MCP correctly answer for a small independently specified Rust oracle, including evidence, freshness and limitations, before choosing changes to production code.

**Architecture:** Reuse the existing syntax index, opt-in SCIP import, graph queries and explicit knowledge bindings. Run the same pinned source with separate source, home, target, producer and output directories; compare actual CLI/MCP answers to a prewritten oracle. Preserve existing trust labels and record every failed, unavailable or inconclusive case.

**Tech stack:** Rust CLI and existing tests, Windows PowerShell, rust-analyzer SCIP if available; GitHub CI for relevant Linux/macOS regression checks when a later change warrants it.

**Spec and evidence:** [research note](riltra-wiy-research-2026-10-31.md), [Rust feasibility](riltra-rust-semantic-feasibility-2026-10-31.md), [impact evidence](riltra-impact-evidence-design-2026-10-31.md), and current code and goldens. The local `docs/riltra-guide/` is a draft and remains untouched.

## Global constraints

- No deletion, recursive cleanup, moving existing data, profile edits, or changes to data elsewhere in `D:\DEV`; leave scratch intact. Create unique directories within the writable workspace. Verify resolved paths before any write.
- No source, home, build, logs, output, archive or index generated inside the source snapshot being measured. Record a manifest before indexing and after all queries; compare it.
- No core network use. Semantic producer is explicit opt-in, may execute build scripts/procedural macros, and is not run on untrusted source. Record producer command, configuration, logs and side effects; do not assert disabling scripts guarantees no execution.
- Preserve `Unverified`, `UNKNOWN`, `POTENTIAL_CALL`, unsupported and stale semantics; a reference is not automatically a call. Do not reinterpret the owner-approved anonymous-scope rule.
- Pin HEAD, exact source selection, binary hash, toolchain and OS. Build the evaluated binary from the pinned code in a separate target directory, or explicitly mark binary equivalence unknown and the result inconclusive for HEAD.
- No source/schema/API changes, dependency additions, published benchmarks, commits, pushes or releases are part of this plan. Owner-approved product changes, if needed, get their own design and regression plan.

## Review focus

- Same-name symbols and aliases must not become a proven wrong binding — Task 1.
- A reference that is not a call must not become `CALLS` — Task 1.
- Closure and dynamic dispatch must not be attributed to a definite runtime caller without evidence — Tasks 1–2.
- Changed source, missing provider and empty results must not appear fresh or safe — Tasks 2–3.
- Missing, unrelated, stale or revised knowledge must not be presented as a current verified rationale — Task 3.

## Task 0: Freeze scope and independent oracle

**Inputs:** existing M6 corpus, `confined_read` oracle and current code. **Outputs:** a case table and manifest in a new run directory; no changes to historical fixtures.

- [ ] Select existing cases first; add a minimal self-authored fixture only where coverage is absent. Record source snapshot, exact paths/line spans and expected relation independently before querying Riltra. Cover direct call, same-name collision, alias, non-call reference, closure, trait-object dispatch and missing-provider file. Define which facts cannot be determined from static evidence.
- [ ] Record a source-backed knowledge record and its exact revision, binding and evidence; include unrelated anchor and no-binding cases. Record expected behavior on a changed source copy separately.
- [ ] Have the oracle reviewed independently of actual outputs; record who reviewed it and unresolved disputes. Do not change expected rows after seeing results without a dated correction and reason.
- [ ] Build or identify a binary tied to the pin; capture hash, version, commands, tool versions and source manifest. Classify build or environment failure as `UNAVAILABLE`, not a product failure.

## Task 1: What — syntax and semantic caller evidence

**Inputs:** Task 0 oracle and a fresh index. **Outputs:** per-case source span, relation, provenance, verification, expected versus observed; command stdout, stderr and exit status.

- [ ] Run `search`, `callers` and, where needed, `callees` against syntax-only data. Check exact source ranges; classify candidate, reference, call, missing and incorrect separately.
- [ ] On trusted isolated fixture, run explicitly opted-in rust-analyzer SCIP with recorded settings; save producer diagnostics separately, import the artifact, then repeat queries. Retain the `Unverified` ceiling. Record indexer errors even if exit code is zero.
- [ ] Compare each result to the frozen oracle, including the accepted closure rule. Count false positive, false negative and unresolved separately, with denominators limited to the annotated universe. Do not call an unresolved case a false positive merely for remaining unresolved.
- [ ] Run corresponding MCP queries and compare meaningful JSON fields to CLI, allowing only documented transport differences. Preserve actual responses and identify mismatches.

## Task 2: Impact — paths and incomplete evidence

**Inputs:** Task 0 annotated call graph and Task 1 indexes. **Outputs:** path-by-path comparison plus reasons for incomplete results.

- [ ] Run syntax-only and SCIP-backed `impact` on a named anchor; verify each reported edge's owner, target and span against source. An empty result is not coverage proof.
- [ ] Exercise missing provider, unresolved owner and a deterministic traversal limit. Check `UNKNOWN`, `partial`, reason and truncation fields against the existing contract. Record observed path evidence even when overall verdict is `UNKNOWN`.
- [ ] Exercise a sufficiently complete, small existing fixture for the currently supported positive verdict, and an incomplete fixture; compare with existing tests/goldens. No inference about production completeness follows from a fixture.
- [ ] Compare CLI and MCP outputs and classify unavailable or untested cases explicitly. A change to snapshot-wide gating is a separate owner decision, not a pass condition.

## Task 3: Why — explicit binding and freshness

**Inputs:** Task 0 knowledge record and code anchor. **Outputs:** create/bind/read results and freshness evidence, in isolated stores.

- [ ] Create or import the source-backed record with its actual provenance, bind the exact anchor and revision, then query `knowledge at` and `riltra_knowledge_at_anchor`. Check exact ID, revision, relation, evidence and `Unverified`; do not claim the content is verified.
- [ ] Query an unbound and an unrelated anchor; verify neither yields an invented binding. Create a new record revision and check the binding still identifies its pinned revision.
- [ ] Modify only an isolated source copy after recording the fresh baseline. Query again and assert the contract's stale indication; preserve both source hashes and output. Never modify the original snapshot or user repository.

## Task 4: End-to-end usage example

**Inputs:** validated outputs from Tasks 1–3. **Outputs:** one reproducible usage example with honest boundaries.

- [ ] Run one concrete What → Impact → Why question using CLI and MCP, from the same pinned snapshot and explicitly linked knowledge; include commands, expected data and actual references to evidence.
- [ ] Explain observed caller evidence, impact path or `UNKNOWN`, and record provenance/freshness separately. Confirm the example can be followed without hidden existing local state.
- [ ] If a real agent/client is used, record client version, actual tool calls and fallback; do not claim productivity improvement without a comparative protocol.

## Task 5: Correctness evaluation and report

**Inputs:** frozen oracle, outputs and manifests from all tasks. **Output:** dated report under `docs/` only when evidence is reproducible and sanitized; otherwise record the blocker and preserve local artifacts.

- [ ] Review all cases for fabricated spans, wrong symbol identity, incorrect `CALLS`, false safety, silent staleness, cross-repository leakage and invented knowledge links. Any occurrence is `FAIL-CORRECTNESS`; honest unsupported results are separately `UNMET`/`UNSUPPORTED`.
- [ ] Report scope and pin, annotated universe, per-case expected/observed, CLI/MCP comparisons, diagnostics, correctness failures, uncertainty, costs and status (`PASS` only for the exact cases supported by evidence; otherwise `FAIL`, `UNAVAILABLE` or `INCONCLUSIVE`).
- [ ] Recheck source hashes and absence of writes outside isolated directories. Sanitize local paths and secrets before adding raw evidence; retain original data locally without deleting it. Document whether an evidence package is portable and verify links and hashes.
- [ ] Prioritize only observed defects. For each proposed change, identify root cause, affected contracts and callers, a failing regression test and Windows plus relevant GitHub CI checks. Do not infer a fix merely from a timeout or `UNKNOWN`.
