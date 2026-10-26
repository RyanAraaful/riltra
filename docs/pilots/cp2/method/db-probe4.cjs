"use strict";
const { DatabaseSync } = require("node:sqlite");
const db = new DatabaseSync(process.argv[2], { readOnly: true });
const all = (sql, ...a) => db.prepare(sql).all(...a);

console.log("== calls in patch_match.cc L655-L690 ==");
for (const r of all("select c.name, c.start_line, c.start_col, c.tcallee_start_line, c.tcallee_start_col from calls c join files f on f.id=c.file_id where f.path='colmap/src/colmap/mvs/patch_match.cc' and c.start_line between 655 and 690 order by c.start_line"))
  console.log("  L" + r.start_line + ":" + r.start_col + "  name=" + r.name + "  tcallee=L" + r.tcallee_start_line + ":" + r.tcallee_start_col);

console.log("== all POTENTIAL_CALL edges with callee_name = 'PatchMatch' ==");
for (const r of all("select caller_path, caller_sbs, unresolved from code_edges where edge_kind='POTENTIAL_CALL' and callee_name='PatchMatch'"))
  console.log("  " + r.caller_path + " " + r.caller_sbs + "  " + r.unresolved);
console.log("  count = " + all("select count(*) c from code_edges where edge_kind='POTENTIAL_CALL' and callee_name='PatchMatch'")[0].c);

console.log("== all POTENTIAL_CALL edges whose callee_name contains 'PatchMatch' ==");
for (const r of all("select callee_name, caller_path, caller_sbs, unresolved from code_edges where edge_kind='POTENTIAL_CALL' and callee_name like '%PatchMatch%'"))
  console.log("  " + r.callee_name + "  <- " + r.caller_path + " " + r.caller_sbs + "  " + r.unresolved);

console.log("== all POTENTIAL_CALL edges whose callee_name = 'PatchMatchController' (exact) ==");
console.log("  count = " + all("select count(*) c from code_edges where edge_kind='POTENTIAL_CALL' and callee_name='PatchMatchController'")[0].c);

console.log("== how many C++ files include patch_match.h (candidate set) ==");
for (const r of all("select f.path from code_edges e join files f on f.id=e.file_id where e.edge_kind='IMPORTS' and e.callee_name like '%mvs/patch_match.h%'"))
  console.log("  " + r.path);
db.close();
