# M7 Full-stage Specification Delta

Status: **DRAFT FOR REVIEW**. This delta extends the approved design spec
(`2026-10-16-m7-native-structured-knowledge-graph-design.md`) and the approved
Foundation plan (`2026-10-16-m7-foundation.md`). It specifies contracts only;
it authorizes **no** implementation. Every clause below is a proposed contract
requiring reviewer ratification. Where alternatives exist, a recommendation is
given with its trade-off; a recommendation is not an approval.

## 0. Scope and ordering

Four gates, each independently implementable and independently verifiable:

- **G-LIFE** — `superseded` / `retracted` record lifecycle.
- **G-PORT** — deterministic JSONL (canonical) and Markdown (lossy) export,
  JSONL import with idempotent merge and atomic failure.
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

### 1.3 New lifecycle data (additive tables, no DDL on existing tables)

Foundation tables (`records`, `heads`, `evidence`, `relations`, `retractions`,
`measurements`, `metric_values`, `meta`, `knowledge_fts`) are untouched — no
column additions, no schema migration, `KNOWLEDGE_SCHEMA_VERSION` stays `1`.
Full-stage state lives in new additive tables created lazily by Full-stage
commands inside the same write transaction as the revision they describe:

- `record_lifecycle` — append-only events keyed by `(id, revision)`:
  `event` ∈ {`supersede`, `retract`, `confirm`}, `target_id`, `target_revision`,
  `reason`, `author`, `created_unix`. One row per lifecycle event.
- `relation_lifecycle` — append-only events keyed by `relation_id` with
  `event` = `confirm`, `author`, `created_unix` (relation retraction already
  exists as `retractions` in Foundation and is unchanged).

`confirm_state` for a record revision is **derived**: the latest `confirm` or
absence of events in `record_lifecycle`. Default is `candidate` (see G-PROV).

> **Decision D1 (needs approval):** additive lazy tables under schema v1
> vs a schema-v2 migration. Recommended: additive lazy tables — Foundation
> stores open byte-identically unchanged, no migration path, no row rewrites.

### 1.4 New relation kind

G-LIFE introduces one new relation kind: `supersedes`. Foundation kinds
(`relates_to`, `supports`, `contradicts`) are unchanged. Relation kinds are
code-validated (`validate_relation_kind`); no DB constraint changes.

> **Decision D2 (needs approval):** new kind `supersedes`, written
> implicitly by the supersede transaction (recommended: implicit, atomic with
> the status change, deterministic relation id
> `supersedes:<src>:<src_rev>:<dst>:<dst_rev>`) vs an explicitly authored link
> in a second command (rejected: non-atomic, two generation bumps).

### 1.5 Search visibility

- Default `knowledge search`: records whose head status is `active` or
  `superseded` (superseded records remain informative pointers to a
  replacement; they are not withdrawn).
- `--include-archived` (Foundation flag): adds `archived`.
- `--include-retracted` (new Full-stage flag): adds `retracted`.
- `knowledge show` / `knowledge history` work for any status, unchanged.

> **Decision D3 (needs approval):** visibility defaults above. Trade-off:
> showing `superseded` by default keeps replacements discoverable; the
> alternative (hide superseded by default) would hide valid replacement
> pointers from plain search.

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
- The superseding target must be an existing record revision in the same
  store (same endpoint-existence rule as relation endpoints: exact id +
  revision, else exit 3 `unknown relation endpoint`-class error).
- Self-supersession (`superseded_by.id` == record id) is rejected (exit 2).
- The single write transaction: (1) validates target, (2) inserts the new
  immutable revision with status `superseded`, (3) appends a `record_lifecycle`
  `supersede` event, (4) appends an implicit `supersedes` relation
  (src = superseded revision, dst = target revision, kind `supersedes`,
  author = revision author, rationale = reason), (5) bumps `knowledge_generation`,
  commits. Any failure rolls back all five.
- Re-superseding an already-`superseded` record to a **different** target is
  allowed (new revision, new target, new event). Superseding to the **same**
  target is rejected (exit 2, no write, no generation bump).
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
retracted record; supersede to the same target.

**Conflicting transitions:** two concurrent `update` calls with the same
`--expected-revision` — exactly one wins (existing optimistic concurrency), the
loser fails with a revision-conflict error, no write, no generation bump.

**Rollback:** unchanged Foundation rule — one transaction, generation included.

### 2.3 Query visibility, current vs historical views

