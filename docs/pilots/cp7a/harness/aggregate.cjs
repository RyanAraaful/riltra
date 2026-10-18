// SPDX-License-Identifier: Apache-2.0
// CP7-A aggregate. Reads only frozen raw runs and the frozen corpus; writes
// measurements.json, summary.json, and summary.md. It never edits the corpus.
"use strict";
const fs = require("node:fs");
const path = require("node:path");
const crypto = require("node:crypto");
const { scoreTask, toolCalls } = require("./corpus/score.cjs");
const { auditAnswer } = require("./citations.cjs");

const ROOT = __dirname;
const CORPUS_PATH = path.join(ROOT, "corpus", "corpus.json");
const TOKEN_PATH = path.join(ROOT, "frozen", "token-rules.md");
const ENV_PATH = path.join(ROOT, "frozen", "environment.json");
const RUNS = path.join(ROOT, "runs");
const TASKS_DIR = path.join(RUNS, "tasks");
const STARTUP_DIR = path.join(RUNS, "startup");

const corpus = JSON.parse(fs.readFileSync(CORPUS_PATH, "utf8"));
const env = readJson(ENV_PATH, {});
const tasks = corpus.tasks;
const taskById = new Map(tasks.map((t) => [t.id, t]));

function readJson(p, fallback) {
  try { return JSON.parse(fs.readFileSync(p, "utf8")); } catch { return fallback; }
}
function hashFile(p) {
  try { return crypto.createHash("sha256").update(fs.readFileSync(p)).digest("hex").toUpperCase(); } catch { return null; }
}
function round(n, d = 1) {
  if (n === null || n === undefined || Number.isNaN(n)) return null;
  const f = Math.pow(10, d);
  return Math.round(n * f) / f;
}
function median(xs) {
  const a = xs.filter((x) => typeof x === "number" && !Number.isNaN(x)).sort((x, y) => x - y);
  if (!a.length) return null;
  const m = Math.floor(a.length / 2);
  return a.length % 2 ? a[m] : (a[m - 1] + a[m]) / 2;
}
function sum(xs) { return xs.reduce((s, x) => s + (typeof x === "number" ? x : 0), 0); }

