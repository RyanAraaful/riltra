# History rewrite plan (local machine paths in git history)

Status: **dry-run only.** Nothing was rewritten and nothing was force-pushed.
Execution is owner-gated (see "Execution" below). The correction round that
expanded the scanner did **not** run `git-filter-repo`.

Post-merge state (2026-10-20, after PR #5) was re-measured with the final
`.gitleaks.toml`; the current numbers, refs and open items are recorded in
"Verified current state" below. The 2026-10-19 measurements in this document
are kept as the historical record of that round.

## Verified current state (2026-10-20, after the PR #5 merge)

Method: pinned gitleaks 8.30.1 (local SHA-256 checked against the published
release checksums), this repository's `.gitleaks.toml`, `git log -p --all`,
and no `.gitleaksignore`. Reproduce with:

    gitleaks git --config .gitleaks.toml --redact --no-banner \
      --log-opts="--all" --gitleaks-ignore-path=<empty-dir> .

| Item | Value |
| --- | --- |
| Full-history findings | 1066 in 8 commits, 183 files |
| Rule mix | 957 `local-absolute-path`, 94 `windows-user-path`, 15 `unix-home-path` |
| Credential-rule findings | 0 |
| Remote branches | 5 (`master`, `m7-…`, `m8-…`, `m9-…`, `security/privacy-remediation`) |
| Commits reachable from all refs | 138 |
| Pull refs | `refs/pull/1/head` … `refs/pull/5/head` |
| Tags / notes | none |
| DCO sign-off lines | 137 across 138 commits (only `cc963ae` carries none) |
| `master` tip | `cbd38f1` (PR #5 merge) |
| `m9` branch tip | `a097e2db` |
| Scanner regression suite | 32 passed, 0 failed |

Difference against the 2026-10-19 measurement below: the two synthetic
fixture findings in `2538e69` are no longer reported under the final config,
and the branch set shrank from eight to five (the three `m6` task branches
were deleted; their tips are ancestors of `master`). Neither measurement
contains a credential finding.

The two `.gitleaksignore` entries are currently **inert**: the new-branch
scan (`cc963ae..7b9720d`) exits 0 with the file removed as well as with it,
and the history scan reports no finding in
`security/testdata/allowed/drive-paths.txt`. Removing the file is a separate,
owner-approved change; it stays documented as temporary here.

## Why a rewrite is needed

The current tree is sanitized on `security/privacy-remediation` and recorded
in `docs/pilots/cp7a/SANITIZATION.md`, but pre-sanitization bytes remain in
history. Full-history scan with pinned gitleaks 8.30.1 and the final
repository `.gitleaks.toml` (`--log-opts=--all`, 2026-10-19):

- **1068 findings in 9 commits, 184 files** (measured 2026-10-19): 959
  absolute-path findings (drive-letter workspace paths, mostly the local
  checkout root), 94 Windows user-profile findings and 15 unix/macOS home
  findings. The 2026-10-20 re-measurement above gives 1066 in 8 commits and
  183 files; the difference is the synthetic fixture finding in `2538e69`,
  which the final config no longer reports.
- This supersedes the earlier 109-finding count, which used the narrower
  profile-only rule set.

Affected commits as measured on 2026-10-19. The 2026-10-20 re-measurement
covers the same 8 commits with the same per-commit counts; `2538e69` is no
longer reported.

| Commit | Date | Subject | Findings |
| --- | --- | --- | --- |
| `6449a42` | 2026-10-19 | docs(m8): add CP7-A OpenCode CLI vs MCP benchmark evidence | 882 |
| `cabfb9f` | 2026-10-16 | m6: validate real providers and record final local evidence | 177 |
| `2538e69` | 2026-10-19 | feat(security): add gitleaks scan with hooks and CI job | 2 |
| `4746fff` | 2026-10-19 | docs: professional English README and untrack private blueprint | 2 |
| `0215123` | 2026-10-19 | docs: add Windows WAL concurrency audit | 1 |
| `9953b0c` | 2026-10-19 | m7: finalize D9 canonical mapping and add Full-stage implementation plan | 1 |
| `b7f4115` | 2026-10-19 | m7: revise Full-stage spec delta per reviewer (schema v2, active-only search, markdown contract) | 1 |
| `bcbd1c6` | 2026-10-19 | feat: M1 native CLI repository registry with git fingerprint and path security | 1 |
| `d1af757` | 2026-10-19 | chore: M0 open-source bootstrap and legal baseline | 1 |

Five remote branches exist today and all five contain at least one affected
commit; the three `m6` task branches from the 2026-10-19 round are gone (their
tips are ancestors of `master`). The two findings in `2538e69` were the old
synthetic allowed-fixture example (a drive path under
`security/testdata/allowed/drive-paths.txt`), not personal data. Under the
final config the scanner no longer reports them, so the replacement-list
decision for that literal is moot and the post-rewrite scan stays clean
without it.

Until the rewrite lands, `.gitleaksignore` still pins those two fingerprints,
but the pins are inert: the new-branch scan (`cc963ae..7b9720d`) exits 0 with
the file removed as well as with it. The entries remain temporary: remove and
re-evaluate them after the rewrite, when the full-history scan must be clean
without them.

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
sanitizer rules byte-for-byte. The absolute-path rules added in the correction
round mean the list must include the checkout-root and example-drive literals,
not only profile prefixes.

## Dry-run (isolated mirror clone, 2026-10-19)

Tool: `git-filter-repo` 2.47.0 (`a40bce548d2c`) on a pinned Python 3.12.10,
invoked as `git-filter-repo --replace-text <list> --force` on a `--mirror`
clone. The original repository was not touched.

Measured with the **then-current, profile-only** scanner:

- Commits: 129 -> 129 (none lost).
- Rewritten commits: 71 of 129 (58 unchanged), across every affected branch.
- After the rewrite: 0 profile/checkout prefix hits across all trees; 0
  gitleaks findings under the old rules; commit subjects unchanged; DCO
  sign-off count unchanged.

This dry-run is **scope-incomplete** for the final scanner: the
absolute-path rule expands the affected set to nine commits (six not covered
by the profile-only scanner) and 959 additional findings, and those commit
contents were not exercised. Run a fresh mirror dry-run with the full
replacement list before any real rewrite; do not cite the old zero-finding
result as proof for the final rule set. The 129-commit baseline above is also
pre-merge; the current baseline is 138 commits on 5 branches.

The mirror dry-run must additionally cover the scanner's mode gap: a history
scan (`gitleaks git`) reports no finding in
`security/testdata/allowed/drive-paths.txt`, while a file scan of the same
content (`gitleaks dir`) reports two. A clean history scan is therefore not
sufficient evidence on its own; the literal grep in the verification list is
mandatory, not optional.

Reference mapping from the earlier dry-run (stale; will shift with the final
list and the newer security commits). The `Old` column is the tip of that
time; the tips have since moved: `master` is now `cbd38f1` (PR #5 merge, with
`cc963ae` as the PR #3 merge below it), `m8` is `d1ce68f`, `m9` is `a097e2db`,
`m7` is `7530102`, and the three `m6` task branches no longer exist.

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

## Impact on PRs, refs, DCO and evidence hashes

- PR #1-#5 (all merged) stay visible, but their head/merge SHAs become
  unreachable from branches after a force-push. The remote exposes
  `refs/pull/1/head` through `refs/pull/5/head`; check them again immediately
  before the rewrite. Cached PR diffs, review threads and timeline events may
  still show pre-rewrite bytes until GitHub removes unreferenced objects.
- No tags or notes exist today; re-check before execution. Local worktrees,
  other clones and forks keep the old objects unless reset or deleted.
- DCO: 137 sign-off lines currently (138 commits; only `cc963ae`, the
  GitHub-generated PR #3 merge, carries none); sign-offs are commit-message
  text and survive byte-for-byte. Every rewritten commit is a new object, so
  SHAs cited in review threads change. No commit is cryptographically signed
  today (`%G?` is `N` for all); if signed commits are added later, a rewrite
  invalidates those signatures and they must be re-signed or accepted as
  unsigned.
- M8/M9: `m8-unified-cli-mcp` is merged but its SHAs change;
  `m9-multirepo-release-candidate` is frozen and its tip changes. No M9 work
  item references rewritten SHAs yet. M9 CP1 remains blocked until the owner
  decides.
- Coordinates that become stale (verified in the current tree):
  `docs/m6-validation.md` (`cabfb9f`, full 40-hex), `docs/m0-verification.md`
  (`d1af757`, full 40-hex), `docs/pilots/cp7a/README.md` (`6449a42`) and
  `docs/m8-final-acceptance-review.md` (`6449a42`). Only the coordinates move;
  the frozen values and derivative hashes they record stay valid.
- M9 baseline: `m9-multirepo-release-candidate` (`a097e2db`) carries eight of
  the nine affected commits, and its `docs/m9-design-gate.md` still pins the
  pre-merge `master` tip `cc963ae`. M9 must re-baseline onto the rewritten
  `master` before it resumes.
- Evidence-hash impact: documents that pin internal git coordinates become
  stale (for example the CP7-A README source pin and the 40-hex commit
  references in `docs/m6-validation.md`). CP7-A file hashes are unaffected
  inside the current tree: frozen values stay recorded as historical and the
  derivative-hash map in `harness/verify.cjs` /
  `harness/verify-global-config.cjs` keeps checking the current bytes.

## GitHub Support purge is not guaranteed

The exposure here is local filesystem layout, not credentials. GitHub Support
may decline or only partially action a purge request for path-only content,
and unreferenced objects, PR refs, caches, logs, forks and existing clones can
retain old bytes indefinitely. The rewrite reduces reachable exposure; it
cannot guarantee erasure. Record this residual risk in the owner decision.

## Recontamination risks

- A stale clone (pre-rewrite history) that pushes afterwards would
  reintroduce the old blobs. Treat the backup mirror as read-only.
- Local branches, worktrees and forks that are not reset keep the old
  objects available.
- The pre-sanitization backups from the current-tree work are outside Git and
  must stay outside.
- CI caches and artifacts may contain old evidence archives; review and purge
  them if the exposure matters.

## Verification (must pass after the real run)

1. `security/scan.sh range <root>..HEAD` on every branch = clean.
2. Full-history scan with the final `.gitleaks.toml` = 0 findings
   (including the synthetic fixture example decision above).
3. Case-insensitive tree grep over `git rev-list --all` for the replaced
   prefixes = 0 hits.
4. `git rev-list --all --count` unchanged; DCO sign-off count unchanged.
5. `bash security/scan-tests.sh` passes on a fresh clone.
6. `git ls-remote origin` contains no unexpected surviving refs carrying old
   bytes (branches, tags, notes, pull refs).
7. File-level spot check of the known path-bearing files at their pre-rewrite
   SHAs (`gitleaks dir`), because the history scan does not report the
   synthetic fixture paths that a file scan does.

## Execution (owner-gated)

Precondition: the `protect-master` ruleset (id 25331026) carries a
`non_fast_forward` rule and a `deletion` rule for `refs/heads/master`, so the
force-push step below is rejected on `master` until the owner opens a ruleset
window. The other four branches are unprotected.

1. Freeze pushes; back up a bare mirror outside the repository.
2. Run the pinned `git-filter-repo --replace-text <full list> --force` on the
   mirror; re-add `origin`; verify per the list above.
3. Force-push the rewritten branches explicitly, one by one.
4. Re-clone locally (or `reset --hard` every branch) and remove stale
   worktrees.
5. Contact GitHub Support to purge unreferenced objects and stale PR refs;
   accept that path-only exposure may not be fully purged.
6. Re-run CI on all branches and confirm the `security` job passes.
