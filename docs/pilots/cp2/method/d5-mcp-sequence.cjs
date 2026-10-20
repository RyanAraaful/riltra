// CP2 step 7c: the D.5 tool sequence over the real MCP stdio transport.
// The natural-language question is interpreted by the agent; this script
// performs the tool calls the agent would make and archives the raw tool
// documents, so the answer can be checked against the CP1 ground truth.
"use strict";
const { spawn } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

const RILTRA = "<RILTRA_ROOT>\\target\\debug\\riltra.exe";
const HOME = process.env.CP2_HOME;
const OUT = process.env.CP2_OUT;
const REPO = "mmsp-pin";

const proc = spawn(RILTRA, ["mcp"], {
  env: { ...process.env, RILTRA_HOME: HOME },
  stdio: ["pipe", "pipe", "pipe"],
});

const outLines = [];
const pending = new Map();
let nextId = 1;
let buf = "";
proc.stdout.on("data", (d) => {
  buf += d.toString();
  let i;
  while ((i = buf.indexOf("\n")) >= 0) {
    const line = buf.slice(0, i);
    buf = buf.slice(i + 1);
    outLines.push(line);
    if (!line.trim()) continue;
    let msg;
    try { msg = JSON.parse(line); } catch (e) { continue; }
    if (msg && msg.id !== undefined && pending.has(msg.id)) {
      const resolve = pending.get(msg.id);
      pending.delete(msg.id);
      resolve(msg);
    }
  }
});
const stderr = [];
proc.stderr.on("data", (d) => stderr.push(d.toString()));

const send = (method, params) =>
  new Promise((resolve, reject) => {
    const id = nextId++;
    const timer = setTimeout(() => {
      pending.delete(id);
      reject(new Error("timeout waiting for " + method + " id=" + id));
    }, 30000);
    pending.set(id, (msg) => { clearTimeout(timer); resolve(msg); });
    proc.stdin.write(JSON.stringify({ jsonrpc: "2.0", id, method, params }) + "\n");
  });
const notify = (method, params) =>
  proc.stdin.write(JSON.stringify({ jsonrpc: "2.0", method, params }) + "\n");
const wait = (ms) => new Promise((r) => setTimeout(r, ms));

(async () => {
  const steps = [];
  try {
    await send("initialize", {
      protocolVersion: "2025-06-18", capabilities: {},
      clientInfo: { name: "cp2-d5", version: "1.0.0" },
    });
    notify("notifications/initialized", {});
    await wait(200);

    // The D.5 question: "how is the max_triangulation_angle option wired from
    // the command line into the source-image selection, and which files would I
    // need to touch to change its default?" -> the tool sequence an agent uses.
    const calls = [
      ["riltra_code_search", { repo: REPO, query: "max_triangulation_angle", limit: 10 }],
      ["riltra_code_search", { repo: REPO, query: "PatchMatchOptions", limit: 10 }],
      ["riltra_code_callers", { repo: REPO, path: "colmap/src/colmap/mvs/patch_match_options.h", line: 56, col: 10 }],
      ["riltra_code_callers", { repo: REPO, path: "colmap/src/colmap/controllers/option_manager.cc", line: 1019, col: 41 }],
      ["riltra_code_impact", { repo: REPO, path: "colmap/src/colmap/mvs/patch_match_options.h", line: 56, col: 10 }],
      ["riltra_code_callees", { repo: REPO, path: "colmap/src/colmap/mvs/patch_match.cc", line: 342, col: 28 }],
      ["riltra_repo_list", {}],
      ["riltra_repo_status", { repo: REPO }],
      ["riltra_code_callers", { repo: "does-not-exist", path: "colmap/src/colmap/mvs/patch_match_options.h", line: 56, col: 10 }],
    ];
    for (const [name, args] of calls) {
      const doc = await send("tools/call", { name, arguments: args });
      const text = doc && doc.result && doc.result.content
        ? doc.result.content.map((c) => c.text || "").join("")
        : JSON.stringify(doc);
      steps.push({ name, args, is_error: doc && doc.result ? !!doc.result.isError : null, text });
    }
  } catch (e) {
    steps.push({ name: "ERROR", is_error: true, text: String(e) });
  }
  proc.stdin.end();
  await wait(400);
  try { proc.kill(); } catch (e) {}

  fs.writeFileSync(path.join(OUT, "d5-tool-sequence.json"), JSON.stringify({ steps, stderr: stderr.join("") }, null, 2));
  fs.writeFileSync(path.join(OUT, "d5-mcp-stdout.txt"), outLines.join("\n"));
  console.log(JSON.stringify(steps.map((s) => ({ name: s.name, is_error: s.is_error, len: (s.text || "").length })), null, 2));
})();
