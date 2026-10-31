# What, Impact and Why follow-up

Run date: 2026-10-29. Implementation parent: `7238da847aa6b48cef73d721299f953ca533925a`; the measured source patch is [`raw/source.diff`](pilots/riltra-wiy-followup-2026-10-29/raw/source.diff). The source fixture is the frozen `confined_read` oracle at baseline `2852ddb4658afdfdcd0286f9a5d1a47f4f5bb87d`. Original evidence is unchanged. Shared output copies have normalized line endings and redacted roots; trailing blank lines were trimmed from the test log. [`redaction-accounting.json`](pilots/riltra-wiy-followup-2026-10-29/raw/redaction-accounting.json) records hashes of original local captures and shared copies, and [`MANIFEST.sha256`](pilots/riltra-wiy-followup-2026-10-29/raw/MANIFEST.sha256) hashes the shared copies.

## What

An inspection of the full import found that a production invocation had SCIP occurrences for the `crate` and `semantic` prefixes and for the `confined_read` terminal. Previously the first two each generated a `target-mismatch` candidate for the same call. The derivation now keeps a prefix occurrence as a `REFERENCES` edge when the same call has a terminal occurrence. A suffix occurrence, such as a C++ template type argument, retains its previous unresolved-candidate behavior. This does not promote references into calls.

On the isolated full-import replay, all 13 oracle invocations map to exactly one result each: 10 `CALLS` with invocation spans and three `REFERENCES` with identifier spans inside the corresponding invocation. There are no unmatched edges in this specific oracle query; the 12 former duplicate candidates are absent. CLI and MCP callers output matched. [`analysis-final.json`](pilots/riltra-wiy-followup-2026-10-29/raw/analysis-final.json) contains the per-site accounting. The provider remains `Unverified`; this result does not establish repository-wide recall or runtime dispatch.

The regression `receiver_references_do_not_duplicate_a_resolved_call` failed before the change with `(calls, potential, references) = (1, 1, 0)` and passed afterward with `(1, 0, 1)`. The existing `cli_scip_template_call_proves_the_function_not_the_type_argument` test initially exposed a too-broad suppression; the final implementation retains that test's expected unresolved suffix candidate.

## Impact and provider

`rust-analyzer 1.99.1 (58cf8f9c 2026-10-13)` was rerun offline against the same source with an isolated Cargo target and temporary directory, outside the restricted sandbox. `rust-analyzer scip <snapshot> --output <artifact> --num-threads 2` exited 0 in 45.91 seconds. The artifact SHA-256 is `f8823e3cdcdccf6849bc8dadd8588ff3c4540f5a6dc5d09dbaf95e46104b9e5d`, identical to the earlier artifact. Its 10,380,540 bytes remain in the local isolated run and are not included in the shared evidence directory.

The new producer stderr contained no permission-denied lines but did contain 171 errors about an enclosing definition without a name, two errors saying definitions should have been present in a SCIP document, and 93 duplicate-symbol diagnostics. A successful process exit and matching digest therefore do not prove complete or correct provider coverage. The full import returned 34 documents, 16,922 definitions, 98,393 references, zero skips and `Unverified`, in 156.47 seconds. See `raw/provider.*` and `raw/import-final.*`.

`impact --repo riltra-e2-confined --path crates/cli/src/semantic.rs --line 276 --col 8 --limit 1000` returned 32 paths, including 13 direct paths, but verdict **UNKNOWN**, partial completeness and a time-budget cutoff. Reported unresolved owners, targets, provider files and other coverage gaps remain. CLI and MCP impact responses matched in this run; paths beyond the previously checked six production sites were not independently validated. An empty or partial set is not evidence of safety.

## Why

The [manual review procedure](riltra-why-review-workflow.md) uses a separate review record and an explicit, revision-pinned `relates_to` link. A synthetic reviewer created an inconclusive review of `e2-codex-confined-read-pr14` revision 1 in an isolated copy of the store; CLI/MCP related output matched and the review remained `candidate` / `Unverified`. See `raw/review-*`. This verifies the recording and read path only. The engine does not authenticate the reviewer, enforce review fields, establish truth or derive a verified state, so content verification remains **UNMET**.

## Verification and remaining work

- `cargo test --locked` exited 0 outside the restricted Windows sandbox: 810 passed across eight groups, including the hard-link and junction tests. `raw/cargo-test.*` preserves the output.
- `cargo clippy --locked --all-targets -- -D warnings` and `cargo fmt --all -- --check` exited 0. Cargo reported Windows incremental-cache hard-link warnings and copied files instead; those warnings are retained.
- This dirty follow-up patch has not yet been validated by cross-platform CI. The earlier successful CI run applies to parent `7238da8`, not this patch.
- Review identity, structured review validation and an auditable first-class review state remain future work. Continue to display `UNKNOWN` and `Unverified` where their evidence thresholds are unmet.

## Amendment 2026-10-31

- **Superseded: "not yet validated by cross-platform CI" (Verification, second bullet).** The follow-up patch is now on master; its regression test is `receiver_references_do_not_duplicate_a_resolved_call` in `crates/cli/src/edges.rs`. CI run [40900940173](https://github.com/RyanAraaful/riltra/actions/runs/40900940173) on master `86d6e42` completed successfully: the check jobs on Windows, Ubuntu and macOS, the security job, and the three installer Phase A jobs.
- **Figures in "The new producer stderr" paragraph** (171 enclosing-definition errors, 93 duplicate-symbol diagnostics) describe the earlier source revision. The 2026-10-31 producer run on `86d6e42` reports 177 and 93. See [`riltra-rust-semantic-feasibility-2026-10-31.md`](riltra-rust-semantic-feasibility-2026-10-31.md).
