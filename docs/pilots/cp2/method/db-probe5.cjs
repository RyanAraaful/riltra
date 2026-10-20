"use strict";
const { DatabaseSync } = require("node:sqlite");
const db = new DatabaseSync(process.argv[2], { readOnly: true });
const all = (sql, ...a) => db.prepare(sql).all(...a);

console.log("== edges whose callee_name contains '<' (template calls) ==");
for (const r of all("select callee_name, count(*) c from code_edges where edge_kind='POTENTIAL_CALL' and callee_name like '%<%' group by callee_name order by c desc limit 15"))
  console.log("  " + r.callee_name + " : " + r.c);
console.log("  distinct = " + all("select count(distinct callee_name) c from code_edges where callee_name like '%<%'")[0].c +
  "  total edges = " + all("select count(*) c from code_edges where callee_name like '%<%'")[0].c);

console.log("== of those, how many end with '>' (mangled terminal) ==");
console.log("  count = " + all("select count(*) c from code_edges where callee_name like '%>'")[0].c);

console.log("== python/other language template calls (should be unaffected) ==");
for (const r of all("select f.path, e.callee_name from code_edges e join files f on f.id=e.file_id where e.callee_name like '%<%' and f.language='python' limit 5"))
  console.log("  " + r.path + " -> " + r.callee_name);

console.log("== how many C++ declarations would a direct-init capture add? (declarations with an argument_list) ==");
console.log("  (approx) calls whose name looks like a type: " +
  all("select count(*) c from calls where name glob '[A-Z]*'").all().length);
db.close();
