# M9 consolidation: gate status at the shipped commit (2026-10-26, revised)

Dated revision. Consolidates M9 on `m9-multirepo-release-candidate` after the G-SYN span fix (`b53d412`), the G-GRF
correction (`b311bb6`) and the pre-release acceptance alignment. The detailed decision, legend and evidence are in
[m9-preprelease-acceptance-amendment.md](m9-preprelease-acceptance-amendment.md). The frozen gate text in
[m9-design-gate.md](m9-design-gate.md) and the CP4 record in [m9-cp4-evidence.md](m9-cp4-evidence.md) are unchanged.
No gate is marked PASS without evidence.

## Branch state

- `origin/m9-multirepo-release-candidate` is at `8861246` (CI and release-candidate workflow results). CI ran green
  on three runners at `f45ddb8` (CP4 §4.1).
- Pushed to `origin/m9-multirepo-release-candidate` at `13f083a` on 2026-10-26, with the owner's yes. CI run
  `40257287230` (`ci`, push event) was queued at push time. A branch push does not trigger the release-candidate
  workflow. No merge, tag or release.
- `Cargo.toml` and `Cargo.lock` are unchanged since `f45ddb8`.
- Local checks at `b311bb6`: `cargo test --locked` 783 passed; `cargo fmt --check` and
  `cargo clippy --all-targets -- -D warnings` exit 0; `grade.test.cjs` 56 of 56; `fact-digest.test.cjs` 16 of 16;
  `security/scan.sh range 8861246..b311bb6` and `tree`: no leaks.
- CI has not run on the local commits.

## Core acceptance (repository validation; no agent, no API key)

| Gate | Status | Still needed for pre-release |
|---|---|---|
| G-LIC | PASS (candidate, this host) | SBOM drift check on the final commit |
| G-NAT | PASS at `f45ddb8`; UNAVAILABLE for the final commit | CI and release workflow on the final commit; macOS ARM64 not covered (owner decision) |
| G-CLI | PASS (candidate) | CI on the final commit |
| G-MCP, core | PASS at `b311bb6` (transport, contract, classified error, stdout purity, read-only check) | repeat in CI on the final commit |
| G-SYN | PASS (candidate, `b311bb6`): source-grounded checks, deterministic repeats, exact unsupported counts, first-party errors recorded. Scan report in [amendment §8.2](m9-preprelease-acceptance-amendment.md). | none for the gate's acceptance checks |
| G-SEM | PASS (2026-10-25 amendment) | none |
| G-GRF, correctness and safety | PASS (Task 6; grader honesty PASS) | none |
| G-GRF, capability | UNMET (D1-C6, D2-C6, D3-C5, D4-C5), kept | owner decision: block or record as known limitation |
| G-FSH | PASS (pilot, CP4 §2) | not repeated on the final commit |
| G-INC | PASS (pilot, CP4 §3) | owner decides whether to repeat after the `PARSER_VERSION` 7 change |
| G-KNW | PASS, release-level isolated round trip at `b311bb6` (73 checks, 0 failed) | none in core |
| G-SEC | PASS (candidate) | CI security job on the final commit |

G-PAR is an M10 target, not an M9 gate.

## Agent integration (per agent, not core)

| Agent | MCP integration (M9) | Token efficiency (G-TOK) |
|---|---|---|
| OpenCode | UNAVAILABLE: API key expired, no session ran. Historical PASS on the M8 surface (CP7-A revised G6). | UNAVAILABLE: same cause. Historical CP7-A G3 on the M8 surface, kept. |
| Claude Code | smoke PASS (single session, five MCP calls) | DEFERRED until owner GO |
| Codex | NOT TESTED | NOT TESTED |

Evidence and limits: [amendment §4–§5](m9-preprelease-acceptance-amendment.md). A failed authentication is not a
Riltra correctness failure.

## Owner decisions

1. Replace the expired OpenCode API key, if OpenCode integration is wanted for the release. Owner action; do not
   paste the key in chat.
2. Done 2026-10-26: the branch is pushed at `13f083a`. Any further push needs an explicit yes.
3. G-GRF capability criteria (four C++ UNMET): block the pre-release, or record as known limitations.
4. Unsupported extensions answer `source-changed` (for example `.md`). Proven. Fix needs an owner GO for a production
   change ([amendment §9](m9-preprelease-acceptance-amendment.md)).
5. G-GRF transparency (a) and (b): indirect dispatch absent and not claimed; the path does not name the CUDA boundary.
   Each needs a graph or output change if the owner wants it.
6. Claude Code token measurement and any further agent integration: owner GO.
7. macOS ARM64 coverage, history rewrite of local-path commits, v0.1.0 merge, tag and publish: owner decisions.

## Known items not investigated

- Index database size: Task 6 and the G-SYN run both give 60,370,944 B, which is 24,576 B above the figure in
  [m9-inl-amendment.md](m9-inl-amendment.md). Facts were identical. The Task 6 record notes it as not investigated.

## Evidence index

- [m9-preprelease-acceptance-amendment.md](m9-preprelease-acceptance-amendment.md): this decision, legend, per-agent
  and core status, deferrals, recommendation, blockers.
- [m9-grf-transparency-amendment.md](m9-grf-transparency-amendment.md) (`b311bb6`), [m9-gsyn-import-span-amendment.md](m9-gsyn-import-span-amendment.md) (`b53d412`),
  [m9-task6-final-verification.md](m9-task6-final-verification.md) (`df35536`, unchanged).
- [m9-cp4-evidence.md](m9-cp4-evidence.md), [m9-gsem-amendment.md](m9-gsem-amendment.md), [m9-inl-amendment.md](m9-inl-amendment.md), [m9-gknw-amendment.md](m9-gknw-amendment.md).
- `pilots/m9-preprelease/`, `pilots/m9-grf-transparency/`, `pilots/m9-gsyn-fix/`, `pilots/m9-task6/`: redacted raw evidence.
