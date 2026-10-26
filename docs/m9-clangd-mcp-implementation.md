# M9 — clangd MCP integration: implementation and evidence

**Status: implemented under an OWNER GO (2026-10-21).** This document records
what was built, what was measured, and what is still unverified. The design
of record is `docs/m9-clangd-mcp-integration-design.md`; the owner's
decisions OD-1..OD-7 are listed there in §10 and were approved as stated in
§1 below.

- Branch: `m9-multirepo-release-candidate`
- Base: `a0bacfc4ec2ae966dafc8658a9b777d06d1ccec2` (the design-correction
  commit)
- Scope: **one** MCP tool. No schema change, no new dependency, no daemon, no
  LSP ingestion, no E12, no CP4, no release.

## 1. The owner's decisions, as implemented

| Decision | Approved | As implemented |
|---|---|---|
| OD-1 | Mode A: `read_only_hint=false`, background indexing active | the tool's annotations are `read_only_hint=false`, `destructive_hint=false`, `idempotent_hint=false`, `open_world_hint=true`; Riltra passes no `--background-index` argument, so the provider's default (indexing on) applies |
| OD-2 | the provider executable comes from the operator's `RILTRA_LSP_SERVER`, absolute path only | `mcp::provider_exe()` reads the variable and refuses an empty or non-absolute value with `provider_unavailable`; no PATH lookup, no auto-install, no client-supplied executable or argument |
| OD-3 | no pagination, no extra response schema | the document is exactly the CLI's `semantic query` document; the adapter's existing caps (8 MiB per message, 10 000 messages) still apply |
| OD-4 | the compilation database is provided by the operator | Riltra never generates, reads-as-authority, or validates one; the provider's own diagnostics carry whatever it says about it |
| OD-5 | no pre-flight compilation-database or readiness check | there is no pre-flight at all; the provider is asked, and its diagnostics are returned |
| OD-6 | D-M9-2 superseded **only** for this one-tool integration | `docs/m9-design-gate.md` §1.8 carries a dated note; the G-SEM status is unchanged |
| OD-7 | the health flags describe the **potential** effects of the whole tool | `capabilities.read_only=false`, `mutations=true`, `network=true`, with a comment in `mcp.rs` stating the scope: the provider may write its own index cache and may reach the network, while **Riltra itself still creates, migrates and replaces no store through this path** (the semantic query has no database handle) |

## 2. The source changes

