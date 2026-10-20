# M9 clangd semantic provider pilot — evidence report

Owner decision: **APPROVED, Approach A** (clangd through the existing LSP
infrastructure). Owner brief 2026-10-21. Branch
`m9-multirepo-release-candidate`, pilot start HEAD
`a3e789dde8afc364fb0a653eddd6fe4ccfe43496`.

This report records what was measured, what the measurements prove, and the
minimum change the measurements justify. **No production code was changed.**
The pilot's artefacts are the fixture, the harness and this report; the
production integration waits for the independent audit and the owner's GO.

Evidence archive: `docs/pilots/clangd/` (method scripts + redacted raw
output).

---

## 1. Verdict

| Question the brief asked | Answer, measured |
|---|---|
| Can clangd fix the C++ semantic gaps without breaking the native architecture? | **Partly.** It answers E1 and E7 correctly at query time, through the LSP adapter that already exists, with **no code change**. |
| Is clangd optional? | **Yes, already.** It is spawned only when `--lsp-server` names it; without it the CLI never touches a server. |
| Does E12 (declaration vs construction) get fixed? | **No.** Neither the definition answer nor the reference list separates a construction from a declaration, and the semantic contract has no field that could carry the difference. |
| Are references distinguishable from calls? | **Yes, in the contract** (`Roles.definition` vs `Roles.reference`, `edge_kind` already has `REFERENCES` and `POTENTIAL_CALL`), and the pilot confirms the provider can be trusted to say *which* is which. But clangd's own reference list for a constructor contains mere type mentions, so a reference still is not a call. |
| Is the result deterministic? | **No.** The answer depends on the compilation database's coverage of the translation unit that holds the definition, and on whether the project index has been built yet (§7.1). |
| Is the answer fresh? | **Not always.** The evidence can carry the digest of the current bytes together with a span that those bytes do not contain (§7.3). |

**Overall: the pilot PASSES for E1 and E7 at query time, FAILS for E12, and
finds two honesty defects in the shipped path that the owner must decide
about (§7).** The minimum change the measurements justify is one small
adapter change plus documentation (§8).

**This pilot changes no release gate.** D-M9-2 (defer clangd) and G-SEM
(NOT YET VERIFIED) stand exactly as they are: `docs/m9-design-gate.md:806`
defers any clangd configuration or integration until a separate
evidence-backed proposal plus OWNER GO, and this report is evidence for such
a proposal, not the proposal. C++ semantics in M9 remain **NOT YET
VERIFIED**, and nothing in the pilot may be cited as a release-gate pass.

---

## 2. What already existed (the audit)

No new infrastructure was needed for the pilot; the repository already had
the parts Approach A assumes.

| Piece | Where | State |
|---|---|---|
| LSP subprocess adapter | `crates/cli/src/lsp.rs` | std-only JSON-RPC, Content-Length framing, absolute-path-only spawn, no shell, bounded timeouts, kill+reap on every failure path, repository containment on every URI |
| Semantic evidence contract | `crates/cli/src/semantic.rs` | one `Evidence` shape (location + optional symbol + roles + relationships), one path validator, one range checker, one diagnostics shape, a static capability registry |
| Live semantic query | `crates/cli/src/index.rs:2392` (`semantic_query_lsp`) | single hop, ephemeral, never persisted, every answer `Unverified` |
| CLI surface | `crates/cli/src/main.rs:928` | `riltra semantic query --repo R --path P --line L --col C [--refs] [--lsp-server <abs path>]` |
| C/C++ language id | `crates/cli/src/syntax.rs` | `.cc/.cpp/.cxx/.h/...` classify as `cpp`, and `lsp.rs:571` sends that as the LSP `languageId` — the id clangd requires |
| Edge kinds | `crates/cli/src/store.rs:213` | `code_edges.edge_kind` already distinguishes `REFERENCES` from `POTENTIAL_CALL` |
| Provider registry | `crates/cli/src/semantic.rs:208` | `lsp-subprocess` is a declared capability with definitions + references |

What the pilot adds: the C++ **language id is already right**, the adapter is
**already generic**, and the production command already reaches any language
server. The gap was never the adapter; it was the absence of a language
server and a compilation database to point it at.

