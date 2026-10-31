# What, Impact, Why: partial correctness evaluation

Date: 2026-10-31. Status: **bounded correctness baseline, PARTIAL for the full evaluation plan**, not a product-wide PASS. Source HEAD: `1bb0f6ab6327ab5cb906c254983f2cea94e9f67f`. No production source or contract was changed. This report records actual Windows runs; it does not claim Linux/macOS or CI results.

## Independent oracle review

Before the current test outputs were inspected, an independent reviewer read the checked-in M6 Rust source and manifests: `direct` expects one inbound `CALLS` and `HIGH` in the synthetic fixture; `reference` and `alias` must not turn a reference into `CALLS`; `nested_anonymous` expects no inbound `CALLS` and `UNKNOWN`; same-name fixtures prohibit unsupported promotion. The reviewer also checked the impact-chain and missing-provider fixtures. The manifests and their zero-based byte coordinates are documented in [M6 fixtures](../crates/cli/tests/fixtures/m6/README.md).

**Gap found:** the Rust fixture named `dynamic_dispatch` uses a concrete `&Obj`, not `dyn Trait`. It is not evidence for trait-object dispatch. M6 uses synthetic SCIP, not output from a real producer. A separate independently reviewed `dyn Trait` oracle was subsequently run below.

## Existing contract tests on current source

The following commands ran offline with a new isolated `CARGO_TARGET_DIR`, compiling current workspace source with `--locked`:

| Command (all include `--locked --offline`) | Observed |
|---|---|
| `cargo test --test edges task7::` | 82 passed, 0 failed; includes multilingual corpus and three focused Rust impact tests. |
| `cargo test --test knowledge code_binding_cli_links_an_explicit_anchor_and_marks_changed_source_stale` | 1 passed, 0 failed. |
| `cargo test --test mcp knowledge_at_anchor_mcp_matches_cli_and_keeps_legacy_store_read_only` | 1 passed, 0 failed. |
| `cargo test --test mcp t3_impact_matches_cli_verdicts_cycles_and_paging` | 1 passed, 0 failed. |

These tests are evidence for their checked-in fixtures and assertions, not an audit of all cases in the plan. The initial mistaken command `cargo test --test m6_corpus --no-run` failed because there is no such target; `edges` is the documented corpus target. This setup error was corrected, not counted as a product failure.

## Real rust-analyzer: direct call and closure

An isolated copy of the existing [closure fixture](pilots/riltra-rust-semantic-feasibility-2026-10-31/diagnosis/closure-fixture/src/lib.rs) was used. Its manifest received an empty `[workspace]` table before the baseline manifest, because a first attempt without it failed when Cargo found the parent workspace. A lockfile was generated offline before the baseline. No fixture source was modified after that baseline. Producer output, target, Riltra home and logs were siblings of the source directory. The before/after source-manifest files have the same SHA-256 `2C5446995B914FC92C7F3411CF1F0F697F8E76735C5B18E051EE1263DDD0FCBF`. This is the hash of the local formatted manifest, not a portable source digest.

The Riltra binary compiled during the test build had SHA-256 `1381AFACF4593077A1DEF8ECF1BFCBDB906AD2574056D91CE56B8DB0820A7ECB`; rust-analyzer and rustc were version 1.99.1 (`58cf8f9c`), Cargo 1.99.1. `rust-analyzer --log-file <producer>/ra.log scip <source> --output <producer>/index.scip --exclude-vendored-libraries` ran with `CARGO_NET_OFFLINE=true` and a separate target directory; it completed in about 35 seconds. Provider execution was explicitly opted into for this trusted fixture. Import reported one document, seven definitions, 11 references, no skipped documents and `Unverified`. The provider's full log is local and was not independently audited for every diagnostic.

The oracle is the source itself: `direct_caller` calls `callee` directly; `closure_caller` contains the other invocation inside an anonymous closure. The accepted closure contract deliberately does not attribute the latter to the enclosing function as a definite caller.

