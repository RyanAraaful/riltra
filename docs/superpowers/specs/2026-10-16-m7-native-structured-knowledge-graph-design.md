# M7 — Native Structured Knowledge Graph

Status: **DRAFT FOR REVIEW (REVISED)**. Architecture direction APPROVED;
reviewer decision REVISE. This revision clarifies contracts only. No
implementation plan or product implementation is included.

## Baseline and intent

Branch: `m7-native-structured-knowledge-graph`, created from integrated master
`105b04252c531f77e4db088e80c127de6c344b18`. [PR #1](https://github.com/RyanAraaful/riltra/pull/1)
preserves every M6 commit, including closure documentation `f80f687`.
[Master CI #38900215738](https://github.com/RyanAraaful/riltra/actions/runs/38900215738)
passed fmt, check, Clippy, tests and CLI smoke on Windows/Ubuntu/macOS
(402/404/404 tests). M6 remains PASS — CLOSED; its proof rules are not reopened.

Riltra users and coding agents need durable answers to why a decision was made,
which constraints apply, and what a recorded measurement actually establishes.
M7 stores these authored claims with explicit relationships, revision history
and source evidence. A valid record is not proof that its contents are true.

The user selected these boundaries:

- Records are authored through the native CLI; SQLite is authoritative.
- Each registered repository has a knowledge database separate from its code index.
- The first scope is decisions, constraints and benchmarks, immutable revisions,
  provenance, lexical search and bounded one-hop relationships.
- No MCP, embeddings, autonomous authoring, remote service or cross-repository graph.

Engineering priority: Correctness → Security → DRY/KISS → Performance.

## M7 scope split: Foundation vs Full Acceptance

M7 Foundation is the only implementation scope after this spec is approved.
M7 Full Acceptance is a later stage with its own checkpoint and GO decision.
Foundation work must not expand to absorb Full capabilities in one task.

**M7 Foundation (implementation scope):**

- Record kinds `decision`, `constraint`, `benchmark`.
- Separate authoritative SQLite store per registered repository.
- `create`/`show`/`update`/`history` with immutable revisions and
  `--expected-revision` optimistic concurrency.
- Evidence/provenance capture and stale/unresolved reporting.
- Typed one-hop relationships with revision-pinned endpoints.
- Deterministic lexical search (FTS5) and pagination with generation guard.
- Storage isolation and one-transaction atomic mutations.

Foundation status values are `active` and `archived` only. `superseded` and
`retracted` are defined below as forward contracts, not implemented behavior.

**M7 Full Acceptance — before CLOSED (later stage, not Foundation):**

- Status lifecycle `superseded` and `retracted`, semantically distinct from
  `archived`, with traceable links and reasons.
- Deterministic Markdown/JSONL import/export with lossless round-trip for
  supported fields, idempotent import and conflict handling, revision
  preservation.
- Backup/restore and integrity verification.
- Provenance plus human-confirmed vs candidate assertions per blueprint.

**Internal roadmap and gates:**

1. Gate F: Foundation spec review → GO authorizes a written implementation
   plan for Foundation only.
2. Gate P: Foundation plan review → GO authorizes Foundation implementation.
3. Gate I: Foundation implementation + acceptance evidence → GO closes
   Foundation and opens Full-stage spec work.
4. Gate C: Full-stage implementation + acceptance evidence → GO closes M7.

No gate is passed on design text alone; each requires the evidence listed in
its acceptance matrix row. M8 and later milestones remain untouched.

## Repository audit and storage decision

Existing foundations are `args.rs`/`main.rs` for dispatch and JSON/exit conventions,
`registry.rs`/`paths.rs` for repository selection and canonical confinement,
`semantic.rs` for confined source reads, and bundled `rusqlite`/FTS5 plus
`serde_json` for storage and structured input. No additional dependency is
proposed. These are extension points, not permission for a shared-core refactor.

`store.rs` currently owns schema v4 and snapshot-scoped code facts. Its
`promote_staging()` replaces the code tables during indexing; file row IDs,
snapshot IDs and import IDs are not permanent knowledge identities. Authored
records must survive every full, incremental and failed index operation.

Two approaches were considered: knowledge tables inside the code-index database,
or a separate authoritative SQLite file. The former couples preservation and
migrations to every index lifecycle path. The selected separate database avoids
that coupling while reusing SQLite and existing repository validation. Its cost
is that knowledge reads and filesystem/code observations are not one atomic
cross-database snapshot; output must be honest about that boundary.

Use `$RILTRA_HOME/knowledge/<registered-name>.db`, with independent schema version
1 and metadata binding the store to the registered canonical root. The name is
validated by existing registry rules. Root movement/retargeting or mismatched
metadata is an explicit data error, not permission to adopt another store.
Repository renaming, aliases and store relocation have no automatic migration in M7.

### Knowledge-location security invariants

Keep `$RILTRA_HOME/knowledge/<registered-name>.db`. The following are
normative and reuse existing registry/canonical-root and confined-path rules
where adequate:

- Validate the canonical repository root and registry binding on every open.
- Reject symlink/junction escape or unauthorized retargeting for the knowledge
  directory, the database file, and SQLite companion files (`-wal`, `-shm`,
  `-journal`); treat violations as storage errors, never follow them.
- One store belongs to one registered repository; a different repository must
  not adopt another store, including by copying the filename or rebinding
  metadata.
- Root movement, registry rename, and root/metadata mismatch fail explicitly
  with a storage error; no silent rebind, adoption, or migration in M7.
- Missing-store reads never create a database; record lookups fail explicitly
  while search may return an empty result with store absent.
- Corrupt or forward-version stores fail closed and are never deleted,
  recreated, or reconstructed by code-index recovery.
- Full/incremental M6 rebuilds never delete or modify authoritative knowledge.
- No claim of atomic filesystem-wide protection against all races is made;
  concurrency protection is limited to SQLite transactions plus bounded writer
  serialization as stated in this spec.

Corresponding acceptance tests are listed under Gates F-STORE and F-PRESERVE.

No knowledge command changes the M6 database, semantic generation or graph-proof
marker. Knowledge works for a registered repository without a code index.
Missing-store reads must not create a database; record lookups
fail explicitly, while search can return an empty result with store absent.
Creation is an explicit write operation. Corrupt or forward-version stores fail
closed and are never deleted/recreated by code-index recovery.

## Record model and immutable history

Record IDs are caller-supplied and repository-scoped: 1–128 ASCII characters,
matching `[a-z0-9][a-z0-9._-]*`. Duplicate IDs fail; ID and kind cannot be renamed.
SQLite row IDs and display titles never serve as semantic identity.

| Field | Contract |
| --- | --- |
| `id`, `kind` | Stable ID; kind is `decision`, `constraint` or `benchmark` |
| `revision` | Integer starting at 1; each successful update adds one revision |
| `title`, `body` | Required UTF-8 title and authored explanation |
| `status` | Foundation: `active` or `archived`; every status change creates a new revision via `knowledge update --expected-revision` (single contract, see below). `superseded`/`retracted` are forward contracts, not Foundation values |
| `author` | Required actor label for this revision, not authenticated identity |
| `created_unix` | Stored write timestamp; revision counters determine ordering |
| `evidence` | Ordered source/artifact references captured with this revision |
| `measurement` | Required only for benchmark records; structured claim below |

Decisions describe the chosen approach and rationale in their body; constraints
state a requirement and its scope. Neither is inferred from source text.
Benchmarks additionally require a method, environment description, positive
sample count and named finite numeric metrics with explicit units. Metric names
must be unique within the record. No command executes the method, normalizes
units, infers comparability or certifies performance from these authored fields.

Each revision is immutable after insertion; a small head mapping identifies the
current revision. Updates replace the editable payload in a new revision and
require `--expected-revision N`. A stale expectation fails without any writes.
No hard deletion, revision rewriting or automatic history pruning is included.
History is not a filesystem backup.

### Record lifecycle contract

- `active`: the record applies in current knowledge; default for new records.
- `archived`: retained historically but not shown as current; reversible only
  by a new revision through the same update contract.
- `superseded` (Full stage, not Foundation): the record was replaced by a
  newer decision/record; the superseding link must be traceable and the old
  history retained. Must not be represented as `archived`.
- `retracted` (Full stage, not Foundation): the claim is withdrawn with an
  author and reason; history is retained and queries distinguish it from
  `archived`. Must not be represented as `archived`.

`archived` is never a substitute for `superseded` or `retracted`. If the
advanced statuses are not implemented in Foundation, their field values,
transition rules, query visibility, and acceptance tests belong to the next
M7 stage after a separate GO.

### Status-transition contract (Foundation)

There is one consistent contract: status changes use
`knowledge update --expected-revision N` with the new `status` in the input
payload. No separate `archive`/`restore` command exists in Foundation. A
dedicated command may only be added later on demonstrated need, and then it
must create a new revision and enforce the same optimistic-concurrency check.
Every transition follows the atomic-transaction, generation-bump, and
failure-atomicity rules in this spec.

Every mutation is one SQLite write transaction covering revisions/heads, evidence,
relations when relevant, FTS maintenance and a monotonic `knowledge_generation`.
Use writer serialization and a bounded lock wait, with errors on contention.
Failed mutations roll back generation as well as data. No index promotion path
may reconstruct this authoritative history from a cache.

## Knowledge relationships

Relations are authored assertions, distinct from M6 code-edge types. Initial
directed kinds are `relates_to`, `supports` and `contradicts`. In particular,
`supports` does not certify either endpoint and `contradicts` is not an automatic
consistency judgment. There is no conversion to CALLS, REFERENCES or impact safety.

A relation has a repository-local opaque relation ID, exact source record ID and
revision, exact target record ID and revision, kind, author, rationale and stored
timestamp. Both endpoint revisions must exist in the same database before commit.
Null, foreign, guessed or name-matched endpoints are rejected. Identical active
endpoint/kind tuples cannot be duplicated accidentally.

Endpoints remain pinned to the authored revisions. Updating a record does not
transfer old relations to its new revision, automatically or by default.
`related` defaults to the current record revision and allows an explicit
historical revision. Callers must distinguish current-revision relations from
historical-revision relations: an empty result for the current revision is
reported as such and must not imply that no historical relation ever existed.
This deliberate cost avoids silently treating an old assertion as applying to
changed text.

Retraction appends an immutable event with author and reason, rather than deleting
the relation. Normal queries omit retracted relations; explicit historical/audit
output retains them. Archiving a record never erases endpoint history. Cycles and
self-links are permitted as authored relationships; queries inspect one hop only.
No BFS, inferred transitive claims or implicit link discovery is part of M7.

## Evidence, freshness and truth boundaries

Authors supply repository-relative paths and optionally half-open raw byte spans.
At write time, Riltra validates confinement and bounds against bounded source
bytes, records the existing content digest, and records available Git revision
metadata. This is an observed source binding, not producer attestation or a claim
that the cited source supports the prose. Files outside the canonical repository,
symlink escapes, unreadable files and invalid spans reject the entire mutation.

Use existing confined-read and digest primitives. Existing FNV change detection
is non-cryptographic: it is not a signature, tamper-proof identifier or author
authentication. No stored knowledge claim is automatically `Current` or verified.
All authored claims remain `Unverified`, independently of source freshness.

On an evidence-inspecting read:

- Matching confined bytes retain the recorded binding and report fresh evidence.
- Changed bytes report stale evidence and preserve the original path/span/digest.
- Missing, escaping, unreadable or over-cap files report unresolved evidence with
  a reason; they never create a replacement target or a successful empty binding.
- No evidence means unknown support, not a completeness or safety claim.

Historical revisions keep their original evidence. A new revision may explicitly
capture new source bytes; no query silently rebinds a stale span by name or text
similarity. M7 initially uses source locations rather than persisted SCIP identity
links, so ephemeral LSP output and snapshot-local row IDs cannot become knowledge IDs.

Knowledge rows and generation share one SQLite read transaction. Filesystem
observations are separately sampled and reported; neither this transaction nor
an unchanged Git HEAD establishes an atomic snapshot of the working tree.

## Proposed CLI and JSON contract

Reuse existing repository selection, stderr diagnostics and exit codes: 0 for a
successful operation (including an empty search), 2 for invalid arguments/input,
3 for repository/storage errors, missing IDs or revision/generation conflicts.
Errors do not emit partial success JSON. Names below are the proposed M7 surface:

```text
riltra knowledge create --repo R --input -
riltra knowledge show --repo R --id ID [--revision N]
riltra knowledge update --repo R --id ID --expected-revision N --input -
riltra knowledge history --repo R --id ID [--limit N] [--offset N] [--generation G]
riltra knowledge search --repo R --query TEXT [--kind KIND] [--include-archived] [--limit N] [--offset N] [--generation G]
riltra knowledge link --repo R --input -
riltra knowledge unlink --repo R --relation ID --input -
riltra knowledge related --repo R --id ID [--revision N] [--direction inbound|outbound] [--include-retracted] [--limit N] [--offset N] [--generation G]
```

Mutation input is strict JSON on stdin, not an executable configuration or a new
file-format importer. Validate required fields, unknown/duplicate keys, field
types and limits before writing. No YAML parser or Markdown/ADR extraction is added.
Create requires ID/kind and the editable payload; update cannot change ID/kind.
Link input names explicit ID/revision endpoints, relation kind, rationale and
author. Unlink requires author and reason and fails if already retracted.
`related` defaults to outbound direction. Relation IDs identify assertions, not
source-code symbols, and never cross the repository/store boundary.

Example create input, not a claim that this example source span proves the prose:

```json
{"id":"decision-001","kind":"decision","title":"Separate knowledge storage","body":"Preserve authored history across code reindex.","status":"active","author":"ryan","evidence":[]}
```

Responses have independent `schema_version: 1`, repository, knowledge generation,
operation/result, explicit claim verification and evidence freshness/uncertainty.
Paged responses also carry limits, next offset, total and truncation. Records
without code references are still valid authored records with unknown support.
Nothing adds SAFE/LOW, proves graph completeness or changes M6's output schemas.

Limit defaults to 50 and is capped at 1000. Responses return the captured
generation; subsequent pages must supply it. A changed generation fails explicitly
instead of serving pages with duplicates or omissions. History orders by revision;
search by record ID; relations by full endpoint/revision/kind plus relation ID,
using binary collation and a total order. Filters precede counts/pagination.
FTS5 indexes current revisions' title/body only, updated in the same transaction.
Search treats user text as literal lexical terms, never raw SQL or an unbounded
query language. Timestamps come from stored rows; rendering does not add clock data.

## Limits, security and module boundaries

Initial bounds: 1 MiB mutation JSON, 512-byte title, 64 KiB body, 64 evidence
references per revision, 4096-byte search text and 1000 rows per page. Source reads
reuse the existing 1 MiB cap. Benchmark metadata must fit the same input bound.
Author labels are capped at 256 UTF-8 bytes; relation rationales and retraction
reasons at 64 KiB. All text limits count bytes rather than Unicode characters.
These are defensive limits, not performance targets or a hard command deadline.

Use bound SQL values, canonical root validation and existing path rules. Authored
text and relationship rationales are inert data: no shell evaluation, templates,
provider launch, repository code execution, source upload or telemetry. Display
author labels as claims, not identities authenticated by Git or a machine account.

Proposed ownership is a focused knowledge model/query module plus a separate
knowledge-store module, with small args/dispatch additions. Exact function/file
breakdown belongs to the implementation plan after spec review. Do not refactor
M0–M6, alter schema v4, reuse code_edges for authored claims or make code-index
availability a prerequisite. A lexical index is derived; revision rows are canonical.

## Acceptance matrix (gates; design-approved, not proven)

Each row is a STOP/GO gate. GO requires all listed evidence. Failure rows must
fail atomically with no partial writes and without changing generation.
Regression evidence means focused new tests plus intact M0–M6 tests/goldens.
Scope is Foundation unless marked Full. Local fmt/check/Clippy/test/build is
required; cross-platform CI is separately authorized and no closure is claimed
from local evidence alone.

### Foundation gates

- **F-REV — Immutable revisions + optimistic concurrency.**
  Objective: history cannot be rewritten; concurrent writers do not lose updates.
  Required: create/show/update/history; `archived`/restore only via new revision
  with `--expected-revision`; one winner on simultaneous updates.
  Negative: stale `--expected-revision`, duplicate ID, ID/kind rename attempt,
  revision rewrite/delete/prune.
  Regression: unit + CLI integration for happy path, stale-writer conflict,
  duplicate ID, and history ordering.
  Scope limit: only `active`/`archived`; no `superseded`/`retracted` behavior.
  STOP/GO: STOP on any lost update, mutated history, or missing conflict error.

- **F-QUERY — Deterministic search and one-hop relations.**
  Objective: stable schemas, ordering, and pagination.
  Required: FTS5 over current title/body only in same write transaction; literal
  lexical terms (no raw SQL); `history` by revision, `search` by record ID,
  `related` by endpoint/revision/kind + relation ID with binary collation;
  limit default 50 cap 1000; generation returned and required for next pages;
  changed generation fails explicitly; empty search is exit 0 with empty result.
  Negative: SQL-injection text, over-limit pages, cross-generation paging,
  unbounded query language.
  Regression: ordering, boundary/empty pages, truncation flags, generation
  conflict, FTS non-leak of historical revisions.
  Scope limit: one hop only; no BFS/transitive inference.
  STOP/GO: STOP on nondeterministic order, duplicate/omitted pages, or raw-SQL handling.

- **F-ATOMIC — Typed validation and failure atomicity.**
  Objective: invalid input never partially commits.
  Required: strict JSON (required fields, unknown/duplicate keys, types, byte
  limits) validated before write; invalid endpoints, duplicate relation tuples,
  over-bound payloads fail with exit 2/3 and no JSON partial success.
  Negative: injected mid-write failure must preserve heads, history, FTS, and
  generation.
  Regression: validation matrix tests + fault-injection test for rollback.
  Scope limit: 1 MiB mutation JSON, 512-byte title, 64 KiB body, 64 evidence
  refs, 4096-byte query, 1000 rows/page, author 256 B, rationale/reason 64 KiB.
  STOP/GO: STOP on any partial commit or generation bump on failure.

- **F-REL — Revision-pinned relations and retraction.**
  Objective: relations assert exact revisions; history is not rewritten.
  Required: both endpoint revisions exist in same DB; no null/foreign/guessed/
  name-matched endpoints; no duplicate active endpoint/kind tuple; updates do
  not move relations; `related` separates current vs historical; normal queries
  omit retracted relations while audit output retains them; cycles/self-links
  allowed but one-hop only.
  Negative: cross-repository IDs, same-name false links, auto-migration on
  update, current-empty misreported as never-related.
  Regression: pin-preservation, retraction visibility, cycle, cross-repo reject.
  Scope limit: kinds `relates_to`/`supports`/`contradicts`; `supports` is not
  certification and `contradicts` is not a consistency judgment.
  STOP/GO: STOP on any false link or silent relation move.

- **F-EVID — Evidence drift and truth boundary.**
  Objective: source binding is observable and never proves prose.
  Required: confined path + optional half-open byte span validated at write;
  digest + Git metadata recorded; fresh/stale/unresolved reported on read;
  historical revisions keep original evidence; no silent rebind by name/text
  similarity; claims stay `Unverified` even when evidence is fresh.
  Negative: outside-root paths, symlink escapes, unreadable/over-cap files,
  invalid spans reject the whole mutation; matching source must not mark prose
  verified.
  Regression: fresh, changed-bytes stale, missing/escaping/unreadable unresolved,
  no-evidence unknown-support cases.
  Scope limit: source locations only; no persisted SCIP-identity links.
  STOP/GO: STOP on any auto-verify, silent rebind, or escape accepted.

- **F-STORE — Location security and store lifecycle.**
  Objective: the per-repository store cannot be adopted, moved silently, or
  auto-repaired.
  Required: canonical-root + registry binding checked on open; symlink/junction
  escape or retargeting on knowledge dir/DB/companions rejected; root movement,
  rename, metadata mismatch fail explicitly; missing-store reads never create;
  corrupt/forward-version fail closed without delete/recreate.
  Negative: cross-repository adoption, silent rebind, recovery deleting a
  knowledge store.
  Regression: escape, mismatch, missing-store, corrupt/forward-version tests.
  Scope limit: no automatic migration or relocation in M7.
  STOP/GO: STOP on any silent adoption, creation on read, or deletion.

- **F-PRESERVE — Isolation from code index.**
  Objective: knowledge survives indexing and never contaminates M6.
  Required: persists across full/no-op/delta index, failed promotion, semantic
  import, graph-proof revalidation; no M6 database/semantic/graph-proof change;
  works without a code index.
  Negative: index promotion reconstructing history from cache; knowledge command
  altering M6 facts.
  Regression: preserve-across-index suite + M0–M6 golden intact check.
  Scope limit: no cross-database atomic snapshot claim; output is honest that
  knowledge reads and filesystem observations are separately sampled.
  STOP/GO: STOP on any knowledge loss or M6 contamination.

### Full-stage gates (not Foundation; need separate GO before work)

- **G-LIFE — `superseded`/`retracted` lifecycle.** Objective, required
  transitions/links/reasons, query visibility, negative misuse of `archived`,
  regression, and STOP/GO to be specified in the Full-stage spec delta.
- **G-PORT — Deterministic Markdown/JSONL import/export.** Lossless round-trip
  for supported fields, idempotent import, conflict handling, revision
  preservation; negative malformed/partial import; STOP/GO on any history loss.
- **G-BACK — Backup/restore + integrity verification.** Objective, offline
  restore semantics, verification output, negative corrupt-backup handling.
- **G-PROV — Human-confirmed vs candidate assertions.** Provenance distinction
  per blueprint; no autonomous authoring implied.

Design-approved means the contract above is agreed. Plan means a written
implementation plan referencing these gates. Proven means the listed evidence
ran and passed. No gate passes on design text alone.

## Review boundary

This revision captures the approved direction, the Foundation/Full split, the
location-security invariants, the lifecycle contract, the revision-pinned
relation rule, the single update-based status contract, and the per-gate
acceptance matrix with STOP/GO conditions.

Review the record/relationship revision semantics, author/evidence distinction,
CLI surface and storage isolation before approving a written implementation
plan for Foundation only. No M7 product code, migration, dependency, external
provider or MCP implementation is included. M8 and later milestones remain
untouched. STOP after this spec revision; no implementation plan or coding
until this revision is audited and receives GO.