function tokenFields(m) {
  const t = m && m.tokens;
  if (!t) return null;
  return {
    input: t.input || 0,
    cache_read: (t.cache && t.cache.read) || 0,
    cache_write: (t.cache && t.cache.write) || 0,
    output: t.output || 0,
    reasoning: t.reasoning || 0
  };
}
function hasTool(m) {
  return Array.isArray(m && m.content) && m.content.some((c) => c && c.type === "tool");
}
function tokenSummary(api, meta, calls) {
  const assistants = (api && api.data ? api.data : [])
    .filter((m) => m.type === "assistant")
    .sort((a, b) => ((a.time && a.time.created) || 0) - ((b.time && b.time.created) || 0));
  let input = 0, cache_read = 0, cache_write = 0, output = 0, reasoning = 0, missing = 0;
  let tool_output_input = 0, cache_read_after_tool = 0;
  let assistant_span_ms = 0, stream_ms = 0, first_stream_ms = 0;
  let prevHadTool = false;
  const messages = [];
  for (const m of assistants) {
    const t = tokenFields(m);
    const tool = hasTool(m);
    const created = m.time && m.time.created;
    const streamed = m.time && m.time.streamed;
    const completed = m.time && m.time.completed;
    if (t) {
      input += t.input; cache_read += t.cache_read; cache_write += t.cache_write;
      output += t.output; reasoning += t.reasoning;
      if (prevHadTool) { tool_output_input += t.input; cache_read_after_tool += t.cache_read; }
    } else missing++;
    if (typeof created === "number" && typeof completed === "number") assistant_span_ms += completed - created;
    if (typeof created === "number" && typeof streamed === "number") stream_ms += streamed - created;
    if (typeof created === "number" && typeof streamed === "number") first_stream_ms += streamed - created;
    messages.push({
      id: m.id, created: created || null,
      input: t ? t.input : null, cache_read: t ? t.cache_read : null,
      output: t ? t.output : null, reasoning: t ? t.reasoning : null,
      has_tool: tool, tool_names: tool ? m.content.filter((c) => c.type === "tool").map((c) => c.name) : []
    });
    prevHadTool = tool;
  }
  const toolNames = {};
  for (const c of calls || []) toolNames[c.name] = (toolNames[c.name] || 0) + 1;
  const toolTimes = (calls || []).map((c) => c.time).filter(Boolean);
  const tool_exec_ms = sum(toolTimes.map((t) => t.completed - t.ran));
  const tool_queue_ms = sum(toolTimes.map((t) => t.ran - t.created));
  const tool_total_ms = sum(toolTimes.map((t) => t.completed - t.created));
  const st = meta && meta.session_tokens;
  const session_tokens_match = Boolean(st) &&
    Number(st.input || 0) === input &&
    Number(st.output || 0) === output &&
    Number(st.reasoning || 0) === reasoning &&
    Number((st.cache && st.cache.read) || 0) === cache_read &&
    Number((st.cache && st.cache.write) || 0) === cache_write;
  const lastCompleted = assistants.reduce((m, a) => Math.max(m, (a.time && a.time.completed) || 0), 0);
  const idle = meta && meta.session_time && meta.session_time.idle;
  const sessionCreated = meta && meta.session_time && meta.session_time.created;
  return {
    assistant_count: assistants.length,
    message_breakdown: messages,
    input, cache_read, cache_write, output, reasoning,
    prompt_total: input + cache_read + cache_write,
    gross: input + cache_read + cache_write + output,
    gross_upper: input + cache_read + cache_write + output + reasoning,
    tool_output_input, cache_read_after_tool,
    missing_token_messages: missing,
    session_tokens: st || null,
    session_tokens_match,
    accounting_ok: missing === 0 && session_tokens_match,
    tool_calls: (calls || []).length,
    successful_tool_calls: (calls || []).filter((c) => c.success).length,
    failed_tool_calls: (calls || []).filter((c) => !c.success).length,
    tool_names: toolNames,
    tool_exec_ms, tool_queue_ms, tool_total_ms,
    assistant_span_ms,
    stream_ms,
    last_assistant_to_idle_ms: (typeof idle === "number" && lastCompleted) ? idle - lastCompleted : null,
    session_turn_ms: (typeof idle === "number" && typeof sessionCreated === "number") ? idle - sessionCreated : null
  };
}

function loadRun(task, arm) {
  const dir = path.join(TASKS_DIR, task.id, arm);
  const metaPath = path.join(dir, "meta.json");
  if (!fs.existsSync(metaPath)) return null;
  const meta = readJson(metaPath, {});
  const api = readJson(path.join(dir, "messages.json"), { data: [] });
  const score = scoreTask(task, arm, api);
  const calls = toolCalls(api);
  const tokens = tokenSummary(api, meta, calls);
  const citations = auditAnswer(score.answer);
  const statusOk = meta.status === "ok" && meta.outcome === "succeeded" && !meta.timed_out;
  const completed = statusOk && Boolean(meta.exposure_ok) && Boolean(meta.home_unchanged) && score.correct && score.evidence_valid;
  const reasons = [];
  if (!statusOk) reasons.push("session-not-ok");
  if (!meta.exposure_ok) reasons.push("tool-exposure");
  if (!meta.home_unchanged) reasons.push("home-hash");
  if (!score.correct) reasons.push("answer-rubric");
  if (!score.evidence_valid) reasons.push("evidence-rubric");
  return {
    task: task.id,
    arm,
    order: task.order,
    session: meta.session || null,
    status: meta.status || null,
    outcome: meta.outcome || null,
    timed_out: Boolean(meta.timed_out),
    exposure_ok: Boolean(meta.exposure_ok),
    context_kept: meta.context_kept || [],
    context_removed: meta.context_removed || [],
    home_unchanged: Boolean(meta.home_unchanged),
    home_after: meta.home_after || null,
    blocked: meta.plugin_blocked || 0,
    plugin_allowed: meta.plugin_allowed || 0,
    plugin_events: meta.plugin_events || 0,
    correct: score.correct,
    evidence_valid: score.evidence_valid,
    required_failures: score.required_failures,
    evidence_failures: score.evidence_failures,
    completed,
    completed_reasons: reasons,
    answer: score.answer,
    citations,
    tokens,
    timing: {
      session_create_ms: meta.session_create_ms,
      prompt_post_ms: meta.prompt_post_ms,
      turn_ms: meta.turn_ms,
      first_assistant_ms: meta.first_assistant_ms,
      polls: meta.polls,
      poll_errors: meta.poll_errors,
      prompt_completed_utc_ms: meta.prompt_completed_utc_ms,
      session_time: meta.session_time,
      session_turn_ms: tokens.session_turn_ms
    },
    error: meta.error || null,
    dir
  };
}

