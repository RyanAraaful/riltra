// SPDX-License-Identifier: Apache-2.0
// Summarize a `riltra scan` JSON document (read-only).
// Usage: node scan-summary.cjs <scan.json> <out.json>
"use strict";
const fs = require("node:fs");
const path = require("node:path");

const [file, outFile] = process.argv.slice(2);
const d = JSON.parse(fs.readFileSync(file, "utf8"));
const tally = (list) => list.reduce((m, k) => ((m[k] = (m[k] || 0) + 1), m), {});
const status = {};
const reason = {};
const extStatus = {};
const cuCuh = [];
const inl = [];
const processedKeys = new Set();
for (const f of d.files) {
  status[f.status] = (status[f.status] || 0) + 1;
  const k = f.status + ":" + (f.reason || "-");
  reason[k] = (reason[k] || 0) + 1;
  const e = path.extname(f.path).toLowerCase() || "(none)";
  extStatus[e] = extStatus[e] || {};
  extStatus[e][f.status] = (extStatus[e][f.status] || 0) + 1;
  if (/\.cuh?$/i.test(f.path)) cuCuh.push(f.status + ":" + (f.reason || "-"));
  if (f.path.endsWith(".inl")) inl.push(f.status + ":" + (f.reason || "-"));
  if (f.status === "processed") for (const key of Object.keys(f)) processedKeys.add(key);
}
const summary = {
  top_level_keys: Object.keys(d),
  repo: d.repo,
  files_discovered: d.files_discovered,
  listed: d.files.length,
  status,
  reason,
  processed_entry_keys: [...processedKeys].sort(),
  cu_cuh_listed: cuCuh.length,
  cu_cuh_outcomes: tally(cuCuh),
  inl_listed: inl.length,
  inl_outcomes: tally(inl),
  extensions_with_status: Object.fromEntries(Object.entries(extStatus).sort((a, b) => a[0].localeCompare(b[0]))),
  policy: d.policy,
  summary: d.summary ?? null,
};
fs.writeFileSync(outFile, JSON.stringify(summary, null, 2));
console.log(JSON.stringify({ status, reason, cu_cuh_outcomes: summary.cu_cuh_outcomes, inl_outcomes: summary.inl_outcomes }, null, 1));
