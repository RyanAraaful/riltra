// Does an MCP read mutate the store's CONTENT? The index file's BYTES can
// change because a WAL checkpoint rewrites the file; the facts must not.
//
// Usage: node db-mutation-check.cjs <index.db> [out.json]
"use strict";
const crypto = require("node:crypto");
const fs = require("node:fs");
const { factTables, combinedDigest } = require("./fact-digest.cjs");

const dbPath = process.argv[2];
const outPath = process.argv[3] || "";
if (!dbPath) {
  console.error("usage: node db-mutation-check.cjs <index.db> [out.json]");
  process.exit(2);
}

const tables = factTables(dbPath);
if (!tables) {
  console.error("the database has no snapshot");
  process.exit(3);
}
const result = {
  file_bytes: fs.statSync(dbPath).size,
  file_sha256: crypto.createHash("sha256").update(fs.readFileSync(dbPath)).digest("hex"),
  tables: tables,
  fact_rows: tables.files.rows + tables.symbols.rows + tables.imports.rows + tables.calls.rows,
  total_rows: Object.values(tables).reduce((n, t) => n + t.rows, 0),
  combined_digest: combinedDigest(tables),
};
if (outPath) fs.writeFileSync(outPath, JSON.stringify(result, null, 2));
console.log(JSON.stringify(result, null, 2));