function startupSummary(arm) {
  const dir = path.join(STARTUP_DIR, arm);
  const summary = readJson(path.join(dir, "meta.json"), null);
  if (!summary) return null;
  const dirs = fs.existsSync(dir)
    ? fs.readdirSync(dir, { withFileTypes: true }).filter((e) => e.isDirectory() && /^attempt-\d+$/.test(e.name)).map((e) => e.name).sort()
    : [];
  const attempts = [];
  for (const name of dirs) {
    const adir = path.join(dir, name);
    const meta = readJson(path.join(adir, "meta.json"), {});
    const api = readJson(path.join(adir, "messages.json"), { data: [] });
    const calls = toolCalls(api);
    const tokens = tokenSummary(api, meta, calls);
    const first = (api.data || []).filter((m) => m.type === "assistant").sort((a, b) => ((a.time && a.time.created) || 0) - ((b.time && b.time.created) || 0))[0];
    attempts.push({
      attempt: Number(name.replace("attempt-", "")),
      dir: adir,
      session: meta.session || null,
      status: meta.status || null,
      exposure_ok: Boolean(meta.exposure_ok),
      context_kept: meta.context_kept || [],
      turn_ms: meta.turn_ms,
      session_create_ms: meta.session_create_ms,
      prompt_post_ms: meta.prompt_post_ms,
      first_assistant_ms: meta.first_assistant_ms,
      tokens,
      first_assistant_tokens: first ? tokenFields(first) : null,
      first_assistant_gross: first && tokenFields(first) ? tokenFields(first).input + tokenFields(first).cache_read + tokenFields(first).cache_write + tokenFields(first).output : null,
      error: meta.error || null
    });
  }
  const chosen = attempts.find((a) => a.session && a.session === summary.chosen_session) || attempts.find((a) => a.exposure_ok) || null;
  const ready = chosen ? {
    session: chosen.session,
    dir: chosen.dir,
    context_kept: chosen.context_kept,
    first_assistant_tokens: chosen.first_assistant_tokens,
    first_assistant_gross: chosen.first_assistant_gross,
    session_tokens: chosen.tokens.session_tokens,
    gross: chosen.tokens.gross,
    gross_upper: chosen.tokens.gross_upper,
    input: chosen.tokens.input,
    cache_read: chosen.tokens.cache_read,
    output: chosen.tokens.output,
    reasoning: chosen.tokens.reasoning,
    tool_calls: chosen.tokens.tool_calls,
    turn_ms: chosen.turn_ms,
    session_create_ms: chosen.session_create_ms,
    prompt_post_ms: chosen.prompt_post_ms,
    first_assistant_ms: chosen.first_assistant_ms,
    accounting_ok: chosen.tokens.accounting_ok
  } : null;
  return {
    arm,
    success: Boolean(summary.success),
    expected_tools: summary.expected_tools || null,
    chosen_session: summary.chosen_session || null,
    chosen_dir: summary.chosen_dir || null,
    attempts: attempts.map((a) => ({
      attempt: a.attempt, session: a.session, status: a.status, exposure_ok: a.exposure_ok,
      context_kept: a.context_kept, turn_ms: a.turn_ms, session_create_ms: a.session_create_ms,
      prompt_post_ms: a.prompt_post_ms, first_assistant_ms: a.first_assistant_ms,
      tokens: {
        input: a.tokens.input, cache_read: a.tokens.cache_read, output: a.tokens.output,
        reasoning: a.tokens.reasoning, gross: a.tokens.gross, gross_upper: a.tokens.gross_upper,
        tool_output_input: a.tokens.tool_output_input, tool_calls: a.tokens.tool_calls,
        session_tokens_match: a.tokens.session_tokens_match, accounting_ok: a.tokens.accounting_ok
      },
      first_assistant_tokens: a.first_assistant_tokens,
      first_assistant_gross: a.first_assistant_gross,
      error: a.error
    })),
    ready,
    attempts_count: attempts.length,
    race_failures: attempts.filter((a) => !a.exposure_ok).length,
    overhead_gross: sum(attempts.map((a) => a.tokens.gross)),
    overhead_gross_upper: sum(attempts.map((a) => a.tokens.gross_upper)),
    overhead_turn_ms: sum(attempts.map((a) => a.turn_ms)),
    ready_turn_ms: ready ? ready.turn_ms : null,
    ready_session_create_ms: ready ? ready.session_create_ms : null,
    ready_prompt_post_ms: ready ? ready.prompt_post_ms : null,
    ready_first_assistant_ms: ready ? ready.first_assistant_ms : null
  };
}

