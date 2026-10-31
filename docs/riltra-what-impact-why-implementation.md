# Riltra What → Impact → Why: implementation and evidence

**Run date:** 2026-10-29
**Implementation baseline:** `master`, `2852ddb4658afdfdcd0286f9a5d1a47f4f5bb87d`
**Status:** implemented and locally verified on Windows; capability verdicts remain deliberately bounded.
**Raw evidence:** [`docs/pilots/riltra-what-impact-why-implementation-2026-10-29/raw/`](pilots/riltra-what-impact-why-implementation-2026-10-29/raw/)

## Executive result

This change improves the Rust-first path from source facts to conservative caller candidates and adds explicit, revision-pinned bindings between a code anchor and a knowledge record. It does not make name matches semantic facts and does not claim a positive impact result without sufficiently complete call evidence.

| Capability | Result on frozen `confined_read` oracle | Meaning |
|---|---|---|
| What | **PARTIAL — 10 of 13 invocation sites returned** | All six production sites and four test sites have exact observed spans. Three test invocations are inside Rust assertion macro token trees and are not emitted as call facts. Each of the 13 oracle rows has an explicit observed span or a specific reason in `raw/15-callsite-accounting.csv`. No precision/recall score is reported because the returned set is not complete and the supported universe is not a semantic-call universe. |
| Impact | **UNKNOWN — correct conservative outcome** | The real snapshot produces no exact `CALLS` path: `visits.edges=0`, coverage is partial, and unresolved owners/targets/provider coverage remain. The run must not be reported as safe or as a positive affected set. |
| Why | **Explicit binding works; anchor precision remains heuristic** | CLI and MCP return the same binding to record `e2-codex-confined-read-pr14`, revision 1, relation `constrains`, freshness `fresh`. The symbol URI is absent and binding support/record verification remain `Unverified`; a binding records an attributed relationship, not truth. |

The previous E2-W `EXPECTED.md`, `STATE.md`, `REPORT.md`, evidence manifest, and source archive were treated as frozen inputs. A new run was executed in an isolated home and its outputs are under the raw-evidence directory above. Hashes before/after are recorded there. The original Why capture had a baseline SHA missing its final `d`; it is preserved unchanged but superseded by the corrected CLI/MCP run in `raw/25-why-corrected/`, verified against the full baseline SHA shown above.

## Scope and design decisions

Work followed **What → Impact → Why** because each stage depends on the evidence quality of the prior one. The baseline and boundaries come from the owner-approved implementation plan and the six existing `docs/riltra-guide` documents. Those six documents and the historical E2-W artifacts were not edited.

1. **What: expose candidate coverage without converting candidates into identity.** The prior query filtered Rust candidates through unqualified import context. Calls written as `crate::semantic::confined_read(...)` did not have that import and were omitted. Candidate selection now includes Rust files within the same conventional `src` root for a local Rust anchor. This widens candidate discovery only; it does not establish Cargo target, module, or callee identity. Edges stay `POTENTIAL_CALL`, `Heuristic`, `Unverified`, with `callee=null`. A same-name match remains ambiguous. The remaining limitation is explicit in the source `shortcut:` comment: Cargo target metadata is needed before this scope can be narrowed with stronger ownership evidence.
2. **Impact: preserve the existing evidence threshold.** Traversal remains bounded and only exact supported edges can establish paths. `POTENTIAL_CALL`, `REFERENCES`, stale evidence, unresolved owner/target, incomplete provider coverage, and empty results do not prove safety. The actual Rust provider inventory had `rust-analyzer 1.99.1`, but no `scip-rust`, `scip`, or `.scip` artifact for the pinned snapshot. No reference occurrence was promoted to a call. Consequently this repository run correctly remains `UNKNOWN`.
3. **Why: add a stored, attributable relation.** A binding joins an exact source anchor to an exact knowledge record ID and revision, with relation, author, rationale, provenance, source/snapshot identity, and freshness. Old anchors stay stored when source changes; the query reports `stale` or `unresolved` and never silently rebinds to a new symbol or revision. Knowledge record-to-record relations remain unchanged.

The implementation follows existing CLI, SQLite, JSONL, backup, and MCP layers. It adds no graph database, cloud service, dependency, or broad refactor.

## What diagnosis and coverage

