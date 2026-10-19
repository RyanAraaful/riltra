// SPDX-License-Identifier: Apache-2.0
// CP7-A direct MCP stdio reference. Spawns the pinned riltra mcp server,
// initializes MCP, lists tools, calls all eleven tools on the frozen registry,
// then proves a domain error is structured and the server stays alive.
"use strict";
const fs = require("node:fs");
const path = require("node:path");
const crypto = require("node:crypto");
const { spawn } = require("node:child_process");

const REPO_ROOT = path.resolve(__dirname, "..", "..", "..", "..", "..");
const ROOT = path.join(REPO_ROOT, ".cp7a");
const HOME = path.join(ROOT, "home-template");
const PRISTINE = path.join(ROOT, "home-pristine");
const BIN = path.join(REPO_ROOT, "target", "debug", "riltra.exe");
const OUT = path.join(ROOT, "reference");
const MAIN = ["registry.json", "index\\riltra.db", "index\\other.db", "knowledge\\riltra.db", "knowledge\\other.db"];
const PROTOCOL = "2024-11-05";

const calls = [
  { key: "repo-list", tool: "riltra_repo_list", args: {}, golden: "repo-list.json" },
  { key: "repo-status", tool: "riltra_repo_status", args: { repo: "riltra" }, golden: "repo-status.json+index-status.json" },
  { key: "search-validate-id", tool: "riltra_code_search", args: { repo: "riltra", query: "validate_id" }, golden: "search-validate-id.json" },
  { key: "callers-validate-id", tool: "riltra_code_callers", args: { repo: "riltra", path: "crates/cli/src/knowledge.rs", line: 89, col: 1 }, golden: "callers-validate-id.json" },
  { key: "callees-validate-new", tool: "riltra_code_callees", args: { repo: "riltra", path: "crates/cli/src/knowledge.rs", line: 199, col: 1 }, golden: "callees-validate-new.json" },
  { key: "trace-validate-new-to-id", tool: "riltra_code_trace", args: { repo: "riltra", path: "crates/cli/src/knowledge.rs", line: 199, col: 1, to: "crates/cli/src/knowledge.rs:89:1" }, golden: "trace-validate-new-to-id.json" },
  { key: "impact-validate-id", tool: "riltra_code_impact", args: { repo: "riltra", path: "crates/cli/src/knowledge.rs", line: 89, col: 1 }, golden: "impact-validate-id.json" },
  { key: "potential-vs-calls", tool: "riltra_code_callers", args: { repo: "other", path: "c/main.c", line: 6, col: 1 }, golden: "callers-other-add.json" },
  { key: "knowledge-search-readonly", tool: "riltra_knowledge_search", args: { repo: "riltra", query: "read-only" }, golden: "knowledge-search-readonly.json" },
  { key: "knowledge-revision-r1", tool: "riltra_knowledge_show", args: { repo: "riltra", id: "cp7a-decision-health", revision: 1 }, golden: "knowledge-show-health-r1.json" },
  { key: "knowledge-revision-current", tool: "riltra_knowledge_show", args: { repo: "riltra", id: "cp7a-decision-health" }, golden: "knowledge-show-health.json" },
  { key: "knowledge-related", tool: "riltra_knowledge_related", args: { repo: "riltra", id: "cp7a-decision-health" }, golden: "knowledge-related-health.json" },
  { key: "knowledge-evidence-readonly", tool: "riltra_knowledge_show", args: { repo: "riltra", id: "cp7a-constraint-readonly" }, golden: "knowledge-show-readonly.json" },
  { key: "stale-evidence", tool: "riltra_knowledge_show", args: { repo: "other", id: "cp7a-other-stale" }, golden: "knowledge-show-stale.json" },
  { key: "missing-index-empty", tool: "riltra_code_search", args: { repo: "empty", query: "main" }, golden: "search-empty-main.json" },
  { key: "missing-knowledge-store-empty", tool: "riltra_knowledge_search", args: { repo: "empty", query: "anything" }, golden: "knowledge-search-empty.json" },
  { key: "health", tool: "riltra_health", args: {}, golden: "status.json+doctor.json" }
];