- `show`/`history`: unaffected by status; every revision is readable.
- `related`: relations are pinned to revisions, not statuses. A superseded or
  retracted record's relations remain visible at their pinned revisions;
  retracting a record does **not** retract its relations, and unlinking a
  relation does **not** change any record status. Record retraction and
  relation retraction are different lifecycles and must never be conflated.
- `search`: visibility per §1.5.

### 2.4 Storage and transaction invariants

- Lifecycle events and implicit `supersedes` relations commit in the same
  transaction as the revision; a failed supersede leaves no revision, no event,
  no relation, no generation bump.
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
   superseded; `related --id X --direction inbound --kind supersedes` shows the
   target; `related --id Y --direction outbound --kind supersedes` shows X.
2. create → retract → history shows the retraction event with author/reason;
   default search hides it; `--include-retracted` reveals it.
3. superseded record with a new target → history shows both supersede events.
4. update with status omitted → regular revision, no lifecycle event.

### 2.7 Negative / adversarial cases

1. supersede with missing `superseded_by` / `reason` → exit 2, no write.
2. supersede with unknown target revision → exit 3, no write.
3. supersede self → exit 2. Supersede same target twice → exit 2.
4. supersede a retracted record → exit 2. Retract twice → exit 2.
5. `superseded`/`retracted` in create payload → exit 2.
6. Concurrent supersede with stale `--expected-revision` → loser exit 3, no
   write, no generation bump.
7. `archived` must not appear under `--include-retracted`-only semantics and
   must not be accepted as a value for `superseded`/`retracted` fields.

### 2.8 Regression evidence required

- All Foundation tests unchanged and passing (statuses active/archived only).
- New tests: each positive and negative case above, plus visibility matrix
  (default / `--include-archived` / `--include-retracted` × four statuses),
  and a test that record retraction never touches relations and vice versa.

### 2.9 STOP / GO

- **GO** when every positive case passes, every negative case fails with the
  specified exit code and no state change, and Foundation regression is green.
- **STOP** on any silent status mapping, any partial lifecycle write, any
  generation bump on failure, or any conflation of record and relation
  retraction.

### 2.10 Compatibility and migration

- Foundation stores open unchanged; lifecycle tables are absent until first
  Full-stage write.
- Foundation CLI behavior for active/archived records is byte-identical;
  JSON responses gain optional fields only when Full-stage data exists.

## 3. G-PORT — Markdown/JSONL portability

### 3.1 Objective and non-goals

**Objective:** deterministic export of the full knowledge store (records,
revisions, evidence, relations, retraction events, lifecycle events) to a
canonical format; idempotent import with an explicit merge policy and atomic
failure.

**Non-goals:** no Markdown import; no incremental/partial export filtering; no
cross-repo export; no human-authored Markdown round-trip; no new file format
beyond the JSONL schema below; no change to M0–M6.

### 3.2 Format contract (version 1)

**Canonical format: JSONL**, UTF-8, one JSON object per line, LF newlines,
no clock data in record content. Three line types in this order:

1. **meta** (exactly one, first line):
   `{"type":"meta","format":"riltra-knowledge","version":1,"repo":"<registered name>","records":<n>,"relations":<m>,"digest":"<digest of the canonical serialization of all record and relation lines, hex, lowercase>"}`
   The digest covers record + relation lines only (not the meta line), making
   the file self-verifying. **The digest algorithm is SHA-256, which requires a
   hash dependency; per D6, if no hash dependency is approved the `digest`
   field is omitted and file integrity is verified structurally (counts,
   ordering, duplicate-id, and re-import validation).** A manifest whose
   declared counts are not met, or whose digest (when present) does not match,
   is a corrupt file.
2. **record** (one per record id, sorted by id with binary collation):
   `{"type":"record","id":"…","kind":"…","head_revision":<n>,"status":"…","relations_count":<k>,"revisions":[ {revision,title,body,status,author,created_unix,evidence[],measurement?,superseded_by?,retracted_reason?,retracted_author?,confirm_state?}, … ]}` — revisions sorted ascending.
3. **relation** (one per relation, sorted by the Foundation total order —
   src, src_revision, dst, dst_revision, kind, relation_id, binary collation):
   `{"type":"relation","relation_id":"…","src_id":"…","src_revision":<n>,"dst_id":"…","dst_revision":<n>,"kind":"…","author":"…","rationale":"…","created_unix":<n>,"retracted":<bool>,"retraction_author":…,"retraction_reason":…,"confirm_state":"…"}`.

