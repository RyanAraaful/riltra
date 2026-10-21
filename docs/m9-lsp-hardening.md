# M9 LSP Correctness Hardening — implementation and review record

Owner decision: **OWNER GO — limited implementation** (Task A: preserve LSP
diagnostics; Task B: prevent misleading stale evidence). Owner brief
2026-10-21. Branch `m9-multirepo-release-candidate`, starting remote HEAD
`72d72b59ade0280b2050af34d1ceb26253981017`.

Scope: the LSP adapter and the semantic-query mapping only. No MCP, graph,
E12, schema, daemon, dependency or release work. The frozen CP1–CP3 evidence
is not edited.

---

## 1. Root cause of each defect

| Defect | Root cause | Where |
|---|---|---|
| The server's own complaints about a file never reached the caller | `note_notification` recorded the notification's **name** for every non-`$/` notification, so the `publishDiagnostics` payload was dropped. The contract's `server_diagnostics` field was there all along; nothing filled it. | `crates/cli/src/lsp.rs` |
| An empty answer could not be told apart from "the server could not read the project" | the payload that explains it was dropped (above), and the empty answer carried no reason. The retry path made the wait 10.2 s. | `crates/cli/src/lsp.rs`, `crates/cli/src/index.rs` |
| A location from a stale index was presented next to the current file's digest | the adapter pairs the server's range with the file **as it is on disk now**; the server's range may come from an index that no longer matches those bytes. Nothing in the answer said so beyond `verification: "Unverified"`. | `crates/cli/src/lsp.rs:locations_to_evidence` |
| A server that publishes a warning *after* it answered lost the warning | `request()` returns at the response; a later notification was read during `shutdown()` and discarded. | `crates/cli/src/lsp.rs` |
| Server text could carry an absolute path into the caller's output | the server's error object, its stderr tail and its notice payloads were stored and emitted verbatim. | `crates/cli/src/lsp.rs` |

## 2. The change and why

### Task A — the diagnostics survive

- `format_diagnostics` turns a `publishDiagnostics` notification into compact
  lines: `diagnostic: <severity>[<code>] <relative-path>:<line> <message>`.
  The existing `server_diagnostics` field carries them; no schema change.
- Bounded three ways: at most `MAX_DIAGNOSTICS` (8) diagnostics per answer,
  at most `MAX_MESSAGE_CHARS` (160) characters of message and
  `MAX_CODE_CHARS` (64) of code, and the notices (protocol messages) are
  bounded at `MAX_NOTICE_CHARS` (200) and 256 entries.
- Path-safe: the location is the path **relative to the repository**, and a
  diagnostic for a file outside the repository is not reported at all
  (`relative_repo_path` + `paths::contains`). Every piece of server text that
  can reach the output goes through `sanitize_message`: a token that carries
  an absolute path is replaced whole by `<path>` — a `file:` URI, a Windows
  drive path, a UNC path, a POSIX absolute path, or a compiler flag that
  names one. URLs, relative paths and protocol names survive.
  The known limits, all deliberate and all on the side of keeping the text:
  - a **quoted** path ends at its closing quote, so the redaction is exact
    for the path — but the whole quoted section is replaced, so the words
    around the path inside the quotes are lost with it;
  - an **unquoted** path has no certain end, so it is redacted together with
    the words that follow it (the ambiguous part is removed rather than
    leaked — the price is that the rest of that message is lost, up to the
    next bracket);
  - a path glued to a letter of any script, a `~`-prefixed path
    (`~/code/x`, `~jane/x`), a single-leading-backslash Windows path
    (`\Users\jane\x`), and a non-`file://` scheme that carries a local path
    (`vscode://file/<local path>`) are **not** detected;
  - a token that starts with two backslashes is redacted as a UNC path, so
    a quoted escape sequence (`'\\d+'`) is over-redacted;
  - the stderr tail starts at a line boundary: a line that would be the tail
    of a path is dropped rather than half-kept.
- A warning never fails the query: the diagnostics are text next to the
  answer, and the answer's facts are untouched.
