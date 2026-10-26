// Redact machine-local paths from CP2 raw evidence before it is archived in
// the public repository.
//
// Usage:
//   node redact.cjs <src-dir> <dst-dir> [<path>=<PLACEHOLDER> ...]
//
// The paths to redact are passed in, so the script carries no machine-local
// path of its own. Each mapping also replaces the JSON-escaped forms of the
// path (one and two levels of escaping), because the raw evidence contains
// the same path in plain and escaped shape. A generic rule covers the
// remaining user profile directories.
//
// SPDX-License-Identifier: Apache-2.0
"use strict";
const fs = require("node:fs");
const path = require("node:path");

const SRC = process.argv[2];
const DST = process.argv[3];
if (!SRC || !DST) {
  console.error("usage: node redact.cjs <src-dir> <dst-dir> [<path>=<PLACEHOLDER> ...]");
  process.exit(2);
}

const mappings = [];
for (const arg of process.argv.slice(4)) {
  const at = arg.indexOf("=");
  if (at < 0) continue;
  mappings.push({ from: arg.slice(0, at), to: arg.slice(at + 1) });
}

// Generic rules for whatever the mappings did not cover.
const generic = [
  [/[A-Za-z]:[\\/]+Users[\\/]+[A-Za-z0-9._-]+/gi, "<USERPROFILE>"],
  [/\/\/Users\/[A-Za-z0-9._-]+/g, "//<USERPROFILE>"],
  [/\/home\/[A-Za-z0-9._-]+/g, "/home/<USER>"],
];

// The plain path and its JSON-escaped forms: the raw evidence embeds the
// same path inside a JSON string, inside a JSON-RPC text field, so the
// separators can carry one to five backslashes.
function variants(p) {
  const out = [p];
  for (let n = 2; n <= 5; n++) {
    out.push(p.split("\\").join("\\".repeat(n)).split("/").join("\\".repeat(n)));
  }
  return out;
}

function redact(text) {
  let out = text;
  for (const m of mappings) {
    for (const v of variants(m.from).sort((a, b) => b.length - a.length)) {
      out = out.split(v).join(m.to);
    }
  }
  for (const [re, to] of generic) out = out.replace(re, to);
  return out;
}

function walk(srcDir, dstDir) {
  fs.mkdirSync(dstDir, { recursive: true });
  for (const e of fs.readdirSync(srcDir, { withFileTypes: true })) {
    const s = path.join(srcDir, e.name);
    const d = path.join(dstDir, e.name);
    if (e.isDirectory()) {
      walk(s, d);
      continue;
    }
    let text;
    try {
      text = fs.readFileSync(s, "utf8");
    } catch (err) {
      continue;
    }
    fs.writeFileSync(d, redact(text));
  }
}

walk(SRC, DST);
console.log("redacted archive written to " + DST);