The frozen oracle contains 13 calls: six in production code and seven in tests. Before changing candidate selection, the call-site inventory and query rows were compared. Five production calls were missing because they were fully qualified through `crate::semantic` without a corresponding `use crate::semantic` import; three test calls were nested in assertion macro token trees and produced no scanner call row. The remaining production call and four other test calls were already visible. After widening Rust candidate-file discovery, all six production calls and four non-macro test calls appear with source spans.

The exact 13-row mapping is in [`raw/15-callsite-accounting.csv`](pilots/riltra-what-impact-why-implementation-2026-10-29/raw/15-callsite-accounting.csv). The rows retain the oracle line, source invocation, expected byte span, whether the query returned it, classification, and reason. [`raw/19-initial-what-gap-diagnosis.csv`](pilots/riltra-what-impact-why-implementation-2026-10-29/raw/19-initial-what-gap-diagnosis.csv) accounts for the eight sites omitted by the previous query: five had syntax/graph candidates filtered at query selection because fully qualified crate calls lacked import context; three never became scan facts inside assertion macro token trees. The three macro-contained calls are not assigned fabricated engine spans. The regression `rust_callers_include_same_crate_calls_but_keep_same_name_as_candidates` checks that a qualified same-crate candidate becomes visible, a same-name ambiguity remains a candidate, and another crate is not included.

The real run returned the six production rows and four test rows as `POTENTIAL_CALL` / `Heuristic` / `Unverified`. There is no recall/precision claim: the output is incomplete, includes name candidates rather than proven call targets, and its denominator is only this pinned oracle—not every Rust invocation in every supported Cargo configuration.

## Impact contract and provider result

Impact consumes the existing bounded graph and retains the existing `UNKNOWN` default. The run output in [`raw/07-impact.stdout.txt`](pilots/riltra-what-impact-why-implementation-2026-10-29/raw/07-impact.stdout.txt) records the snapshot, budgets, completeness counters, unresolved reasons, and zero traversed edges. The relevant meaning is:

- an exact graph fixture with direct/transitive `CALLS` is expected to return supported paths (covered by existing impact tests in `crates/cli/src/edges.rs`);
- a `POTENTIAL_CALL`, a reference occurrence, missing provider, stale source, unresolved owner/target, or exhausted traversal budget is not promoted to proof;
- an empty affected set is not proof of no impact when coverage is partial;
- verdict and capability are separate: **honest UNKNOWN** is correct behavior, while positive repository impact capability remains **UNMET** until provider-backed call evidence and adequate coverage exist.

Provider data for this snapshot is in [`raw/14-provider-inventory.json`](pilots/riltra-what-impact-why-implementation-2026-10-29/raw/14-provider-inventory.json) and [`raw/20-provider-snapshot-accounting.json`](pilots/riltra-what-impact-why-implementation-2026-10-29/raw/20-provider-snapshot-accounting.json), with the CLI semantic status in `raw/08-semantic-status.stdout.txt`. The inventory distinguishes an installed rust-analyzer executable from a provider artifact imported by Riltra. No SCIP artifact means its artifact digest is explicitly unavailable; the pinned source archive digest and run scope are recorded. References remain references. A later provider integration must preserve occurrence roles and provenance and show a call-specific contract before it can supply `CALLS`.

## Why binding contract

### Stored fields and freshness

The additive schema-v3 `code_bindings` row stores:

- registered repository name plus canonical-root digest;
- indexed snapshot ID, nonce and digest;
- source digest, repository-relative path, byte start/end and cursor line/column;
- symbol URI when one is available, display name, anchor kind and precision;
- exact record ID and revision;
- one typed relation: `documents`, `constrains`, `implements`, `tests`, `motivates`, or `observes`;
- author, rationale, provenance and creation time.

Retractions are stored separately with author, reason and time. The original binding remains available for audit. Freshness compares the bound source and snapshot identity with current indexed source. A changed source is not rebound automatically. A missing source/anchor or repository identity mismatch is reported unresolved. Freshness and explicit confirmation do not validate the truth of the record’s contents.

### CLI and MCP

The CLI surface is:

```text
riltra knowledge bind --repo <name> --path <rel> --line <n> --col <n> --input -
riltra knowledge unbind --repo <name> --binding <id> --input -
riltra knowledge at --repo <name> --path <rel> --line <n> --col <n> [--symbol <uri>] [--limit <n>] [--offset <n>] [--generation <n>]
```

