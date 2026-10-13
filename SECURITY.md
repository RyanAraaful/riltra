# Security Policy (M0 draft)

Riltra is local-first: core performs no source uploads, no telemetry, and no
network calls. External language servers/indexers (M5+) run only with explicit
user opt-in and are documented per provider.

## Reporting a vulnerability

Use the repository host's **private vulnerability reporting** feature for this
repo. Do not open a public issue for unpatched security bugs.

Include: affected version/commit, OS, reproduction steps, and impact.
We will acknowledge within 7 days and publish a fix + advisory once resolved.

## Scope notes

- M0 has no network attack surface beyond the Rust toolchain itself.
- Path handling policy (repo-root confinement, symlink/junction validation)
  lands with M1 (`repo` registry); until then the CLI reads no repository
  files at all.
