// Read-only analysis of a Riltra index database (evidence collection, no writes).
"use strict";
const { DatabaseSync } = require("node:sqlite");
const path = require("node:path");
const crypto = require("node:crypto");

const dbPath = process.argv[2];
const mode = process.argv[3] || "facts";
const db = new DatabaseSync(dbPath, { readOnly: true });
const all = (sql, ...args) => db.prepare(sql).all(...args);
const TP = (p) => (p && p.startsWith("colmap/src/thirdparty/")) ? "third-party" : "first-party";

if (mode === "facts") {
  const snap = all("select * from snapshots order by id desc limit 1")[0];
  console.log("snapshot id=" + snap.id + " head=" + snap.head + " dirty=" + snap.dirty + " status_at_write=" + snap.status_at_write);
  const c = (t) => all(`select count(*) c from ${t} where snapshot_id=?`, snap.id)[0].c;
  console.log(`files=${c("files")} symbols=${c("symbols")} imports=${c("imports")} calls=${c("calls")} code_edges=${c("code_edges")}`);
  const byLang = all("select language, count(*) c, sum(errors) e from files where snapshot_id=? group by language order by c desc", snap.id);
  console.log("files by language: " + JSON.stringify(byLang));
  const byEdge = all("select edge_kind, precision, provider, count(*) c from code_edges where snapshot_id=? group by edge_kind, precision, provider", snap.id);
  console.log("edges: " + JSON.stringify(byEdge));
  const byUnres = all("select coalesce(unresolved,'(null)') u, count(*) c from code_edges where snapshot_id=? group by u order by c desc limit 8", snap.id);
  console.log("edges by unresolved: " + JSON.stringify(byUnres));
  const callsUnres = all("select unresolved, count(*) c from calls where snapshot_id=? group by unresolved order by c desc", snap.id);
  console.log("calls by unresolved flag: " + JSON.stringify(callsUnres));
  const byProv = all("select provider, grammar, language, count(*) c from symbols where snapshot_id=? group by provider, grammar, language order by c desc", snap.id);
  console.log("symbols by provider/grammar/language: " + JSON.stringify(byProv));
} else if (mode === "split") {
  const snap = all("select * from snapshots order by id desc limit 1")[0];
  const rows = all(
    "select f.path, f.language, f.errors, f.bytes, f.digest, " +
    "(select count(*) from symbols s where s.snapshot_id=f.snapshot_id and s.file_id=f.id) sym, " +
    "(select count(*) from imports i where i.snapshot_id=f.snapshot_id and i.file_id=f.id) imp, " +
    "(select count(*) from calls c where c.snapshot_id=f.snapshot_id and c.file_id=f.id) cal " +
    "from files f where f.snapshot_id=?", snap.id);
  const agg = {};
  for (const r of rows) {
    const k = TP(r.path);
    agg[k] = agg[k] || { files: 0, sym: 0, imp: 0, cal: 0, errors: 0, bytes: 0, lang: {} };
    const a = agg[k];
    a.files++; a.sym += r.sym; a.imp += r.imp; a.cal += r.cal; a.errors += r.errors; a.bytes += r.bytes;
    a.lang[r.language] = (a.lang[r.language] || 0) + 1;
  }
  console.log(JSON.stringify(agg, null, 2));
  const ext = {};
  for (const r of rows) {
    const e = path.extname(r.path).toLowerCase() || "(none)";
    ext[e] = ext[e] || { files: 0, sym: 0, imp: 0, cal: 0, errors: 0 };
    ext[e].files++; ext[e].sym += r.sym; ext[e].imp += r.imp; ext[e].cal += r.cal; ext[e].errors += r.errors;
  }
  console.log("by extension: " + JSON.stringify(Object.entries(ext).sort((a, b) => b[1].files - a[1].files).slice(0, 12)));
  console.log("top error files (first-party): " + JSON.stringify(
    rows.filter((r) => TP(r.path) === "first-party" && r.errors > 0).sort((a, b) => b.errors - a.errors).slice(0, 8).map((r) => r.path + ":" + r.errors)));
  console.log("top error files (third-party): " + JSON.stringify(
    rows.filter((r) => TP(r.path) === "third-party" && r.errors > 0).sort((a, b) => b.errors - a.errors).slice(0, 5).map((r) => r.path + ":" + r.errors)));
  console.log("first-party files with errors: " + rows.filter((r) => TP(r.path) === "first-party" && r.errors > 0).length +
    " of " + rows.filter((r) => TP(r.path) === "first-party").length);
  console.log("third-party files with errors: " + rows.filter((r) => TP(r.path) === "third-party" && r.errors > 0).length +
    " of " + rows.filter((r) => TP(r.path) === "third-party").length);
} else if (mode === "digest") {
  const snap = all("select * from snapshots order by id desc limit 1")[0];
  const h = crypto.createHash("sha256");
  let n = 0;
  const push = (tag, row) => { h.update(tag + "\u0001" + row.join("\u0001") + "\n"); n++; };
  for (const r of all("select path, digest, bytes, language, errors from files where snapshot_id=? order by path", snap.id)) push("file", r);
  for (const r of all("select f.path, s.kind, s.name, s.start_line, s.start_col, s.end_line, s.end_col, s.byte_start, s.byte_end, s.provider, s.grammar, s.language from symbols s join files f on f.id=s.file_id where s.snapshot_id=? order by f.path, s.start_line, s.start_col, s.kind, s.name", snap.id)) push("sym", r);
  for (const r of all("select f.path, i.start_line, i.start_col, i.end_line, i.end_col, i.byte_start, i.byte_end, i.text, i.provider from imports i join files f on f.id=i.file_id where i.snapshot_id=? order by f.path, i.start_line, i.start_col, i.text", snap.id)) push("imp", r);
  for (const r of all("select f.path, c.start_line, c.start_col, c.end_line, c.end_col, c.byte_start, c.byte_end, c.name, c.unresolved, c.provider, c.grammar from calls c join files f on f.id=c.file_id where c.snapshot_id=? order by f.path, c.start_line, c.start_col, c.name", snap.id)) push("call", r);
  console.log(JSON.stringify({ rows: n, fact_digest: h.digest("hex") }, null, 2));
  const he = crypto.createHash("sha256");
  let m = 0;
  for (const r of all("select edge_kind, precision, provider, caller_path, caller_sbs, caller_sbe, callee_name, callee_kind, coalesce(unresolved,'') from code_edges where snapshot_id=? order by caller_path, caller_sbs, edge_kind, callee_name", snap.id)) { he.update(r.join("\u0001") + "\n"); m++; }
  console.log(JSON.stringify({ edges: m, edge_digest: he.digest("hex") }, null, 2));
}
db.close();
