"use strict";
// Deterministic, order-independent fact digest of a Riltra index database.
const { DatabaseSync } = require("node:sqlite");
const crypto = require("node:crypto");
const fs = require("node:fs");

const dbPath = process.argv[2];
const outPath = process.argv[3] || "";
const flat = (row) => Object.values(row).map((v) => (v === null || v === undefined ? "" : String(v))).join("\u0001");

function digestRows(rows) {
  const lines = rows.map(flat).sort(); // order-independent
  const h = crypto.createHash("sha256");
  for (const l of lines) h.update(l + "\n");
  return { rows: lines.length, digest: h.digest("hex") };
}

function tables(p) {
  const db = new DatabaseSync(p, { readOnly: true });
  const all = (sql, ...a) => db.prepare(sql).all(...a);
  const snap = all("select * from snapshots order by id desc limit 1")[0];
  if (!snap) { db.close(); return null; }
  const out = {
    files: digestRows(all("select path, digest, bytes, language, errors from files where snapshot_id=?", snap.id)),
    symbols: digestRows(all("select f.path, s.kind, s.name, s.start_line, s.start_col, s.end_line, s.end_col, s.byte_start, s.byte_end, s.provider, s.grammar, s.language from symbols s join files f on f.id=s.file_id where s.snapshot_id=?", snap.id)),
    imports: digestRows(all("select f.path, i.start_line, i.start_col, i.end_line, i.end_col, i.byte_start, i.byte_end, i.text, i.provider, i.grammar, i.language from imports i join files f on f.id=i.file_id where i.snapshot_id=?", snap.id)),
    calls: digestRows(all("select f.path, c.start_line, c.start_col, c.end_line, c.end_col, c.byte_start, c.byte_end, c.name, c.unresolved, c.provider, c.grammar, c.language from calls c join files f on f.id=c.file_id where c.snapshot_id=?", snap.id)),
    edges: digestRows(all("select edge_kind, precision, provider, caller_path, caller_sbs, caller_sbe, callee_name, callee_kind, coalesce(unresolved,'') from code_edges where snapshot_id=?", snap.id)),
  };
  db.close();
  return out;
}

const t = tables(dbPath);
const combined = crypto.createHash("sha256");
for (const k of Object.keys(t).sort()) combined.update(k + "=" + t[k].digest + "\n");
const result = {
  file_bytes: fs.statSync(dbPath).size,
  file_sha256: crypto.createHash("sha256").update(fs.readFileSync(dbPath)).digest("hex"),
  tables: t,
  combined_digest: combined.digest("hex"),
};
if (outPath) fs.writeFileSync(outPath, JSON.stringify(result, null, 2));
console.log(JSON.stringify(result, null, 2));
