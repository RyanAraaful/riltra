# M9 consolidation: gate status after the G-SYN and G-GRF corrections (2026-10-26)

Dated revision. This consolidates the M9 gate state on `m9-multirepo-release-candidate` after the G-SYN
span fix (`b53d412`), the G-GRF correction (`b311bb6`) and the G-TOK attempt. The gate records in
[m9-design-gate.md](m9-design-gate.md) and the CP4 table in [m9-cp4-evidence.md](m9-cp4-evidence.md) are
unchanged. No gate is marked PASS here without evidence.

## Branch state

- `origin/m9-multirepo-release-candidate` is at `8861246`, which records the CI and release-candidate workflow
  results. CI ran green on three runners at `f45ddb8` (CP4 §4.1).
- Local commits ahead of origin are not pushed. Through `b311bb6` there are 20. Commits after it are this consolidation record and its corrections. `git log origin/m9-multirepo-release-candidate..HEAD` lists them all.
  No push, merge, tag or release was made.
- `Cargo.toml` and `Cargo.lock` are unchanged since `f45ddb8`, so the SBOM and licence record still apply.
- Local checks at `b311bb6`:
  - `cargo test --locked`: 783 passed, 0 failed.
  - `cargo fmt --all -- --check` and `cargo clippy --all-targets -- -D warnings`: exit 0.
  - `node --test`: `grade.test.cjs` 56 of 56, `fact-digest.test.cjs` 16 of 16.
  - `security/scan.sh range 8861246..b311bb6` (20 commits) and `security/scan.sh tree`: no leaks.
- CI has not run on the local commits.

## Gate status

