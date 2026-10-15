# M5 Semantic Provider Protocol (LSP + SCIP) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Import verifiable semantic evidence from LSP servers and SCIP indexes into the existing SQLite store, served through three minimal CLI commands, with syntax-only M2–M4 paths untouched.

**Architecture:** New `semantic.rs` contract module (shared provenance/validation/registry) consumed by two provider adapters (`scip.rs` importer, `lsp.rs` subprocess adapter); two new snapshot-scoped tables via transactional v2→v3 migration; semantic writes reuse the M4 staging/promotion path; single-hop def/ref queries only (traversal is M6).

**Tech Stack:** Rust, rusqlite 0.40 (bundled), serde + serde_json (new, MIT/Apache-2.0), one pure-Rust protobuf crate (chosen in Task 1, no protoc download), tree-sitter stack unchanged.

**Spec:** GOAL M5 prompt (this session) + canonical `scip.proto` (sourcegraph/scip, fetched 2026-10-16) + LSP 3.17 framing/lifecycle.

## Global Constraints

- Correctness → Security → DRY/KISS → Performance, in that order.
- No new database/graph engine; no daemon/watcher/scheduler/background service/network API; no dynamic plugin framework.
- Syntax-only indexing stays the default; no mandatory language server or SCIP producer.
- Never equate unresolved syntax call candidates with resolved semantic references; never promote incomplete facts to exact; never mix evidence across source revisions.
- No path:line or bare function name as stable global symbol ID; same name from two providers is never assumed identical.
- No claim of `current` without a proven producer→source-revision binding; otherwise `unverified`/`stale` explicitly.
- std-only CLI stays; new deps only with license audit and proven need (serde/serde_json + one protobuf crate).
- Commit author email `terapetralabs@gmail.com`; no push without approval; M5 scope ends before callers/callees/trace (M6).
- CI 3-OS green, core-only passes with zero external providers; fmt/check/clippy `-D warnings`/test green; no freshness/security/M4-parity regression.

## Pinned spec values (implementers: use exactly these)

- SCIP `SymbolRole` bits: Definition `0x1`, Import `0x2`, WriteAccess `0x4`, ReadAccess `0x8`, Generated `0x10`, Test `0x20`, ForwardDefinition `0x40`. Roles field is a bitset.
- SCIP `PositionEncoding`: Unspecified `0` (reject for new data), UTF8 `1`, UTF16 `2`, UTF32 `3`. `TextEncoding`: Unspecified `0`, UTF8 `1`, UTF16 `2` (no UTF32 variant exists).
- Occurrence range: `typed_range` oneof (`single_line_range` field 8 / `multi_line_range` field 9) takes precedence when both forms present; deprecated packed `range` field 1 (`[line, char, endChar]` or `[line, char, endLine, endChar]`) is the fallback. All lines/characters 0-based, half-open `[start, end)`. Consumers MUST accept multi-line encoding even when the range fits one line.
- Symbol grammar: `<scheme> ' ' <package> ' ' <descriptor>+` or `'local ' <local-id>`; package = `<manager> <name> <version>` with `'.'` as the empty placeholder. `local` symbols are document-scoped and MUST never cross documents.
- `Document.relative_path` rules: relative to `Metadata.project_root` (URI-encoded absolute), no leading `/`, no empty/`'.'`/`'..'` components, `/` separators even on Windows, regular file never symlink. Index stream: `metadata` first, exactly once; documents/external_symbols any order.
- `Relationship`: `is_reference` (find-refs closure), `is_implementation`, `is_type_definition`, `is_definition` (definition override). M5 stores them; M6 traverses them.
- LSP: `Content-Length: N\r\n\r\n` framing on stdout; lifecycle `initialize` → `initialized` → requests → `shutdown` → `exit`; default position encoding UTF-16 unless negotiated otherwise; stderr is diagnostics only, never protocol.

## File structure

- Modify `crates/cli/Cargo.toml`: add serde + serde_json + one protobuf crate (Task 1 decision).
- Modify `crates/cli/src/store.rs`: `SCHEMA_VERSION 2 → 3`, v2→v3 migration, `semantic_imports`/`semantic_facts` helpers.
- Create `crates/cli/src/semantic.rs`: evidence contract (provenance, precision, verification, roles), shared path+range validator, static capability registry, fallback reporting.
- Create `crates/cli/src/scip.rs`: bounded SCIP importer producing `semantic::` facts.
- Create `crates/cli/src/lsp.rs`: std-only subprocess adapter producing `semantic::` facts.
- Modify `crates/cli/src/args.rs` + `main.rs`: `semantic import|status|query`, usage text, JSON output.
- Tests: unit tests inline per module + `crates/cli/tests/semantic.rs` (Tier A integration) + checked-in fixtures under `crates/cli/tests/fixtures/scip/` (+ `crates/cli/src/scip.proto` copy or `proto/scip.proto` with version pin note).
- Docs: `docs/m5-validation.md` (Task 10).

## Review Focus

- A 400 MB `index.scip` from an untrusted producer must abort mid-stream on caps (`bytes_consumed < file_len`), never OOM the importer — Task 4 pins byte/record/nesting/RSS caps plus the abort-before-EOF proof test.
- A UTF-16 SCIP document containing an emoji before a symbol on the same line must resolve to the exact byte span, not an off-by-N span — Task 4 pins the 🚀-style position test, separate from the on-disk UTF-16 text-encoding test.
- A language server that prints log lines to stdout must fail the request loudly, and a server flooding stderr must not deadlock the response pump — Task 5 pins rejection + the stderr-flood test.
- A SCIP file whose `project_root` is `/` with `relative_path` `../../etc/x` must be rejected before any DB write — Task 4 pins traversal tests.
- A `semantic query` after the user edits one source file must report `Stale`/`Unverified`, never `Current` — Task 6 pins the tripartite-binding test (identical artifact, moved source evidence).
- An `index` rebuild (incremental or full) must never attach pre-rebuild facts to the new snapshot id except through digest-verified carry-over — Task 7 pins the import→noop→delta→full→query lifecycle test.
- A `semantic import` committed while a staging build is in flight must survive promotion — Task 7 pins the deterministic race test (`semantic-changed` abort, no silent wipe).
- A `semantic query` for references must return occurrences in other files sharing the resolved identity, and must never merge same-name symbols across providers — Task 7 pins the cross-file def/ref tests.

