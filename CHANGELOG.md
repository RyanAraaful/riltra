# Changelog

All notable changes to Riltra are recorded here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html). Before 1.0, the [stability notes](README.md#stability-pre-10) say what stays fixed within `0.1.x`.

## [Unreleased]

Changes made after `0.1.0` is cut are listed here.

## [0.1.0] - 2026-11-04

First pre-release. The version number is already in the build. Native archives for Windows x64, Linux x64 and macOS Intel x64 are built by a manually started workflow ([release/README.md](release/README.md)).

### Added

- Repository registry and a read-only Git fingerprint (`init`, `repo list`, `repo status`), and `doctor` health checks.
- Deterministic JSON output with a fixed key order and exit codes `0`, `2` and `3`; an optional per-repository `.riltraignore`.
- Syntax facts with line, column and byte spans (definitions, imports, call candidates) for Python, JavaScript, TypeScript and TSX, Rust, C and C++. Other extensions are reported as unsupported, never guessed (`scan`).
- A durable SQLite code index with source digests, provenance, full-text symbol search and explicit fresh, stale or unknown freshness (`index`, `search`).
- Relationship queries `callers`, `callees`, `trace` and `impact`, labelled by evidence: syntax-only `POTENTIAL_CALL` candidates, source-checked `CALLS` edges from a semantic provider, and impact answers that stay `UNKNOWN` or `partial` unless proven. For C++, `search --occurrences` lists where an exact name occurs as a labelled census.
- Opt-in semantic providers: SCIP import (`semantic import`) and queries to a language server you name by absolute path (`semantic query --lsp-server`).
- A knowledge store of revisioned records per repository, code-anchor bindings (`knowledge bind`, `unbind`, `at`), deterministic export, backup, verification and fail-closed restore.
- An MCP server over local stdio (`riltra mcp`) with thirteen tools: twelve read-only and one provider-backed (`riltra_code_semantic`).
- Release packaging: archives, SHA-256 checksums, third-party notices, a software bill of materials, and user-level installers for Windows (PowerShell) and for Linux and macOS (shell) ([release/INSTALL.md](release/INSTALL.md)).

### Scope and limits of the evidence

- The evidence behind this release comes from fixtures, pilots and CI checks. It supports no performance or accuracy claim beyond the measured corpus, no comparison with other products, and no claim of superiority ([M10 consolidation, section 10](docs/m10-consolidation.md#10-public-reporting-boundary)).
- In the agent pilot, 24 slots were planned and 18 reached an agent turn; no agent made a Riltra call in any run of the arm that had Riltra available, so the pilot says nothing about benefit to agents ([section 4.1](docs/m10-consolidation.md#41-totals)).
- The installers were checked against locally built archives; the check against a published release has not run ([section 6](docs/m10-consolidation.md#6-release-readiness)).
- Four C++ capability criteria are unmet, and macOS coverage is x64 only ([Known limitations](README.md#known-limitations)).