| File | Change |
|---|---|
| `crates/cli/src/response.rs` | `semantic_hit_json` and `semantic_query_json` extracted from `main.rs`, byte-identical (the trailing newline is part of the format string, unlike the other formatters in that module); a unit test pins the bytes against a verbatim copy of the CLI's historical format |
| `crates/cli/src/main.rs` | the inline semantic-query formatter replaced by the shared one; the CLI's output bytes do not change |
| `crates/cli/src/mcp.rs` | the `Operation` enum, the `SemanticInput` struct, the `riltra_code_semantic` tool, `provider_exe()`, `semantic_query_error()`, `semantic_doc()` (repository → position → path → provider → query), the health document's capability flags and tool list, the module header |
| `crates/cli/src/index.rs` | one unit test for the weakest-fact verdict (the mixed verdict is not reachable through the adapter, see §4) |
| `crates/cli/examples/mock_lsp.rs` | three modes: `huge-answer` (an answer above the adapter's message cap), `echo-position` (the answer echoes the requested character), `barrier` (two calls must meet before either answers) |
| `crates/cli/tests/mcp.rs` | **thirteen** new tests (§4), the tools-list test updated to twelve tools with per-tool annotations, the health test updated to the OD-7 flags and to the twelve tool names, the G4 unsupported-field sweep and the `additionalProperties` loop extended to the new tool, a `run_cli_with_env` helper, a `recv_with`/`call_tool_with_timeout` helper, a `mock_lsp_bin` helper (`testutil` is `#[cfg(test)]`-only, so an integration test repeats the build/locate logic) |
| `README.md`, `docs/m9-design-gate.md`, `docs/m9-clangd-mcp-integration-design.md`, `docs/m9-clangd-pilot.md` | the tool count, the read-only claim, the dated design-gate note, the design's status header, and a dated note in the pilot report's §7.5 (its frozen raw evidence is untouched) |

## 3. RED / GREEN and the negative controls

**RED (before the implementation).** The first run of the new test block
failed on the missing pieces, not on the assertions: `error[E0425]: cannot
find function semantic_query_json` (the extracted formatter) and the MCP
tests failing at runtime with `unknown tool` (the tool did not exist yet).
The new unit test in `index.rs` failed to compile before the helper existed.

**GREEN.** `cargo test --locked`: **711 passed, 0 failed** (393 unit + 16 +
145 + 21 + 52 + 57 MCP + 15 + 12).

**The negative controls** — the behaviour that must *not* change:

| Control | Where |
|---|---|
| the CLI's `semantic query` output stays byte-identical | `response.rs`'s unit test (a verbatim golden) and the MCP/CLI parity test (a parsed-value comparison) |
| the eleven existing tools keep their behaviour | the existing MCP suite with only the tool count, the per-tool annotation branch and the health flags updated |
| the index and the graph edges are untouched, and Riltra writes nothing at all | `riltra_writes_nothing_to_the_repository_or_the_stores`: the database file's bytes, the edge-table row count, and the repository's and home's file listings, before and after. (The knowledge store is not opened by this path at all — the semantic query has no knowledge code — so it is not compared byte for byte.) |
| an empty answer is never "the symbol does not exist" | `semantic_empty_answer_carries_the_provider_diagnostics`: the empty answer carries the provider's diagnostic and `fallback: null` |
| a stale fact makes the whole answer stale | `semantic_marks_a_changed_source_stale` and `the_answer_verdict_is_the_weakest_fact` |
| references are never promoted to call edges | the same no-write test: the edge table is unchanged |
| the provider is never chosen by the client | `semantic_reports_a_missing_provider` (unset, relative) and `semantic_classifies_its_own_refusals` |
| a path that escapes the repository is refused | `semantic_refuses_an_escaping_or_missing_path` (five shapes plus a symlink out) |
| stdout stays protocol-only and the answer stays bounded | the harness asserts every stdout line is JSON-RPC; `semantic_keeps_mcp_stdout_pure_and_the_answer_bounded` (a 9 MiB answer is refused by the adapter's cap) |

## 4. The tests, and one that had to move

The design's §6.2 listed fourteen acceptance tests. Thirteen are MCP-level
tests; **one is a unit test**: "the answer's verdict is the weakest fact" is
not reachable through the MCP path, because the adapter marks **every** fact
stale when the queried file changed (`lsp.rs:802-812`) and a fact's digest is
computed from the bytes on disk at answer time (`lsp.rs:909-911`), so a
single stale fact among fresh ones cannot be produced deterministically. The
rule is proven by the unit test `index::tests::the_answer_verdict_is_the_weakest_fact`
instead. This is a deviation from the design's test plan, reported rather
than papered over.

| Design §6.2 | Where it landed |
|---|---|
| 1 parity (parsed value) + the formatter's bytes | `semantic_mcp_carries_the_cli_document` + `response.rs`'s unit test |
| 2 definition and references separate | `semantic_definition_and_references_stay_separate` |
| 3 provider missing / not absolute / missing executable | `semantic_reports_a_missing_provider` |
| 4 empty answer, compilation-database diagnostic, honest fallback | `semantic_empty_answer_carries_the_provider_diagnostics` |
| 5 stale | `semantic_marks_a_changed_source_stale` |
| 6 weakest-fact verdict | `index::tests::the_answer_verdict_is_the_weakest_fact` (see above) |
| 7 no index or graph change | `riltra_writes_nothing_to_the_repository_or_the_stores` |
| 8 provider crash, timeout, and the diagnostics that ride with a failure | `semantic_survives_a_provider_crash_and_a_timeout` (the `crash`, `hang` and `fail-after-diagnostics` modes) |
| 9 provider without the capability | `semantic_reports_a_provider_without_the_capability` |
| 10 stdout purity and the response bound | `semantic_keeps_mcp_stdout_pure_and_the_answer_bounded` |
| 11 invalid path, escape, symlink, repository isolation | `semantic_refuses_an_escaping_or_missing_path` |
| 12 the refusals and the check order | `semantic_classifies_its_own_refusals` |
| 13 UTF-16 positioning | `semantic_column_unit_is_utf16` |
| 14 two calls in flight | `semantic_survives_two_calls_in_flight` |

The design's §6.3 (the existing tests that change) is covered by the
updated tools-list test (twelve tools, per-tool annotations, the operation
enum, the disclosed containment text), the updated health test (the OD-7
flags and the twelve tool names) and the G4 sweep (the new tool's
unsupported field).

## 5. The measurements

All on this host, the debug build, the same provider and the same position.

### 5.1 The transport overhead (the question: is the MCP slower?)

Two measurements, both paired:

| Measurement | Result |
|---|---|
| one MCP round trip with no provider work (`riltra_repo_list`) | **1.4 ms median** (min 1.3, max 11.9, n=10) |
| the CLI's `semantic query --lsp-server` against the mock | 176.2 ms mean (n=10) |
| the MCP's `riltra_code_semantic` against the same mock | 160.9 ms mean (n=10) |
| the difference | **−15.3 ms**, i.e. **within the run-to-run noise**: the direction is not established (the CLI leg's spread is ±10 ms and the mock's own startup dominates both). What the pair does show is that the transport adds no material cost: the CLI's extra cost is its own process start and exit, which a long-lived server does not pay. |

**The transport overhead is ~1.4 ms per call, immaterial** against the
provider's own cost. It is measured, not assumed.

With the **real clangd** (LLVM 23.1.4, the pilot's fixture and its
CMake-generated compilation database):

| | Result |
|---|---|
| the CLI | 1811–5408 ms (median 2338 ms, n=5) |
| the MCP | 1711–1764 ms (median 1737 ms, n=5) |

The spread in the CLI leg is the provider's own cold-index readiness race
(the pilot's finding, `docs/m9-clangd-pilot.md:322-344`), not the transport.

### 5.2 The concurrency result

The design left the dispatcher's behaviour open ("not established by this
repository"). It is now **established, by measurement**:

- **The rmcp 3.5.1 dispatcher spawns every peer request into its own task**
  (`rmcp-3.5.1/src/service.rs:1603`, `spawn_service_task` → `tokio::spawn`).
- **Observed with a barrier**: two `tools/call` requests sent before either
  answer is read both reach the mock's barrier; the second instance sees the
  first's marker and answers while the first is still waiting (probe output:
  one live mock process and one `wait-<pid>` marker after 8 s, with the
  first answer already returned).
- **Observed with two real clangd processes on one cold index**: the index
  cache was dropped, two concurrent `tools/call` requests were sent, and both
  answers arrived **2.38 s** after the calls — one provider's time, not two —
  each with a definition and **no fallback**; the provider's cache was then
  written (12 `*.idx` files) and a **third run read it back** and answered
  again with a definition and no fallback.

**What this does and does not prove.** It shows that two overlapping calls
both succeed, that the cache survives this run, and that the transport
serializes neither. It is **one observation, not a proof**: no claim is made
that concurrent writers cannot corrupt the cache in some other interleaving.
The mock's barrier rules (per-process marker, empty marker directory, a
timeout that is a hard failure with a `timeout-<pid>` marker) make the
client-side test fail rather than pass on a sequential dispatcher.

**The dispatcher was not changed** and no concurrency infrastructure was
added, exactly as instructed.

**One deviation from the design's barrier rules** (§6.2 test 14): the
design asked each mock instance to *remove* its marker when it proceeds;
the implementation keeps the marker until the process exits. Removing it
races — an instance that has already proceeded disappears from the peer's
sight — and the barrier then timed out in the first run, which is how the
race was found. The rules that make the test meaningful are kept: per-process
marker names, an empty marker directory at the start, and a timeout that is a
hard failure (a `timeout-<pid>` marker, a non-zero exit) so that no
well-formed answer is possible without a peer.

## 6. The observed filesystem effects, and their limits

- The **provider** writes `<project>/.cache/clangd/index/*.idx` — inside the
  repository (observed: 12 index files under the pilot fixture's `.cache`).
  Riltra does not write it; the provider does.
- **Riltra writes nothing**: the database file's bytes, the edge rows, the
  repository's file listing and the home's listing are all unchanged after a
  call (`riltra_writes_nothing_to_the_repository_or_the_stores`).
- The provider's **system-header cache** goes to the user cache directory
  (outside the repository) — not observed in this run, and not Riltra's
  business.
- The MCP **stdout carries JSON-RPC only** (the harness asserts it on every
  line it reads); the provider's stdout and stderr are the adapter's pipes.

## 7. The independent review

The implementation was audited by an independent reviewer
(`opencode/deepseek-v4.1-flash-free#high`), read-only, over the whole
working-tree change before the commit.

**Round 1 — NO-GO.** Fourteen findings: the README still claimed a
read-only server; the evidence record claimed a CI result that cannot exist
for an uncommitted change; the new tool was missing from the
schema-refusal sweep and the `additionalProperties` loop; the "no provider"
test was not hermetic (it inherited the operator's `RILTRA_LSP_SERVER`); the
empty-answer test could exceed the harness's 15 s timeout; the latency
record stated a direction from two means; the health test asserted only a
count; the schema refusals asserted only `is_error`; the record claimed a
knowledge-store comparison the test does not make; the test count was
wrong; the existing suite's changes were understated; the mock's barrier
deviated from the design's marker rule; the tool description had lost the
containment text; and `response.rs`'s module header no longer matched the
semantic formatter's newline rule.

**Round 1 also required one check the reviewer could not run**: that the
CLI's old format strings and the extracted ones are byte-for-byte equal,
the trailing newline included. It was performed before the second round
(`git show a0bacfc:crates/cli/src/main.rs`): the old `semantic_hit_json`
format string and the old `cmd_semantic_query` format string (with its
`\n`) are identical to the ones now in `response.rs`, so the CLI's output
bytes did not change.

**Round 2 — GO for the code**, conditional on two documentation
corrections (the dated note in `docs/m9-clangd-pilot.md` §7.5 that the
design requires, and the CI sentence in §8 of this record, which cannot
reference the CI run of its own commit). Both were made. The reviewer's
words:

> The second review round verified all fourteen applied corrections in the
> working tree — the schema sweeps, the hermetic environment, the 30 s
> empty-answer timeout, the health tool-name list, the refusal text, the
> per-tool annotation branch, the barrier deviation note, and the
> README/module-header/response-header wording — and none of them weakened a
> test. No correctness, security, regression, or scope defect was found in
> `riltra_code_semantic`, the shared semantic formatter, the LSP adapter
> path, or the CLI/MCP parity: the tool validates repository → position →
> path → provider, opens no store, and the CLI's output bytes are unchanged.
> The verdict is GO for the code, conditional on two one-paragraph
> documentation corrections in the same commit: the dated note in
> `docs/m9-clangd-pilot.md` §7.5 required by design §9.1, and the CI
> sentence in this record's §8, which cannot reference the CI run of its own
> commit.

**Round 3 — GO for the code and the commit**, conditional on two
documentation corrections in the same commit (the pilot note's sentence and
the §4 table's numbering), which were made. The reviewer's words:

> **Round 3 — GO for the code, conditional on two documentation
> corrections in the same commit.** The third review round verified every
> round-2 correction landed and that none of the new assertions is vacuous:
> the five path-refusal texts match `semantic.rs:319-350`; the
> `fail-after-diagnostics` diagnostic reaches the fallback reason through
> `lsp.rs:598-638`, `index.rs:2466-2482` and `lsp_fallback_reason`
> (`index.rs:1045-1050`); the tools-list test pins
> `document path escapes repository` (`mcp.rs:386`); the missing-provider
> path is under the test's own home and asserted absent; and the no-write
> test compares the database's bytes, not a digest. No correctness,
> security, regression, or scope defect was found in the code, and the
> owner's constraints hold as far as the working tree shows.

The reviewer's independent checks that stand: the provider is unreachable
from the request (no executable, argument or environment field in the input
schema; `provider_exe()` reads only the operator's variable and refuses an
empty or non-absolute value); the repository resolution is the same
registry + canonical-root guard the other tools use; the path is validated
twice (in the tool and again inside the query); the check order matches the
design; no production `fs::write`/`create_dir` exists on the LSP or
semantic path; and the extracted formatter's key order matches the frozen
pre-change CLI output in the pilot's raw evidence.

## 8. The test and CI evidence, and what is NOT verified

**Verified here:** `cargo fmt --all -- --check` clean; `cargo clippy
--all-targets -- -D warnings` clean; `cargo test --locked` 711 passed / 0
failed; `git diff --check` clean; gitleaks 8.30.1 (with the repository's
config) clean. **The CI result for this change is not part of this document**:
the run is referenced in the delivery report and in the branch's CI page, not
here — an evidence record must not claim a result it cannot show.

**A real OpenCode session** (not a mock) against `riltra mcp` with the
provider configured — the agent called the tool and reported the document:

```
riltra_riltra_code_semantic {"col":8,"line":12,"operation":"definition","path":"src/point.cc","repo":"demo"}
```
```json
{"definitions":[{"display_name":null,"path":"include/geom/point.h","precision":"Heuristic",
  "roles":1,"span":{"byte_end":582,"byte_start":578,"end_col":11,"end_line":21,
  "start_col":7,"start_line":21},"symbol_uri":null,"verification":"Unverified"}],
 "fallback":null,"provider":"lsp-subprocess",
 "provider_version":"clangd version 23.1.4 (…7a941dfb…) windows x86_64-pc-windows-msvc",
 "references":[],"repo":"demo","server_diagnostics":[],"verification":"Unverified"}
```

**NOT verified:**

1. **The parallel cache safety is one observation** (§5.2), not a proof.
2. **The real-clangd path is local-only evidence**: CI has no clangd and no
   C++ toolchain, so the MCP tests run against the mock; the real-provider
   runs above were local.
3. **The hang test takes the adapter's own 30 s deadline** (one CI job is
   ~35 s slower because of it).
4. **The reference list of a large project is not bounded by Riltra** (OD-3):
   the answer is whatever the provider sent, up to the adapter's 8 MiB
   message cap.
5. **E12 stays open**: the tool answers "definition" and "references"; it
   makes no construction/declaration claim.
6. **The Windows/macOS/Linux behaviour of the new tool** rests on the CI
   jobs (the mock-backed tests are cross-platform; the real-provider runs
   were on this Windows host only).

## 9. The gate status, on the actual evidence

- **G-MCP (OpenCode interoperability):** the evidence for this change is a
  real OpenCode session that called the new tool and received the document,
  plus the 12-tool catalog and the stdout-purity tests. The gate's own
  release acceptance (a candidate-level smoke against the release candidate)
  is still **NOT YET VERIFIED** — the gate is not closed by this work.
- **G-SEM (semantic providers):** still **NOT YET VERIFIED** for C++: this
  work adds no provider-family evidence, no MMSP-scale C++ measurement and no
  completeness claim. The D-MCP tool answers one position at a time; it
  proves the *path*, not the *coverage*.
- **G-SEC:** the no-write guarantees are now covered by a test (§3); the
  provider's cache write inside the repository is documented, not prevented.

## 10. The commit boundary

The frozen CP1–CP3 evidence, the pilot's frozen measurements and the
repository's `docs/pilots/clangd/` archive are **untouched** by this work.
The only production surface is the §2 list. No dependency was added, so
`docs/licensing.md` does not change. No MCP tool other than the new one
changed its contract; the health tool's output changed with the owner's
approval (OD-7).
