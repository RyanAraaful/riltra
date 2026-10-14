# Dependency & License Inventory

Re-check sources at pin time before any release.
Policy: every bundled grammar, indexer binary, or vendored source needs an
entry here (package + exact version + license + origin URL + redistributability
note) in the same PR that introduces it. Never introduce code under
noncommercial/use-restricted licenses or from sources without clear rights.

Current state: tree-sitter parsing (M2) plus a durable SQLite index via
`rusqlite` with bundled SQLite (M3), all pinned and permissive; the project
started dependency-free (see history).

## Original baseline: zero third-party dependencies (pre-syntax)

| Component | Version | License | Origin | Redistributable? |
|---|---|---|---|---|
| Rust std only (no crates.io deps) | toolchain stable | MIT OR Apache-2.0 (std) | rust-lang/rust | yes — linked std, no vendoring |
| Build/test tooling (rustc, cargo, rustfmt, clippy) | stable (see CI log) | MIT OR Apache-2.0 | rust-lang | build-time only, not shipped |

The registry core stays std-only; parsing support arrived with pinned
dependencies (see M2 addition below), each with an SBOM entry.

## Reference notes for future introductions (NOT bundled — audit per package)

- tree-sitter core: MIT — each grammar licensed separately, audit per grammar.
- SCIP spec/tooling: Apache-2.0 — record proto origin + version.
- Rust MCP SDK (`rmcp`): license changed over time; keep the exact LICENSE of
  the pinned version.
- `gix`/gitoxide: MIT OR Apache-2.0.
- clangd/LLVM: Apache-2.0 WITH LLVM-exception — keep notices.
- rust-analyzer: MIT OR Apache-2.0. petgraph: MIT OR Apache-2.0 (optional algo only).

## Audit result (rebased at M3 — the zero-dependency claim below is history)

- `cargo tree --depth 1`: `rusqlite`, `tree-sitter`, and 6 grammar crates;
  full transitive set audited in the M2/M3 sections below.
- License risk: permissive-only (MIT / Apache-2.0 / Unlicense / Zlib /
  Unicode-3.0); no copyleft, no noncommercial code.

## M2 addition: tree-sitter syntax baseline (pinned versions, verified 2026-10-14)

Direct dependencies (all permissive; `Cargo.lock` pins exact versions):

| Package | Version | License | Origin |
|---|---|---|---|
| tree-sitter | 0.25.10 | MIT (declared in package metadata) | https://github.com/tree-sitter/tree-sitter |
| tree-sitter-python | 0.25.0 | MIT (LICENSE file, Max Brunsfeld 2016) | https://github.com/tree-sitter/tree-sitter-python |
| tree-sitter-javascript | 0.25.0 | MIT (LICENSE file, Max Brunsfeld 2014) | https://github.com/tree-sitter/tree-sitter-javascript |
| tree-sitter-typescript | 0.23.2 | MIT (declared; covers typescript + tsx) | https://github.com/tree-sitter/tree-sitter-typescript |
| tree-sitter-rust | 0.24.2 | MIT (LICENSE file, Maxim Sokolov 2017) | https://github.com/tree-sitter/tree-sitter-rust |
| tree-sitter-c | 0.24.2 | MIT (LICENSE file, Max Brunsfeld 2014) | https://github.com/tree-sitter/tree-sitter-c |
| tree-sitter-cpp | 0.23.4 | MIT (declared) | https://github.com/tree-sitter/tree-sitter-cpp |

Build-time transitive dependencies (compiled into the binary via `cc` build scripts):

| Package | Version | License |
|---|---|---|
| cc | 1.8.0 | MIT OR Apache-2.0 |
| find-msvc-tools | 0.1.14 | MIT OR Apache-2.0 |
| shlex | 2.0.1 | MIT OR Apache-2.0 |
| tree-sitter-language | 0.1.9 | MIT (LICENSE file) |
| streaming-iterator | 0.1.9 | MIT OR Apache-2.0 |
| memchr / regex / regex-syntax / regex-automata / aho-corasick | 2.8.3 / 1.13.1 / 0.8.11 / 0.4.18 / 1.1.5 | MIT OR Apache-2.0 / Unlicense OR MIT |
| serde_core / serde_json / itoa / zmij / indexmap / hashbrown / equivalent / foldhash | as pinned in `Cargo.lock` | MIT OR Apache-2.0 (zmij: MIT) |
| unicode-ident | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 |

Notes:

- Grammar `.scm` query files are NOT used or vendored: all captures are
  self-authored node-kind matching in `crates/cli/src/syntax.rs`.
- No grammar, parser source, or query file is copied into this repository;
  everything arrives via crates.io with pinned versions.
- `proc-macro2`/`quote`/`syn`/`serde`/`serde_derive` appear in the resolve graph
  but are not compiled into this feature set (absent from the build cache).

## M3 addition: durable SQLite index (pinned versions, verified 2026-10-14)

| Package | Version | License | Origin |
|---|---|---|---|
| rusqlite | 0.40.2 | MIT (declared in package metadata) | https://github.com/rusqlite/rusqlite |
| libsqlite3-sys | 0.38.2 | MIT (declared); bundled SQLite amalgamation is public domain | bundled via `rusqlite/bundled` |
| smallvec | 1.16.3 | MIT OR Apache-2.0 | transitive via rusqlite |
| bitflags | 2.13.2 | MIT OR Apache-2.0 | transitive via rusqlite |
| fallible-iterator | 0.3.0 | MIT/Apache-2.0 | transitive via rusqlite |
| fallible-streaming-iterator | 0.1.9 | MIT/Apache-2.0 | transitive via rusqlite |
| hashlink | 0.12.2 | MIT OR Apache-2.0 | transitive via rusqlite |
| hashbrown | 0.17.1 | MIT OR Apache-2.0 | transitive via rusqlite |

Notes:

- The `bundled` feature compiles SQLite from source (no system library);
  FTS5 availability is pinned by test (`fts5_is_available_in_bundled_sqlite`)
  on every platform.
- No graph database, vector store, or second database engine is used.