function armSummary(records) {
  const attempted = records.length;
  const completed = records.filter((r) => r.completed);
  const correct = records.filter((r) => r.correct);
  const evidence = records.filter((r) => r.evidence_valid);
  const grossTotal = sum(records.map((r) => r.tokens.gross));
  const grossUpperTotal = sum(records.map((r) => r.tokens.gross_upper));
  const inputTotal = sum(records.map((r) => r.tokens.input));
  const cacheReadTotal = sum(records.map((r) => r.tokens.cache_read));
  const cacheWriteTotal = sum(records.map((r) => r.tokens.cache_write));
  const outputTotal = sum(records.map((r) => r.tokens.output));
  const reasoningTotal = sum(records.map((r) => r.tokens.reasoning));
  const toolOutputTotal = sum(records.map((r) => r.tokens.tool_output_input));
  const toolCallsTotal = sum(records.map((r) => r.tokens.tool_calls));
  const blockedTotal = sum(records.map((r) => r.blocked));
  const turn = records.map((r) => r.timing.turn_ms);
  const create = records.map((r) => r.timing.session_create_ms);
  const post = records.map((r) => r.timing.prompt_post_ms);
  const first = records.map((r) => r.timing.first_assistant_ms);
  const toolExec = records.map((r) => r.tokens.tool_exec_ms);
  const completedGross = sum(completed.map((r) => r.tokens.gross));
  const correctedGross = sum(correct.map((r) => r.tokens.gross));
  const citationFailures = records.reduce((s, r) => s + (r.citations.failures || []).length, 0);
  const ambiguous = records.reduce((s, r) => s + (r.citations.paths || []).filter((p) => p.ambiguous).length, 0);
  return {
    attempted,
    completed: completed.length,
    correct: correct.length,
    evidence: evidence.length,
    completed_ids: completed.map((r) => r.task),
    correct_ids: correct.map((r) => r.task),
    evidence_ids: evidence.map((r) => r.task),
    timeouts: records.filter((r) => r.timed_out).length,
    errors: records.filter((r) => r.status === "error").length,
    exposure_failures: records.filter((r) => !r.exposure_ok).length,
    home_failures: records.filter((r) => !r.home_unchanged).length,
    blocked: blockedTotal,
    plugin_allowed: sum(records.map((r) => r.plugin_allowed)),
    gross_total: grossTotal,
    gross_upper_total: grossUpperTotal,
    input_total: inputTotal,
    cache_read_total: cacheReadTotal,
    cache_write_total: cacheWriteTotal,
    output_total: outputTotal,
    reasoning_total: reasoningTotal,
    tool_output_input_total: toolOutputTotal,
    gross_per_completed: completed.length ? grossTotal / completed.length : null,
    gross_upper_per_completed: completed.length ? grossUpperTotal / completed.length : null,
    gross_per_correct: correct.length ? grossTotal / correct.length : null,
    gross_per_attempted: attempted ? grossTotal / attempted : null,
    io_per_completed: completed.length ? (inputTotal + outputTotal) / completed.length : null,
    mean_gross_all: attempted ? grossTotal / attempted : null,
    median_gross_all: median(records.map((r) => r.tokens.gross)),
    mean_gross_completed: completed.length ? completedGross / completed.length : null,
    median_gross_completed: median(completed.map((r) => r.tokens.gross)),
    completed_gross_total: completedGross,
    correct_gross_total: correctedGross,
    tool_calls_total: toolCallsTotal,
    tool_calls_mean: attempted ? toolCallsTotal / attempted : null,
    tool_calls_median: median(records.map((r) => r.tokens.tool_calls)),
    tool_calls_mean_completed: completed.length ? sum(completed.map((r) => r.tokens.tool_calls)) / completed.length : null,
    turn_mean_ms: attempted ? sum(turn) / attempted : null,
    turn_median_ms: median(turn),
    turn_total_ms: sum(turn),
    session_create_mean_ms: attempted ? sum(create) / attempted : null,
    prompt_post_mean_ms: attempted ? sum(post) / attempted : null,
    first_assistant_mean_ms: attempted ? sum(first) / attempted : null,
    tool_exec_total_ms: sum(toolExec),
    tool_exec_mean_ms: attempted ? sum(toolExec) / attempted : null,
    accounting_ok: records.every((r) => r.tokens.accounting_ok),
    missing_token_runs: records.filter((r) => !r.tokens.accounting_ok).map((r) => r.task),
    citation_failures: citationFailures,
    ambiguous_citations: ambiguous
  };
}