- An empty result is never turned into a fallback: the fallback is still
  only what a *provider failure* produces.
- A diagnostic published after the answer is delivered: `drain_notifications`
  reads the inbox until it is quiet for 100 ms (hard cap 500 ms), and it
  answers a server→client request while it drains.

### Task B — the stale evidence

- The invariant: a location is never presented as verified against the
  current source when that cannot be shown.
- What a client can **prove** is narrow but real: a file it read changed
  while the answer was produced. `mark_changed_sources` compares
  (a) the file the question was asked about against the text the server was
  given at `didOpen` time, and (b) each fact's target file against the
  digest the fact carries. A difference proves the fact is out of date with
  the live source, and the fact is marked `Verification::Stale` — the
  contract's existing state for exactly that — and the answer's own verdict
  becomes `Stale` too (`lsp_answer_verification`: the answer is as certain
  as its weakest fact).
- The latest digest is never presented as proof that the server's span is
  fresh: the digest is the file's content, the span is the server's claim,
  and the *verification* says which one it is.
- No name or text comparison: only whole-file digests are compared. A
  constructor, an overload, an operator, a template or an alias is never
  rejected on the grounds of what its text looks like.
- Valid results and unchanged source keep working: an unchanged file leaves
  the facts `Unverified` and the answer `Unverified`.

### The residual limit (reported, not papered over)

The pilot's own case — a location from an index that no longer matches a
file that changed **before** the session — is **not detectable from the
client**. The pilot measured that opening the target file with its current
bytes does not change clangd's answer, and no LSP request exposes the
server's view of a file's text. Such a fact therefore stays `Unverified`,
which is exactly what it is: an unverified claim. Closing it needs either a
contract field that says "the server's span was not checked against the
source" or a server that answers from the open document — both are owner
decisions, and neither is faked here.

## 3. Tests: RED/GREEN and the negative controls

The tests of the **first** round were written before the production change
and observed RED against the pre-change code; the RED column records what
each one catches if its change were reverted. The tests added in the
**review rounds** were written together with the fix they cover, and their
RED column says the same thing.

