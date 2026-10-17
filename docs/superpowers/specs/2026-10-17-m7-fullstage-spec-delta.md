# M7 Full-stage Specification Delta (Revision 2)

Status: **DRAFT FOR REVIEW — REVISED**. This delta extends the approved design
spec (`2026-10-16-m7-native-structured-knowledge-graph-design.md`) and the
approved Foundation plan (`2026-10-16-m7-foundation.md`). It specifies
contracts only; it authorizes **no** implementation. Every clause below is a
proposed contract requiring reviewer ratification. Where alternatives exist, a
recommendation is given with its trade-off; a recommendation is not an approval.

**Revision 2 changes (reviewer-driven):** schema v2 with guarded migration
(replaces the Revision 1 additive-tables-under-v1 approach); search visibility
defaults to `active` only; supersedes-relation contract made exact; Markdown
import/export contract proposed with a scope decision; import resource limits
set; backup/restore failure and tamper-detection semantics made explicit;
provenance default `candidate` approved with identity distinction; command
surface and versioned JSONL/manifest schemas finalized with examples.

## 0. Scope and ordering

Four gates, each independently implementable and independently verifiable:

- **G-LIFE** — `superseded` / `retracted` record lifecycle.
- **G-PORT** — deterministic JSONL (canonical, lossless) export/import and
  Markdown export/import under an explicit contract.
- **G-BACK** — offline backup, integrity verification, offline restore.
- **G-PROV** — human-confirmed vs candidate provenance for records and
  relations.

Dependencies between gates: none required. G-LIFE's `supersedes` relation kind
is referenced by G-PORT export and G-BACK backup (both must carry it), but each
gate's acceptance does not require the others to be implemented first.

## 1. Shared Full-stage contracts

These apply to all four gates and are compatible extensions of Foundation.

### 1.1 Status vocabulary

Foundation statuses remain `active` and `archived`. Full-stage adds
`superseded` and `retracted`. The four are distinct states; code and tests must
never map one onto another. `archived` is never a substitute for `superseded`
or `retracted`, and vice versa.

### 1.2 Single transition contract (unchanged shape)

Every status change is a new revision written by the existing
`knowledge update --expected-revision N --input -` path: one SQLite write
transaction, immutable revision, monotonic `knowledge_generation` bump,
generation rollback on failure. **No new lifecycle commands are introduced by
G-LIFE.** A dedicated command is only permitted on demonstrated need and must
create a new revision with the same optimistic-concurrency check.

### 1.3 Schema v2 with guarded migration (D1)

Full-stage ships as **schema version 2**. Foundation stores are schema version
1. A guarded, one-time migration moves a store from v1 to v2.

**Version constants and open behavior:**

- `KNOWLEDGE_SCHEMA_VERSION` becomes `2` in the Full-stage binary. The
  Foundation binary's constant stays `1`.
- The Full-stage binary accepts stores with `PRAGMA user_version` 1 or 2 and
  refuses `> 2` with a forward-version storage error (exit 3). It also refuses a
  store whose `user_version` and `meta.knowledge_schema_version` disagree.
- The Foundation binary refuses a v2 store: `user_version > 1` is already
  rejected by its existing forward-version check. **An old Foundation binary
  cannot open, read, or adopt a Full-stage v2 store.**

**Explicit v1-read / v2-missing behavior:**

- A Full-stage **read** against a v1 store never migrates and never errors on
  missing v2 tables. Reads that derive state from v2 lifecycle tables
  (`confirm_state`, lifecycle events) against a v1 store return the documented
  defaults (`candidate`, no lifecycle events) without migrating.
- A Full-stage **write** that needs v2 tables against a v1 store performs the
  migration as the first step inside its own write transaction (above), then
  proceeds. There is no code path that needs v2 tables without migrating
  first, and no code path that migrates without a Full-stage write.

**When migration runs:**

- Migration is **never** triggered by a read. Reads (including Foundation
  commands and Full-stage `show`/`history`/`search`/`related`/`export`/`verify`)
  against a v1 store never migrate it.
- Migration runs **lazily inside the first Full-stage write transaction**
  against a v1 store (a Full-stage `update`/`link`/`import`/`restore`/`backup`
  that writes). The transaction performs: (1) `CREATE TABLE IF NOT EXISTS` for
  `record_lifecycle` and `relation_lifecycle`, (2) update
  `meta.knowledge_schema_version` to `'2'`, (3) `PRAGMA user_version = 2`, then
  (4) the command's own write, all in one transaction.
- Foundation writes against a v1 store by the Full-stage binary do **not**
  migrate: Foundation writes (create/update/link/unlink on active/archived
  data) proceed against v1 exactly as before. Only commands that need v2
  lifecycle tables migrate.

**Treatment of existing data:** migration creates two empty tables and stamps
version metadata. Existing records, revisions, heads, evidence, relations,
retractions, measurements, FTS content, and the `knowledge_generation` counter
are **byte-identical and untouched**. No row is rewritten.

**Atomicity, rollback, crash recovery:** the migration shares the write
transaction. SQLite's `PRAGMA user_version` is transactional: a crash between
table creation and version stamp rolls back both; a crash after commit leaves
a consistent v2 store. There is no partial-migration state. On any migration
failure the whole command fails (exit 3) and the store remains v1.

**Negative tests required:** injected failure mid-migration → rollback, store
still opens as v1, lifecycle tables absent, `user_version` still 1, generation
unchanged; corrupt v2 store (`user_version = 2` with missing lifecycle tables,
or `user_version` ≠ `meta.knowledge_schema_version`) → exit 3, fail closed,
no delete/recreate.

> **Decision D1 (resolved by reviewer):** schema v2 with guarded migration.
> Revision 1's additive-tables-under-v1 approach is withdrawn.