**Lossless round-trip:** for every supported field (all Foundation fields
plus Full-stage lifecycle and provenance fields), importing an export of store
S into an empty store reproduces S's content exactly. Not round-tripped (and
documented as such): the store's `knowledge_generation` counter and SQLite
internal state — these are store counters, not record data.

**Determinism:** two exports of the same store content produce byte-identical
files (no timestamps, no map ordering — all orderings are the canonical orders
above; `evidence` keeps captured order; `measurement` metrics keep authored
order).

### 3.3 Markdown export (lossy, non-round-trip)

Optional `--format md`: a fixed-template human-readable rendering of **current
head revisions only**. It is explicitly **not** importable and must not be
described as a round-trip format. Markdown cannot represent revision history,
evidence digests, retraction reasons, or lifecycle events; those are omitted.
Every Markdown export carries a header line stating it is a lossy view.

> **Decision D4 (needs approval):** include lossy Markdown export (recommended:
> yes, clearly marked lossy, no import) vs JSONL-only (simpler, smaller
> surface).

### 3.4 Import semantics

**Input:** `knowledge import --repo R --file <path>` (file, not stdin — stdin
is capped at 1 MiB, too small for real stores). File cap: 64 MiB, 16 MiB per
line; over-limit → exit 2.

**Idempotent import:** re-importing an export of the current store content
makes no changes: no new revisions, no new relations, no generation bump,
exit 0. Idempotence is detected by comparing canonical record/relation content
(ids, revision payloads, endpoint tuples), never by timestamps.

**Merge policy (existing store):**

| Case | Behavior |
| --- | --- |
| Record id absent in store | Insert all its revisions (one transaction per record or batched — see D5). |
| Record id present, revision set identical | Skip (idempotent). |
| Record id present, incoming is a strict superset with identical prefix | Append the new revisions in one transaction. |
| Divergent (same revision number, different content) | **Conflict**: entire import fails atomically (exit 3), no partial state, diagnostics naming the conflicting id. |
| Relation tuple already present with identical payload | Skip. |
| Relation tuple divergent (same tuple, different payload) | Conflict, atomic failure. |

**Atomicity:** the whole import is validated and applied in one transaction per
batch; any conflict or malformed line rolls back everything. A partial import
is never left behind. Interrupted import (process killed mid-write) leaves the
store unchanged because the transaction never commits.

### 3.5 Malformed and hostile input

- Invalid JSON line, unknown line type, missing required field, wrong type,
  unknown relation kind, duplicate record id within the file → exit 2.
- Digest mismatch, truncated file (declared counts not met) → exit 3
  (integrity class), no state change.
- Unsupported `version` in meta → exit 2.
- Oversized file/line → exit 2.
- Import into a store whose schema is forward-versioned → exit 3.

### 3.6 Storage and transaction invariants

- Import writes revisions, evidence, heads, FTS, lifecycle tables, relations,
  and generation bumps in the same transaction(s) as Foundation updates.
- Export is a read-only operation: one read transaction per consistent view;
  export never mutates the store (read-only open, no generation change).
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

1. Export empty store → meta with zero counts + digest of empty content;
   import into empty store → no change, exit 0.
2. Export store S → import into empty store → content-equal store (all
   supported fields).
3. Re-import the same file twice → second import is a no-op (no generation
   bump).
4. Export → import → export → byte-identical files (determinism).
5. Import a superset export into a subset store → prefix revisions preserved,
   new revisions appended.

### 3.9 Negative / adversarial cases

1. Malformed JSON line / unknown line type / duplicate id in file → exit 2,
   no state change.
2. Truncated file / digest mismatch → exit 3, no state change.
3. Divergent revision content → exit 3, atomic rollback, no partial import.
4. Unsupported version → exit 2. Oversized file/line → exit 2.
5. Markdown file passed to import → rejected (exit 2), no partial import.

### 3.10 Regression evidence required

- All Foundation tests unchanged and passing.
- New tests: each positive/negative case above; determinism (two exports
  byte-identical); round-trip equality assertions over every supported field;
  a test that export is read-only (store bytes and generation unchanged).

### 3.11 STOP / GO

- **GO** when round-trip, idempotence, merge, atomicity, and all negative cases
  pass, and Foundation regression is green.
- **STOP** on any silent overwrite, any partial import, any non-deterministic
  export, or any generation bump on a no-op import.

### 3.12 Compatibility and migration

- Export files are new artifacts; no Foundation file format changes.
- Import accepts only `version:1` files; a future version bump is a new
  decision, not automatic.