---

### Task 1: Dependency selection + license audit

**Files:**
- Modify: `crates/cli/Cargo.toml`
- Test: `crates/cli/tests/semantic.rs` (license-audit test asserting exact crate names/versions against `Cargo.lock`)

**Interfaces:**
- Consumes: nothing.
- Produces: `serde`/`serde_json` for `lsp.rs`; one protobuf runtime + codegen path for `scip.rs`; `cargo tree` audit baseline appended to plan evidence.

- [ ] **Step 1: Evaluate the protobuf crate and record the decision.**

  Candidates: rust-protobuf `protobuf` + `protobuf-codegen` (pure Rust, no protoc binary, BSD-3-Clause) vs `prost` + `prost-build` (needs protoc at build time — rejected unless vendored, because CI 3-OS must not download toolchains). Default to rust-protobuf unless its streaming/limit API cannot express the Task 4 caps; if it cannot, document why and pick the minimal alternative. `serde` + `serde_json` (both MIT/Apache-2.0) are approved for LSP JSON-RPC: hand-rolled JSON parsing of server responses is banned. No other new deps.

- [ ] **Step 2: Write the failing license-audit test** in `crates/cli/tests/semantic.rs`:

```rust
#[test]
fn new_dependencies_are_audited() {
    // Versions come from Cargo.lock ONLY (name + exact version).
    // Licenses come from `cargo metadata` / crate manifests ONLY
    // (registry src under CARGO_HOME) — never claim a license came
    // from Cargo.lock; it records no license fields. Assert each new
    // crate's SPDX expression is in the allowlist
    // (MIT/Apache-2.0/BSD-3-Clause) and assert no `protoc`, `tokio`,
    // `async-*`, `libloading`, or socket/network crates entered the tree.
}
```

- [ ] **Step 3: Run it, verify it fails** (`cargo test --locked --test semantic new_dependencies_are_audited`, FAIL: file/dep missing).
- [ ] **Step 4: Add the deps to `crates/cli/Cargo.toml`, run `cargo update -p <new>` + `cargo tree`, paste the audit (name, version, license) into the test comment and `docs/m5-validation.md` stub.**
- [ ] **Step 5: Verify green** (`cargo test --locked --test semantic`, `cargo clippy --all-targets -- -D warnings`).
- [ ] **Step 6: Commit** (`git add crates/cli/Cargo.toml Cargo.lock crates/cli/tests/semantic.rs`, `git commit -s -m "deps: audited serde/serde_json/protobuf for M5"`).

### Task 2: Schema v3 migration (minimal, transactional, regression-tested)

**Files:**
- Modify: `crates/cli/src/store.rs:11-15` (`SCHEMA_VERSION`), `migrate()` at `store.rs:149-170`, new row structs + insert/query helpers beside `SymbolRow`.
- Test: inline `store.rs` tests + v2 fixture DB bytes checked into `crates/cli/tests/fixtures/db-v2-baseline.db` (built by a throwaway script, reviewed, then frozen).

**Interfaces:**
- Consumes: `content_digest`, `Snapshot.nonce`, `FileRow.id`.
- Produces:
  - `pub struct SemanticImport { pub snapshot_id: i64, pub provider_name: String, pub provider_version: String, pub capability: String, pub project_root: String, pub text_encoding: String, pub artifact_id: String, pub source_revision: String, pub claimed_source_digest: String, pub content_identity: String, pub external_symbols: String, pub verification: String }` (`claimed_source_digest` = user-supplied `--source-digest` claim, recorded verbatim, never trusted (Task 6). `content_identity` = digest over sorted `(path, file_digest)` pairs bound at import. `external_symbols` = verbatim JSON array of `{symbol, display_name}` from SCIP `external_symbols` metadata — never occurrences, locations, or relationships.)
  - `pub struct SemanticFact { pub snapshot_id: i64, pub import_id: i64, pub file_id: Option<i64>, pub rel_path: String, pub file_digest: String, pub symbol_uri: Option<String>, pub display_name: Option<String>, pub roles: u32, pub start_line: u32, pub start_col: u32, pub end_line: u32, pub end_col: u32, pub byte_start: usize, pub byte_end: usize, pub precision: String, pub relationships: String, pub external: bool }` (`roles` = full SCIP role bitset verbatim (`0x1`…`0x40`); definition/reference filtering is a query-time bitmask so import/read/write/generated/test/forward-definition bits are never lost. `relationships` = verbatim JSON array of `{target, is_reference, is_implementation, is_type_definition, is_definition}` exactly as the provider gave; never synthesized. `symbol_uri = None` for LSP results — LSP has no symbol identity and none is fabricated.)
  - `pub fn insert_semantic_import(&Connection, &SemanticImport) -> Result<i64, String>`
  - `pub fn insert_semantic_facts(&Connection, &[SemanticFact]) -> Result<(), String>`

  Schema (appended to `SCHEMA_SQL`, created by the same `migrate`):
  `semantic_imports(id PK, snapshot_id INT NOT NULL, provider_name TEXT, provider_version TEXT, capability TEXT, project_root TEXT, text_encoding TEXT, artifact_id TEXT, source_revision TEXT, claimed_source_digest TEXT NOT NULL DEFAULT '', content_identity TEXT NOT NULL DEFAULT '', external_symbols TEXT NOT NULL DEFAULT '[]', verification TEXT)` + `semantic_facts(id PK, snapshot_id INT NOT NULL, import_id INT NOT NULL, file_id INT NULL, rel_path TEXT NOT NULL, file_digest TEXT NOT NULL, symbol_uri TEXT NULL, display_name TEXT NULL, roles INTEGER NOT NULL, start_line/end_line/start_col/end_col INT, byte_start/byte_end INT, precision TEXT, relationships TEXT NOT NULL DEFAULT '[]', external INT NOT NULL DEFAULT 0)` + `CREATE INDEX idx_semfacts_lookup ON semantic_facts(snapshot_id, symbol_uri)` + `CREATE INDEX idx_semfacts_span ON semantic_facts(snapshot_id, rel_path, start_line)`. Writer coordination reuses the existing `meta(key,value)` table: `semantic_generation` (integer counter, `0` when absent) is bumped inside the same `BEGIN IMMEDIATE` txn as every committed semantic import — no hashing of semantic rows. No changes to existing tables. `file_id` is NULL when the SCIP document has no row in the M4 syntax snapshot (unsupported/skipped/absent file) — never a fabricated id; `rel_path` + `file_digest` always stored so the fact stays locatable. `artifact_id` / `source_revision` / `claimed_source_digest` / `content_identity` are the separated binding inputs defined in Task 6 (never conflated into one hash).

