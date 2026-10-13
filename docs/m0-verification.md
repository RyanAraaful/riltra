# M0 Verification Report (auditable)

Scope: blueprint `OPEN_SOURCE_CODE_INTELLIGENCE_KNOWLEDGE_BLUEPRINT.md` §15 (M0) + §18 (M0 contract).
Method: every claim below is backed by a command output or a GitHub API response observed on 2026-10-14 (UTC 2026-10-13/14).
No head-to-head or performance claims are made.

## Commits

| Commit | Message | Author |
|---|---|---|
| `d1af757` | chore: M0 open-source bootstrap and legal baseline | rnafulh-wq <terapetralabs@gmail.com> |
| `0a30142` | ci: run cargo check --locked to cover blueprint M0 gate | rnafulh-wq <terapetralabs@gmail.com> |
| `61608d4` | docs: record auditable M0 verification report (this file, v1) | rnafulh-wq <terapetralabs@gmail.com> |

Verified code state: `0a30142`. Commits after it (`61608d4`, this clarification)
are documentation-only and do not change code, workflow, or fixtures, so gates
stay valid. Working tree at verification time: clean (`git status --short` empty).
`git ls-remote origin`: HEAD and `refs/heads/master` both `0a30142` — local == remote.

## Gate G1 — Rust workspace compiles, fmt/check/test green (Windows local)

Toolchain (rustup stable): cargo/rustc 1.99.0, rustfmt 1.10.0, clippy 0.1.99.

| Check | Command | Result |
|---|---|---|
| Format | `cargo fmt --all -- --check` | exit 0 |
| Check | `cargo check --locked` | `Finished dev profile`, no errors |
| Lint | `cargo clippy --all-targets -- -D warnings` | `Finished`, 0 warnings |
| Test | `cargo test --locked` | 4 passed, 0 failed |
| Smoke | `cargo run -q -p riltra -- status` | `{"name":"riltra","version":"0.1.0","scope":"m0-bootstrap","networking":"disabled"}` |
| Smoke | `cargo run -q -p riltra -- bogus-cmd` | exit 2 (`unknown arguments`) |

Status: **PASS**.

## Gate G2 — CI green on Windows + Linux + macOS (actual results)

Workflow: `.github/workflows/ci.yml` (fmt, check, clippy, test, 3 CLI smokes), matrix
`[windows-latest, ubuntu-latest, macos-latest]`, trigger `push` + `pull_request`.

| Run | SHA | windows | ubuntu | macos |
|---|---|---|---|---|
| run1 (`38459223579`) | `2d886ea` | success | success | success |
| run2 (`38459224942`) | `2d886ea` | success | success | success |
| run3 (`38460006086`) | `d1af757` | success | success | success |
| run4 (HEAD at writing) | `0a30142` | success | success | success |
| run5 | `61608d4` | success | success | success |

Source: `GET /repos/RyanAraaful/riltra/actions/runs/{id}/jobs` — every job
`status=completed`, `conclusion=success` on all three runs covering both SHAs and HEAD.
Run4 executes the workflow **including** the `cargo check --locked` step.

Note (fact, no speculation): the macOS jobs of runs 1–3 spent time in
`queued`/`in_progress` (`runner_id: null`, then started 20:09 UTC) before completing
with `success`. No workflow change was made to address queueing.

Status: **PASS**.

## Gate G3 — Legal baseline present and consistent

Files: `LICENSE` (Apache-2.0 full text, canonical apache.org wording),
`NOTICE` (M0: no bundled third-party source), `CONTRIBUTING.md` (DCO sign-off required),
`SECURITY.md`, `README.md` (M0 scope, non-goals, no `ocik` brand adoption),
`docs/licensing.md` (M0 inventory: zero third-party deps; pre-cleared refs incl.
GitNexus PolyForm Noncommercial DO-NOT-COPY recorded, re-verify at pin time).

Dependency audit: `Cargo.lock` contains exactly one package (`riltra 0.1.0`);
workspace has zero `[dependencies]`. `git ls-files` = 18 files, no `target/`.

Status: **PASS**.

## Gate G4 — Independence (STOP conditions clear)

- Single history line from root commit; all files authored in-session for M0.
- `gitnexus|terapetra` matches exist **only** as prohibition/boundary statements in
  the user-supplied blueprint, `CONTRIBUTING.md`, `README.md`, `docs/licensing.md`.
  No GitNexus/TK source, schema, fixture, or test imported.
- No restricted/unclear-licensed source found → STOP condition not triggered.

Status: **PASS** (proceed to M1 only on explicit approval; M1 not started).

## Final status: **PASS** (4/4 gates), verified code `0a30142` (this doc: see Commits table)
