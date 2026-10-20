// Order-independent fact digest of a Riltra index database.
//
// The digest covers the facts (files with their content digests, symbols,
// imports, calls with their spans and provenance) and the derived edge set.
// Rows are sorted before hashing, so the digest does not depend on the order
// SQLite happens to return them in (the ORDER BY of a non-total order is
// not stable between two builds).
"use strict";
const { DatabaseSync } = require("node:sqlite");
const crypto = require("node:crypto");

const flat = (row) =>
  Object.values(row)
    .map((v) => (v === null || v === undefined ? "" : String(v)))
    .join("\u0001");

function digestRows(rows) {
  const lines = rows.map(flat).sort();
  const h = crypto.createHash("sha256");
  for (const l of lines) h.update(l + "\n");
  return { rows: lines.length, digest: h.digest("hex") };
}

function factTables(dbPath) {
  const db = new DatabaseSync(dbPath, { readOnly: true });
  const all = (sql, ...a) => db.prepare(sql).all(...a);
  const snap = all("select * from snapshots order by id desc limit 1")[0];
  if (!snap) {
    db.close();
    return null;
  }
  const tables = {
    files: digestRows(all("select path, digest, bytes, language, errors from files where snapshot_id=?", snap.id)),
    symbols: digestRows(all("select f.path, s.kind, s.name, s.start_line, s.start_col, s.end_line, s.end_col, s.byte_start, s.byte_end, s.provider, s.grammar, s.language from symbols s join files f on f.id=s.file_id where s.snapshot_id=?", snap.id)),
    imports: digestRows(all("select f.path, i.start_line, i.start_col, i.end_line, i.end_col, i.byte_start, i.byte_end, i.text, i.provider, i.grammar, i.language from imports i join files f on f.id=i.file_id where i.snapshot_id=?", snap.id)),
    calls: digestRows(all("select f.path, c.start_line, c.start_col, c.end_line, c.end_col, c.byte_start, c.byte_end, c.name, c.unresolved, c.provider, c.grammar, c.language from calls c join files f on f.id=c.file_id where c.snapshot_id=?", snap.id)),
    edges: digestRows(all("select edge_kind, precision, provider, caller_path, caller_sbs, caller_sbe, callee_name, callee_kind, coalesce(unresolved,'') from code_edges where snapshot_id=?", snap.id)),
  };
  db.close();
  return tables;
}

function combinedDigest(tables) {
  const h = crypto.createHash("sha256");
  for (const k of Object.keys(tables).sort()) h.update(k + "=" + tables[k].digest + "\n");
  return h.digest("hex");
}

module.exports = { factTables, combinedDigest, digestRows };
