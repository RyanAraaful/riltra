# M10 Task 5 — distribution decision record: Option B installers

**Date:** 2026-10-27
**Branch:** `m10-evidence-distribution`
**Baseline:** `aa348ecf4f6a3908a3ff3e8d2234f7c695048fb2`
**Scope:** M10 Task 5, Phase A only. No merge, tag, GitHub Release, publication,
signing, packaged-manager output or auto-update.

## 1. Decision

Option B, the existing M9 archive plus two minimal installers, is implemented
for Phase A. Option A (`dist`) was rejected by the owner from the feasibility
review under the `docs/m10-design-gate.md` §9.2 decision rule.

Option B keeps M9 unchanged: `release/package.sh`, `release/smoke.sh`, the
single-folder archive layout, the adjacent `.sha256`, and the
`release_docs --check` gate. This task adds only:

- `release/install.ps1` and `release/install.sh`;
- `release/test-install.ps1` and `release/test-install.sh`;
- the `installer-phase-a` matrix job in `.github/workflows/ci.yml`;
- the user instructions in `release/INSTALL.md`.

No dependency, `Cargo.toml`, `Cargo.lock`, product source, benchmark evidence,
`release-candidate.yml` guard, or release-publication behaviour changed.

## 2. Reported feasibility versus re-verified evidence

| Item | Reported before this task | Status in this task |
|---|---|---|
| Option A (`dist 0.33.1`) | Rejected: generated installers did not meet the §9.1 checksum-before-install and no-PATH-edit requirements. | Accepted as reported; no `dist` run. The retained generated PowerShell installer was inspected again: it has no checksum/`Get-FileHash` reference and, unless suppressed, writes the install directory into the `HKCU\Environment` `Path` value. Both conflict with §9.1. |
| Option B archive reuse | Feasible with the M9 archive and checksum. | Re-verified on Windows x64 and Linux x64 with archives built by `release/package.sh`; hosted native execution is reported in §6. |
| An earlier Option B draft layout | The first feasibility draft placed the documents in `~/.local/bin` next to the command. | Rejected by owner Correction A. This implementation keeps the payload and notices in `$HOME/.local/share/riltra` and puts only the `riltra` symlink in `$HOME/.local/bin`. |
| Checksum failure order | Not verified for Option A. | Re-verified for Option B: a missing, malformed or mismatched `.sha256` exits non-zero before archive listing, extraction or any payload move. The previous installation is unchanged and no staging directory remains. |
| Unsupported platforms | Not verified for Option A. | Re-verified for Option B: an unsupported OS or ARM64 exits non-zero, names the detected platform and the supported list, and installs nothing. |

The feasibility conclusion for Option A is therefore recorded as reported, with
one static re-check of the retained generated installer. No Option A benchmark,
release or CI result from this task is claimed.

## 3. Installation layout and ownership

| Platform | Command | Payload and documents |
|---|---|---|
| Windows x64 | `%LOCALAPPDATA%\Programs\Riltra\bin\riltra.exe` | Same directory |
| Linux x64 | `$HOME/.local/bin/riltra` (symlink) | `$HOME/.local/share/riltra/` |
| macOS Intel x64 | `$HOME/.local/bin/riltra` (symlink) | `$HOME/.local/share/riltra/` |

Rules enforced by both installers:

- No administrative rights, no elevation, no machine-wide path, no registry or
  `PATH` edit. The installer prints the `PATH` line the user may add.
- The private payload holds the binary and `LICENSE`, `NOTICE`,
  `THIRD_PARTY_LICENSES.txt`, `sbom.cdx.json`, `INSTALL.md`. The shared
  `~/.local/bin` directory receives only the symlink.
- The installed payload carries `.riltra-manifest`, the exact list of installed
  files. Reinstall moves the old payload aside and swaps in the new one only
  when every existing entry is manifest-owned. A foreign file, directory or
  link in the payload aborts the install before the swap.
- An existing `riltra` file or symlink is replaced only when the symlink points
  at `$HOME/.local/share/riltra/riltra`. The installer never clears or rewrites
  the whole `~/.local/bin` directory.
- Staging is created under the private install parent and removed on success,
  failure and interruption.

## 4. Archive and checksum checks

