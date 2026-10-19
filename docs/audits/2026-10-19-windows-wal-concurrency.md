# Windows WAL concurrency audit -- `r2_knowledge_show_snapshot_on_v1_and_under_concurrent_writes`

<!-- SPDX-License-Identifier: Apache-2.0 -->

- Date: 2026-10-19
- Repository: `RyanAraaful/riltra`, branch `m8-unified-cli-mcp`
- Baseline commit (unchanged by this audit): `43b2ea2af5056c1786499bd85d89e6590975ebf3`
- CI under investigation: [run 39201873049](https://github.com/RyanAraaful/riltra/actions/runs/39201873049)
- Authorization: diagnostic audit only; no production fix. M8 remains OWNER ACCEPTED -- CLOSED.
- Change footprint: this one document. No code, test, dependency, schema, workflow, or
  acceptance-record changes; no benchmark rerun; no merge, tag, or release.

Conventions: **Observed** = captured from CI logs or local command output; **Inferred** =
engineering reasoning over observed facts; **Unverified** = hypothesis without direct evidence.

## 1. Investigation scope and baseline

Examined the Windows-only failure of `r2_knowledge_show_snapshot_on_v1_and_under_concurrent_writes`
(`crates/cli/tests/mcp.rs`) reported as `store_error: cannot inspect ... krepo.db-wal: Access is
denied (os error 5)`, using: original CI logs for both attempts, the full call path from the test
to the SQLite filesystem validation, bounded local reproduction, and throwaway Windows filesystem
mechanism probes (outside the repository).

Preflight (Observed): branch `m8-unified-cli-mcp`; local `HEAD` == `origin/m8-unified-cli-mcp` ==
`43b2ea2af5056c1786499bd85d89e6590975ebf3`; `origin/master` untouched at
`cb40aea1581683857dd55eff7ea304c311313f31`; worktree clean except the pre-existing untracked
`AGENTS.md` (never staged, not modified).

Local reproduction environment (Observed): Windows 11 build 10.0.26100.0; rustc/cargo 1.99.1
(2026-10-13); Intel i5-8250U, 8 logical processors, 19.9 GB RAM; system temp on `C:`; Windows
Defender real-time protection and antivirus enabled. CI target (Observed from job logs):
GitHub-hosted `windows-latest`, stable Rust via `dtolnay/rust-toolchain@stable`.

Explicitly out of scope (per authorization): any fix, retries, timeouts, workflow changes,
dependency changes, test changes, M8 record changes, CP7-A reruns, M9 work.

## 2. Original CI failure evidence (attempt 1)

Run `39201873049` triggered by push of `43b2ea2af5056c1786499bd85d89e6590975ebf3`
(`headSha` confirmed via API). Attempt 1 started 2026-10-19 04:55:06Z. Jobs (Observed via
`gh api .../attempts/1/jobs`):

| job | id | result | window (UTC) |
|---|---|---|---|
| check (ubuntu-latest) | 117896351953 | success | 04:55:08 - 04:56:49 |
| check (windows-latest) | 117896352103 | **failure** | 04:55:08 - 04:59:19 |
| check (macos-latest) | 117896352121 | success | 04:55:13 - 04:57:07 |

Windows job details (Observed, full log retrieved with `gh run view 39201873049 --attempt 1 --log`,
saved to a temp file, 562,581 bytes): all other test binaries passed (337, 16, 137, 21, 52, 15, 12
tests); the `mcp` binary started at 04:59:06.3241023Z (`running 44 tests`); only
`r2_knowledge_show_snapshot_on_v1_and_under_concurrent_writes` failed, marker at 04:59:09.4090677Z.
Exact failure text (Observed):

```
thread 'r2_knowledge_show_snapshot_on_v1_and_under_concurrent_writes' (9224) panicked at crates\cli\tests\mcp.rs:3563:9:
assertion `left == right` failed: {"kind":"store_error","message":"cannot inspect knowledge store path '\\\\?\\C:\\Users\\runneradmin\\AppData\\Local\\Temp\\riltra-mcp-cp5-r2-live-3184-32\\knowledge\\krepo.db-wal': Access is denied. (os error 5)"}
  left: Null
 right: String("d1")
```

Followed by `failures: r2_knowledge_show_snapshot_on_v1_and_under_concurrent_writes`,
`test result: FAILED. 43 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.64s`,
`error: test failed, to rerun pass --test mcp`, job exit code 1.

Observed interpretation of the excerpt:

- Only this test failed; the failure is a classified JSON-RPC error response
  (`{"kind":"store_error", ...}`), so the MCP server answered the request: the process stayed
  alive; no crash, no hang, no dropped connection; the harness 15 s timeout was not involved.
- The test path suffix `riltra-mcp-cp5-r2-live-3184-32` is the `{pid}-{counter}` home from
  `home()` (`crates/cli/tests/mcp.rs:21-28`).
- The error text is produced by the store-path inspection (see section 4): the inspected path is
  the `-wal` companion of `krepo.db`, the failure is name-based metadata access, and the OS error
  is `ERROR_ACCESS_DENIED` (5). The `\\?\` prefix is the canonicalized home used by the read path.
- The writer thread's own assertions (`kupdate`, `crates/cli/tests/mcp.rs:2946`) produced no
  failure lines, so the concurrent CLI updates succeeded (Observed); the refusing operation was
  the reader/MCP side.
- The panic aborted the test at the first bad round, so `writer.join()` and the later rounds were
  skipped: the store state after the refusal was not directly observed (Unverified; no mutation is
  expected from a read path that refuses at inspection).

## 3. Attempt 1 versus attempt 2

Attempt 2 (rerun of failed jobs only) started 05:01:16Z on the same commit:
windows job 117897894583 success 05:01:16 - 05:05:55; ubuntu/macOS results were carried over
green from attempt 1 (their timestamps are unchanged). Attempt-2 windows log (Observed): `mcp`
binary `running 44 tests` ~05:05:35.58; line
`test r2_knowledge_show_snapshot_on_v1_and_under_concurrent_writes ... ok` at 05:05:38.766;
`test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.73s`.

Comparison: same commit (`headSha` identical), same test selection, same binary duration
(~9.64 s failed vs ~9.73 s green), Windows-only, no code changed between attempts.
**Attempt 1 remains FAILED in the record.** The green rerun demonstrates non-determinism
(a transient/environmental trigger), not the absence of the condition; and it is not treated
as a fix. No per-attempt environment diff is available from the logs beyond the runner image
being equivalent `windows-latest` (Unverified: any runner-image or AV-definition difference
between the two attempts).

## 4. Exact code path examined

Reader path (Observed in source):

- Test `crates/cli/tests/mcp.rs:3503-3573`. Concurrent section: setup 3535-3539; writer thread
  3540-3555 runs eight sequential CLI `knowledge update` processes (`kupdate`, 2929-2947, each
  asserting exit code 0); reader loop 3556-3569 issues eight `riltra_knowledge_show` calls over
  the same MCP server session (`kjson`, 2978) and asserts at 3563-3568 that the record id and
  revision==generation pairing come from one snapshot.
- MCP tool: registration/handler `crates/cli/src/mcp.rs:322-335`; `knowledge_show_doc` at
  1135-1150 calls `knowledge_store::open_knowledge(&entry, false)` (1141) then
  `knowledge_store::show_snapshot` (1146); every store error is routed through
  `knowledge_query_error` (1010-1032), which maps "cannot inspect knowledge store path ..." to
  **`store_error`** (the fail-closed fallback branch).
- Store open: `crates/cli/src/knowledge_store.rs:53-64` (`open_knowledge` /
  `open_knowledge_at`) into `open_inner` 76-174:
  - 82-88 name validation, home/root canonicalization, knowledge-dir validation
    (`ensure_knowledge_dir` 220-247);
  - 90-94 **pre-open hostile-link scan** over the database path and its companions;
  - 112-117 open: read-only for `create=false` (`SQLITE_OPEN_READ_ONLY`);
  - 118-129 canonicalized-path escape comparison for the database path;
  - 130-132 **post-open companion re-check** (`-wal`, `-shm`, `-journal`);
  - 133-138 3000 ms busy timeout plus a readability probe;
  - 139-162 schema version/completeness/meta checks (fail-closed);
  - 163-173 `PRAGMA journal_mode=WAL` with the documented note that the scans cover the open
    instant only and writers re-validate companions (see `revalidate_open_paths` 702-716).
  Companion suffix construction: `companion_paths` 176-187 (`-wal`, `-shm`, `-journal`).
- The failing operation is `reject_link` (189-205): `std::fs::symlink_metadata` on one path;
  `NotFound` -> treated as absent; **any other I/O error -> fail-closed refusal** with the exact
  message `"cannot inspect knowledge store path '{}': {e}"` (195-198); a symlink observation ->
  the distinct escape error (199-202). The message is produced only here, at either the pre-open
  scan (93) or the post-open re-check (131) -- the text is identical for both sites
  (Unverified: which site fired in CI).
- Snapshot semantics: `show_snapshot_inner` 1520-1551 (one `BEGIN` ... record + generation ...
  `COMMIT`, optional test-only probe seam) and `show_snapshot` 1553-1561.
- Security tests around this guard (all in `knowledge_store.rs`): 5187 (hostile db symlink),
  5213 (hostile companion symlink), 5244 (dangling companion symlink), 5272 (post-open re-check),
  5300 (Windows junction on the knowledge dir), 5387 (Unix dir symlink), 5663 (write-path
  companion revalidation). Helpers: `plant_link` 5340, `is_junction` 5327.

Observed: with the writer thread updating through separate CLI processes, each update opens the
store (which activates WAL, 172), commits, closes; SQLite's documented behavior on the last clean
connection close is a checkpoint plus **unlink of the `-wal` and `-shm` files** (Context7
`/websites/sqlite_docs`, WAL documentation). So the reader's companion scans run concurrently
with the writer's WAL/SHM create-unlink lifecycle -- that much is by test design.

## 5. Bounded reproduction results

All local runs used the real test binary with isolated temp homes; the filter selected only the
one test except in phase B. No test file or production file was modified.

| phase | method | attempts | failures |
|---|---|---|---|
| sequential | `cargo test --locked -p riltra --test mcp r2_knowledge_show_snapshot_on_v1_and_under_concurrent_writes -- --nocapture`, 20 rounds | 20 | 0 |
| amplification A | same test binary, 4 instances in parallel x 10 rounds | 40 | 0 |
| amplification B | full `mcp` binary suite (44 tests incl. r2), 5 runs | 5 (suite runs; r2 x 5) | 0 |

Result: **NOT REPRODUCED UNDER TEST CONDITIONS** -- 20 focused + 40 parallel + 5 full-suite runs,
0 reproductions (per-attempt pass/fail logs and environment capture kept in the temp audit
directory listed in section 11). This is evidence of low local frequency, not proof of race
freedom; retry counts are not used as such proof.

Mechanism probes (Observed; throwaway scripts and a tiny Rust probe calling exactly
`std::fs::symlink_metadata`, the call inside `reject_link`; all outside the repository):

| state created while the path exists | probe result | `reject_link` outcome |
|---|---|---|
| S1: `DeleteFileW` with a FILE_SHARE_DELETE holder open | `NotFound` (os error 2) immediately; name unlinked | treated absent -> Ok (benign race) |
| S2: open by an exclusive-share handle (`share=0`) | `Ok` (attribute query still succeeds) | Ok |
| S3: legacy delete disposition (`SetFileInformationByHandle` FileDispositionInfo) | `Ok`; name visible until close; `NotFound` after close | Ok |
| S4: memory-mapped section + `DeleteFileW` | `NotFound` at all stages | treated absent -> Ok |

Observed conclusion: none of the locally reproducible delete/unlink/open/mapped states yields
`Access is denied (os error 5)` on this OS build -- so the CI refusal cannot be explained by the
ordinary in-test unlink and concurrency races (those resolve to absent or to a successful query).
External references (Observed): Microsoft `DeleteFileW` documentation ("marks a file for deletion
on close ... deletion does not occur until the last handle ... is closed") and `CreateFileW`
documentation ("If you call CreateFile on a file that is pending deletion as a result of a
previous call to DeleteFile, the function fails"); SQLite `SQLITE_FCNTL_WIN32_AV_RETRY`
documentation ("the windows VFS will retry file read, file write, and file delete operations up
to 10 times, with a delay of 25 milliseconds before the first retry ...") -- i.e. SQLite itself
treats transient Windows antivirus-induced I/O failures as a real condition, and its retry
protection covers file read/write/delete, **not** path metadata inspection.

## 6. Root-cause classification

**Primary classification: PROBABLE CAUSE -- INSUFFICIENT PROOF.**

Probable cause (Inferred): a transient, externally induced access denial on the `-wal` path
during the writer's WAL/SHM churn -- the known Windows class where a filter driver (antivirus /
real-time scanning) or other external actor returns `ERROR_ACCESS_DENIED` for a name lookup
while the file is being created/deleted/examined -- surfaced through the intentionally
fail-closed metadata inspection (`reject_link`). This matches every observed fact: the exact raw
OS error (5), the path in flux (`-wal`), the Windows-only, non-deterministic occurrence, the fast
classified refusal (no timeout), and the immediate green rerun. The precedent citation is
SQLite's own AV-retry facility (section 5). The exact actor at the CI instant was not observed
(no kernel-level tracing on the runner), which is why this remains a probable, not confirmed,
cause.

Category mapping against the task's list:

1. *Legitimate transient Windows filesystem condition* -- **most probable** (the OS-level
   denial class is legitimate; only the specific actor is unobserved).
2. *Race in SQLite WAL sidecar validation* -- **ruled out as a logic race** (Observed): the
   validation is stateless and idempotent; the ordinary unlink race resolves to `NotFound` and
   is accepted by design (S1/S4); 40 parallel + 20 sequential local runs never misfired; attempt
   2 changed nothing and passed.
3. *Overly strict or incorrect test assumption* -- **contributing factor** (Inferred): the test
   reasonably expects every read to be inspectable during continuous writes, but the
   implementation only guarantees a fail-closed refusal when inspection errors. The test is not
   wrong about committed data; it is unconditional about availability. This is a contract /
   availability gap, not a test defect.
4. *Real product reliability defect during concurrent reads/writes* -- **partially confirmed as
   an availability consequence** (Observed by construction): any transient non-`NotFound`
   inspection error on the db or a companion converts to a user-visible `store_error` on the read
   path, with no bounded retry. The frequency is externally driven (not a logic defect); the
   consequence is a rare, transient refusal, not corruption.
5. *Another cause* -- the residual (the unobserved external denial mechanism).

Confidence: high that this is not a deterministic logic bug, not a symlink/junction escape, and
not data corruption; medium that the trigger is the antivirus/external-denial class; low for the
exact kernel/actor state at the CI instant. Explicitly not claimed: that the rerun proves
absence; that every `AccessDenied` is safe to retry; that the condition cannot recur.

## 7. Security and snapshot-consistency assessment

These three properties are assessed separately and are not conflated.

**Safety property -- held (Observed/Inferred).** The incident ended fail-closed: the suspect path
was not silently trusted, the refusal is a classified tool error, the MCP server stayed alive,
and no link was followed through the guard. In the pre-open case nothing was opened; if the
post-open re-check fired (indistinguishable by message text), the connection was dropped by the
error path and the documented TOCTOU narrowing (163-173, 702-706) is unchanged -- this incident
neither widens nor weakens it.

**Reliability property -- gap confirmed by construction (Observed).** Any transient non-`NotFound`
metadata error on the database or a companion becomes a user-visible `store_error` on read; there
is no bounded retry. On Windows, normal concurrent use (writer process closing a WAL while a
reader inspects the store) can therefore surface rare avoidable refusals. Impact: availability
only; the caller can retry; no data mutation, no corruption, SQLite waits remain bounded by the
3000 ms busy timeout (133).

**Snapshot correctness -- unaffected (Observed).** `show_snapshot` reads the record and the
generation inside one read transaction (1520-1561), so a successful response always pairs data
from one committed snapshot (asserted by the test at 3564-3568). The incident produced no
successful response -- the refusal happens at/around the store-open inspection, outside the
snapshot transaction -- so no inconsistent pair was or could be returned.

**Is a legitimate `AccessDenied` during WAL-sidecar inspection expected and unavoidable?**
(Inferred) The denial class is a legitimate transient Windows condition (section 5 references);
it is not *unavoidable*, but it also must not be silently swallowed: the current strategy can
generate avoidable false refusals under normal concurrent use, and the safe correction is a
bounded, kind-scoped retry of the identical pure inspection (section 8, Option A), never an
acceptance of the denial.

## 8. Minimum remediation options (no implementation in this audit)

**Option A -- bounded retry of transient `PermissionDenied` inside `reject_link` (recommended
if a code change is authorized).**

- Files/functions likely affected: `crates/cli/src/knowledge_store.rs`, `reject_link` (189-205).
- Expected behavior change: when `std::fs::symlink_metadata` fails with
  `ErrorKind::PermissionDenied`, re-run the identical check a small bounded number of times with a
  short fixed delay (for example 3 attempts at 25 ms, mirroring SQLite's documented 25 ms Windows
  antivirus-retry convention); if it still fails, return the existing fail-closed error unchanged.
- Security implications: the same check is re-executed; no check is skipped or weakened; the
  final observation is authoritative. A persistent denial (hostile ACL, planted link observed as
  a link) still fails closed; the symlink branch is untouched and never retried. Retrying a pure,
  side-effect-free metadata read is safe; the only cost is bounded latency in the denial path.
- Regression tests required: (i) all existing link/junction refusal tests stay green (5187, 5213,
  5244, 5272, 5300, 5387, 5663); (ii) a new Windows-only test that forces a transient
  `PermissionDenied` clearing within the budget, asserting success after it clears and refusal
  when it persists (mechanism to be designed: ACL-based denial or a small test-only seam);
  (iii) `r2_knowledge_show_snapshot_on_v1_and_under_concurrent_writes` stays the end-to-end
  acceptance test.
- Masking risk: cannot mask a persistent ACL misconfiguration (still refused after the budget);
  it does by design absorb the transient availability symptom.
- Cross-OS: raw OS error 5 occurs on Windows as observed; Unix `EACCES` from `lstat` is normally
  persistent, so an unconditional small retry adds negligible delay before the same refusal.
  Either unconditional (single code path) or `#[cfg(windows)]`-gated (zero Unix behavior change)
  is defensible; prefer the unconditional form unless maintainers want zero Unix change.

**Option B -- test-only tolerance of the classified transient (fallback).**

- Files: `crates/cli/tests/mcp.rs` reader loop (3556-3569): on a `store_error` matching
  `cannot inspect knowledge store path`, retry the show call a bounded number of times before
  failing.
- Behavior: CI stops failing on this class; production behavior unchanged (still fail-closed, no
  retry). Security implications: none (test-only). Regression tests: none. Masking risk: masks
  the product availability symptom; a systemic (persistent) refusal would still fail the test if
  retries are bounded. Cross-OS: n/a.

**Option C -- accept and document (no change).** Treat as a rare Windows transient; callers may
retry on `store_error`. Cheapest, keeps fail-closed purity, leaves the CI-flake potential.

**Option D -- rejected: `PERSIST_WAL`/`SQLITE_FCNTL_PERSIST_WAL` to avoid unlink churn.** Broader
on-disk lifecycle change; no evidence it removes the trigger (denials can hit create/rename paths
too); increases surface. Not minimal.

**Option E -- rejected: ignore or unconditionally retry `AccessDenied`, raise timeouts to make
CI green, or add configurable retry frameworks.** Weakens or evaluates the security property,
masks persistent errors, or exceeds scope; prohibited by the audit authorization.

Recommendation: Option A if any code change is authorized; Option B only if the owner prefers
zero production change; do not adopt D/E; do not change timeouts or workflows for CI.

## 9. Recommendation and proposed acceptance tests

Recommend Option A (bounded, kind-scoped retry of the identical inspection, fail-closed after the
budget) as the minimum defensible correction, contingent on a confirmation step: before
implementing, ideally capture the refusal once more with a temporary instrumented build on CI
(log the raw error and the sidecar state at the refusal) so the actor class moves from probable
to confirmed. This audit performs no such change.

Proposed acceptance tests for a future authorized fix: the Windows-only transient-denial unit
test and the unchanged link/junction suite (Option A, regression tests); the existing MCP
concurrency test as the end-to-end scenario; and a Windows CI run of the full `mcp` binary
(the original failure context) with no rerun masking the first result.

## 10. Remaining uncertainty and skipped checks

- The exact CI actor was not observed: no ETW/ProcMon/AV telemetry on the runner; no
  reproduction of `os error 5` locally (only its documented OS class was demonstrated).
- Pre-open scan (93) versus post-open re-check (131) cannot be distinguished from the message
  text; both produce identical output.
- The store state after the refusal was not observed by the test (panic skipped writer join);
  no mutation is expected from a read path refusing at inspection, but this is Unverified.
- Frequency evidence is weak: one Windows CI failure in the suite's known history (previous
  Windows runs were green), one immediate green rerun, 0/65 local attempts. The local machine,
  OS build, and Defender configuration are not a CI-runner mirror.