- Markdown is export-only and never imported.

### 3.13 Unresolved decisions

- **D5:** per-record vs batched transactions for large imports. Recommended:
  validate everything first, then apply in one transaction (simplest, truly
  atomic); batching is a performance optimization, not a correctness
  requirement.
- Exact field names in the JSONL schema are drafts pending ratification.

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

**Command:** `knowledge backup --repo R --out <path>` (default stdout is not
suitable for a DB snapshot; `--out` is required for backup).

**Consistency:** the store is WAL-mode, so a plain file copy is **not** a
consistent snapshot. Backup uses SQLite's own consistent-snapshot mechanism
(`VACUUM INTO` or the SQLite Online Backup API) to produce a single
self-contained `.db` file containing schema, meta binding, data, and FTS.
Companion files (`-wal`, `-shm`, `-journal`) are not part of the backup; the
snapshot is a standalone database.

**Manifest:** the backup is two files: `<path>.db` (the snapshot) and
`<path>.manifest.json` containing `format: "riltra-knowledge-backup"`,
`version: 1`, `repo` (registered name), `canonical_root` (bound root),
`schema_version: 1`, `records`, `relations`, and a digest of the `.db` bytes.

> **Decision D6 (needs approval):** digest algorithm. Adding a SHA-256 crate
> is a new dependency and is **not** authorized by this delta. Recommended:
> integrity verification via SQLite `PRAGMA integrity_check` + structural
> verification (schema version, meta binding, table presence, manifest counts)
> — this detects corruption and truncation honestly. Tamper-evidence /
> cryptographic authenticity would require a separate dependency decision
> (STOP-for-approval) and is explicitly **not** claimed here.

### 4.3 Verification contract

`knowledge verify --repo R` (live store) and restore-time verification of the
backup both perform: `PRAGMA integrity_check` → `ok`; `user_version` ≤ current;
meta binding matches the registered repo (name + canonical root); required
tables present; manifest counts match actual counts; digest recomputed and
compared (when a digest is present). Any failure → exit 3 with a specific
reason; the store/backup is left untouched.

### 4.4 Restore contract

**Command:** `knowledge restore --repo R --file <path> [--overwrite]`.

**Preconditions (all must hold, else exit 3 with no state change):**

- The backup manifest binds to the same registered repository name and
  canonical root as the target `--repo` (no cross-repo restore, no rebinding).
- Backup schema version ≤ current schema version (forward versions refused).
- Verification (§4.3) passes.
- Target store is absent, **or** `--overwrite` is given (explicit, destructive;
  without it, restore refuses rather than replace an existing store).

**Atomicity:** restore writes the snapshot to a temp file, verifies it, then
atomically renames it into place. An interrupted restore leaves the original
store intact (temp file is cleaned up). Restore never deletes or truncates an
existing store on failure.

> **Filesystem honesty:** atomic rename is not a claim of filesystem-wide
> race freedom. Restore while another process holds the store open is not
> guarded beyond SQLite's own locking and may fail with a storage error; no
> TOCTOU protection is claimed.

### 4.5 Corrupt, tampered, missing, unsupported input

- Missing `.db` or manifest → exit 3, no state change.
- Digest mismatch / `integrity_check` failure / structural mismatch → exit 3,
  corruption reported by name; the backup is not restored and the live store
  is untouched.
- Unsupported backup version → exit 3.
- Tampering detection is limited to corruption detection (above); authenticity
  is not claimed (see D6).

### 4.6 Storage and transaction invariants

- Backup is read-only against the live store (consistent snapshot; no mutation,
  no generation change, no lock held beyond the snapshot).
- Restore never modifies the M6 index, semantic generation, or graph-proof
  marker.
- Backup/restore are confined to the registered repo's knowledge directory.

### 4.7 Positive acceptance cases

1. Backup a store → verify → restore into an absent target → content-equal
   store (all supported fields; generation may differ and is documented).
2. Backup → restore → export both → byte-identical exports (round-trip through
   backup).
3. `verify` on a healthy store → exit 0, explicit OK.

### 4.8 Negative / adversarial cases

1. Restore with a manifest bound to a different repo/root → exit 3.
2. Restore over an existing store without `--overwrite` → exit 3, store intact.
3. Restore a corrupt backup (flipped byte / truncated file) → exit 3, live
   store intact.
4. Restore a forward-version backup → exit 3.
5. Backup while the store is being written → consistent snapshot (no torn
   WAL), exit 0.