`bind` input names the exact `record_id`, `record_revision`, typed `relation`, `author`, `rationale`, and `provenance`. `unbind` input supplies author and reason. Duplicate bindings are rejected rather than silently merged; retraction is append-only; a record revision is never substituted for the revision requested. Path traversal and cross-repository/root access fail closed.

MCP adds `riltra_knowledge_at_anchor`, a read-only counterpart to `knowledge at`. It accepts the repository, relative path, line/column, and optional symbol/pagination/generation fields. An unlinked anchor remains unlinked; it does not infer a relationship from matching names or paths. The corrected CLI and MCP payloads are structurally equal after JSON canonicalization; both returned `linked`, revision 1, relation `constrains`, `fresh`, and `Unverified` in [`raw/25-why-corrected/09-cli-mcp-parity.json`](pilots/riltra-what-impact-why-implementation-2026-10-29/raw/25-why-corrected/09-cli-mcp-parity.json). The original captures remain unchanged but are superseded because their record body contained the mistyped baseline SHA.

### Store compatibility and portability

- Existing knowledge schema v1 and v2 stores remain readable under their prior contract. The code-binding tables are added at schema v3 only inside an explicit binding write or import transaction. Read-only MCP does not create a missing store or migrate an old one.
- Foundation-only record reads/writes keep their existing migration behavior. Transaction rollback leaves the prior schema marker intact on failure. Unsupported future versions fail closed.
- JSONL export/import adds a versioned `code_binding` envelope when bindings exist. It validates relation, anchor, record revision, repository identity and provenance; repeated identical data is idempotent while conflicting provenance is an error. Retractions travel with the binding. Existing record-only export/import remains supported.
- Backup manifest v2 includes the binding count. Manifest v1 remains readable. Verify and restore compare the binding count, and verification checks binding/retraction structure; this is structural integrity, not cryptographic authenticity.
- CLI read operations do not make or migrate stores. MCP remains read-only.

## Acceptance and evidence

| Acceptance condition | Result | Evidence and reason |
|---|---|---|
| Pinned What oracle: count every invocation with correct path/span or specific unavailability reason; preserve candidate labels | **PARTIAL, audited** | `raw/15-callsite-accounting.csv`, `raw/05-callers.stdout.txt`; 10 observed candidates with exact spans, three macro-inner invocations have a specific scanner gap. All six production sites are present. No false semantic identity is claimed. |
| Same-crate qualified Rust names are visible; same-name ambiguity stays heuristic | **PASS for bounded query behavior** | `crates/cli/src/index.rs` unit regression and `raw/cargo-test-final.log`; candidate root is conservative and excludes sibling crate roots. |
| Exact graph impact returns bounded paths; incomplete/ambiguous cases do not become SAFE | **PASS for existing graph tests; real repo capability UNMET** | `crates/cli/src/edges.rs` tests in `raw/cargo-test-final.log`; live `raw/07-impact.stdout.txt` is `UNKNOWN` with zero edges and partial coverage. No positive real-repository path was demonstrated. |
| Provider source, version, digest, coverage and availability are recorded | **PASS: unavailable artifact documented** | `raw/14-provider-inventory.json`; rust-analyzer is installed, but the SCIP CLI, Rust SCIP indexer, and snapshot artifact are absent. Provider-backed calls remain unproven. |
| Code anchor reads exact binding and record revision in CLI/MCP; no implicit link | **PASS on corrected pinned run** | `raw/25-why-corrected/` contains the full-SHA record create, binding, CLI lookup, MCP lookup, and `09-cli-mcp-parity.json`. The earlier `raw/09-knowledge-create.*`, `raw/10-knowledge-bind.*`, `raw/11-knowledge-at.stdout.txt`, `raw/13-mcp-at.stdout.txt`, and `raw/16-cli-mcp-parity.json` are retained unchanged but superseded for the Why verdict. Both corrected surfaces agree structurally. The anchor’s precision and record verification stay `Heuristic`/`Unverified`. |
| Source edit, reindex, repository swap, duplicate/retract, revision pin, pagination, path escape, import/export, backup/restore and old-store migration are covered | **PASS in focused/integration tests** | `crates/cli/tests/knowledge.rs`, `crates/cli/tests/mcp.rs`, migration tests in `crates/cli/src/knowledge_store.rs`; full test log at `raw/cargo-test-final.log`. A changed source retains the original binding and returns stale; it does not rebind. |
| CLI/MCP parity for equivalent read fields | **PASS for tested anchor lookup** | `raw/16-cli-mcp-parity.json`; comparison canonicalizes JSON object key order and compares field values. |
| `cargo fmt --check`, Clippy, locked build, tests | **PASS with bounded environment exclusions** | Original isolated-run logs and fresh checkout logs (`raw/cargo-fmt-commit.log`, `raw/cargo-clippy-commit.log`, `raw/cargo-build-commit.log`, `raw/cargo-test-commit.log`) are preserved with `raw/22-commit-validation.json`; counts: 805 passed, 0 failed, 3 filtered. The excluded hard-link and junction tests were attempted and failed only because the host denied `mklink /J` / hard-link operations; raw initial failure and standalone junction output are retained. |
| Gitleaks on the staged change set | **PASS after path redaction** | Pinned gitleaks 8.30.1 first found 133 local-path detections across 33 evidence files and no credential-rule findings. After redaction, `raw/23-gitleaks-staged.*` records exit 0 with zero findings. |
| Cross-platform CI | **NOT RUN** | This local run did not run CI on Linux/macOS. This remains a validation gap. |

