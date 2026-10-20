# CP7-A sanitization record (privacy remediation)

- Date: 2026-10-19
- Scope: `docs/pilots/cp7a/**` plus three M6/M8 evidence files:
  `docs/m6-validation.md`, `docs/pilots/m6-windows-evidence.json`,
  `docs/audits/2026-10-19-windows-wal-concurrency.md`.
- Reason: the tracked evidence contained local machine paths (Windows user
  profile, repository checkout root, pilot home, per-arm temp directories).
  They were replaced before publication.
- Method: deterministic literal replacement of four path prefixes (all
  slash/backslash/JSON-escaped variants) over tracked text files, plus manual
  refactors of harness scripts. The pre-sanitization bytes were backed up
  outside Git with a SHA-256 manifest before modification. This directory and
  the three files above are therefore a **sanitized derivative**, not a
  byte-identical snapshot, of the measurement host state.

## Evidence states

### Historical frozen evidence (not published)

The pre-sanitization bytes are retained privately, outside Git: the 167
automatically replaced files and the 13 manually refactored harness files are
backed up with per-file before/after SHA-256 values. The manifest SHA-256 is
`39167218...` (full value under "Rollback" below). The frozen hashes quoted in
`summary.json`, `measurements.json`, `method/environment.json`,
`method/fixtures-manifest.json`, `raw/run.log`, `summary.md` and the
historical `gate/verify.*` logs continue to describe those pre-redaction
bytes; this remediation did not edit them.

### Public sanitized derivative (this tree)

Local path prefixes were replaced by the placeholders below in 180
path-bearing files (167 automatic replacements, 13 manually refactored
harness files). `corpus/corpus.sha256` was updated separately to the
derivative corpus hash; the frozen value remains recorded in the evidence.

Textual fields that embedded a path did change at the string level. The
structured comparison found 874 changed string fields in JSON/JSONL evidence,
including 16 task `prompt` fields, 48 `answer` fields, and command/log output
fields that quoted paths. Every such change is exactly the recorded
placeholder substitution; no other wording, score, verdict, count, timing or
token total was edited.

### Post-sanitization verification (2026-10-19)

A structured comparison of the private backup against this tree checked every
changed file. It is local-only because it requires the backup; the method and
results are recorded here.

- 167/167 recorded before-hashes matched the backup files.
- 126 JSON and 17 JSONL evidence files parsed and compared field by field:
  874 changed string fields, all placeholder substitutions; zero changes to
  numbers, booleans, nulls, types, array lengths, or key sets.
- 24 text files: numeric-token sequences are identical after placeholder
  normalization. The only residual line differences are the reviewed harness
  refactors and derivative-hash documentation described below (README and
  `harness/verify-global-config.cjs`); no evidence log line changed except by
  placeholder substitution.
- `harness/verify.cjs`: 56/57 checks pass; the sole failure is
  `binary sha256`, explained below.
- `harness/verify-global-config.cjs`: PASS; reconstruction matches the
  recorded derivative `D4D892...`, while the frozen `1E9671...` stays in the
  record.

The comparison was configured to stop on any numeric or structural change;
none occurred. If an invariant metric had changed, this remediation would
have stopped instead of publishing the result.

## Placeholder scheme

| Placeholder | Replaced prefix |
| --- | --- |
| `<USER_HOME>` | local Windows user-profile directory |
| `<RILTRA_REPO_ROOT>` | repository checkout root |
| `<CP7A_ROOT>` | pilot home directory (`.cp7a` inside the checkout root) |
| `<CP7A_ARM_ROOT>` | temporary OpenCode working directories of the two arms |

Occurrences: 1025 automatic replacements across 167 tracked files, plus 52
path occurrences replaced by manual harness refactors in 13 files, for 1077
path occurrences in 180 path-bearing files. The CI-runner profile path
(`C:\Users\runneradmin`, `D:\a\...`) appears once in the M8 WAL audit log; it
is runner-generated, not personal, and was kept.

## Harness changes (functional, not placeholder-redacted)

