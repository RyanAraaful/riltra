# Full repository Rust semantic recheck

Date: 2026-10-31. Source archive: commit `ad120b7abbd3a0a2442d89e0b71ab128f143b11a`; Windows; rust-analyzer 1.99.1 (`58cf8f9c`), offline with a separate Cargo target. Riltra binary SHA-256 `1381afacf4593077a1def8ecf1bfcbdb906ad2574056d91ce56b8db0820a7ecb`; built at `1bb0f6a`. `git diff 1bb0f6a ad120b7 -- crates Cargo.toml Cargo.lock` returned no differences. Source is a Git archive without `.git`; Riltra reports `head=-`, so the archive pin is external provenance, not embedded snapshot provenance.

## Independent oracle and measured results

An independent source-only review, before seeing query output, identified 16 source call expressions for `confined_read`: nine production sites (`edges.rs:111`, `index.rs:815,1357`, `knowledge_binding.rs:579,695,1111`, `knowledge_store.rs:880,974`, `scip.rs:643`) and seven test sites (`semantic.rs:672,677,691,692,697,698,708`). One production site is in a closure; three test sites are in assertion macros. This is an oracle for one symbol and source spelling, not every Rust symbol or possible indirect call.

Rust-analyzer produced a SCIP artifact in 42.5 seconds. Import: 35 documents, 17,944 definitions, 104,712 references, no skipped documents, `Unverified`. The resulting fresh, non-truncated callers query returned all 16 oracle sites: 12 `CALLS`, one `POTENTIAL_CALL` (`owner-unproven` closure) and three `REFERENCES` (assertion macros). No returned row was outside these 16. **Site discovery** is 16/16 recall and 16/16 precision for this defined universe; it is not call-edge recall because four rows are not `CALLS`. For the independently source-identified 16 call expressions, `CALLS` recall is 12/16; all 12 returned `CALLS` corresponded to oracle call expressions, yielding 12/12 precision within this query. The three macro expressions and the closure follow current conservative labels, not necessarily defects. No claim of repository-wide precision or recall is made.

`impact` returned `UNKNOWN`, 27 paths and `truncated: true`, with `time-budget` and `unresolved-owner` among reasons. These paths were not independently checked one by one and are not a complete affected set. Query output, raw SCIP and logs remain in ignored `target/wiy-full-ad120b7/` and contain local paths; they have not been sanitized for publication.

## Producer diagnostics inventory

All 179 `ERROR`/`WARN` lines in `producer.log` were classified by message: 177 `Encountered enclosing definition with no name` and two missing SCIP document definitions, at `crates/cli/src/semantic.rs:53:16-53:21` and `:151:16-151:21`. No other error/warning message class was found in that log. Producer stdout listed 93 duplicate-symbol sites, covering 24 distinct final symbol strings (including 21 `scip_proto/descriptor.`, 13 `scip_proto/instance.`, and nine `crate/`); the string grouping is diagnostic inventory, not a claim of 24 globally unique full symbol URIs. The producer warns that duplicate lookup can be ambiguous. These diagnostics are not preserved in the import status and exit success is not proof of full correctness. This inventory does not establish root causes or quantify the effect of every diagnostic on every symbol; full impact audit remains open.

## CI

Run [41206590035](https://github.com/RyanAraaful/riltra/actions/runs/41206590035) passed on `ad120b7` on branch `eval/wiy-correctness-ad120b7`: check and installer jobs passed on Windows, Ubuntu and macOS, and security passed. The run executes ordinary build/lint/tests and installer checks; it does not reproduce this rust-analyzer SCIP measurement. macOS runner is Intel x64, not ARM64.