CLI evaluation captures preserve command arguments, stdin inputs where applicable, stdout, stderr and exit status as separate files. Cargo verification output is preserved as process logs with exact arguments and exit codes. The final commit-tree refresh is recorded in `raw/22-commit-validation.json` with per-command logs. The initial pinned Gitleaks pass found 133 `local-absolute-path` detections across 33 evidence files; no credential rule was reported, and the scanner redacted all matched values in its report. The newly created raw captures were then sanitized with stable root placeholders; `raw/24-redaction-accounting.json` records each affected file’s before/after SHA-256, replacement counts, and the limited trailing-blank-line normalization applied to three captured test logs. [`raw/00-protected-before.json`](pilots/riltra-what-impact-why-implementation-2026-10-29/raw/00-protected-before.json) and `raw/00-protected-after.json` fingerprint all six guide files and the frozen E2-W oracle/evidence/source archive. `raw/99-evidence-manifest.sha256.txt` hashes the new run package after canonicalizing CRLF and CR line endings to LF while preserving all other bytes; this keeps hashes stable across Git line-ending conversion. The frozen oracle’s own manifest is included in the protected-file comparison.

## Verification details and environment limits

The original evaluation commands ran in an isolated baseline copy, with Cargo target, test home and test temp below `.codex-wiy-work`. A fresh pre-commit verification ran on the current checkout with Cargo target and temp under `.codex-wiy-commit-check`; its exact commands, environment and results are in `raw/22-commit-validation.json`.

```text
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked -p riltra
cargo test --locked -- --skip ignore::tests::ignore_hardlink_inside_is_read_not_falsely_rejected --skip semantic::tests::confined_read_reports_non_not_found_stat_errors --skip status_rejects_windows_junction_retarget
```

The final successful filtered run used the exact test names shown above; the failed unfiltered run and the separate junction-only attempt are retained for context. The suite result is authoritative in the raw log.

The complete unfiltered `cargo test --locked` was attempted. It reported 440 passing and two host-permission failures: a hard-link operation and `mklink /J` in the confined-read test. A separate junction test also failed because `mklink /J` was denied. The three affected tests were then excluded by exact test name; the remaining 805 passed in both the isolated baseline copy and the fresh pre-commit checkout run. No product tests were edited to hide these environment constraints.

Toolchain was `rustc 1.99.1 (58cf8f9c3 2026-10-13)` and `cargo 1.99.1 (5f94df478 2026-08-27)`. The final staged scan used the owner-supplied gitleaks 8.30.1 executable; `raw/23-gitleaks-staged.*` records its binary hash, command, zero-findings JSON report and exit 0. CI on Linux/macOS was not run. Raw captures redact local root paths; review the redaction accounting before sharing the evidence outside the workspace.

## Research basis and fit

These primary sources describe mechanisms used by existing tools; they are design references, not claims of feature parity or measured competitor performance:

