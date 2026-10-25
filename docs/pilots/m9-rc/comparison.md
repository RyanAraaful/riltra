# M9 release-candidate comparison: before vs after

Same host, same pinned snapshot, debug profile, the frozen CP2 harness. "Before" is the M9
tip `f65c183`, "after" the release-candidate code. Single-host figures: no claim beyond them.

## Cold index (two independent runs each)

| | before | after | change |
|---|---|---|---|
| run 1 wall | 98.987 s | 97.268 s | -1.7 % |
| run 2 wall | 99.253 s | 98.634 s | -0.6 % |
| mean wall | 99.12 s | 97.95 s | -1.2 % |
| database bytes | 50233344 | 55627776 | 10.7 % |

## Incremental refresh and the independent full rebuild

| step | before | after | change |
|---|---|---|---|
| refresh, nothing changed | 35.2 s | 36.7 s | 4.4 % |
| refresh, one file changed | 36.8 s | 36.6 s | -0.4 % |
| independent full rebuild | 98.2 s | 94.9 s | -3.3 % |

- before: delta mode `incremental`, reparsed 1, reused 1437
- after: delta mode `incremental`, reparsed 1, reused 1437

## Warm queries (5 repeats each, p50 / p95 in ms)

| query | before p50 | before p95 | after p50 | after p95 | p50 change |
|---|---|---|---|---|---|
| search-PatchMatch | 399.7 | 434.3 | 375.4 | 383.5 | -6.1 % |
| search-Run | 401 | 404.8 | 386.7 | 393.9 | -3.6 % |
| search-Print | 397.9 | 401 | 391.2 | 393.3 | -1.7 % |
| search-normalize_vec | 390 | 393.2 | 383 | 388 | -1.8 % |
| search-max-triang | 395.8 | 403.3 | 384.7 | 389.5 | -2.8 % |
| callers-PrintDef | 709.6 | 715 | 807.2 | 813.8 | 13.8 % |
| callees-PatchMatchRun | 684 | 687.1 | 695.1 | 709.2 | 1.6 % |
| callees-CtrlRun | 780.8 | 792 | 795.7 | 803.9 | 1.9 % |
| trace-depth4 | 699.7 | 706.4 | 717.8 | 718 | 2.6 % |
| impact-member | 590.8 | 593.4 | 587.1 | 592.1 | -0.6 % |
| index-status | 384.8 | 388.4 | 389.1 | 389.6 | 1.1 % |
| repo-status | 155.7 | 156.5 | 151.1 | 165.3 | -3.0 % |

## The five workflows, graded by the frozen grader

| check | layer | before | after |
|---|---|---|---|
| D1-H2 | honesty | PASS | PASS |
| D1-H1 | honesty | PASS | PASS |
| D1-H3 | honesty | PASS | PASS |
| D1-H4 | honesty | PASS | PASS |
| D1-H5 | honesty | PASS | PASS |
| D1-C6 | capability | UNMET (reference-set-empty-or-unresolved) | UNMET (reference-set-empty-or-unresolved) |
| D2-H1 | honesty | PASS | PASS |
| D2-H2 | honesty | PASS | FAIL (expected-callee-missing) **changed** |
| D2-H3 | honesty | PASS | PASS |
| D2-H4 | honesty | PASS | PASS |
| D2-H5 | honesty | UNAVAILABLE (no-reference-returned) | PASS **changed** |
| D2-C6 | capability | UNMET (construction-sites-not-attributed) | UNMET (construction-sites-not-attributed) |
| D3-H2 | honesty | PASS | PASS |
| D3-H3 | honesty | PASS | PASS |
| D3-C5 | capability | UNMET (trace-does-not-extend-beyond-the-first-hop) | UNMET (trace-does-not-extend-beyond-the-first-hop) |
| D3-H4 | honesty | PASS | PASS |
| D3-E1 | evidence | UNAVAILABLE (cuda-unsupported) | UNAVAILABLE (cuda-unsupported) |
| D4-H2 | honesty | PASS | PASS |
| D4-H3 | honesty | PASS | PASS |
| D4-H4 | honesty | PASS | PASS |
| D4-C5 | capability | UNMET (affected-set-empty) | UNMET (affected-set-empty) |
| D5-H1 | honesty | PASS | PASS |
| D5-H2 | honesty | PASS | PASS |
| D5-E1 | evidence | UNAVAILABLE (session-not-run) | UNAVAILABLE (session-not-run) |

Summary before: {"honesty":{"PASS":17,"UNAVAILABLE":1},"capability":{"UNMET":4},"evidence":{"UNAVAILABLE":2}}

Summary after: {"honesty":{"PASS":17,"FAIL":1},"capability":{"UNMET":4},"evidence":{"UNAVAILABLE":2}}

## What the workflow queries return

| query | before | after |
|---|---|---|
| d1-1-search-Print | 31 hits | 31 hits |
| d1-4-search-max_triangulation | 1 hits | 1 hits |
| d1-5-callers-PrintDef | 0 edges; anchor [5117,5568] | 3 edges (mvs.cc, patch_match.cc); anchor [5117,5568] |
| d1-6-callers-PrintDecl | 0 edges; anchor [2279,2830] | 0 edges; anchor [2279,2830] |
| d2-1-callers-PatchMatchCtrlRun | 1 edges (automatic_reconstruction.cc); anchor [4541,5597] | 1 edges (automatic_reconstruction.cc); anchor [4541,5597] |
| d2-3-callees-CtrlRun | 21 edges (patch_match.cc); anchor [9619,12963] | 21 edges (patch_match.cc); anchor [9619,12963] |
| d2-4-callers-CreateWorkspace | 3 edges (patch_match.cc); anchor [2368,3307] | 3 edges (patch_match.cc); anchor [2368,3307] |
| d2-5-callers-optmgr-binding | 0 edges; anchor [47492,50984] | 2 edges (option_manager.cc, mvs.cc); anchor [47492,50984] |
| d3-1-trace-depth4 | 21 paths; anchor [9619,12963] | 21 paths; anchor [9619,12963] |
| d4-1-impact-member | UNKNOWN, 0 paths; anchor [1696,5794] | UNKNOWN, 0 paths; anchor [2405,2428] |
| d4-2-impact-function | UNKNOWN, 0 paths; anchor [8344,8522] | UNKNOWN, 0 paths; anchor [8344,8522] |
| p2-callers-class-PatchMatch | 1 edges (automatic_reconstruction.cc); anchor [4541,5597] | 1 edges (automatic_reconstruction.cc); anchor [4541,5597] |