- Not checked: network/OneDrive-synced home directories (different Windows semantics), other
  Windows versions, Linux/macOS stress behavior (both attempts green; Unix `lstat` does not
  exhibit this denial class), and any kernel-level file state at the incident.
- Web-search tooling was unavailable during the audit (one failed attempt); references were
  instead taken from Context7 and the vendors' documentation fetched directly (section 11).

## 11. Tool usage and evidence index

- Context7 MCP: `/websites/sqlite_docs` (WAL/`-wal`/`-shm` lifecycle: checkpoint and unlink on
  last clean close; read-only-last-client nuance); `/websites/rs_rusqlite_rusqlite` (checkpoint
  APIs for the rusqlite 0.40 dependency). The SQLite antivirus-retry opcode query returned
  unrelated pages via Context7, so that citation was taken from sqlite.org directly.
- Superpowers: systematic-debugging (evidence and mechanism isolation before any remediation
  proposal; no fix written); verification-before-completion (every claim cross-checked against
  the captured logs/probe outputs before this document was committed).
- Ponytail (full): no code changes, no retry framework/abstraction introduced, probes are
  throwaway scripts outside the repository, and the remediation section records the smallest
  defensible options only.
- CI evidence (Observed): run 39201873049; jobs 117896351953 / 117896352103 / 117896352121
  (attempt 1) and 117897894583 (attempt 2, rerun of failed jobs); attempt-1 logs retrieved with
  `gh run view 39201873049 --attempt 1 --log`.
- Local evidence (Observed; temp directory `<USER_HOME>\AppData\Local\Temp\opencode\m8-wal-audit`,
  not committed): `attempt1-run-attempt1.log` (full attempt-1 logs, 562,581 bytes);
  sequential run logs `attempt-1..20.log` and `results.csv`; amplification logs
  `par-*.out.log` and `full-*.out.log`; `environment.log`; probe sources/scripts `walprobe.rs`
  (exact `std::fs::symlink_metadata` call), `delete-pending-demo.ps1`, `pending-state-demo.ps1`;
  mapped-section experiment run inline.

M8 remains OWNER ACCEPTED -- CLOSED. This audit changes no acceptance record; historical results
(frozen G1-G8, CP7-B deferral, untested Claude Code/Codex paths) stand as recorded.
