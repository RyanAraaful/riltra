# Security Policy

Riltra is local-first: it performs no source uploads, no telemetry, and no
network calls. Repository inspection uses read-only Git plumbing and never
executes a repository's code. Planned external tool integrations will run only
with explicit user opt-in and will be documented per integration.

## Reporting a vulnerability

Use the repository host's **private vulnerability reporting** feature for this
repo. Do not open a public issue for unpatched security bugs.

Include: affected version/commit, OS, reproduction steps, and impact.
We will acknowledge within 7 days and publish a fix + advisory once resolved.

## Scope notes

- The CLI has no network attack surface beyond the Rust toolchain itself.
- The repository registry confines every query to canonical, registered roots;
  moved or retargeted roots are rejected, and unreadable Git metadata is
  reported as invalid rather than guessed.