### 1.4 Lifecycle tables and supersedes relation (D2/D3)

`record_lifecycle` — append-only events keyed by `(id, revision)`:
`event` ∈ {`supersede`, `retract`, `confirm`}, `target_id`, `target_revision`,
`reason`, `author`, `created_unix`. One row per lifecycle event.

`relation_lifecycle` — append-only events keyed by `relation_id` with
`event` = `confirm`, `author`, `created_unix` (relation retraction already
exists as `retractions` in Foundation and is unchanged).

`confirm_state` for a record revision is **derived**: the latest `confirm` or
absence of events in `record_lifecycle`. Default is `candidate` (see G-PROV).

**New relation kind `supersedes`** — exact contract:

- Source: the **superseding** (newer) record at the revision named by
  `superseded_by.revision`. Target: the **superseded** (older) record at the
  revision that carries the `superseded` status.
- Direction: `src` = superseding record revision, `dst` = superseded record
  revision, kind `supersedes`. So `related --id <old> --direction inbound
  --kind supersedes` lists the replacement; `related --id <new> --direction
  outbound --kind supersedes` lists the superseded.
- Written **implicitly and atomically** inside the supersede transaction, with
  deterministic relation id `supersedes:<src>:<src_rev>:<dst>:<dst_rev>` and
  the revision's author/rationale.
- Valid target conditions: the target must be an existing record revision in
  the same store (exact id + revision, same endpoint-existence rule as
  relation endpoints), must not be the superseded record itself, and must not
  be a `retracted` record (a superseded transition of a retracted record is
  already rejected).
- Cycle/conflict rules: the authoritative `supersedes` lineage is **acyclic**.
  Generic Foundation relations (`relates_to`, `supports`, `contradicts`) remain
  freely cyclic; only the supersedes lineage is constrained. Before writing a
  supersede transition, the transaction checks that adding the edge
  (superseding revision → superseded revision) would not create a cycle:
  walking supersedes edges from the superseded record must never reach the
  superseding record. Self-supersession, direct cycles (A supersedes B, then B
  supersedes A), and indirect cycles (A supersedes B, B supersedes C, C
  supersedes A) are all rejected (exit 2, no write, no generation bump). A
  duplicate active `supersedes` tuple is rejected per Foundation's
  duplicate-active-tuple rule. Atomicity and immutable history are preserved:
  the cycle check runs inside the supersede transaction, so a rejected cycle
  leaves no revision, no event, no relation, no generation bump.

### 1.5 Search visibility (D2/D3)

- **Default `knowledge search` shows `active` records only.** This matches
  Foundation default behavior (Foundation hides `archived` by default).
- `superseded`, `archived`, and `retracted` are **hidden by default** and each
  requires an **explicit** flag: `--include-superseded`,
  `--include-archived` (Foundation flag, unchanged), `--include-retracted`.
- Explicit access **preserves status, provenance, and history**: `show`,
  `history`, and `related` work for every status without any flag; `search`
  with a flag returns the record with its `status`, `confirm_state`, and full
  revision history intact. Nothing is relabeled or re-homed.
- FTS, pagination, total counts, and the generation guard all apply the **same
  visibility filter** (Foundation rule: filters precede counts/pagination).
  A page of a filtered search counts only visible records.

> **Decision D3 (resolved by reviewer):** default = `active` only; superseded
> is not shown by default.

### 1.6 Exit codes (unchanged set)

0 success (incl. empty search); 2 invalid args/input; 3 repo/storage/missing
ID/revision/generation conflict/corrupt backup. Full-stage errors reuse these;
no new exit codes.

## 2. G-LIFE — Record lifecycle

### 2.1 Objective and non-goals

**Objective:** `superseded` and `retracted` as first-class, auditable record
states with traceable supersession and explicit retraction, built on the
existing revision + generation machinery.

**Non-goals:** no hard deletion; no automatic supersede/retract detection; no
cross-repo lifecycle; no lifecycle for kinds beyond the three Foundation kinds;
no new commands beyond the shared contract in §1.2; no change to M0–M6.

### 2.2 Exact behavioral semantics

**Status values:** `active` (default for new records), `archived`,
`superseded`, `retracted`.

**Create:** a new record may be created with `active` or `archived` only.
`superseded` or `retracted` in a create payload is rejected (exit 2): a new
record has no history to supersede or retract.

**Supersede transition** (`update` with `"status":"superseded"`):

- Payload must include `superseded_by: {"id": "<record id>", "revision": <n>}`
  and `reason` (string, 1–64 KiB, reusing `MAX_RATIONALE_BYTES`).
- Valid target per §1.4: existing revision, same store, not self, not
  retracted, else exit 3 (unknown endpoint class).
- The single write transaction (schema v2, post-migration): (1) validates
  target, (2) inserts the new immutable revision with status `superseded`,
  (3) appends a `record_lifecycle` `supersede` event, (4) appends the implicit
  `supersedes` relation (§1.4), (5) bumps `knowledge_generation`, commits. Any
  failure rolls back all five plus the migration if it was in this transaction.
- Re-superseding an already-`superseded` record to a **different** valid
  target is allowed (new revision, new target, new event, new relation id),
  provided the resulting `supersedes` lineage stays acyclic (§1.4).
  Superseding to the **same** target is rejected (exit 2, no write, no
  generation bump).
- Superseding an already-`retracted` record is rejected (exit 2): a withdrawn
  claim must not be silently revived into a replacement pointer.

**Retract transition** (`update` with `"status":"retracted"`):

- Payload must include `reason` (string, 1–64 KiB). No target is required.
- The transaction: (1) inserts the new immutable revision with status
  `retracted`, (2) appends a `record_lifecycle` `retract` event with author and
  reason, (3) bumps generation, commits.
