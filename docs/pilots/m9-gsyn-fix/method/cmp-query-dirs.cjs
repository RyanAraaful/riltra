// SPDX-License-Identifier: Apache-2.0
// Compare two frozen-query output directories answer by answer (read-only).
// The bench-root path is replaced by a placeholder on both sides first, because the two
// runs use different bench roots; everything else goes through fact-digest.cjs compareOutputs.
// Usage: node cmp-query-dirs.cjs <dirA> <dirB> <methodDir> <benchA> <benchB>
"use strict";
const fs = require("node:fs");
const path = require("node:path");

const [dirA, dirB, methodDir, benchA, benchB] = process.argv.slice(2);
const { compareOutputs } = require(path.join(methodDir, "fact-digest.cjs"));
const summaryA = JSON.parse(fs.readFileSync(path.join(dirA, "_summary.json"), "utf8"));
const summaryB = JSON.parse(fs.readFileSync(path.join(dirB, "_summary.json"), "utf8"));

// Replace the bench root in both the plain and the JSON-escaped forms.
const escaped = (p) => p.split("\\").join("\\\\");
function scrub(text, bench) {
  return text.split(escaped(bench)).join("<BENCH>").split(bench).join("<BENCH>");
}

const result = { compared: 0, identical: 0, errors_compared: 0, errors_identical: 0, differing: [] };
for (const q of summaryA) {
  const b = summaryB.find((x) => x.name === q.name);
  const ta = scrub(fs.readFileSync(path.join(dirA, q.name + ".json"), "utf8"), benchA);
  const tb = scrub(fs.readFileSync(path.join(dirB, q.name + ".json"), "utf8"), benchB);
  if (q.exit === 0 && b.exit === 0) {
    result.compared++;
    const r = compareOutputs({ exitCode: 0, text: ta }, { exitCode: 0, text: tb });
    if (r.identical) result.identical++;
    else result.differing.push({ name: q.name, reason: r.reason });
  } else {
    result.errors_compared++;
    if (ta === tb) result.errors_identical++;
    else result.differing.push({ name: q.name, reason: "error-text-differs" });
  }
}
console.log(JSON.stringify(result));
