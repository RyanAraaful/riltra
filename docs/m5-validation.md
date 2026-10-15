# M5 validation

## Gates (HEAD at close-out)

- `cargo fmt --all -- --check`, `cargo check --locked`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test --locked`:
  green locally (156 + 16 + 21 + 8 + 12 = 213 tests).
- Core-only passes with zero external providers (LSP live test skips
  silently without `RILTRA_TEST_LSP`; mock-server tests spawn a local
  binary, no network).
- M0–M4 regression: full suite green, including all M4 parity tests.
- M4 incremental parity: MMSP no-op `incremental/11765/0` in 15.9 s
  (M4: ~15.5 s; delta is noise). One 205 s outlier during validation was
  contention with an orphaned overlapping run, not a regression (clean
  re-run: 15.9 s). MMSP tree porcelain 0 throughout.

## Dependency audit (Task 1, 2026-10-16)

| Crate | Version | License (manifest) | Role |
|---|---|---|---|
| serde_json | 1.0.154 | MIT OR Apache-2.0 | LSP JSON-RPC parse/emit |
| serde_core | 1.0.229 | MIT OR Apache-2.0 | transitive via serde_json |
| protobuf | 3.7.2 | MIT | SCIP wire reader + generated bindings |
| protobuf-support | 3.7.2 | MIT | transitive via protobuf |

Rejected: prost/prost-build (needs protoc at build time; CI 3-OS must not
download toolchains). protobuf-codegen is a developer-side tool only
(bindings checked in under `crates/cli/src/scip_proto.rs` from
`proto/scip.proto`, the exact upstream copy), never a build dependency.
No async/network/loader crates in the tree (pinned by
`new_dependencies_are_audited`: versions from Cargo.lock, licenses from
`cargo tree` manifest data — never claimed from Cargo.lock).

## Tier A (deterministic)

- Contract: traversal/absolute/drive-qualified/symlink/missing paths
  rejected; backslash deviation normalizes (`win\d.py` → `win/d.py`);
  range disorder rejected, empty allowed; fallback JSON exact keys;
  capability registry resolves both providers, rejects unknown;
  same-name-different-provider never merges.
- Migration: v1→v3 (nonce backfill + tables), v2→v3 (data intact),
  mid-migration failure rolls back (version + reads intact).
- SCIP: valid counts + snapshot binding (no minted snapshot); truncated
  (3 cut points) → clean rollback; traversal → no write; metadata-not-first
  rejected; conflicting duplicates rejected; UTF-8/16/32 + 🚀 byte-exact;
  on-disk UTF-16 decodes (digest covers raw bytes, like M4); unindexed doc
  → NULL file_id, still queryable; full role bitset round-trips
  (`0x2|0x8|0x10|0x20|0x40` preserved, def/ref are query bitmasks);
  external metadata creates zero facts, referencing occurrences marked
  external + Unverified; packed 3/4 ranges decode, bad shapes skip;
  multi-line-typed single-line accepted; empty ranges stored;
  relationships verbatim (is_reference/is_implementation recorded,
  untraversed); Unspecified encoding exact on ASCII lines, non-ASCII
  skipped; over-cap aborts mid-stream (`bytes consumed < total` proven).
- LSP (mock server, 9 scenarios): success def/ref + shutdown/exit log
  receipts; hang → timeout + kill + bounded reap + dead session;
  crash/malformed/pollution → explicit broken (no resync); 2 MB
  stderr flood → no deadlock + tail captured; `$​/progress` consumed,
  unknown `$/weird` recorded; capability-less server → `unsupported`;
  cancel payload shape pinned (wire receipt unobservable against a hung
  server by construction — documented, not tested).
- Lifecycle (CLI): malformed import → exit 3, snapshot + generation
  untouched; cross-file def→ref both directions; local symbols never cross
  documents; empty positions → exit 3; import→noop (stays Unverified)→delta
  (whole import Stale)→full (Stale-marked, queryable); missing server →
  fallback JSON exit 0 with syntax search intact; repo isolation.
- Registry/binding: noop-identical → Unverified; same-git-fingerprint
  different-bytes → Stale; config drift → Stale; dirty drift → Stale;
  attested-yet-matching → still Unverified (claim ≠ proof; `Current`
  unreachable in M5 by rule).
- Concurrency: deterministic promote-vs-import race both orders
  (`semantic-changed` abort, committed import intact; post-import staging
  promotes and carries); WAL reader stable across semantic commit.
- Query contract: narrowest enclosing occurrence wins; no identity →
  `unresolved`; `--refs-depth` and unknown flags rejected (M6 boundary).

## Tier B (real providers, paired, no contention)

Providers installed explicitly and legitimately for this gate:
rust-analyzer 1.99.0 via `rustup component add`; scip-typescript 0.4.0
via `npm i -g` (fixture typescript 5.6.3 via local `npm install`).

### B1 — rust-analyzer (LSP) on a 12-line Rust fixture crate

- Definition at call site (main.rs line 6) → `(0,3)-(0,8)` bytes 3..8:
  exactly `greet` in `fn greet(name: &str) -> String {`. Verified by eye
  against source.
- References at the definition → 3 rows: both call sites
  (`greet("world")` bytes 96..101, `greet("mars")` bytes 155..160) plus
  the declaration (includeDeclaration=true — expected, not a false
  positive). Verified by eye.
- Missing coverage: none on this fixture. False positives: none.
- Runtime: 15.8 s / 15.7 s def/ref (cold spawn + crate load + bounded
  retries dominate; one-shot CLI pays full server startup per query).
  riltra peak RSS 4.8–5.5 MB (server process extra, not measured).
  Verification `Unverified` throughout (M5 rule).
- Real bugs found and fixed by this gate: `file://` URIs built from
  `\\?\`-extended roots rejected ("url is not a file"); hardcoded
  `languageId: "plaintext"` yielded empty answers; cold-server empty and
  `content-modified` (-32801) answers required bounded retry; lowercase
  drive letters needed URI normalization.

### B2 — scip-typescript 0.4.0 (SCIP) on a 2-file TS fixture

- Producer run: 1.1 s, index.scip 1429 bytes. Import: 0.3 s, peak RSS
  5.5 MB, 2 documents / 5 definitions / 7 references, 0 skipped,
  `fresh` syntax snapshot untouched, verification `Unverified`.
- Definition at `greet("world")` (main.ts line 2) →
  `scip-typescript npm tierb-ts 1.0.0 src/greet.ts/greet().` at
  (0,16)-(0,21): exactly `greet` in `export function greet(`. Verified.
- References at the definition → import binding `(0,9)-(0,14)` and call
  site `(2,24)-(2,29)`. Verified.
- Missing coverage: none on this fixture. False positives: none.
- Real producer deviations handled (all recorded, none silent):
  `file://` project_root, `\` separators (normalized), omitted
  `position_encoding` (ASCII-exact rule), deprecated packed `range`
  (decoded), empty `display_name` (stored verbatim).

### Tier B verdict

PASS: two languages, two providers, definition + references proven by
source inspection (not counts), costs recorded. No speedup or coverage
claim beyond these fixtures.

## Trust boundary (LSP subprocess)

Language servers are untrusted: they may run project hooks, build
scripts, or network access. Mitigations: explicit absolute `--lsp-server`
opt-in only (no auto-install/download, never spawned by syntax paths),
no shell (argv directly), 30 s default timeout, 8 MB message cap, 10k
message cap, stderr drained off-protocol with a 256 KB tail, kill +
bounded reap on every failure path, URI containment on every location,
`RILTRA_LSP_DEBUG=1` prints raw responses for audits.

## Limitations / not proven

- `Current` unreachable in M5 by rule (no independent producer-to-source
  proof exists); everything rests at `Unverified`/`Stale`.
- `--source-digest` is a recorded user claim, never proof.
- Unspecified position encoding + non-ASCII lines → document skipped
  (explicit reason); backslash paths normalized (documented deviation).
- Per-query cold server startup (~15 s rust-analyzer) + bounded retries:
  one-shot CLI pays this every time; no daemon in M5 by design.
- Cancel-request wire receipt unobservable against a hung server
  (payload shape pinned by unit test instead).
- `semantic_generation` coordination assumes SQLite writer serialization
  + the promote assert; cross-process lock files deliberately not added.
- Tier B fixtures are small on purpose (correctness over scale); MMSP +
  SCIP at scale explicitly out of scope for M5.
- scip-typescript Tier B requires network install (pinned 0.4.0);
  core CI never needs it.
- Deferred: independent attestation (for `Current`), incremental
  semantic graph, traversal/callers (M6), streaming field-visitor beyond
  document granularity (current: per-document streaming with byte/record/
  nesting budgets + abort-before-EOF proof).

## M5/M6 boundary

`semantic query` is single-hop def/ref only. No callers/callees, no
transitive closure, no impact analysis. `--refs-depth` does not exist and
is rejected as a usage error (pinned by test).
