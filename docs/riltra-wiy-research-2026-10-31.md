# What, Impact, Why: research and current evidence

Date: 2026-10-31. Status: research note, not a change to product contracts or historical acceptance. Local repository HEAD inspected: `1bb0f6ab6327ab5cb906c254983f2cea94e9f67f`. Earlier measured runs are pinned separately below. The original product blueprint was not available for this review; the local, ignored `docs/riltra-guide/` was consulted as a draft, not treated as an approved replacement for existing contracts.

## Question and correction

How should Riltra answer what code does, what a change may affect, and why a decision was made, while preserving evidence? An earlier recommendation suggested making the Rust SCIP workflow easier before establishing whether that addresses the material failure. That is a hypothesis, not a demonstrated fix. In particular, SCIP can improve symbol identity and selected call evidence without making impact complete.

## What: syntax, symbols and calls

**External evidence.** Sourcegraph distinguishes search-based navigation, using text and syntax heuristics, from opt-in precise navigation using language-specific SCIP indexes ([navigation](https://sourcegraph.com/docs/code-navigation), [precise navigation](https://sourcegraph.com/docs/code-navigation/precise-code-navigation)). SCIP describes symbol occurrences and relationships for code navigation; an occurrence alone is not proof of a call from a particular owner ([SCIP schema](https://github.com/scip-code/scip/blob/main/docs/scip.md)). LSP defines requests such as definition, references, and call hierarchy, rather than guaranteeing a complete call graph for every server ([LSP specification](https://github.com/microsoft/language-server-protocol/blob/gh-pages/_specifications/lsp/3.18/specification.md)).

**Riltra evidence.** In the Rust SCIP run pinned at `86d6e42`, 16 of 16 expressions in the `confined_read` oracle were returned. Eight of eight production call sites outside anonymous scope were `CALLS`; the ninth production site, inside a closure, was `POTENTIAL_CALL`/`owner-unproven`. Three test expressions inside macros were `REFERENCES`. These counts describe one frozen oracle, not repository-wide recall. `Exact` and `Unverified` describe different properties; an imported edge can have both. See [Rust feasibility](riltra-rust-semantic-feasibility-2026-10-31.md).

**Inference.** Preserve the existing distinctions among `POTENTIAL_CALL`, `REFERENCES` and evidence-backed `CALLS`, and test their actual derivation against independently reviewed source examples before proposing implementation changes. Owner decision D-4 keeps the closure rule; this note does not reopen it.

## Impact: observed paths and coverage

**External evidence.** CodeQL's Java call-graph documentation distinguishes a statically resolved callee from an overriding method potentially invoked at runtime; this illustrates a general distinction, not a claim about a Rust implementation ([CodeQL](https://codeql.github.com/docs/codeql-language-guides/navigating-the-call-graph/)). Rust trait objects use dynamic dispatch based on the object at runtime ([Rust reference](https://doc.rust-lang.org/reference/types/trait-object.html)).

**Riltra evidence.** On the pinned Rust SCIP run above, impact stayed `UNKNOWN` and partial, with a time-budget cutoff and unresolved owner, despite observed semantic call paths. The current implementation also applies snapshot-wide coverage gates. Its verdict is about static evidence, not execution, severity or safety ([impact evidence design](riltra-impact-evidence-design-2026-10-31.md)). A regression fixture shows an observed exact path alongside `UNKNOWN` when another caller lacks provider coverage; it does not prove complete producer coverage.

**Inference.** A useful presentation must retain both observed paths and their labels, and the reasons coverage is incomplete. Whether a narrower completeness scope could be correct is an open research and contract question; neither these sources nor the existing experiment prove it safe. No new positive verdict or schema change is authorized by this note.

## Why: decisions and source evidence

**External evidence.** ADR guidance records decision context, the decision and consequences, and recommends versioning decisions near the code ([Google Cloud ADR guidance](https://docs.cloud.google.com/architecture/architecture-decision-records)); [adr-tools](https://github.com/npryce/adr-tools) supports explicit decision records and supersession. These are examples of recording decisions, not evidence that a record is true or that a link proves its rationale.

**Riltra evidence.** Explicit bindings join a code anchor to a record revision; `knowledge at` and MCP can read them. A binding is an asserted relationship, not a factual verification. See [Why review workflow](riltra-why-review-workflow.md) and [binding compatibility](riltra-binding-compatibility.md).

**Inference.** Validate retrieval of an explicitly authored, source-backed record, its revision and freshness; do not infer a decision from a name or claim that `Unverified` content is authenticated.

## Local exploratory run, not a correctness benchmark

An agent ran CLI and selected MCP queries on Windows using a pre-existing `riltra 0.1.0` binary with SHA-256 `FB1616674CFA1F40079EA7DE5F1D5540BBE2987AEDBFC43451BFD5CA4173C987`; its equivalence to HEAD was not established. A `git archive` selection containing `crates`, Cargo files and README (314 files, not the complete repository and without `.git`) was indexed in a source directory separate from the home and output. Its source file hash manifests matched before and after queries; index status was `fresh` after CLI and MCP. Indexing took about 36 seconds on that host. A full-repository exploratory run was stale with `file-added`; the precise triggering file was not identified, so it is not fresh evidence.

For `crates/cli/src/semantic.rs:276:8`, the selected snapshot returned 13 syntax-only candidate callers, and `impact` returned `UNKNOWN`, no paths and partial coverage. A historical record was explicitly recreated and bound, then returned as `fresh`/`Unverified`; it was not independently authenticated. CLI and MCP were exercised for the Why and impact queries, but no independent full parity audit or comparative agent benchmark was run. Local raw outputs and manifests reside under the ignored `target/wiy-clean-644355b4431c4040ac0cfe79cdc7aaf3/output/`; this path is ephemeral and not a portable archived evidence package. No provider was run in this exploratory trial. No source, profile, or other data was deleted.

## Provider and platform constraints

The prior SCIP producer run was opt-in, on Windows, and its logs showed build scripts executing. The producer reported diagnostics even though it exited successfully; its diagnostics are not preserved by the importer. rust-analyzer documents build-script and procedural-macro configuration ([configuration](https://rust-analyzer.github.io/book/configuration)). Disabling them may affect results and has not been tested for this oracle. Do not assume disabling one setting makes every provider action safe or network-free.

Development and initial evaluation are on Windows. Linux and macOS validation, when relevant and authorized, is through GitHub CI. This research changes no existing release, trust, closure or reliability decision.

## Next work

The executable evaluation plan is [What, Impact, Why correctness plan](riltra-wiy-correctness-plan-2026-10-31.md). It separates baseline evidence from future findings and requires recording failures and limitations rather than promoting an inconclusive run to PASS.
