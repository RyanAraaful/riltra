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
- **Durable code index** - SQLite-backed snapshots of the syntax facts with
  source digests, provenance, full-text search, and explicit
  fresh/stale/unknown freshness (`index`, `search`); one database file per
  repository under the local Riltra home.
- **Relationship graph** - `callers`, `callees`, `trace` and `impact` answer
  with evidence and explicit labels. Syntax-only relations are
  `POTENTIAL_CALL` candidates matched by name, never proven calls; only a
  semantic provider's source-checked evidence makes a `CALLS` edge, and an
  impact answer is `UNKNOWN` and `partial` unless it is proven. A C++ data
  member is a valid anchor, and `search --occurrences` lists where an exact
  name occurs (member accesses, the address of a member, same-file macro
  arguments) as a labelled name census, not a binding.
- **Opt-in semantic providers** - import a SCIP index (`semantic import`) or
  query a language server you name by absolute path (`semantic query
  --lsp-server`); the answers carry provider evidence and are never silently
  trusted.
- **Knowledge store** - revisioned decision/constraint/benchmark records per
  repository with deterministic export, backup, verification and fail-closed
  restore (`knowledge ...`).
- **MCP server** - `riltra mcp` serves twelve tools over local stdio: eleven
  read-only ones and one provider-backed tool that starts the operator's
  language server. Reads never create or migrate a store: a missing
  index returns `no-index` without creating files, and an unsupported schema is
  refused as `schema_outdated` with a CLI migration instruction.

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
cargo run -q -p riltra -- index --repo riltra
cargo run -q -p riltra -- index status --repo riltra
cargo run -q -p riltra -- search --repo riltra --symbol version --limit 5
```

`repo status` prints one JSON line per repository, for example (trimmed,
illustrative values):

```json
{"name":"riltra","root":"…","git":{"present":true,"valid":true,"head":"…","branch":"master","dirty":false,"untracked":0}}
```

`scan` reports symbol names with source locations, for example (trimmed real
output — `…` marks omitted facts):

```json
{"path":"crates/cli/src/main.rs","status":"ok","language":"rust",
 "definitions":[{"kind":"function","name":"version_line",
 "span":{"start_line":38,"start_col":1,"end_line":40,"end_col":2},"bytes":[1464,1527]}, …],
 "calls":[{"kind":"call","name":"registry::load",
 "span":{"start_line":58,"start_col":25,"end_line":58,"end_col":50},"bytes":[2044,2069]}, …],
 "errors":0}
```

Call names are recorded exactly as written (`w.process`, `registry::load`)
and are never resolved to targets — they are candidates for you or your agent
to follow, not claims.

Notes:

- The registry lives in `%APPDATA%\riltra` (Windows) or `~/.config/riltra`
  (Linux/macOS). Set `RILTRA_HOME` to point it elsewhere (useful for scripts
  and tests).
- Git fingerprints need a Git executable: without one, `init` and `repo list`
  still work, while `repo status` reports `valid:false` and `doctor` flags the
  missing executable — nothing is silently faked.
- Run `cargo run -q -p riltra -- --help` for the full command list.

## MCP server (eleven read-only tools, one provider-backed tool)

`riltra mcp` starts a local stdio MCP server for coding agents. It exposes
twelve tools:

- `riltra_repo_list`, `riltra_repo_status`, `riltra_health`
- `riltra_code_search`, `riltra_code_callers`, `riltra_code_callees`,
  `riltra_code_trace`, `riltra_code_impact`
- `riltra_knowledge_search`, `riltra_knowledge_show`, `riltra_knowledge_related`
- `riltra_code_semantic` — definition or references at a position, answered by
  the language server the operator configured in `RILTRA_LSP_SERVER` (an
  absolute path). It is the one tool that starts a subprocess: the server may
  write its own index cache and may reach the network, so the server is no
  longer read-only as a whole. Riltra itself still creates, migrates and
  replaces no database through it.

The server never creates, migrates, or replaces a database. A missing index or
knowledge store returns its documented empty/`no-index` result without creating
files; a store with an unsupported schema is refused as `schema_outdated`
without modification, with a CLI migration instruction. Results keep the same
freshness, provenance, and `POTENTIAL_CALL`/`CALLS` semantics as the CLI.

Minimal OpenCode configuration for the local stdio server:

```jsonc
{
  "mcp": {
    "servers": {
      "riltra": {
        "type": "local",
        "command": ["riltra", "mcp"],
        "environment": { "RILTRA_HOME": "<your Riltra home>" }
      }
    }
  }
}
```

See [docs/pilots/cp7a/README.md](docs/pilots/cp7a/README.md) for the CP7-A
benchmark report, raw evidence, and limitations.

## Language support

| Language | Extensions | Syntax facts | Name occurrences | Semantic evidence |
|---|---|---|---|---|
| Python | `.py` | definitions, imports, call candidates | no | SCIP import (contract only) |
| JavaScript | `.js` `.mjs` `.cjs` | same | no | SCIP import (contract only) |
| TypeScript / TSX | `.ts` `.mts` `.cts` `.tsx` | same | no | SCIP import, verified on a fixture (scip-typescript) |
| Rust | `.rs` | same | no | SCIP import, verified on a fixture (rust-analyzer) |
| C | `.c` | same | no | SCIP import (contract only) |
| C++ | `.cc` `.cpp` `.cxx` `.h` `.hh` `.hpp` `.hxx` | same, plus classes, structs and data members | yes | language server query (clangd, operator-configured); not verified on a real project |
| CUDA | `.cu` `.cuh` | reported as unsupported, never parsed | no | none |

Other extensions are reported as unsupported, never guessed. Headers parse as
C++.

## Known limitations

- Syntax-only answers are candidates. Without a semantic provider, `callers`
  match call sites by name inside a scope of related files, `trace` ends at the
  anchor (no edge has a resolved target), and `impact` is `UNKNOWN`.
- There is no C++ semantic resolution out of the box and no CUDA support. A C++
  project needs a compilation database and a language server you configure.
- Macro interiors and C++ templates are opaque to the syntax layer. `.inl`
  files are parsed as C++ headers; a fragment that needs its header context
  keeps its syntax errors in the `errors` count. `.ipp` and `.tpp` files are
  not indexed.
- `search --occurrences` is a repository-wide census of an exact name. An empty
  list is not proof that a name is unused.
- macOS continuous integration covers the x64 pool only; macOS ARM64 is not
  covered.
- The repository history still carries machine-local paths from early commits
  (a history rewrite is a separate, owner-gated decision).

## Stability (pre-1.0)

Within `0.1.x` the exit codes (`0` ok, `2` usage, `3` repository/data), the
fixed key order of every JSON document and the MCP tool names are stable; byte
goldens enforce them. New keys appear only behind an opt-in flag or are
appended to the end of a document, and a change to an existing key is made in a
minor version and written in the release notes. The on-disk index schema is
versioned: an older index reads as stale or `schema_outdated` and is rebuilt
with `index`, never silently reused.

## Project status

Riltra is in pre-release development; `0.1.0` has release candidates but no
published release. Continuous integration runs format, compile, lint, and test
checks on Windows, Linux, and macOS (x64) for every change. A manually started
workflow builds the native archives of a release candidate, checks each with a
clean-install smoke and writes their checksums; see
[release/README.md](release/README.md). The M9 evidence is under `docs/m9-*.md`.

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
