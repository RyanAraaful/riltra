// CP2 step 7a: direct MCP stdio smoke against the Riltra MCP server.
// Verifies: 12-tool surface, JSON-RPC stdout purity, classified error,
// non-mutating reads, transport health, repository selection.
//
// The binary and the paths come from the environment, so the script runs
// from any machine:
//   CP2_RILTRA  path to the riltra binary (required)
//   CP2_HOME    RILTRA_HOME of the pilot index (required)
//   CP2_OUT     output directory (required)
//   CP2_REPO    repository name (default mmsp-pin)
"use strict";
const { spawn } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");
const crypto = require("node:crypto");

const RILTRA = process.env.CP2_RILTRA;
const HOME = process.env.CP2_HOME;
const OUT = process.env.CP2_OUT;
const REPO = process.env.CP2_REPO || "mmsp-pin";
if (!RILTRA || !HOME || !OUT) {
  console.error("CP2_RILTRA, CP2_HOME and CP2_OUT are required");
  process.exit(2);
}
fs.mkdirSync(OUT, { recursive: true });

function homeDigest(dir) {
  const out = {};
  const walk = (d, base) => {
    for (const e of fs.readdirSync(d, { withFileTypes: true })) {
      const p = path.join(d, e.name);
      const rel = base ? base + "/" + e.name : e.name;
      if (e.isDirectory()) walk(p, rel);
      else {
        const h = crypto.createHash("sha256").update(fs.readFileSync(p)).digest("hex");
        out[rel] = { sha256: h, size: fs.statSync(p).size };
      }
    }
  };
  if (fs.existsSync(dir)) walk(dir, "");
  return out;
}

const before = homeDigest(HOME);
const proc = spawn(RILTRA, ["mcp"], {
  env: { ...process.env, RILTRA_HOME: HOME },
  stdio: ["pipe", "pipe", "pipe"],
});

const stdoutLines = [];
const stderrLines = [];
let buf = "";
proc.stdout.on("data", (d) => {
  buf += d.toString();
  let i;
  while ((i = buf.indexOf("\n")) >= 0) {
    stdoutLines.push(buf.slice(0, i));
    buf = buf.slice(i + 1);
  }
});
proc.stderr.on("data", (d) => stderrLines.push(d.toString()));

const send = (obj) => proc.stdin.write(JSON.stringify(obj) + "\n");
const wait = (ms) => new Promise((r) => setTimeout(r, ms));

(async () => {
  const report = { tool_list: null, calls: [], stdout_pure: null, errors: [], stderr: "" };

  // 1. initialize
  send({ jsonrpc: "2.0", id: 1, method: "initialize", params: {
    protocolVersion: "2025-06-18",
    capabilities: {},
    clientInfo: { name: "cp2-smoke", version: "1.0.0" },
  } });
  await wait(1200);
  send({ jsonrpc: "2.0", method: "notifications/initialized" });
  await wait(300);

  // 2. tools/list
  send({ jsonrpc: "2.0", id: 2, method: "tools/list", params: {} });
  await wait(2500);

  // 3. a successful source-grounded call
  send({ jsonrpc: "2.0", id: 3, method: "tools/call", params: {
    name: "riltra_code_search", arguments: { repo: REPO, query: "normalize_vec", limit: 5 },
  } });
  await wait(4000);

  // 4. repository selection
  send({ jsonrpc: "2.0", id: 4, method: "tools/call", params: {
    name: "riltra_repo_list", arguments: {},
  } });
  await wait(2500);

  // 5. classified error: unknown repository
  send({ jsonrpc: "2.0", id: 5, method: "tools/call", params: {
    name: "riltra_code_search", arguments: { repo: "does-not-exist", query: "Print" },
  } });
  await wait(2500);

  // 6. health
  send({ jsonrpc: "2.0", id: 6, method: "tools/call", params: {
    name: "riltra_health", arguments: {},
  } });
  await wait(3000);

  proc.stdin.end();
  await wait(800);
  try { proc.kill(); } catch (e) {}

  // --- analysis ---
  const parsed = [];
  let nonJson = 0;
  for (const line of stdoutLines) {
    if (line.trim() === "") continue;
    try { parsed.push(JSON.parse(line)); } catch (e) { nonJson++; }
  }
  report.stdout_pure = { lines: stdoutLines.filter((l) => l.trim() !== "").length, parsed: parsed.length, non_json: nonJson };
  report.stderr = stderrLines.join("");

  const toolList = parsed.find((m) => m.id === 2);
  report.tool_list = toolList && toolList.result ? toolList.result.tools.map((t) => t.name) : null;
  const callNames = { 3: "riltra_code_search", 4: "riltra_repo_list", 5: "riltra_code_search", 6: "riltra_health" };
  for (const id of [3, 4, 5, 6]) {
    const m = parsed.find((x) => x.id === id);
    const text = m && m.result && m.result.content ? m.result.content.map((c) => c.text || "").join("") : "";
    report.calls.push({
      id,
      name: callNames[id],
      present: !!m,
      is_error: m && m.result ? !!m.result.isError : null,
      text_len: text.length,
      text: text,
    });
  }
  const errCall = parsed.find((x) => x.id === 5);
  report.classified_error_text = errCall && errCall.result && errCall.result.content ? errCall.result.content.map((c) => c.text || "").join("") : null;

  const after = homeDigest(HOME);
  report.home_unchanged = JSON.stringify(before) === JSON.stringify(after);
  report.home_files = Object.keys(after).length;

  fs.writeFileSync(path.join(OUT, "mcp-smoke.json"), JSON.stringify(report, null, 2));
  fs.writeFileSync(path.join(OUT, "mcp-smoke-stdout.txt"), stdoutLines.join("\n"));
  fs.writeFileSync(path.join(OUT, "mcp-smoke-stderr.txt"), stderrLines.join("\n"));
  console.log(JSON.stringify({
    stdout_pure: report.stdout_pure,
    tool_count: report.tool_list ? report.tool_list.length : null,
    tools: report.tool_list,
    calls: report.calls,
    home_unchanged: report.home_unchanged,
    classified_error: (report.classified_error_text || "").slice(0, 400),
  }, null, 2));
})();