- [ ] **Step 1: Write failing tests:** `v2_db_migrates_to_v3_with_data_intact` (opens frozen v2 fixture, asserts old snapshots/symbols/calls byte-identical after `migrate`, new tables exist and empty), `v1_db_migrates_to_v3` (frozen v1 fixture → nonce backfill intact + v3 tables present), `migration_failure_rolls_back` (fault-injected migrate failing mid-way → `PRAGMA user_version` still 2 and v2 reads succeed), and `semantic_roundtrip_is_snapshot_scoped` (insert import + facts, query by `(snapshot_id, symbol_uri)`, other snapshot invisible).
- [ ] **Step 2: Run, verify FAIL** (no v3 tables; no explicit txn).
- [ ] **Step 3: Implement migration + helpers in `store.rs`.** Migration runs as one explicit transaction (`BEGIN IMMEDIATE; <schema + backfills>; PRAGMA user_version = 3; COMMIT;`) with rollback on any error — the current `execute_batch` autocommit-per-statement shape is replaced, not extended. Backfill: none needed for new tables (empty); v1→v2 nonce backfill preserved inside the same txn.
- [ ] **Step 4: Verify green** (`cargo test --locked store::`, full `cargo test --locked` for M0–M4 regression).
- [ ] **Step 5: Commit** (`git commit -s -m "store: schema v3 semantic tables, transactional v2 migration"`).

### Task 3: Semantic evidence contract + shared validation + capability registry

**Files:**
- Create: `crates/cli/src/semantic.rs`
- Modify: `crates/cli/src/main.rs` (module declaration only)
- Test: inline unit tests.

**Interfaces:**
- Consumes: `paths::` containment helpers, `store::content_digest`, `store::SemanticImport/SemanticFact`.
- Produces (used by Tasks 4–7):
  - `pub enum Precision { Exact, Heuristic }`, `pub enum Verification { Current, Unverified, Stale }`
  - `pub struct Evidence { pub location: Location, pub symbol: Option<ProviderSymbol>, pub roles: Roles, pub relationships: Vec<Relationship>, pub external: Option<ExternalMeta>, pub precision: Precision, pub verification: Verification, pub provider: ProviderId }`
  - `pub struct Location { pub path: String, pub digest: String, pub span: SpanBytes }` (always required — location evidence is the one mandatory half of every fact)
  - `pub struct ProviderSymbol { pub uri: String, pub display_name: Option<String> }` (optional half — `Some` for SCIP symbol URIs, `None` for LSP results; no SCIP-style identity is ever required of or fabricated for LSP output)
  - `pub struct Roles { pub raw: u32, pub definition: bool, pub reference: bool, pub import: bool, pub read: bool, pub write: bool, pub generated: bool, pub test: bool, pub forward_declaration: bool }` (decoded from the full SCIP role bitset / LSP request kind; `raw` round-trips the verbatim bits into `SemanticFact.roles` so nothing is lost. Stored roles are occurrence roles, never inferred relations. Reference predicate: `!definition` — an occurrence without the Definition bit is reference-side.)
  - `pub struct Relationship { pub target: String, pub is_reference: bool, pub is_implementation: bool, pub is_type_definition: bool, pub is_definition: bool }` (verbatim provider data; M5 stores, M6 traverses)
  - `pub struct ExternalMeta { pub package_manager: String, pub package: String, pub version: String }` (SCIP `external_symbols` package triple; marks facts the producer does not define locally)
  - `pub struct ProviderId { pub name: String, pub version: String }`
  - `pub struct Capability { pub name: &'static str, pub definitions: bool, pub references: bool }`
  - `pub fn validate_doc_path(project_root: &Path, rel: &str) -> Result<String, String>` (enforces the six relative_path rules + canonical containment via `paths::`, rejects symlink escapes)
  - `pub fn validate_range(...) -> Result<..., String>` (single shared range checker: ordered, in-file; line/col base documented once)
  - `pub fn capabilities() -> &'static [Capability]` (static table; initial entries `"scip-import"` {defs+refs} and `"lsp-subprocess"` {defs+refs}; unknown provider → unsupported)
  - `pub fn fallback_report(provider: &str, reason: &str) -> String` (one JSON shape for every unavailable/timeout/error/unverifiable path)

  Every fact must carry repo + source revision (via snapshot binding in Task 7), provider name/version + capability, true relationship type, namespaced symbol identity, path + content digest + byte/line/col span, precision/coverage/verification, and a reason string when unresolved/stale/conflicted/failed.

