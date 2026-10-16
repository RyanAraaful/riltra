# M6 validation — real providers and final local checkpoint

Task 8 on branch `m6-task7-multilanguage-corpus`, based on accepted correction
`6e54d5e5579a6810b386063d0ee77925f94873ee`. The local DCO commit carrying this
report is the audit unit. Production Rust, schema v4, dependencies and existing
goldens are unchanged. Scope is the two real pilots, measurements and regression.

**M6 is not CLOSED.** Final Windows/Ubuntu/macOS CI is NOT TESTED. No push,
M7/M8 work, installation, binary download or package installation script was performed.

## Acceptance ledger

| Gate | Status | Evidence |
| --- | --- | --- |
| Task 1: full/terminal callee spans | PASS | Existing span/raw-byte regressions |
| Task 2: v4 migration/storage | PASS | v1/v2/v3 migration, fresh DB, NULL spans and promotion tests |
| Task 3: strict edges/lifecycle | PASS | Terminal overlap, target/owner proof, digest and competing-target regressions |
| Task 4: callers/callees | PASS | [Task 4 assertion map](m6-task4-validation.md), transaction/order/pagination tests |
| Task 5: bounded deterministic trace | PASS | [Task 5](m6-task5-validation.md), destination/cycles/mappings/bounds/concurrency |
| Task 6: conservative impact | PASS | [Task 6](m6-task6-validation.md), UNKNOWN-first, zero SAFE/LOW |
| Task 7: multilingual corpus | PASS | [Task 7](m6-task7-validation.md): 79 cases / 494 full responses, unchanged and rerun |
| Task 7: persisted proof compatibility | PASS | [Correction](m6-task7-correction-validation.md), upgrade and rollback regressions rerun |
| Task 8: real scip-typescript CALLS + negatives | PASS | Actual artifact, no syntax reindex, source assertions |
| Task 8: real rust-analyzer locations | PASS | Correct definition/references, zero persisted semantic identities/CALLS |
| Resource/repeated-parsing ledger | PASS | Actual small-fixture measurements; unmeasured fields explicit |
| Local Windows fmt/check/Clippy/test/build | PASS | All exit 0, 402 tests, zero failed/ignored |
| Independent read-only review | PASS | No critical/important findings; minor harness restoration finding fixed and reviewed |
| Final CI Windows/Ubuntu/macOS | NOT TESTED | Separate push permission required; earlier milestone CI does not certify M6 |

## Environment, licenses and exact reproduction

Measured 2026-10-16, Windows x64 10.0.26100, Intel Core i5-8250U @ 1.60 GHz,
8 logical CPUs, 21,353,435,136 bytes visible RAM. Hardware via Node `os`; CIM
was unavailable in the sandbox. Debug build, stable MSVC rustc 1.99.0
(`b940084d7`), cargo 1.99.0 (`5f94df478`). No release-build, large-repository
or competitive performance claim is made.