| Query | Observed | Interpretation |
|---|---|---|
| Syntax `callers` | Two `POTENTIAL_CALL`/`Heuristic` edges, one with `owner-unproven` | Both are candidates; no semantic identity claimed. |
| SCIP `callers` | Direct site `CALLS`/`Exact`/`Unverified`; closure site `POTENTIAL_CALL`/`Heuristic`/`owner-unproven` | Matches this two-site oracle and current closure contract. |
| Syntax `impact` | `UNKNOWN`, no paths, partial, `empty-result-not-coverage-proof` | Empty does not imply safe. |
| SCIP `impact` | One direct `CALLS` path, `UNKNOWN`, partial | The path remains visible even though snapshot coverage does not support a positive verdict. |

Index status remained `fresh`; the archive-less fixture had no Git HEAD in its snapshot. The semantic import is `Unverified` and the static path does not prove execution. Results are restricted to this tiny fixture and this configuration.

## Real provider: trait-object dispatch

An independent reviewer specified [the source and oracle](pilots/wiy-correctness-2026-10-31/ORACLE.md) before inspecting outputs: `dispatch` calls a trait method on `&dyn Service`, whose runtime implementation cannot be chosen from the call site alone; `direct` uses `&A`. The independent oracle did not preassign a SCIP role or require a particular Riltra edge kind. The fixture was generated offline with the same producer and isolated target/home; source manifests matched before and after. The producer completed in about 3.5 seconds; import returned one document, 14 definitions, 11 references and `Unverified`.

At the trait method anchor, `callers` returned the dynamic call as `POTENTIAL_CALL`, `target-unproven`; impact was `UNKNOWN`, empty and partial. At the A implementation anchor, the direct call appeared as `CALLS`, `Exact`, `Unverified`, while the dynamic call stayed a candidate. These observations do not establish runtime execution, an exhaustive set of implementations, or general Rust trait coverage. The pre-import syntax query at the trait method returned `no anchor here`, so this case has no comparable syntax caller output at that position. [Trait callers](pilots/wiy-correctness-2026-10-31/results/trait-callers.json), [A callers](pilots/wiy-correctness-2026-10-31/results/trait-impl-a-query.txt), [impact](pilots/wiy-correctness-2026-10-31/results/trait-impact.json).

## End-to-end Why and MCP evidence

On the real-provider closure fixture, an explicitly authored evaluation decision was bound to `callee`, revision 1, with source evidence `src/lib.rs`. The result was `linked`, `fresh`, `Unverified`; querying the unbound `direct_caller` returned `no-link-proven`. An MCP client initialized the server and called `riltra_knowledge_at_anchor`, `riltra_code_impact`, and `riltra_code_callers`; each returned structured JSON equal to its CLI counterpart. The record describes this synthetic fixture, not a historical project rationale. [Why](pilots/wiy-correctness-2026-10-31/results/e2e-why.stdout.json), [unbound](pilots/wiy-correctness-2026-10-31/results/e2e-unbound.stdout.json), [MCP](pilots/wiy-correctness-2026-10-31/results/e2e-mcp.jsonl).

In a separate source copy and isolated home, a second record was bound to revision 1 and then updated to revision 2. The binding remained pinned to revision 1. After appending a comment to that isolated source copy, the binding reported `stale` and retained revision 1. The original fixture was not modified. [Before](pilots/wiy-correctness-2026-10-31/results/stale-before.stdout.json), [after](pilots/wiy-correctness-2026-10-31/results/stale-after.stdout.json), [source hashes](pilots/wiy-correctness-2026-10-31/results/stale-source-hashes.json).

## Remaining plan work

- Alias, collisions and non-call references were checked by the synthetic corpus, not the real-producer runs. A larger independent Rust oracle is needed before stating precision or recall for those cases.
- Capture complete per-case CLI/MCP outputs and verify all plan-specific coverage, including deterministic limit and missing-provider cases against the frozen oracle. The selected existing tests cover related cases but are not a substitute for that full run.
- Selected raw JSON evidence and a SHA-256 manifest are retained under [results](pilots/wiy-correctness-2026-10-31/results/); a simple drive/user-path scan found no match. Full logs and provider artifacts remain under ignored `target/wiy-eval-20261031/` and may include local paths. The retained fixture reproduces semantics but differs in line endings from the measured source. Independent reproduction and a complete security scan of the evidence package have not been performed.

No observed result in the exercised cases requires changing the contract. No claim is made that there are no correctness defects outside these cases. No manual deletion of existing user data was performed and no commit was made; automated test fixtures may manage their own temporary files.
