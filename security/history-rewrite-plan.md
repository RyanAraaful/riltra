# History rewrite plan (local machine paths in git history)

Status: **dry-run only.** Nothing was rewritten and nothing was force-pushed.
Execution is owner-gated (see "Execution" below).

## Why a rewrite is needed

The current tree is sanitized on `security/privacy-remediation` and recorded
in `docs/pilots/cp7a/SANITIZATION.md`, but pre-sanitization bytes remain in
the history of three commits:

| Commit | Date | Subject |
| --- | --- | --- |
| `cabfb9f` | 2026-10-16 | m6: validate real providers and record final local evidence |
| `6449a42` | 2026-10-19 | docs(m8): add CP7-A OpenCode CLI vs MCP benchmark evidence |
| `0215123` | 2026-10-19 | docs: add Windows WAL concurrency audit |

Full-history scan (pinned gitleaks 8.30.1, repository `.gitleaks.toml`,
`--log-opts=--all`): 109 findings in exactly these three commits (94 Windows
user-profile path matches, 15 unix/macOS home matches). Older commits
additionally carry local-checkout-root prefixes in blobs; they are not
personal, but the same replacement scheme removes them so that no local
machine layout survives in history.

## Replacement scheme

Same placeholders as the current-tree sanitization:

| Placeholder | Replaces |
| --- | --- |
| `<USER_HOME>` | local Windows user-profile prefix (slash, backslash and JSON-escaped spellings) |
| `<RILTRA_REPO_ROOT>` | local checkout root, all spellings |
| `<CP7A_ROOT>` | pilot home (`<checkout>/.cp7a`) |
| `<CP7A_ARM_ROOT>` | the two arms' temporary OpenCode working directories |

The exact literal list is deliberately not committed (it encodes the original
values); it lives in the private remediation workspace and mirrors the
sanitizer rules byte-for-byte.

## Dry-run (isolated mirror clone, 2026-10-19)

Tool: `git-filter-repo` 2.47.0 (`a40bce548d2c`) on a pinned Python 3.12.10,
invoked as `git-filter-repo --replace-text <list> --force` on a `--mirror`
clone. The original repository was not touched.

Measured:

- Commits: 129 -> 129 (none lost).
- Rewritten commits: 71 of 129 (58 unchanged), across every branch that
  contains an affected commit: `master`, `m6-task4-callers-callees`,
  `m6-task5-static-trace`, `m6-task6-conservative-impact`,
  `m7-native-structured-knowledge-graph`, `m8-unified-cli-mcp`,
  `m9-multirepo-release-candidate` and `security/privacy-remediation`. All of
  these get new tips and new SHAs after a real rewrite.
- After the rewrite: case-insensitive search for the profile/checkout
  prefixes across every commit's tree = **0 hits**; full-history gitleaks
  rescan = **0 findings**; commit-subject list unchanged; DCO sign-off line
  count unchanged.
- Remaining benign matches: config file names (`global-opencode.cp7a.jsonc`),
  the relative `.cp7a/source` worktree path and the SANITIZATION.md record
  itself. No absolute local paths remain.

Reference mapping from the dry-run (shifts when re-run on a later tip):

| Old | New | Note |
| --- | --- | --- |
| `cabfb9f` | `5834e36` | first affected commit (M6 evidence) |
| `6449a42` | `9a5c97e` | CP7-A M8 benchmark evidence |
| `0215123` | `070017a` | WAL concurrency audit |
| `784b956` | `5cf46c0` | current-tree sanitization commit |
| `2538e69` | `0e52ce5` | scanner-infrastructure commit / `security` tip |
| `cc963ae` | `b4e2763` | `master` tip (PR #3 merge) |
| `m8` tip | `1d69a9d` | branch `m8-unified-cli-mcp` |
| `m9` tip | `774b961` | branch `m9-multirepo-release-candidate` |
| `m7` tip | `34c8e71` | branch `m7-native-structured-knowledge-graph` |
| `m6` tips | `54cd114` / `ec8c2d4` / `453a1b6` | task4 / task5 / task6 branches |

`git-filter-repo` removes the `origin` remote and remote-tracking refs in its
scratch clone by design; the real run re-adds `origin` before pushing.

## Impact on PRs, DCO and evidence hashes

- PR #1-#3 (merged) stay visible, but their head/merge SHAs become
  unreachable from branches after a force-push; cached PR diffs may still
  show pre-rewrite bytes until GitHub purges unreferenced objects (Support
  step below).
- DCO: sign-offs are commit-message text and survive byte-for-byte (verified:
  sign-off line counts identical before and after). Every rewritten commit is
  a new object, so SHAs cited in review threads change.
- M8/M9: `m8-unified-cli-mcp` is merged but its SHAs change;
  `m9-multirepo-release-candidate` is frozen and its tip changes. No M9 work
  item references rewritten SHAs yet.
- Evidence-hash impact: documents that pin internal git coordinates become
  stale (e.g. the CP7-A README source pin `0fd22a1...` and the 40-hex commit
  references in `docs/m6-validation.md`). CP7-A file hashes are unaffected
  inside the current tree: the frozen values stay recorded as historical and
  the derivative-hash map in `harness/verify.cjs` /
  `harness/verify-global-config.cjs` keeps checking the current bytes.

## Recontamination risks

- A stale clone (pre-rewrite history) that pushes afterwards would
  reintroduce the old blobs. Treat the backup mirror as read-only.
- Local branches, worktrees and forks that are not reset keep the old
  objects available.
- The pre-sanitization backups from the current-tree work are outside Git and
  must stay outside.

## Verification (must pass after the real run)

1. `security/scan.sh range <root>..HEAD` on every branch = clean.
2. Full-history scan with `.gitleaks.toml` = 0 findings.
3. Case-insensitive tree grep over `git rev-list --all` for the replaced
   prefixes = 0 hits.
4. `git rev-list --all --count` unchanged; DCO sign-off count unchanged.
5. `bash security/scan-tests.sh` passes on a fresh clone.

## Execution (owner-gated)

1. Freeze pushes; back up a bare mirror outside the repository.
2. Run the pinned `git-filter-repo --replace-text <list> --force` on the
   mirror; re-add `origin`; verify per the list above.
3. Force-push the rewritten branches explicitly, one by one.
4. Re-clone locally (or `reset --hard` every branch) and remove stale
   worktrees.
5. Contact GitHub Support to purge unreferenced objects and stale PR refs
   that still expose the old bytes.
6. Re-run CI on all branches and confirm the `security` job passes.
