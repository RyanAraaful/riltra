# M7 validation — native structured knowledge graph and closure

M7 Full-stage work on branch `m7-native-structured-knowledge-graph`. The
bounded hardening checkpoint was audited against baseline
`ee14612e1ac3f829cae7926e65b9c0cf96dcac3b` and landed three commits: `7064773`
(G-BACK restore fail-closed), `7736d6e` (G-PORT canonical JSONL events and
G-PROV trust boundary), `fb8d109` (restore refusal guidance). The accepted
implementation HEAD is `fb8d1090f9ea18251d58fb1723dd72419aa971ab`. The
checkpoint delta touches `backup.rs`, `knowledge.rs`, `knowledge_store.rs`
(source-comment contracts only) and `tests/knowledge.rs`; schema v2,
dependencies, CI configuration, M0–M6 and MCP are unchanged.

**M7 — Native Structured Knowledge Graph: PASS — CLOSED**, by project-owner
decision on 2026-10-18, with the accepted limitations recorded below. The
reviewer validated the G-BACK, G-PORT and G-PROV checkpoint and accepted the
offline restore limitations. This documentation follow-up changes no
production code and does not claim CI ran the tests on the documentation
commit.

## Closure decision

| | |
| --- | --- |
| Decision | M7: PASS — CLOSED |
| Decision date | 2026-10-18 |
| Accepted implementation HEAD | `fb8d1090f9ea18251d58fb1723dd72419aa971ab` |
| Final CI evidence | [Actions #39091621405](https://github.com/RyanAraaful/riltra/actions/runs/39091621405) |
| CI Windows / Ubuntu / macOS | PASS / PASS / PASS |

[Actions #39091621405](https://github.com/RyanAraaful/riltra/actions/runs/39091621405)
completed successfully in 3m 37s on SHA `fb8d109` in every job:

| Check | Windows | Ubuntu | macOS |
| --- | --- | --- | --- |
| `cargo fmt --all -- --check` | PASS | PASS | PASS |
| `cargo clippy --all-targets -- -D warnings` | PASS | PASS | PASS |
| `cargo check --locked` | PASS | PASS | PASS |
| `cargo test --locked` | PASS | PASS | PASS |
| CLI help/version/status smoke | PASS | PASS | PASS |

The local acceptance run on the same implementation state (Windows, before
push) recorded `cargo test --locked` with 565 passed and 0 failed across seven
suites — 314 unit tests plus 16, 137, 21, 50, 15 and 12 integration tests —
and an end-to-end backup/restore CLI smoke (backup, verify, store loss,
restore, show/history/related/search/verify, byte-identical export, generation
preserved). CI job logs confirm the `cargo test --locked` step succeeded on
all three platforms; per-OS test counts were not extracted and are not
claimed. All three implementation commits carry `Signed-off-by` (DCO).

## Accepted capabilities

Implemented and tested, per registered repository, offline under
`$RILTRA_HOME/knowledge/<name>.db`, on schema v2 (v1 stores stay readable and
migrate lazily on the first Full-stage write). The M6 code index is never
touched.

- **Structured knowledge records** — create, show and update with explicit
  input validation, deterministic key-ordered JSON and exit codes (0 ok,
  2 usage, 3 repository/data).
- **Revision and lifecycle history** — immutable revisions with optimistic
  `--expected-revision`; explicit supersede and retract transitions carrying
  reason and author; paginated `history` with a generation guard.
- **Typed relationships** — revision-pinned links with kind and rationale,
  canonical relation ids, retraction, and inbound/outbound `related`
  traversal.
- **Provenance/confirmation semantics** — `candidate` is the default;
  `confirmed` is an explicit confirmation claim, not an authenticated
  identity; the state is derived from stored `confirm` events and is never
  promoted automatically from evidence, freshness, search or indexing.
- **Evidence and freshness** — captured source bindings with digests and git
  context, reported as fresh/stale/unresolved independently of confirmation
  state.
- **Search and retrieval** — deterministic FTS5 search with active-only
  default visibility, explicit archived/superseded/retracted flags, bounded
  pagination and a generation guard.
- **JSONL/Markdown portability** — canonical v1 JSONL whose lifecycle audit
  metadata rides as structured `events`; byte-identical export→import→export,
  atomic rejection of malformed, divergent and duplicate payloads, and a
  re-import that does not advance the generation; the Markdown wrapper
  carries the same canonical payload.
- **Backup, verify, restore** — consistent `VACUUM INTO` snapshot plus a
  manifest bound to repository name and canonical root; structural `verify`
  (`PRAGMA integrity_check`, schema version, binding, required tables, counts
  — no digest or authenticity claim); staged, verified, single-rename restore
  that refuses an existing store without `--overwrite` and leaves the
  original untouched on failure.
- **Repository isolation and compatibility** — one store per registered
  repository, cross-repo restore refused by binding, no M6 index changes.

These are the verified capabilities within the agreed M7 scope, not universal
claims: verification is structural rather than cryptographic, the tool is
local and single-user, and no concurrency, crash or durability guarantee
beyond the recorded evidence and the limitations below is made.

## Accepted limitations

**L1 — Restore offline only.** Restore is supported only while no other
process is using the knowledge store. M7 makes no broader locking claim.

**L2 — SQLite companion fail-closed.** Restore refuses (exit 3, with the store
and the companion left untouched) whenever a `-wal`, `-shm` or `-journal`
file exists next to the target store — including companions left behind by
ordinary read-only operations. The refusal is deliberate: a companion may
still hold committed transactions of the store being replaced, so it is never
deleted or moved automatically and must not be discarded casually.

**L3 — Concurrent-access race.** A check-to-rename window remains; restore is
not claimed safe against concurrent opens of the database. Recorded as a
`shortcut:` marker in `backup.rs`.

**L4 — Crash/power-loss durability.** No guarantee is made for interruption
at any point of backup or restore beyond the failure paths exercised by the
tests; no fsync-level durability is claimed.

All four limitations are accepted for the M7 scope by the project-owner
decision above. They are maintenance candidates, not closure blockers.

## Deferred maintenance

- Restore UX and a safe SQLite checkpoint procedure, so the fail-closed
  companion refusal is the exception rather than the common path after
  read-only operations.
- Restore handling of concurrent access, if it is ever required.
- Minor cleanup: the duplicated scratch-WAL test fixture, and documenting
  that `--file` takes the backup base path.

## Scope verification

The accepted implementation delta (`ee14612..fb8d109`) changes exactly four
files — `backup.rs`, `knowledge.rs`, `knowledge_store.rs` (source-comment
contracts only) and `tests/knowledge.rs` — with 259 insertions and 77
deletions. It contains no schema, dependency, CI-configuration, M0–M6 or MCP
change; `Cargo.toml`, `Cargo.lock` and `.github/` are untouched. Internal
planning/spec documents remain ignored and unpublished, and `AGENTS.md`
remains untracked.

This closure commit adds this file and updates the README status reference
only.
