// SPDX-License-Identifier: Apache-2.0
// Compare the import facts of two index databases of the same snapshot (read-only).
// Usage: node import-before-after.cjs <before.db> <after.db> <source-root> <out.json>
"use strict";
const { DatabaseSync } = require("node:sqlite");
const fs = require("node:fs");
const path = require("node:path");

const [beforeDb, afterDb, root, outFile] = process.argv.slice(2);
if (!beforeDb || !afterDb || !root || !outFile) {
  console.error("usage: node import-before-after.cjs <before.db> <after.db> <source-root> <out.json>");
  process.exit(2);
}

function load(dbPath) {
  const db = new DatabaseSync(dbPath, { readOnly: true });
  const snap = db.prepare("select * from snapshots order by id desc limit 1").get();
  const rows = db
    .prepare(
      "select f.path path, i.start_line sl, i.start_col sc, i.end_line el, i.end_col ec, " +
        "i.byte_start bs, i.byte_end be, i.text text, i.provider provider, i.grammar grammar, " +
        "i.language language from imports i join files f on f.id=i.file_id where i.snapshot_id=?"
    )
    .all(snap.id);
  db.close();
  return {
    meta: { parser_version: snap.parser_version, config_fingerprint: snap.config_fingerprint, head: snap.head },
    rows,
  };
}

const cache = new Map();
function lineOf(rel, off) {
  if (!cache.has(rel)) cache.set(rel, fs.readFileSync(path.join(root, rel)));
  const bytes = cache.get(rel);
  let line = 1;
  for (let i = 0; i < off; i++) if (bytes[i] === 0x0a) line++;
  return line;
}

const before = load(beforeDb);
const after = load(afterDb);
const key = (r) => `${r.path}|${r.bs}|${r.be}`;
const beforeMap = new Map(before.rows.map((r) => [key(r), r]));
const fields = ["sl", "sc", "ec", "text", "provider", "grammar", "language"];
const fieldDiff = Object.fromEntries(fields.map((f) => [f, 0]));
let matched = 0;
const endLineChanged = [];
let endLineMismatchBefore = 0;
let endLineMismatchAfter = 0;
let endLineChangedNotMultiline = 0;
let endLineChangedNotComputed = 0;
for (const r of after.rows) {
  const o = beforeMap.get(key(r));
  if (!o) continue;
  matched++;
  for (const f of fields) if (o[f] !== r[f]) fieldDiff[f]++;
  if (o.el !== r.el) {
    endLineChanged.push({ path: r.path, byte_start: r.bs, byte_end: r.be, before_end_line: o.el, after_end_line: r.el, start_line: r.sl, multi_line_text: String(r.text).includes("\n") });
    if (!String(r.text).includes("\n")) endLineChangedNotMultiline++;
    if (lineOf(r.path, r.be) !== r.el) endLineChangedNotComputed++;
  }
}
for (const r of before.rows) if (lineOf(r.path, r.be) !== r.el) endLineMismatchBefore++;
for (const r of after.rows) if (lineOf(r.path, r.be) !== r.el) endLineMismatchAfter++;

const out = {
  before_rows: before.rows.length,
  after_rows: after.rows.length,
  matched_by_path_and_bytes: matched,
  before_only: before.rows.length - matched,
  after_only: after.rows.length - matched,
  field_differences: fieldDiff,
  end_line_changed: endLineChanged.length,
  end_line_changed_not_multiline_text: endLineChangedNotMultiline,
  end_line_changed_not_equal_computed_line: endLineChangedNotComputed,
  end_line_mismatch_before: endLineMismatchBefore,
  end_line_mismatch_after: endLineMismatchAfter,
  meta_before: before.meta,
  meta_after: after.meta,
  changed_rows: endLineChanged,
};
fs.writeFileSync(outFile, JSON.stringify(out, null, 2));
console.log(JSON.stringify({ ...out, changed_rows: undefined }, null, 1));
