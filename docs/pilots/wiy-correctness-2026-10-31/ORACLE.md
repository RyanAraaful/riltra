# Rust trait dispatch oracle

This oracle was supplied by an independent reviewer before the query output was examined. Source: `trait-fixture/src/lib.rs`. The retained source is a readable reproduction; the measured Windows file used PowerShell UTF-8 output and CRLF, so its exact bytes and offsets differ. The source expressions and line numbers are the same.

| Source | Independently known fact | Permitted inference |
|---|---|---|
| Line 5, `x.run()` | `x` has type `&dyn Service`; the method call is dynamically dispatched. | The chosen implementation depends on the runtime value; do not state A or B is always invoked, or that the listed implementations exhaust all possible contexts. |
| Line 6, `Service::run(x)` | The argument has concrete type `&A`; the expression calls the trait method with an A receiver. | The implementation is A's in this fixture; a static edge still does not prove execution. |

The oracle does not pre-require a particular SCIP role or Riltra edge kind; the provider data and existing derivation contract determine whether resolution can be established. A syntax candidate must not be represented as an independently verified runtime call. Empty impact does not prove safety. Measured results and their scope are in [the evaluation report](../../riltra-wiy-correctness-evaluation-2026-10-31.md).
