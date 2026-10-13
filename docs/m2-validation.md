# M2 Validation Summary

Scope: syntax baseline — per-file definitions, imports, and unresolved call
candidates with line/column spans and byte offsets for Python, JavaScript,
TypeScript (`.ts`/`.tsx`), Rust, C, and C++ (headers parse as C++; `.c` is
strict C). Calls are name candidates as written, never resolved targets.
CUDA (`.cu`) is explicitly experimental/unsupported; unknown extensions are
reported, never parsed.

## Tier A — controlled fixtures (CI)

Self-authored fixtures under `crates/cli/tests/fixtures/m2/`, one golden file
per source file under `crates/cli/tests/golden/`. Every golden was generated
once and verified by hand (names, spans, byte offsets) before pinning.

Covered: definitions (functions/methods/classes/structs/interfaces, including
qualified C++ definitions such as `PatchMatch::Run` recorded verbatim as
unresolved candidates), imports and includes (single-line span of the
directive), call candidates, malformed input recovery (`errors` counted,
neighboring facts still extracted), unsupported extensions, `.cu` refusal,
byte-identical output on repeat runs, and `--symbol` filtering.

## Tier B — real repository (MMSP, read-only, local only)

Target: MMSP clone, branch `main`, clean worktree. The repository was only
read (source + `git` metadata); nothing was pulled, checked out, built, or
modified, and no MMSP source was copied into this repository. Full outputs
stay outside the MMSP tree.

- **Subset:** five representative files (C++/Python/TypeScript) scanned with
  full facts; sample definitions, includes/imports, call candidates, and spans
  checked by hand against the sources. Declarations without bodies are
  correctly absent; all five files report zero errors.
- **Broad smoke:** full-tree scan with explicit policy (sorted walk, no
  symlink following, `.git` never entered, ignore rules honored, 1 MiB
  per-file cap checked before reading, binary/non-UTF8 refused). Two runs on
  identical input produce byte-identical output. Third-party/generated
  directories dominate the observed syntax-error counts; first-party sources
  parse almost cleanly (see metrics below). No crash, no hang, no unbounded
  memory use.
- **Metrics (final build, MMSP `main/d62f06a`, clean):** 15,176 files
  discovered; 11,765 processed (170,586 definitions, 64,954 imports, 562,384
  calls — calls are unresolved candidates); 152,543 errors counted across
  5,481 files, of which only 55 are first-party (rest: `colmap/build*`,
  vendored third-party); 291 skipped-large, 492 skipped-binary, 2,628
  unsupported (all 562 `.cu` as `cuda-experimental`); languages
  cpp=11,394, python=299, c=52, typescript=18, javascript=2.
  Full-tree runs: ~179–366 s (debug build), peak ~222–325 MB RSS, exit 0;
  byte-identical SHA256 across runs on identical input.

## Limitations observed (honest, not fixed in M2)

- Macro interiors (e.g. Rust `println!(w.process(..))`) are opaque token
  trees to the grammar: calls nested inside macros are not extracted.
- Bare declarations without bodies (C prototypes) are not recorded; named
  `struct`/`class` specifiers (including forward declarations) are.
- Heavy macro/template metaprogramming in third-party headers yields high
  `errors` counts there; facts are still extracted by recovery and every
  error is counted, never hidden.
- Column numbers are byte-based (identical to character columns for ASCII).

## Gates

- `cargo fmt --check`, `cargo check --locked`, `cargo clippy --all-targets --
  -D warnings`, `cargo test --locked`: green locally and on Windows, Linux,
  and macOS CI using self-contained fixtures only.
- No MMSP path exists anywhere in CI configuration or committed tests; the
  MMSP hook runs only with an explicit local environment variable.
- Dependency and grammar license audit: `docs/licensing.md`.
