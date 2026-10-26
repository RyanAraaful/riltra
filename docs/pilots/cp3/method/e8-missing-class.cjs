"use strict";
// CP3 harness: classify the E8 "unmatched" includes.
//
// An include literal that names no indexed file is not necessarily a defect:
// the header may be absent from the repository (fetched by the build system),
// or present in the tree under an extension Riltra does not index. This
// probe separates the two classes against the real git tree.
//
// Usage:
//   node e8-missing-class.cjs <baseline.json> <git-ls-tree-file>

const fs = require("node:fs");

const baseline = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));
const tree = fs
  .readFileSync(process.argv[3], "utf8")
  .split(/\r?\n/)
  .map((l) => l.trim())
  .filter(Boolean);

const tracked = new Set(tree);
const bySuffix = new Map();
for (const p of tree) {
  const parts = p.split("/");
  for (let i = 0; i < parts.length; i++) {
    const key = parts.slice(i).join("/");
    if (!bySuffix.has(key)) bySuffix.set(key, []);
    bySuffix.get(key).push(p);
  }
}

const rows = baseline.samples.missing;
let absentFromRepo = 0;
let presentButNotIndexed = 0;
let inIndexedForm = 0;
const detail = [];

for (const r of rows) {
  const suffixHits = bySuffix.get(r.literal) || [];
  if (suffixHits.length === 0) {
    absentFromRepo++;
    detail.push({ ...r, class: "absent-from-repository", candidates: [] });
    continue;
  }
  // The literal names a tracked file. Is any tracked file with that literal
  // also an indexed file?
  const indexedHit = suffixHits.find((p) => baseline.indexed_paths.includes(p));
  if (indexedHit) {
    inIndexedForm++;
    detail.push({ ...r, class: "indexed", candidates: suffixHits });
  } else {
    presentButNotIndexed++;
    detail.push({
      ...r,
      class: "present-in-tree-not-indexed",
      candidates: suffixHits.slice(0, 4),
      extensions: [...new Set(suffixHits.map((p) => p.slice(p.lastIndexOf("."))))].slice(0, 6),
    });
  }
}

console.log(
  JSON.stringify(
    {
      tracked_files: tree.length,
      missing_include_rows: rows.length,
      absent_from_repository: absentFromRepo,
      present_in_tree_not_indexed: presentButNotIndexed,
      indexed: inIndexedForm,
      detail,
    },
    null,
    2
  )
);