- Retracting an already-`retracted` record is rejected (exit 2) — mirrors
  Foundation double-`unlink` rejection.
- Retracting an already-`superseded` record is allowed: supersession is a
  pointer, retraction is a withdrawal; the lifecycle event records both.

**Invalid transitions (exit 2, no write, no generation bump):** missing
`superseded_by` or `reason` on supersede; missing `reason` on retract;
self-supersession; supersede/retracted on create; double-retract; supersede of a
retracted record; supersede to the same target; any supersede that would create
a `supersedes`-lineage cycle (direct or indirect, §1.4).

**Conflicting transitions:** two concurrent `update` calls with the same
`--expected-revision` — exactly one wins (existing optimistic concurrency), the
loser fails with a revision-conflict error, no write, no generation bump.

**Rollback:** unchanged Foundation rule — one transaction, generation included.

### 2.3 Query visibility, current vs historical views

- `show`/`history`: unaffected by status; every revision is readable with its
  status and lifecycle events.
- `related`: relations are pinned to revisions, not statuses. A superseded or
  retracted record's relations remain visible at their pinned revisions;
  retracting a record does **not** retract its relations, and unlinking a
  relation does **not** change any record status. Record retraction and
  relation retraction are different lifecycles and must never be conflated.
- `search`: visibility per §1.5 (default active-only; explicit flags for the
  others; filter applies to FTS/counts/pagination/generation guard).

### 2.4 Storage and transaction invariants

- Lifecycle events and implicit `supersedes` relations commit in the same
  transaction as the revision; a failed supersede leaves no revision, no event,
  no relation, no generation bump, and no partial migration.
- `record_lifecycle` and `relation_lifecycle` are append-only; no update or
  delete path.
- FTS5 indexing unchanged: the new revision's title/body enter FTS via the
  existing head-move triggers.

### 2.5 Security and trust boundaries

- Author labels are claims, not authenticated identity.
- No lifecycle transition may be triggered by a machine process on its own;
  all transitions require an explicit human-authored `update` command.
- `superseded_by` target validation uses the same confinement/endpoint rules
  as relation endpoints (exact id + revision, same store).

### 2.6 Positive acceptance cases

1. create → supersede (valid target) → history shows rev1 active, rev2
   superseded; `related --id <old> --direction inbound --kind supersedes`
   lists the replacement; `related --id <new> --direction outbound --kind
   supersedes` lists the old record.
2. create → retract → history shows the retraction event with author/reason;
   default search hides it; `--include-retracted` reveals it with status and
   history intact.
3. superseded record with a new target → history shows both supersede events
   and two distinct `supersedes` relations. Acyclic chain (A supersedes B, then
   B supersedes C) → allowed; two lineage edges, no cycle.
4. update with status omitted → regular revision, no lifecycle event.
5. Migration: v1 store opened read-only by Full-stage binary → no migration;
   first Full-stage write migrates atomically; store is v2 afterwards.

### 2.7 Negative / adversarial cases

1. supersede with missing `superseded_by` / `reason` → exit 2, no write.
2. supersede with unknown target revision → exit 3, no write.
3. supersede self (self-reference) → exit 2. Supersede same target twice →
   exit 2. Supersede with a non-existent `superseded_by.revision` (invalid
   superseding revision) → exit 3.
4. supersede a retracted record → exit 2. Retract twice → exit 2.
5. `superseded`/`retracted` in create payload → exit 2.
6. Concurrent supersede with stale `--expected-revision` → loser exit 3, no
   write, no generation bump.
7. `archived` must not appear under `--include-retracted`-only semantics and
   must not be accepted as a value for `superseded`/`retracted` fields.
8. Migration failure injection → rollback, store still v1, tables absent,
   generation unchanged; corrupt v2 store → exit 3, fail closed.
9. Old Foundation binary opening a v2 store → refused (forward version).
10. Direct supersedes cycle (A supersedes B, then B supersedes A via a new B
    revision) → exit 2, no write. Indirect cycle (A supersedes B, B supersedes
    C, C supersedes A) → exit 2, no write. A `relates_to` cycle A↔B remains
    allowed (generic relations stay cyclic).
11. Full-stage read (e.g. `show`, `search --include-retracted`) against a v1
    store → no migration, defaults returned for v2-derived fields
    (`confirm_state` = candidate).

### 2.8 Regression evidence required

- All Foundation tests unchanged and passing (statuses active/archived only).
- New tests: each positive and negative case above; visibility matrix
  (default / `--include-superseded` / `--include-archived` /
  `--include-retracted` × four statuses); filter-applies-to-counts/pagination
  tests; a test that record retraction never touches relations and vice versa;
  migration atomicity and failure tests (§1.3).

### 2.9 STOP / GO

- **GO** when every positive case passes, every negative case fails with the
  specified exit code and no state change, migration is atomic and
  failure-safe, and Foundation regression is green.
- **STOP** on any silent status mapping, any partial lifecycle write, any
  generation bump on failure, any silent read-triggered migration, any v2 store
  openable by a Foundation binary, or any conflation of record and relation
  retraction.

### 2.10 Compatibility and migration

- Foundation stores open read-only unchanged under the Full-stage binary until
  a Full-stage write triggers migration.
- Foundation CLI behavior for active/archived records is byte-identical on v1
  stores; JSON responses gain optional fields only when Full-stage data
  exists.

## 3. G-PORT — Markdown/JSONL portability

### 3.1 Objective and non-goals

**Objective:** deterministic export of the full knowledge store (records,
revisions, evidence, relations, retraction events, lifecycle events) to a
canonical lossless format; idempotent import with an explicit merge policy,
resource limits, and atomic failure; Markdown import/export under an explicit
contract.

