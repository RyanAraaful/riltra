"use strict";
// CP3 harness: the E8 before/after diff.
//
// Both runs answer the same query set against the SAME pilot index, so the
// diff isolates the E8 correction exactly. For every query the script
// reports the edges before and after and classifies the change.
//
// Usage:
//   node e8-diff.cjs <queries-dir>
//
// Output: JSON on stdout.

const fs = require("node:fs");
const path = require("node:path");

const root = process.argv[2];
if (!root) {
  console.error("usage: node e8-diff.cjs <queries-dir>");
  process.exit(2);
}

function readJson(file) {
  try {
    return JSON.parse(fs.readFileSync(file, "utf8"));
  } catch (err) {
    return null;
  }
}

function edgesOf(v) {
  if (!v) return [];
  const out = [];
  for (const e of v.edges || []) {
    out.push({
      kind: e.kind,
      callee_name: e.callee_name ?? null,
      caller_path: e.caller_path ?? null,
      precision: e.precision ?? null,
      provider: e.provider ?? null,
      unresolved: e.unresolved ?? null,
      callee_is_null: e.callee === null || e.callee === undefined,
    });
  }
  return out;
}

function keyOf(e) {
  return [e.kind, e.callee_name, e.caller_path, e.unresolved].join("|");
}

const beforeDir = path.join(root, "before");
const afterDir = path.join(root, "after");
const names = fs
  .readdirSync(beforeDir)
  .filter((f) => f.endsWith(".json") && f !== "_summary.json")
  .map((f) => f.slice(0, -5));

const report = [];
const promoted = [];
const totals = {
  queries: names.length,
  changed: 0,
  unchanged: 0,
  candidates_added: 0,
  candidates_removed: 0,
  calls_added: 0,
  references_added: 0,
  other_added: 0,
};

for (const name of names) {
  const b = readJson(path.join(beforeDir, name + ".json"));
  const a = readJson(path.join(afterDir, name + ".json"));
  const be = edgesOf(b);
  const ae = edgesOf(a);
  const bKeys = new Set(be.map(keyOf));
  const aKeys = new Set(ae.map(keyOf));
  const added = ae.filter((e) => !bKeys.has(keyOf(e)));
  const removed = be.filter((e) => !aKeys.has(keyOf(e)));

  const changed = added.length > 0 || removed.length > 0;
  if (changed) totals.changed++;
  else totals.unchanged++;

  for (const e of added) {
    if (e.kind === "POTENTIAL_CALL") totals.candidates_added++;
    else if (e.kind === "CALLS") {
      totals.calls_added++;
      promoted.push({ query: name, edge: e });
    } else if (e.kind === "REFERENCES") {
      totals.references_added++;
      promoted.push({ query: name, edge: e });
    } else totals.other_added++;
  }
  totals.candidates_removed += removed.length;

  if (changed) {
    report.push({
      query: name,
      before_edges: be.length,
      after_edges: ae.length,
      added,
      removed,
      before_completeness: b ? b.completeness ?? null : null,
      after_completeness: a ? a.completeness ?? null : null,
    });
  }
}

console.log(
  JSON.stringify(
    {
      queries_dir: root,
      totals,
      promoted_edges: promoted,
      changed_queries: report,
    },
    null,
    2
  )
);