- [GitHub code navigation](https://docs.github.com/en/repositories/working-with-files/using-files/navigating-code-on-github) combines syntax-aware indexing (Tree-sitter) with name search for symbol navigation. Riltra’s syntax spans and clearly labelled candidates are a useful fallback, but do not provide binding identity by themselves.
- [Sourcegraph precise navigation](https://sourcegraph.com/docs/code-navigation/precise-code-navigation) separates search-based fallback from SCIP-based precise navigation; precise indexing is opt-in and depends on an indexer/artifact. [Writing a SCIP indexer](https://sourcegraph.com/docs/code-navigation/writing-an-indexer) emphasizes symbol roles and source occurrences and encourages golden checks of ranges/symbols. This supports leaving Riltra’s references as references and testing source spans against a pinned oracle.
- [CodeQL call graph navigation](https://codeql.github.com/docs/codeql-language-guides/navigating-the-call-graph/) distinguishes static callee relationships from possible runtime dispatch. Riltra’s current heuristic candidates have even less target evidence, so they must not drive a positive impact verdict.
- [Embedded ADR](https://adr.github.io/e-adr/) documents explicit code-to-decision references. Riltra uses a persisted typed binding with identity, author and provenance instead of deriving a relationship from coincidental names or paths.

## Remaining work in priority order

1. Add syntax facts for invocations inside Rust macro token trees, or a provider-backed occurrence route that accounts for them. Keep their macro-expanded semantics explicit: source occurrence alone does not prove runtime call behavior.
2. Establish a reproducible Rust SCIP producer/artifact for the same pinned source snapshot. Record tool versions, digest, occurrence roles, call-specific evidence and coverage. Promote only a provider occurrence whose contract proves a call; references alone remain references.
3. Re-run on a host/CI environment that permits the hard-link/junction tests, run `security/scan.sh` with pinned gitleaks 8.30.1, and run the repository’s relevant cross-platform CI. Until then, these gates are open.
4. Only after exact provider evidence exists, evaluate whether real Impact can return a positive affected set; maintain `UNKNOWN` when owner resolution or coverage remains insufficient.

## Implementation files

- `crates/cli/src/index.rs`: Rust same-source-root candidate coverage; no semantic promotion.
- `crates/cli/src/knowledge_binding.rs`: typed binding model, validation, freshness, query, retraction and portability helpers.
- `crates/cli/src/knowledge_store.rs`: schema v3, transactional migration, integrity verification and stored binding operations.
- `crates/cli/src/args.rs`, `crates/cli/src/main.rs`: CLI parser, commands and JSON responses.
- `crates/cli/src/mcp.rs`: read-only `riltra_knowledge_at_anchor`.
- `crates/cli/src/backup.rs`: backup manifest v2 and compatibility checks.
- `crates/cli/tests/knowledge.rs`, `crates/cli/tests/mcp.rs`: persistence, migration, freshness, portability, isolation and parity regression coverage.

No source change claims a complete Rust semantic call graph, a positive impact result for the frozen repository, or truth verification of the linked knowledge record.

## Amendment 2026-10-31

- **Superseded: "Cross-platform CI — NOT RUN" (acceptance table, last row).** CI on master `86d6e42`, which contains this change, completed successfully (run [40900940173](https://github.com/RyanAraaful/riltra/actions/runs/40900940173)): the check jobs on Windows, Ubuntu and macOS, the security job, and the three installer Phase A jobs.
- **Superseded: "no `scip-rust`, `scip`, or `.scip` artifact" and "the Rust SCIP indexer … absent"** (decision 2 under Scope, and the provider-inventory row). rust-analyzer 1.99.1 includes the `scip` subcommand, so no separate indexer is required. The standalone `scip` CLI, used to inspect indexes, is still absent and was not needed.
- **Remaining work item 2 (Rust SCIP producer):** stage 1 is recorded in [`riltra-rust-semantic-feasibility-2026-10-31.md`](riltra-rust-semantic-feasibility-2026-10-31.md). On `86d6e42`, the producer output is reproducible on one host, Riltra imports it as `Unverified`, and 16 of 16 frozen oracle call expressions return; eight of the nine production call sites are `CALLS`. The item stays open until the proof contract is decided and cross-platform evidence exists.
- **Impact remains `UNKNOWN`** for the frozen repository on that pin. No positive affected set is claimed.
