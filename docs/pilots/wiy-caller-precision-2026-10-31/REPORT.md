# Rust caller precision and recall: real SCIP producer

Date: 2026-10-31. Scope: one self-authored Windows fixture, rust-analyzer 1.99.1 (`58cf8f9c`), Riltra binary SHA-256 `1381afacf4593077a1def8ecf1bfcbdb906ad2574056d91ce56b8db0820a7ecb`, built from HEAD `1bb0f6ab6327ab5cb906c254983f2cea94e9f67f`. SCIP import reports `Unverified`, one document, 11 definitions and 9 references. The producer ran with `CARGO_NET_OFFLINE=true`, separate target directory, `rust-analyzer scip <source> --output <artifact> --exclude-vendored-libraries`. Query anchor is each `target` definition. Source and outputs are retained next to this report. This is Riltra plus real producer, not a measurement of rust-analyzer alone.

## Oracle and measurement

The [source](source/src/lib.rs) was authored before the producer and queries. The three call expressions in the defined universe invoke `a::target` directly, invoke it through local alias `f`, and invoke `b::target` directly. The alias initialization and the reference-only initialization are references to `a::target`, not call expressions. The two target definitions have the same name but distinct identities. This deliberately includes indirect alias calls in recall; the existing Riltra contract does not promise to resolve them. There was no independent review of this new oracle before the run; the source is retained for independent review.

| Source location | Expected identity / role | Observed |
|---|---|---|
| line 3 `a::target()` | call to `a` | `CALLS` to `a`, `Exact`, `Unverified` |
| line 4 `a::target` | reference to `a` | `REFERENCES` to `a` |
| line 4 `f()` | indirect call to `a` | no `CALLS` edge to `a` returned |
| line 5 `a::target` | reference to `a`, no call | `REFERENCES` to `a` |
| line 6 `b::target()` | call to `b` | `CALLS` to `b`, `Exact`, `Unverified` |

Among three annotated call sites and both targets: **TP=2, FP=0, FN=1**, precision **2/2 = 100%**, recall **2/3 = 66.7%**. FN is the indirect `f()` call, treated as a missed call for this explicitly defined oracle, not as a violation of the existing contract. The reference-only site was not promoted to a call, and the two same-name definitions were not confused in these results. Among four annotated target-name source occurrences, all four appeared under the correct symbol: **4/4 occurrence recall**, with no extra returned occurrence on these two queries (precision **4/4**). These quantities are not repository-wide metrics or statistically representative estimates.

The reported spans were checked by slicing the exact retained source bytes: two call spans equal `a::target()` and `b::target()`, and two reference spans equal `target`. Each returned edge was `Unverified`. Query output lists were not truncated. Index status was fresh; the source snapshot has no Git metadata. See [a results](results/a.json), [b results](results/b.json), and [import](results/import.json). Producer logs and SCIP artifact remain locally under the ignored run directory and can embed local paths; they are not published here. A complete producer diagnostic audit, separate independent rerun, wider corpus, and CI cross-platform verification were not done.