These scripts now derive their paths from their own location
(`__dirname` / `$PSScriptRoot` / `os.tmpdir()`), so they run from any checkout:
`harness/bin/riltra-ro.cjs`, `harness/citations.cjs`, `harness/g6-analyze.cjs`,
`harness/g6-smoke.ps1`, `harness/run-gate.ps1`, `harness/run-paired.ps1`,
`harness/reference/cli-compare.cjs`, `harness/reference/direct-mcp.cjs`,
`harness/verify.cjs`, `corpus/self-test.cjs`; `corpus/build-corpus.cjs` and
`corpus/self-test.cjs` emit `<CP7A_ROOT>` unless `RILTRA_CP7A_ROOT` is set.
The arm-A location plugin now matches the wrapper by its final path segments
(`.cp7a\bin\riltra-ro.cmd`) instead of the frozen host prefix. Archival configs
(`method/global-opencode.cp7a.jsonc`, `harness/configs/arm-a/opencode.jsonc`,
`harness/configs/arm-b/opencode.jsonc`) keep placeholders: substitute locally
to replay.

These edits were reviewed in the structured comparison: they change code and
documentation only, and leave every numeric token (scores, counters, timings,
token totals) unchanged.

## Frozen vs derivative hashes (hash-checked files)

| File | Frozen (pre-redaction) | Published derivative |
| --- | --- | --- |
| `corpus/corpus.json` | `D9F33A765A98723B0ACFD314865C976FB8BD58A680C911932B64C83E8530C824` | `4FB9A77E49F10340E6804A6ACFBD966302E4E1C126868A18F9216D2999FF7A56` |
| `method/global-opencode.cp7a.jsonc` | `1E9671EC34F536F9E6070F0F791BE466A3ECB025BD2D5EDECE3081552047F93C` | `D4D892E6CB7F0731543C0B5826CDA60E0990A662D7F00CC382A9F3DF4AA17C96` |
| `harness/configs/arm-a/opencode.jsonc` | `69CF31E776164A732CA7269598B52289DB9C8F87EB755AFA653C7C86CC68EE0D` | `88AFCBC8D74CDB89AC04B6E7F19F2ACB08D87CB085E0E427DD25D4D3F4AABC69` |
| `harness/configs/arm-b/opencode.jsonc` | `69CF31E776164A732CA7269598B52289DB9C8F87EB755AFA653C7C86CC68EE0D` | `88AFCBC8D74CDB89AC04B6E7F19F2ACB08D87CB085E0E427DD25D4D3F4AABC69` |
| `harness/configs/arm-a/plugin.ts` | `70036542DFC933BC3B39961D8E5C51D231ADAF94D5001D6DE52222EDCDCA2E14` | `9403FDEC64E86F5E66B6542D86DE5BA9E0F3DCC28D7AE5E475E99EE6216499F4` |
| `harness/configs/arm-b/plugin.ts` | `423B7D0210DFE6D9E328DED514DF2E3C3820FAFFF37C7ED0032A64D482520721` | `297380442E6B8865F2E8331E7469B6B843E06AE19A8A905BE5C4C596E958F550` |
| `harness/bin/riltra-ro.cjs` | `BECF06E9643342A917F5A42DEF0865FF7B9829181B65B520A293F436B8E73158` | `62AD3D772DF096E8698C99980FB6D5CDFC08833113939FC3682E566572E58A7B` |

`harness/verify.cjs` accepts the derivative hash for exactly these seven paths
and still asserts the frozen values everywhere else. `harness/verify-global-config.cjs`
re-derives the archived global config from
`method/global-opencode.original-backup.jsonc` plus the two recorded edits and
reports both hashes (it now expects the derivative).

## Binary-hash status

`harness/verify.cjs` reports 56/57. The only failure is `binary sha256`: it
compares the locally rebuilt `target/debug/riltra.exe` against the frozen
`environment.binary_sha256` recorded when the measurements were taken. The
measured binary was never committed and the local build was rebuilt later, so
the hashes differ. No score, verdict, aggregate, token total or hash check of
the published evidence depends on that rebuilt binary, and no historical
result was changed. This is a replay-environment note, not a sanitization
defect.

## Reproducibility impact

- Replaying the harness requires substituting the placeholders with local
  paths (or setting `RILTRA_CP7A_ROOT`) and rebuilding `riltra`; the measured
  numbers depend on harness, source pin and model, not on path text.
- Rollback: restore from the pre-sanitization backup whose per-file SHA-256
  manifest is retained privately (manifest SHA-256
  `3916721869E3EC06DEF0A367E86930D642BD6AC0AA0B7B925E8705E1522F9AB4`).
