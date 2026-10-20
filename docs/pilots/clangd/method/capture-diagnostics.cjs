// SPDX-License-Identifier: Apache-2.0
// Capture what a language server actually publishes about a file.
//
// The Riltra LSP adapter records the NAME of the `textDocument/publishDiagnostics`
// notification and drops its payload (`crates/cli/src/lsp.rs`, note_notification).
// This harness-only client speaks the same protocol and keeps the payload,
// so the pilot can state what the server says about a file it could not
// read - and whether that is an error or a warning.
//
// Usage:
//   node capture-diagnostics.cjs --server <clangd.exe> --root <dir> --file <abs path> [--out <json>]
"use strict";
const { spawn } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

const args = process.argv.slice(2);
function arg(name, fallback) {
  const at = args.indexOf(name);
  return at >= 0 && args[at + 1] ? args[at + 1] : fallback;
}
const server = arg("--server");
const root = arg("--root");
const file = arg("--file");
const out = arg("--out", "");
if (!server || !root || !file) {
  console.error("usage: node capture-diagnostics.cjs --server <exe> --root <dir> --file <abs path> [--out <json>]");
  process.exit(2);
}

function fileUri(p) {
  const text = p.replace(/\\/g, "/");
  return "file:///" + text.replace(/^\//, "");
}

const child = spawn(server, [], { cwd: root, stdio: ["pipe", "pipe", "pipe"] });
let buffer = Buffer.alloc(0);
const notifications = [];
let stderr = "";

child.stderr.on("data", (d) => { stderr += d.toString(); });

function send(body) {
  const text = JSON.stringify(body);
  child.stdin.write(`Content-Length: ${Buffer.byteLength(text)}\r\n\r\n${text}`);
}

child.stdout.on("data", (chunk) => {
  buffer = Buffer.concat([buffer, chunk]);
  for (;;) {
    const split = buffer.indexOf("\r\n\r\n");
    if (split < 0) return;
    const header = buffer.slice(0, split).toString();
    const match = /content-length:\s*(\d+)/i.exec(header);
    if (!match) return;
    const length = Number(match[1]);
    if (buffer.length < split + 4 + length) return;
    const body = JSON.parse(buffer.slice(split + 4, split + 4 + length).toString());
    buffer = buffer.slice(split + 4 + length);
    if (body.method === "textDocument/publishDiagnostics") {
      notifications.push(body.params);
    }
  }
});

const text = fs.readFileSync(file, "utf8");
send({ jsonrpc: "2.0", id: 1, method: "initialize", params: {
  processId: process.pid, rootUri: fileUri(root), capabilities: {},
  workspaceFolders: [{ uri: fileUri(root), name: "root" }] } });
send({ jsonrpc: "2.0", method: "initialized", params: {} });
send({ jsonrpc: "2.0", method: "textDocument/didOpen", params: { textDocument: {
  uri: fileUri(file), languageId: "cpp", version: 1, text } } });

setTimeout(() => {
  // The stderr tail often carries absolute paths. They are not evidence, and
  // a path that survives redaction as a JSON-escaped Windows path trips the
  // repository's own secret scan, so the path-bearing lines are dropped.
  const kept = stderr
    .split(/\r?\n/)
    .filter((l) => l.trim() && !/[A-Za-z]:[\\/]{1,2}|\\\\[A-Za-z0-9._-]+\\[A-Za-z0-9$._ -]+/.test(l))
    .slice(-6);
  const report = { file, notifications, stderr_tail: kept.join("\n") };
  const json = JSON.stringify(report, null, 2);
  if (out) fs.writeFileSync(out, json);
  console.log(json);
  child.kill();
  process.exit(0);
}, Number(arg("--wait", "6000")));