### 4.9 Regression evidence required

- All Foundation tests unchanged and passing.
- New tests: each positive/negative case above; a test that backup does not
  mutate the store (bytes + generation unchanged); a test that restore is
  atomic against a simulated interruption (no partial store).

### 4.10 STOP / GO

- **GO** when backup/verify/restore round-trips, all negative cases fail
  closed, and Foundation regression is green.
- **STOP** on any partial restore, any silent overwrite, any cross-repo
  restore, or any claim of cryptographic authenticity.

### 4.11 Compatibility and migration

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

### 5.2 Provenance state

- Every record revision and every relation carries `confirm_state` ∈
  `{candidate, confirmed}`.
- **Default is `candidate`.** Records and relations created via the CLI are
  authored but unconfirmed until a human explicitly confirms them. Authored ≠
  confirmed.
- `confirm_state` is stored as lifecycle events (`record_lifecycle` /
  `relation_lifecycle`, `event: confirm`) and derived per revision/relation.

> **Decision D7 (needs approval):** default `candidate` for CLI-authored
> records. Trade-off: defaulting to `candidate` means no existing Foundation
> record is silently promoted (safe); the alternative (authored = confirmed)
> would treat the CLI author as a confirter by default, which the trust
> boundary explicitly warns against.

### 5.3 Promotion (confirm)

- **Records:** confirm via the existing `update` path with an optional
  `confirm_state: "confirmed"` in the payload. The confirming revision is
  written by the same transaction that appends a `confirm` lifecycle event.
  The `author` of that revision is the **claimed** confirmer.
- **Relations:** confirm via the existing `link` input's optional
  `confirm_state: "confirmed"`, recorded in the same transaction as the
  relation insert.
- **No other promotion path exists.** Evidence freshness, search hits,
  imports, restores, and machine processes never change `confirm_state`.
- Re-confirming an already-confirmed revision/relation is a no-op success
  (idempotent), not an error.

### 5.4 Revision, supersession, and retraction with confirmation

- Confirming creates a new revision (immutable history preserved); a confirmed
  record's confirmation is part of its audit trail.
- Superseding or retracting a **confirmed** record follows the G-LIFE contract;
  the lifecycle event records the action and author. Confirmation does not
  block supersession or retraction.
- Source changes after confirmation affect **evidence freshness** (stale
  reporting), never `confirm_state`: a confirmed claim whose cited source
  changes stays confirmed and becomes stale — the two axes are orthogonal.

### 5.5 Claim verification vs evidence freshness

Unchanged Foundation boundary: `verification` is always `Unverified`;
`support` is `unknown`/`unverified` by evidence count. `confirm_state` is a
**provenance** fact (a human reviewed this), not a verification of truth. No
command maps confirmed → verified, or fresh evidence → confirmed.

### 5.6 Audit trail

Confirmation, supersession, and retraction are immutable lifecycle events with
author and timestamp, readable through `history` (which includes lifecycle
events in its output). No deletion path exists for these events.

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
4. Re-confirm → idempotent no-op.

### 5.9 Negative / adversarial cases

1. Any attempt (via import, restore, or a future machine path) to set
   `confirm_state` without an explicit human `update`/`link` → rejected or
   ignored; never auto-set.
2. Confirming must not change `verification` or `support`.
3. Confirming a record whose source changed → confirmation survives; evidence
   reports stale.

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
- No schema migration (additive lifecycle tables, per §1.3).

## 6. Cross-gate acceptance matrix

| Gate | Positive cases | Negative cases | Regression | STOP/GO |
| --- | --- | --- | --- | --- |
| G-LIFE | §2.6 | §2.7 | §2.8 | §2.9 |
| G-PORT | §3.8 | §3.9 | §3.10 | §3.11 |
| G-BACK | §4.7 | §4.8 | §4.9 | §4.10 |
| G-PROV | §5.8 | §5.9 | §5.10 | §5.11 |

Each gate's GO requires all its listed evidence plus an intact Foundation
suite (498 tests at the time of this delta). No gate requires the others.

## 7. New decisions requiring approval

- **D1:** additive lazy lifecycle tables under schema v1 (recommended) vs
  schema-v2 migration.
- **D2:** new relation kind `supersedes`, written implicitly and atomically
  with the supersede transaction (recommended) vs explicit second command.
- **D3:** search visibility defaults (default = active + superseded;
  `--include-archived`; `--include-retracted`).
- **D4:** include lossy, non-round-trip Markdown export (recommended, clearly
  marked) vs JSONL-only.