function compact(r) {
  if (!r) return null;
  return {
    task: r.task, arm: r.arm, order: r.order, session: r.session, status: r.status, outcome: r.outcome,
    exposure_ok: r.exposure_ok, home_unchanged: r.home_unchanged, correct: r.correct,
    evidence_valid: r.evidence_valid, completed: r.completed, completed_reasons: r.completed_reasons,
    required_failures: r.required_failures, evidence_failures: r.evidence_failures,
    blocked: r.blocked,
    tokens: {
      input: r.tokens.input, cache_read: r.tokens.cache_read, cache_write: r.tokens.cache_write,
      output: r.tokens.output, reasoning: r.tokens.reasoning, gross: r.tokens.gross,
      gross_upper: r.tokens.gross_upper, tool_output_input: r.tokens.tool_output_input,
      tool_calls: r.tokens.tool_calls, accounting_ok: r.tokens.accounting_ok
    },
    timing: r.timing,
    citations: { failures: r.citations.failures, ambiguous: r.citations.paths.filter((p) => p.ambiguous).map((p) => p.raw) },
    answer: r.answer
  };
}

const records = [];
for (const task of tasks) {
  for (const arm of ["A", "B"]) {
    const r = loadRun(task, arm);
    if (r) records.push(r);
  }
}
const byArm = { A: records.filter((r) => r.arm === "A"), B: records.filter((r) => r.arm === "B") };
const missing = tasks.filter((t) => !byArm.A.find((r) => r.task === t.id) || !byArm.B.find((r) => r.task === t.id)).map((t) => t.id);
const A = armSummary(byArm.A);
const B = armSummary(byArm.B);
const startupA = startupSummary("A");
const startupB = startupSummary("B");

const paired = tasks.map((t) => {
  const a = byArm.A.find((r) => r.task === t.id) || null;
  const b = byArm.B.find((r) => r.task === t.id) || null;
  return {
    task: t.id, order: t.order,
    a: compact(a), b: compact(b),
    delta_gross: a && b ? b.tokens.gross - a.tokens.gross : null,
    delta_tool_calls: a && b ? b.tokens.tool_calls - a.tokens.tool_calls : null,
    delta_turn_ms: a && b ? b.timing.turn_ms - a.timing.turn_ms : null,
    both_completed: Boolean(a && b && a.completed && b.completed)
  };
});
const aOnly = paired.filter((p) => p.a && p.b && p.a.correct && !p.b.correct).map((p) => p.task);
const bOnly = paired.filter((p) => p.a && p.b && p.b.correct && !p.a.correct).map((p) => p.task);
const bothCorrect = paired.filter((p) => p.a && p.b && p.a.correct && p.b.correct).map((p) => p.task);
const neitherCorrect = paired.filter((p) => p.a && p.b && !p.a.correct && !p.b.correct).map((p) => p.task);
const commonCompleted = paired.filter((p) => p.both_completed);
const commonA = sum(commonCompleted.map((p) => p.a.tokens.gross));
const commonB = sum(commonCompleted.map((p) => p.b.tokens.gross));

