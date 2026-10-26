# M9 release-candidate gate evidence at `ee7482d`

Minimal evidence for the repeat of G-SEM, G-INC, and G-FSH on commit `ee7482dfb12f0e88e3620ada5ce35f74f45041a3` (merge of PR #7 into `master`). The record is the [release-candidate gate amendment](../../m9-release-candidate-gate-amendment-2026-10-26.md), sections 5 and 6.

## Contents

| Folder | Content | Gate |
|---|---|---|
| `gsem/raw/` | G-SEM attempt 1 outputs (first home) | G-SEM |
| `gsem/raw-attempt2/` | G-SEM attempt 2 outputs (warm re-run, same home) | G-SEM |
| `gsem/raw-attempt3/` | G-SEM attempt 3 outputs (new home) | G-SEM |
| `gsem/fixture/` | Source files read by the fixture byte checks (`rs`: Rust, `ts`: TypeScript) | G-SEM |
| `bench/cp2/raw/incremental/` | G-INC outputs: no-op, one-file delta, oracle, fact digests, 12 query pairs, restore | G-INC |
| `bench/cp2/raw/freshness/` | G-FSH outputs F0–F5: index status and repo status per scenario | G-FSH |
| `evidence/` | Run logs with timings, toolchain and binary summary, self-test logs, pinned fact tables, grounding check and output, run scripts | All three |

Each attempt folder holds the same 11 files: provider versions, rust-analyzer status and definition, rust-analyzer references, scip-typescript import, producer log, query, TypeScript status, TypeScript call-site definition, and a summary.

## Selection

Included: the text outputs and logs of the frozen scripts that the three gates read, the scripts that produced the grounding check and the run record, and the source files of the two fixtures.

Not included:
- binaries, SQLite databases, and the TypeScript fixture's `index.scip` (derived binaries; `index.scip` differs between the attempt 1–2 home and the attempt 3 home);
- the source archive, the MMSP snapshot, and build target folders;
- working copies of the amendment;
- the intermediate single-attempt byte check. Its rows are the attempt 1 rows of `evidence/gsem-byte-check-all-attempts.json`.

The full verification-host set (191 files) stays outside the repository. Its manifest SHA-256 is recorded in the amendment, section 5.4.

## Redaction

Only path strings and line endings changed:
- `<EVIDENCE_ROOT>`: the verification-host folder that held the run, in every form it appears (extended-length, JSON-escaped, 8.3 short name).
- `<CARGO_HOME>`: the Cargo home directory of the verification host.
- `<ORIGINAL_MMSP>`: the original MMSP benchmark clone, which was only read.
- CRLF line endings became LF, as `.gitattributes` (`* text=auto eol=lf`) requires for text files. Three files have CRLF on some lines only; their line numbers are in `REDACTION.tsv`.

Reversing both changes reproduces the verification-host SHA-256 of every file (column `scratch_sha256` in `REDACTION.tsv`). The archived digests in `SHA256SUMS.txt` are of the LF files.

## Verify

From this folder:

```bash
sha256sum -c SHA256SUMS.txt
```

From the repository root:

```bash
bash security/scan.sh dir docs/pilots/m9-rc-ee7482d
```

Timings are single runs on one host. Wall times are in `evidence/cold-index.log` (`wall_s`) and `evidence/incremental.log` (`wall_ms`).
