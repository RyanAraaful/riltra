# M7 — Native Structured Knowledge Graph

Status: **DRAFT FOR REVIEW**. The user approved the design direction and creation
of this draft, not an implementation plan or product implementation.

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
| `status` | `active` or `archived`; archiving/restoring creates a revision |
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
Archive is reversible through another revision; history is not a filesystem backup.

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
transfer old relations to its new revision. `related` defaults to the current
record revision and allows an explicit historical revision; this deliberate cost
avoids silently treating an old assertion as applying to changed text.

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

## Acceptance evidence required before M7 can close

These are future requirements, not tests claimed to have run in this design branch:

1. CLI create/show/update/history and archive/restore preserve immutable revisions;
   simultaneous expected-revision updates allow one winner without lost updates.
2. Search and one-hop relations have stable schemas, deterministic ordering,
   correct boundary/empty pages and explicit generation-conflict handling.
3. Typed validation, duplicate IDs/keys, invalid endpoints and mutation limits fail
   atomically. Injected mid-write errors preserve heads, history, FTS and generation.
4. Relations preserve exact endpoint revisions and retractions; same-name records,
   cross-repository IDs, cycles and historical revisions never create false links.
5. Evidence drift, missing files, path escapes, symlinks and invalid spans remain
   stale/unresolved. Matching source never verifies authored prose or benchmark claims.
6. Knowledge persists across full/no-op/delta index, failed promotion, semantic
   import and graph-proof revalidation. No knowledge command contaminates M6 facts.
7. Registered repositories without an index remain usable; reads do not create or
   repair authoritative stores. Corrupt, moved-root and forward-schema stores fail
   explicitly without deletion or silent rebinding.
8. Existing M0–M6 tests/goldens remain intact, with focused new unit/integration
   cases and local fmt/check/Clippy/test/build followed by separately authorized
   Windows/Ubuntu/macOS validation. No closure claim from local evidence alone.

## Review boundary

This draft captures the approved direction and concrete proposed contracts.
Review the record/relationship revision semantics, author/evidence distinction,
CLI surface and storage isolation before approving a written implementation plan.
No M7 product code, migration, dependency, external provider or MCP implementation
is included. M8 and later milestones remain untouched.
