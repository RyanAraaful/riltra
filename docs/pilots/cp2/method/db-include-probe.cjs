"use strict";
// How many C/C++ include rows could resolve to an indexed file if the
// literal were also matched as a unique path suffix? (diagnostic only)
const { DatabaseSync } = require("node:sqlite");
const db = new DatabaseSync(process.argv[2], { readOnly: true });
const all = (sql, ...a) => db.prepare(sql).all(...a);

const files = all("select path from files where snapshot_id=(select max(id) from snapshots)");
const byPath = new Set(files.map((f) => f.path));
const imports = all(
  "select f.path importer, f.language, i.text from imports i join files f on f.id=i.file_id where i.snapshot_id=(select max(id) from snapshots) and f.language in ('c','cpp')"
);

let literalOk = 0, literalMiss = 0;
let suffixUnique = 0, suffixAmbiguous = 0, suffixNone = 0;
const examples = [];
for (const im of imports) {
  const m = /["']([^"']+)["']/.exec(im.text || "");
  if (!m) continue;
  const lit = m[1];
  if (byPath.has(lit)) { literalOk++; continue; }
  literalMiss++;
  const suffix = "/" + lit;
  const hits = files.filter((f) => f.path.endsWith(suffix));
  if (hits.length === 1) {
    suffixUnique++;
    if (examples.length < 6) examples.push(`${im.importer} -> ${lit} => ${hits[0].path}`);
  } else if (hits.length > 1) {
    suffixAmbiguous++;
  } else {
    suffixNone++;
  }
}
console.log(JSON.stringify({
  cpp_import_rows: imports.length,
  resolved_by_current_rule: literalOk,
  unresolved_by_current_rule: literalMiss,
  of_those_unique_suffix_match: suffixUnique,
  ambiguous_suffix_match: suffixAmbiguous,
  no_suffix_match: suffixNone,
  examples,
}, null, 2));
db.close();
