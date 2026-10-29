# E0-S: Rust semantic provider reliability

**Run date:** 2026-10-29 · **Baseline:** `master` at `7855a60` (merge of PR #16) · **Branch:** `investigate/e0s-rust-semantic-reliability` · **Evidence:** [`pilots/riltra-e0s-semantic-reliability-2026-10-29/`](pilots/riltra-e0s-semantic-reliability-2026-10-29/)

## Summary

- **Correctness: no defect found.** 102 of 103 riltra semantic queries returned complete, grounded answers (57 Rust, 46 TypeScript control). No answer was wrong, partial, or labelled `Exact` or `Current`.
- **Availability: one miss in 28 fresh-home Rust definition queries.** 27 of 28 succeeded, a rate of about 96%, with a 95% Wilson interval of [0.82, 0.99]. The miss is the first Rust definition in cold trial 1. Every later fresh-home definition succeeded: 27 in a row across the two remaining counted cold trials, five diagnostic replicates, and 20 further cold homes.
- **Rust references and warm queries:** 23 of 23 fresh-home references succeeded, and 6 of 6 warm queries succeeded.
- **Transparency: no silent failure.** The one empty answer carried the diagnostic `analysis wait timed out; answers may be empty`. There were no empty answers without a diagnostic, no fallbacks, and no errors.
- **Status: OPEN, narrowed.** The M9 open finding is not closed, because the cause of the single miss is not established. The interval's lower bound (0.82) is the figure for the owner to weigh against a reliability target.

## Protocol

The protocol follows the E0-S plan: the frozen M9 G-SEM fixture, three cold homes, three warm rounds, and the TypeScript SCIP control. The counted queries are the cold and warm trials. Diagnostic replicates, the 20 further cold homes, and readiness probes are reported separately, each under its own label.

- **Pre-registration.** [`method/expected.json`](pilots/riltra-e0s-semantic-reliability-2026-10-29/method/expected.json) (sha256 `a2bb6d6398836f83cc0c28b8bb36985de4bf83af2d591b81af7ea2a7c1641e1c`) was written at 09:58:27 UTC, before the first riltra query at 09:58:42 UTC. The first draft (sha256 `9c4ab10e…`) matched `greet` inside the module specifier `"./greet"` in `main.ts`. That draft was overwritten before any riltra query ran, and the TypeScript rule was corrected. No riltra output had been produced when the rule changed.
- **Fixtures.** `docs/pilots/m9-rc-ee7482d/gsem/fixture`, copied unchanged into each home. Each copy's bytes were checked against the frozen SHA-256 before the run.
- **Binaries and toolchain.** riltra debug build at `7855a60` (`riltra.exe`, sha256 `fb1616674cfa1f40079ea7de5f1d5540bbe2987aedbfc43451bfd5ca4173c987`). rust-analyzer 1.99.1 (`58cf8f9c 2026-10-13`, sha256 `af626a75…`). scip-typescript 0.4.0. Node v24.19.0. Windows 11 Pro 10.0.26300, 16 logical CPUs.
- **Constants under test.** Analysis-wait cap 15 s (`crates/cli/src/lsp.rs`, `await_analysis`), request timeout 30 s (`ServerSpec::new`), empty-answer retries 5 × 2 s.
- **Cold trial.** A fresh `RILTRA_HOME` and a fresh fixture copy with no `target/` directory. Then `init` and `index`, then Rust definition, then Rust references in the same home, then the TypeScript control (producer, import, references, definition from the call site).
- **Warm rounds.** Three rounds of Rust definition and references in cold trial 1's home, which by then has `target/`.
- **Classification.** [`method/e0s_run.py`](pilots/riltra-e0s-semantic-reliability-2026-10-29/method/e0s_run.py) assigns one status per query: `CORRECT-COMPLETE`, `PARTIAL`, `WRONG`, `EMPTY-DIAGNOSED`, `EMPTY-SILENT`, `FALLBACK`, `ERROR`, `TIMEOUT`. The frozen `check-gsem.ps1` byte check was run on every trial.

## Results (counted protocol)

| Phase | Query | n | Correct and complete | Empty, with diagnostic | Wrong or partial | Median ms (max) | Availability, Wilson 95% |
|---|---|---|---|---|---|---|---|
| cold | Rust definition | 3 | 2 | 1 | 0 | 4,641 (27,251) | 2/3, [0.21, 0.94] |
| cold | Rust references | 3 | 3 | 0 | 0 | 6,801 (18,111) | 3/3, [0.44, 1.00] |
| warm | Rust definition | 3 | 3 | 0 | 0 | 4,701 (4,723) | 3/3, [0.44, 1.00] |
| warm | Rust references | 3 | 3 | 0 | 0 | 6,682 (6,809) | 3/3, [0.44, 1.00] |
| cold | TypeScript references (control) | 3 | 3 | 0 | 0 | 25 (26) | 3/3 |
| cold | TypeScript definition (control) | 3 | 3 | 0 | 0 | 25 (28) | 3/3 |

The counted sample is three per row, so the intervals are wide. The TypeScript control uses a SCIP artifact, not a language-server query, so its results show the harness and the import path, not provider readiness.

Elapsed times are wall-clock, measured around each riltra process and include server start-up and shutdown.

The one miss is `cold-1`, Rust definition. It was the first language-server query in its home and the first run of the binary after it was rebuilt. The analysis wait reached its 15 s cap, and the query returned `definitions: []` with `analysis wait timed out; answers may be empty`. The total time was 27.3 s, which is the wait plus the empty-answer retries.

## Replication: 20 further cold homes

After the counted protocol, the owner asked for 20 more cold homes. Each home followed the same cold procedure (fresh home, fresh fixture, `init`, `index`, definition, references, TypeScript control). They were run with `method/e0s_run.py cold --prefix rep20 --phase rep20` and are reported separately.

| Query | n | Correct and complete | Availability, Wilson 95% | Median ms (max) |
|---|---|---|---|---|
| Rust definition | 20 | 20 | 20/20, [0.84, 1.00] | 4,681 (4,939) |
| Rust references | 20 | 20 | 20/20, [0.84, 1.00] | 6,644 (6,921) |
| TypeScript references (control) | 20 | 20 | 20/20, [0.84, 1.00] | 23 (27) |
| TypeScript definition (control) | 20 | 20 | 20/20, [0.84, 1.00] | 23 (27) |

No query in the replication hit the analysis-wait cap, and every answer was grounded to the expected bytes.

## Fresh-home availability, pooled

| Query | Homes | Succeeded | Wilson 95% |
|---|---|---|---|
| Rust definition, first query in a fresh home (counted cold, five replicates, 20 replication) | 28 | 27 | [0.82, 0.99] |
| Rust references, cold homes (counted cold, 20 replication) | 23 | 23 | [0.86, 1.00] |
| Rust definition and references, warm rounds | 6 | 6 | [0.61, 1.00] |

The only definition miss is `cold-1`.

## Diagnostics (not counted in the protocol)

These runs were made to locate the miss. They are reported separately and do not change the counted numbers.

- **Five fresh-home Rust definition replicates through riltra** (`method/e0s_run.py replicate`). All five returned the correct definition with no diagnostics, in 1.8 to 4.8 s.
- **Nineteen readiness probes** ([`method/readiness_probe.py`](pilots/riltra-e0s-semantic-reliability-2026-10-29/method/readiness_probe.py)). Each probe starts rust-analyzer the way the riltra client does (no arguments, empty client capabilities, `didOpen` of `src/main.rs`) and records when `publishDiagnostics` arrives for that file. Readiness was seen in 19 of 19 probes, between 2.20 s and 2.37 s. No probe reached the 15 s cap. The definition answers came back in 4.26 to 4.51 s.

## Interpretation

1. **Correctness.** When riltra returned answers, they were right. No answer was wrong, partial, or promoted to a stronger label. The correctness hard stops (fabricated spans, false `SAFE`, silent staleness) were not triggered.
2. **Reliability.** A first Rust definition in a fresh home can come back empty. In 28 such queries it happened once. The miss was reported, not silent, and the 27 fresh-home definitions after it all succeeded.
3. **What the miss looked like.** Cold trial 1 differs from the other trials in two measurements. Its first riltra process, `init`, took 961 ms against 17 to 19 ms in the other trials. Its references query took 18.1 s against 6.6 to 6.8 s. Both fit a slow first run on the host, with the provider's readiness signal arriving close to the 15 s cap. This is an inference, not a measured fact: riltra does not log when readiness arrived.
4. **What the evidence does not explain.** It does not establish why the miss happened. The 19 probes and the 27 later fresh-home definitions did not reproduce it. The data do not separate a host-level first-run slowdown from a readiness-matching problem in riltra for that run. In every probe the decoded URI matched the expected path. In every other riltra run the readiness signal was accepted, so the path comparison matched there too. That argues against a path-matching defect, but it does not rule one out for the single miss.
5. **Relation to the M9 finding.** The M9 release-candidate run (`ee7482d`) recorded 1 of 3 Rust definitions, including a warm rerun that also timed out. That run predates [PR #10](https://github.com/RyanAraaful/riltra/pull/10), which fixed a Windows readiness path mismatch that, according to its description, made every semantic query wait the full cap. It also predates [PR #13](https://github.com/RyanAraaful/riltra/pull/13), which answers server requests during warm-up. The warm rounds here passed 3 of 3, and the later fresh-home runs passed 27 of 27. These runs do not establish whether the fixes account for the M9 failures.

## Status and decision

- **E0-S reliability: OPEN, narrowed.** One miss in 28 fresh-home Rust definition queries, about 4%, cause unknown. The M9 open finding is not closed.
- **Not a correctness failure.** No defect in answer content was found in 103 queries.
- **No code change.** The E0-S plan (`docs/riltra-guide/03_BENCHMARK_STANDARD.md`, local only) rules out adding providers, caches or retries on the strength of one timeout. The evidence supports keeping the current behaviour.
- **Owner decision.** Whether 27 of 28 meets a reliability target is the owner's call. A product-grade target should be set from this baseline when the owner decides, not inferred from it here.

## Limits

- One miss. The rate estimate is imprecise, and the interval is wide at the low end.
- "Cold" means cold for riltra and rust-analyzer state: fresh `RILTRA_HOME`, fresh fixture, no `target/`. The operating-system page cache was not cleared, and the host is a single development Windows machine.
- The 20 replication homes ran in one session right after the counted set, so they share host state. A miss tied to host state would not show up within a short run.
- In each cold home the references query ran after the definition query, so it found `target/` present. Only the definition queries are cold in the strict sense.
- The riltra build is a debug build on one host. The readiness probes mirror the riltra client's sequence but do not share its retry logic.
- The TypeScript control is a deterministic SCIP import, not a language-server test.

## Next steps (owner decision)

1. **Decide the reliability baseline.** Accept 27 of 28 as the recorded baseline, or set a target and measure against it.
2. **Test the host-state hypothesis (optional).** Run one unrecorded throwaway riltra query before a cold set. This needs the owner's go-ahead, and the priority is lower now that 27 later homes were clean.
3. **Instrument readiness if the miss recurs.** A temporary, uncommitted build that logs the first `publishDiagnostics` time and URI during the wait would show whether readiness arrived late or never matched. This needs the owner's go-ahead.
4. The Rust SCIP producer work (remaining item 2 in [`riltra-what-impact-why-implementation.md`](riltra-what-impact-why-implementation.md)) is a separate track and was not started here.

## Redaction and verification

- Local paths in the evidence are replaced with `<SCRATCH>`, `<REPO>` and `<USERHOME>`. The redaction record is in [`results/redaction-accounting.json`](pilots/riltra-e0s-semantic-reliability-2026-10-29/results/redaction-accounting.json): 273 files, 269 replacements. A pattern scan of the evidence found no drive letters, user-profile paths, session paths or percent-encoded drive letters.
- File hashes for the whole evidence folder (311 files) are in `MANIFEST.sha256`.
- Secret and local-path scan with the pinned gitleaks 8.30.1, run through `security/scan.sh` (`GITLEAKS_BIN` set in the user's Claude Code settings):
  - `dir` on the evidence folder: no leaks (350.42 KB scanned).
  - `dir` on this document: no leaks.
  - `tree` on the committed HEAD: no leaks (24.10 MB scanned).
  - Scanner regression suite (`security/scan-tests.sh`): 32 passed, 0 failed.
- The evidence and this document are added in one commit on this branch. No tracked source file was changed.
