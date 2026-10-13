# Riltra

[![ci](https://github.com/RyanAraaful/riltra/actions/workflows/ci.yml/badge.svg)](https://github.com/RyanAraaful/riltra/actions)

Local-first code intelligence and engineering knowledge for developers and
AI coding agents. Riltra runs entirely on your machine: no accounts, no cloud,
no telemetry. It is free and open-source software under the Apache License 2.0,
suitable for personal, organizational, and commercial use.

## Why Riltra

Understanding a codebase means answering three questions: *what does this code
do, what breaks if I change it, and why was it built this way?* Riltra is being
built to answer all three locally — with evidence pointing at your source, in
compact output that humans can skim and coding agents can consume without
burning context.

For **developers**, that means definitions, callers, impact analysis, and the
reasoning behind past decisions, without reading the whole repository.
For **AI coding agents**, that means small, source-backed answers instead of
thousands of dumped lines.

## Available today

The current binary already does the following, offline, with no runtime
dependencies beyond itself:

- **Repository registry** — register local project roots (`init`) and list
  them as deterministic JSON (`repo list`).
- **Git fingerprint** (requires a Git executable) — per repository: HEAD commit,
  branch, dirty/untracked state, worktree detection, and ignore-file summary
  (`repo status`).
- **Health checks** — path confinement, record integrity, and git readability
  checks with explicit pass/fail (`doctor`).
- **Machine-readable output** — stable JSON with fixed key order, meaningful
  exit codes (`0` ok, `2` usage error, `3` repository/data error), and an
  optional `.riltraignore` file per repository.
- **Syntax baseline** — per-file definitions, imports, and unresolved call
  candidates with line/column spans and byte offsets for Python, JavaScript,
  TypeScript, Rust, C, and C++ (`scan`); other extensions are reported as
  unsupported, never guessed.

## Quick start

Prerequisites: a stable Rust toolchain and `git`. No releases are published
yet — build from source:

```powershell
git clone https://github.com/RyanAraaful/riltra.git
cd riltra

cargo run -q -p riltra -- init --repo . --name riltra
cargo run -q -p riltra -- repo list
cargo run -q -p riltra -- repo status --repo riltra
cargo run -q -p riltra -- doctor --repo riltra
cargo run -q -p riltra -- scan --repo riltra --path crates/cli/src/main.rs
```

`repo status` prints one JSON line per repository, for example (trimmed):

```json
{"name":"riltra","root":"…","git":{"present":true,"valid":true,"head":"…","branch":"main","dirty":false,"untracked":0}}
```

Notes:

- The registry lives in `%APPDATA%\riltra` (Windows) or `~/.config/riltra`
  (Linux/macOS). Set `RILTRA_HOME` to point it elsewhere (useful for scripts
  and tests).
- Git fingerprints need a Git executable: without one, `init` and `repo list`
  still work, while `repo status` reports `valid:false` and `doctor` flags the
  missing executable — nothing is silently faked.
- Run `riltra --help` for the full command list.

## Roadmap (not yet implemented)

The following are planned directions, not working features and not dated
promises:

- Syntax-aware code search across multiple languages.
- Cross-file call/reference/dependency graph with source evidence.
- Opt-in semantic providers (language servers / index formats) for deeper accuracy.
- Persistent, revisioned knowledge graph for decisions, constraints, and benchmarks.
- An MCP server so AI agents can query code and knowledge through one interface.

## Project status

Riltra is in early, pre-release development. Continuous integration runs format,
compile, lint, and test checks on Windows, Linux, and macOS for every change.
See [docs/m1-verification.md](docs/m1-verification.md) for the latest verified
capabilities and their evidence.

## Contributing

Contributions are welcome under the Apache License 2.0 (inbound = outbound).
Every commit needs a `Signed-off-by` line (`git commit -s`), and
`cargo fmt`, `cargo clippy`, and `cargo test` must pass. Details live in
[CONTRIBUTING.md](CONTRIBUTING.md).

## Security

Riltra is local-first: it uploads nothing, phones home to nothing, and never
executes a repository's code to inspect it. Future external tool integrations
will be explicit opt-in only. Please report vulnerabilities through the
repository host's private vulnerability reporting — see [SECURITY.md](SECURITY.md).

## License

Apache License 2.0 — see [LICENSE](LICENSE) and [NOTICE](NOTICE).