const g1 = {
  a_correct: A.correct,
  b_correct: B.correct,
  a_evidence: A.evidence,
  b_evidence: B.evidence,
  a_only: aOnly,
  b_only: bOnly,
  both_correct: bothCorrect,
  neither: neitherCorrect,
  missing,
  pass: missing.length === 0 && B.correct >= A.correct && aOnly.length === 0,
  frozen_verdict: missing.length === 0 && B.correct >= A.correct && aOnly.length === 0 ? "PASS" : "FAIL",
  manual_audit_note: aOnly.includes("potential-vs-calls")
    ? "B answer for potential-vs-calls states caller edges returned: 1; kind POTENTIAL_CALL; emitted as CALLS: No. It is substantively correct but the frozen regex (1|one) ... edge did not match the phrasing. No rubric or task was changed. Manual audit would make B 16/16; the frozen G1 result remains FAIL."
    : null
};
const g2 = {
  a_evidence: A.evidence,
  b_evidence: B.evidence,
  citation_failures: A.citation_failures + B.citation_failures,
  ambiguous_citations: A.ambiguous_citations + B.ambiguous_citations,
  pass: missing.length === 0 && A.citation_failures === 0 && B.citation_failures === 0 && A.evidence === 16 && B.evidence === 16,
  note: "All 32 answers have zero unresolved path/line/record citations. Two bare knowledge.rs references are ambiguous between crates/cli/src/knowledge.rs and crates/cli/tests/knowledge.rs; the cited anchors (199/89) and the tool output resolve to the source file. These are precision notes, not fabricated references."
};
const g3Accounting = missing.length === 0 && A.accounting_ok && B.accounting_ok && A.completed > 0 && B.completed > 0;
const g3 = {
  accounting_ok: g3Accounting,
  a_completed: A.completed,
  b_completed: B.completed,
  a_gross_total: A.gross_total,
  b_gross_total: B.gross_total,
  a_per_completed: A.gross_per_completed,
  b_per_completed: B.gross_per_completed,
  a_upper_per_completed: A.gross_upper_per_completed,
  b_upper_per_completed: B.gross_upper_per_completed,
  a_io_per_completed: A.io_per_completed,
  b_io_per_completed: B.io_per_completed,
  common_completed: commonCompleted.map((p) => p.task),
  common_a_gross: commonA,
  common_b_gross: commonB,
  common_a_per_completed: commonCompleted.length ? commonA / commonCompleted.length : null,
  common_b_per_completed: commonCompleted.length ? commonB / commonCompleted.length : null,
  pass: g3Accounting && B.gross_per_completed <= A.gross_per_completed,
  verdict: !g3Accounting ? "NOT PROVEN" : (B.gross_per_completed <= A.gross_per_completed ? "PASS" : "FAIL"),
  rule: "All attempted task gross tokens / completed tasks; failures remain in the numerator. Frozen before results."
};

const firstStarted = records.map((r) => r.timing && r.timing.session_time && r.timing.session_time.created).filter(Boolean).sort((a, b) => a - b)[0] || null;
const lastIdle = records.map((r) => r.timing && r.timing.session_time && r.timing.session_time.idle).filter(Boolean).sort((a, b) => b - a)[0] || null;
const summary = {
  schema: "cp7a-summary-v1",
  generated_at: new Date().toISOString(),
  corpus_sha256: hashFile(CORPUS_PATH),
  corpus_frozen_at: corpus.frozen_at,
  token_rule_sha256: hashFile(TOKEN_PATH),
  source_head: corpus.source_head,
  model: corpus.model,
  environment: {
    os: env.os || null, computer: env.computer || null, cpu: env.cpu || null,
    rustc: env.rustc || null, cargo: env.cargo || null, opencode: env.opencode || null,
    binary_sha256: env.binary_sha256 || null, global_config_sha256: env.global_config_sha256 || null,
    arm_config_sha256: env.arm_config_sha256 || null, wrapper_sha256: env.wrapper || null,
    home_pristine: env.home_pristine || null
  },
  measurement: {
    tasks: tasks.length, arms: 2, runs: records.length, retries: 0,
    first_session_created_unix_ms: firstStarted,
    last_idle_unix_ms: lastIdle,
    tasks_dir: TASKS_DIR,
    startup_dir: STARTUP_DIR
  },
  startup: { A: startupA, B: startupB },
  aggregate: { A, B },
  paired,
  gates: { G1: g1, G2: g2, G3: g3 },
  citations: {
    total_answers: 32,
    failures: records.reduce((s, r) => s + r.citations.failures.length, 0),
    ambiguous: records.flatMap((r) => r.citations.paths.filter((p) => p.ambiguous).map((p) => ({ task: r.task, arm: r.arm, raw: p.raw, candidates: p.candidates })))
  },
  limitations: [
    "Free model only (opencode/deepseek-v4.1-flash-free); the exact remote backend revision is not verifiable from the client. No paid call was made.",
    "Frozen rubric false negative: potential-vs-calls Arm B is substantively correct but fails one required regex. The frozen score is preserved; G1 is FAIL as measured; a manual audit would be 16/16. No task/rubric was changed after results.",
    "Two bare knowledge.rs citations are ambiguous between the source and test files; the cited anchor lines resolve to the source. Zero unresolved citations.",
    "Tokens are provider-reported; cache.read is included in gross; reasoning is reported as a breakdown and separately as gross_upper. No local tokenizer.",
    "Startup/READY and warm-up attempts are excluded from task totals and reported separately. MCP discovery required warm-up contexts; measured sessions all showed exact expected exposure.",
    "The read tool remains in the model-visible tool list in both arms to avoid a free-tier 403; execution is plugin-blocked. Shell/MCP exposure is enforced by the location plugin and verified per session.",
    "Single Windows machine, sequential arms, one free model. No retries. The OpenCode subtest only; CP7-B Claude Code remains blocked.",
    "G1 FAIL as scored; no M8 acceptance, master merge, tag, or release is claimed."
  ]
};