| Test | Proves | RED against the pre-change code |
|---|---|---|
| `lsp::tests::diagnostic_messages_never_carry_an_absolute_path` | the sanitizer redacts every absolute-path form and leaves URLs, protocol names and relative paths alone | yes: no redaction existed |
| `lsp::tests::mock_diagnostics_are_reported_with_severity_code_and_location` | severity, code, message and location survive | yes: only the notification name was kept |
| `lsp::tests::mock_diagnostics_never_fail_the_answer` | a warning is delivered and the answer is intact | yes: no warning was delivered |
| `lsp::tests::mock_diagnostics_outside_the_repository_are_dropped` | the inside diagnostics are delivered and the outside ones are not | partly: it is RED for the whole feature, and the positive assertion makes it non-vacuous. The containment guard is defence in depth: the canonicalization rejects the outside path as well |
| `lsp::tests::mock_diagnostics_are_bounded_and_carry_no_absolute_path` | the diagnostics are capped at 8 and carry no absolute path | yes: 1 entry before, 8 now, and the path was unredacted |
| `lsp::tests::mock_diagnostics_after_the_answer_are_still_delivered` | a diagnostic published after the response is not lost | yes: the drain did not exist |
| `lsp::tests::mock_empty_answer_with_an_error_stays_empty` | an empty answer with an error diagnostic stays empty and carries the diagnostic | yes: the diagnostic was dropped |
| `lsp::tests::a_target_file_that_changed_marks_only_its_fact_stale` | the second staleness branch marks only the changed file's fact | yes: the branch did not exist |
| `lsp::tests::mock_stale_input_marks_the_facts_stale` | a source that changed under the server is marked `Stale` | yes: the first branch did not exist |
| `lsp::tests::mock_unchanged_source_keeps_the_facts_unverified` | negative control: an unchanged source is **not** marked stale, and a fact stays `Heuristic`/`Unverified` | guard: it passes before and after, and catches a false positive |
| `lsp::tests::a_diagnostic_code_is_bounded_and_sanitized` | a code cannot carry a path nor grow without bound | yes |
| `lsp::tests::a_diagnostic_line_number_cannot_overflow` | an untrusted line number cannot panic the adapter | yes: `l + 1` overflowed in debug builds |
| `lsp::tests::a_broken_server_stream_does_not_leak_its_output` | a server that breaks the stream cannot put a path into the reason | yes |
| `lsp::tests::the_stderr_tail_never_carries_a_path` | the server's stderr is sanitized and the tail keeps its end | yes |
| `lsp::tests::a_server_request_after_the_answer_is_answered` | a server→client request that arrives after the answer is answered | yes: the drain did not answer it |
| `lsp::tests::a_stray_response_is_recorded_as_a_bounded_notice` | a response nobody asked for is recorded as a bounded, path-free notice | yes |
| `lsp::tests::a_failed_query_keeps_the_diagnostics_of_the_server` | a failing query keeps the diagnostics the server published | yes |
| `lsp::tests::mock_stderr_flood_never_deadlocks` (extended) | the server's stderr is captured and the adapter does not deadlock on it | partly: the path-free tail is covered by the white-box test above |
| `index::tests::lsp_hits_carry_the_facts_own_precision_and_verification` | a hit carries the fact's own precision and verification | yes: the hit hard-coded `Unverified` |
| `index::tests::an_lsp_answer_is_as_certain_as_its_weakest_fact` | the answer's verdict is the weakest fact's | yes: the summary was a constant |
| `index::tests::a_failed_query_reports_the_diagnostics_of_the_server` | the failure reason keeps the server's diagnostics | yes: the reason dropped them |
| `lsp::tests::a_path_with_a_space_leaves_nothing_sensitive` | a path with a space leaves no sensitive component, quoted or bare, on Windows or POSIX | yes: the old sanitizer redacted only the first chunk |
| `lsp::tests::a_path_inside_a_json_payload_is_redacted` | a path inside a JSON string value is redacted, in the debug line and in the message | yes: the old tokenizer glued the value to its key and missed it |
| `lsp::tests::the_lsp_debug_output_never_carries_a_path` | the debug line is bounded and carries no path | yes: the raw response was printed |
| `lsp::tests::a_path_behind_odd_punctuation_is_still_redacted` | a path behind any punctuation is redacted | yes |
| `lsp::tests::an_escaped_quote_does_not_end_the_section` | a path behind an escaped quote in a JSON value is redacted | yes |
| `lsp::tests::ordinary_prose_with_a_slash_survives` | a slash between two words, and an ordinary word ending in "file:", are not redacted | yes |
| `lsp::tests::an_unterminated_quote_with_a_multibyte_character_does_not_panic` | the sanitizer cannot panic on a multi-byte character at the end of a cut text | yes: the byte slicing could panic |
| `lsp::tests::the_stderr_tail_starts_at_a_line_boundary` | the stderr tail never starts inside a path | partly: it pins the swallow of the path, and the line alignment itself is exercised by the same test |
| `lsp::tests::ordinary_prose_with_a_slash_survives` | a slash between two words, a bare `//`, a relative `./` path and an ordinary word ending in "file:" are not redacted | yes |
| `lsp::tests::a_unc_path_is_still_redacted` | a real UNC path is still redacted | yes |
| `lsp::tests::an_odd_run_of_backslashes_escapes_the_quote` | a path behind an odd run of backslashes stays inside the value | yes |
| `lsp::tests::a_retryable_server_answer_is_retried` | the "not indexed yet" marker survives the sanitizing, so the answer is still retried | yes |
| `tests/semantic.rs::cli_lsp_live_query_gated` (extended) | end to end: the diagnostics reach the output, the answer is intact, a changed source is `Stale` in the answer itself, an empty answer is not a fallback | yes (gated: local evidence, not CI) |

