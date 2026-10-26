"use strict";
// CP3 harness: the frozen CP1 ground-truth anchors, resolved through the
// E8 rules, with the compiler-equivalent ground truth alongside.
//
// Usage:
//   node e8-anchor.cjs <db> [root1,root2,...]
const { DatabaseSync } = require("node:sqlite");

const db = new DatabaseSync(process.argv[2], { readOnly: true });
const roots = (process.argv[3] || "").split(",").map((r) => r.trim()).filter(Boolean);
const all = (sql, ...a) => db.prepare(sql).all(...a);
const snapshotId = all("SELECT MAX(id) AS id FROM snapshots")[0].id;
const files = all("SELECT path FROM files WHERE snapshot_id=?1 ORDER BY path", snapshotId).map((f) => f.path);
const byPath = new Set(files);

function quotedLiteral(text) {
  const m = /["']([^"']+)["']/.exec(text || "");
  return m ? m[1] : null;
}
function relativeToImporter(importer, literal) {
  const parts = importer.split("/").slice(0, -1);
  for (const part of literal.split("/")) {
    if (part === "." || part === "") continue;
    if (part === "..") { if (!parts.length) return null; parts.pop(); continue; }
    parts.push(part);
  }
  return parts.join("/");
}
function suffixHits(literal) {
  const needle = "/" + literal;
  return files.filter((f) => f === literal || f.endsWith(needle));
}
function compilerResolves(importer, literal) {
  const candidates = [relativeToImporter(importer, literal)];
  for (const root of roots) candidates.push(root + "/" + literal);
  for (const c of candidates) if (c && byPath.has(c)) return c;
  return null;
}

// The frozen CP1 anchors: the translation units CP1 verified by hand as the
// construction sites / call sites of the mvs/ symbols.
const FOCUS = [
  "colmap/src/colmap/exe/mvs.cc",
  "colmap/src/colmap/controllers/automatic_reconstruction.cc",
  "colmap/src/colmap/ui/dense_reconstruction_widget.cc",
  "colmap/src/colmap/mvs/patch_match.cc",
  "colmap/src/colmap/mvs/patch_match_cuda.cc",
  "colmap/src/colmap/mvs/patch_match_options.cc",
];

const rows = [];
for (const importer of FOCUS) {
  const imports = all(
    `SELECT i.text AS text FROM imports i JOIN files f ON f.id=i.file_id
      WHERE i.snapshot_id=?1 AND f.path=?2 ORDER BY i.byte_start`,
    snapshotId,
    importer
  );
  for (const im of imports) {
    const literal = quotedLiteral(im.text);
    if (!literal) continue;
    const relative = relativeToImporter(importer, literal);
    const hits = suffixHits(literal);
    rows.push({
      importer,
      literal,
      today_exact_rule: relative && byPath.has(relative) ? relative : null,
      suffix_unique: hits.length === 1 ? hits[0] : null,
      suffix_ambiguous: hits.length > 1 ? hits.length : null,
      compiler_opens: compilerResolves(importer, literal),
    });
  }
}

console.log(JSON.stringify({ snapshot_id: snapshotId, roots, rows }, null, 2));
db.close();
