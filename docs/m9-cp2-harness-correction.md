# M9 — CP2 harness correction: the amendments of 2026-10-22

**Status: HARNESS CORRECTED — AWAIT INDEPENDENT AUDIT AND OWNER GO.**
The CP2 pilot procedure could not be reproduced: the home copy buried the
registry, the frozen grader still demanded the eleven-tool MCP surface, and
the OpenCode session neither delivered its prompt nor depended on a
project-local MCP configuration. This record is the limited amendment that
fixes the *test procedure*. **No product behaviour changed, no acceptance
criterion was lowered, and no historical verdict was rewritten.**

- Branch: `m9-multirepo-release-candidate`
- Starting HEAD (the audited baseline): `ffe776d8830014c7671af655bdf381a89733fae5`
- The corrections are **harness-side only**: PowerShell and Node scripts under
  `docs/pilots/`. `crates/` is untouched.
- The pilot input, the ground truth and the frozen acceptance criteria are
  untouched: `docs/m9-cp1-pilot-spec.md` is not in this diff.

---

## 1. The three defects and their root causes

### A. The home copy buried the registry

`run-all.ps1` step 2 was:

```powershell
$homeMain = Reset-BenchDir (Join-Path $bench 'home-main') -Root $bench
Copy-Item (Join-Path $bench 'home-cold1') $homeMain -Recurse -Force
```

**Root cause.** `Reset-BenchDir` (`common.ps1`) *creates* the destination
directory, and `Copy-Item <directory> <existing directory> -Recurse` copies
the **source folder into** the destination rather than its contents. The
destination therefore always existed, so the copy always nested:
`home-main/home-cold1/registry.json`. `RILTRA_HOME` pointed at
`home-main`, whose registry was not there, and every command answered
`unknown repository: 'mmsp-pin'` (exit 3).