Both installers verify the `.sha256` file before extraction. The checksum must
be present, must contain 64 hexadecimal characters, must name the archive when
a file name is present, and must match the archive bytes. Failure exits non-zero
before any extraction or install.

Archive entries are rejected when they are absolute, contain `..` or `.`
components, use backslashes, escape the staging directory, contain more than one
top-level directory, or carry a symbolic-link or other non-regular type. The
payload must contain `riltra` (`riltra.exe` on Windows) and all five documents,
all non-empty. Both installers also refuse an archive whose file name is not a
`windows-x64` zip or a matching Linux/macOS x64 tar.gz.

## 5. Phase A test coverage

`release/test-install.sh` and `release/test-install.ps1` take a local archive
(`--archive` and `-ArchivePath`) and use a throwaway `HOME` or
`LOCALAPPDATA`. Each covers:

1. positive install into a fresh test home;
2. reinstall over the existing install;
3. checksum mismatch, missing and malformed, with the old installation
   preserved and no staging left;
4. missing checksum into a second fresh home, with nothing installed;
5. unsupported OS and ARM64 shims, with the detected platform and supported
   list named, and nothing installed;
6. install location under the test home; on POSIX every payload file is owned
   by the invoking user;
7. foreign executable, foreign symlink and foreign payload file;
8. path traversal, absolute entry and archive symlink;
9. static no-elevation check (`sudo`, `RunAs`, `/usr/local`, `HKLM`,
   `Program Files`, registry or `PATH` edit);
10. the existing `release/smoke.sh` CLI/MCP smoke against the installed binary.

Windows runs the positive install under Windows PowerShell 5.1 and the
reinstall and negative cases under PowerShell 7. POSIX runs under `sh` for the
installer and `bash` for the test harness.

## 6. Evidence

Local Phase A, before the hosted run:

- Windows x64: Windows PowerShell 5.1 and PowerShell 7, fresh archive
  `riltra-0.1.0-windows-x64.zip` with SHA-256
  `5ba255d39edebcd9cc4bd45ba07113572202c899470bdcdf1c0ee82cd6a2e306`.
  All checks passed, including the installed CLI/MCP smoke.
- Linux x64: WSL Ubuntu (x86_64) with the M9 Linux archive; all checks passed,
  including the installed CLI/MCP smoke.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test --locked`, `cargo build --release --locked`, and
  `cargo run --locked -q --example release_docs -- --check` all passed.
- `security/scan.sh tree` and `security/scan-tests.sh` passed.

Hosted Phase A is the `installer-phase-a` job in `.github/workflows/ci.yml`,
with one job per pinned native runner:

- `installer (windows-2025)`
- `installer (ubuntu-24.04)`
- `installer (macos-15-intel)`

Each job builds with `cargo build --release --locked`, runs
`release_docs --check`, packages with `release/package.sh`, and runs the
platform test above. The required `check (windows-latest)`,
`check (ubuntu-latest)` and `check (macos-15)` contexts are unchanged.

Hosted run evidence is added by a follow-up DCO commit once the run exists.
Until that evidence is recorded, the hosted result is **pending**, not PASS.

## 7. Limitations

- Phase B (download from a published GitHub Release) is not tested and needs a
  separate owner authorization and a published pre-release. The installer's
  download path exists but Phase A always uses a local archive.
- Native macOS and Ubuntu 24.04 execution is available only from the hosted
  runners. WSL is not a substitute for native Ubuntu, and no local macOS host
  exists.
- GitHub-hosted Windows runners run as an administrator. The no-admin property
  rests on the location and static checks, not on an unprivileged account.
- macOS ARM64 and Linux ARM64 are refused by design; no ARM64 archive is built
  or claimed.
- No code signing, notarization, package manager, GUI installer, auto-update or
  PATH modification is provided.
- The installed location keeps the notices because the owner requires them
  there; Option A's binary-only layout was not adopted.

## 8. Status

- Option B implementation: PASS on the local Windows and Linux x64 Phase A
  runs listed in §6.
- Hosted Windows, Ubuntu and macOS Phase A: pending the CI run recorded after
  the first push.
- Owner audit readiness: ready for review of the decision, installers, tests
  and CI job. Merge, tag, release and Phase B remain separate owner decisions.
