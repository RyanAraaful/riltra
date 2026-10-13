# Dependency & License Inventory (M0 baseline)

Date: 2026-10-14. Sources re-checked at pin time before any release.
Policy: every bundled grammar, indexer binary, or vendored source needs an
entry here (package + exact version + license + origin URL + redistributability
note) in the same PR that introduces it.

## M0: zero third-party dependencies

| Component | Version | License | Origin | Redistributable? |
|---|---|---|---|---|
| Rust std only (no crates.io deps) | toolchain stable | MIT OR Apache-2.0 (std) | rust-lang/rust | yes — linked std, no vendoring |
| Build/test tooling (rustc, cargo, rustfmt, clippy) | stable (see CI log) | MIT OR Apache-2.0 | rust-lang | build-time only, not shipped |

There is intentionally **no** `clap`/`serde`/`rusqlite`/`tree-sitter` in M0:
the CLI placeholder is std-only so the legal baseline stays trivially
auditable. Those arrive with their milestones (CLI contract M1, parser M2,
storage M3) plus SBOM entries.

## Pre-cleared references (NOT yet bundled — audit per package at introduction)

From blueprint §12/§19 (14 Oct 2026, re-verify at pin time):

- tree-sitter core: MIT — each grammar licensed separately, audit per grammar.
- SCIP spec/tooling: Apache-2.0 — record proto origin + version.
- `rmcp` Rust MCP SDK: MIT→Apache-2.0 transition in progress; keep the exact
  LICENSE of the pinned version.
- `rusqlite`: MIT; bundled SQLite: public domain — audit features + bundled libs.
- `gix`/gitoxide: MIT OR Apache-2.0.
- clangd/LLVM: Apache-2.0 WITH LLVM-exception — keep notices.
- rust-analyzer: MIT OR Apache-2.0. petgraph: MIT OR Apache-2.0 (optional algo only).
- **GitNexus: PolyForm Noncommercial 1.0.0 — DO NOT copy/reuse source.**
- TK internal: not a source — no read/access/copy for this project.

## M0 audit result

- `cargo tree` equivalent: no dependencies (workspace has zero `[dependencies]`).
- License risk: none beyond Rust std build-time linkage.
- Brand: `ocik` working name NOT adopted; availability unclaimed.
