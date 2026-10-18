// SPDX-License-Identifier: Apache-2.0
// CP7-A deterministic scorer. Reads a run's messages.json and meta.json, or
// exports scoreTask for the corpus self-test and the aggregate step.
"use strict";
const fs = require("node:fs");
const path = require("node:path");

function textOf(content) {
  if (typeof content === "string") return content;
  if (Array.isArray(content)) {
    return content
      .map((c) => (typeof c === "string" ? c : c && typeof c.text === "string" ? c.text : ""))
      .filter(Boolean)
      .join("\n");
  }
  if (content && typeof content.text === "string") return content.text;
  return "";
}

function answer(api) {
  const parts = [];
  for (const m of (api && api.data) || []) {
    if (m.type !== "assistant") continue;
    for (const c of m.content || []) {
      if (c.type === "text" && typeof c.text === "string") parts.push(c.text);
    }
  }
  return parts.join("\n");
}

function toolCalls(api) {
  const calls = [];
  for (const m of (api && api.data) || []) {
    if (m.type !== "assistant") continue;
    for (const c of m.content || []) {
      if (c.type !== "tool") continue;
      const state = c.state || {};
      const err = state.error;
      const status = state.status;
      const output = textOf(state.content ?? state.output ?? state.result);
      calls.push({
        name: c.name || "",
        input: state.input || {},
        output,
        status,
        error: err || null,
        success: !err && status !== "error" && status !== "failed",
        time: c.time || state.time || null
      });
    }
  }
  return calls;
}

function matchEvidence(call, ev) {
  if (!call.success) return false;
  try {
    if (!new RegExp(ev.tool, "i").test(call.name)) return false;
    if (ev.command) {
      const command = typeof call.input.command === "string" ? call.input.command : "";
      if (!new RegExp(ev.command, "i").test(command)) return false;
    }
    for (const p of ev.output || []) {
      if (!new RegExp(p, "i").test(call.output)) return false;
    }
    return true;
  } catch (e) {
    return false;
  }
}

function scoreTask(task, arm, api) {
  const text = answer(api);
  const calls = toolCalls(api);
  const required_failures = (task.required || []).filter((p) => {
    try { return !new RegExp(p, "i").test(text); } catch (e) { return true; }
  });
  const evidence_failures = [];
  (task.evidence[arm] || []).forEach((ev, i) => {
    if (!calls.some((c) => matchEvidence(c, ev))) evidence_failures.push(i);
  });
  return {
    task: task.id,
    arm,
    correct: required_failures.length === 0,
    evidence_valid: evidence_failures.length === 0,
    required_failures,
    evidence_failures,
    answer: text,
    tool_calls: calls.length,
    successful_tool_calls: calls.filter((c) => c.success).length,
    failed_tool_calls: calls.filter((c) => !c.success).length,
    calls
  };
}

function main() {
  const runDir = process.argv[2];
  if (!runDir) {
    console.error("usage: node score.cjs <run-dir>");
    process.exit(64);
  }
  const corpus = JSON.parse(fs.readFileSync(path.join(__dirname, "..", "corpus", "corpus.json"), "utf8"));
  const meta = JSON.parse(fs.readFileSync(path.join(runDir, "meta.json"), "utf8"));
  const api = JSON.parse(fs.readFileSync(path.join(runDir, "messages.json"), "utf8"));
  const task = corpus.tasks.find((t) => t.id === meta.task);
  if (!task) throw new Error(`unknown task ${meta.task}`);
  const score = scoreTask(task, meta.arm, api);
  fs.writeFileSync(path.join(runDir, "score.json"), JSON.stringify(score, null, 2) + "\n");
  console.log(JSON.stringify({ task: score.task, arm: score.arm, correct: score.correct, evidence_valid: score.evidence_valid, required_failures: score.required_failures, evidence_failures: score.evidence_failures, tool_calls: score.tool_calls }));
}

if (require.main === module) main();
module.exports = { scoreTask, toolCalls, answer, matchEvidence, textOf };