The defect is pinned by a characterization test
(`test-harness.ps1`, "the historical copy operation nests the source
folder"), so the mechanism cannot silently change.

**Correction.** One helper, `Copy-BenchDir` in `common.ps1`, copies the
*contents* of the source into the destination; `run-all.ps1` calls it and
now fails loudly if the copied home does not answer:

```powershell
$homeMain = Reset-BenchDir (Join-Path $bench 'home-main') -Root $bench
Copy-BenchDir -Source (Join-Path $bench 'home-cold1') -Destination $homeMain -Root $bench
$env:RILTRA_HOME = $homeMain
& $cfg.Bin index status --repo mmsp-pin
if ($LASTEXITCODE -ne 0) { throw "the copied home does not answer: index status failed with exit $LASTEXITCODE" }
```

### B. The frozen grader demanded the eleven-tool surface

**Root cause.** `grade.cjs` compared the MCP catalog's size against a
hard-coded `11`, the surface of the CP2 run in 2026-10-20. The twelfth tool
(`riltra_code_semantic`, merged in `1b162b5`) made the check fail for the
current server — a check that no server could satisfy, not a product defect.

**Correction.** The count follows the approved MCP contract (**12**), and
the count alone is not enough: the catalog must be **exactly** the approved
set — a catalog of the right size that lacks an approved tool, or repeats
one, is a different surface. The other four conditions of the check (stdout
purity, empty stderr, at least one answered call, the repository identity in
the answer) are unchanged, and the classified-error check (`D5-H2`) is
untouched.

```js
const MCP_TOOL_COUNT = 12;
const MCP_REQUIRED_TOOLS = [ /* the twelve approved names */ ];
...
const sortedTools = toolList.slice().sort();
const expectedTools = MCP_REQUIRED_TOOLS.slice().sort();
const requiredToolsPresent =
  sortedTools.length === expectedTools.length && sortedTools.every((t, i) => t === expectedTools[i]);
const surfaceOk = toolCount === MCP_TOOL_COUNT && requiredToolsPresent;
const ok = surfaceOk && nonJson === 0 && stderrBytes === 0 && answered.length > 0 && grounded;
```

A surface that does not match now says so (`mcp-tool-surface-mismatch`)
instead of the undifferentiated "did not answer cleanly".

### C. The OpenCode session was truncated and unconfigured

**Root cause 1 — the prompt.** `opencode-session.ps1` embedded the
multi-line prompt in a generated `.cmd` file. A batch file executes one
command per line, so the argument was truncated at the first line break and
the agent received no question ("No question came through in your message").

**Root cause 2 — the global configuration.** The script expected the
Riltra MCP server to be registered in the **global** OpenCode configuration.
It was not, so the agent found no server, registered one **of its own** —
pointed at the original MMSP clone — and started an index build there. The
clone's state was unchanged, but the CP1 isolation rule ("no MMSP source is
read") was broken once.

**Correction.** The session is now self-contained:

- a **project-local** `opencode.json` (`<sessionDir>/opencode.json`) registers
  exactly one MCP server, `riltra`, pointing at the isolated pilot home
  (`RILTRA_HOME`). The global configuration is **not written** and not
  relied upon: OpenCode discovers `<project>/opencode.json(c)` from the
  working directory (the session directory), which is the documented
  precedence order, so the session works whether or not a global
  configuration exists. (What the *agent* does with a global configuration
  is OpenCode's behaviour, not the harness's; the evidence that the
  session used the pilot's server is in §4.3.)
- the prompt is passed as **one argument on one line**
  (`ConvertTo-SingleLinePrompt`): the instruction content is preserved, the
  line breaks are not. The non-batch branch quotes the argument itself, so
  the prompt cannot be split into many arguments.
- **fail closed, on evidence**: the tool calls are read from the stream's
  **structured `tool_use` events** (`Get-SessionToolCalls`), never scraped
  from the text — a tool name in a sentence, in an error or in a tool result
  is not a call. `Get-SessionExitCode` turns a clean exit that made **no
  Riltra tool call** into exit 3, so such a session is recorded as blocked
  and never as a pass. The answer is the agent's **final message**
  (`Get-SessionAnswer`: the last text part), so a citation in an
  intermediate note cannot be counted as a citation of the answer.
- **the isolation guard cannot be switched off**: `-MmspRoot` is mandatory,
  and `Assert-IsolatedHome` refuses a home that *is*, *lives inside*, or
  *registers* the original repository — the registered root is resolved
  first, so a subdirectory, a parent or a **direct** junction of the
  original is caught too. (A *chain* of junctions is not resolved
  recursively — that limitation is pre-existing in `Resolve-RealPath` and is
  out of scope for this correction.) The same guard covers the snapshot path when the harness is run as a whole (`run-all.ps1`); the standalone snapshot steps keep their own optional guard.

**Two further corrections in the same script, disclosed.** They are the same
defect class (a hard-coded value that decides a verdict instead of a
measurement) and they are in the script the brief already puts in scope:

- the summary's `cited_sources` was the constant `false`, which made the
  `D5-H3` check fail for **every** session, whatever the answer said. It is
  now measured from the answer text (a repository-relative `path:line`
  citation). The criterion itself is unchanged.
- the summary's `reason` was the constant `provider-auth-403` for every
  failure. It is now the truthful reason (`mcp-server-not-reached` when the
  tools were never reached, `session-failed` otherwise); the provider's own
  error text is still captured in `error`.

---

## 2. The amendments, file by file

Every change is a **limited amendment** to a method script. The "before" is
the blob id at the baseline commit `ffe776d`; the "after" is the blob id in
the final tree of this correction.

| File | Before (blob at `ffe776d`) | After | Why |
|---|---|---|---|
| `docs/pilots/cp2/method/common.ps1` | `c87e262d9c8d5e98fda5db5f7293c8e8d46b0478` | `484ef3eb05f5b460a0fb8bfe28ba42eb2b9bc5b3` | the copy helper, the isolation guard, the prompt flattening, the project-local config writer, the event-stream reader (the structured tool calls and the final message), the fail-closed outcome
| `docs/pilots/cp2/method/run-all.ps1` | `ed570b4b066aa4f54862b08c2f011e03e3996f60` | `3d14850f6483f19333c4f4fbcb837ed2c387cf8a` | the copy step (defect A), the new test in step 0 and the D.5 re-grading after the MCP smoke
| `docs/pilots/cp2/method/grade.cjs` | `adec365656bf56cba1498e17d60fc720e17abc37` | `7c42958fbfc76a8c997fafc94c82a41d26efe8dd` | the twelve-tool catalog contract (defect B)
| `docs/pilots/cp2/method/grade.test.cjs` | `4cda3e1c3ff7c6ef63f7d89db3ac947447cff090` | `2e68cade8bf5b0405a8064a9f3bc29131297c7c3` | the twelve regression cases for the catalog contract and the blocked-session verdict
| `docs/pilots/cp2/method/opencode-session.ps1` | `e5c40dbb1718d4abbdf39f808cdda572e9c269f3` | `e30343b9c149ceab634f9c5aa9aaabfa921d0713` | the prompt, the project-local config, the isolation guard, the structured tool calls, the measured citations, the fail-closed outcome (defect C)
| `docs/pilots/cp2/method/test-harness.ps1` | `(new file)` | `3f2ee5aaaafb2051775294c5247adfed0550700f` | the regression tests of the harness helpers
| `docs/pilots/cp2-reeval/method/run-steps.ps1` | `20420fdf544b99cfafee4a1056567f60a1401a6c` | `e674fd971cb7b1ab57b0f5ee7599cf552e2f215d` | explicit parameters instead of placeholder defaults; the behaviour of the 2026-10-21 run is unchanged
| `docs/pilots/cp2-reeval/method/opencode-session-d5.ps1` | `698767591351337ddec6ca04f1263de9acc4d7ac` | `e836c934220fb734311294bab21915f65e634ed1` | the same, plus a note that the canonical harness now supersedes it
| `docs/pilots/cp2/method/README.md` | `a38d8480eaabc6ee37b22e9407fc482159d54b2a` | `794cbc53b527cb1937c2c794c15878125b12c6cc` | the amended procedure and the new scripts
| `docs/pilots/cp2-reeval/README.md` | `0fe9bc88a60f0d121a2b0892afe1fa28310f358b` | `dd4ec6274ee5b3693fac6533029001d5ec95cc37` | the reproduction commands with every parameter explicit

**The historical evidence was produced with the historical harness.** The raw
evidence under `docs/pilots/cp2/raw/**`, `docs/pilots/cp2/raw-audit/**` and
`docs/pilots/cp2-reeval/raw/**` was produced by the scripts as they stood at
their own commits. It is **not** regenerated, not re-graded and not touched:
the D5-H1 verdict in `docs/pilots/cp2-reeval/raw/grading.json` stays
**FAIL** (`mcp-surface-did-not-answer-cleanly`), because that is what the
historical grader said about the historical evidence. The corrected grader
applies to the **next** evaluation; §4.2 records what it says about the same
archived evidence, as clearly separated additional evidence.

---

## 3. The RED → GREEN evidence

| # | The property | RED (before the fix) | GREEN (after the fix) |
|---|---|---|---|
| 1 | the copied home holds the source's contents at its own root | `FAIL … (The term 'Copy-BenchDir' is not recognized …)` | `PASS the registry is at the root of the copied home` / `PASS the source folder is not nested in the copied home` |
| 2 | the defect's mechanism is pinned | `PASS the historical operation nests the source folder` (characterization: it nests **and** leaves no registry at the root) | unchanged (it still documents the defect) |
| 3 | the approved twelve-tool catalog passes | `not ok 22 - the MCP surface with the approved twelve-tool catalog passes` | `ok 22` |
| 4 | an eleven-tool catalog is rejected | `not ok 23 - an eleven-tool catalog is rejected` (it was accepted) | `ok 23` (reason `mcp-tool-surface-mismatch`) |
| 5 | a thirteen-tool catalog is rejected | `not ok 24 - a thirteen-tool catalog is rejected` (it was accepted) | `ok 24` |
| 6 | a twelve-tool catalog without `riltra_code_semantic` is rejected | `not ok 25 - a twelve-tool catalog without the semantic tool is rejected` (it was accepted) | `ok 25` |
| 7 | stdout purity, stderr, an answered call, the repository identity, the classified error still decide the check | `ok 26` … `ok 30` **before** the fix (they pin the behaviour the correction must not weaken) | `ok 29` … `ok 33` (the file order changed when the three new cases were inserted) |
| 8 | the prompt keeps its instruction content | `FAIL … (The term 'ConvertTo-SingleLinePrompt' is not recognized …)` | `PASS the flattened prompt has no line break` + the three instruction checks |
| 9 | the MCP configuration is project-local, one server, pointing at the isolated home, the global configuration untouched | `FAIL … (The term 'New-OpenCodeProjectConfig' is not recognized …)` | the eight configuration checks pass |
| 10 | a home that registers the original repository is refused | `FAIL … (the refusal was: 'The term 'Assert-IsolatedHome' is not recognized …')` | `PASS a home registering the original repository is refused` (the refusal names the original repository) |
| 11 | a session that never called a Riltra tool does not pass | `FAIL … (The term 'Get-SessionExitCode' is not recognized …)` | `PASS a session without Riltra tools fails closed` |
| 12 | a tool name in the agent's **text** is not a tool call | `FAIL … (The term 'Get-SessionToolCalls' is not recognized …)` | `PASS a session whose text names a Riltra tool fails closed` + `PASS that session made no tool call` |
| 13 | the answer is the agent's **final** message | `FAIL … (The term 'Get-SessionAnswer' is not recognized …)` | `PASS the answer is the last text part` + `PASS a citation in an intermediate note is not a citation of the answer` |
| 14 | the procedure **uses** the corrected helpers | `FAIL opencode-session.ps1 reads the answer from the event stream (the answer is not read from the stream)` | the seven call-site checks pass (`run-all.ps1` uses `Copy-BenchDir` and no longer copies a directory into an existing one; the session writes the project config, asserts the isolated home, computes the outcome from the tool calls, reads the answer from the stream, and clears the stale summary before it starts) |
| 15 | a home registering a **subdirectory or a parent** of the original is refused | `FAIL … (the refusal was: '')` | `PASS a home registering a subdirectory of the original is refused` + `PASS a home registering a parent of the original is refused` |
| 16 | the isolation guard cannot be **switched off** | `FAIL … (the refusal was: '')` | `PASS the guard cannot be switched off by omitting the original repository` |
| 17 | the copy refuses a **non-disjoint** source and destination, and a destination that is not empty | `FAIL copying a directory into itself is refused (the refusal was: 'Cannot overwrite the item … with itself.')` | `PASS copying a directory into itself is refused` + `PASS copying a directory into its own subdirectory is refused` + `PASS a destination with leftover content is refused` |
| 18 | a twelve-tool catalog missing **another** approved tool, or with a **duplicate**, is rejected | `not ok 26 …` / `not ok 27 …` (both were accepted) | `ok 26` / `ok 27` (reason `mcp-tool-surface-mismatch`) |
| 19 | a session that reached **no tool** is not a passing agent session | `not ok 28 …` (it passed) | `ok 28` (the verdict is `UNAVAILABLE`, and no D5-H3 is produced) |

**The test commands** (all from the repository root):

```powershell
pwsh -NoProfile -File docs/pilots/cp2/method/test-harness.ps1   # 41 checks
node --test docs/pilots/cp2/method/grade.test.cjs               # 33 tests
node --test docs/pilots/cp2/method/fact-digest.test.cjs         # 16 tests
pwsh -NoProfile -File docs/pilots/cp2/method/test-path-safety.ps1
```

---

## 4. The targeted smokes

The full CP2 benchmark was **not** repeated. Three targeted smokes carry the
proof:

1. **The copy, against the real binary.** `Copy-BenchDir` copied the pilot's
   `home-cold1` into `home-main` in the bench workspace; the registry and the
   index are at the root of the copy, no `home-main/home-cold1` exists, and
   `riltra index status --repo mmsp-pin` answers `fresh` with **exit 0**.
   The original MMSP clone was not modified — the same
   HEAD, the same two untracked entries, an empty diff and a `.git`
   directory last written on 2026-10-20. The whole
   record is archived as `raw/copy-smoke.txt`.
2. **The corrected grader, against the archived evidence.** The corrected
   `grade.cjs` was run over `docs/pilots/cp2-reeval/raw/queries/`,
   `docs/pilots/cp2-reeval/raw/mcp/mcp-smoke.json` and **this run's**
   session summary `docs/pilots/cp2-harness-fix/raw/opencode-session.json`
   (all read-only). The result is
   archived as clearly separated additional evidence in
   `docs/pilots/cp2-harness-fix/raw/grading-corrected.json`: `D5-H1` is now
   **PASS** (`tool_count = 12`, `required_tools_present = true`), the grader
   exits 0, and **every other verdict is unchanged** (18 honesty PASS, 1
   honesty UNAVAILABLE, 4 capability UNMET, 1 evidence UNAVAILABLE, 1 evidence
   PASS). The historical `raw/grading.json` still says `D5-H1 = FAIL`.
3. **The session, against the real agent.** The corrected
   `opencode-session.ps1` ran a real OpenCode session in the pilot home
   (the copied `home-main`), with the project-local configuration,
   `-TimeoutSec 3600` and `-Model opencode/minimax-m3.1-flash-free`:

   | | |
   |---|---|
   | the session | **exit 0**, `reason: null`, **1 230.6 s** (the provider was slow in this run — the wall time is reported as measured, with the load condition, never as a mean), **706 544 B** of stdout, **0 B** of stderr |
   | the tools the agent **called** | **69** tool calls, read from the stream's structured `tool_use` events, each with its status — the Riltra tool calls, the semantic tool among them (it failed, with the provider's own `provider_unavailable` refusal, which the agent quoted) |
   | the prompt | **delivered**: the agent answered Task 1 (the D.5 question) and Task 2 (the unknown repository) |
   | the classified error | quoted exactly, as the tool returned it: `{"kind":"not_registered","message":"unknown repository: 'mmsp'"}` |
   | the answer | the agent's **final** message, with **29** repository-relative `path:line` citations (`cited_sources: true`, `citations_found: 29`, both measured from that message) and its own statement of what it could not resolve ("`riltra_code_search` cannot see the option", "`riltra_code_semantic` is unavailable", "all graph edges come back `kind: POTENTIAL_CALL` … `verification: Unverified`") |
   | the agent's sources | the archived `execute` inputs (`raw/opencode-session.execute-inputs.txt`) show that the agent's four `execute` calls were Code Mode snippets: the first two call `tools.riltra_*` — the Riltra MCP tools — and the last two call `tools.opencode.list_mcp_resources`; **none of them reads the source filesystem**. The archived answer, summary and execute inputs contain **no** reference to the original clone, only the pilot snapshot's root |
   | the original MMSP clone | **untouched**: the same HEAD, the same two untracked entries, an empty diff against HEAD, and the newest `.git` metadata still dated 2026-10-20 (`raw/copy-smoke.txt`). No second server appears in the tool-call record (the registration itself is a configuration action, which the archive does not contain) |
   | the store | the before/after digests are **identical** - the database, the two `-wal`/`-shm` sidecars and the registry are unchanged |

   **One disclosure.** An *earlier* session of the same corrected harness —
   the one whose summary exposed the byte-count defect, and therefore not
   archived — also read files directly (its `read`/`grep`/`shell` calls) and
   said so in its answer ("Correction to the task premise: the filesystem is
   readable"). The archived run does not: its `execute` inputs are archived
   and call only the Riltra tools. The agent's behaviour varies between
   runs; the harness's evidence is the **structured tool-call record**, not
   the agent's account of itself.

---

## 5. What the correction does **not** do

- It does not change the product: `crates/` is untouched, no schema, no
  dependency, no MCP contract, no Rust code.
- It does not lower a criterion. **One criterion changed, and it was a
  deliberate correction, not a relaxation — see §5.1.**
- It does not change any *other* honesty or capability criterion. The
  `D5-H1` check still requires a source-grounded answer with the repository
  identity, a pure stdout, an empty stderr and at least one answered call.
- It does not change what a capability gate means: all four measured
  capability gates stay `UNMET`, and the D5 capability gate stays the
  pre-registered `UNMET`.
- It does not rewrite a historical verdict, and it does not regenerate any
  archived raw evidence.
- It does not create a compilation database, implement E7/E9/E12, add CUDA
  support, or start CP4, a merge, a tag or a release.
- It does not read, index or experiment on the original MMSP clone.

### 5.1 The one criterion that changed: CP1 D.5.4's tool count

CP1 §D.5.4 (frozen) describes the MCP surface as "the 11 read-only tools",
and the frozen grader compared the catalog against `11`. The approved
design gate (`docs/m9-design-gate.md` §1.8) says the surface "serves 12
tools … eleven read-only ones and `riltra_code_semantic`", and the twelfth
tool was merged in `1b162b5` — after CP1 was frozen and after the first CP2
run graded against the eleven-tool surface.

⇒ **The grader now requires the approved twelve-tool surface, and the
catalog must be exactly that set** (a catalog of the right size that lacks
an approved tool, or repeats one, is a different surface and is rejected).
This is a correction of a stale expectation to the owner's approved MCP
contract; it is not a lowering of a criterion, and it does not touch the
honesty or capability *gates*. CP1's frozen text is **not** edited: the
divergence is recorded here, and the historical verdict it produced stays in
the archive as `FAIL`.

---

## 6. The commit

The "after" blob ids of the amended files, the final remote HEAD and the CI
run are recorded in the commit message and in the final report of this task.