- [ ] **Step 1: Write failing tests:** traversal rejected (`../` + absolute + `//` + symlink-escape via tmpdir link), range checker rejects end<start, and `fallback_report` JSON has exactly keys `provider,status,reason,syntax_fallback`.
- [ ] **Step 2: Run, verify FAIL.**
- [ ] **Step 3: Implement `semantic.rs` reusing `paths::canonical_root`/containment and `syntax::Span`-style byte spans (no parallel types).**
- [ ] **Step 4: Verify green.**
- [ ] **Step 5: Commit** (`git commit -s -m "semantic: shared evidence contract and path/range validation"`).

### Task 4: SCIP importer (bounded, validated, atomic)

**Files:**
- Create: `crates/cli/src/scip.rs`; check in `proto/scip.proto` (exact upstream copy + version/date note) and generated bindings (checked in, no codegen at build time).
- Test: inline unit tests + fixtures `crates/cli/tests/fixtures/scip/{valid-ts.scip, valid-rust.scip, malformed-*.scip, traversal.scip, dup-conflict.scip, utf{8,16,32}-edge.scip}` (small, hand-built or producer-cut; malformed = truncated varint, unknown wire fields tolerated, duplicate symbol with conflicting display_name).

**Interfaces:**
- Consumes: `semantic::{Evidence, validate_doc_path, validate_range, ProviderId}`, `store::{insert_semantic_import, insert_semantic_facts, content_digest}`.
- Produces: `pub struct ImportLimits { pub max_bytes: u64, pub max_documents: usize, pub max_occurrences: usize, pub max_nesting: u32, pub max_mem_bytes: u64 }` (defaults: 64 MB file, 100k docs, 5M occurrences, nesting 32, 512 MB RSS cap — each recorded in diagnostics), `pub fn import_file(conn: &Connection, snapshot_id: i64, repo_root: &Path, scip_path: &Path, limits: &ImportLimits) -> Result<ImportOutcome, String>`, `pub struct ImportOutcome { pub import_id: i64, pub documents: usize, pub definitions: usize, pub references: usize, pub skipped: Vec<String> }`.

  Rules: decode with a hand-rolled top-level streaming loop over the protobuf low-level reader (read field tag → dispatch `metadata`/`documents`/`external_symbols` → skip unknown; per-document sub-parse with its own occurrence counter). Whole-message parse is BANNED even behind a `CodedInputStream` — the loop must be able to abort mid-stream, proven by test (below). Caps enforced during the loop: byte budget, document count, per-document and total occurrence counts, nesting depth, and an RSS watermark check per N documents; exceeding any cap aborts with a named reason before further consumption. `text`/`documentation`/`signature_documentation` fields are skipped at the wire level, never materialized. Buffers released after ingest.

  File model: a valid SCIP document whose path has no row in the M4 syntax snapshot (unsupported extension, skipped large/binary, or simply unindexed) is stored with `file_id = NULL`, its `rel_path` + disk-byte `file_digest` always recorded — queries locate it by path, never by a fabricated identity. Position encoding (`UTF8/16/32CodeUnitOffsetFromLineStart`) and text encoding (`Metadata.text_document_encoding`: on-disk UTF-16 sources must be decoded to bytes for digest/span mapping) are handled by separate converters and tested separately: position tests use fixed byte content with scripted encodings; text-encoding tests use a real UTF-16-encoded fixture file on disk.

  Remaining validations (unchanged): metadata first/once else fail; `project_root` must canonical-match the registered repo root or the whole import fails; per-document character→byte conversion against real file bytes read from disk (digest mismatch → that document skipped with reason, never silently converted); `UnspecifiedPositionEncoding`/`UnspecifiedTextEncoding` rejected; local symbols never leave their document; duplicate/conflicting records rejected with reason. External symbols: `external_symbols[]` entries are `SymbolInformation` metadata only — copied verbatim into `semantic_imports.external_symbols`, never turned into occurrences, locations, or relationships. An occurrence whose symbol URI parses to a foreign package (or appears in `external_symbols`) is stored as a reference fact with `precision=Heuristic, verification=Unverified, external=1` + `ExternalMeta` package triple parsed from the URI — no location is invented for the external definition itself. Every import lands `Unverified` (`Current` is unreachable in M5 — Task 6 proof boundary). Any failure → `Err` before commit; the caller holds the transaction so a failed import loses nothing.

- [ ] **Step 1 (RED):** tests for valid TS import counts, truncated-file error, traversal rejection, `..` rejection, conflicting-duplicate rejection, UTF-8/16/32 + non-BMP (🚀) byte-exact spans, on-disk UTF-16 text-encoding fixture (digest + spans correct), unindexed-but-valid document stored with `file_id IS NULL` and still queryable by path, digest-mismatch→document skipped with reason, every import lands `Unverified` (`Current` unreachable in M5 — Task 6 proof boundary; no attestation system built), over-cap file rejection, full role-bitset round-trip (`0x2|0x8|0x10|0x20|0x40` occurrence stored verbatim in `roles` and still returned by def/ref-filtered queries), external-symbol metadata stored without creating any fact row (fact count unchanged by `external_symbols[]`), and the streaming proof: a crafted over-cap Index fed through a byte-counting reader aborts with `bytes_consumed < file_len` (whole-message parse would consume everything — this test fails against it).
- [ ] **Step 2: Run, verify FAIL.**
- [ ] **Step 3: Implement importer.** Decode only `Index{metadata, documents, external_symbols}` + nested `Document/Occurrence/SymbolInformation/Relationship` subsets; skip `text/documentation/signature_documentation` (never persisted — RAM discipline).
- [ ] **Step 4: Verify green** incl. `cargo test --locked` full.
- [ ] **Step 5: Commit** (`git commit -s -m "scip: bounded importer with encoding-aware spans"`).

### Task 5: LSP subprocess adapter (explicit opt-in, std-only)

**Files:**
- Create: `crates/cli/src/lsp.rs`
- Test: inline tests with scripted byte streams + one `current_exe` mock-server integration test (mock speaks framing; scenarios: success, timeout, crash/EOF, malformed JSON, stdout pollution, hang-then-kill).