The compilation database is the one prerequisite, and the design gate had
already recorded how it works (`docs/m9-design-gate.md:266-274`): clangd
discovers it in **parent directories and `build/` subdirectories**, and
CMake only exports one with a Makefile or Ninja generator — the Visual
Studio generator does not, which is why the pilot configured the fixture
with the NMake generator. The pilot confirms both halves of that statement
(§6.1, `raw/env/clangd-check-fixture.txt`).

Two limits found in the audit, both confirmed by measurement:

1. **`semantic_query_lsp` passes no argv.** The server is spawned with no
   arguments, so no `--compile-commands-dir` and no `--background-index`.
   §7.1 shows this costs nothing: clangd finds the database by walking up
   from the file (and in a `build/` subdirectory), and the background index
   is on by default.
2. **The adapter discards the server's diagnostics.** `lsp.rs:493`
   (`note_notification`) records `notification: textDocument/publishDiagnostics`
   — the notification's *name*, not its payload. The errors clangd itself
   reports about a file (an unparsable file, an ambiguous call) never reach
   the caller. §7.2.

---

## 3. The environment (what was available, what was installed, where)

Everything ran in an isolated bench root
(`%TEMP%\opencode\clangd-pilot`), never in the original repositories, and
nothing was installed system-wide. (Two transient compiler artefacts — an
object file and a proof directory — are written under `%TEMP%` itself, and
nothing else leaves the bench root.)

| Tool | Availability on this host | What the pilot used |
|---|---|---|
| C++ compiler | **absent from `PATH`**, but MSVC Build Tools 2022 **are installed** (`cl.exe` 19.34.31948, MSVC 14.34.31933, Windows SDK 10.0.19041.0 / 10.0.20348.0) | the installed MSVC, driven through `vcvars64.bat` |
| `clangd` | **absent** | LLVM 23.1.4 portable archive, extracted into the bench root, never on `PATH`, never installed |
| CMake | **absent** | CMake 4.4.4 portable zip, extracted into the bench root |
| `compile_commands.json` | **absent** everywhere | generated by CMake for the fixture; written from the repository's own documented command for MMSP; reconstructed (and labelled) for colmap |

The earlier CP3 statement "no C++ toolchain" was true of `PATH`, not of the
machine: the toolchain exists and was used here as the **ground truth**, not
as the provider. **This pilot therefore supersedes the host-toolchain line
of `docs/m9-cp3-evidence.md:48`** (which listed `g++`, `clang++`, `gcc`,
`cl` and `cmake` as missing): on this host MSVC's `cl.exe` and the Windows
SDK are installed, and only `cmake` and `clangd` really are absent. The
frozen CP1-CP3 evidence is not edited; this paragraph records the
correction where it is read.

Evidence: `docs/pilots/clangd/raw/env/environment.json`.

**Nothing was installed system-wide**, no package manager ran, and no build
or dependency script of a test repository was executed. colmap's CMake
`FetchContent` (PoseLib, faiss, ONNX) would have downloaded from the
network; it was never invoked, which is why the colmap part of this pilot
uses a labelled reconstruction instead.

---

## 4. The fixture and its ground truth

`docs/pilots/clangd/fixture/` is a small CMake C++17 project whose cases
were written so that **the compiler itself states the ground truth**:

| Case | Where | Ground truth, from the compiler |
|---|---|---|
| E1, same name, different entity | `include/geom/point.h` | `double norm(const Point& p)` (free) and `double Point::norm() const` (method) share a name |
| E1, overloads | `include/geom/overload.h` | `scale(const Point&, double)` and `scale(const Point&, int)` |
| E1, same name in another namespace | `include/geom/overload.h` | `other::scale` |
| E1, template | `include/geom/overload.h` | `clampValue` |
| E7, namespace-scope state | `include/geom/point.h` | `geom::kBudget`, read from **two** translation units |
| E7, overloaded member | `include/geom/point.h` | `Point::budget()` const and non-const |
| E12, construction | `src/use_widget.cc:22` (assert at `:23`) | `static_assert(std::is_same_v<decltype(w), ui::Widget>)` — **compiles**, so `w` is an object |
| E12, most vexing parse | `src/use_widget.cc:28` (assert at `:29`) | `static_assert(std::is_function_v<...decltype(w2)...>)` — **compiles**, so `w2` is a *function* |
| Overload ambiguity | `src/ambiguous.cc` | excluded from the build; compiled alone it gives `error C2668: 'amb::pick': ambiguous call to overloaded function` with both candidates named |

