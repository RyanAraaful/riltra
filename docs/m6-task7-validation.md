# M6 Task 7 validation — multilingual golden corpus

Local Task 7 checkpoint on `m6-task7-multilanguage-corpus`, based on accepted
Task 6 commit `453a1b6f00d89f24d11e6d2346722047f10b7d1b`. The commit carrying
this report is the audit unit. No push, Task 8 pilot, external indexer/LSP
experiment, dependency or schema migration is included.

## Corpus and source-truth gates

The [fixture guide](../crates/cli/tests/fixtures/m6/README.md) describes the
manifest format, supported boundaries and deliberate golden-update workflow.
The harness is [m6_corpus/mod.rs](../crates/cli/tests/m6_corpus/mod.rs), loaded
by the existing edge integration suite.

| Directory | Source/manifest pairs | Full response bundles | Responses |
| --- | ---: | ---: | ---: |
| `py` | 16 | 16 | 100 |
| `ts` | 16 | 16 | 100 |
| `rs` | 16 | 16 | 100 |
| `c` | 15 | 15 | 94 |
| `cpp` | 16 | 16 | 100 |
| Total | 79 | 79 | 494 |

Every language covers direct calls, reference without call, nested calls,
local identities, same-name definitions, competing semantic targets, aliases,
same-name parameter bindings, recursion, cycles, missing definitions,
dynamic receiver evidence, unresolved owners, stale edits and unsupported
constructs. Python/TS/Rust/C++ additionally cover nested anonymous callables.
Standard C has no corresponding anonymous callable; its macro boundary is
tested without claiming macro expansion or expanded call resolution.

For each case, the real CLI runs init → syntax index → syntax-only queries
→ explicit synthetic SCIP import → callers/callees/trace/impact. Import must
leave the snapshot unchanged and make proven relations available without
reindex. Stale cases edit source after import without rebuilding. Local-scope
cases include two documents with identical local URIs, asserting that source
paths, counts and identity stay document-scoped.

The producer occurrences are hand-authored contract inputs. Their coordinates
and literal tokens are checked against source before payload construction;
they are not inferred from parser output. Independent assertions check:

- Exact expected CALLS and REFERENCES counts, including zero for false targets.
- Raw source bounds and digest; callee terminal within callsite; matching
  producer reference URI/span and an actual target definition occurrence.
- Import identity, provider and document scope; nonempty artifact and
  snapshot-nonce revision binding before normalization.
- Syntax-only queries never prove CALLS, and verification never becomes Current.
- Exact expected impact verdicts; all impact responses remain partial.
- Cycle termination and byte-identical repeated raw JSON for every response.

Only snapshot nonce and producer artifact identity are normalized for golden
comparison. All other fields, evidence, counters, reasons and ordering are
frozen. Explicit golden updates execute truth assertions first. The 494
responses include 163 impact answers: **30 HIGH, 133 UNKNOWN, zero MEDIUM,
SAFE or LOW**. Plain references have no proven callable owner under the
accepted derivation, so they stay UNKNOWN rather than being promoted to MEDIUM.
HIGH is observed static proof, not runtime execution or graph completeness.

Representative complete bundles:

- [Python nested calls](../crates/cli/tests/golden/m6-corpus/py/nested_calls.json)
- [TypeScript alias](../crates/cli/tests/golden/m6-corpus/ts/alias.json)
- [Rust nested anonymous scope](../crates/cli/tests/golden/m6-corpus/rs/nested_anonymous.json)
- [C competing targets](../crates/cli/tests/golden/m6-corpus/c/competing_targets.json)
- [C++ stale edit](../crates/cli/tests/golden/m6-corpus/cpp/stale_after_edit.json)

## Concrete bugs and minimal corrections

Initial RED: all 65 initial golden cases failed against empty expectations.
With deliberate updates enabled, 60 passed their truth assertions and five
alias cases failed: the implementation emitted Exact CALLS to a variable
binding. No golden was written for those failing cases. A first containment
restriction fixed those examples, but independent review found that a
same-name parameter still fit within the enclosing function range, and that
nested anonymous callsites could borrow the enclosing function as owner.

Additional RED runs reproduced all five same-name parameter failures and
all four nested-anonymous failures. Five competing-target fixtures were
also added to distinguish conflicting producer identities from mere absence
of semantic proof. After correction all 79 cases passed. The production fix
is restricted to evidence validation:

- A target definition occurrence must lie within the actual callable
  declaration-name token, not anywhere inside a same-name function body.
- A call's owner must match its nearest named callable source scope.
  Anonymous and class scope boundaries prevent borrowing an outer owner.
- Missing/changed/unreadable source or unprovable parse evidence fails closed.
  Results remain POTENTIAL_CALL with explicit reasons, including
  `target-unproven` and `owner-unproven`. No target or owner is fabricated.

The existing symbol rows retain whole-definition spans, which cannot prove
name-token identity or anonymous scope ancestry. To fix those concrete bugs
without a schema migration, derivation validates the missing evidence using
the existing tree-sitter grammars on confined, digest-matching source bytes
(1 MB file cap). This is an explicit departure from the earlier preference
for no syntax reparse at semantic import. It restricts accepted relationships;
it introduces no new semantic resolution, identity bridge, traversal feature,
or emitted syntax facts. Existing C declarator traversal is shared to locate
the real function name. Query paths still never reparse source.

Cost and limitations are explicit: syntax edge derivation parses supported
snapshot files for proof, and each semantic upsert does so again. A rebuild
with multiple carried imports can therefore parse each file multiple times,
once per derivation/upsert, rather than once per rebuild. No performance
optimization or new cross-request cache was introduced. Parse errors, stale
bytes, missing files and unsupported encodings can reduce proven coverage;
they cannot authorize a speculative CALLS edge. The new fail-closed unit test
checks changed and missing files. Old graph rows produced before this fix
need semantic reimport or `riltra index --full` to be re-derived; merely
upgrading the binary does not rewrite stored relationships.

Existing in-memory derivation tests now write their actual source into
temporary snapshot roots so they exercise real source evidence. Their
relationship assertions are preserved. Existing Task 4–6 goldens are unchanged.

## Validation and review

All commands returned exit 0 locally on Windows:

```text
cargo fmt --all -- --check
cargo check --locked
cargo clippy --all-targets -- -D warnings
cargo test --locked
```

**392 tests passed, zero failed or ignored**: 199 unit, 16 CLI, 129 edge
integration, 21 index, 15 semantic and 12 syntax. Baseline: 312; additions:
79 corpus tests and one source-proof failure regression. The corpus alone
passes with golden updating disabled, so verification cannot silently rewrite
expectations. This local result does not claim Linux/macOS CI completion.

Independent read-only review identified the two proof gaps above; both had
observed RED/GREEN regressions. Follow-up review found them closed, checked
the competing-target cases, and independently ran all 79 corpus tests.
Review also clarified the repeated parsing cost, documented above.

Scratch logs and the aggregate JSON audit are under gitignored
`target/m6-task7/`. The tracked fixtures, manifests, goldens and this report
are the portable audit evidence. Synthetic Tier A producer metadata does
not establish correctness of a real SCIP indexer or dynamic language runtime.
Those pilots and performance measurements belong to Task 8 and were not run.
Stop here for Task 7 audit; no push or Task 8 work.
