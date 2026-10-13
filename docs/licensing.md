# Dependency & License Inventory

Re-check sources at pin time before any release.
Policy: every bundled grammar, indexer binary, or vendored source needs an
entry here (package + exact version + license + origin URL + redistributability
note) in the same PR that introduces it. Never introduce code under
noncommercial/use-restricted licenses or from sources without clear rights.

## Current: zero third-party dependencies

| Component | Version | License | Origin | Redistributable? |
|---|---|---|---|---|
| Rust std only (no crates.io deps) | toolchain stable | MIT OR Apache-2.0 (std) | rust-lang/rust | yes — linked std, no vendoring |
| Build/test tooling (rustc, cargo, rustfmt, clippy) | stable (see CI log) | MIT OR Apache-2.0 | rust-lang | build-time only, not shipped |

The CLI is intentionally std-only so the legal baseline stays trivially
auditable. Future capabilities (richer CLI, parsers, storage) will add
SBOM entries here as they land.

## Reference notes for future introductions (NOT bundled — audit per package)

- tree-sitter core: MIT — each grammar licensed separately, audit per grammar.
- SCIP spec/tooling: Apache-2.0 — record proto origin + version.
- Rust MCP SDK (`rmcp`): license changed over time; keep the exact LICENSE of
  the pinned version.
- `rusqlite`: MIT; bundled SQLite: public domain — audit features + bundled libs.
- `gix`/gitoxide: MIT OR Apache-2.0.
- clangd/LLVM: Apache-2.0 WITH LLVM-exception — keep notices.
- rust-analyzer: MIT OR Apache-2.0. petgraph: MIT OR Apache-2.0 (optional algo only).

## Audit result

- `cargo tree`: no dependencies (workspace has zero `[dependencies]`).
- License risk: none beyond Rust std build-time linkage.
