# M9 — clangd MCP integration design (proposal)

**Status: PROPOSAL ONLY. This document authorizes no implementation.** It
designs the minimum integration that lets an MCP client (OpenCode) ask a
registered repository for a semantic *definition* or *references* answer
produced by a language server, reusing the code the CLI already ships. No
production code, schema, dependency, MCP tool, or frozen evidence is changed
by this document.

- Date: 2026-10-21
- Branch: `m9-multirepo-release-candidate`
- Base: `ee80646722031e8b67879d3ad970ceb78370703c` (the privacy-corrections
  commit; `0b453bf` and `22bc7b7` before it)
- Scope: **design only**. The implementation described in §9 needs a separate
  OWNER GO. It **proposes to supersede D-M9-2**
  (`docs/m9-design-gate.md:806`) for the narrow scope in §9 — only that OWNER
  GO can supersede it, not this document (OD-6, §10).
- Out of scope, explicitly: MCP/graph integration beyond the single tool,
  E12 (construction vs declaration), LSP ingestion into the graph or the
  database, CP4, merge, release.

## 1. The existing paths this design uses

### 1.1 The path that already answers a live language-server question

```
CLI   riltra semantic query --repo R --path P --line L --col C [--refs] --lsp-server <abs>
        main.rs:928 cmd_semantic_query
          -> index.rs:2439 semantic_query_lsp(root, span, exe)
               canonical_repo_root                (index.rs:2444)
               semantic::validate_doc_path         (semantic.rs:319)  relative, no `..`, canonical containment
               lsp::ServerSpec::new                (lsp.rs:29)       absolute path only, 30 s, 8 MiB, 10k messages
               lsp::spawn -> Session                (lsp.rs:286)      stdio, framed JSON-RPC, kill+reap
               Session::definition / references     (lsp.rs:973/1006)
               locations_to_evidence               (lsp.rs:885)      digest + span
               mark_changed_sources                 (lsp.rs:802)      the Stale marking (called at lsp.rs:995/1026)
               lsp_hit_from_fact                    (index.rs:1009)
               lsp_answer_verification              (index.rs:1029)   the weakest fact decides
               lsp_fallback_reason                  (index.rs:1045)   the server's diagnostics ride along
          -> the document is assembled **inline in main.rs:973-986**
```

Two properties of that path matter for this design:

1. **It never touches the index or the knowledge store.** `semantic_query_lsp`
   takes `(root, span, exe)` — no `home`, no database handle. The LSP answer
   is ephemeral evidence that is returned and discarded. This is why the
   "MCP reads never create or migrate a store" rule is satisfied here by
   construction, not by a guard.
2. **It needs no index.** The repository must be *registered* (so the root can
   be resolved), but it does not need to be indexed.

### 1.2 The MCP surface today