The mock server (`crates/cli/examples/mock_lsp.rs`) grew three modes for
this: `diagnostics` (an error, a warning, a long message, a batch, and a
notification for a file outside the repository), `late-diagnostics` (the
diagnostics come after the answer) and `stale-input` (the queried file
changes while the server answers).

**Negative controls** (the checks that must NOT change): a missing server
still produces the fallback payload with exit 0; a malformed, crashing,
hanging or stdout-polluting server is still rejected, killed and reaped; a
server without `referencesProvider` still reports the capability as
unsupported; the unchanged-source answer is still `Unverified`/`Heuristic`.

## 4. The review rounds

Four independent review rounds were run by the reviewer subagent
(`opencode/deepseek-v4.1-flash-free#high`) on the whole diff, before any
commit.

### Round 1 — 27 findings, all addressed

| Severity | Finding | Action |
|---|---|---|
| HIGH | POSIX absolute paths were not redacted; the "no absolute path" test would fail on ubuntu/macOS | the sanitizer was rewritten to work on tokens; a unit test pins both directions |
| MEDIUM | a diagnostic published after the response was never delivered | `drain_notifications` + the `late-diagnostics` mock mode + a test |
| MEDIUM | the diagnostic code was neither bounded nor sanitized | the code uses the same sanitize-and-cap path |
| MEDIUM-LOW | the answer's summary said `Unverified` while the facts said `Stale` | the summary is derived from the facts; a unit test pins the rule |
| LOW | an untrusted line number could overflow (`l + 1`) | `checked_add(1)` + a test |
| LOW | the sanitizer corrupted URLs | the token-boundary rule |
| LOW×several | weak or missing tests (the target-file branch, the confinement, the empty answer) | the tests above |
| suggestions | the formatter formatted a whole notification before the cap; the notices were unbounded; the error path dropped the diagnostics; a root diagnostic produced an empty location | all four taken up |

### Round 2 — 8 findings, all addressed

| Severity | Finding | Action |
|---|---|---|
| MEDIUM | the sanitizer still missed `file:` URIs, paths glued to a flag (`-I/usr/include`) and paths after a scheme | the sanitizer was rewritten around tokens: a token carrying an absolute path is replaced whole; URLs are kept |
| MEDIUM-LOW | four channels bypassed the sanitizer and the caps: the server's error object, the echoed URI, the stderr tail, the stray-response notices | all four now go through the sanitizer and the caps; the URI is no longer quoted at all |
| MEDIUM-LOW | the fallback reason did not actually contain the diagnostics (they lived in a separate field) | `shutdown` returns the diagnostics with the notices, and `lsp_fallback_reason` keeps them in the reason |
| LOW | the drain did not answer a server→client request | the drain answers it, as `request()` does |
| MEDIUM | the summary-verification fix had no failing test | `an_lsp_answer_is_as_certain_as_its_weakest_fact`; the CLI test now asserts the answer's own field, not only the per-fact one |
| LOW | the drain window can still lose a late diagnostic; the test could be flaky | a `shortcut:` comment records the limit; the mock does its file work before it answers, so the notification follows the response immediately |
| LOW | weak or vacuous assertions | the outside-repository test now also asserts the inside diagnostics arrive; the cap is counted over the diagnostic lines; the target-file test proves only the changed file's fact is marked |
| INFO | the pilot report still described the old adapter | a dated note was added; the frozen raw evidence was not edited |

The findings that were **rejected**, with the reason: the reviewer's
suggestion to compare the text at a reported offset with the text at the
query position was rejected — it is exactly the name-comparison heuristic
the brief forbids, and it would reject destructors and other legitimate
symbols. The finding that the "empty answer + error diagnostic" unit test
costs 10 s was accepted as a fair price: the adapter's retry schedule is
production behaviour, and the test pins a requirement that would otherwise
be untested.

### Round 3 — 9 findings, all addressed