**Interfaces:**
- Consumes: `semantic::{Evidence, ProviderId, validate_range}`, `paths::` containment.
- Produces:
  - `pub struct ServerSpec { pub exe: String, pub args: Vec<String>, pub root: PathBuf, pub timeout_ms: u64, pub max_msg_bytes: usize, pub max_messages: usize }` (explicit absolute exe path, no shell, no auto-install/download; defaults: timeout 30 s, max 8 MB/msg, max 10k msgs/run)
  - `pub struct LspEvidence { pub facts: Vec<Evidence>, pub server_version: String, pub position_encoding: String, pub diagnostics: Vec<String> }`
  - `pub fn query_definitions<T: evidenced-transport>(...)` / `pub fn query_references(...)` — transport generic over `Read+Write` streams so tests inject scripted bytes without processes; production constructor `spawn_spec` uses `std::process::Command` directly (argv array, `stdin/stdout piped`, `stderr captured separately`).
  - `pub fn source_binding(server: &ServerSpec, head: &str) -> String` (digest binding server identity + repo revision; feeds one third of the Task 6 tripartite binding).

  Transport and reliability (all mandatory, all mock-regressed): three threads — stdout reader, stderr drain, stdin writer — joined over bounded channels so a chatty stderr or a slow consumer can never deadlock the response pump; stdout pollution (non-framing bytes) is a protocol error, never skipped; interleaved `window/*` / `$/progress` notifications are consumed by kind (progress tracked, unknown `$/`-methods rejected explicitly); every request carries a deadline — on timeout send `$/cancelRequest`, then kill, then reap (`try_wait` loop → `kill` → blocking `wait`, asserting the handle is gone; zombie handle = test failure); crash/EOF/malformed/hang → typed error → caller falls back (Task 6), never partial facts. Advertised-capability honesty: the client advertises exactly `textDocument.definition`, `textDocument.references`, and one negotiated `positionEncodingKind`; any request/notification outside what handshake agreed (e.g. server lacking `referencesProvider` but queried for refs) returns explicit `unsupported`, never an error masquerading as evidence. Trust boundary documented in help text + validation doc: servers may run project hooks/build scripts or network — subprocess is untrusted, explicit opt-in only, syntax-only path never executes repo code.

- [ ] **Step 1 (RED):** framing round-trip, init handshake bytes, def/ref parse, timeout kill + handle-reaped assertion, crash EOF error, malformed JSON error, stdout-pollution rejection, stderr-flood-during-request (no deadlock, response still processed), interleaved `$/progress` consumed vs unknown `$/`-method rejected, `$/cancelRequest` emitted on timeout (mock asserts receipt), references-against-capability-less-server → `unsupported`, URI-escape rejection, UTF-16 surrogate-pair span math, mock-server success end-to-end via `current_exe`.
- [ ] **Step 2: Run, verify FAIL.**
- [ ] **Step 3: Implement with `std::io` + `std::process` only (no async runtime, no new deps beyond Task 1).**
- [ ] **Step 4: Verify green.**
- [ ] **Step 5: Commit** (`git commit -s -m "lsp: opt-in subprocess adapter with bounded JSON-RPC"`).

### Task 6: Provider compatibility registry + fallback + staleness binding

**Files:**
- Modify: `crates/cli/src/semantic.rs` (registry table + `resolve()`), `crates/cli/src/index.rs` (`evaluate_snapshot` extension point only — no behavior change to syntax freshness).
- Test: inline + `crates/cli/tests/semantic.rs`.

**Interfaces:**
- Consumes: Tasks 3–5 outputs, `store::Snapshot.nonce`.
- Produces: `pub fn resolve(provider: &str) -> Result<&Capability, String>`, `pub fn verify_live(import: &SemanticImport, live: &LiveState) -> Verification`, `pub struct LiveState { pub head: String, pub dirty: bool, pub untracked: u32, pub config_fingerprint: String, pub snapshot_nonce: String, pub content_identity: String }` — the ONE freshness/nonce rule for semantic facts, computed at query time from three separated inputs that are stored separately and never conflated:
  1. **artifact identity** (`artifact_id` = digest of the exact producer bytes: SCIP file bytes / LSP server identity + query inputs) — answers "which producer output is this?";
  2. **source digest/revision evidence**: `source_revision` (repo HEAD + dirty/untracked + config fingerprint + live snapshot nonce, compared field-by-field) PLUS `content_identity` — digest over the sorted `(path, file_digest)` pairs of the live `files` table. Nonce ≠ content: HEAD/dirty/untracked alone cannot detect content changes that leave the git fingerprint identical (non-git repos, fingerprint granularity, change-and-restore races), so the content digest is compared independently. Answers "which source state was it proven against?";
  3. **provider verification** (capability still advertised; `claimed_source_digest` recorded as an untrusted claim only) — answers "may it be trusted now?" (in M5 the answer is at best `Unverified`: no independent producer-to-source proof exists).

  Proof boundary (explicit): Riltra-generated artifact hashes, HEAD strings, and snapshot nonces prove Riltra's bookkeeping only — they say nothing about what the producer actually read. `--source-digest` is a user-supplied CLAIM, not trusted producer attestation: matching it never yields `Current`. Without an independent producer-to-source proof mechanism (no new attestation system is built for M5 — explicitly deferred), `Current` is UNREACHABLE: every M5 import lands `Unverified`, and any source/config/dependency drift flips it to `Stale`. The claim is still recorded (`claimed_source_digest`, surfaced verbatim in `semantic status` as a claim, never as proof) for audit transparency. LSP live-query answers are always `Unverified` (ephemeral, unbindable).

  Conservative invalidation (M5 rule, no semantic dependency graph): an unchanged file digest does NOT imply relationships through that file are still valid — any other file or dependency may have changed. Therefore verification is import-granular: if ANY field of the live revision evidence differs from the import's bound evidence, EVERY fact of that import reports `Stale` (still returned with explicit marking — never silently dropped, never served as `Current`). Carry-over across rebuilds preserves row locatability only; it never upgrades verification. Full regeneration/re-import is the M5 invalidation strategy — semantic incremental dependency analysis is out of scope.

  Rules enforced here: provider missing/timeout/error/unverifiable → `fallback_report` + syntax index stays usable; facts from two revisions never mixed in one response (import binds one nonce; query filters by live snapshot id).

