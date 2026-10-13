# M1 Verification Report (auditable)

Scope: blueprint `OPEN_SOURCE_CODE_INTELLIGENCE_KNOWLEDGE_BLUEPRINT.md` §15 (M1) +
backlog #2/#3. Follow-up hardening of M1 at code commit `1dcedba`
(this doc committed separately; doc-only commits never change gates).
Method: TDD red→green locally (Rust 1.99.0, Windows); every claim backed by a
test name or command output below. No M2 code. No push without approval, so the
CI gate below is PENDING, not claimed.

## Blueprint acceptance → evidence

| # | Blueprint M1 requirement | Evidence | Status |
|---|---|---|---|
| A1 | `init`, `repo list` register + machine-readable JSON | `tests/cli.rs:init_and_list_round_trip` (round-trip, idempotent re-add, sorted `{"repos":[...]}`) | PASS (local) |
| A2 | `repo status`: Git fingerprint (HEAD/branch/dirty/untracked/worktree) | `git::fingerprints_clean_repo_with_branch`, `detects_dirty_and_untracked`, `detects_linked_worktree`, `tests/cli.rs:status_git_repo_reports_head_and_branch` | PASS (local) |
| A3 | dirty/untracked/branch cases | same as A2 | PASS (local) |
| A4 | symlink/junction cases | `paths::contains_rejects_symlink_escape`, `status_rejects_windows_junction_retarget` (mklink /J, real on Windows), `status_rejects_symlink_retarget_outside_root` (unix) | PASS local, platform-split (see §Skips) |
| A5 | permission failure case | `init_fails_when_registry_home_is_a_file` (portable), `unreadable_git_metadata_is_invalid` (unix chmod 000) | PASS local, platform-split |
| A6 | corrupt repo case | `non_git_…_and_corrupt_is_invalid` (dir + file variants), `status_corrupt_git…` + `status_corrupt_gitfile…` (exit 3, `valid:false`) | PASS (local) |
| A7 | repo isolation proven | `repos_are_isolated_by_name`, `name_clash_across_roots_is_rejected` | PASS (local) |
| A8 | ignore policy | `ignore::loads_file…`, `matches_exact_and_dir_prefix_only`, status carries `ignore` counts | PASS (local) |
| A9 | path security | name validation, canonical records, canonical-match on status (gap 1), doctor checks | PASS (local) |
| A10 | fmt/check/clippy/test green + CI Windows/Linux/macOS | local: fmt exit 0, check ok, clippy `-D warnings` exit 0, 39 unit + 14 integration pass. CI run8 (`be762f2`): windows/ubuntu/macos `completed+success` (jobs API, 2026-10-14) | PASS |

## Audit gaps (this change) — red→green

1. **Canonical-match on `repo status`** (`main.rs:cmd_repo_status`): rejects moved
   roots and symlink/junction retargets (exit 3, empty stdout) instead of
   fingerprinting the wrong directory. RED watched: junction test failed pre-fix
   (`assertion left == right failed`, exit was 0); GREEN post-fix.
2. **Status evidence** (`git.rs:fingerprint`): `valid` now requires a successful
   `git status` run. Fixture: truncated `.git/index` (probe: status exit 128,
   rev-parse exit 0) with a real modification present — old code reported
   `valid:true` + clean (missed dirt). RED watched (`status_failure_is_never_clean_nor_valid`
   failed pre-fix); GREEN post-fix.
3. **UTF-8 multibyte registry parsing** (`registry.rs:Parser::string`): cursor now
   advances by `ch.len_utf8()` instead of one byte. RED watched
   (`round_trips_unicode_names_and_paths` failed: `tëst-üñî-🦀` unloadable);
   plus `parses_unicode_escapes_to_same_characters` pin. GREEN post-fix.
4. **Regression inventory**: junction (Windows mklink /J, hard-fail, no skip),
   symlink retarget, moved root, registry-home-is-file, unix chmod 000,
   name clash, corrupt gitfile — all in `tests/cli.rs`; unicode round-trip in
   `registry.rs`; status-failure in `git.rs`.

