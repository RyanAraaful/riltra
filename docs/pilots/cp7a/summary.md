# CP7-A OpenCode CLI vs MCP — frozen result

- Model: `opencode/deepseek-v4.1-flash-free` (free; no paid call)
- Source: `0fd22a1134274c1e92f741fc0d4e3b4886ceeaf3`
- Corpus: `D9F33A765A98723B0ACFD314865C976FB8BD58A680C911932B64C83E8530C824` (frozen `2026-10-18T18:05:37.264Z`)
- Token rule: `F14C339C76DC916D4E3E09C164AB8F28B2DD14844EA691902352720A24AD7C56`
- Runs: 32 measured sessions, 16 paired tasks, zero retries
- Backend revision: unverifiable from the client

## Paired tasks

| Task | Order | A correct | B correct | A evidence | B evidence | A gross | B gross | Delta | A tools | B tools | A turn ms | B turn ms |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| repo-list | A | PASS | PASS | PASS | PASS | 16453 | 14590 | -1863 | 2 | 1 | 11826 | 5052 |
| repo-status | B | PASS | PASS | PASS | PASS | 17327 | 15266 | -2061 | 4 | 2 | 18478 | 6788 |
| search-validate-id | A | PASS | PASS | PASS | PASS | 23063 | 14831 | -8232 | 3 | 1 | 16827 | 13393 |
| callers-validate-id | B | PASS | PASS | PASS | PASS | 24510 | 16204 | -8306 | 4 | 1 | 15169 | 6716 |
| callees-validate-new | A | PASS | PASS | PASS | PASS | 25065 | 17100 | -7965 | 3 | 1 | 13429 | 6689 |
| trace-validate-new-to-id | B | PASS | PASS | PASS | PASS | 23774 | 15876 | -7898 | 3 | 1 | 13400 | 6698 |
| impact-validate-id | A | PASS | PASS | PASS | PASS | 65665 | 15427 | -50238 | 16 | 1 | 47022 | 6685 |
| potential-vs-calls | B | PASS | FAIL | PASS | PASS | 17520 | 15460 | -2060 | 3 | 1 | 11730 | 8380 |
| knowledge-search-readonly | A | PASS | PASS | PASS | PASS | 30019 | 14828 | -15191 | 4 | 1 | 15157 | 5035 |
| knowledge-revision | B | PASS | PASS | PASS | PASS | 16917 | 15316 | -1601 | 3 | 2 | 8464 | 5035 |
| knowledge-related | A | PASS | PASS | PASS | PASS | 16711 | 14952 | -1759 | 2 | 1 | 8449 | 5004 |
| knowledge-evidence-readonly | B | PASS | PASS | PASS | PASS | 17234 | 14786 | -2448 | 2 | 1 | 10072 | 5034 |
| stale-evidence | A | PASS | PASS | PASS | PASS | 16869 | 14928 | -1941 | 3 | 1 | 11755 | 6697 |
| missing-index-empty | B | PASS | PASS | PASS | PASS | 29008 | 22426 | -6582 | 4 | 2 | 21782 | 8424 |
| missing-knowledge-store-empty | A | PASS | PASS | PASS | PASS | 29088 | 14681 | -14407 | 4 | 1 | 15172 | 5046 |
| health | B | PASS | PASS | PASS | PASS | 16932 | 14797 | -2135 | 5 | 1 | 8511 | 5025 |

## Aggregate

| Arm | Completed | Correct | Evidence | Gross total | Gross/completed | Upper/completed | Input | Output | Reasoning | Cache read | Tool-output input | Tool calls total/mean | Turn median ms | Create mean ms | Blocked |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| A (CLI) | 16/16 | 16/16 | 16/16 | 386155 | 24134.7 | 24483.6 | 50655 | 9612 | 5582 | 325888 | 46830 | 65/4.06 | 13414.2 | 186.4 | 2 |
| B (MCP) | 15/16 | 15/16 | 16/16 | 251468 | 16764.5 | 16810.4 | 20175 | 5757 | 688 | 225536 | 18558 | 19/1.19 | 6686.8 | 191.3 | 0 |

## Gates (measured)

- **G1: FAIL (frozen rubric).** A 16/16, B 15/16; A-only: potential-vs-calls; B-only: none.
  - Frozen-rubric false negative: B answer for potential-vs-calls states caller edges returned: 1; kind POTENTIAL_CALL; emitted as CALLS: No. It is substantively correct but the frozen regex (1|one) ... edge did not match the phrasing. No rubric or task was changed. Manual audit would make B 16/16; the frozen G1 result remains FAIL.
- **G2: PASS.** Evidence 16/16 A, 16/16 B; unresolved citations 0; ambiguous bare basenames 2.
  - All 32 answers have zero unresolved path/line/record citations. Two bare knowledge.rs references are ambiguous between crates/cli/src/knowledge.rs and crates/cli/tests/knowledge.rs; the cited anchors (199/89) and the tool output resolve to the source file. These are precision notes, not fabricated references.
- **G3: PASS.** A 24134.7 gross/completed (386155/16); B 16764.5 (251468/15); common completed pairs: B 15733.9 vs A 24575.7; upper-bound B 16810.4 vs A 24483.6; accounting reconciled.

## Startup / READY

| Arm | READY session | Kept tools | READY first input | READY cache read | READY output | READY gross | Attempts | Race failures | Startup gross | READY turn ms | Create ms | Prompt ms |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| A (CLI) | `ses_eafc36ab9ffevccMEdtoUOucuX` | 2 | 112 | 4352 | 46 | 4510 | 1 | 0 | 4510 | 3373.8 | 224.7 | 194.5 |
| B (MCP) | `ses_eafc3591dffezV3YE1tXYMS8NJ` | 12 | 20 | 7040 | 3 | 7063 | 1 | 0 | 7063 | 3358.6 | 201.9 | 190.4 |

## Evidence and limitations

- The 32 measured sessions all showed the exact expected tool exposure and unchanged five authoritative home DB/registry hashes; no `empty` index or knowledge store was created.
- The frozen `potential-vs-calls` B answer is correct but the required regex missed the phrasing. The frozen score remains FAIL for that task; G1 is reported as measured. No rubric was rewritten.
- Citation audit: zero unresolved paths/lines/records; two bare `knowledge.rs` references are precision notes, not fabrications.
- Tokens: provider-reported; `gross = input + cache.read + cache.write + output`; `reasoning` is reported separately and `gross_upper` adds it. Startup/READY and the scripted reference are excluded from task totals.
- G4–G8, G6 OpenCode error smoke, direct MCP/CLI reference, and the full G1–G8 report are in the CP7-A report; CP7-B Claude Code remains **BLOCKED** and G6 overall cannot pass.
- No M8 acceptance, master merge, tag, or release is claimed.