The two `static_assert`s are the point of the fixture: they make the E12
distinction a *compiler-verified* fact rather than an opinion.

The fixture's compilation database is **generated by CMake**
(`-DCMAKE_EXPORT_COMPILE_COMMANDS=ON`, NMake generator inside the MSVC
environment). The whole fixture builds, and the program it builds runs with
exit code 0 (`raw/env/fixture-cmake-build.txt`,
`raw/env/fixture-run.txt`); the E12 translation unit compiles with no
diagnostics (`raw/env/fixture-e12-compiler-verdict.txt`) and the ambiguity
file fails with `C2668` as intended
(`raw/env/fixture-ambiguity-compiler-verdict.txt`).

clangd consumes it as intended: `Loaded compilation database from
<fixture>/build/compile_commands.json`, `Compile command from CDB is:
...cl.exe --driver-mode=cl ... -I<fixture>/include ... -std:c++17`,
`All checks completed, 0 errors`
(`docs/pilots/clangd/raw/env/clangd-check-fixture.txt`).

---

## 5. The measurement method

`docs/pilots/clangd/method/probe.ps1` drives the **shipped CLI**
(`riltra semantic query --lsp-server <clangd>`) and records, per probe: the
click position (found by a marker string, never a hard-coded line), the
answer, the elapsed time, the server's peak working set, and the **source
text of every reported span** so each answer is checked against the bytes
rather than against a file name. An expectation holds only when the
expected text sits on the **reported line** — a wrong answer inside the
right file does not pass.

**Every probe is asked twice.** The first answer can be the declaration
while clangd's project index is still being built (§7.1); the second is the
stable one. Both answers are recorded in full (`first_answer`,
`second_answer`) and `stable` says whether they agreed, so the effect is
visible in the probe data itself and not only in the dedicated experiment.
(In the archived run no probe flipped — the probes ran after the index was
already there; the flips are in `raw/readiness.json`.)

The probe tables are `method/probes.json` (fixture + MMSP, 17 probes) and
`method/probes-colmap.json` (5 probes). The raw results are
`raw/probes-shipped.json` and `raw/probes-colmap.json`.

Every answer is reported with the contract's own honesty fields:
`precision: "Heuristic"`, `verification: "Unverified"`, `symbol_uri: null`,
`roles` 1 for a definition and 0 for a reference. The adapter never
promotes an LSP answer to a verified fact, and this pilot found no way to
make it do so accidentally.

---

## 6. The three gaps

### 6.1 E1 — same-named functions: **CLOSED at query time**

| Probe | Click | Answer | Correct? |
|---|---|---|---|
| free function vs method | `src/use_point.cc:9:27` (`geom::norm`) | `src/point.cc:12:8` = `norm` | **yes** — the free function, not `Point::norm` |
| method | `src/use_point.cc:10:25` (`p.norm`) | `src/point.cc:6:15` = `norm` | **yes** — the method, in a different file and line |
| overload, `double` | `src/use_overload.cc:9:20` | `src/overload.cc:6:8` | **yes** |
| overload, `int` | `src/use_overload.cc:10:17` | `src/overload.cc:8:8` | **yes** — the other overload, one line away |
| same name, other namespace | `src/use_overload.cc:11:21` | `src/overload.cc:14:8` | **yes** — not merged with `geom::scale` |
| template instantiation | `src/use_overload.cc:12:20` | `include/geom/overload.h:14:3` | **yes** |

Six of six. The two same-named entities (`geom::norm` and `Point::norm`)
resolve to **different lines in the same file** (`src/point.cc:12` against
`src/point.cc:6`), which is exactly the identity the syntax index cannot
express.

On real MMSP code, with the database built from the repository's own
documented command:

| Probe | Answer |
|---|---|
| `typeSize` at `tools/metrics.cpp:120` | `tools/metrics.cpp:34:5` = `typeSize` |
| `loadPLY` at `tools/metrics.cpp:352` | `tools/metrics.cpp:58:6` = `loadPLY` |
| `NNIndex` at `tools/metrics.cpp:490` | `tools/metrics.cpp:231:8` = `NNIndex` |
| `queryKN` at `tools/metrics.cpp:499` | `tools/metrics.cpp:263:8` = `queryKN` |

**E1 is answered.** With **no change to Riltra**.

### 6.2 E7 — variable and member references: **CLOSED at query time**