`crates/cli/src/mcp.rs` serves **eleven** read-only tools
(`mcp.rs:185-368`): `riltra_repo_list`, `riltra_repo_status`,
`riltra_code_search`, `riltra_code_callers`, `riltra_code_callees`,
`riltra_code_trace`, `riltra_code_impact`, `riltra_knowledge_search`,
`riltra_knowledge_show`, `riltra_knowledge_related`, `riltra_health`.
Every tool reuses a CLI formatter, classifies refusals as
`{"kind": ..., "message": ...}` (`mcp.rs:401`), and runs its blocking work on
the tokio blocking pool (`mcp.rs:382`). **There is no semantic tool** — the
pilot recorded this as the last open gap
(`docs/m9-clangd-pilot.md:417-421`, §7.5: "the MCP surface does not reach
the LSP").

Three things about that surface constrain this design, and they are easy to
miss:

- The health document is **hand-written** and carries a hard-coded tool list
  and a server-level `read_only: true` (`mcp.rs:1240-1271`). A twelfth tool
  changes that document (OD-7).
- The existing MCP test asserts `readOnlyHint == true`,
  `openWorldHint == false` and `idempotentHint == true` **for every
  advertised tool** (`tests/mcp.rs:220-222`), and the health test asserts
  `read_only == true` and a tool count of 11 (`tests/mcp.rs:458`, `:463`).
- The MCP transport re-serializes the document: `doc()` parses the CLI text
  into a `serde_json::Value` and `CallToolResult::structured` renders it
  (`mcp.rs:390-398`). `serde_json` is used without the `preserve_order`
  feature (`crates/cli/Cargo.toml:11`), so the **MCP result's key order is
  the transport's, not the formatter's** — the frozen CP7-A transcript shows
  alphabetically sorted keys (`docs/pilots/cp7a/reference/mcp-transcript.json:688`).

### 1.3 What is reused unchanged

| Need | Existing piece | Where |
|---|---|---|
| Repository resolution + "moved or retargeted" refusal | `mcp::repo_root` | `mcp.rs:746-778` |
| Relative-path validation, symlink/junction escape rejection | `semantic::validate_doc_path` | `semantic.rs:319-350` |
| Provider launch, timeouts, message caps, kill+reap | `lsp::ServerSpec`, `lsp::spawn` | `lsp.rs:18-44`, `lsp.rs:286` |
| Diagnostics capture, sanitizing, bounding | the diagnostics drain + `bounded_server_text` | `lsp.rs:649`, `lsp.rs:1362` |
| Location containment (a location outside the root is refused) | the adapter's canonical-root check | `lsp.rs:941-949` |
| Evidence shape, verification states | `semantic::Evidence`, `Precision`, `Verification` | `semantic.rs:14-194` |
| Fallback payload | `semantic::fallback_report` | `semantic.rs:371-376` |
| The provider registry entry | `lsp-subprocess` (definitions + references) | `semantic.rs:208-221` |
| The mock language server | `examples/mock_lsp.rs` | `crates/cli/examples/mock_lsp.rs` |
| The MCP test harness (real process, JSON-RPC over stdio) | `crates/cli/tests/mcp.rs` | `tests/mcp.rs:36-151` |

### 1.4 The one structural gap this design closes

The CLI's semantic document is assembled **inline in `main.rs:973-986`**,
while every other CLI/MCP-shared document lives in `response.rs` (whose
header states the discipline: "One code path builds each response shape …
This module is extraction-only: every byte matches the historical inline
formatting in `main.rs`", `response.rs:1-5`). To give the MCP tool the same
document without duplicating the format string, the formatter is
**extracted** to `response.rs` and both callers use it. That is the whole of
the "no duplicated logic" work: one extraction, no new abstraction. A unit
test pins the extracted formatter's bytes against the historical inline
format, including the trailing newline, so the extraction cannot drift.

## 2. The tool contract

### 2.1 One new tool, not an extension of an existing one

| Option | Verdict | Why |
|---|---|---|
| Extend `riltra_code_search` | **rejected** | changes an existing tool's input schema (client-visible), and mixes the syntax index provider with the live LSP provider in one result shape |
| Extend `riltra_code_callers` / `riltra_code_callees` | **rejected** | those tools answer *graph edges*; a *references* answer is a location list, not a call edge. Merging them would put `REFERENCES` and `CALLS` in one namespace, and it would change two existing tools |
| Extend `riltra_health` | **rejected** | health reports state; it does not answer questions |
| **One new tool** | **chosen** | adds capability without changing the eleven tools' behaviour; a new schema is additive |

The tool is **not** called "read-only" in this document: §4 shows that the
honest annotation for a provider-backed tool is not `read_only_hint = true`.

### 2.2 Name and input

Name: **`riltra_code_semantic`** (the candidate name from the brief; it sits
beside `riltra_code_*` and says what it is).

```jsonc
{
  "repo":      "mmsp",                                  // registered repository name
  "path":      "colmap/src/colmap/mvs/patch_match.h",   // repository-relative, '/' separators
  "line":      129,                                     // 1-based
  "col":       8,                                       // 1-based
  "operation": "definition"                             // "definition" | "references"
}
```

- `repo`, `path`, `line`, `col` use the names of the existing graph tools
  (`mcp.rs:50-66`), so a client that already calls `riltra_code_callers`
  needs no new mental model. **The column unit is not the same, though** —
  see below.
- `operation` is a real enum (`#[derive(Deserialize, JsonSchema)]` with
  `#[serde(rename_all = "lowercase")]`), so the advertised JSON Schema
  carries the two allowed values instead of a free-form string. An explicit
  enum rather than a boolean: an LLM client reads `"references"` as "where is
  this symbol used", and the two answers are genuinely different (§5.1).
- **Column unit.** The LSP path sends the position in the encoding the
  server negotiated — `utf-16` unless the server says otherwise
  (`lsp.rs:51-53`). `lsp.rs:982`/`:1016` send `col-1` as the LSP `character`
  **as it is**: the client counts in the negotiated unit, and the adapter
  converts the **returned** positions with `lsp_pos_to_bytes`
  (`lsp.rs:165-191`, used at `lsp.rs:931-932`). The
  **graph** tools use *byte* columns (`index.rs:1240-1256`,
  `syntax.rs:145-157`). The same `col` number therefore addresses a
  different character in `riltra_code_semantic` than in
  `riltra_code_callers` on any line with non-ASCII text. The `col` field's
  schema description says so, and a non-ASCII acceptance case covers it
  (§6.2). The MCP/CLI parity is unaffected: the CLI's `--lsp-server` path
  uses the same unit.
- `line = 0` or `col = 0` is refused with `invalid_input`, matching the
  graph tools (`mcp.rs:659-673`). This is a **deliberate divergence from the
  CLI**: `riltra semantic query` accepts 0 and queries line/character 0
  (`main.rs:939-952`, `lsp.rs:982` uses `saturating_sub(1)`). The tool
  refuses the meaningless position instead of answering about it.
- **No field names the language server, its arguments, or any path outside
  the repository.** See §3.1.
- The schema is derived from a struct with `deny_unknown_fields`, as the
  other eleven do (`mcp.rs:26-181`); unknown fields are refused, not ignored
  (the G4 rule, `tests/mcp.rs:527`).

### 2.3 The output document

**The MCP result carries the same document as the CLI's `semantic query
--lsp-server`** — the same members, the same values, produced by one shared
formatter (`response::semantic_query_json`, extracted from `main.rs`). The
transport then re-serializes it, so the MCP result's **key order is the
transport's** (sorted), not the CLI's insertion order, and the CLI's trailing
newline is not part of the MCP payload. Parity is therefore asserted on the
parsed value (the pattern every MCP test already uses, `tests/mcp.rs:938`),
and byte-identity is asserted separately in a unit test on the extracted
formatter (§6.2, test 1).

```json
{
  "repo": "mmsp",
  "definitions": [
    {
      "symbol_uri": null,
      "display_name": null,
      "path": "colmap/src/colmap/mvs/patch_match.h",
      "span": {"start_line": 128, "start_col": 8, "end_line": 128, "end_col": 24,
               "byte_start": 4821, "byte_end": 4837},
      "roles": 1,
      "precision": "Heuristic",
      "verification": "Unverified"
    }
  ],
  "references": [],
  "provider": "lsp-subprocess",
  "provider_version": "clangd version 23.1.4 (...)",
  "verification": "Unverified",
  "server_diagnostics": [],
  "fallback": null
}
```

Field notes (all existing contract, `semantic.rs` and `index.rs`):

- The input is **1-based**; the output spans are **0-based** display
  coordinates (`semantic.rs:52-53`), so line 129 in becomes `start_line: 128`
  out. The same convention as the graph tools.
- `symbol_uri` and `display_name` are `null` for LSP answers: an LSP answer
  carries no symbol identity, and none is fabricated
  (`semantic.rs:70-80`, `semantic.rs:173-193`).
- `roles` is `1` for a definition and `0` for a reference
  (`Roles::from_lsp_kind`, `semantic.rs:117-129`).
- `precision` is `Heuristic` and `verification` is `Unverified` — never
  `Exact`, never `Current`: no independent producer-to-source proof exists
  (`semantic.rs:184-192`, `semantic.rs:406-421`).
- `verification` at the top level is the **weakest fact in the answer**
  (`index.rs:1029-1041`): one `Stale` fact makes the whole answer `Stale`.
- `server_diagnostics` carries the provider's own diagnostics (severity,
  code, message, location), sanitized and bounded (`docs/m9-lsp-hardening.md`
  §2). It is empty when the provider said nothing — and also empty when the
  provider **failed**: on a failure the diagnostics ride inside
  `fallback.reason` instead (`index.rs:2466-2482`).
- `fallback` is `null` on success, and on a provider failure it is the
  contract's fallback payload with the reason — the server's diagnostics ride
  inside the reason (`index.rs:1045-1050`):

```json
{"provider":"lsp-subprocess","status":"fallback",
 "reason":"server error: ...; server diagnostics: ... | ...",
 "syntax_fallback":true}
```

Only the requested operation's array is filled; the other stays empty, as in
the CLI (`index.rs:2494-2501`).

### 2.4 Example exchange

Request (OpenCode → `riltra mcp`):

```json
{"jsonrpc":"2.0","id":7,"method":"tools/call",
 "params":{"name":"riltra_code_semantic",
           "arguments":{"repo":"mmsp","path":"colmap/src/colmap/mvs/patch_match.h",
                        "line":129,"col":8,"operation":"definition"}}}
```

Result (`rmcp`'s `CallToolResult::structured`, the same shape the other
eleven tools return; the key order is the transport's):

```json
{"jsonrpc":"2.0","id":7,
 "result":{"content":[{"type":"text","text":"{\"definitions\":[],\"fallback\":null,…}"}],
           "structuredContent":{"repo":"mmsp","definitions":[…],"references":[],
                                "provider":"lsp-subprocess","provider_version":"…",
                                "verification":"Unverified","server_diagnostics":[],
                                "fallback":null},
           "isError":false}}
```

### 2.5 Refusal classification

The existing classification vocabulary (`mcp.rs:471-520`, `mcp.rs:780-799`)
is reused. One kind is added (`provider_unavailable`), because it is the one
case with no CLI counterpart.

| Situation | Result |
|---|---|
| Unknown repository | MCP error `not_registered`, message `unknown repository: 'R'` (the CLI's text) |
| Registered root missing / moved / retargeted (symlink or junction) | MCP error `not_registered`, the CLI's text |
| `line` or `col` = 0 | MCP error `invalid_input` (the graph tools' rule; the CLI accepts 0 — §2.2) |
| `line` or `col` is not a number, negative, or fractional | **SDK schema refusal** - `isError: true` with "failed to deserialize parameters" (`tests/mcp.rs:617-664`); it never reaches the tool, so there is no `kind` |
| `operation` is not `definition`/`references` | **SDK schema refusal** - the operation is a deserialized enum, so an unknown value never reaches the tool (`tests/mcp.rs:617-664`); there is no `kind` |
| Path is absolute, drive-qualified, contains `..`, or escapes the repository | MCP error `invalid_input`, with `validate_doc_path`'s own text (`semantic.rs:319-350`: "empty document path", "document path not relative/portable", "non-canonical document path", "absolute document path") |
| **The named file does not exist** | MCP error `invalid_input` — but with the containment text: `paths::contains` canonicalizes the candidate and a missing file fails that check (`paths.rs:72-76`), so the message reads "document path escapes repository: '…'". The text is imprecise; it is the existing contract's behaviour, not something this design changes. Disclosed here and in the tool description. |
| The file exists but is a symlink pointing out of the repository | MCP error `invalid_input` (the canonical containment check) |
| **No provider configured** (`RILTRA_LSP_SERVER` unset, empty, or not absolute) | MCP error `provider_unavailable` — a server-configuration problem, not an answer |
| Provider missing / crashed / timed out / protocol error | **not an error**: the contract document with `fallback` set, exactly like the CLI (exit 0 there, a successful result here). Note that on a failure `server_diagnostics` is **empty** and the provider's diagnostics live inside `fallback.reason` (`index.rs:2466-2482`) |
| Provider answered but with nothing | the contract document with empty arrays, `server_diagnostics` as the provider sent them, and `fallback: null` |

The last two rows are the CLI's behaviour, kept deliberately: a provider
failure is *evidence about the provider*, and the contract document is where
that evidence belongs. §5.1 states how a caller must read it.

**The order of the checks is part of the contract**: the repository, the
position and the path are validated *before* the provider is looked up, so a
client that passes a bad path hears about the path, not about a missing
provider (§9.2).

## 3. The provider configuration and the trust boundary

### 3.1 The rule

**The MCP client never names the executable, its arguments, or anything
outside the repository.** A tool argument that carries a path is a remote
code execution primitive: the model would be choosing the program Riltra
runs. The tool schema has no such field (§2.2), and no future field may add
one.

### 3.2 The channel — options evaluated

| Channel | Trusted? | Cost | Verdict |
|---|---|---|---|
| **Environment variable `RILTRA_LSP_SERVER`** (an absolute path) | yes — set by the operator in the MCP client's launch configuration, inherited by the server process; the model inside the session cannot change it | one `env::var` read | **recommended** |
| A config file under `RILTRA_HOME` (e.g. `semantic.json`) | yes | a new file, a new parser, a new failure mode | alternative; see OD-2 |
| A field in the repository registry (`registry.json`) | yes | changes the registry format that eleven tools read; per-repository when the need is per-machine | rejected (schema churn for a machine-level need) |
| A tool argument | **no** | — | **forbidden** |
| `PATH` lookup | weak — any writable directory on `PATH` becomes executable | — | **forbidden** (`ServerSpec::new` already requires an absolute path, `lsp.rs:31-34`) |

The environment variable is the same pattern the project already uses for
operator-controlled switches (`RILTRA_HOME`, `RILTRA_LSP_DEBUG`,
`RILTRA_TEST_LSP`) and it is settable where the operator expects it:
OpenCode's local MCP server configuration has an `environment` map
(Context7: `opencode.ai/v2/docs/mcp-servers`), and `opencode mcp add --env
KEY=VALUE` exists (Context7: `opencode.ai/v2/docs/cli/commands`). The **V2**
shape matters and is easy to get wrong: servers live under
`mcp.servers.<name>`, `enabled` became the inverse `disabled`, and
`timeout` is an **object** — `{startup, catalog, execution}` — not the
single number of V1 (Context7: `opencode.ai/v2/docs/migrate-v1`):

```jsonc
{ "$schema": "https://opencode.ai/config.json",
  "mcp": { "servers": { "riltra": {
      "type": "local",
      "command": ["riltra", "mcp"],
      "environment": { "RILTRA_LSP_SERVER": "/opt/llvm/bin/clangd" } } } } }
```

**No timeout override is needed.** The default `execution` timeout is **12
hours** (Context7: `opencode.ai/v2/docs/mcp-servers`), far above the
provider's bounded worst case of under a minute (§7.2); the other two
defaults are 30 s for `startup` and 30 s for `catalog`, which this server
also fits inside. If the operator lowers it, the knob is
`mcp.servers.riltra.timeout.execution` (or the global
`mcp.timeout.execution` default) and it should stay comfortably above the
adapter's worst case — 120 s is a reasonable margin (§7.2). The flat V1
form `"timeout": 120000` is not the V2 shape and would be ignored or
rejected.

### 3.3 What is validated before anything is spawned

1. The repository name resolves in the registry; the recorded root must
   canonicalize to itself (a moved root, a symlink or a junction retarget is
   refused) — `mcp.rs:746-778`.
2. The document path is repository-relative, has no `..`, no drive letter, no
   empty component, and canonicalizes inside the root — `semantic.rs:319-350`.
   A symlink inside the repository that points outside fails the containment
   check.
3. The provider path is absolute (`lsp.rs:31-34`) — checked by the tool
   before the query, so a relative value is a configuration error
   (`provider_unavailable`) and not a query error. The spawn failure of a
   missing file is reported as the contract's fallback reason ("language
   server not found: '…'"), never as a crash.
4. No shell is used; arguments are passed as argv (`lsp.rs:16-17`).

### 3.4 Arguments: none from the client

The pilot explicitly rejected a `--lsp-arg` passthrough
(`docs/m9-clangd-pilot.md:485`). That stays rejected: a client-supplied
argument is a client-supplied code path. Riltra may pass its **own** fixed
arguments (the only candidate is `--background-index=0`, §4.3), and they are
part of the code, not of any request.

### 3.5 The limits that are inherited unchanged

30 s request timeout (`lsp.rs:39`, applied at `lsp.rs:461`), 8 MiB per
message, 10 000 messages, bounded and sanitized diagnostics, stderr drained
and sanitized, kill + bounded reap on every failure path (`lsp.rs:18-44`,
`lsp.rs:257-286`). The MCP server never writes to the child's stdout: the
adapter owns the child's streams, and the `RILTRA_LSP_DEBUG` line goes to
**stderr** (`lsp.rs:466-467`), so MCP stdout stays protocol-only (§6.2,
test 10).

## 4. The read-only strategy and clangd's cache

### 4.1 What clangd writes — and what it does not run (verified)

- **Project index cache**: "Before indexing each file, the index checks for a
  cached `*.idx` file on disk. After indexing, it writes this file. These
  files are located in `.cache/clangd/index/` **next to
  `compile_commands.json`**." (Context7: `clangd.llvm.org/design/indexing`.)
  The pilot measured the consequence: the cache lands in the **project root**
  — `<fixture>/.cache`, `<mmsp>/.cache` (`docs/m9-clangd-pilot.md:326-327`).
  With the usual `build/compile_commands.json` inside the repository, the
  write is **inside the repository**.
- **System-header cache**: "For headers with no CDB, such as the standard
  library, they are in `clangd/index` under the user's cache directory
  (`$XDG_CACHE_HOME`, `DARWIN_USER_CACHE_DIR`, or `%LocalAppData%`)."
  (Context7, same page.) A write outside the repository, on the machine.
- **Compilation database**: clangd *reads* `compile_commands.json` — the
  pilot's frozen log shows only a load ("Loaded compilation database from
  …", `docs/pilots/clangd/raw/env/clangd-check-fixture.txt:10`) and no
  write — and Riltra never generates one either (§4.5).
- **Background index**: on by default; `--background-index=0` turns it off
  (Context7: `clangd.llvm.org/troubleshooting`).
- **The compilation database is interpreted, not built — and the compiler it
  names is not executed either.** clangd "determines a virtual compiler
  command for each file … Ideally this command comes from the build system"
  (Context7: `clangd.llvm.org/design/compile-commands`), and the "tweaks
  always applied" are: "`-fsyntax-only` is added because we're just parsing,
  not compiling", "`-resource-dir=...` is added, because built-in headers
  like `<stddef.h>` must be the ones installed with clangd", "certain
  unsupported flags like `-plugin` are dropped", and bare command names are
  made absolute by looking them up on the `$PATH` (the same page).
  **Tweaks, not execution:** the pilot's own frozen log shows what clangd
  does with the entry — it *reconstructs* a command line and parses it
  in-process:
  `"Compile command from CDB is: [… ] \"…\\cl.exe\" --driver-mode=cl /nologo /TP … -c \"-resource-dir=…\" -- \"…\\use_point.cc\""`,
  then `"Parsing command..."`, then
  `"internal (cc1) args are: -cc1 -triple x86_64-pc-windows-msvc19.34.31948 -O0 -fsyntax-only …"`
  (`docs/pilots/clangd/raw/env/clangd-check-fixture.txt:11-13`; the same
  for MMSP at `clangd-check-mmsp.txt:11-13`). The reconstructed argv
  contains options `cl.exe` does not accept (`--driver-mode=cl`,
  `-resource-dir=`) and the `-cc1` invocation is clangd's **own frontend**:
  so the project's build step is never run — no object file, no link step,
  no build script — and the binary the database names is **not executed**.
  (The pilot's phrase "clangd really drives `cl.exe` with the MSVC include
  paths", `docs/m9-clangd-pilot.md:606-611`, is a figure of speech about
  cl-style flags and the MSVC standard-library include paths, not a
  measurement of a child process; the frozen log is the measurement.)
- **`--query-driver` is a different mechanism, and it is off by default.**
  It is an *allowlist* for the driver clangd may execute **to infer include
  paths**: "If the compile flags has `/custom/compiler` as the driver name,
  clangd will run something similar to `/custom/compiler -E -xc++ -v
  /dev/null` and parse its output … Since it implies executing arbitrary
  binaries, that might be checked-in with the project, clangd does not
  perform this inference automatically. You need to allowlist binaries you
  deem safe for execution using `--query-driver`" (Context7:
  `clangd.llvm.org/guides/system-headers`). It is **not** how the
  compilation database is consumed, and Riltra does not use it.

So the executable inside the trust boundary is the **operator-configured
language server** (`lsp.rs:29-33`) — not a project-supplied binary. What
the project controls is the **configuration and content the server reads**:
the compilation database, a `.clangd` file, the source tree, and the
provider's own index state. That is the same "untrusted child" boundary the
adapter already documents (`lsp.rs:4-6`: "servers may run hooks/builds/
network") — a *language server* may run build tools, whatever it is.

So: **a clangd run may write something somewhere** (the caches above). It is
not a read-only program, and no configuration of Riltra can make it one.

### 4.2 What the MCP annotation actually means

The MCP specification defines `readOnlyHint` as: *"If true, the tool does not
modify its environment. Default: false."* (Context7:
`modelcontextprotocol.io/specification/2025-11-25/schema`, `ToolAnnotations`.)
It is a **hint about the whole environment**, not only about the project
directory, and the specification says annotations are *"untrusted unless they
come from trusted servers"* — the reverse direction (a server promising
read-only) is exactly what a client relies on.

The eleven existing tools are honestly `read_only_hint = true` in the sense
that matters for Riltra's state: they modify no schema and no data, and they
never create or migrate a store. The one caveat is the project's accepted
limitation L2: "opening or reading any WAL-mode store can create or touch
its `-wal`/`-shm` sidecars" (`docs/m8-design-gate.md:221-227`, `:239-242`).
A clangd-backed tool cannot make the same promise at all: the provider
writes a cache (§4.1).

### 4.3 Two modes, and the recommendation

| | **Mode A — honest annotation** (recommended) | **Mode B — closer to the literal wording** |
|---|---|---|
| Annotation | `read_only_hint = false`, `destructive_hint = false`, `idempotent_hint = false`, `open_world_hint = true` | `read_only_hint = true`, `open_world_hint = true` |
| Background index | on (the default) | off — Riltra passes `--background-index=0` (Context7-verified spelling) |
| Repository writes | the provider writes `<project>/.cache/clangd/index/*` | none from the provider — *inferred* from the same readiness experiment, not measured |
| Machine writes | the provider writes the user cache dir for system headers | the same |
| Answer quality | the pilot's: cross-translation-unit references work, project-wide coverage | **degraded**: without the project index the server answers from the current translation unit only. This is *inferred* from the pilot's readiness experiment — the pilot dropped the index and measured the effect, it did not run `--background-index=0` (`docs/m9-clangd-pilot.md:322-344`); Mode B itself is not yet measured. |
| What the annotation means | "this tool does not modify the repository or Riltra's stores; the language server it starts writes its own index cache" | "this tool does not modify its environment" — **not literally true**: the system-header cache is still written to the machine |
| Cost in the diff | none in the CLI path | `index.rs` and therefore the CLI path (§9.1) |

The recommendation is **Mode A**, because it is the only one whose claim is
true as written. Mode B's `read_only_hint = true` would be true only for the
*project* directory and false for the machine, and it would buy that by
degrading every answer. If the owner prefers a literal `read_only_hint =
true`, Mode B is the only way to get close to it, at the inferred cost of the
project index — that is OD-1, and it is the owner's decision, not the
design's.

The remaining three hints, in both modes:

- `idempotent_hint = false` — the specification's meaning is *"calling the
  tool repeatedly with the same arguments will have no additional effect on
  its environment"*, and the provider's cache and index writes are exactly
  such an effect. (Separately, the answer itself is not stable either: the
  same question can have two different answers depending on how long the
  index has been building, `docs/m9-clangd-pilot.md:331-344`. That is a
  reason for the caller not to assume stability, not the reason for the
  hint.)
- `destructive_hint = false` — the provider only adds to or refreshes its own
  cache; it deletes nothing of the user's.
- `open_world_hint = true` — the specification's meaning is *"this tool may
  interact with an 'open world' of external entities"* (default: true). The
  tool starts a **provider the operator chose**, and Riltra does not restrict
  what that provider may spawn: a language server such as rust-analyzer
  runs the project's build tools, and those may reach the network. For
  clangd specifically the pilot's frozen log shows the opposite — the
  compilation database is parsed in-process and no project binary is
  executed (§4.1). The honest value is therefore `true`: the risk is
  **provider-dependent**, and the tool cannot know which provider the
  operator configured. (The other eleven tools keep `open_world_hint =
  false`: they start no process at all.)

### 4.3.1 The health document's three capability flags

The health document is hand-written (`mcp.rs:1240-1271`) and carries nine
fields under `capabilities`, three of which this design has to scope —
`read_only`, `mutations` and `network` — because "read-only", "mutates" and
"network" are three different questions, and because the project already
uses a *store-centric* reading of read-only elsewhere: the design gate says
the server "serves 11 read-only tools … reads never create or migrate a
store" (`docs/m9-design-gate.md:152-154`), and G-MCP's stop condition is "a
read tool creating / migrating a store" (`:540-541`).

| Flag | The readings | Value under each |
|---|---|---|
| `capabilities.read_only` | (i) **tool-level**: "the server exposes only read-only tools", in the MCP annotation's sense (§4.2) · (ii) **server-level**: "the server modifies nothing on the machine" · (iii) **store-level**, the design gate's own wording: "no tool creates or migrates a store" | (i) `false` — the twelfth tool is not `read_only_hint = true` · (ii) `false` — the provider writes its cache · (iii) `true` — the LSP path is store-free (§4.4) |
| `capabilities.mutations` | (i) **narrow**: "a tool mutates Riltra's own state (the index, the knowledge store)" · (ii) **broad**: "the server mutates anything on the machine" | (i) `false`, unchanged · (ii) `true` — the provider writes its cache |
| `capabilities.network` | (i) **narrow**: "Riltra opens a network connection" · (ii) **broad**: "the tool's execution may reach the network" | (i) `false`, unchanged — Riltra makes no network call · (ii) `true` — depends on the provider (a server that runs build tools may fetch, §4.3) |

The table names the readings. The design **proposes** reading (i) for
`read_only` (`false`) and the narrow readings for `mutations` and `network`
(`false` and `false`) — OD-7 — while reading (iii), the gate's store-centric
wording, would leave `read_only` **`true`**; the scope is the owner's to fix
(OD-7), and this document does not present one reading as the answer. The
broad readings are the honest ones if a client reads the flags as claims
about the whole machine.

### 4.4 What Riltra itself guarantees (the provable part)

These hold in both modes and are testable:

1. **Riltra never writes into the repository.** The LSP path touches no file
   in the project: it only reads the queried document through the provider
   and hashes it (`lsp.rs:826` `disk_digest`).
2. **Riltra never opens the index or the knowledge store** for this tool.
   `semantic_query_lsp` has no database handle (`index.rs:2439-2443`); the
   MCP tool therefore inherits the "reads never create or migrate a store"
   rule by construction.
3. **The MCP stdout carries JSON-RPC only.** The provider's stdout and stderr
   are the adapter's pipes (`lsp.rs`); the debug line goes to stderr
   (`lsp.rs:466-467`).
4. **The provider's side effect is outside Riltra's control, and Riltra's own
   is empty.** The provider's known writes are its project index cache
   (`<project>/.cache/clangd/index/*`) and the user's cache directory
   (§4.1) — it may write elsewhere as well, which is why the annotation is
   not `read_only_hint = true`. What Riltra itself writes is nothing: test 7
   (§6.2) digests the database file, reads the edge rows back, and compares
   the repository's file listing before and after the call.

### 4.5 Alternatives considered and rejected

| Alternative | Rejected because |
|---|---|
| Copy the compilation database to a scratch directory outside the repository and pass `--compile-commands-dir` (Context7-verified flag) so the index cache lands there | it makes Riltra write (outside the repository, but still a write) and adds a copy mechanism to keep in sync; the strict `read_only_hint` is still false because of the system-header cache |
| Set `XDG_CACHE_HOME` / `LOCALAPPDATA` for the child | only moves the *header* cache; the project index cache still lands next to the CDB |
| Riltra generates a `compile_commands.json` | it would be a build action in a read tool, it would need the project's real build system, and CMake only exports it for Makefile/Ninja generators (`docs/m9-design-gate.md:266-274`) |
| A daemon, a persistent language server, or a connection pool | the pilot's cost is per question anyway; §7.4 |
| Disable the background index and keep `read_only_hint = true` | Mode B: the answer quality loss is inferred, not measured, and the annotation is still not literally true |
| A project `.clangd` or a user `config.yaml` that sets `Index: Background: Skip` (Context7: `clangd.llvm.org/config.html`) | it would change the project's own configuration — a write into the repository — and it would change the provider's behaviour for every other tool. Not Riltra's business. |

## 5. The correctness and freshness invariants

### 5.1 The invariants this tool must keep

| Invariant | Enforcement | How it is tested |
|---|---|---|
| A definition answer is not a references answer | the operation selects the branch (`index.rs:2461-2465`); the two arrays stay separate | test 2 |
| **References are not calls.** The tool returns *locations*, never edges; it does not write to `code_edges` | the LSP path never opens the store (§4.4) | test 7 (the database file and the edge rows are unchanged after the call) |
| `POTENTIAL_CALL` is never promoted to `CALLS` | the tool does not touch the graph at all | test 7 |
| No LSP answer is ingested into the graph or the database | the LSP path is store-free (§4.4) | test 7 |
| The evidence stays `Unverified`, or `Stale` when a change is proven | `semantic::Evidence::lsp_location` (`semantic.rs:173-193`) and `mark_changed_sources` (`lsp.rs:802-811`) | test 5 |
| The answer's verdict is the weakest fact | `lsp_answer_verification` (`index.rs:1029-1041`) | test 6 (a mixed Stale/Unverified answer) |
| A provider failure is explained by the provider's own diagnostics | `lsp_fallback_reason` (`index.rs:1045-1050`) plus the diagnostics drain (`lsp.rs:649`) | test 8 (the `fail-after-diagnostics` mock: the diagnostics text is inside the fallback reason) |
| **An empty answer never means "the symbol does not exist"** | an empty answer carries the provider's diagnostics and no fallback; the rule is stated in the tool's description (§9.2) | test 4 |
| The background index's and the compilation database's incompleteness are disclosed as far as the evidence allows | the provider's diagnostics are the evidence (clangd's own `severity: 1` "file not found" reaches the caller — `docs/m9-clangd-pilot.md:364-376`); the tool's description states that an answer is the provider's locations, **not** a completeness claim | test 4 + the tool's description |
| A provider without the capability is reported, not guessed | the adapter refuses with "unsupported: server lacks referencesProvider" (`lsp.rs:1008`) and the answer becomes a fallback | test 9 |
| E12 stays open | the tool answers "definition" / "references" only; it makes no construction/declaration claim | no test: the gap is documented, not closed |

### 5.2 The freshness bound that is actually guaranteed

Unchanged from the hardening (`docs/m9-lsp-hardening.md` §5), and the tool
must state it in the same terms:

| Fact | Status |
|---|---|
| The location is inside the registered repository | guaranteed (the adapter's canonical-root check, `lsp.rs:941-949`: "location outside repository") |
| The span is a valid range over the file as it was read | guaranteed by the adapter |
| The file did not change between the read and the check | provable → `Stale` when violated |
| The span matches the bytes on disk **now** | **not provable** — the server's index may be stale for a file that changed before the session (the pilot's residual limit, `docs/m9-clangd-pilot.md:378-402`) |
| The symbol identity | **not claimed** — no SCIP identity is fabricated for an LSP answer |
| The answer is complete | **not claimed** — it depends on the provider's index and compilation database |

So the answer is `Unverified` (or `Stale`), and the document says so at two
levels: per hit and for the answer.

### 5.3 No readiness mechanism

There is no "index ready" flag, no "coverage" claim, and no pre-flight check
that the compilation database exists — such a mechanism could only *claim*
completeness without proof, which the brief forbids and which the pilot's
readiness experiment shows is not even true (the same question has two
different answers depending on the index's age). The provider's diagnostics
are the evidence; Riltra reports them.

## 6. The acceptance tests

### 6.1 The harnesses (no new framework, no new repository)

- **The mock language server** `crates/cli/examples/mock_lsp.rs`. Its modes
  already cover: `success` (the default), `diagnostics`, `late-diagnostics`,
  `empty`, `retryable`, `stale-input`, `fail-after-diagnostics`,
  `client-request`, `stray-response`, `crash`, `hang`, `malformed`,
  `pollution`, `stderr-flood`, `no-refs`, `progress`. Four modes are added
  by this design (the oversized answer, the mixed verdict, the echoed
  position, the barrier — §6.2 tests 6, 10, 13, 14). It reads its mode and
  root from the environment (`RILTRA_MOCK_LSP`, `RILTRA_MOCK_ROOT`), so the
  MCP test harness sets them on the spawned server (`tests/mcp.rs:40-49`
  already has a `configure` hook ✓).
- **Locating the mock from an integration test**: `testutil::mock_lsp_bin`
  (`testutil.rs:56-72`) is inside `#[cfg(test)] mod testutil;`
  (`main.rs:32-33`), so an integration test cannot call it and duplicates
  the build/locate logic instead — the precedent is `tests/semantic.rs:966-983`.
  The new MCP test does the same (one small helper, no new module).
- **The MCP harness** `crates/cli/tests/mcp.rs` — the real binary, an isolated
  `RILTRA_HOME`, newline-delimited JSON-RPC over stdio, `call_tool` / `doc` /
  `is_error` helpers (`tests/mcp.rs:913-948`), a 15 s receive timeout
  (`tests/mcp.rs:17`).
- **The CLI harness** `crates/cli/tests/cli.rs` and
  `crates/cli/tests/semantic.rs` (the gated live test
  `cli_lsp_live_query_gated`).
- **The pilot fixture** `docs/pilots/clangd/fixture/` with its
  compiler-verified ground truth, for the local-only real-clangd tests.

### 6.2 The acceptance tests

| # | Test | What it proves | Harness |
|---|---|---|---|
| 1 | `semantic_mcp_carries_the_cli_document` | the MCP `structuredContent` equals the CLI's `semantic query --lsp-server` output **as a parsed value** (the transport re-serializes it, so bytes are not comparable); plus a unit test that the extracted `response::semantic_query_json` reproduces the historical inline bytes, trailing newline included | mock + MCP harness + CLI harness + a unit test in `response.rs` |
| 2 | `semantic_definition_and_references_stay_separate` | the definition answer fills `definitions` only, the references answer fills `references` only; the mock's two locations never mix | mock (`diagnostics` mode) |
| 3 | `semantic_reports_a_missing_provider` | with `RILTRA_LSP_SERVER` unset, empty, or relative: MCP error `provider_unavailable`; with a configured-but-missing executable: the contract document with `fallback` and the reason "language server not found" | mock + env manipulation |
| 4 | `semantic_empty_answer_carries_the_provider_diagnostics` | an empty answer with the provider's `severity 1` diagnostic reaches the caller in `server_diagnostics`, and `fallback` is `null` — the shape that distinguishes "no location" from "no such symbol". This is also the missing-compilation-database case: the provider cannot resolve the file, says so in a diagnostic, and the answer stays empty (`docs/m9-clangd-pilot.md:346-376`) | mock (`empty`, `late-diagnostics`) |
| 5 | `semantic_marks_a_changed_source_stale` | the answer's own `verification` is `Stale` when the source changed while the answer was produced, and the hit carries it too | mock (`stale-input`) |
| 6 | `semantic_reports_the_weakest_fact` | an answer that mixes a `Stale` and an `Unverified` fact is reported as `Stale` at the answer level | mock (a new mode: the answer carries two locations, one in a file that changed while the answer was produced and one in an untouched file — the existing `stale-input` mode changes the *queried* file, so every fact goes `Stale` and the mix cannot occur) |
| 7 | `riltra_writes_nothing_to_the_repository_or_the_stores` | after a call: the index database file's byte digest is unchanged, the edge rows read back are identical, and **Riltra's own** writes are empty — the repository's file listing is compared before and after (`listing(&repo)` **and** `listing(&home)`, `tests/mcp.rs:950`), and the mock provider writes nothing, so the listing is unchanged. (With the real provider the repository listing *does* change: the provider's `.cache/clangd/index` — §6.4. The claim under test is Riltra's, not the provider's.) The `-wal`/`-shm` sidecars are excluded from the digest (accepted L2, §4.2) | mock + the listing helper |
| 8 | `semantic_survives_a_provider_crash_and_a_timeout` | a crashing server and a hanging server both end in the contract document with a fallback reason, inside the adapter's bounded time, and the fallback reason carries the provider's diagnostics (`fail-after-diagnostics`). The hanging case needs its own receive timeout: the harness's 15 s (`tests/mcp.rs:17`) is shorter than the adapter's 30 s request deadline (`lsp.rs:39`), so this test uses a 45 s receive timeout through a small `recv_timeout`/`call_tool_with_timeout` helper | mock (`crash`, `hang`, `fail-after-diagnostics`) |
| 9 | `semantic_reports_a_provider_without_the_capability` | a server that does not advertise `referencesProvider` yields the fallback "unsupported: server lacks referencesProvider", not an empty success | mock (`no-refs`) |
| 10 | `semantic_keeps_mcp_stdout_pure_and_the_answer_bounded` | every stdout line of the MCP server is a JSON-RPC 2.0 message (the harness's `recv()` already asserts this), and a provider answer larger than the adapter's 8 MiB message cap is refused with a fallback instead of being relayed | mock (a new "huge answer" mode) |
| 11 | `semantic_refuses_an_escaping_or_missing_path` | `../escape.cc`, an absolute path, a drive-qualified path, a missing file and a symlink pointing out of the repository are all refused `invalid_input` — each with its own text (the texts differ: `semantic.rs:319-350`), and the missing file's containment text is disclosed in the tool's description | mock + the existing symlink helper (`tests/mcp.rs:1007-1027`) |
| 12 | `semantic_classifies_its_own_refusals` | an unknown repository → `not_registered`; a corrupt registry → `store_error`; a non-numeric `line` and an unknown `operation` → the SDK's schema refusal (both are added to the schema-refusal sweep, `tests/mcp.rs:617-664`). And the **order** of the checks: a known repository with an escaping path and **no provider configured** is refused `invalid_input` (the path), not `provider_unavailable` (§2.5) | MCP harness + env (no mock) |
| 13 | `semantic_column_unit_is_utf16` | a position on a line with non-ASCII text resolves the same way in the CLI and in MCP, **and** the offset the adapter received is the expected UTF-16 offset (not the byte offset) — the unit is the negotiated `utf-16`; the `col` schema description says the column is not the graph tools' byte column | mock (a new mode that echoes the requested `character` back, so the test can read the unit the adapter actually sent) + a non-ASCII fixture |
| 14 | `semantic_survives_two_calls_in_flight` | two `tools/call` requests in flight at the same time both return a **non-fallback** answer carrying the expected location (`fallback: null`) — a fallback document would not count, because a dead mock still produces one (`index.rs:2447-2458`, `:2466-2482`). The MCP session stays healthy and no stdout line is lost. The test **observes the overlap** with a barrier in the mock, under four rules: each instance writes a **per-process** marker under `RILTRA_MOCK_ROOT` (the PID-naming precedent is `mock_lsp.rs:116-117`), removes its marker when it proceeds or exits, the test starts from an **empty** marker directory, and a **barrier timeout is a hard failure** — the mock exits non-zero or answers with an error, and leaves a `timeout-<pid>` marker behind, so a non-fallback answer is impossible without the peer. The test asserts that no barrier timeout occurred (no `timeout-*` marker exists) and that both answers carry `fallback: null`. It must not assert per-call log receipts: the mock's log is a read-modify-write (`mock_lsp.rs:59-67`), so two concurrent instances can lose events. The provider-side half needs a real clangd and is local-only (§6.4) | MCP harness + the mock (a new barrier mode) |

### 6.3 The existing tests that change

| Test | Change |
|---|---|
| `mcp_tools_list_advertises_exactly_eleven_readonly_tools` (`tests/mcp.rs:175`) | renamed (the count is no longer eleven) and the count becomes **twelve**; the per-tool annotation loop (`tests/mcp.rs:213-225`) asserts `readOnlyHint`, `openWorldHint` and `idempotentHint` **per tool**: the eleven keep their current values, and the new tool carries its own (`false`, `true`, `false`) |
| `t1_health_composition_matches_cli_and_creates_nothing` (`tests/mcp.rs:427`, the assertions at `:458` and `:463`) | the health document lists the twelfth tool, and the three capability flags take the values of the scope the owner fixes (OD-7, §4.3.1) |
| the G4 unknown-input-fields loop (the assertions at `tests/mcp.rs:610-616`) and the schema-refusal sweep (`tests/mcp.rs:617-664`) | the new tool is added to both |
| the CLI's `semantic query` output | **unchanged** — the formatter is extracted byte-for-byte (test 1's unit test pins it) |

The eleven existing tools keep their names, descriptions, schemas and
annotations. The health tool's *output* changes (it must list the twelfth
tool; whether `read_only` changes follows the owner's scope choice — OD-7,
§4.3.1) — that is an existing tool's output, and it needs the owner's
approval (OD-7).

### 6.4 The local-only evidence (not in CI)

- The **real clangd** run against the pilot fixture: the E1/E7 answers are the
  pilot's, the `.cache/clangd/index` write is observed in the repository, and
  the Mode A / Mode B difference is measured there (Mode B's cost is
  currently only inferred, §4.3). Gated exactly like the existing live test
  (`RILTRA_TEST_LSP=1`), because CI has no clangd and no C++ toolchain.
- A **read-only filesystem** variant: the repository mounted read-only (or
  its `.cache` path made unwritable) — the answer is unchanged and the
  provider's failure to write is visible in its diagnostics.
- A **parallel-cache** variant: two clangd processes against the same
  repository with a cold index; afterwards the index cache is still readable
  by a third run and every answer is well-formed (§7.5, test 2).

## 7. The performance impact and the limits

### 7.1 The measured baseline (the pilot, unchanged)

| Measure | Fixture | MMSP | colmap (reconstructed database) |
|---|---|---|---|
| Latency, second ask (min–median–max) | 1.49–1.63–1.73 s | 2.17–2.39–2.55 s | 2.67–2.93–11.46 s |
| Server peak working set | 168–191 MiB | 240–247 MiB | 314–350 MiB |
| No compilation database | 10.18 s, empty answer, `fallback: null` | | |
| clangd not installed | 0.05 s, the fallback payload | | |

(`docs/m9-clangd-pilot.md:573-587`.)

### 7.2 What the MCP path adds

**No material overhead.** The MCP tool calls the same `semantic_query_lsp`
(`index.rs:2439`) with the same adapter, the same limits, and the same
one-process-per-question model. The transport cost is one JSON-RPC round
trip — milliseconds. The latency and the memory are the provider's, and they
are the numbers above. The MCP server process itself adds nothing
measurable: the child is spawned and reaped inside the call.

Three operational consequences:

- **The client's timeout must exceed the provider's worst case.** The
  adapter's request deadline is 30 s (`lsp.rs:39`, applied at `lsp.rs:461`),
  the warm-up deadline is 15 s (`lsp.rs:844`), and a **timeout is not
  retried** — the retry loop only repeats the "unready" answer
  (`lsp.rs:156-159`, the loops at `lsp.rs:977`/`:1012`). The whole bounded
  worst case is therefore **under a minute**: the warm-up, the request
  deadline, and the kill + bounded reap (5 s, `lsp.rs:412-427`). OpenCode's
  default `execution` timeout is **12 hours** (Context7:
  `opencode.ai/v2/docs/mcp-servers`), so the default already covers it and
  no override is needed; if the operator sets one, it belongs in
  `mcp.servers.riltra.timeout.execution` and 120 s is a comfortable margin
  above that worst case — a chosen margin, not a derived minimum (§3.2).
- **Peak memory is per call, not cumulative across sequential calls**: the
  server process is killed at the end of every call (`lsp.rs:1038`
  `shutdown`), so the 168–350 MiB is transient. The MCP server's own
  footprint is unchanged. (Overlapping calls are a different matter — see
  §7.5.)
- **A client timeout does not cancel the call.** The tool runs on
  `tokio::task::spawn_blocking` (`mcp.rs:382-387`), and a blocking task
  cannot be cancelled; the child is reaped only when the call ends
  (`lsp.rs:1038-1068`). If the dispatcher reads a retry while the first
  call is still outstanding (§7.5 — not established by this repository), a
  client that gives up and retries can therefore have two providers alive at
  once (168–350 MiB each). The MCP server sets no **explicit** concurrency
  limit; that is a risk to state, not a defect to fix here.

### 7.3 The limits

- The answer's completeness depends on the provider's project index and
  compilation database, which Riltra neither builds nor measures (§5.3).
- The answer is not fresh against bytes that changed before the session
  (§5.2).
- The response size is bounded by the adapter's existing caps (8 MiB per
  message, 10 000 messages). There is no paging: the tool returns everything
  the provider said for that position. A project-wide references list can
  therefore be large. Capping it needs a new member in the shared document
  (and a matching CLI flag) — OD-3.

### 7.4 What is deliberately not built

No daemon, no persistent language server, no connection pool, no result
cache, no background indexing service, no LSP ingestion, no new database, no
new dependency. The pilot's measurements "do not justify a schema change, a
new database, a daemon, a new dependency, or an LSP ingest path"
(`docs/m9-clangd-pilot.md:427-429`), and its cost is per question because
the process is spawned per question (`docs/m9-clangd-pilot.md:594-604`).
That is the same behaviour the CLI has today, and the pilot gives no
evidence that a persistent server would pay for its complexity.

### 7.5 The risk of parallel calls against one provider cache

Each `tools/call` spawns its own language server and kills it at the end of
the call (§7.2), and the MCP server does **not** serialize them itself: the
tool bodies run on `tokio::task::spawn_blocking` (`mcp.rs:382-387`) with no
lock, a blocking task cannot be cancelled, and the child is reaped only when
the call ends (`lsp.rs:1038-1068`). Whether the **rmcp dispatcher** reads a
second request while the first is still outstanding is **not established by
this repository** — rmcp 3.5.1 is a dependency, not vendored source — so the
document states it as an open question and lets test 14 (§6.2) settle it
with an observable barrier. If the dispatcher does overlap them, two
overlapping calls mean **two provider processes against the same
repository**, and with clangd they share the same project index cache
(`.cache/clangd/index`, §4.1).

What the evidence says, and what it does not:

- The pilot measured a **readiness race** with a single process: the first
  question after a cold index can be answered from the current translation
  unit alone — the declaration instead of the definition
  (`docs/m9-clangd-pilot.md:322-344`). That concurrent cold calls widen this
  window is an **inference**, not a measurement.
- **Two writers on the same `*.idx` cache is not something the pilot
  measured**, and the adapter's contract says nothing about it: the cache is
  the provider's own file format, written and read only by clangd. Whether
  concurrent writers corrupt it, or merely race to refresh it, is **unknown
  from the evidence available here**.
- The MCP protocol gives no ordering guarantee for `tools/call`, and this
  design adds no lock, no semaphore, no pool and no daemon (§7.4) —
  deliberately.

The minimum tests that would settle it, with no new machinery:

1. **Client-side (CI, mock):** two `tools/call` requests in flight at the
   same time both return a **non-fallback** answer carrying the expected
   location (`fallback: null` — a fallback document would not count, because
   a dead mock still produces one), the MCP session stays healthy, and no
   stdout line is lost — and the test **observes the overlap** with a
   barrier in the mock, under four rules that make a sequential dispatcher
   unable to pass: each instance writes a **per-process** marker under
   `RILTRA_MOCK_ROOT` (the PID-naming precedent is `mock_lsp.rs:116-117`)
   and removes it when it proceeds or exits; the test starts from an
   **empty** marker directory; and a **barrier timeout is a hard failure** —
   the mock exits non-zero or answers with an error and leaves a
   `timeout-<pid>` marker behind, so no non-fallback answer is possible
   without the peer. The test asserts that no barrier timeout occurred (no
   `timeout-*` marker exists) and that both answers carry `fallback: null`.
   It must not assert per-call log receipts: the mock's log is a
   read-modify-write (`mock_lsp.rs:59-67`), so two concurrent instances can
   lose events. This settles the **transport and the adapter** for the
   pinned rmcp version and the stdio transport; it says nothing about the
   provider's cache.
2. **Provider-side (local-only, real clangd):** two clangd processes against
   the same repository with a cold index; afterwards the index cache is
   still readable by a third run, and every answer is well-formed. This is
   the measurement the pilot did not make, and it is the one that decides
   whether a future decision — a lock, one long-lived provider, or
   accepting the risk — is needed.

Until test 2 exists, the risk is **recorded, not mitigated**: the design
adds nothing to prevent it, and says so rather than pretending the
one-process-per-question model makes it impossible.

## 8. The reviewer's findings and the follow-up

The design was audited by an independent reviewer
(`opencode/deepseek-v4.1-flash-free#high`), read-only, over the whole
document and the code it cites. **Round 1: NO-GO as written** — two High
findings, nine Medium findings and eight Low findings. All were corrected in
this document (the corrections are in the document, not in code) and the
rewritten document went back to the same reviewer for a second round.

### Round 1 — the findings and the corrections

| Severity | Finding | Correction |
|---|---|---|
| HIGH | "byte for byte" MCP/CLI parity is impossible: `doc()` parses the CLI text into a `serde_json::Value` and `structured()` re-serializes it with sorted keys (`serde_json` without `preserve_order`, `Cargo.toml:11`), so the MCP result's key order is the transport's, and test 1 could never pass | §2.3 now states the real guarantee (the same document; the transport re-serializes it), and test 1 compares parsed values, with a separate unit test pinning the extracted formatter's bytes |
| HIGH | the twelfth tool breaks the health document (a hard-coded 11-name list and `read_only: true`, `mcp.rs:1240-1271`), the health test (`tests/mcp.rs:458`, `:463`), and the per-tool annotation loop (`tests/mcp.rs:220-222`), and the design gate still says "11 read-only tools" (`docs/m9-design-gate.md:152`, G-MCP `:521-539`) | §6.3 lists the three existing tests that change; §9.1 adds `health_doc` and the health test to the diff; §10 adds OD-7 (the health document's `capabilities.read_only` and the G-MCP/§1.8 amendment) |
| MEDIUM | the column unit differs: the LSP path uses the negotiated UTF-16 code units (`lsp.rs:51-53`, `:165-191`, `:982`), the graph tools use byte columns (`index.rs:1240-1256`) — the "same convention" claim was false | §2.2 states the unit, says the column is not interchangeable with the graph tools', and test 13 covers a non-ASCII line |
| MEDIUM | `operation` was a bare `String`, so the advertised schema carried no enum values | §2.2 and §9.2 use a real `Deserialize`/`JsonSchema` enum |
| MEDIUM | the refusal table misclassified: non-numeric values are SDK schema refusals, a missing file reads as "escapes repository", a relative `RILTRA_LSP_SERVER` would have been a `store_error` | §2.5 now has a row for each, and §3.3 requires the absolute-path check before the query |
| MEDIUM | the tool's description lacked the "empty ≠ not found" rule that §5.1 claimed it carried | the rule is now in the description (§9.2) |
| MEDIUM | test 7's `assert_home_untouched` asserts the home is *empty*, which cannot hold for a registered, indexed repository | test 7 now uses the before/after listing helper |
| MEDIUM | test 8 could not run under the harness's 15 s timeout against a 30 s request deadline, and §7.2's retry derivation was wrong (a timeout is not retried; 15 s is the warm-up) | test 8 uses its own receive timeout; §7.2 cites `lsp.rs:39`/`:461`, `lsp.rs:844` and `lsp.rs:156-159` |
| MEDIUM | "the eleven tools write nothing" contradicts the accepted L2 (`-wal`/`-shm` sidecars, `docs/m8-design-gate.md:221-227`, `:239-242`) | §4.2 states the caveat and keeps the conclusion |
| MEDIUM | Mode B is not one line: `semantic_query_lsp` builds the spec internally (`index.rs:2446`), so the argument change touches `index.rs` and the CLI path | §4.3 and §9.1 state the cost; §9.3 no longer claims "one line" |
| MEDIUM | §8 pre-claimed an audit with blank findings | this section |
| LOW ×8 | citation errors (`bounded_server_text` is `lsp.rs:1362`; the containment check is `lsp.rs:941-949`; the Stale marking is `mark_changed_sources` at `lsp.rs:802`; the pilot's `:427-429` says something else; the G-GRF paraphrase was too strong), test-plan gaps (the mock's `no-refs` mode, the invalid operation, the unknown repo, the schema refusal, the mixed-fact verdict, the `fail-after-diagnostics` text), the annotation rationales (`idempotent_hint` is about the environment, not answer stability; `open_world_hint` needed a rationale), Mode B's cost being inferred rather than measured, the header's "supersedes D-M9-2" overstatement, the `line`/`col` = 0 divergence from the CLI, the cancellation caveat, and the missing-file text | all corrected in §1.3, §1.1, §2.2, §2.5, §4.2, §4.3, §6.1, §6.2, §7.2, §7.4 and the header |

Rejected findings: none. Every finding was either a factual error of the
document or a gap in it; none was a matter of taste.

### Round 2 — the follow-up on the rewritten document

The same reviewer audited the rewritten document and returned a further
checklist: three contract corrections, five test-plan corrections, four
completeness items and eight low nits. All were applied, and the result went
back for a third round.

| Area | Finding | Correction |
|---|---|---|
| Contract | with `operation` as a deserialized enum, an unknown value never reaches the tool, so it is an SDK schema refusal and not `invalid_input` | §2.5's row changed; test 12 follows it |
| Contract | the derived enum needs the same `schemars` crate override as the other input structs | §9.2's sketch carries `#[schemars(crate = "rmcp::schemars")]` on `Operation` |
| Contract | on a provider failure `server_diagnostics` is empty and the diagnostics live inside `fallback.reason` (`index.rs:2466-2482`) | §2.5 says so explicitly |
| Contract | the order of the checks was not stated: a bad path must be reported before a missing provider | §2.5 states the order; the round-2 fix was incomplete and was completed in round 3 (the sketch now validates the path explicitly) |
| Contract | §4.4's "at most `<project>/.cache/…` and the user cache directory" overstates what is known, and cited the wrong test | §4.4 now says the provider's writes are outside Riltra's control and cites test 7 |
| Test plan | test 6's "or `stale-input` plus a second location" cannot produce a mixed verdict — that mode makes every fact `Stale` | test 6 now requires a new mock mode with two locations, one in a changed file |
| Test plan | test 7 needed a database digest and an edge-table read, not only a listing | test 7 now states all three, with the `-wal`/`-shm` sidecars excluded (accepted L2) |
| Test plan | test 8 needed a stated receive timeout and a helper for it | test 8 states 45 s and a `recv_timeout`/`call_tool_with_timeout` helper |
| Test plan | test 13 needed a mock mode that echoes the requested position, and the schema-description assertion | test 13 states both |
| Test plan | test 12's non-numeric case belongs to the schema-refusal sweep (`tests/mcp.rs:617-664`) | test 12 points there |
| Completeness | the README (`:46`, `:106`), the `mcp.rs` module header (`:5`) and the renamed tools-list test also state the tool count | §9.1 lists all three; §6.3 lists the rename |
| Completeness | the M8/CP1/CP3 documents that state "11 tools" are historical records | §9.3 classifies them as historical (not amended); only the live design gate is amended |
| Completeness | the `tests/semantic.rs` row in §9.1 contradicted §6.1 (the MCP test has its own locate helper) | the row is removed |
| Completeness | `open_world_hint = false` was not defensible: the provider executes the project's compile commands, which may reach the network | §4.3 sets `open_world_hint = true` (also the specification's default) and gives the reason; the tool's description carries the provider caveat — **the premise of this finding was itself wrong** and was corrected in round 5 (§4.1: the compilation database is parsed in-process, no project binary is executed); the annotation value is unchanged, for a different reason |
| Low nits | citation ranges, a duplicated citation, the health-test range, "each with its own kind" (the kind is the same; the texts differ), "all addressed" in the present tense, and the missing-file text | all corrected in §1.1, §1.3, §2.5, §6.2, §6.3, §8 and §11 |

Rejected findings: none in either round.

### Round 3 — the verification of the round-2 corrections

The same reviewer verified the corrected document and returned **one High
defect, three Medium defects and nine Low nits** — the High one being that
the document promised a check order that the sketch did not implement. All
were corrected:

| Severity | Finding | Correction |
|---|---|---|
| HIGH | the promised order (repository → position → **path** → provider) was not implemented: `validate_doc_path` runs *inside* `semantic_query_lsp` (`index.rs:2445`), i.e. after the provider lookup, so a bad path with no provider configured was answered `provider_unavailable` instead of `invalid_input` | §9.2's sketch now validates the path explicitly, with `semantic::validate_doc_path`, before the provider is looked up; test 12 gained that exact combination |
| MEDIUM | the missing-file disclosure was claimed in §2.5 and test 11 but was not in the tool's description | the description now says that a path that does not exist is reported as a path-containment refusal, never as an empty answer |
| MEDIUM | test 7 compared `listing(&home)` — the Riltra home — while claiming the repository's listing; and the test's name claimed more than the mock can prove (the real provider writes `.cache/clangd/index`) | test 7 is renamed `riltra_writes_nothing_to_the_repository_or_the_stores`, lists **both** the repository and the home, and states that the claim under test is Riltra's, not the provider's (§6.4 keeps the provider's write visible) |
| MEDIUM | the README's headings still claimed a read-only server (`README.md:46`, `:103`) | §9.1 lists the heading as well, and the estimate grows accordingly |
| LOW ×9 | the annotation loop asserts three hints, not one; "the other two hints" preceded three bullets; the `server_diagnostics` note did not cover the failure case; the column-unit wording claimed the adapter converts the request position (it converts the *returned* positions; the request is sent in the negotiated encoding); Mode B's "no repository writes" was unmarked as inferred; "the eleven existing tools … behaviour unchanged" contradicted the health output; three citation ranges and two size estimates were off; the trailing-newline rule of `response.rs` was not accounted for | all corrected in §2.2, §2.3, §4.2, §4.3, §6.2, §6.3, §9.1 and §9.3 — except the column-unit wording, which round 4 found still wrong (§2.2 now says the request carries `col-1` as it is and the adapter converts the *returned* positions) |

Rejected findings: none in any round.

### Round 4 — the verification of the round-3 corrections

The same reviewer verified the corrected document: **no High or Medium
defect remained**, and the verdict was *GO as a design proposal*, conditional
on five Low corrections — all applied:

| Finding | Correction |
|---|---|
| §2.2 still claimed the adapter converts the **request** position (it converts the *returned* positions; the request carries `col-1` as it is) | §2.2 corrected, and the §8 round-3 row no longer claims it was already fixed |
| test 12's harness said "mock", but the order case needs no mock | the harness column now reads "MCP harness + env (no mock)" |
| `tests/semantic.rs:966-980` was short | the range is now `:966-983` |
| §9.3's "only the live design gate and the README" omitted the module header | the module header is named |
| the §8 round-2 row was stale after the round-3 fix | the row now says the round-2 fix was completed in round 3 |

The reviewer's closing words: *"No High or Medium defects remain, and the
implementation stays out of scope pending its own OWNER GO."*

### Round 5 — the owner-directed corrections, and their verification

After round 4, the **owner** (not the reviewer) asked for four corrections:
the OpenCode v2 configuration, the claim about clangd running compile
commands, the health document's three flags, and the risk of parallel calls
against one provider cache. They were applied to this document and then
verified by the same reviewer.

The verification found that **the second correction had replaced one
overstatement with a larger factual error** — the correction had claimed
that "the compiler binary the database names is executed as a frontend",
which the repository's own frozen clangd log contradicts
(`docs/pilots/clangd/raw/env/clangd-check-fixture.txt:11-13`: the
reconstructed argv contains options `cl.exe` does not accept, and the
`-cc1 … -fsyntax-only` arguments are clangd's **own in-process** frontend).
The error had propagated into five places, including the client-visible tool
description.

| Severity | Finding | Correction |
|---|---|---|
| HIGH | "the compiler binary the database names is executed as a frontend" — contradicted by the frozen log; the database entry is **interpreted**, not executed, and no project binary runs (only `--query-driver` names a binary clangd may run, and it is not used here) | §4.1 rewritten with the log as evidence; the trust-boundary sentence now names the **operator-configured server** as the executable and the project's *configuration and content* as what the project controls; §4.3's `open_world_hint` rationale, the §9.2 tool description and its annotation comment were rewritten |
| MEDIUM | test 14 ("two calls in flight") could not distinguish "two providers alive at once" from "one after the other", so it did not prove what §7.5 claimed | test 14 now **observes the overlap** with a bounded barrier in the mock, and §7.5 states that rmcp's dispatch model is not established by this repository; the mock's log race is called out |
| MEDIUM | §4.3.1 scoped `read_only` to the tool list while scoping its siblings to the server, and the design gate's own store-centric wording would leave it `true` | §4.3.1 now names the three readings for each flag with the value under each, and the value is the owner's to fix (OD-7); the design proposes the narrow scope without presenting it as forced |
| MEDIUM | the §8 round-2 row repeated the false premise | the row is annotated as superseded by this round |
| LOW ×6 | "a clangd run always writes" → "may write"; the 120 s threshold was unexplained → now "comfortably above the adapter's under-a-minute worst case"; "peak memory is per call, not cumulative" → "not cumulative across **sequential** calls"; §11's row still described the V1 flat `timeout` → now the V2 object; "concurrent cold calls widen that window" was inference presented as measurement; the `$PATH` sentence is now cited with its source | all corrected in §3.2, §4.1, §7.2, §7.5 and §11 |

Verification limits of this round: the OpenCode timeout defaults were **not**
verified in round 5 (that review had no Context7 access). The main agent
re-queried `opencode.ai/v2/docs/mcp-servers` on 2026-10-21 and confirmed
startup 30 s, catalog 30 s, **execution 12 hours**. rmcp's request-dispatch
model is not established here (the crate is not vendored).

### Round 6 — the verification of the round-5 corrections

The same reviewer verified the round-5 corrections. The High defect (the
compiler-execution claim) was confirmed fixed against the frozen log
(`clangd-check-fixture.txt:11-13`), and the Lows were applied — but the
applications themselves left four Mediums and five Lows:

| Severity | Finding | Correction |
|---|---|---|
| MEDIUM | §4.3.1 said both "this design does not pick one for `read_only`" and "the narrow scope … is what the design *proposes* (OD-7)", and claimed the proposal matched the gate's store-centric wording, which the table contradicts; §6.3's closing paragraph still required the health output to change | §4.3.1 now proposes reading (i) for `read_only` and leaves the scope to the owner; §6.3 defers to that choice |
| MEDIUM | test 14's barrier rules were incomplete — a stale marker, a timeout that proceeded instead of failing, and a reused marker directory could let a sequential dispatcher pass | the four rules and the `timeout-*` marker were added; §7.5 scopes the result to the pinned rmcp version |
| MEDIUM | §7.2 asserted "two providers alive at once" unconditionally, while §7.5 said the overlap is not established | §7.2's claim is now conditional and says "no **explicit** concurrency limit" |
| MEDIUM | the §8 round-5 entry presented a Context7 confirmation as part of a round that had no Context7 access | the entry now attributes the re-query to the main agent and to 2026-10-21 |
| LOW ×5 | the mock-mode count was three, not four; the tool description had lost the provider caveat; §4.1's heading was too narrow; §1.2 omitted `openWorldHint`; §11 had no row for the in-process interpretation | all corrected in §1.2, §4.1, §6.1, §9.1, §9.2 and §11 |

### Round 7 — the verification of the round-6 corrections

The same reviewer verified the round-6 corrections. Applied and confirmed:
§4.3.1 and §6.3 now agree with OD-7 — the design **proposes** reading (i)
for `read_only` (`false`) and the narrow readings for `mutations` and
`network`, and leaves the scope to the owner; test 14 and §7.5 require a
**non-fallback** answer and the `timeout-*` marker; §7.2's claim is
conditional on the dispatcher overlap that §7.5 says is not established
here; the round-5 entry attributes the Context7 re-query to the main agent;
the fourth mock mode appears in §6.1 and §9.1; the tool description carries
the provider caveat; §4.1's heading covers what clangd does not run; §1.2
names all three asserted annotations (`tests/mcp.rs:220-222`); and §11 has
the row for the in-process interpretation. The H1 correction was unchanged
and still matches the frozen log (`clangd-check-fixture.txt:11-13`).

One MEDIUM was found in the record: the new round-6 entry repeated round 5's
findings (the §4.3.1 scoping, the test-14 indistinguishability, and round
5's five Lows) instead of round 6's (the §4.3.1 contradiction, the
barrier's missing rules, and that round's five Lows). It now records round
6's findings and corrections. Two LOW findings: §4.3.1 said `capabilities`
carries "eight fields" — it carries nine (`mcp.rs:1249-1271`); and §6.2's
test 14 still said "well-formed answer" where §7.5 says "non-fallback" (a
fallback document is well-formed; `index.rs:2447-2458`, `:2466-2482`). Both
corrected.

| Severity | Finding | Correction |
|---|---|---|
| MEDIUM | the §8 round-6 entry repeated round 5's findings instead of round 6's | the entry now records round 6's findings and corrections |
| LOW | `capabilities` has nine fields, not eight (`mcp.rs:1249-1271`) | corrected |
| LOW | test 14's "well-formed answer" included the fallback document | "non-fallback answer", matching §7.5 |

No High or Medium defect remained after these corrections; **GO as a design
proposal** — the implementation stays out of scope pending its own OWNER GO.

Verification limits: this review had no Context7 access and ran no
commands; the OpenCode timeout values remain the main agent's re-query, and
the "only this file changed" and "696 tests pass" claims are the main
agent's.

## 9. The proposed minimum implementation diff

Not applied by this document. This is what an approved implementation would
touch.

### 9.1 The files

| File | Change | Size |
|---|---|---|
| `crates/cli/src/response.rs` | add `semantic_query_json(...)` and `semantic_hit_json(...)`, extracted from `main.rs:973-986` and `main.rs:904-926`, byte-identical; add the unit test that pins the bytes. Note the module's own rule ("callers add the trailing newline", `response.rs:3`): the semantic formatter keeps the newline **inside** its format string (as the CLI's inline version does, `main.rs:974`), so its doc comment must say so, and the CLI caller's bytes are pinned by a test | ~45 lines moved, ~20 lines of test |
| `crates/cli/src/main.rs` | call the shared formatter instead of the inline one; the CLI's output bytes do not change | ~35 lines removed |
| `crates/cli/src/mcp.rs` | the `SemanticInput` struct and the `Operation` enum, the `riltra_code_semantic` tool, the provider resolution (`RILTRA_LSP_SERVER`, absolute-path check), the semantic refusal classification, **the module header** (it says "eleven tools total", `mcp.rs:5`), **and the health document** (the twelfth tool in the list; `capabilities.read_only` per OD-7) | ~95 lines |
| `crates/cli/src/index.rs` | nothing, unless Mode B is chosen — then `semantic_query_lsp` gains the fixed `--background-index=0` argument, which **changes the CLI path too** | 0, or ~5 lines in the shared path |
| `crates/cli/tests/mcp.rs` | the acceptance tests of §6.2, the tools-list count 11 → 12 and the test's rename, the per-tool annotation assertions, the health test, the G4 sweep, and a small helper that locates the mock binary (the same build/locate logic `tests/semantic.rs:966-983` uses, duplicated because `testutil` is `#[cfg(test)]`-only) | ~320 lines |
| `crates/cli/examples/mock_lsp.rs` | four new modes: the oversized answer (test 10), the mixed-verdict answer (test 6), the echoed position (test 13), the barrier (test 14) | ~40 lines |
| `README.md` | the section heading "MCP server (read-only)" (`:103`) and the bold summary at `:46` stop claiming a read-only server; the two sentences that say "eleven read-only tools" (`:46`, `:106`) become twelve, and the new tool's non-read-only annotation is stated | ~6 lines |
| `docs/m9-clangd-pilot.md` | a dated note that the MCP surface now reaches the LSP — the pilot's own frozen measurements stay untouched (the precedent: the 2026-10-21 update note at `docs/m9-clangd-pilot.md:84-94`) | 1 paragraph |
| `docs/m9-design-gate.md` | a dated note that D-M9-2 and G-MCP's "11 read-only tools" are amended **by an explicit OWNER GO** (OD-6, OD-7) | 1 paragraph |
| `docs/licensing.md` | **no change**: no new dependency | 0 |

### 9.2 The shape of the code

```rust
// mcp.rs — the new surface
#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(crate = "rmcp::schemars")]
enum Operation {
    Definition,
    References,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(crate = "rmcp::schemars")]
struct SemanticInput {
    /// Registered repository name.
    repo: String,
    /// Repository-relative path of the anchor position.
    path: String,
    /// 1-based line of the anchor position.
    line: u32,
    /// 1-based column of the anchor position, in the server's negotiated
    /// encoding (utf-16 by default) — not the graph tools' byte column.
    col: u32,
    /// "definition" or "references".
    operation: Operation,
}

#[tool(name = "riltra_code_semantic",
       description = "Definition or references of the symbol at a position, answered by a language server (the same document as CLI `semantic query --lsp-server`). References are locations, not call edges: they are never edges, and they are never a completeness claim — the provider's project index and compilation database may be incomplete. An empty answer means the provider found nothing at this position; it is not a claim that the symbol does not exist. A path that does not exist is reported as a path-containment refusal (`document path escapes repository`), never as an empty answer. The answer carries the provider's verification state and its diagnostics. The language server is a local process that the operator configured; it reads the project and may write its own index cache, and what it runs is provider-dependent — some servers run the project's build tools — so this tool is not read-only.",
       annotations(title = "Semantic definition or references",
                   read_only_hint = false,      // Mode A: the provider writes its own index cache (§4.3)
                   destructive_hint = false,
                   idempotent_hint = false,     // the provider writes its cache on every call (§4.3)
                   open_world_hint = true))]    // the provider may spawn build tools (§4.3)
async fn riltra_code_semantic(&self, Parameters(input): Parameters<SemanticInput>) -> CallToolResult {
    blocking(move || semantic_doc(&input)).await
}

fn provider_exe() -> Result<String, CallToolResult> {
    // The operator's configuration, never the request's.
    let exe = std::env::var("RILTRA_LSP_SERVER").unwrap_or_default();
    if exe.is_empty() || !std::path::Path::new(&exe).is_absolute() {
        return Err(tool_error(
            "provider_unavailable",
            "no language server configured; set RILTRA_LSP_SERVER to an absolute path",
        ));
    }
    Ok(exe)
}

fn semantic_doc(input: &SemanticInput) -> CallToolResult {
    // 1. the repository: the same resolution the other tools use
    let (_home, canon) = match repo_root(&input.repo) { Ok(v) => v, Err(e) => return e };
    // 2. the position: the graph tools' rule (the CLI's `semantic query`
    //    accepts 0; this tool refuses it — §2.2)
    if let Err(e) = graph_anchor(input.line, input.col) { return e; }
    // 3. the document path: the same validator the query uses
    //    (`semantic::validate_doc_path`), run here so that a bad path is
    //    reported before a missing provider (§2.5). The query re-validates
    //    it; the check is cheap and idempotent.
    if let Err(e) = crate::semantic::validate_doc_path(&canon, &input.path) {
        return tool_error("invalid_input", &e);
    }
    // 4. the provider: operator-configured, never from the request
    let exe = match provider_exe() { Ok(exe) => exe, Err(e) => return e };
    // 5. the same query the CLI runs, and the same document it prints
    let span = index::SpanQuery {
        rel: &input.path,
        line1: input.line,
        col1: input.col,
        refs: matches!(input.operation, Operation::References),
    };
    match index::semantic_query_lsp(&canon, &span, &exe) {
        Ok(out) => doc(&response::semantic_query_json(&input.repo, &out)),
        Err(e) => semantic_query_error(e),
    }
}
```

`semantic_query_error()` classifies the CLI's refusal texts the way
`graph_query_error` does (`mcp.rs:783-799`): path and anchor problems →
`invalid_input`; everything else → `store_error`, fail-closed, message
preserved.

### 9.3 The explicit non-changes

- The eleven existing MCP tools: their names, descriptions, schemas and
  annotations are unchanged, and their behaviour is unchanged **except the
  health tool's output** (it must list the twelfth tool, and its read-only
  claim follows the scope the owner fixes — OD-7, §4.3.1).
- The index and knowledge stores: unchanged — no schema change, no
  migration, no new table, no write from this tool.
- The semantic contract (`semantic.rs`): unchanged — no new `Verification`
  state, no new `Precision`, no new field in `Evidence`.
- The LSP adapter (`lsp.rs`): unchanged. (Mode B changes `index.rs`, and
  therefore the CLI's provider arguments.)
- The graph (`edges.rs`), the MCP transport, the dependency set, the frozen
  CP1–CP3 and pilot evidence: unchanged.
- The **historical** documents that state "11 tools" (`docs/m8-design-gate.md`,
  `docs/m8-final-acceptance-review.md`, `docs/m9-cp1-pilot-spec.md`,
  `docs/m9-cp3-evidence.md`) are records of what was true when they were
  written; they are **not** amended. Only the live design gate
  (`docs/m9-design-gate.md`), the README and the `mcp.rs` module header
  carry the current state.

## 10. The decisions that still need the owner

**None of these is decided.** They are proposals with a recommendation, put
forward for the owner to accept, reject or replace; nothing here is an
authorization, and the implementation in §9 stays out of scope until the
owner says otherwise (OD-6).

| # | Decision | Options | Recommendation |
|---|---|---|---|
| OD-1 | The MCP annotation for a provider-backed tool, given that clangd writes a cache inside the project | **A**: `read_only_hint = false` + the background index on, honest and documented; **B**: `read_only_hint = true` + `--background-index=0`, closer to the literal wording, at the (inferred) cost of the project index — and it changes the CLI's provider arguments | **A** — B's claim is still not literally true (the system-header cache) and it buys a degraded answer |
| OD-2 | Where the provider executable comes from | **A**: the `RILTRA_LSP_SERVER` environment variable; **B**: a config file under `RILTRA_HOME` | **A** — the smallest, matches the existing operator switches, and OpenCode already supports a per-server `environment` |
| OD-3 | The MCP response bound | **A**: no paging (the answer is whatever the provider sent, bounded by the adapter's 8 MiB cap); **B**: a locations cap with a new `pagination` member in the shared document and a matching CLI flag | **A** for the minimum; **B** before a project-wide references list is used in practice |
| OD-4 | Who provides the compilation database | the operator (Riltra never generates one) | settled by the brief — listed so the record shows it was considered |
| OD-5 | Whether the tool refuses to run when no compilation database exists | no pre-flight check; the provider's diagnostics carry it | settled by the brief (§5.3) — a readiness mechanism is forbidden |
| OD-6 | The gate | this design **proposes** to supersede D-M9-2 (`docs/m9-design-gate.md:806`) for the scope in §9; only an explicit OWNER GO can do that. The C++ semantic gap in G-SEM stays `NOT YET VERIFIED` until the implementation's own evidence exists | an explicit OWNER GO naming this document |
| OD-7 | The health document's three capability flags and the G-MCP text | the twelfth tool must appear in `capabilities.tools`. For the other three flags, §4.3.1 names the readings: `read_only` is `false` under the tool-level and server-level readings and `true` under the gate's store-level wording; `mutations` and `network` are `false` under the narrow (Riltra's own) reading and `true` under the broad one. The design gate's "11 read-only tools" (`docs/m9-design-gate.md:152`) and G-MCP's "the 11 tools exposed" (`:521-539`) need an amendment | amend the gate and the README with the OWNER GO of OD-6; the design **proposes reading (i) for `read_only` (`false`) and the narrow readings for `mutations` and `network` (`false` and `false`)** — §4.3.1 — while the gate's store-centric wording would leave `read_only` `true`; the scope is the owner's to fix, and the broad scope is stated in §4.3.1 so the choice is visible |

## 11. The evidence behind the design

| Claim | Source |
|---|---|
| The MCP surface has no semantic tool | `docs/m9-clangd-pilot.md:417-421`; `crates/cli/src/mcp.rs:185-368` |
| The health document is hand-written, with a hard-coded tool list and `read_only: true` | `crates/cli/src/mcp.rs:1240-1271`; the tests at `crates/cli/tests/mcp.rs:458`, `:463`, `:220-222` |
| The CLI's live LSP query path and its guarantees | `crates/cli/src/index.rs:2439-2507`; `crates/cli/src/lsp.rs:18-44`, `286`, `461`, `649`, `802`, `826`, `844`, `885`, `941-949`, `973`, `1006-1008`, `1038` |
| The evidence contract | `crates/cli/src/semantic.rs:14-50`, `70-80`, `117-129`, `173-193`, `319-350`, `371-376`, `406-421` |
| The hardening that keeps the diagnostics and the staleness marking | `docs/m9-lsp-hardening.md` (§2 limits, §5 guarantees) |
| The measurements | `docs/m9-clangd-pilot.md:573-587`; the frozen raw evidence under `docs/pilots/clangd/raw/` |
| clangd parses the database entry **in-process**: the reconstructed command line is converted to `-cc1 … -fsyntax-only …` arguments, so the named compiler is not executed (only `--query-driver` names a binary clangd may run, and it is not used here) | `docs/pilots/clangd/raw/env/clangd-check-fixture.txt:11-13`, `clangd-check-mmsp.txt:11-13` (the pilot's frozen clangd log); Context7, `clangd.llvm.org/guides/system-headers` (2026-10-21) |
| The index cache lands next to the compilation database; the header cache lands in the user cache dir | Context7, `clangd.llvm.org/design/indexing` (2026-10-21) |
| clangd builds a **virtual** compile command from the database entry and applies `-fsyntax-only`, `-resource-dir` and drops unsupported flags; the project's build step is never run | Context7, `clangd.llvm.org/design/compile-commands` (2026-10-21) |
| `--query-driver` is an allowlist for the driver clangd may execute to *infer include paths*, and that inference is off unless allowlisted | Context7, `clangd.llvm.org/guides/system-headers` (2026-10-21) |
| `--background-index=0` disables the background index | Context7, `clangd.llvm.org/troubleshooting` (2026-10-21) |
| `--compile-commands-dir=<path>` exists | Context7, `clangd.llvm.org/faq` (2026-10-21) |
| `readOnlyHint` means "does not modify its environment"; the other hints' meanings | Context7, `modelcontextprotocol.io/specification/2025-11-25/schema` (2026-10-21) |
| A tool result carries `content` and `isError`; `structuredContent` is the structured form | Context7, `modelcontextprotocol.io/specification/2025-11-25/server/tools` (2026-10-21) |
| `rmcp` 3.5.1 exposes `read_only_hint`, `destructive_hint`, `idempotent_hint`, `open_world_hint` on `ToolAnnotations` | Context7, `docs.rs/rmcp` (2026-10-21) |
| OpenCode's local MCP server configuration has an `environment` map and a per-server `timeout` **object** (`startup`/`catalog`/`execution`); `opencode mcp add --env` exists | Context7, `opencode.ai/v2/docs/mcp-servers`, `opencode.ai/v2/docs/cli/commands` (2026-10-21) |
| OpenCode V2 groups servers under `mcp.servers`, replaces `enabled` with `disabled`, and splits `timeout` into `{startup, catalog, execution}` | Context7, `opencode.ai/v2/docs/migrate-v1` (2026-10-21) |
| The timeout defaults: startup 30 s, catalog 30 s, **execution 12 hours** | Context7, `opencode.ai/v2/docs/mcp-servers` (2026-10-21) |
| The accepted L2 limitation: a read-only open can leave `-wal`/`-shm` companions | `docs/m8-design-gate.md:221-227`, `:239-242` |
| The tool count is stated in the README, the module header and the live design gate | `README.md:46`, `:106`; `crates/cli/src/mcp.rs:5`; `docs/m9-design-gate.md:152`, `:522`, `:537` |
| clangd is deferred until a proven need with its own proposal and OWNER GO | `docs/m9-design-gate.md:806` (D-M9-2), `:580-600` (G-SEM), `:152` and `:521-539` (the eleven read-only tools) |

## 12. The boundary this document respects

- It changes **no** production code, schema, dependency, MCP tool, or frozen
  evidence. The only artifact this task produces is this document.
- It does **not** authorize an implementation, a checkpoint, a merge, a tag,
  or a release.
- It does **not** close E12, does not ingest LSP answers into the graph, and
  does not change the G-SEM status: C++ semantics on a real repository stay
  `NOT YET VERIFIED` until an implementation produces its own evidence.
- It does **not** modify D-M9-2 on its own: §9 needs an explicit OWNER GO
  (OD-6), and the implementation's evidence is what would move G-SEM.
