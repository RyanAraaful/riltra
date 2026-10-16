# M6 Task 7 correction — persisted graph proof compatibility

Correction of reviewer REVISE against local checkpoint
`7964219c228896f4e90df16b968941c7bd1c2202`. The commit carrying this report
is the new local audit unit on `m6-task7-multilanguage-corpus`.
Scope is upgrade compatibility only: no push, Task 8, M7/M8, dependency or
schema migration. SQLite remains schema v4; existing JSON goldens are unchanged.

## Actual old-binary reproduction

Built a pre-Task7 binary from accepted Task 6 commit
`453a1b6f00d89f24d11e6d2346722047f10b7d1b` using a local `git archive` under
gitignored `target/task7-correction/legacy/`. Built the checkpoint binary from
`7964219`. Distinct Cargo `--cfg` flags forced recompilation before copying
the two executables, avoiding reuse of the same workspace output path.
No checkout, remote operation or source snapshot rewrite was needed.

The old binary initialized, indexed and imported SCIP into three real SQLite
databases using the existing Python corpus sources/manifests:
`alias`, `same_name_binding` and `nested_anonymous`. Then both binaries queried
the same databases and source paths, without reindex:

| Source defect | Old binary | Task 7 checkpoint binary |
| --- | --- | --- |
| Alias variable treated as callable | False CALLS in all four commands | Same false CALLS persisted |
| Parameter sharing function name | False CALLS in all four commands | Same false CALLS persisted |
| Anonymous body borrowing outer owner | False CALLS in all four commands | Same false CALLS persisted |

`callers`, `callees`, `trace` and `impact` could expose the stored rows as
Exact CALLS; freshness/Unverified labeling did not invalidate the obsolete
proof. Some anchors require a syntax-body position to expose the old syntax
owner. This proves the gap was persisted proof compatibility, independent
of the corrected derivation code.

After the correction, on those same actual old databases:

- All 12 initial graph queries returned exit 3, `graph-proof-incompatible`,
  and empty stdout. No stale-proof graph JSON was returned.
- Successful corrected full index restored all 12 queries. The false CALLS
  were absent from every result; valid source evidence remained conservative.
- Running the old binary's semantic import against each repaired database
  advanced its semantic generation. All 12 corrected queries were rejected
  again, before anchor ambiguity or relationship serialization.

Raw before/after responses and the three-case summary are in
`target/task7-correction/repro/` and `verified.json`. The ignored scripts
`repro.cjs` and `verify.cjs` repeat the actual-binary experiment; the tracked
regressions below reproduce its old-v4 row shapes without requiring an old
binary in CI.

## Minimum compatibility strategy

The existing `meta` table stores one `graph_proof` token bound to:

1. Derivation revision `m6-task7-source-proof-1`.
2. The exact source snapshot nonce, not recycled snapshot rowid.
3. The existing `semantic_generation` counter.

All four graph query entry points require a matching token inside the same
SQLite read transaction that pins their snapshot. Missing, malformed, stale,
or unknown-version tokens fail closed with the existing data-error exit code
3 and actionable repair instructions. This applies to empty and otherwise
valid graphs too: absence of proof metadata is not evidence of compatibility.
Queries neither repair the database nor parse source to guess compatibility.

A full derivation records proof metadata only after syntax and every semantic
import have been processed under current rules. A successful import into an
already certified graph can update its own edges and record the next generation.
An import into an uncertified graph must re-derive **all** existing imports
and syntax edges before recording certification; refreshing only the new
import would leave unrelated old false CALLS trusted.

Facts, edges, generation and marker changes share the existing writer
transaction. A failed import cannot certify any partial work. Staging promotion
removes the previous live token and transfers certification only when the
staging token matches the current revision, staging nonce and staging generation.
It binds the transferred token to the retained live generation within the same
promotion transaction. Missing or mismatched staging metadata never inherits
the prior live token. Promotion failure now explicitly rolls back before
detaching, including when callers retain the connection.

