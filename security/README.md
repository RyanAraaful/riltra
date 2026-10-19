# Security scans (secrets + local machine paths)

Pinned scanner: **gitleaks 8.30.1** (MIT, github.com/gitleaks/gitleaks), plus
two custom rules in `.gitleaks.toml` for local machine paths: Windows
user-profile paths and unix/macOS home directories, in backslash,
forward-slash and JSON-escaped forms.

## What runs where

| Trigger | Command | Coverage |
| --- | --- | --- |
| pre-commit hook | `security/scan.sh staged` | staged changes |
| pre-push hook | `security/scan.sh outgoing` | commits being pushed + pushed tip tree |
| CI `security` job (push/PR) | `security/scan.sh range <before/base>..<head>` + `security/scan.sh tree` | new commits + whole committed tree |
| manual | `security/scan.sh tree \| range A..B \| dir <path>` | as requested |

Exit codes: `0` clean, `1` findings, `3` scanner unavailable (fail-closed),
`4` usage or config error. Findings are printed redacted; secret values are
never echoed.

Environment: `GITLEAKS_BIN` (scanner binary; default `gitleaks` from `PATH`),
`GITLEAKS_CONFIG` (default: this repository's `.gitleaks.toml`).

## Install the pinned scanner

Download the v8.30.1 asset for the platform and verify it against the
published `gitleaks_8.30.1_checksums.txt` from the release page, e.g. Linux
x64:

    curl -sSfL -o /tmp/gitleaks.tar.gz \
      https://github.com/gitleaks/gitleaks/releases/download/v8.30.1/gitleaks_8.30.1_linux_x64.tar.gz
    echo "551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb  /tmp/gitleaks.tar.gz" | sha256sum -c -
    tar -xzf /tmp/gitleaks.tar.gz -C "$HOME/.local/bin" gitleaks

On Windows unpack `gitleaks_8.30.1_windows_x64.zip` from the same release and
point `GITLEAKS_BIN` at `gitleaks.exe` from Git Bash, e.g.
`GITLEAKS_BIN=/c/tools/gitleaks.exe`.

## Enable the local hooks (once per clone)

    git config core.hooksPath .githooks

Hooks are a local guard, not enforcement: they can be bypassed with
`git commit --no-verify` / `git push --no-verify`. The CI `security` job is
the backstop; configuring it as a required check on the default branch is a
repository-settings action for the owner.

## Tests and allowlists

    bash security/scan-tests.sh

The suite copies synthetic fixtures (`security/testdata/`) to temporary
directories and checks: blocked fixtures (path and credential shapes),
allowed fixtures, secret redaction, fail-closed behavior when the scanner is
missing, ignored-but-committed files, staged changes, and a clean-tree check
of this repository.

Allowlists in `.gitleaks.toml` are intentionally narrow and documented inline:
synthetic fixtures under `security/testdata/`, CI runner profile paths
(`runner` / `runneradmin`, runner-generated and not personal), synthetic
unit-test paths from the project's own test data, and two reviewed false
positives (40-hex commit SHAs matched by the `sourcegraph-access-token`
heuristic, and SHA-256 integrity constants matched by `generic-api-key`). No
blanket allowlists (directories, `*.log`, `*.json`) are used.
