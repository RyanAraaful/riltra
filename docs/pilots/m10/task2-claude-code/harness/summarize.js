// SPDX-License-Identifier: Apache-2.0
// Splits one Claude Code stream-json transcript into evidence files. Reads the transcript only.
// Usage: node summarize.js <transcript.jsonl> <outdir>
// Heuristics (also stated in the Task 2 README):
//   riltra_cli      Bash command whose first executable is riltra or riltra.exe (after VAR=value prefixes)
//   native_search   Grep, Glob, or Bash command whose first executable is grep, rg, or find, or git grep
//   riltra problem  tool error, empty result, or a "freshness":"stale" marker
const fs = require("fs");
const path = require("path");

const [src, outDir] = process.argv.slice(2);
fs.mkdirSync(outDir, { recursive: true });

const events = fs.readFileSync(src, "utf8").split("\n").filter(Boolean).map((line) => {
  try {
    return JSON.parse(line);
  } catch {
    return { type: "unparseable" };
  }
});
const init = events.find((e) => e.type === "system" && e.subtype === "init") ?? null;
const result = events.filter((e) => e.type === "result").pop() ?? null;

function executables(command) {
  return command.split(/&&|\|\||[;|\n]/).map((segment) => {
    const words = segment.trim().split(/\s+/).filter(Boolean);
    while (words.length && /^[A-Za-z_][A-Za-z0-9_]*=/.test(words[0])) words.shift();
    if (!words.length) return "";
    return path.basename(words[0].replace(/^[("']+|[)"']+$/g, "")).toLowerCase();
  });
}

function categorize(name, input) {
  if (name.startsWith("mcp__riltra__")) return "riltra_mcp";
  if (name === "Grep" || name === "Glob") return "native_search";
  if (name === "Read") return "native_read";
  if (name === "Edit" || name === "Write") return "edit";
  if (name !== "Bash") return "other";
  const command = String(input.command ?? "");
  const exes = executables(command);
  if (exes.some((x) => x === "riltra" || x === "riltra.exe")) return "riltra_cli";
  if (exes.some((x) => ["grep", "rg", "find"].includes(x)) || (exes.includes("git") && /\bgrep\b/.test(command))) {
    return "native_search";
  }
  return "shell";
}

const calls = [];
const byId = new Map();
for (const e of events) {
  const content = e.message?.content;
  if (!Array.isArray(content)) continue;
  for (const block of content) {
    if (e.type === "assistant" && block.type === "tool_use") {
      const call = { i: calls.length + 1, id: block.id, name: block.name, input: block.input ?? {}, is_error: false, text: "" };
      call.category = categorize(call.name, call.input);
      calls.push(call);
      byId.set(call.id, call);
    }
    if (e.type === "user" && block.type === "tool_result" && byId.has(block.tool_use_id)) {
      const call = byId.get(block.tool_use_id);
      call.is_error = block.is_error === true;
      call.text = typeof block.content === "string" ? block.content
        : (block.content ?? []).filter((b) => b.type === "text").map((b) => b.text).join("\n");
    }
  }
}

function riltraProblem(call) {
  if (call.is_error) return "error";
  if (call.text.trim() === "") return "empty";
  if (/"freshness":\s*"stale"/.test(call.text)) return "stale";
  return null;
}

const fallbacks = [];
let open = null; // last riltra call that had a problem, until a riltra call succeeds again
for (const call of calls) {
  if (call.category === "riltra_mcp" || call.category === "riltra_cli") {
    const problem = riltraProblem(call);
    open = problem ? { name: call.name, cause: problem } : null;
  } else if (call.category === "native_search" && open) {
    fallbacks.push({ i: call.i, tool: call.name, after: open.name, cause: open.cause });
  }
}

const countBy = (key) => calls.reduce((acc, c) => { acc[c[key]] = (acc[c[key]] ?? 0) + 1; return acc; }, {});
const firstSearch = calls.find((c) => ["riltra_mcp", "riltra_cli", "native_search"].includes(c.category));
const tools = {
  total: calls.length,
  by_category: countBy("category"),
  by_name: countBy("name"),
  first_search: firstSearch ? { i: firstSearch.i, name: firstSearch.name, category: firstSearch.category } : null,
  fallbacks,
  tool_errors: calls.filter((c) => c.is_error).length,
  calls: calls.map((c) => ({ i: c.i, name: c.name, category: c.category, input: JSON.stringify(c.input).slice(0, 300),
    is_error: c.is_error, result_chars: c.text.length })),
};

const answer = typeof result?.result === "string" ? result.result : "";
const brief = {
  init: init && {
    model: init.model, apiKeySource: init.apiKeySource, claude_code_version: init.claude_code_version,
    permissionMode: init.permissionMode, tools: init.tools, mcp_servers: init.mcp_servers,
    plugins: init.plugins, skills: init.skills, slash_commands: init.slash_commands,
  },
  result: result && {
    subtype: result.subtype, is_error: result.is_error, error_text: result.is_error ? answer.slice(0, 300) : null,
    num_turns: result.num_turns, duration_ms: result.duration_ms, duration_api_ms: result.duration_api_ms,
    total_cost_usd: result.total_cost_usd, usage: result.usage, modelUsage: result.modelUsage,
  },
  tools: { total: tools.total, by_category: tools.by_category, first_search: tools.first_search,
    fallbacks: fallbacks.length, tool_errors: tools.tool_errors },
};

fs.writeFileSync(path.join(outDir, "init.json"), JSON.stringify(init, null, 2) + "\n");
fs.writeFileSync(path.join(outDir, "result.json"), JSON.stringify(result, null, 2) + "\n");
fs.writeFileSync(path.join(outDir, "answer.txt"), answer);
fs.writeFileSync(path.join(outDir, "tools.json"), JSON.stringify(tools, null, 2) + "\n");
fs.writeFileSync(path.join(outDir, "summary.json"), JSON.stringify(brief, null, 2) + "\n");
console.log(JSON.stringify(brief));
