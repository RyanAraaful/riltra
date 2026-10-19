# CP7-A token counting rule (frozen before measured results)

Status: frozen at corpus hash `D9F33A765A98723B0ACFD314865C976FB8BD58A680C911932B64C83E8530C824`,
before any measured task beyond the already-recorded `repo-list` pilot.

## Unit

Use only the OpenCode session/provider-reported token fields on each assistant
message:

- `tokens.input` — new (uncached) prompt tokens;
- `tokens.cache.read` — prompt tokens read from the provider cache;
- `tokens.cache.write` — prompt tokens written to the provider cache;
- `tokens.output` — completion tokens reported by the provider;
- `tokens.reasoning` — provider-reported reasoning breakdown.

No local tokenizer, character heuristic, or content slice is used. The exact
remote backend revision is not verifiable from the client; this limitation is
disclosed in the report. The model is the free model
`opencode/deepseek-v4.1-flash-free`; no paid call is made.

## Derived quantities

- `prompt_total = input + cache.read + cache.write`
- `gross = prompt_total + output`
- `gross_upper = gross + reasoning`
- `completion = output`

`reasoning` is reported separately and is not added to the primary `gross`
because the provider reports it as a breakdown; `gross_upper` is the
conservative upper bound. Both are reported.

## Task attribution

Every assistant message in the measured task session is counted. Session
creation, prompt posting, polling, harness/API overhead, the READY warm-up, and
the scripted reference are excluded from task totals and reported separately.
There are zero model/task retries; if any infrastructure retry occurs it is
recorded and included in the numerator.

`tool_output_input` is the sum of `tokens.input` on assistant messages that
follow an assistant message containing a tool call, after sorting messages by
their `time.created`. It is the new uncached prompt content after a tool result.
`cache.read` on those messages is repeated context and is reported separately,
not attributed to tool output.

## READY baseline and startup overhead

The successful startup session per arm (`Reply READY. Do not call any tools.`)
is the READY baseline. Its first assistant call's token fields estimate the
fixed system/instruction/tool-schema context. All startup attempts, including
the attempts that raced MCP discovery, are reported as startup overhead and are
excluded from task totals. The direct MCP stdio initialize/tools-list handshake
is measured separately.

## Completed task and G3

A measured task is **completed** only when all of these hold:

1. the session reached idle with a succeeded outcome;
2. the first model-call plugin log exposed exactly the arm's expected tools;
3. the five authoritative `RILTRA_HOME` main files and the registry hash are
   unchanged, and no `empty` index/knowledge store was created;
4. the frozen answer rubric passes;
5. the frozen evidence rubric passes.

G3 primary is:

`MCP gross tokens per completed task <= CLI gross tokens per completed task`

where each arm's numerator is the sum of `gross` across **all attempted**
measured task sessions (failures and timeouts included; no cherry-picking) and
the denominator is that arm's count of completed tasks. If either denominator is
zero, or if any task session lacks provider token fields, or if the per-message
sum does not equal the session aggregate, G3 is **NOT PROVEN**.

Sensitivity only, never a substitute for the primary: common completed pairs
(both arms completed), mean/median per completed task, `gross_upper`, and the
`input + output` no-cache subtotal. G3 is not changed after seeing results.
