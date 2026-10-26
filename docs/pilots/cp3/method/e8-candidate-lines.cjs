"use strict";
// CP3 harness: map the added E8 candidate rows back to exact source lines.
//
// The edge rows carry byte spans into the INDEXED file, which is the working
// tree file. On Windows that file has CRLF endings, so a blob offset and a
// working-tree offset differ; this script resolves the span against the
// working tree and prints the exact line and its text.
//
// Usage:
//   node e8-candidate-lines.cjs <clone-root> <queries-dir>

const fs = require("node:fs");
const path = require("node:path");

const cloneRoot = process.argv[2];
const queriesDir = process.argv[3];
if (!cloneRoot || !queriesDir) {
  console.error("usage: node e8-candidate-lines.cjs <clone-root> <queries-dir>");
  process.exit(2);
}

const afterDir = path.join(queriesDir, "after");
const diff = JSON.parse(fs.readFileSync(path.join(queriesDir, "diff.json"), "utf8"));

const rows = [];
for (const q of diff.changed_queries) {
  const raw = fs.readFileSync(path.join(afterDir, q.query + ".json"), "utf8");
  const answer = JSON.parse(raw);
  for (const e of answer.edges || []) {
    const span = e.evidence && e.evidence.callee_span;
    if (!span) continue;
    rows.push({
      query: q.query,
      caller_path: e.caller_path,
      callee_name: e.callee_name,
      callee_span: span,
    });
  }
}

// The candidate call sites, deduplicated by (caller, span).
const seen = new Set();
const unique = rows.filter((r) => {
  const key = r.caller_path + "|" + r.callee_span.join(",");
  if (seen.has(key)) return false;
  seen.add(key);
  return true;
});

const out = [];
for (const r of unique) {
  const file = path.join(cloneRoot, r.caller_path.replace(/\//g, path.sep));
  let line = null;
  let text = null;
  try {
    const b = fs.readFileSync(file);
    const before = b.slice(0, r.callee_span[0]).toString("utf8");
    line = before.split("\n").length;
    const ls = before.lastIndexOf("\n") + 1;
    text = b.slice(ls, b.indexOf(10, ls)).toString("utf8").replace(/\r$/, "");
  } catch (e) {
    text = "UNREADABLE: " + e.message;
  }
  out.push({
    caller_path: r.caller_path,
    callee_name: r.callee_name,
    callee_span: r.callee_span,
    line,
    source_line: text,
    span_text: (() => {
      try {
        const b = fs.readFileSync(file);
        return b.slice(r.callee_span[0], r.callee_span[1]).toString("utf8");
      } catch (e) {
        return null;
      }
    })(),
  });
}

console.log(JSON.stringify({ clone_root: "<CLONE_ROOT>", candidates: out }, null, 2));
