"use strict";
const { DatabaseSync } = require("node:sqlite");
const db = new DatabaseSync(process.argv[2], { readOnly: true });
const all = (sql, ...a) => db.prepare(sql).all(...a);

console.log("== symbols in colmap/src/colmap/mvs/patch_match.cc ==");
for (const r of all("select kind, name, start_line, start_col from symbols s join files f on f.id=s.file_id where f.path='colmap/src/colmap/mvs/patch_match.cc' order by start_line"))
  console.log("  L" + r.start_line + ":" + r.start_col + "  " + r.kind + "  " + r.name);

console.log("== symbols whose name contains 'Print' ==");
for (const r of all("select f.path, s.kind, s.name, s.start_line, s.start_col from symbols s join files f on f.id=s.file_id where s.name like '%Print%' order by f.path limit 25"))
  console.log("  " + r.path + ":" + r.start_line + ":" + r.start_col + "  " + r.kind + "  " + r.name);
console.log("  total = " + all("select count(*) c from symbols where name like '%Print%'")[0].c);

console.log("== symbols whose name contains 'PatchMatch' ==");
for (const r of all("select f.path, s.kind, s.name, s.start_line, s.start_col from symbols s join files f on f.id=s.file_id where s.name like '%PatchMatch%' order by f.path, s.start_line limit 25"))
  console.log("  " + r.path + ":" + r.start_line + ":" + r.start_col + "  " + r.kind + "  " + r.name);
console.log("  total = " + all("select count(*) c from symbols where name like '%PatchMatch%'")[0].c);

console.log("== does any symbol exist for a qualified C++ method? sample '::' names ==");
for (const r of all("select f.path, s.kind, s.name, s.start_line from symbols s join files f on f.id=s.file_id where s.name like '%::%' order by f.path limit 20"))
  console.log("  " + r.path + ":" + r.start_line + "  " + r.kind + "  " + r.name);
console.log("  total qualified names = " + all("select count(*) c from symbols where name like '%::%'")[0].c);
db.close();
