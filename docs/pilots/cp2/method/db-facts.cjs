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
  // The digest is shared with the mutation check so that one implementation
  // exists (node:sqlite returns row objects, and the row order of a
  // non-total ORDER BY is not stable between two builds).
  const { factTables, combinedDigest } = require("./fact-digest.cjs");
  const tables = factTables(dbPath);
  if (!tables) {
    console.error("the database has no snapshot");
    process.exit(3);
  }
  console.log(JSON.stringify({ tables: tables, combined_digest: combinedDigest(tables) }, null, 2));
}
db.close();
