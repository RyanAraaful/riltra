# M6 multilingual relationship corpus

These are Tier A source/SCIP contract fixtures. They do not claim output from
a real language indexer, execute the source, or constitute the Task 8 pilot.
Python, TypeScript, Rust and C++ have 16 cases each; C has 15: **79 cases**.

Each source file has a matching JSON manifest. Coordinates are zero-based
lines and UTF-8 byte columns into the checked-in LF source. Each occurrence
includes its literal `text`, symbol `id` and definition/reference role. The
test checks those bytes before writing the minimal SCIP payload. Expected
edge counts and impact verdicts are authored in the manifest, independently
of tree-sitter or CLI output.

Cases cover direct calls, non-call references, nested calls, document-local
identities, same-name definitions, competing terminal targets, aliases,
same-name parameter bindings, recursion, cycles, missing definitions,
receiver-only evidence, unproven owners, stale edits and unsupported scopes.
The `target` manifest field identifies the inbound/impact query anchor;
outbound/trace queries use the named `c` definition.

`local_scope` also imports a mirrored document containing the same local URI
and distinct global caller URI. Counts and evidence paths must stay confined
to the queried document. `stale_after_edit` appends a comment without reindex:
the original digest and graph proof remain visible with Stale verification
and UNKNOWN impact.

`unsupported` tests module-level anonymous functions in Python/TS/Rust/C++;
`nested_anonymous` tests them inside a named function. Standard C has no
equivalent anonymous callable, so that case is not instantiated for C.
C's `unsupported.c` tests the raw macro boundary: macro expansion and the
expanded call target are explicitly out of scope. There is no positive macro
resolution assertion or ignored test concealing an unsupported feature.
Global C function-pointer initialization is a reference; executable global
call initialization is not valid standard C and is not used as a fixture.

Run the corpus from the repository root:

```text
cargo test --locked --test edges task7::
```

Full normalized responses are in `tests/golden/m6-corpus/{py,ts,rs,c,cpp}`:
494 JSON responses across 79 bundles. Each includes syntax-only callees and
impact, then imported callers/callees/trace/impact; stale cases add four more
responses. Only nonce and producer artifact identity are normalized. Raw
byte-for-byte repeatability and nonempty artifact/revision bindings are
checked before normalization. No graph-completeness assumption is made.

To deliberately regenerate goldens in PowerShell:

```powershell
$env:RILTRA_UPDATE_M6_GOLDENS = '1'
cargo test --locked --test edges task7::
Remove-Item Env:RILTRA_UPDATE_M6_GOLDENS
```

The update path runs all source-truth, edge-count and verdict assertions
before writing each bundle. Review source, manifest and golden changes
together; changing the implementation's output alone is not sufficient to
make a failing truth assertion pass.