**Non-goals:** no incremental/partial export filtering; no cross-repo export;
no change to M0–M6.

### 3.2 Canonical JSONL format (version 1) — finalized schema (D9)

UTF-8, one JSON object per line, LF newlines, no clock data in record content.
Three line types in this order:

1. **meta** (exactly one, first line):
```json
{"type":"meta","format":"riltra-knowledge","version":1,"repo":"krepo","records":2,"relations":1,"digest":"<digest of canonical serialization of all record+relation lines, hex, lowercase>"}
```
The digest covers record + relation lines only (not the meta line). Per D6, if
no hash dependency is approved the `digest` field is omitted and integrity is
verified structurally. A file whose declared counts are not met, or whose
digest (when present) does not match, is corrupt.

2. **record** (one per record id, sorted by id, binary collation):
```json
{"type":"record","id":"d1","kind":"decision","head_revision":2,"status":"active","revisions":[{"revision":1,"title":"First","body":"alpha","status":"active","author":"a","created_unix":1792214895,"evidence":[{"path":"a.py","byte_start":0,"byte_end":5,"digest":"fnv…","git_head":"…","git_branch":"…"}],"measurement":null,"superseded_by":null,"retracted_reason":null,"retracted_author":null,"confirm_state":"candidate"},{"revision":2,"title":"Second","body":"beta","status":"active","author":"a","created_unix":1792214900,"evidence":[],"measurement":null,"superseded_by":null,"retracted_reason":null,"retracted_author":null,"confirm_state":"confirmed"}]}
```
Revisions sorted ascending. `superseded_by`, `retracted_reason`,
`retracted_author`, `confirm_state` are present only when non-null.

3. **relation** (one per relation, sorted by Foundation total order — src,
src_revision, dst, dst_revision, kind, relation_id, binary collation):
```json
{"type":"relation","relation_id":"supersedes:d2:1:d1:1","src_id":"d2","src_revision":1,"dst_id":"d1","dst_revision":1,"kind":"supersedes","author":"a","rationale":"replaced","created_unix":1792214901,"retracted":false,"retraction_author":null,"retraction_reason":null,"confirm_state":"candidate"}
```

**Lossless round-trip:** for every supported field (all Foundation fields
plus Full-stage lifecycle and provenance fields), importing an export of store
S into an empty store reproduces S's content exactly. Not round-tripped
(documented): the store's `knowledge_generation` counter and SQLite internal
state — store counters, not record data.

**Determinism:** two exports of the same store content produce byte-identical
files (no timestamps, no map ordering; `evidence` keeps captured order;
`measurement` metrics keep authored order).

### 3.3 Markdown import/export contract (D4)

Markdown alone cannot carry revision history, evidence digests, retraction
reasons, or lifecycle events. To satisfy the roadmap's Markdown import/export
requirement with the simplest possible contract, G-PORT defines
**Markdown-as-wrapper**: a Markdown document that carries a machine-readable
region of fenced code blocks, each containing exactly one canonical JSONL line
(§3.2), in canonical order, followed by human-readable prose.

```markdown
# Riltra knowledge export — krepo
Lossless machine-readable payload in fenced blocks below.

```json
{"type":"meta", …}
{"type":"record","id":"d1", …}
{"type":"relation","relation_id":"supersedes:d2:1:d1:1", …}
```

## d1 — decision (current revision 2, active)
Second — beta
```

- **Export (`--format md`)** writes the fenced canonical lines plus prose
  rendered from current head revisions. The fenced region is the payload; prose
  is decoration.
- **Import** extracts fenced ```json blocks, concatenates them, and feeds them
  to the §3.2/§3.4 JSONL parser. Prose outside fenced blocks is **presentational
  and ignored** — the canonical JSONL blocks are the **only** structured source
  of truth.
- **Round-trip claim bounds:** Markdown → import is lossless for supported
  fields **only when** the fenced blocks form a complete canonical payload (meta
  + all record + relation lines, canonical order, matching counts). Prose is not
  round-tripped. A Markdown export that omits or truncates fenced blocks is not
  a lossless export and must not be claimed as one.

> **Decision D4 (resolved by reviewer):** Markdown-wrapper import/export is
> **approved**. The canonical JSONL blocks are the only structured source of
> truth; prose is presentational. G-PORT's JSONL half and Markdown half are both
> satisfied under this contract.

**Block treatment (import):**

| Block condition | Treatment |
| --- | --- |
| No fenced ```json blocks at all | Malformed (missing payload) → exit 2, no write. |
| Fenced block with invalid JSON | Malformed → exit 2, no write. |
| Fenced block with unsupported `version` or unknown line `type` | Unsupported → exit 2, no write. |
| Duplicate canonical line (e.g. two `record` lines with the same id, or a duplicated meta) | Conflict → exit 3, atomic failure. |
| Fenced blocks not in canonical order / counts inconsistent | Malformed → exit 2, no write. |
| Prose containing JSON-like text outside fenced blocks | Ignored (presentational). |

All block-level validation follows §3.4 ordering (validation completes before
the single write transaction promotes data as authoritative).

### 3.4 Import semantics (D5)

**Input:** `knowledge import --repo R --file <path>` (file, not stdin — stdin
is capped at 1 MiB). **Resource limits (over-limit → exit 2, before any
write):** file ≤ 256 MiB; line ≤ 16 MiB; ≤ 1,000,000 record lines; ≤
2,000,000 relation lines. Parsing is streaming with these bounds, so memory
cannot be exhausted by a hostile file within the limits.

**Idempotent import:** re-importing an export of the current store content
makes no changes: no new revisions, no new relations, no generation bump, exit
0. Idempotence is detected by comparing canonical record/relation content
(ids, revision payloads, endpoint tuples), never by timestamps.