- [ ] **Step 1 (RED):** unknown provider → `unsupported`; simulated timeout → fallback JSON + syntax results intact; no-op rebuild with byte-identical source → import stays `Unverified` (rebuilds alone never invalidate AND never upgrade); same git fingerprint but different content (fixture: bytes changed while HEAD/dirty/untracked replayed identical) → `Stale` (proves `content_identity` is compared independently of git fields); dependency/config change (config fingerprint drift with identical file bytes) → `Stale`; attested import (`--source-digest` given) with everything matching → still `Unverified` (claim recorded as claim, never trusted as proof); unattested import → `Unverified`; two-revision facts never co-occur.
- [ ] **Step 2: Run, verify FAIL.**
- [ ] **Step 3: Implement (small match + binding compare; no new storage).**
- [ ] **Step 4: Verify green + full M0–M4 suite green.**
- [ ] **Step 5: Commit** (`git commit -s -m "semantic: capability registry and revision binding"`).

### Task 7: Atomic semantic import path + minimal CLI (`import|status|query`)

**Files:**
- Modify: `crates/cli/src/index.rs` (`build_full`/`build` epilogue: optional semantic phase reusing staging txn + nonce guard pattern), `crates/cli/src/args.rs` (`Command::Semantic{Import|Status|Query}`), `crates/cli/src/main.rs` (`cmd_semantic_*`, usage lines).
- Test: `crates/cli/tests/semantic.rs` end-to-end.

