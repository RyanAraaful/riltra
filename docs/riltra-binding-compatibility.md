# Code binding compatibility

Code binding exports include the digest of the registered canonical repository root. Import validates that digest and the repository name against the target store. An export containing bindings therefore cannot be imported into a clone registered at a different canonical path or on another machine. Record-only exports do not carry this binding constraint. This is the current fail-closed behavior; there is no automatic anchor rebinding or relocation operation.

Knowledge stores at schema versions 1 and 2 can be read by this version and migrated on a write that needs newer features. Schema version 3 requires a binary that understands it; older binaries reject it even when the store currently has no bindings. A backup of an unchanged schema 1 or 2 store uses the original version 1 manifest. A schema 3 backup uses manifest version 2 and is not a downgrade path.

`knowledge at` returns `page-empty` when the requested page has no rows but matching bindings exist at other offsets. When anchor resolution fails, `unresolved_reason` describes the failure; the caller must still inspect freshness and pagination before drawing conclusions.

The read path reuses its source bytes and does not construct a snapshot-wide file map or snapshot digest. The existing index schema has an index on `snapshot_id` but not `(snapshot_id, path)`, so SQLite can still scan snapshot file rows to locate a path. No constant-time lookup or repository-scale latency is claimed; eliminating that scan requires a compatible index migration and measurement.

## Review fixes and evidence

- `knowledge bind --symbol` accepts both `--symbol value` and `--symbol=value`. On an isolated copy of the pinned SCIP fixture, binding and reading the actual `confined_read` symbol succeeded; an incorrect symbol returned `unresolved` with its reason. The semantic anchor remains `Heuristic` and `Unverified`.
- The anchor lookup regression with 20,000 unrelated indexed files exceeds a SQLite operation budget with the former materialized file map and passes with the targeted lookup. This fixture measures the operation budget, not elapsed time or absence of a SQLite scan. A separate test proves the resolver can reuse supplied source bytes.
- The review fix uses deterministic, explicitly escaped JSON for binding responses and export. CLI/MCP parity, special-character round-trip, duplicate import provenance conflicts, retraction merge and conflicting retractions are covered by tests. Failed conflicting imports leave the store unchanged.
- An offline debug build from the frozen baseline `2852ddb4658afdfdcd0286f9a5d1a47f4f5bb87d` restored newly generated backups for schema v1 and v2 in isolated homes; both manifests used version 1. The baseline executable SHA-256 was `501bd4c8b3a7d348485137e4c71daa5256d5507ec6d6d911ddf78b3e8a8d60b9`. A schema v3 fixture with zero bindings uses a version 2 manifest. These tests do not claim schema v3 downgrade support.
- A synthetic link without confirmation did not migrate the first fixture to schema v2; the compatibility test was repeated using explicit confirmation. This confirmation remains a provenance claim and does not verify content.

Sanitized captures and their accounting are under `docs/pilots/riltra-binding-review-2026-10-29/raw/`; original local captures remain under `target/review-fixes/`. Test output and scan results apply only to the recorded source revision and environment. PR CI must be checked separately after pushing.

The initial staged security scan reported nine local-absolute-path findings in nested legacy-backup JSON; after the path redaction was corrected, the staged scan reported zero findings. The redaction accounting preserves hashes of both local originals and shared derivatives.