| Existing component | Version | License/provenance |
| --- | --- | --- |
| Node.js / npm | 22.23.3 / 10.9.9 | Existing runtime; MIT plus bundled notices |
| `@sourcegraph/scip-typescript` | 0.4.0 | Apache-2.0; installed package.json and LICENSE inspected |
| Its TypeScript runtime | 5.9.3 | Apache-2.0; installed dependency package.json inspected |
| rust-analyzer | 1.99.0 (`b940084d`, 2026-09-28) | Existing rustup component; upstream [Apache](https://github.com/rust-lang/rust-analyzer/blob/master/LICENSE-APACHE) / [MIT](https://github.com/rust-lang/rust-analyzer/blob/master/LICENSE-MIT) licenses |

Run sequentially from the repository root with existing PowerShell 7 and
Node 22. The runner uses experimental built-in Node SQLite, not an npm dependency:

```powershell
cargo build --locked -p riltra
node --version
npm --version
scip-typescript --version
rust-analyzer --version
node docs/pilots/m6-real-providers.cjs `
  C:/Users/ryan/AppData/Roaming/npm/node_modules/@sourcegraph/scip-typescript/dist/src/main.js `
  C:/Users/ryan/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/rust-analyzer.exe
# Use the directory printed by the preceding command:
node docs/pilots/m6-profile.cjs target/m6-pilot-An19ih
```

Supply existing absolute provider paths on another machine. The runner creates
unique fixture roots and RILTRA_HOME under gitignored `target/`, asserts results,
and records argv, exit codes, raw JSON, SQLite rows and metrics. Successful
measured run: `target/m6-pilot-An19ih/`. Selected portable raw results:
[m6-windows-evidence.json](pilots/m6-windows-evidence.json). Nonces/artifact IDs
change across runs. The actual external indexer command is:

```powershell
node <installed-scip-main.js> index --cwd <isolated-ts-code> --output index.scip --no-progress-bar
```

Final assertion replay also exited 0 in `target/m6-pilot-GqNqPb/`, including
the exact four expected CALLS, same-import target definitions and cross-file
terminal span checks added during review. It does not replace the measured
An19ih run. Native binary SHA-256 for these runs:
`5759bfeb624e246ce351ce99dfb7052114e51e28af713de48f0deaf88758e15b`.

Only plain single-project mode is used: no yarn/pnpm discovery or package scripts.
LSP runs solely via explicit `--lsp-server <absolute-executable>`. The harness sets
`CARGO_NET_OFFLINE=true`. Rust fixture has its own `[workspace]`, no dependency,
build script, proc macro or executable. No user source program is executed.
No upload command, hosted indexer or telemetry endpoint is configured. Network
traffic was not packet-captured. Terapetra Knowledge/GitNexus source was not used.

## TypeScript: source truth and real producer output

The runner contains the complete fixture: three TS files, 567 source bytes before
the edit. `lib.ts`: leaf, middle → leaf, non-call `return leaf`, parameter-shadowed
leaf(). `main.ts`: imported entry → middle, alias call, nested local function.
`other.ts`: unrelated same-name leaf and another local function. Source is not executed.

Syntax indexing precedes the unmodified real producer artifact. Semantic import
is the next writer, with no syntax reindex: snapshots and files tables remain
identical; nonce is unchanged; semantic generation becomes 1. First import:
27 facts, 12 edges = 4 CALLS + 2 POTENTIAL_CALL + 5 REFERENCES + 1 IMPORTS.
Two CALLS target document-scoped locals, with callee_doc equal to source document.
The real producer assigned distinct local URIs; duplicate-local-URI collision
isolation is additionally covered by deterministic tests, not claimed from this pilot.

Source checks: `lib.ts` bytes [87,91) are `leaf`, inside call [87,93) and named
owner middle. Terminal/reference spans agree. `main.ts` [79,85) is `middle`,
with target definition in lib.ts in the same import. `callers` finds both direct
relations; syntax-fallback `callees` and `trace` expose outbound CALLS. Example
excerpt (full schemas/provenance are in the evidence JSON):

```json
{"kind":"CALLS","precision":"Exact","callee_name":"leaf","verification":"Unverified","evidence":{"path":"lib.ts","span":[87,93],"callee_span":[87,91],"terminal_span":[87,91],"reference_span":[87,91],"digest":"fe5e52b5052a37fb","provider":"scip-import"}}
```

Evidence includes import_id 1, scip-typescript 0.4.0, artifact digest, source
revision/nonce and content identity. The bundle additionally records SHA-256 of
the original artifact. Existing Riltra digest formats remain unchanged. Every
Exact CALLS remains Unverified; source edit changes query verification to Stale.
Neither exact precision nor successful traversal establishes Current.

Negatives: non-call [138,142) remains REFERENCES; parameter and alias calls are
POTENTIAL_CALL with null targets; unrelated other.ts leaf has no incoming edges.
Impact remains UNKNOWN/partial. Five warm repetitions are structurally identical;
adjacent one-item pages match slices of the complete same-state result.

Producer boundary: scip-typescript 0.4.0 emits empty definition display names here.
Accepted proof cannot bridge these semantic owner identities to SyntaxDef.
Name-position semantic callees/trace can therefore be empty/partial despite a
syntax-owner CALLS. Column 8 on `function` explicitly resolves syntax-fallback,
Heuristic, anchor_uncertainty and no symbol_uri. Trace returns a plausible one-hop
entry → middle and never invents a bridge to continue to leaf. No proof rule was relaxed.

## Rust: ephemeral LSP location evidence

Dependency-free Rust library: 3 functions, 109 bytes:

```rust
pub fn leaf() -> i32 { 7 }
pub fn caller() -> i32 { leaf() }
pub fn reference_only() -> fn() -> i32 { leaf }
```

```powershell
riltra semantic query --repo rs --path src/lib.rs --line 2 --col 26 --lsp-server <absolute-rust-analyzer.exe>
riltra semantic query --repo rs --path src/lib.rs --line 1 --col 8 --refs --lsp-server <absolute-rust-analyzer.exe>
```

Both exit 0. Definition: raw [7,11), zero-based (0,7)-(0,11), exactly declaration
`leaf`. References include [52,56) at call, [102,106) at non-call, and declaration
because the adapter requests it. Every hit is sliced against source and must equal
leaf. Provider lsp-subprocess, symbol_uri null, Heuristic, Unverified; no fake SCIP
identity/import_id. All captured meta, snapshot, file, semantic-import/fact and
edge rows remain identical across live queries and unavailable-provider query.
Semantic imports/facts and persisted CALLS remain zero.

Missing absolute executable returns exit 0 with explicit fallback and no invented
locations; core syntax callees still works. Timeout, crash, malformed protocol,
flooding and bounded cleanup run through existing deterministic mocks. Failure
injection against the real provider was not separately performed. Real successful
queries still report the existing 15-second analysis-wait diagnostic; returned
locations do not certify analysis completeness. Nothing enters the persistent graph.

Initial fixture setup lacked its own [workspace] and used column 25 instead of 26.
Empty answers were rejected; fixture setup was corrected. These were harness
corrections, not production bugs. No Task 8 production RED→GREEN fix is claimed.

## Freshness, proof compatibility and atomicity

A real artifact truncated near its end fails import with exit 3 and leaves captured
database rows, graph marker and generation unchanged: no partial graph survives.
First proof token is `m6-task7-source-proof-1:<same-nonce>:1`. No-op index ingestion
reparsed=0; a single-file edit reparsed=1. Successful import/reimport does not mint
snapshot rows. Reimport advances generation to 2; duplicate import anchors fail
explicitly as ambiguous instead of merging identities.

Deleting the isolated marker blocks queries with graph-proof-incompatible.
Successful import revalidates syntax and all 3 imports, generation=3, without changing
source snapshot/files. Old-writer generation mismatch, all-four-query upgrade
rejection/recovery, late rollback, retained-connection rollback and concurrent
promotion are also rerun in correction tests. Those upgrade tests remain distinct
from the real-provider compatibility pilot.

## Resource and repeated parsing

[m6-measure.ps1](pilots/m6-measure.ps1) records direct-process launch-to-exit wall
time, including managed launch/wait overhead, UTF-8 stdout bytes, and best-effort
sampled PeakWorkingSet64. Short-lived process peaks can be missed: null means
not measured. Windows peak working set is an RSS analogue, not heap or aggregate
child-tree memory. rust-analyzer child RSS is not measured. Index/import rows are
single observations. Warm queries are 5 fresh processes with OS/SQLite caches warm,
not a persistent server/cache. External indexer time is separate from native import.

| Operation | elapsed_ms | Peak working set bytes | Output bytes | Visits nodes/edges |
| --- | ---: | ---: | ---: | --- |
| Full syntax index | 827.769 | 8,208,384 | 497 | not measured |
| No-op incremental | 421.524 | 8,568,832 | 496 | not measured |
| Single-file refresh | 467.307 | 8,577,024 | 497 | not measured |
| First SCIP import | 211.367 | 8,163,328 | 207 | not measured |
| Semantic reimport | 134.070 | 8,171,520 | 207 | not measured |
| Incompatible-proof revalidation | 163.205 | 8,306,688 | 207 | not measured |
| Warm callers, median | 42.309 | 7,258,112 max sample | 4,565 | not instrumented |
| Warm syntax callees, median | 44.591 | 7,499,776 max | 2,082 | not instrumented |
| Warm syntax trace, median | 44.522 | 7,659,520 max | 2,437 | 2 / 1 |
| Warm impact, median | 40.627 | 7,516,160 max | 5,565 | 1 / 4 |
| External scip-typescript | 1,174.617 | 144,465,920 | producer text in bundle | n/a |
| Opted-in LSP definition, end-to-end | 15,333.301 | 5,873,664 parent only | 555 | n/a |
| Opted-in LSP references, end-to-end | 15,354.082 | 5,795,840 parent only | 991 | n/a |

Warm ranges ms: callers 39.719–44.441, callees 37.886–50.042, trace 41.919–52.062,
impact 37.809–51.982. Three syntax files, 11 definitions, 6 syntax calls; first
import has 27 facts/12 edges. This small fixture does not justify throughput extrapolation.

[m6-profile.cjs](pilots/m6-profile.cjs) archives the baseline to target/, inserts
scope timers only in that copy, and builds in a separate Cargo target directory.
It instruments derive_syntax_edges, upsert_semantic_edges, rederive_after_carry,
source_proofs and each relationship_proof call. Same source/artifact, isolated
database. It temporarily restores original fixture source for the initial index,
then restores final bytes in finally. Run after native pilot exits. Production
source/binary remain unchanged; counters are not shipped.

Counts below are actual instrumented calls, not inference from reparsed. Proof
parse time includes Tree-sitter plus relationship-proof extraction, excluding
source IO/hash. Enclosing derivation times include nested work, SQL and logging
overhead. Inclusive columns must not be summed. Instrumented timing is distinct
from the native table above; no optimization was attempted.

Instrumented run: `target/m6-profile-1CYjk2/`, archived baseline above.

| Operation | Ingestion files parsed | Proof parses | Proof passes | Proof parse ms | Source-proof ms | Whole derivation ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| full | 3 | 3 | 1 | 0.983 | 2.148 | 3.533 |
| import | 0 | 3 | 1 | 1.250 | 2.226 | 4.431 |
| noop | 0 | 6 | 2 | 2.239 | 4.335 | 7.149 |
| delta | 1 | 6 | 2 | 1.816 | 3.869 | 6.232 |
| reimport | 0 | 3 | 1 | 1.227 | 2.216 | 4.109 |
| revalidate | 0 | 12 | 4 | 3.722 | 7.598 | 14.029 |

The four instrumented queries emit zero proof/derivation events: no query-time
Tree-sitter parse or graph derivation. Revalidation has three imports, hence
one syntax + three semantic passes across three files (12 actual proof parses).

Zero ingestion reparses still costs syntax-proof plus a pass per semantic import.
Incompatible certification revalidates all imports. This measures actual repeated
work without claiming large-repository scalability.

## Regression, review and remaining gates

All local commands exit 0:

```powershell
cargo fmt --all -- --check
cargo check --locked
cargo clippy --all-targets -- -D warnings
cargo test --locked
cargo build --locked -p riltra
```

402 tests: 201 unit + 16 CLI + 137 edges + 21 index + 15 semantic + 12 syntax;
zero failed/ignored. Existing goldens unchanged. Some platform/provider-gated test
bodies return early; passing counts alone do not prove those paths ran. Actual
external-provider evidence above is separate. No new Rust test was necessary
because Task 8 changes no production behavior; manual pilot assertions cover its gaps.

All 79 multilingual cases/494 responses and mandatory zero-false-safe assertions
rerun: never SAFE/LOW, clean leaves and insufficient evidence UNKNOWN, traversal
completion never graph completeness. Existing suite also checks alias/parameter/
anonymous/competing targets, CALLS vs REFERENCES/POTENTIAL_CALL, strict endpoint
and import/document/repository scope, unresolved endpoints, full/delta parity,
no-reindex import, atomicity/concurrent promotion, stale verification, pagination
and actual query read transactions. Linked checkpoint reports retain test-name
maps and historical RED/GREEN. Task 8 makes no production bug-fix RED/GREEN claim.

Independent read-only review checked raw pilot outputs against the portable bundle,
source spans and negatives, unchanged source snapshot/SQLite state, proof lifecycle,
measurement counters, 402-test totals and 79-case/494-response corpus inventory.
It found no critical/important issues. A minor harness restoration concern was
fixed: source mutation is now inside try/finally, with completed-target-fixture
input confinement. Wrong-path rejection exits 1 before mutation. The reviewer
did not rerun providers, measure large-repository scalability, inject real-provider
failures, capture network traffic or certify Linux/macOS. Those remain outside
the review's evidence; final cross-platform CI remains NOT TESTED.

Known limitations: incomplete semantic coverage and unsupported/provider gaps;
empty producer display names preventing owner bridges; append-only reimports causing
explicit ambiguity; plausible static trace never runtime proof; cooperative 2000 ms
traversal deadline rather than a hard whole-command limit. Walks, verification,
SQL and subprocesses have separate costs/bounds. Repeated parsing grows with files
× import passes. Windows symlink bodies may require privileges, and local Windows
evidence does not establish Linux/macOS behavior.

Recommendation: proceed to separately authorized final cross-platform validation
after Task 8 audit. M6 remains open. STOP here; no push or new milestone.