The generation binding catches writes by pre-correction binaries that leave
the source nonce unchanged. The nonce binding catches a different promoted
snapshot. This metadata describes **proof-rule compatibility**, not freshness,
runtime execution or completeness. Existing M5 freshness/verification checks
still apply; Exact never becomes Current.

## Repair behavior and source identity

```text
riltra index --repo R --full
riltra semantic import --repo R --scip INDEX.scip
```

Full index creates a legitimate new snapshot and re-derives its graph. Reimport
does not rewrite the source snapshot: regression assertions compare all
`snapshots` and `files` row values before and after successful revalidation,
including nonce, digests, root and revision metadata. Positive direct relations
recover, while false alias and anonymous-owner rows are removed even from
imports untouched by the new artifact.

Existing append-only import identity rules are preserved. Reimporting an
identical artifact adds another import; overlapping identities can therefore
remain explicitly ambiguous. Certification does not merge imports or select
one arbitrarily. The regression verifies those anchors return ambiguity errors,
the false alias CALLS rows are gone, and all four queries over the other imports
recover correctly. Prefer full index on the original legacy database when
duplicate-import ambiguity is unwanted; this correction does not resolve
preexisting duplicate identities.

## Regression and RED/GREEN evidence

Eight CLI integration tests in
[upgrade.rs](../crates/cli/tests/m6_corpus/upgrade.rs) cover:

- Three old graph shapes, each rejected by all four query commands.
- Successful reimport validating every import with unchanged source identity.
- Full-index recovery with a new nonce and preserved valid CALLS.
- Missing, malformed/future, wrong-snapshot and old-writer generation markers.
- Import rollback after an injected proof write failure: original facts,
  edge counts, generation and incompatible status remain intact.
- Promotion rollback after an injected proof write failure: previous snapshot
  and certification remain intact, and its valid queries continue working.

Two store tests additionally check missing/mismatched/current staging marker
transfer (staging generation 3, live generation 7), and rollback on a retained
connection. Initial upgrade RED had five failures and one already-passing
full-index case; the three legacy query tests returned exit 0 instead of 3.
After the guard, all eight upgrade tests pass. The retained-connection test
separately reproduced an open transaction after failed proof promotion; an
explicit ROLLBACK changed that RED to GREEN. Additional rollback/transfer cases
are coverage extensions; no historical RED is claimed for each one.

Independent read-only review found no important correctness issues and ran
the eight upgrade tests. Its staging-transfer coverage recommendation was
addressed; follow-up review and both store tests also passed independently.

## Validation, costs and remaining limits

All required commands returned exit 0 locally on Windows:

```text
cargo fmt --all -- --check
cargo check --locked
cargo clippy --all-targets -- -D warnings
cargo test --locked
```

**402 passed, zero failed/ignored**: 201 unit, 16 CLI, 137 edge integration,
21 index, 15 semantic, 12 syntax. This adds 10 regressions to the 392-test
checkpoint. Existing 79 multilingual cases and 494 JSON responses still pass
without golden updates. Linux/macOS CI was not run locally.

The M6 architecture/lifecycle plan and importer comments now explicitly allow
Task 7's Tree-sitter validation during derivation/import. It reads confined,
digest-matching files capped at 1 MB each; it scans supported snapshot files
for syntax derivation and again per semantic upsert. An incompatible-graph
repair processes every import and can repeat those scans. Unavailable or
unprovable source reduces relations to uncertainty; it does not authorize
old CALLS. Queries still do not reparse. Optimization and scalability benchmarks
remain Task 8 and were not performed.

Unmarked but otherwise correct old graphs also require explicit revalidation.
Metadata is not a cryptographic defense against arbitrary direct database
tampering; supported application writers maintain it transactionally. Future
proof-rule changes must bump the compatibility revision. Existing import
ambiguity, conservative source-proof coverage, freshness and cooperative
traversal deadline limits remain unchanged.

Stop for Task 7 re-audit. No push or Task 8 work.