| Severity | Finding | Action |
|---|---|---|
| MEDIUM | the sanitizer still missed a `file:` URI inside a token (a markdown link) and a POSIX path glued to punctuation | the token delimiters now include the brackets, the backtick and the separators, and a token that *contains* `file:` is a path |
| MEDIUM-LOW | the broken-stream channel (the server's stdout) was not sanitized | the non-framing-stdout text and the "server broken" text are sanitized and bounded |
| MEDIUM-LOW | the server's error object was sanitized but not capped | it now goes through the same bounded helper |
| MEDIUM-LOW | four of the round-2 fixes had no test at all | the mock grew the modes to produce them (`fail-after-diagnostics`, `client-request`, `stray-response`, a path in the stderr flood) and four tests were added |
| LOW | a fact's path could fall back to an absolute path | a fact whose path cannot be made relative is now an error |
| LOW | over-redaction of `\\` and `//host`, and the space-in-path limit | the limits are documented in §2; the space-in-path limit is accepted, not hidden |
| LOW | the sanitizer worked on the whole (possibly megabyte) message | the raw text is cut before it is sanitized |
| LOW | several statements in this document were stronger than the code | they were corrected here (the "every piece of server text" claim, the "between the read and the answer" claim, the notice cap, the error-object cap, the RED claim) |
| LOW | the `late-diagnostics` mock still did file work between the answer and the notification | the outside file is prepared before the answer and passed in |

The reviewer's finding that the "256 entries" cap does not apply to two
other pushes (the warm-up timeout and the retry notice) was accepted as a
wording fix: both are bounded by the number of documents and retries, not
by 256.

### Round 4 — 8 findings, all addressed; the reviewer recommended **GO**

| Severity | Finding | Action |
|---|---|---|
| MEDIUM | capping the server's error object silently disabled the `-32801` retry (the marker was cut away) | the retry decision is now made on the **parsed** error value, and the marker is kept in the message |
| MEDIUM-LOW | the drain's "server broken" text and the "bad header line" text were not sanitized | both go through the bounded sanitizer |
| MEDIUM-LOW | the broken-stream test was vacuous (the mock's stdout carried no path) | the mock now prints a path and the test asserts it was replaced |
| MEDIUM-LOW | the stderr-path assertion was vacuous (the path was cut away by the tail) | the tail now keeps its **end**, and a white-box test covers the channel deterministically |
| LOW | a drive path inside a non-`file:` URL token survived | the drive check now runs before the URL exemption, and it only matches a *single* letter (so `http://host` is still a URL) |
| LOW | two more residual limits were undocumented (`\\…` over-redaction, the `~` prefix) | they are listed in §2 |
| LOW | the raw text was cut before the sanitizer only in some channels | every channel now cuts first |
| LOW | the document's own statements were stale | corrected (the review-round count, the two RED claims, the character bound) |

The reviewer's GO was explicit: "I found nothing blocking for the shipped
CLI path: no data loss, no crash, no unbounded allocation, no fabricated
evidence, and Task A/Task B behave as documented on the success path."

### Round 5 — the privacy corrections (owner brief, 2026-10-21)

Scope: privacy only, the functional work was not reopened. Two defects:
a path with a space kept its tail, and the `RILTRA_LSP_DEBUG` line printed
the raw server response.

| Severity | Finding | Action |
|---|---|---|
| HIGH | the tokenizer did not close a token at the closing quote, so a path inside a JSON string value was glued to its key and escaped every check | the token is now closed at the closing quote, and the path checks look **inside** the token, at a path boundary |
| MEDIUM | `token_content` sliced bytes, so an unterminated quoted token ending on a multi-byte character could panic | the trimming is on characters (`strip_prefix`/`strip_suffix`), never on bytes |
| MEDIUM-LOW | an unquoted absolute path deletes the words that follow it, so a clang-style message loses its text | accepted and documented in §2: the ambiguous part is removed rather than leaked |
| LOW | the stderr tail window could start in the middle of a path | the window now starts at a line boundary, and a partial first line is dropped |
| LOW | two assertions were vacuous (the broken-stream and stray-response tests) | the mock now puts the path where the assertion looks for it |
| LOW | the implementation record contradicted the code | corrected in §2 |

The new tests: `a_path_with_a_space_leaves_nothing_sensitive`,
`a_path_inside_a_json_payload_is_redacted`,
`the_lsp_debug_output_never_carries_a_path`,
`a_path_behind_odd_punctuation_is_still_redacted`,
`an_escaped_quote_does_not_end_the_section`,
`ordinary_prose_with_a_slash_survives`,
`an_unterminated_quote_with_a_multibyte_character_does_not_panic`,
`the_stderr_tail_starts_at_a_line_boundary`.

A second round on the same diff found three more leaks and one new
over-redaction; all were fixed: a path behind non-ASCII punctuation (the
boundary rule now ends a *word*, not a fixed set of ASCII characters), a path
behind an escaped quote in a JSON value (an escaped quote no longer closes
the section), a quoted `a / b` (a separator followed by a space is not a
path), and the `profile:` word (only `file://` is a URI scheme now).

A third round found four more: a bare `//` (the integer division of
ordinary prose) was treated as a UNC path, an odd run of backslashes did not
escape a quote, a relative `./` or `../` path was over-redacted, and the
stderr-window test did not exercise the window alignment. All were fixed, and
the residual limits are listed in §2. The reviewer's verdict: "GO on the
shipped path, conditional on two small pre-commit fixes" — both were made.

## 5. The freshness that can actually be guaranteed

| Property | Guaranteed? | Why |
|---|---|---|
| the location is inside the repository | **yes** | `paths::contains` on the canonicalized path; a fact whose path cannot be made relative is an error, never an absolute path |
| the range is a valid range of the file as it was read | **yes** | the range is converted in the same buffer the digest was computed from |
| the file did not change between the read and the freshness check | **yes** (now) | `mark_changed_sources` compares the digests; a difference is `Stale` |
| the server's span was checked against the current source | **no** | the client cannot see the server's index (measured) |
| the symbol the server meant is the symbol asked about | **no** | no identity proof; deliberately not attempted |
| the fact is current | **no** | it is `Unverified`, and `Stale` when a change was proven |

## 6. Validation
| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo test --locked` | 391 + 16 + 145 + 21 + 52 + 44 + 15 + 12 = **696 passed, 0 failed** |
| `cargo test --locked` with `RILTRA_TEST_LSP=1` | the gated end-to-end LSP test also passes (local only; CI does not set the gate) |
| `git diff --check` | clean |
| security scan (`security/scan.sh staged`, gitleaks 8.30.1) | no findings; one allowlist entry was added for a verified synthetic UNC fixture in the LSP unit test, in the same style as the existing entries |
| CI on the commit | `39488750755`: windows, ubuntu, macos-15 and security all green (the functional hardening) |
| CI on the privacy corrections | see the commit message of the privacy commit |

**No regression on the pilot's E1/E7 findings.** The pilot's own probe
table (17 probes over the fixture and MMSP) was re-run against the hardened
build: every answer is the same file and line as the pilot measured, and
the only expectation that does not hold is the E12 one — which is the
finding, not a regression. The hardening changes what the caller is told
about the answer (the diagnostics, the staleness), not what the answer is.

The CI matrix does not set `RILTRA_TEST_LSP`, so the end-to-end LSP test is
local evidence; the adapter-level tests are in CI. That is a known limit of
the existing test design, not of this change.

## 7. What the owner must decide

1. **The residual staleness.** A location from a server index that no
   longer matches a file that changed *before* the session is undetectable
   from the client. Closing it needs a contract field or a server that
   answers from the open document. The pilot proposes neither.
2. **The drain window.** A diagnostic published more than 100 ms after the
   answer is not delivered. The quiet period is a trade-off against the
   latency of every query.
3. **The M12-gate question is untouched**: D-M9-2 and G-SEM are unchanged by
   this work, and nothing here may be cited as a release-gate pass.

**STOP — awaiting an independent audit and OWNER GO.**