## Security gap closures (audit follow-up, code commit `d675690`)

| # | Gap | Mitigasi minimum + bukti | Status |
|---|---|---|---|
| S1 | `git status` mengeksekusi `core.fsmonitor` milik repo (RCE sebagai user) | Override env kosong (`GIT_CONFIG_COUNT/KEY_0/VALUE_0`) di `git_out` (`git.rs`) — berlaku untuk `status` dan `doctor`. Probe: payload forward-slash tanpa mitigasi → marker TERBUAT; dengan mitigasi → marker absen + exit 0 + rev-parse ok. Cakupan minimum: hanya `core.fsmonitor` yang dioverride — tidak ada command kami (rev-parse/status/branch) yang menjalankan hooks, pager, editor, atau network helper; re-audit saat menambah command. Test: `git::status_never_runs_repo_configured_fsmonitor` (RED disaksikan: marker dibuat pre-fix), `tests/cli.rs:status_and_doctor_never_run_fsmonitor` | PASS (local); unix-payload RED diwakilkan mekanisme hook yang sama, CI unix mengonfirmasi GREEN |
| S2 | `.riltraignore` mengikuti symlink/junction keluar root | `load_rules` resolve sekali + wajib containment via `contains` (`ignore.rs`); escape dibaca absent (konservatif). `contains` kini dipakai produksi (M1-enforced). Test: `ignore_symlink_outside_root_is_not_followed`, `ignore_symlink_retarget_outside_is_not_followed`, `ignore_hardlink_inside_is_read_not_falsely_rejected` (tanpa privilege, jalan di semua OS), `status_ignores_symlinked_ignore_outside_root` | PASS (local); symlink-file skip di Windows privilege (lihat §Skips), evidensi symlink dari CI unix |

## Skips (explicit, not PASS evidence)

- `status_rejects_symlink_retarget_outside_root`: `symlink_dir` needs privilege
  on Windows → skipped on this machine (vacuous pass). Real symlink-retarget
  evidence must come from Linux/macOS CI. Junction evidence comes from Windows CI.
- `unreadable_git_metadata_is_invalid`: `#[cfg(unix)]` only; Windows ACL case unverified.
- `contains_rejects_symlink_escape` (unit): same privilege skip on Windows.
- Tests skip (not fail) when the `git` executable is absent.
- `ignore_symlink_outside_root_is_not_followed`,
  `ignore_symlink_retarget_outside_is_not_followed`, and
  `status_ignores_symlinked_ignore_outside_root`: file-symlink creation needs
  privilege on Windows → skipped on this machine (vacuous pass). Real
  symlink-escape evidence for the ignore file must come from Linux/macOS CI.
  Junction evidence for roots comes from Windows CI (`mklink /J`, no privilege).
  The hardlink test needs no privilege and runs on all OSes.

## Invariants kept

- 0 dependencies (`Cargo.lock`: exactly one `[[package]]`).
- No Tree-sitter/graph/MCP/knowledge code (same grep audit as M1).
- No validation weakened: fixes strictly narrow acceptance (new rejections, no new allowances).
- Exit codes unchanged: 0 ok, 2 usage, 3 repository/data.

## Final status: **PASS** — local gates PASS (59 tests: 43 unit + 16 integrasi), CI run10 PASS (3/3 OS)

- Verified code commit: `d675690` (`fix: M1 close fsmonitor execution and ignore-file escape gaps`).
- CI evidence: run10 on `bdfc160`, windows/ubuntu/macos `completed+success`
  (jobs API). Unix symlink-ignore + sh-fsmonitor payloads confirmed GREEN on
  Linux/macOS runners; junction + hardlink + ps1-fsmonitor GREEN on Windows.
  Push approved by owner; remote HEAD tracks local.
- M2 not started.
