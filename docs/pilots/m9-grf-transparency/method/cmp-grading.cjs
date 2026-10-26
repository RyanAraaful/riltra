// SPDX-License-Identifier: Apache-2.0
// Compare two grading.json files check by check (read-only). Prints differences only.
// Usage: node cmp-grading.cjs <a.json> <b.json>
"use strict";
const fs = require("node:fs");
const [a, b] = process.argv.slice(2).map((f) => JSON.parse(fs.readFileSync(f, "utf8")));
const byId = (g) => new Map(g.checks.map((c) => [c.id, c]));
const A = byId(a), B = byId(b);
const diffs = [];
for (const id of new Set([...A.keys(), ...B.keys()])) {
  const x = A.get(id), y = B.get(id);
  if (!x || !y) { diffs.push({ id, note: "missing-on-one-side" }); continue; }
  const d = ["verdict", "reason"].filter((k) => x[k] !== y[k]).map((k) => ({ field: k, a: x[k], b: y[k] }));
  if (JSON.stringify(x.evidence) !== JSON.stringify(y.evidence)) {
    const ek = new Set([...Object.keys(x.evidence || {}), ...Object.keys(y.evidence || {})]);
    const ed = [...ek].filter((k) => JSON.stringify((x.evidence || {})[k]) !== JSON.stringify((y.evidence || {})[k]));
    d.push({ field: "evidence", keys: ed, a: ed.map((k) => (x.evidence || {})[k]), b: ed.map((k) => (y.evidence || {})[k]) });
  }
  if (d.length) diffs.push({ id, diffs: d });
}
console.log(JSON.stringify({ checks: [A.size, B.size], summary_equal: JSON.stringify(a.summary) === JSON.stringify(b.summary), differing: diffs }, null, 1));