| Probe | Click | Answer |
|---|---|---|
| `geom::kBudget` | `src/use_point.cc:11:9` | **6** references: `src/use_point.cc:11`, `include/geom/point.h:25` (declaration), `src/point.cc:8` and `src/point.cc:10` (the two member functions that read it), `src/point.cc:14` (the definition), and **`src/use_state.cc:11` — a different translation unit** |
| `Point::budget()` (non-const) | `src/use_state.cc:10:5` | **3** references: the call, the declaration `include/geom/point.h:18`, the definition `src/point.cc:10` — and *not* the const overload, which is a different symbol |
| `typeSize` (MMSP) | `tools/metrics.cpp:120:21` | **4** references: the definition and the three call sites |
| `options_` (colmap) | `colmap/src/colmap/ui/dense_reconstruction_widget.cc:399:17` | `colmap/src/colmap/ui/dense_reconstruction_widget.h:75:18` = `options_` |

The cross-translation-unit reference is the decisive one: a member read from
a second TU is found, and the const/non-const overloads stay apart.

**E7 is answered at query time.** With **no change to Riltra**.

### 6.3 E12 — declaration versus construction: **NOT CLOSED**

The fixture's two `static_assert`s make the distinction compiler-verified.
clangd's answers do not carry it:

| Probe | Click | Answer | What it means |
|---|---|---|---|
| construction | `src/use_widget.cc:22:14` (`w`) | `src/use_widget.cc:22:14` = `w` | the variable's own declaration |
| most vexing parse | `src/use_widget.cc:28:14` (`w2`) | `src/use_widget.cc:28:14` = `w2` | the declaration's own declaration |

**Both answers have the same shape.** A local object and a local *function
declaration* are indistinguishable in a `textDocument/definition` answer.
The reference list does not separate them either: the constructor's
references contain the construction site `src/use_widget.cc:22` **and** the
most-vexing-parse line `src/use_widget.cc:28` (`exclude_leaked: true` in
`raw/probes-shipped.json`), because clangd reports a mention of the class
name as a mention of its constructor.

On the colmap sources, with the **reconstructed** database (§10), the same
holds: the constructor `PatchMatchController(const PatchMatchOptions&, ...)`
(`colmap/src/colmap/mvs/patch_match.h:131`) has exactly **one** reference —
its own declaration — while the construction at
`colmap/src/colmap/exe/mvs.cc:271` is not among them.

Two independent reasons, therefore:

1. **The provider does not distinguish them** in the answers the adapter
   requests.
2. **The contract cannot express them.** `semantic::Roles` has
   definition/reference/import/read/write/generated/test/forward_declaration
   and nothing else; `Evidence.symbol` is `None` for every LSP answer; the
   adapter implements `textDocument/definition` and `textDocument/references`
   and **not** `textDocument/hover`, which is where clangd would say
   "function" or "variable".

Closing E12 therefore needs **both** a contract change (a symbol kind) and
an adapter change (a request that carries the kind — hover is the natural
one). clangd itself needs no change: it already knows the difference, and
the fixture's `static_assert`s prove the compiler agrees. The pilot does not
make either change; it measures the need.

### 6.4 The overload ambiguity: clangd admits it, and the contract can carry it

`amb::pick('c')` is ambiguous; the compiler says `C2668` and names both
candidates. clangd's definition answer returns **both**:
`src/overload.cc:20:6` and `src/overload.cc:22:6`. A provider that returns
two candidates instead of silently picking one is exactly what a semantic
layer should do, and the contract's `Evidence` list carries both.

This is the strongest argument for the `REFERENCES`/`POTENTIAL_CALL`
separation that already exists: a provider that reports *candidates* must
never be collapsed into one edge.

---

## 7. What the pilot found about the shipped path

### 7.1 The answer depends on the project index, not only on the source

Two experiments, both archived.

**Coverage.** The compilation database decides whether the *definition* of
`geom::norm` is reachable at all. The database was reduced by exactly one
entry at a time, and the same question was asked four times per variant
(`raw/index-coverage.json`):

