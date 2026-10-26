// SPDX-License-Identifier: Apache-2.0
// Occurrence parity at pilot scale: the rows of `reference_occurrences` of two
// indexes of the same source tree must be identical. The frozen CP2 fact
// digest (fact-digest.cjs) covers files, symbols, imports, calls and edges but
// not this table, so this script supplies the missing comparison.
//
// Usage: node occurrence-parity.cjs <db-a> <db-b> [<db-c> ...]
// Reads each database read-only; prints the row count and a SHA-256 over the
// ordered rows, and exits 1 unless every digest is equal.
'use strict';
const crypto = require('node:crypto');
const { DatabaseSync } = require('node:sqlite');

const dbs = process.argv.slice(2);
if (dbs.length < 2) {
  console.error('usage: node occurrence-parity.cjs <db-a> <db-b> [<db-c> ...]');
  process.exit(2);
}
const digests = dbs.map((file) => {
  const db = new DatabaseSync(file, { readOnly: true });
  const rows = db
    .prepare(
      `SELECT f.path, r.name, r.role, r.start_line, r.start_col, r.end_line, r.end_col,
              r.byte_start, r.byte_end, r.provider, r.grammar, r.language
       FROM reference_occurrences r JOIN files f ON f.id = r.file_id
       ORDER BY f.path, r.byte_start, r.role, r.name`
    )
    .all();
  db.close();
  const hash = crypto.createHash('sha256');
  for (const row of rows) hash.update(JSON.stringify(Object.values(row)) + '\n');
  return { file, rows: rows.length, sha256: hash.digest('hex') };
});
const tail = (p) => p.split('\\').join('/').split('/').slice(-3).join('/');
for (const d of digests) console.log(`${d.rows} rows  ${d.sha256}  ${tail(d.file)}`);
const same = digests.every((d) => d.sha256 === digests[0].sha256 && d.rows === digests[0].rows);
console.log(same ? 'IDENTICAL: reference_occurrences' : 'DIFFERENT: reference_occurrences');
process.exit(same ? 0 : 1);