function hash(p) { return crypto.createHash("sha256").update(fs.readFileSync(p)).digest("hex").toUpperCase(); }
function writeJson(p, v) { fs.writeFileSync(p, JSON.stringify(v, null, 2) + "\n"); }
function rm(p) { try { fs.rmSync(p, { recursive: true, force: true }); } catch {} }
function resetHome() {
  for (const d of ["index", "knowledge"]) { rm(path.join(HOME, d)); fs.mkdirSync(path.join(HOME, d), { recursive: true }); }
  for (const rel of MAIN) fs.copyFileSync(path.join(PRISTINE, rel), path.join(HOME, rel));
}
function homeState() {
  const main = {};
  for (const rel of MAIN) { const p = path.join(HOME, rel); main[rel] = fs.existsSync(p) ? hash(p) : null; }
  return { main, empty_index: fs.existsSync(path.join(HOME, "index\\empty.db")), empty_knowledge: fs.existsSync(path.join(HOME, "knowledge\\empty.db")) };
}
function changed(a, b) {
  return !a || !b || MAIN.some((rel) => a.main[rel] !== b.main[rel]) || b.empty_index || b.empty_knowledge;
}
function now() { return Number(process.hrtime.bigint() / 1000000n); }

async function main() {
  fs.mkdirSync(OUT, { recursive: true });
  resetHome();
  const before = homeState();
  writeJson(path.join(OUT, "mcp-home-before.json"), before);
  const started = now();
  const child = spawn(BIN, ["mcp"], {
    cwd: REPO_ROOT,
    env: { ...process.env, RILTRA_HOME: HOME },
    stdio: ["pipe", "pipe", "pipe"],
    windowsHide: true
  });
  const transcript = [];
  const stderr = [];
  let buffer = "";
  let seq = 0;
  const pending = new Map();
  let exited = false;
  let exitInfo = null;
  child.stdout.setEncoding("utf8");
  child.stdout.on("data", (chunk) => {
    buffer += chunk;
    let i;
    while ((i = buffer.indexOf("\n")) >= 0) {
      const line = buffer.slice(0, i).trim();
      buffer = buffer.slice(i + 1);
      if (!line) continue;
      let msg;
      try { msg = JSON.parse(line); } catch { transcript.push({ dir: "recv-raw", line, at_ms: now() - started }); continue; }
      transcript.push({ dir: "recv", msg, at_ms: now() - started });
      if (msg.id !== undefined && pending.has(msg.id)) {
        const p = pending.get(msg.id);
        pending.delete(msg.id);
        p.resolve(msg);
      }
    }
  });
  child.stderr.setEncoding("utf8");
  child.stderr.on("data", (d) => stderr.push(String(d)));
  child.on("exit", (code, signal) => { exited = true; exitInfo = { code, signal }; for (const p of pending.values()) p.reject(new Error("server exited")); pending.clear(); });

  function request(method, params, timeoutMs = 60000) {
    const id = ++seq;
    const msg = { jsonrpc: "2.0", id, method };
    if (params !== undefined) msg.params = params;
    transcript.push({ dir: "send", msg, at_ms: now() - started });
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => { pending.delete(id); reject(new Error(`timeout ${method}`)); }, timeoutMs);
      pending.set(id, { resolve: (m) => { clearTimeout(timer); resolve(m); }, reject: (e) => { clearTimeout(timer); reject(e); } });
      child.stdin.write(JSON.stringify(msg) + "\n");
    });
  }
  function notify(method, params) {
    const msg = { jsonrpc: "2.0", method };
    if (params !== undefined) msg.params = params;
    transcript.push({ dir: "send", msg, at_ms: now() - started });
    child.stdin.write(JSON.stringify(msg) + "\n");
  }

  const init = await request("initialize", {
    protocolVersion: PROTOCOL,
    capabilities: {},
    clientInfo: { name: "cp7a-direct", version: "1.0.0" }
  });
  if (init.error) throw new Error("initialize: " + JSON.stringify(init.error));
  notify("notifications/initialized", {});
  const list = await request("tools/list", {});
  if (list.error) throw new Error("tools/list: " + JSON.stringify(list.error));
  const tools = (list.result && list.result.tools) || [];
  const toolNames = tools.map((t) => t.name);
  const expected = ["riltra_code_callees", "riltra_code_callers", "riltra_code_impact", "riltra_code_search", "riltra_code_trace", "riltra_health", "riltra_knowledge_related", "riltra_knowledge_search", "riltra_knowledge_show", "riltra_repo_list", "riltra_repo_status"];

  const results = [];
  for (const c of calls) {
    const t0 = now();
    const res = await request("tools/call", { name: c.tool, arguments: c.args }, 90000);
    const elapsed = now() - t0;
    let text = null, parsed = null, isError = null, error = res.error || null;
    if (res.result) {
      isError = Boolean(res.result.isError);
      const content = res.result.content || [];
      const first = content.find((x) => x && x.type === "text");
      text = first ? first.text : null;
      if (text) { try { parsed = JSON.parse(text); } catch {} }
    }
    results.push({ key: c.key, tool: c.tool, args: c.args, golden: c.golden, elapsed_ms: elapsed, isError, error, text, json: parsed, structuredContent: res.result ? res.result.structuredContent || null : null, result: res.result || null, rpc_error: res.error || null });
  }

  const invalidArgs = { repo: "does-not-exist", path: "crates/cli/src/knowledge.rs", line: 89, col: 1 };
  const inv0 = now();
  const invalidRes = await request("tools/call", { name: "riltra_code_callers", arguments: invalidArgs }, 90000);
  const invalid = {
    args: invalidArgs,
    elapsed_ms: now() - inv0,
    isError: invalidRes.result ? Boolean(invalidRes.result.isError) : null,
    rpc_error: invalidRes.error || null,
    result: invalidRes.result || null
  };
  if (invalidRes.result) {
    const first = (invalidRes.result.content || []).find((x) => x && x.type === "text");
    invalid.text = first ? first.text : null;
    if (invalid.text) { try { invalid.json = JSON.parse(invalid.text); } catch {} }
  }
  const healthAfter = await request("tools/call", { name: "riltra_health", arguments: {} }, 90000);
  let healthAfterParsed = null;
  if (healthAfter.result) {
    const first = (healthAfter.result.content || []).find((x) => x && x.type === "text");
    if (first && first.text) { try { healthAfterParsed = JSON.parse(first.text); } catch {} }
  }
  const alive = Boolean(healthAfter.result && !healthAfter.result.isError && healthAfterParsed && healthAfterParsed.status && healthAfterParsed.status.version);

  child.stdin.end();
  const exitWait = new Promise((resolve) => {
    if (exited) return resolve(exitInfo);
    const timer = setTimeout(() => { try { child.kill(); } catch {} resolve({ code: null, signal: "killed-after-timeout" }); }, 10000);
    child.on("exit", (code, signal) => { clearTimeout(timer); resolve({ code, signal }); });
  });
  const exit = await exitWait;
  const after = homeState();
  const total = now() - started;
  const meta = {
    schema: "cp7a-direct-mcp-v1",
    started_at: new Date().toISOString(),
    binary: BIN,
    binary_sha256: hash(BIN),
    protocol_requested: PROTOCOL,
    protocol_negotiated: init.result ? init.result.protocolVersion : null,
    server_info: init.result ? init.result.serverInfo : null,
    capabilities: init.result ? init.result.capabilities : null,
    tools_count: tools.length,
    tools: tools.map((t) => ({ name: t.name, description: t.description, inputSchema: t.inputSchema })),
    tools_exact_expected: toolNames.slice().sort().join(",") === expected.slice().sort().join(","),
    calls: results,
    invalid_call: invalid,
    health_after_invalid: { isError: healthAfter.result ? Boolean(healthAfter.result.isError) : null, json: healthAfterParsed },
    server_alive_after_error: alive,
    home_before: before,
    home_after: after,
    home_changed: changed(before, after),
    total_ms: total,
    exit,
    stderr,
    transcript
  };
  writeJson(path.join(OUT, "mcp-results.json"), meta);
  writeJson(path.join(OUT, "mcp-home-after.json"), after);
  writeJson(path.join(OUT, "mcp-transcript.json"), transcript);
  console.log(JSON.stringify({
    protocol: meta.protocol_negotiated, server: meta.server_info, tools: meta.tools_count, tools_exact: meta.tools_exact_expected,
    calls: results.map((r) => ({ key: r.key, tool: r.tool, isError: r.isError, error: r.rpc_error, elapsed_ms: r.elapsed_ms, has_json: Boolean(r.json) })),
    invalid: { isError: invalid.isError, rpc_error: invalid.rpc_error, text: invalid.text, json: invalid.json },
    health_after_invalid: { isError: healthAfter.result && healthAfter.result.isError, version: healthAfterParsed && healthAfterParsed.status && healthAfterParsed.status.version },
    server_alive_after_error: alive, home_changed: meta.home_changed, exit, stderr: stderr.join("").slice(0, 2000)
  }, null, 2));
}
main().catch((e) => { console.error(e && e.stack || e); process.exit(1); });