**Interfaces:**
- Consumes: Tasks 2–6.
- CLI (final, testable, M6-disjoint):
  - `riltra semantic import --repo <name> --scip <file> [--source-digest <hex>]` — does NOT trigger a syntax rebuild. Opens the live DB, `BEGIN IMMEDIATE`, reads the live latest snapshot (id + nonce + head), computes `content_identity` over the live `files` rows, runs `scip::import_file` against that snapshot id, bumps `meta.semantic_generation`, `COMMIT`; any error rolls back leaving the syntax snapshot byte-identical. `--source-digest` records the user's claim verbatim (`claimed_source_digest`); omitted or not, verification lands `Unverified` (Task 6 — a claim is never proof). JSON: `{snapshot, import_id, provider, documents, definitions, references, verification, fallback_reason}`.
  - `riltra semantic status --repo <name>` — per-provider rows: `{provider, version, capability, verification, artifact_id, claimed_source_digest, content_identity_match, source_revision_match, limitation}` + syntax freshness passthrough.
  - `riltra semantic query --repo <name> --path <rel> --line <n> --col <n> [--refs] [--lsp-server <abs-path>]` — single-hop only, filtered to the live latest snapshot id with `verify_live` applied per fact at query time:
    - **Go to Definition** (default): from the occurrence enclosing (`path`, `line`, `col`), resolve its symbol identity (SCIP: non-local URI required — local symbols resolve within their document only; LSP: the server's `definition` response for the position) and return the fact row(s) holding the Definition role for that identity. No valid identity → explicit `unresolved`, never a guessed target.
    - **Find References** (`--refs`): return reference-role occurrences sharing that same identity (SCIP bitmask `!definition`; LSP: the server's `references` response). `is_reference` relationship closure is stored data, not traversal — M6 owns expansion.
    - One live LSP def/ref call with explicit server path answers the same shape but is never persisted in M5 (ephemeral evidence always `Unverified`) and always carries `fallback` when the server is missing/slow/crashing.
    Output `{definitions|references:[{symbol_uri|null, display_name, path, span, roles, precision, verification}], provider, fallback}`. No traversal, no callers/callees — M6 boundary pinned by test (`--refs --refs-depth` rejected as unknown flag; nested queries absent).

  Snapshot lifecycle (audit of `store::promote_staging`, `store.rs:336-361`: it DELETEs + `INSERT..SELECT`s only the 5 syntax tables in one txn — semantic tables MUST join that same txn or every rebuild orphans them): extend the promote batch with `DELETE FROM semantic_facts; DELETE FROM semantic_imports; INSERT INTO semantic_imports SELECT * FROM src...; INSERT INTO semantic_facts SELECT * FROM src...;` so syntax + semantic promote atomically or not at all. Because every rebuild mints a new snapshot id, facts are carried inside the staging build by per-file digest-verified copy (same triple-predicate discipline as M4 `copy_file_row`: path + digest must match the new snapshot's `files` row, else the fact is dropped, never re-keyed blindly). Carry-over preserves locatability only — it never upgrades verification (Task 6: any revision drift forces the whole import to `Stale` at query time regardless of per-file matches). No snapshot-id reuse can ever attach old facts to a new revision: carry-over is explicit, verified, and same-transaction.

  Concurrent-writer coordination (minimum correct: a `semantic_generation` counter in the existing `meta` table, bumped in-txn on every committed import — no row hashing): the build records `base = (live snapshot nonce, semantic_generation)` at staging start; promote asserts both unchanged inside the same `BEGIN IMMEDIATE` txn and aborts with `fallback_reason: semantic-changed` (live syntax + committed import intact) on mismatch. A `semantic import` that commits between staging-start and promote can therefore never be silently wiped by a stale staging copy. Deterministic race regression (below) pins both orders.

- [ ] **Step 1 (RED):** CLI parse tests (all three commands + `--scip` missing → usage exit 2); import atomicity test (malformed SCIP → exit≠0, previous snapshot id + facts + syntax rows unchanged — and no new snapshot row created by the import itself); query parity test (stored fact == query output); cross-file def/ref test (definition in file A, references in files B and C: query at B resolves to A; `--refs` at A returns B and C; a same-name symbol from the other provider never merges); local-symbol test (def/ref resolve within one document only, never across); unresolved-identity test (occurrence without Definition role anywhere and no valid URI → `unresolved`, no guess); lifecycle sequence test `import → no-op index → delta index → full rebuild → query` (facts carried for location; after the delta edit the whole import reports `Stale` per Task 6 even for untouched files; promote-failure injection leaves live syntax + semantic rows intact); deterministic race test both orders (staging built → import commits → promote aborts `semantic-changed` with import intact; import commits → staging built after → promote succeeds and carries the import); LSP live-query test gated behind `RILTRA_TEST_LSP` env (skipped silently otherwise, skip reason printed — never fake PASS).
- [ ] **Step 2: Run, verify FAIL.**
- [ ] **Step 3: Implement args + dispatch + live-txn import + promote-batch extension + verified carry-over (semantic phase reuses staging discipline; `--full` syntax behavior byte-identical when no `--scip` given — pin with existing M4 parity tests; no M4 refactor beyond appending the two tables to the promote batch and one carry-over helper beside `copy_mapped_facts`).**
- [ ] **Step 4: Verify green (full suite).**
- [ ] **Step 5: Commit** (`git commit -s -m "cli: semantic import/status/query on staging path"`).

### Task 8: Tier A deterministic suite (fixtures + adversarial + regression)

**Files:**
- Modify: `crates/cli/tests/semantic.rs` (assemble all Tier A cases), fixtures from Task 4 + new LSP scripted streams.
- Covers, each a named test: SCIP valid + malformed set; UTF-8/16/32 + non-BMP + range-boundary (empty range, range past EOF, multi-line-encoded single-line); def/ref resolution incl. `is_definition` override and `is_reference` closure recorded-but-untraversed; full role-bitset preservation (import/read/write/generated/test/forward bits survive store + query); duplicate/conflicting symbols; external symbols → metadata stored, zero fact rows fabricated, referencing occurrences marked `Unverified`+external; unattested import permanently `Unverified`; cross-file def→refs query contract (Task 7); digest mismatch + stale revision + path escape; LSP mock success/timeout/crash/cancel/malformed/kill-reaped/capability-honesty; provider-unavailable → syntax-only fallback with intact `search`; atomic import rollback (snapshot id + row counts unchanged); deterministic promote-vs-import race both orders (`semantic-changed` abort, no silent wipe); concurrent query during import (WAL read snapshot stability); repo isolation (facts never leak across repos); v1→v3 and v2→v3 migration + rollback + full M0–M4 regression (`cargo test --locked` green).

- [ ] **Step 1: List any Tier A case lacking a test after Tasks 3–7; write the missing failing tests.**
- [ ] **Step 2: Run full suite, verify only the new ones fail.**
- [ ] **Step 3: Implement minimal fixes (no architecture changes; any new gap → fix in owning module).**
- [ ] **Step 4: Verify `cargo fmt --check && cargo check --locked && cargo clippy --all-targets -- -D warnings && cargo test --locked` green on Windows.**
- [ ] **Step 5: Commit** (`git commit -s -m "test: Tier A semantic coverage complete"`).

### Task 9: Tier B real providers (gated, inspection-proven)

**Files:** none (evidence only) + `docs/m5-validation.md` Tier B section.
- Provider 1 (LSP): local `rust-analyzer` (present: `%CARGO%\rust-analyzer.exe`) against the Riltra repo itself or a small Rust fixture repo: prove one definition + one references result by quoting actual source spans (file, line, symbol), report false positives/missing coverage, wall time, peak RSS, server startup cost. Syntax-only path never spawns it (prove by absence in logs/behavior).
- Provider 2 (SCIP): `scip-typescript` (NOT installed locally) on a small TS fixture: install explicitly (`npm i -g` or npx-pinned, version recorded), import, prove def/ref by source inspection, same metrics.
- Gate honesty: Tier B PASS requires both providers; if `scip-typescript` cannot be installed legitimately in this environment, report Tier B as FAIL-blocked with the exact blocker — mock tests never substitute. Paired runs, no contention, ≥2 repetitions; record `fallback_reason`, phase timings, verification states.

- [ ] **Step 1: Run rust-analyzer gate, record evidence.**
- [ ] **Step 2: Attempt explicit scip-typescript setup; record version or blocker.**
- [ ] **Step 3: Write Tier B section with verdict (no speedup/coverage claim before numbers exist).**
- [ ] **Step 4: Commit docs** (`git commit -s -m "docs: Tier B real-provider evidence"`).

### Task 10: Validation doc, parity gates, close-out

**Files:**
- Create: `docs/m5-validation.md` (pattern of `m4-validation.md`: gates, Tier A table, Tier B numbers, limitations).
- Must prove: M4 incremental parity (no-op + one-file delta timings unchanged — no re-run of full MMSP benchmark required, cite M4 numbers + one fresh no-op confirmation), freshness regression (all M3/M4 freshness tests green), core-only CI equivalence (all semantic tests skip-or-pass with no providers installed), 3-OS CI green, license audit table, trust-boundary statement for LSP subprocess, deferred list (incremental semantic graph, M6 traversal, streaming field-visitor if caps+release chosen over true streaming).

- [ ] **Step 1: Draft `docs/m5-validation.md` from task evidence.**
- [ ] **Step 2: Self-review against M5 spec sections A–H; every MUST has a task/test pointer; gaps become new failing tests (return to owning task), not prose.**
- [ ] **Step 3: Final full gate locally; record output.**
- [ ] **Step 4: Commit; STOP for review — no push, no M6.**

## Self-review (against spec A–H + 9 review findings)

- Finding 1 (snapshot lifecycle): Task 7 extends the audited `promote_staging` batch with both semantic tables in the same txn + verified carry-over + the import→noop→delta→full→query lifecycle test; snapshot-id reuse is impossible by construction (explicit re-key or drop).
- Finding 2 (no rebuild for import): Task 7 specifies the live-DB `BEGIN IMMEDIATE` import path with rollback proof and a no-new-snapshot assertion.
- Finding 3 (tripartite binding): Task 6 separates artifact identity / source evidence / provider verification with a dedicated non-conflation test; Task 2 stores the three columns separately.
- Finding 4 (no mandatory SCIP identity): Tasks 2–3 split `Location` (required) from `ProviderSymbol` (optional), verbatim `Relationship`s, `Roles`, and `ExternalMeta`; LSP `symbol_uri = None` by rule.
- Finding 5 (file model): Task 2 (`file_id NULL` + always-stored `rel_path`/`file_digest`) and Task 4 (no-fabricated-identity test, separate position vs text-encoding converters and tests).
- Finding 6 (true streaming): Task 4 bans whole-message parse and pins the abort-before-EOF proof test with a byte-counting reader.
- Finding 7 (LSP reliability): Task 5 pins the three-thread pump, kill+reap assertion, cancel emission, capability honesty, and one mock regression per path.
- Finding 8 (migration txn): Task 2 uses one explicit transaction, a mid-migration rollback test, and a v1→v3 fixture test.
- Finding 9 (license sources): Task 1 takes versions from Cargo.lock and licenses from cargo metadata/manifests, with the distinction pinned in the test.
- Third-review corrections (2):
  1. Claim ≠ proof: `--source-digest` is recorded verbatim as `claimed_source_digest` and surfaced as a claim in `semantic status`; matching it never yields `Current`. With no independent producer-to-source proof in M5, `Current` is unreachable by rule (enum variant kept for contract stability; independent attestation deferred, no new system built). Task 6 tests pin attested-yet-matching → `Unverified`.
  2. Nonce ≠ content: `LiveState` gains `content_identity` (digest over sorted live `(path, file_digest)` pairs, stored per import and recomputed at query); git fields alone are insufficient. Task 6 pins three regressions: no-op-identical-source (verification untouched), same-git-fingerprint-different-content (→ `Stale`), dependency/config drift with identical bytes (→ `Stale`). Writer coordination uses the minimum-correct `meta.semantic_generation` counter (Task 2 schema, Task 7 promote assert) instead of hashing semantic rows.
- Second-review corrections (6):
  1. Conservative invalidation: Task 6 makes verification import-granular (any revision drift → whole import `Stale`); per-file digest gates location only, never `Current`. Carry-over (Task 7) preserves rows, never upgrades verification. No semantic dependency graph.
  2. External symbols: Task 2 stores `external_symbols[]` as verbatim metadata on the import row; Task 4 forbids fabricating occurrences/locations/relationships from it, with a zero-new-facts test.
  3. Proof boundary: Task 6 requires producer attestation (`--source-digest`, Task 7 CLI) for `Current`; unattested imports and all LSP live answers are permanently `Unverified`; Riltra-side hashes/nonces alone prove bookkeeping only.
  4. Full bitset: Task 2 stores `roles INTEGER`, Task 3 round-trips `Roles.raw`, Task 4 pins the preservation test; def/ref filtering is a query-time bitmask.
  5. Writer coordination: Task 7 records `(snapshot nonce, meta.semantic_generation)` at staging start and asserts both at promote with `semantic-changed` abort and deterministic both-orders race regression — no lock files, no framework, no row hashing.
  6. Query contract: Task 7 defines Go-to-Definition (occurrence → definition via valid identity) and Find References (shared-identity reference occurrences) with cross-file, local-symbol, and unresolved-identity acceptance tests.
- A1 migrations: Task 2 (transactional, regression-tested). A2/A3/A4 providers→one contract: Tasks 3–5. A5 syntax/semantic separation: Tasks 3 (types) + 6 (fallback) + 10 (parity). A6 default syntax-only: Tasks 6–7. A7 no M6: Task 7 boundary test.
- B reuse list: registry/paths/digests/SQLite/migrations/staging/spans/freshness/nonce each consumed by a named task interface. One path+range validator (Task 3), one provenance/diagnostics shape (Task 3). Tested JSON/protobuf libs (Task 1). License audit (Task 1). Bans restated in Global Constraints + license test.
- C contract fields: Task 3 `Evidence` + `SemanticImport` binding; no-stable-ID rules pinned in Task 3 tests (same name two providers → distinct URIs).
- D SCIP: Task 4 (bounded/streaming-disciplined parse, traversal/symlink/dup/malformed rejection, UTF handling, provenance, caps, atomicity, attestation→Unverified rule, official + adversarial fixtures, producer-agnostic decode of the standard subset).
- E LSP: Task 5 (all seven lifecycle items, all eight security rules, encoding negotiation incl. non-BMP, no generic framework).
- F registry/fallback: Task 6 (caps, no-fabrication, no-promotion, no-mixing, syntax-usable, explicit status; regeneration-over-incremental).
- G Tier A: Task 8 (every listed case named). Tier B: Task 9 (two providers, inspection proof, costs, no mock-only PASS). CI: Global Constraints + Task 10.
- H: this plan is the single plan; file paths/interfaces/RED→GREEN/fallback/boundaries/gates included; plan saved at `docs/superpowers/plans/2026-10-16-m5-semantic-providers.md`; stop after review.
- Type consistency: `Evidence` (Task 3) is the single cross-task type; `ImportOutcome`/`LspEvidence` convert into it at module edges; `Verification` enum shared by Tasks 3/4/5/6/7.
- Proportion: 10 tasks for an 8-section spec; code bodies left to implementers, only pinned values and signatures given.
