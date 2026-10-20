// Redact machine-local paths from CP2 raw evidence before it is archived in
// the public repository. Handles both plain and JSON-escaped separators.
"use strict";
const fs = require("node:fs");
const path = require("node:path");

const SRC = process.argv[2];
const DST = process.argv[3];

// Order matters: the bench root lives under the user profile, so it is
// replaced first. The separator is "one or more" so that JSON-escaped
// paths (four backslashes) are covered as well as plain ones.
const rules = [
  [/[A-Za-z]:[\\/]+Users[\\/]+[A-Za-z0-9._-]+[\\/]+AppData[\\/]+Local[\\/]+Temp[\\/]+opencode[\\/]+bench/gi, "<CP2_BENCH>"],
  [/[A-Za-z]:[\\/]+Users[\\/]+[A-Za-z0-9._-]+/gi, "<USERPROFILE>"],
  [/[A-Za-z]:[\\/]+dev[\\/]+riltra/gi, "<RILTRA_ROOT>"],
  [/[A-Za-z]:[\\/]+dev[\\/]+mmsp/gi, "<MMSP_ROOT>"],
  [/\/\/Users\/[A-Za-z0-9._-]+/g, "//<USERPROFILE>"],
  [/\/home\/[A-Za-z0-9._-]+/g, "/home/<USER>"],
];

function redact(text) {
  let out = text;
  for (const [re, to] of rules) out = out.replace(re, to);
  return out;
}

function walk(srcDir, dstDir) {
  fs.mkdirSync(dstDir, { recursive: true });
  for (const e of fs.readdirSync(srcDir, { withFileTypes: true })) {
    const s = path.join(srcDir, e.name);
    const d = path.join(dstDir, e.name);
    if (e.isDirectory()) { walk(s, d); continue; }
    let text;
    try { text = fs.readFileSync(s, "utf8"); } catch (e) { continue; }
    fs.writeFileSync(d, redact(text));
  }
}
walk(SRC, DST);
console.log("redacted archive written to " + DST);
