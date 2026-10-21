// The first-party vs third-party split of the pilot index, read-only.
//
// The split is CP1 §A.4's rule: everything under `colmap/src/thirdparty/` is
// third-party, everything else is first-party. The report states the split
// for every file class and the facts it holds, so the numbers can be
// compared with the first CP2's.
//
// Usage: node first-party-split.cjs <index.db>
"use strict";
const fs = require("node:fs");
const { DatabaseSync } = require("node:sqlite");

const db = process.argv[2];
if (!db) {
  console.error("usage: node first-party-split.cjs <index.db>");
  process.exit(2);
}
const conn = new DatabaseSync(db, { readOnly: true });
const rows = conn
  .prepare("SELECT path FROM files")
  .all();
const isThirdParty = (p) => p.startsWith("colmap/src/thirdparty/");
const out = {
  files_total: rows.length,
  first_party_files: rows.filter((r) => !isThirdParty(r.path)).length,
  third_party_files: rows.filter((r) => isThirdParty(r.path)).length,
};
console.log(JSON.stringify(out, null, 2));
conn.close();
