# M10 amendment: Phase B installer checks against the published 0.1.0 release

**Date:** 2026-11-04
**Status:** dated amendment to [M10 consolidation](m10-consolidation.md). It records a result. It changes no earlier document, no source file and no release asset.
**Scope:** the installer checks that [M10 design gate](m10-design-gate.md) section 9.4 calls Phase B: the Phase A checks, run on assets downloaded from the GitHub Release URL.

## 1. What this amends

[M10 consolidation](m10-consolidation.md) sections 5.3 and 6 record "Phase B: NOT TESTED". At that time no pre-release existed, and Phase B needed two owner decisions: a published pre-release (O-13) and authorization to download its assets (O-12). Both were given on 2026-11-04. The `v0.1.0` tag and the pre-release were published that day, and Phase B then ran once. The earlier documents stay as written. This amendment supersedes only their statement that Phase B has not run, and only for the `0.1.0` assets.

## 2. What was run

The manual workflow [`installer-phase-b`](../.github/workflows/installer-phase-b.yml), added in pull request 35 (merge commit `87d1304`), was started by hand from `master` with version `0.1.0`: [run 41797080889](https://github.com/terapetralabs/riltra/actions/runs/41797080889), 2026-11-04. It checks out the tag `v0.1.0` (commit `e482347`), so the install scripts under test are the ones the release notes link to. It does not run on pull requests, so this run was also the first execution of its acceptance and installer steps.

On each of three GitHub-hosted x64 runners it does three things:

1. Downloads the release archive and its `.sha256` file from the release URL with the runner's own tools, and verifies the checksum.
2. Runs the acceptance suite `release/test-install.*` on that downloaded archive: install and reinstall, refusal of bad checksums, unsupported platforms and unsafe archives, the static check that the installer needs no elevation, and a smoke run of the installed CLI and MCP server.
3. Runs the installer once the way a user does, with no local archive, in a fresh home directory (a fresh `LOCALAPPDATA` on Windows). The installed binary must report `riltra 0.1.0` and must be byte-identical to the binary inside the archive from step 1.

The workflow has read-only permissions (`contents: read`). It creates no tag, release, artifact or commit, and it uses no secret.

## 3. Result

| Runner | Job time | Acceptance checks | Download, checksum and user-style install |
|---|---|---|---|
| `windows-2025` | 44 s | 35 `ok`, none failed | passed |
| `ubuntu-24.04` | 14 s | 44 `ok`, none failed | passed |
| `macos-15-intel` | 19 s | 44 `ok`, none failed | passed |

Technical result: **PASS** on all three runners, for the `0.1.0` assets.

## 4. How the result was checked

A green job is not enough evidence here. The whole run took 48 seconds and the workflow guards its steps by operating system, so a job could finish green with the intended steps skipped. The run was therefore read step by step:

- On each runner the steps for its own operating system ran and succeeded, and the steps for the other system were skipped. No intended step was skipped.
- The checkout on each runner was at `e482347`, the commit of the tag.
- On Linux and macOS the checksum step printed `OK` for the downloaded archive. The Windows step prints nothing when it passes; it stops the job if the SHA-256 of the downloaded archive differs from the listed one, and it succeeded.
- The acceptance output of each runner was compared with the same suite in the Phase A `installer` jobs of CI run [41796972858](https://github.com/terapetralabs/riltra/actions/runs/41796972858), on the same commit of `master`, where the archive is built locally. The `ok` checks and their order are identical on all three systems (44, 44 and 35), and no line reports `not ok`.
- The user-style install step succeeded, so its three checks held: the installer created the expected files, the installed binary reported `riltra 0.1.0`, and the binary's SHA-256 equals that of the binary in the archive.

## 5. Limits

- GitHub-hosted Windows runners run as an administrator. As recorded in [M10 consolidation](m10-consolidation.md) section 5.3, the no-administrator property rests on the install location and the static no-elevation check, not on an install under an unprivileged account. No least-privilege claim follows from this result.
- Only x64 was covered. The macOS runner is Intel. macOS ARM64 and Linux ARM64 are refused by design, and no ARM64 archive exists.
- Only version `0.1.0` was checked, in one run on one day, with the install scripts as they are at the tag. A later release needs its own run.
- The Windows suite and the Linux and macOS suite are different scripts, with 35 and 44 checks. For example, the Windows list has no check for absolute-path or symbolic-link archive entries, and the other list has no PowerShell-version runs.
- The step summary that the workflow also writes was not read. The findings above come from the run logs, which GitHub keeps for a limited time.
- Everything else recorded in [M10 consolidation](m10-consolidation.md) still stands: Linux and macOS evidence is hosted-only, nothing here is a performance or accuracy claim, and there is no code signing, notarization or package manager.

This amendment does not approve a final or stable release.

## 6. Other documents

The [roadmap](../ROADMAP.md) no longer lists the check against the published assets as open, and the [changelog](../CHANGELOG.md) records the result under Unreleased. The earlier records named above are not edited.

## 7. Repeating the check

To repeat it for another published release that uses the same asset names, start the workflow from `master` with that version as input (`gh workflow run installer-phase-b.yml --ref master -f version=<version>`), then read the step list and the acceptance output as described in section 4.