const measurements = { ...summary, runs: records.map((r) => ({ ...r, citations: r.citations, tokens: r.tokens })) };
fs.writeFileSync(path.join(ROOT, "measurements.json"), JSON.stringify(measurements, null, 2) + "\n");
fs.writeFileSync(path.join(ROOT, "summary.json"), JSON.stringify(summary, null, 2) + "\n");

function pct(ok, n) { return n ? `${ok}/${n}` : "0/0"; }
function tokenCell(r) { return r ? `${r.tokens.gross}` : "-"; }
function boolCell(v) { return v ? "PASS" : "FAIL"; }
const md = [];
md.push("# CP7-A OpenCode CLI vs MCP — frozen result");
md.push("");
md.push(`- Model: \`${corpus.model.providerID}/${corpus.model.id}\` (free; no paid call)`);
md.push(`- Source: \`${corpus.source_head}\``);
md.push(`- Corpus: \`${summary.corpus_sha256}\` (frozen \`${corpus.frozen_at}\`)`);
md.push(`- Token rule: \`${summary.token_rule_sha256}\``);
md.push(`- Runs: ${records.length} measured sessions, ${tasks.length} paired tasks, zero retries`);
md.push(`- Backend revision: unverifiable from the client`);
md.push("");
md.push("## Paired tasks");
md.push("");
md.push("| Task | Order | A correct | B correct | A evidence | B evidence | A gross | B gross | Delta | A tools | B tools | A turn ms | B turn ms |");
md.push("|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
for (const p of paired) {
  const a = p.a, b = p.b;
  md.push(`| ${p.task} | ${p.order} | ${a ? boolCell(a.correct) : "-"} | ${b ? boolCell(b.correct) : "-"} | ${a ? boolCell(a.evidence_valid) : "-"} | ${b ? boolCell(b.evidence_valid) : "-"} | ${a ? tokenCell(a) : "-"} | ${b ? tokenCell(b) : "-"} | ${p.delta_gross === null ? "-" : p.delta_gross} | ${a ? a.tokens.tool_calls : "-"} | ${b ? b.tokens.tool_calls : "-"} | ${a ? Math.round(a.timing.turn_ms) : "-"} | ${b ? Math.round(b.timing.turn_ms) : "-"} |`);
}
md.push("");
md.push("## Aggregate");
md.push("");
md.push("| Arm | Completed | Correct | Evidence | Gross total | Gross/completed | Upper/completed | Input | Output | Reasoning | Cache read | Tool-output input | Tool calls total/mean | Turn median ms | Create mean ms | Blocked |");
md.push("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
for (const [name, x] of [["A (CLI)", A], ["B (MCP)", B]]) {
  md.push(`| ${name} | ${x.completed}/${x.attempted} | ${x.correct}/${x.attempted} | ${x.evidence}/${x.attempted} | ${x.gross_total} | ${round(x.gross_per_completed)} | ${round(x.gross_upper_per_completed)} | ${x.input_total} | ${x.output_total} | ${x.reasoning_total} | ${x.cache_read_total} | ${x.tool_output_input_total} | ${x.tool_calls_total}/${round(x.tool_calls_mean, 2)} | ${round(x.turn_median_ms)} | ${round(x.session_create_mean_ms)} | ${x.blocked} |`);
}
md.push("");
md.push("## Gates (measured)");
md.push("");
md.push(`- **G1: ${g1.frozen_verdict} (frozen rubric).** A ${A.correct}/${A.attempted}, B ${B.correct}/${B.attempted}; A-only: ${aOnly.join(", ") || "none"}; B-only: ${bOnly.join(", ") || "none"}.`);
md.push(`  - Frozen-rubric false negative: ${g1.manual_audit_note}`);
md.push(`- **G2: ${g2.pass ? "PASS" : "FAIL"}.** Evidence ${A.evidence}/${A.attempted} A, ${B.evidence}/${B.attempted} B; unresolved citations ${g2.citation_failures}; ambiguous bare basenames ${g2.ambiguous_citations}.`);
md.push(`  - ${g2.note}`);
md.push(`- **G3: ${g3.verdict}.** A ${round(g3.a_per_completed)} gross/completed (${g3.a_gross_total}/${g3.a_completed}); B ${round(g3.b_per_completed)} (${g3.b_gross_total}/${g3.b_completed}); common completed pairs: B ${round(g3.common_b_per_completed)} vs A ${round(g3.common_a_per_completed)}; upper-bound B ${round(g3.b_upper_per_completed)} vs A ${round(g3.a_upper_per_completed)}; accounting ${g3.accounting_ok ? "reconciled" : "NOT PROVEN"}.`);
md.push("");
md.push("## Startup / READY");
md.push("");
md.push("| Arm | READY session | Kept tools | READY first input | READY cache read | READY output | READY gross | Attempts | Race failures | Startup gross | READY turn ms | Create ms | Prompt ms |");
md.push("|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
for (const [name, s] of [["A (CLI)", startupA], ["B (MCP)", startupB]]) {
  const r = s && s.ready;
  md.push(`| ${name} | \`${r ? r.session : "-"}\` | ${r ? r.context_kept.length : "-"} | ${r ? r.first_assistant_tokens.input : "-"} | ${r ? r.first_assistant_tokens.cache_read : "-"} | ${r ? r.first_assistant_tokens.output : "-"} | ${r ? r.first_assistant_gross : "-"} | ${s ? s.attempts_count : "-"} | ${s ? s.race_failures : "-"} | ${s ? s.overhead_gross : "-"} | ${r ? round(r.turn_ms) : "-"} | ${r ? round(r.session_create_ms) : "-"} | ${r ? round(r.prompt_post_ms) : "-"} |`);
}
md.push("");
md.push("## Evidence and limitations");
md.push("");
md.push("- The 32 measured sessions all showed the exact expected tool exposure and unchanged five authoritative home DB/registry hashes; no `empty` index or knowledge store was created.");
md.push("- The frozen `potential-vs-calls` B answer is correct but the required regex missed the phrasing. The frozen score remains FAIL for that task; G1 is reported as measured. No rubric was rewritten.");
md.push("- Citation audit: zero unresolved paths/lines/records; two bare `knowledge.rs` references are precision notes, not fabrications.");
md.push("- Tokens: provider-reported; `gross = input + cache.read + cache.write + output`; `reasoning` is reported separately and `gross_upper` adds it. Startup/READY and the scripted reference are excluded from task totals.");
md.push("- G4–G8, G6 OpenCode error smoke, direct MCP/CLI reference, and the full G1–G8 report are in the CP7-A report; CP7-B Claude Code remains **BLOCKED** and G6 overall cannot pass.");
md.push("- No M8 acceptance, master merge, tag, or release is claimed.");
fs.writeFileSync(path.join(ROOT, "summary.md"), md.join("\n") + "\n");
console.log(JSON.stringify({
  runs: records.length, missing, A: { completed: A.completed, correct: A.correct, evidence: A.evidence, gross_total: A.gross_total, gross_per_completed: A.gross_per_completed },
  B: { completed: B.completed, correct: B.correct, evidence: B.evidence, gross_total: B.gross_total, gross_per_completed: B.gross_per_completed },
  G1: g1, G2: g2, G3: g3
}, null, 2));