| the database | answer (4 runs) |
|---|---|
| complete | the declaration, then the definition ×3 |
| without `src/point.cc` (the definition's translation unit) | **the declaration ×4** |
| without `src/use_point.cc` (the *queried* translation unit) | the declaration, then the definition ×3 |

So the answer is complete only when the database covers the translation unit
that **holds the definition**; the queried file's own entry is not what
decides. (The first run of the other two variants is the readiness effect
below, not a coverage effect: it is the same declaration-then-definition
pattern the complete database shows.)

**Readiness.** The project index is built in the background, and the first
question asked after the index has been dropped can be answered from the
current translation unit alone — which yields the *declaration*. The index
cache is written to `.cache/clangd/index/` in the **project root** (the
parent of the directory that holds the compilation database: `<fixture>/.cache`,
`<mmsp>/.cache`, `<mmsp>/colmap>/.cache` — three cases, one rule). Five
cycles of "drop the index, ask, ask again", with three control asks that
drop nothing (`raw/readiness.json`):

| cycle | first answer after the drop | second answer |
|---|---|---|
| 1 | `src/point.cc` | `src/point.cc` |
| 2 | `include/geom/point.h` | `src/point.cc` |
| 3 | `src/point.cc` | `src/point.cc` |
| 4 | `src/point.cc` | `src/point.cc` |
| 5 | `include/geom/point.h` | `src/point.cc` |
| control ×3 (nothing dropped) | `src/point.cc` | — |

Two of the five first answers were the declaration; the second answer was
always the definition, and the control never produced a declaration. The
same question on the same bytes therefore has two different answers, and
which one comes out depends on how long the index has been building. The
index is cached, so a fresh checkout and a warm one can answer differently.

### 7.2 An empty answer does not say why it is empty

With the compilation database moved aside, clangd cannot resolve anything,
and the CLI answers:

```json
{"definitions":[],"references":[],"provider":"lsp-subprocess",
 "server_diagnostics":["notification: textDocument/publishDiagnostics"],
 "fallback":null}
```

**Empty, with `fallback: null`.** A caller cannot tell "there is no such
symbol" from "the server could not read the project", and the empty answer
cost **10.18 s** (the adapter's bounded retry: 6 attempts, 2 s apart). The
one piece of information that would have explained it — clangd's own
diagnostics about the file — is **discarded by the adapter**
(`lsp.rs:493` keeps the notification's name, not its payload).

What the payload says was captured with the protocol itself
(`method/capture-diagnostics.cjs`, a harness-only client that keeps the
notification the adapter drops). Without the compilation database
(`raw/env/lsp-diagnostics-without-database.json`):

```json
{"severity": 1, "code": "pp_file_not_found",
 "message": "'geom/point.h' file not found", "source": "clang"}
```

and with it (`raw/env/lsp-diagnostics-with-database.json`) the diagnostics
array is **empty**. So the explanation exists, and it is an **error**
(`severity: 1`) — it simply never reaches the caller.

### 7.3 The evidence can be stale against the bytes it names

Three cycles of "warm the index up, delete the definition from `point.cc`,
ask twice", plus two control cycles that drop the index before the edit
(`raw/staleness.json`):

| | answer | the bytes it names, as they are on disk now |
|---|---|---|
| before the edit | `src/point.cc` | `norm` — the definition ✓ |
| after the edit, first query | `src/point.cc` | **`niti`** — the tail of the comment that replaced the definition |
| after the edit, later queries | `include/geom/point.h` | `norm` — the declaration, which is now correct |
| control: the index dropped before the edit | `include/geom/point.h` | `norm` ✓ — the correct answer, immediately |

The answer keeps naming the old line, and the bytes that line now holds are
part of a comment. The control identifies the cause: with the index present
the server answers from it, and it has not noticed the change; with the
index dropped the server reads the file again and answers correctly.

The part Riltra owns is worse than the cause:
`locations_to_evidence` (`lsp.rs:663`) computes the digest from the file
**as it is on disk now** and attaches the span the server believes. The fact
therefore carries a current digest next to a span that the current bytes do
not contain. The contract does mark the fact `Unverified`, so the contract
is not violated — but a consumer that treats the span as a fact about the
current bytes is wrong, and nothing in the answer says so.

### 7.4 Missing clangd, missing server, missing everything

With a path that does not exist, the CLI answers with the contract's
fallback payload and exit code 0:

```json
{"provider":"lsp-subprocess","status":"fallback",
 "reason":"language server not found: '...no-such-clangd.exe'",
 "syntax_fallback":true}
```

This path is correct and honest. clangd remains optional.

### 7.5 The MCP surface does not reach the LSP

`crates/cli/src/mcp.rs` has no `semantic`/`lsp` tool, so the live LSP query
is **CLI-only** today. The MCP server cannot answer a C++ definition
question.

---

## 8. The minimum implementation plan

The measurements justify **one small adapter change and one documentation
change**. They do **not** justify a schema change, a new database, a
daemon, a new dependency, or an LSP ingest path.

### 8.1 The one code change: keep the server's diagnostics

`crates/cli/src/lsp.rs`, `note_notification`: record the payload of
`textDocument/publishDiagnostics` (severity, message, and the line it
refers to) instead of the notification's name.

* Why: §7.2. An empty answer is the normal outcome of a missing or wrong
  compilation database, and today it is indistinguishable from "no such
  symbol" — after 10.2 s of retrying. The payload that would explain it
  exists, and in this case it is an **error**: `severity: 1`,
  `pp_file_not_found`, `'geom/point.h' file not found`
  (`raw/env/lsp-diagnostics-without-database.json`).
* Size: about 20 lines in one function, plus one field on the answer. No
  schema, no dependency, no behaviour change for a healthy server.
* Test: the mock server in `crates/cli/examples/mock_lsp.rs` has **no**
  `publishDiagnostics` mode today (its modes are `success`, `crash`,
  `hang`, `malformed`, `pollution`, `no-refs`, `stderr-flood`, `progress`),
  so the test needs a new mode that publishes a diagnostic, and one that
  publishes a *warning* to prove a warning is not turned into a failure.

**Deliberately not proposed:** turning an empty answer into a `fallback`.
The pilot measured that clangd's payload in the no-database case is an
error, but it did **not** measure what a *file with an unrelated error*
does to the answer, and a file that has an error somewhere else would then
lose its answer for no reason. The first, safe half of the change (keep the
payload) is what the measurements justify; the second half (turn an empty
answer into a fallback) needs its own experiment and the owner's agreement.

### 8.2 The documentation change: the operational contract

State, in the user-facing docs, what the pilot measured:

1. The server needs a **compilation database**: at the repository root, in an
   ancestor, or in a `build/` subdirectory (clangd searches all three; the
   design gate records the same rule at `docs/m9-design-gate.md:266-274`,
   and the pilot's own fixture is found through the `build/` case). Without
   one the answer is empty, and the reason is in the server's diagnostics
   that the adapter currently drops (§7.2, §8.1).
2. The answer is **complete only if the database covers the translation
   unit that holds the definition** (§7.1). The index is cached next to the
   database, so a fresh checkout and a warm one can answer differently.
3. An LSP answer is a **hint, not a fact**: it is `Heuristic` /
   `Unverified`, its span may not match the bytes it names (§7.3), and it
   is never persisted.
4. A **reference is not a call**. If LSP facts are ever persisted, they
   belong in `REFERENCES` edges, never `CALLS`, and a reference that is
   only a mention of a type must not become an edge.

### 8.3 Deliberately **not** in the plan

| Not doing | Why |
|---|---|
| Persisting LSP facts into the index | The pilot closes E1/E7 at **query time**; an ingest path is a new pipeline with its own freshness and provenance rules. Owner decision, not a pilot result. |
| A symbol-kind field in the semantic contract | It is needed to close E12, and it is a contract change to every provider. The pilot measured the *need*; the design is the owner's. |
| `--lsp-arg` passthrough for `--compile-commands-dir` / `--background-index` | The pilot shows the shipped path needs neither: the database is found by the ancestor walk and the background index is on by default. Would only matter for a database outside the repository. |
| A `clangd` dependency, a bundled tool, a daemon | clangd stays optional and external. |
| A colmap/MMSP-specific configuration in Riltra | The provider is generic; the *project's* build configuration is the project's business. |
| Any claim about CUDA | The colmap site that would exercise the template-argument case sits behind `#if defined(COLMAP_CUDA_ENABLED) || defined(COLMAP_HIP_ENABLED)` in `colmap/src/colmap/ui/dense_reconstruction_widget.cc:405-406`. The reconstruction does not define those macros, so that region is not analysed (§10), and no CUDA capability is claimed. |

---

## 9. MMSP with the repository's own build command

MMSP `README.md:62` states the build of its only C++ file:

```
cl /O2 /std:c++17 tools\metrics.cpp   # -> metrics.exe
```

`method/mmsp-cdb.ps1` turns that into a compilation database — the same
command with `cl` resolved to its installed path (found through `vswhere`,
not hard-coded) and `/nologo` added — and then **runs the database's own
command** and reports the compiler's verdict: `compiler exit code: 0`,
`object produced: True`, `exe produced in tmp: True`
(`raw/env/mmsp-cdb-proof.txt`). The database is therefore the real build
command, not a reconstruction, and the script leaves the clone with nothing
but the untracked database file itself.

clangd's own report confirms that it uses that database:
`Loaded compilation database from <clone>/compile_commands.json`, and
`Compile command from CDB is: ...cl.exe --driver-mode=cl /nologo /O2
/std:c++17 tools\metrics.cpp` (`raw/env/clangd-check-mmsp.txt`).

`tools/metrics.cpp` is C++17, STL only, and the whole file is one
translation unit. clangd answers every probe on it (§6.1, §6.2) with the
documented database.

## 10. colmap: what a reconstruction can and cannot show

colmap is vendored inside MMSP, and the E1/E7/E12 cases CP3 recorded live
in it (`colmap/src/colmap/exe/mvs.cc:271`, the
`mvs::PatchMatchController` construction; and
`colmap/src/colmap/ui/dense_reconstruction_widget.cc:399`, the
`options_->patch_match_stereo` member read).

The **real** build configuration of colmap is not obtainable here: its
optional dependencies (Eigen, glog, Ceres, Boost, FLANN, FreeImage) are not
installed, its `add_compile_definitions` calls only fire when those
dependencies are found, and its CMake uses `FetchContent`, so configuring
it would download from the network. `method/colmap-cdb.ps1` therefore
builds a **reconstruction**: the include root and the C++ standard, both
taken from colmap's own CMakeLists, and nothing else. The script says so in
its own output, and the file says so in its own name.

Measured with the reconstruction:

| Probe | Answer |
|---|---|
| the type of the constructed object | `colmap/src/colmap/mvs/patch_match.h:129` = `PatchMatchController` |
| the member `options_` | `colmap/src/colmap/ui/dense_reconstruction_widget.h:75` = `options_` |
| the class inside `std::make_shared<mvs::PatchMatchController>(...)` | `colmap/src/colmap/mvs/patch_match.h:129` = `PatchMatchController` |
| references to the class `PatchMatchController` | **0** (11.5 s of retrying) |
| references to the constructor | **1** — its own declaration; the construction at `mvs.cc:271` is not among them |

So: **definitions of project types survive an incomplete configuration;
the reference lists do not.** The absence of references is a property of the
reconstruction (the other translation units do not parse without their
dependencies), not a property of clangd. This is the concrete form of the
brief's warning: a fallback flag set is not the build configuration, and
the difference is visible in the answers — the fixture, whose database is
real, answers **6** references for a cross-translation-unit variable and
**3** for a member, while colmap's reconstruction answers **0** for its
class.

The template-argument construction is worth one line of its own: clicking
the `mvs::PatchMatchController` inside `std::make_shared<...>` **does**
resolve to the class, so the *type* is found even where the construction
is not. What is missing is the identification of the construction as a
construction — the same E12 gap as in the fixture.

The colmap numbers also cost the most: 2.64–11.54 s per query and a peak
working set of **315–350 MiB**, against 1.63–2.43 s and 166–184 MiB for the
fixture.

---

## 11. Measurements

Every probe is asked twice (§5); the table reports the **second** ask, and
the first ask's median is given beside it, because the first ask is the one
that can race the index.

| Measure | Fixture (12 probes) | MMSP (5 probes) | colmap, reconstruction (5 probes) |
|---|---|---|---|
| latency, second ask: min – median – max | 1.49 – 1.63 – 1.73 s | 2.17 – 2.39 – 2.55 s | 2.67 – 2.93 – 11.46 s |
| latency, first ask: median | 1.65 s | 2.38 s | 2.92 s |
| server peak working set, second ask: min – median – max | 168.4 – 175.5 – 190.6 MiB | 240.2 – 245.9 – 247.3 MiB | 314.1 – 324.9 – 350.4 MiB |
| server peak working set, first ask: median | 175.5 MiB | 242.3 MiB | 323.2 MiB |
| probes whose two answers disagreed | 0 of 12 | 0 of 5 | 0 of 5 |

Two more timings, from the degradation experiments:

| Case | Time | Answer |
|---|---|---|
| no compilation database | **10.18 s** | empty, `fallback: null` |
| clangd not installed | 0.05 s | the fallback payload with the reason, exit code 0 |

Raw values: `raw/probes-shipped.json`, `raw/probes-colmap.json`,
`raw/robustness.json`. The peak is the server process's own
`PeakWorkingSet64`, sampled while the CLI was running and restricted to
the CLI's own children, so a stray `clangd` on the machine cannot
contribute to it.

The latency is dominated by the server: the CLI's own overhead is
milliseconds, and the adapter waits for the server's first
`publishDiagnostics` when the server advertises the capability before
asking. The cost is **per question**, because `semantic_query_lsp` spawns a
fresh server process, parses the file, asks one question and shuts the
server down. A project with a warm index still costs about 1.5 s and about
170 MiB per question.

That cost is the reason the plan does not propose an LSP ingest path: doing
it for a whole repository would mean a server per question, and the pilot
gives no evidence that the answers would be worth that.

The MMSP numbers are higher than the fixture's because the MMSP database is
the **real** documented command: clangd really drives `cl.exe` with the
MSVC include paths, and it parses the whole standard library headers with
them. The fixture's database is CMake's, and the fixture's own **headers**
include nothing at all (only its `.cc` files include a standard header,
`<type_traits>`).

---

## 12. Evidence index

| File | What it holds |
|---|---|
| `docs/pilots/clangd/method/probe.ps1` | the probe driver: click by marker, ask twice, record the answer, the elapsed time, the server's peak memory, and the source text of every span |
| `docs/pilots/clangd/method/probes.json` | the 17 fixture and MMSP probes |
| `docs/pilots/clangd/method/probes-colmap.json` | the 5 colmap probes |
| `docs/pilots/clangd/method/msvc.ps1` | finds the Visual Studio toolchain through `vswhere` (no hard-coded path) |
| `docs/pilots/clangd/method/mmsp-cdb.ps1` | the MMSP database from the repository's documented command, plus the proof that the command compiles |
| `docs/pilots/clangd/method/colmap-cdb.ps1` | the **reconstructed** colmap database, labelled as such |
| `docs/pilots/clangd/method/collect.ps1` | the environment facts, the compiler's verdicts, and what clangd itself reports (with and without a database) |
| `docs/pilots/clangd/method/robustness.ps1` | clangd absent, no database, source changed, index cold |
| `docs/pilots/clangd/method/staleness.ps1` | the deleted-definition experiment, three cycles |
| `docs/pilots/clangd/method/readiness.ps1` | the index-readiness race, with a control |
| `docs/pilots/clangd/method/capture-diagnostics.cjs` | a protocol-only client that keeps the `publishDiagnostics` payload the adapter drops |
| `docs/pilots/clangd/method/coldstart.ps1` | the cold start with both index caches cleared |
| `docs/pilots/clangd/method/index-coverage.ps1` | the database-coverage experiment, with a control |
| `docs/pilots/clangd/fixture/` | the C++ fixture and its CMake project |
| `docs/pilots/clangd/raw/` | the redacted raw output of every run |

---

## 13. What the owner must decide

1. **E12** stays open until the semantic contract can carry a symbol kind
   **and** the adapter asks a question that carries it. The pilot says
   clangd is not the blocker: it already knows the difference, and the
   compiler agrees with it (§4).
2. **The two honesty defects** (§7.2 empty answer, §7.3 stale span) are in
   the shipped path. The pilot proposes one small adapter change
   (§8.1); the contract's `Unverified` marking already keeps them from
   being *false* facts, but not from being *misleading* ones.
3. **The index-coverage and index-readiness dependency** (§7.1) makes a
   live LSP answer incomplete in a way the answer does not disclose, and
   makes the same question have two answers. A minimum answer could carry
   a coverage flag ("the definition's translation unit is not in the
   database") and a "the index may not be ready" flag, but that is a
   contract decision, and it is the owner's.
4. **colmap** can only be measured properly on a machine that can configure
   it. The pilot's colmap numbers are the *reconstruction's* numbers and
   must not be quoted as colmap's.
5. **No CUDA claim** is made anywhere in this report. The CUDA-guarded
   colmap region is not analysed: the reconstruction does not define
   `COLMAP_CUDA_ENABLED` or `COLMAP_HIP_ENABLED`, so the code behind that
   guard is not even parsed. The template-argument probe was run against
   the *unguarded* part of the same file, and its answer says nothing
   about CUDA.

**STOP — awaiting an independent audit and the owner's GO. No production
integration was started.**
