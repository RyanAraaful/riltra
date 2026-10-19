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
- At M2, `proc-macro2`/`quote`/`syn`/`serde`/`serde_derive` appeared in the
  resolve graph but were not compiled into that feature set (absent from the
  build cache). Since M7, `serde` is a direct dependency and is compiled into
  the binary (see the M7 addition below); since M8 CP3 the proc-macro trio is
  compiled via `rmcp-macros` (see the CP3 addition below).

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

## M7 addition: structured knowledge store (pinned versions, verified 2026-10-18)

| Package | Version | License | Origin |
|---|---|---|---|
| serde | 1.0.229 | MIT OR Apache-2.0 (declared in package metadata) | https://github.com/serde-rs/serde |

Notes:

- `serde` became a **direct dependency** in M7 (`serde = "1"` in
  `crates/cli/Cargo.toml`), used for strict JSON validation of knowledge
  records (`crates/cli/src/knowledge.rs`, `crates/cli/src/main.rs`). It is
  compiled and linked into the binary.
- Arrives via crates.io and is pinned in `Cargo.lock`; nothing is vendored.
  Redistributable: MIT OR Apache-2.0, permissive, no copyleft.
- Only the base `serde` feature set is enabled (the `derive` feature is off),
  so `serde_derive` was not compiled at M7; since M8 CP1 it is compiled as
  part of the rmcp/schemars graph (see below).

## M8 CP1 addition: MCP stdio server runtime (pinned versions, verified 2026-10-18)

Direct dependencies (owner decision D1; `rmcp` with `default-features = false`
and only the `server` + `transport-io` features — no HTTP/SSE transports,
macros, base64, auth, or elicitation):

| Package | Version | License | Origin |
|---|---|---|---|
| rmcp | 3.5.1 | Apache-2.0 (declared in package metadata; the crate package ships no standalone LICENSE file) | https://github.com/modelcontextprotocol/rust-sdk |
| tokio | 1.53.3 | MIT (declared) | https://github.com/tokio-rs/tokio |

Transitive additions pulled in by the rmcp graph (`Cargo.lock` diff at CP1: 44
new entries, none removed; all permissive, nothing vendored):

| Package | Version | License |
|---|---|---|
| futures / futures-channel / futures-core / futures-executor / futures-io / futures-macro / futures-sink / futures-task / futures-util | 0.3.34 | MIT OR Apache-2.0 |
| tracing / tracing-attributes / tracing-core | 0.1.44 / 0.1.31 / 0.1.36 | MIT |
| tokio-macros / tokio-util | 2.7.2 / 0.7.20 | MIT |
| schemars / schemars_derive / serde_derive_internals | 1.2.3 / 1.2.3 / 0.30.0 | MIT |
| uuid | 1.28.0 | Apache-2.0 OR MIT |
| chrono + platform crates (iana-time-zone, iana-time-zone-haiku, android_system_properties, core-foundation-sys, windows-core, windows-implement, windows-interface, windows-link, windows-result, windows-strings) | 0.4.45 (platform crates as pinned) | MIT OR Apache-2.0 |
| getrandom / r-efi / libc / log / num-traits / autocfg | as pinned in `Cargo.lock` | MIT OR Apache-2.0 |
| dyn-clone / pastey / ref-cast / ref-cast-impl / pin-project-lite | as pinned in `Cargo.lock` | MIT OR Apache-2.0 |
| slab / bytes | 0.4.12 / 1.12.1 | MIT |

Notes:

- `schemars` is not a direct dependency; it arrives with rmcp's `server`
  feature and is compiled into the binary, so it is inventoried here.
- The dependency audit test (`crates/cli/tests/semantic.rs`) bans protoc
  bridges, non-tokio async runtimes, dynamic loading, and network stacks
  (`reqwest`, `hyper`, `ureq`); tokio is allowed only as the M8 MCP stdio
  runtime.
- Redistributable: permissive only (MIT / Apache-2.0 alternatives), no
  copyleft, no noncommercial code.

## M8 CP3 addition: rmcp macro-based tool registration (pinned versions, verified 2026-10-18)

Enabling the `macros` feature of the already-audited `rmcp 3.5.1` dependency
(design gate §2: `#[tool_router]`/`#[tool]`/`#[tool_handler]` with
schemars-derived input schemas; no new transports, no base64/auth features).
`Cargo.lock` diff: 6 new entries, none removed; `serde`'s `derive` feature is
now declared explicitly (serde_derive was already compiled since CP1; no new
package).

| Package | Version | License | Origin |
|---|---|---|---|
| rmcp-macros | 3.5.1 | Apache-2.0 (declared in package metadata) | https://github.com/modelcontextprotocol/rust-sdk |
| darling / darling_core / darling_macro | 0.24.1 | MIT (declared) | https://github.com/TedDriggs/darling |
| ident_case | 1.0.1 | MIT/Apache-2.0 (declared) | https://github.com/TedDriggs/ident_case |
| strsim | 0.11.1 | MIT (declared) | https://github.com/rapidfuzz/strsim-rs |

Build-time transitive dependencies now compiled (previously resolve-graph
only, as pinned in `Cargo.lock`): `proc-macro2` 1.0.107, `quote` 1.0.47,
`syn` 2.0.119 + 3.0.9, `unicode-ident` 1.0.27 — all MIT OR Apache-2.0
(unicode-ident adds Unicode-3.0).

Notes:

- `rmcp-macros` is the proc-macro half of the same `rmcp` package family
  (same repository and license); it is compiled into the build, not shipped
  as a runtime library.
- Redistributable: permissive only; no copyleft, no noncommercial code,
  nothing vendored.

## Security tooling addition: gitleaks (privacy remediation, 2026-10-19)

| Package | Version | License | Origin | Redistributable? |
|---|---|---|---|---|
| gitleaks | 8.30.1 | MIT | https://github.com/gitleaks/gitleaks | tool only - pinned release binary, checksum-verified at fetch time; never compiled into or shipped with `riltra` |

Used by the CI `security` job and the optional local hooks (`security/scan.sh`,
`.githooks/`); it is not a build dependency and no code is vendored.