| Gate | Status | Basis | Change since CP4 |
|---|---|---|---|
| G-LIC | PASS (candidate, this host) | SBOM, notices, licence audit (CP4 §5) | none |
| G-NAT | PASS (acceptance test; macOS x64 only) | native archives and clean-install smoke on three CI runners at `f45ddb8` (CP4 §4.1) | not rerun on the local commits; macOS ARM64 not covered |
| G-CLI | PASS (candidate) | suite green; pilot commands repeat byte for byte (CP4 §5) | 783 tests; 88 M6 golden files changed by the `cfg=` token only (`b53d412`) |
| G-MCP | NOT YET VERIFIED, blocked | real OpenCode run impossible: the API key is expired | preflight 2026-10-26, see [G-TOK](#g-tok-this-step) |
| G-SYN | NOT YET VERIFIED (formal) | checks pass after `b53d412`: 17 import `end_line` values corrected, 0 import position mismatches, no other fact change ([m9-gsyn-import-span-amendment.md](m9-gsyn-import-span-amendment.md)); `.inl` closed for the syntax layer ([m9-inl-amendment.md](m9-inl-amendment.md)) | open: archived release-candidate scan report (CP3 §11) not produced |
| G-SEM | PASS | two families, rust-analyzer and scip-typescript ([m9-gsem-amendment.md](m9-gsem-amendment.md), 2026-10-25) | C++ on MMSP still has no semantic provider |
| G-GRF | NOT YET VERIFIED | correctness and safety PASS (Task 6: 0 CALLS, 0 Exact, 71 of 71 citations, 13 of 13 anchors); capability gates D1-C6, D2-C6, D3-C5, D4-C5 UNMET; honesty 17 PASS | `b311bb6`: CUDA refusal named; D3-H3 predicate corrected; indirect dispatch still absent and not claimed; the path still does not name the CUDA boundary ([m9-grf-transparency-amendment.md](m9-grf-transparency-amendment.md)) |
| G-FSH | PASS (pilot) | F1 to F5 match their pre-registered verdicts (CP4 §2) | none |
| G-INC | PASS (pilot) | parity and single-file reparse (CP4 §3) | not measured again after the `PARSER_VERSION` 7 change |
| G-KNW | PASS for the isolated-home acceptance test on this host ([m9-gknw-amendment.md](m9-gknw-amendment.md), at `f9eed48`) | 52 knowledge tests green | release round trip on a separate isolated home not run |
| G-TOK | NOT YET VERIFIED, blocked | the new measurement needs a working OpenCode key; preflight failed | see [G-TOK](#g-tok-this-step) |
| G-SEC | PASS (candidate) | isolation and non-mutation (CP4 §3, §5); range and tree scans clean on 2026-10-26 | none |

G-PAR is an M10 target, not an M9 gate.

## Formal state

M9 does not yet pass every gate. Open formal items: G-MCP and G-TOK (both blocked by the expired key), G-SYN
(archived scan report), G-GRF (four capability gates UNMET, plus the transparency decisions below) and the G-KNW
release round trip. The v0.1.0 merge, tag and publish stay a separate owner GO.

<a id="g-tok-this-step"></a>
## G-TOK (this step)

- Requested: a new measurement under the CP7-A frozen rules, labelled separately from CP7-A (CP4 §8, design gate
  G-TOK).
- Preflight on 2026-10-26: one `READY` call with the frozen model `opencode/deepseek-v4.1-flash-free` returned
  `This API key has expired`. Raw output: [raw/g-tok-preflight.txt](pilots/m9-consolidation/raw/g-tok-preflight.txt).
  Result: zero measured sessions, zero tokens counted, and no change to the CP7-A archive.
- Other prerequisites before a run:
  - The installed OpenCode CLI is 1.18.35. CP7-A used v2.0.32 ([cp7a summary](pilots/cp7a/summary.md)). A new run
    pins the CLI version or discloses the difference.
  - The CP7-A harness swaps the global OpenCode configuration and restores a backup
    (`pilots/cp7a/method/global-opencode.original-backup.jsonc`). The run plan states this before it starts.
- Not substituted: no other model, no other provider key, and no static size proxy is reported as G-TOK. The CP7-A
  token rule and the design-gate STOP rules forbid it.
- Next step, owner: replace the expired key in the OpenCode console and the local secret store. Do not paste it in
  chat. A labelled measurement can then run.

## Owner decisions

1. Replace the expired OpenCode API key. This unblocks G-TOK and the G-MCP agent side. Owner action.
2. Push the local commits to `origin/m9-multirepo-release-candidate`. Needs an explicit yes.
3. G-GRF transparency (b): name the CUDA boundary in the path. This needs a graph or output change with its own RED
   tests. Owner decides whether it blocks G-GRF.
4. G-GRF transparency (a): indirect dispatch. Either accept "absent and not claimed", or require a labelled
   candidate for `ThreadPool::AddTask`, which is a graph change.
5. Unsupported extensions in general, such as `.md`, also answer `source-changed` for an unindexed target. Found in
   `b311bb6`'s check and left out of scope. Owner decides whether to extend the classification.
6. G-SYN: produce the archived release-candidate scan report (CP3 §11), or amend the gate text.
7. C++ capability gates D1-C6, D2-C6, D3-C5 and D4-C5: not chased further in M9, as the owner directed. Type identity
   needs a semantic C++ provider (clangd, which needs an MMSP compile database and owner approval) or graph work. This
   is a later milestone decision. Root causes: [m9-task5-capability-gap-evaluation.md](m9-task5-capability-gap-evaluation.md).
8. v0.1.0 merge, tag and publish: separate owner GO. macOS ARM64 stays uncovered.

## Known items not investigated

- Index database size: Task 6 and the G-SYN run both give 60,370,944 B, which is 24,576 B above the figure in
  [m9-inl-amendment.md](m9-inl-amendment.md). Facts are identical. The Task 6 record notes it as not investigated
  ([m9-task6-final-verification.md](m9-task6-final-verification.md), raw evidence notes).

## Evidence index

- `m9-gsyn-import-span-amendment.md` (commit `b53d412`): G-SYN span fix and its checks.
- `m9-grf-transparency-amendment.md` (commit `b311bb6`): CUDA refusal, D3-H3 correction, regrades.
- `m9-task6-final-verification.md` (commit `df35536`): G-SYN and G-GRF verification record, unchanged.
- `m9-cp4-evidence.md`: release-candidate pack, release gates, CI results.
- `m9-gsem-amendment.md`, `m9-inl-amendment.md`, `m9-gknw-amendment.md`: later gate amendments.
- `m9-task5-capability-gap-evaluation.md`, `m9-grader-correction-amendment-2026-10-25.md`,
  `m9-cpp-semantic-index-*-2026-10-25.md`: C++ capability and index decisions.
- `pilots/m9-grf-transparency/`, `pilots/m9-gsyn-fix/`, `pilots/m9-task6/`: redacted raw evidence.