- **D5:** import applies in one transaction (recommended) vs batched
  transactions.
- **D6:** digest algorithm for backup manifests and export meta lines. Adding
  a SHA-256 crate is a new dependency and is **not** authorized by this delta.
  Recommended: integrity via SQLite `PRAGMA integrity_check` + structural
  verification (schema version, meta binding, table presence, manifest counts)
  for backups, and structural + re-import validation for exports (the `digest`
  field is omitted when no hash dependency exists). This detects corruption and
  truncation honestly. Tamper-evidence / cryptographic authenticity would
  require a separate dependency decision (STOP-for-approval) and is explicitly
  **not** claimed here.
- **D7:** `confirm_state` defaults to `candidate` for CLI-authored data
  (recommended) vs `confirmed`.
- **D8:** new CLI commands introduced by this delta: `knowledge export`,
  `knowledge import`, `knowledge backup`, `knowledge restore`, `knowledge
  verify` (and flags `--include-retracted`, `--format md`, `--overwrite`).
- **D9:** JSONL schema field names and manifest schema are drafts pending
  ratification.

## 8. Dependency and schema implications

- **Dependencies:** none added by this delta. A SHA-256 crate for backup
  digests (D6 alternative) is **not** authorized here and would require a
  separate dependency decision with licensing review and acceptance tests.
- **Schema:** no DDL on Foundation tables; `KNOWLEDGE_SCHEMA_VERSION` stays
  `1`; new additive tables (`record_lifecycle`, `relation_lifecycle`) are
  created lazily by Full-stage commands. Foundation stores open unchanged.
- **Existing contracts preserved:** statuses `active`/`archived`; the single
  update transition contract; immutable revisions; expected-revision
  concurrency; generation guards; evidence freshness; `Unverified`
  truth boundary; FTS5 lexical search; revision-pinned relations;
  relation-retraction events; M6 isolation; root binding; exit codes; byte
  limits; deterministic JSON.

## 9. Security and correctness risks

- **Lifecycle conflation:** mapping `superseded`/`retracted` to `archived` or
  conflating record retraction with relation retraction. Mitigated by explicit
  distinct states and tests (G-LIFE §2.7).
- **Partial import / restore:** leaving a half-applied state. Mitigated by
  single-transaction application and temp-file + atomic rename.
- **Silent overwrite:** import or restore replacing conflicting data.
  Mitigated by fail-atomic merge policy and `--overwrite` precondition.
- **Cross-repo rebinding:** restore/import adopting another store. Mitigated
  by manifest repo + canonical-root binding checks.
- **Trust overreach:** claiming authenticated identity, automatic verification,
  cryptographic authenticity, or atomic tree snapshots. Explicitly out of
  scope and called out per gate.
- **Clock nondeterminism:** export determinism forbids timestamps in record
  content; the manifest digest makes files self-verifying.

## 10. Areas not specific enough for implementation

- **G-PROV blueprint:** the approved design spec references a "blueprint" for
  provenance semantics, but no blueprint document exists. This delta defines
  the provenance contract (§5) as a stand-in; the blueprint may refine
  confirmer identity or confirmation UX and could change D7.
- **Relation confirmation UX:** how a human confirms a relation in practice
  (via `link` payload vs a dedicated command) is proposed as the `link` payload
  field (§5.3); a dedicated command remains possible under the §1.2 rule.
- **JSONL field names / manifest schema:** drafts in §3.2, pending
  ratification (D9).
- **Backup digest:** deferred to D6; without it, tamper-evidence is limited to
  corruption detection.
- **Import batch size / performance bounds:** D5 recommends a single
  transaction; large-store import performance is untested here.

## 11. Read-only consistency review (vs Foundation + design spec)

- Status set: Foundation `active`/`archived` preserved; `superseded`/`retracted`
  additive; create rejects the new statuses (Foundation had no such states).
- Transition contract: unchanged single `update --expected-revision` path;
  no new lifecycle commands.
- Immutable revisions, generation guards, exit codes, byte limits, FTS5,
  pinned relations, relation retraction events, evidence freshness,
  `Unverified`, M6 isolation, root binding: all preserved verbatim.
- New data is additive (tables, relation kind, JSON fields); no Foundation
  field is removed or redefined.
- Read-only rules: export and verify do not mutate; import/restore are explicit
  writes with the same transaction and failure-atomicity rules as Foundation.
- This delta specifies contracts only; it does not authorize implementation.