**Merge policy (existing store):**

| Case | Behavior |
| --- | --- |
| Record id absent in store | Insert all its revisions. |
| Record id present, revision set identical | Skip (idempotent). |
| Record id present, incoming is a strict superset with identical prefix | Append the new revisions. |
| Divergent (same revision number, different content) | **Conflict**: entire import fails atomically (exit 3), no partial state, diagnostics naming the conflicting id. |
| Relation tuple identical | Skip. |
| Relation tuple divergent | Conflict, atomic failure. |

**Validation before promotion:** all structural validation (JSON syntax, line
types, required fields, types, limits, duplicate ids, canonical ordering) and
all referential validation (every relation endpoint must resolve to a record
revision present in the file or already in the target store; every
`supersedes` endpoint must resolve) **complete before the single write
transaction commits and promotes the data as authoritative.** No partial import
is ever left behind; an interrupted import leaves the store unchanged because
the transaction never commits.

### 3.5 Malformed and hostile input

- Invalid JSON line, unknown line type, missing required field, wrong type,
  unknown relation kind, duplicate record id within the file, non-canonical
  ordering, unresolved relation endpoint → exit 2.
- Digest mismatch, truncated file (declared counts not met) → exit 3
  (integrity class), no state change.
- Unsupported `version` in meta → exit 2.
- Over-limit file/line/counts → exit 2 (before any write).
- Import into a store whose schema is forward-versioned → exit 3.

### 3.6 Storage and transaction invariants

- Import writes revisions, evidence, heads, FTS, lifecycle tables, relations,
  and generation bumps in the **one** transaction; export and verify are
  read-only (no mutation, no generation change).
- Relation retraction events and `record_lifecycle`/`relation_lifecycle`
  events export and re-import losslessly.

### 3.7 Security and trust boundaries

- Exported files are inert data: no code execution, no templates, no provider
  launch. Import parses strictly (duplicate/unknown field rejection per
  Foundation's strict JSON rules).
- Import never silently overwrites a conflicting record or revision; it fails.
- Repository binding: import does not rebind stores across repositories; it
  writes only into the named repo's store.

### 3.8 Positive acceptance cases

1. Export empty store → meta with zero counts (+ digest if available); import
   into empty store → no change, exit 0.
2. Export store S → import into empty store → content-equal store (all
   supported fields).
3. Re-import the same file twice → second import is a no-op (no generation
   bump).
4. Export → import → export → byte-identical files (determinism).
5. Import a superset export into a subset store → prefix revisions preserved,
   new revisions appended.
6. Markdown export → import (extracting fenced blocks) → content-equal store.
7. Referential validation: a relation referencing a revision absent from the
   file and the store → exit 2, no write.

### 3.9 Negative / adversarial cases

1. Malformed JSON line / unknown line type / duplicate id in file → exit 2,
   no state change.
2. Truncated file / digest mismatch → exit 3, no state change.
3. Divergent revision content → exit 3, atomic rollback, no partial import.
4. Unsupported version → exit 2. Over-limit file/line/counts → exit 2.
5. Markdown prose containing fake JSON outside fenced blocks → ignored; only
   fenced blocks are parsed.
6. Markdown fenced block with invalid JSON → exit 2.
7. Markdown file with no fenced blocks (prose only) → exit 2 (missing payload).
8. Markdown with duplicate record blocks → exit 3, atomic failure.

### 3.10 Regression evidence required

- All Foundation tests unchanged and passing.
- New tests: each positive/negative case above; determinism (two exports
  byte-identical); round-trip equality over every supported field; export
  read-only (store bytes + generation unchanged); Markdown round-trip.

### 3.11 STOP / GO

- **GO** when round-trip, idempotence, merge, limits, atomicity, and all
  negative cases pass, Markdown round-trip holds (if D4 adopts the contract),
  and Foundation regression is green.
- **STOP** on any silent overwrite, any partial import, any non-deterministic
  export, any generation bump on a no-op import, or any resource limit exceeded
  without a clean exit 2.

### 3.12 Compatibility and migration

- Export files are new artifacts; no Foundation file format changes.
- Import accepts only `version:1` files; a future version bump is a new
  decision, not automatic.

### 3.13 Unresolved decisions

- **D5:** per-record vs batched transactions for large imports. Resolved by
  reviewer: one transaction (recommended); batching is a future performance
  optimization, not a correctness requirement.
- **D9:** field names in the JSONL schema above are finalized pending
  ratification.

## 4. G-BACK — Backup, restore, integrity

### 4.1 Objective and non-goals

**Objective:** offline, consistent, verifiable backups of a knowledge store and
offline restore into the same registered repository, with corrupt-backup
handling that fails closed.

**Non-goals:** no cloud/remote backup; no incremental backup; no encryption;
no key management; no backup scheduling; no cross-repo restore; no claim of
cryptographic authenticity or tamper-evidence without an explicit dependency
decision (see D6).

### 4.2 Backup contract

**Command:** `knowledge backup --repo R --out <path>` (`--out` required; a DB
snapshot is not suitable for stdout).

**Consistency:** the store is WAL-mode, so a plain file copy is **not** a
consistent snapshot. Backup uses SQLite's own consistent-snapshot mechanism
(`VACUUM INTO` or the SQLite Online Backup API) to produce a single
self-contained `.db` file containing schema, meta binding, data, and FTS.
Companion files (`-wal`, `-shm`, `-journal`) are **not** part of the backup;
the snapshot is a standalone database and carries no WAL state.

**Manifest:** the backup is two files: `<path>.db` (the snapshot) and
`<path>.manifest.json`:
```json
{"format":"riltra-knowledge-backup","version":1,"repo":"krepo","canonical_root":"C:\\repo\\krepo","schema_version":2,"records":2,"relations":1,"digest":"<digest of the .db bytes, hex, lowercase>"}
```
The manifest binds the backup to the registered repository name and canonical
root, and records schema version, entity counts, and (optionally) a digest.
DB and manifest are consistent when: the DB opens, its meta binding matches
the manifest's `repo`/`canonical_root`, its `user_version` matches
`schema_version`, and its actual record/relation counts match the manifest
counts.

### 4.3 Integrity: structural vs checksum vs authenticity (D6)

Three distinct properties, none implying the others:

- **Structural integrity:** `PRAGMA integrity_check` → `ok`; schema version ≤
  current; meta binding matches; required tables present; manifest counts
  match actual counts. This detects **corruption, truncation, and structural
  damage**.
- **Checksum:** the manifest digest (when present) detects byte-level change
  of the `.db`. Requires a hash dependency — **not authorized** (D6).
- **Authenticity:** proof that a specific party produced this backup. **Not
  claimed.** There is no key, signature, or authenticated manifest.

**Tamper-detection limit (explicit):** `integrity_check` and structural
verification do **not** detect all malicious modification. A structurally
valid database with altered rows, or a swapped valid database from another
store with matching counts, passes structural checks. Only the optional
digest (checksum) detects byte-level change, and even that is not
authenticity. This delta makes no promise that all malicious modification is
detected.

### 4.4 Verification contract

`knowledge verify --repo R` (live store) and restore-time verification both
perform: `PRAGMA integrity_check` → `ok`; `user_version` ≤ current;
`user_version` == `meta.knowledge_schema_version`; meta binding matches the
registered repo (name + canonical root); required tables present; manifest
counts match actual counts; digest recomputed and compared (when present). Any
failure → exit 3 with a specific reason; the store/backup is left untouched.

### 4.5 Restore contract

**Command:** `knowledge restore --repo R --file <path> [--overwrite]`.

**Preconditions (all must hold, else exit 3 with no state change):**

- Manifest binds to the same registered repository name and canonical root as
  the target `--repo` (no cross-repo restore, no rebinding).
- Backup schema version ≤ current schema version (forward versions refused).
- Verification (§4.3) passes on the backup.
- Target store is absent, **or** `--overwrite` is given (explicit, destructive;
  without it, restore refuses rather than replace an existing store).
- Target path is not a symlink/junction (existing location-security rules).

**Atomicity and crash recovery:** restore writes the snapshot to a temp file,
verifies it, then atomically renames it into place. An interrupted restore
leaves the original store intact and no partial store (temp file is cleaned
up). Restore never deletes or truncates an existing store on failure.

**WAL/SHM/journal on restore:** the restored `.db` is standalone; WAL mode
re-initializes its own `-wal`/`-shm`/`-journal` on first access. No companion
files are copied from the backup.

**Filesystem differences (honest limits):**

- **Linux/macOS:** `rename(2)` atomically replaces the target; overwrite is
  atomic.
- **Windows:** `rename` semantics differ (replacing an existing file may fail
  without `MoveFileEx`/`ReplaceFile`); overwrite may require an explicit
  delete-then-rename and is **not** atomic against concurrent readers. No
  cross-platform identical atomicity is claimed.
- Junction/symlink at the target path is rejected on every platform (existing
  location-security rules).

### 4.6 Corrupt, tampered, missing, unsupported input

- Missing `.db` or manifest → exit 3, no state change.
- Digest mismatch / `integrity_check` failure / structural mismatch / count
  mismatch / binding mismatch → exit 3, corruption reported by name; the
  backup is not restored and the live store is untouched.
- Unsupported backup version → exit 3.
- Fail-closed is guaranteed for **detected** corruption or mismatch; undetected
  malicious modification (per §4.3) is not claimed to be caught.

### 4.7 Storage and transaction invariants

- Backup is read-only against the live store (consistent snapshot; no mutation,
  no generation change).
- Restore never modifies the M6 index, semantic generation, or graph-proof
  marker.
- Backup/restore are confined to the registered repo's knowledge directory.

### 4.8 Positive acceptance cases

1. Backup a store → verify → restore into an absent target → content-equal
   store (all supported fields; generation may differ and is documented).
2. Backup → restore → export both → byte-identical exports (round-trip through
   backup).
3. `verify` on a healthy store → exit 0, explicit OK.
4. Backup while the store is being written → consistent snapshot (no torn
   WAL), exit 0.

### 4.9 Negative / adversarial cases

1. Restore with a manifest bound to a different repo/root → exit 3.
2. Restore over an existing store without `--overwrite` → exit 3, store intact.
3. Restore a corrupt backup (flipped byte / truncated file) → exit 3, live
   store intact.
4. Restore a forward-version backup → exit 3.
5. Restore a tampered-but-structurally-valid backup (no digest) → may pass;
   documented as undetected, not a failure.
6. Restore with a symlink at the target path → exit 3.

### 4.10 Regression evidence required

- All Foundation tests unchanged and passing.
- New tests: each positive/negative case above; backup does not mutate the
  store (bytes + generation unchanged); restore is atomic against a simulated
  interruption (no partial store); DB/manifest consistency checks.

### 4.11 STOP / GO

- **GO** when backup/verify/restore round-trips, all detected-corruption cases
  fail closed, and Foundation regression is green.
- **STOP** on any partial restore, any silent overwrite, any cross-repo
  restore, any v2 store adopted by a Foundation binary, or any claim of
  cryptographic authenticity or complete tamper detection.

### 4.12 Compatibility and migration

- Backup format is a new artifact; no Foundation format changes.
- Backups bind to the same registered repo/root; no migration path.

## 5. G-PROV — Human-confirmed vs candidate

### 5.1 Objective and non-goals

**Objective:** a provenance layer distinguishing human-**confirmed** assertions
from **candidate** (machine-suggested / not-yet-reviewed) ones, with an explicit,
auditable human promotion action and no automatic promotion.

**Non-goals:** no autonomous authoring; no automatic promotion from evidence
freshness, search, or any machine process; no authenticated identity (author
labels stay claims); no change to the truth boundary (claims stay
`Unverified`); no new dependency.

### 5.2 Provenance state (D7 — default `candidate` approved)

- Every record revision and every relation carries `confirm_state` ∈
  `{candidate, confirmed}`.
- **Default is `candidate`.** Records and relations created via the CLI are
  authored but unconfirmed until a human explicitly confirms them. Authored ≠
  confirmed.
- `confirm_state` is stored as lifecycle events (`record_lifecycle` /
  `relation_lifecycle`, `event: confirm`) and derived per revision/relation.

### 5.3 Identity: explicit user assertion ≠ authenticated human identity

- Confirmation records an **explicit user assertion**: "the author label on
  this revision asserts that a human confirmed this." The confirmer identity is
  the **author label**, which is a claim, not an authenticated identity.
- No Git account, machine identity, or external authentication is involved or
  claimed. A confirmation by label `alice` means "alice asserted confirmation,"
  not "a verified human named alice confirmed."

### 5.4 Promotion (confirm) and audit semantics

- **Records:** confirm via the existing `update` path with an optional
  `confirm_state: "confirmed"` in the payload. The confirming revision is
  written by the same transaction that appends a `confirm` lifecycle event
  (author = the revision author label, `created_unix` = stored write time).
- **Relations:** confirm via the existing `link` input's optional
  `confirm_state: "confirmed"`, recorded in the same transaction as the
  relation insert.
- **Idempotency:** re-confirming an already-confirmed revision/relation is a
  no-op success (exit 0), not an error and not a new revision.
- **Confirmation conflict:** confirming with a stale `--expected-revision`
  fails with the standard revision conflict (exit 3), no write. There is no
  "confirmation conflict" beyond optimistic concurrency.
- **Audit trail:** confirmation, supersession, and retraction are immutable
  lifecycle events with author label and timestamp, readable through `history`
  (which includes lifecycle events in its output). No deletion path exists.

### 5.5 Revision, supersession, and retraction with confirmation

- Confirming creates a new revision (immutable history preserved); a confirmed
  record's confirmation is part of its audit trail.
- Superseding or retracting a **confirmed** record follows the G-LIFE contract;
  the lifecycle event records the action and author label. Confirmation does
  not block supersession or retraction.
- Source changes after confirmation affect **evidence freshness** (stale
  reporting), never `confirm_state`: a confirmed claim whose cited source
  changes stays confirmed and becomes stale — the two axes are orthogonal.

### 5.6 Claim verification vs evidence freshness

Unchanged Foundation boundary: `verification` is always `Unverified`;
`support` is `unknown`/`unverified` by evidence count. `confirm_state` is a
**provenance** fact (a human label asserted review), not a verification of
truth. **A confirmed record never becomes `Current`, never changes
`verification` or `support`, and never independently proves its claim.** No
command maps confirmed → verified, or fresh evidence → confirmed.

### 5.7 Security and trust boundaries

- Author labels are claims, not authenticated identity; confirmation by a label
  is a human-claimed action, not a machine-authenticated one.
- No machine process may promote or demote `confirm_state`.
- Confirmation is per revision/relation; it does not certify endpoints.

### 5.8 Positive acceptance cases

1. Create record (candidate) → confirm → `show`/`search` report
   `confirm_state: "confirmed"`; history shows the confirming revision.
2. Confirm a relation → `related` reports `confirm_state: "confirmed"`.
3. Confirm with evidence present → still `verification: "Unverified"`.
4. Re-confirm → idempotent no-op, no new revision.

### 5.9 Negative / adversarial cases

1. Any attempt (via import, restore, or a future machine path) to set
   `confirm_state` without an explicit human `update`/`link` → rejected or
   ignored; never auto-set.
2. Confirming must not change `verification` or `support` (stays `Unverified`).
3. Confirming a record whose source changed → confirmation survives; evidence
   reports stale.
4. Confirming with stale `--expected-revision` → exit 3, no write.

### 5.10 Regression evidence required

- All Foundation tests unchanged and passing.
- New tests: each positive/negative case above; a test that fresh evidence +
  confirmation still yields `Unverified`; a test that confirmation appears in
  history output.

### 5.11 STOP / GO

- **GO** when confirmation is explicit-only, idempotent, auditable, and
  orthogonal to verification and freshness, and Foundation regression is green.
- **STOP** on any automatic promotion, any conflation of provenance with
  verification, or any deletion of lifecycle events.

### 5.12 Compatibility and migration

- Foundation records/relations read as `candidate` (default) — additive field
  in JSON output only.
- No schema migration beyond §1.3 (additive lifecycle tables).

## 6. Interface contracts — finalized command surface (D8)

**No production CLI implementation is authorized by this delta.** The proposed
surface:

```
riltra knowledge create --repo R --input -                     (unchanged)
riltra knowledge show --repo R --id ID [--revision N]          (unchanged)
riltra knowledge update --repo R --id ID --expected-revision N --input -   (unchanged shape; payload gains optional superseded_by, reason, confirm_state)
riltra knowledge history --repo R --id ID [--limit N] [--offset N] [--generation G]   (unchanged)
riltra knowledge search --repo R --query TEXT [--kind K] [--include-superseded] [--include-archived] [--include-retracted] [--limit N] [--offset N] [--generation G]
riltra knowledge link --repo R --input -                      (unchanged shape; input gains optional confirm_state)
riltra knowledge unlink --repo R --relation ID --input -       (unchanged)
riltra knowledge related --repo R --id ID [--revision N] [--direction inbound|outbound] [--include-retracted] [--limit N] [--offset N] [--generation G]   (unchanged)
riltra knowledge export --repo R [--format jsonl|md] [--out PATH]   (new)
riltra knowledge import --repo R --file PATH                   (new)
riltra knowledge backup --repo R --out PATH                    (new)
riltra knowledge restore --repo R --file PATH [--overwrite]    (new)
riltra knowledge verify --repo R                               (new)
```

**Compatibility rules:**

- Foundation flags/flags-with-values (`--flag value` and `--flag=value`) are
  preserved; new flags follow the same parsing.
- `search` without new flags is byte-identical to Foundation (active-only).
- New commands are additive; removing them leaves Foundation intact.
- JSON responses gain optional fields only when Full-stage data exists;
  Foundation field names and values are unchanged.

## 7. Decisions D1–D9 — resolutions

- **D1 — schema:** schema v2 with guarded migration (§1.3). Resolved by
  reviewer.
- **D2/D3 — lifecycle/search:** implicit atomic `supersedes` relation kept, made exact (§1.4), and the supersedes lineage is **acyclic** (generic Foundation relations stay cyclic); search default = active only, explicit flags for the rest (§1.5). Resolved by reviewer.
- **D4 — Markdown:** Markdown-wrapper import/export **approved**; canonical JSONL blocks are the only structured source of truth, prose is presentational; missing/duplicate/malformed/unsupported block treatment and round-trip bounds defined (§3.3). Resolved by reviewer.
- **D5 — import:** one transaction, resource limits (§3.4), validation before
  promotion. Resolved by reviewer.
- **D6 — backup:** SQLite-consistent backup; structural vs checksum vs
  authenticity distinguished; no hash dependency; tamper-detection limits
  explicit (§4.3). Resolved by reviewer.
- **D7 — provenance:** default `candidate` approved; explicit-assertion vs
  authenticated identity distinguished (§5.3). Resolved by reviewer.
- **D8 — command surface:** finalized in §6.
- **D9 — JSONL/manifest schemas:** finalized in §3.2 and §4.2, with examples.

**Still open:** D4 scope (Markdown-wrapper vs JSONL-only), and ratification
of the finalized schemas (D9).

## 8. Dependency and schema implications

- **Dependencies:** none added by this delta. A SHA-256 crate for digests (D6
  alternative) is **not** authorized here and would require a separate
  dependency decision with licensing review and acceptance tests.
- **Schema:** schema v2 = Foundation tables (unchanged) + `record_lifecycle` +
  `relation_lifecycle`, created by the guarded migration (§1.3). Foundation
  stores (v1) open read-only under the Full-stage binary without migration.
- **Existing contracts preserved:** statuses `active`/`archived`; the single
  update transition contract; immutable revisions; expected-revision
  concurrency; generation guards; evidence freshness; `Unverified` truth
  boundary; FTS5 lexical search; revision-pinned relations; relation-retraction
  events; M6 isolation; root binding; exit codes; byte limits; deterministic
  JSON.

## 9. Security and correctness risks (residual)

- **Lifecycle conflation:** mapping `superseded`/`retracted` to `archived` or
  conflating record retraction with relation retraction. Mitigated by distinct
  states and tests (G-LIFE §2.7).
- **Silent read-triggered migration:** a read upgrading a v1 store. Mitigated
  by migration-on-write-only (§1.3).
- **Partial import / restore:** half-applied state. Mitigated by single
  transaction and temp-file + atomic rename; Windows overwrite atomicity is
  honestly limited (§4.5).
- **Silent overwrite / cross-repo rebinding:** mitigated by fail-atomic merge,
  `--overwrite` precondition, and manifest binding checks.
- **Trust overreach:** authenticated identity, automatic verification,
  cryptographic authenticity, complete tamper detection, atomic tree
  snapshots — explicitly not claimed (§4.3, §5.3, §5.6).
- **Resource exhaustion:** import limits (§3.4) prevent hostile-file memory
  growth within documented bounds.

## 10. Areas not specific enough for implementation

- **D4 scope:** whether the Markdown-wrapper contract satisfies the roadmap's
  Markdown import/export target or whether G-PORT is re-scoped to JSONL-only
  with Markdown as a separate gate.
- **G-PROV blueprint:** the approved design spec references a "blueprint" for
  provenance semantics, but no blueprint document exists. §5 is a stand-in that
  may be refined by the blueprint.
- **Import batch size / performance bounds:** one transaction is specified;
  large-store import performance is untested.
- **Windows overwrite semantics:** exact `MoveFileEx`/`ReplaceFile` behavior
  is platform-detail work for implementation, not spec.

## 11. Read-only consistency review (vs Foundation + design spec)

- Status set: Foundation `active`/`archived` preserved; `superseded`/`retracted`
  additive; create rejects the new statuses.
- Transition contract: unchanged single `update --expected-revision` path; no
  new lifecycle commands.
- Immutable revisions, generation guards, exit codes, byte limits, FTS5,
  pinned relations, relation retraction events, evidence freshness,
  `Unverified`, M6 isolation, root binding: all preserved verbatim.
- New data is additive (schema v2 tables, one relation kind, optional JSON
  fields); no Foundation field is removed or redefined.
- Read-only rules: export and verify do not mutate; import/restore are explicit
  writes with the same transaction and failure-atomicity rules as Foundation;
  migration is write-triggered only.
- This delta specifies contracts only; it does not authorize implementation.
