// Targeted read-only queries for the CP2 decision package.
"use strict";
const { DatabaseSync } = require("node:sqlite");
const db = new DatabaseSync(process.argv[2], { readOnly: true });
const all = (sql, ...a) => db.prepare(sql).all(...a);

console.log("== imports recorded for colmap/src/colmap/mvs/patch_match.cc ==");
for (const r of all("select i.text, i.start_line, i.end_line from imports i join files f on f.id=i.file_id where f.path='colmap/src/colmap/mvs/patch_match.cc' order by i.start_line limit 14"))
  console.log("  L" + r.start_line + "-" + r.end_line + "  " + r.text);

console.log("== IMPORTS edges whose callee_name mentions patch_match.h ==");
for (const r of all("select callee_name, caller_path from code_edges where edge_kind='IMPORTS' and callee_name like '%patch_match.h%' limit 10"))
  console.log("  " + r.callee_name + "  <- " + r.caller_path);
console.log("  count = " + all("select count(*) c from code_edges where edge_kind='IMPORTS' and callee_name like '%patch_match.h%'")[0].c);

console.log("== POTENTIAL_CALL edges named 'Print' ==");
for (const r of all("select caller_path, caller_sbs, caller_sbe, unresolved from code_edges where edge_kind='POTENTIAL_CALL' and callee_name='Print'"))
  console.log("  " + r.caller_path + " " + r.caller_sbs + "-" + r.caller_sbe + "  " + r.unresolved);
console.log("  count = " + all("select count(*) c from code_edges where edge_kind='POTENTIAL_CALL' and callee_name='Print'")[0].c);

console.log("== POTENTIAL_CALL edges named 'Run' (count by caller file) ==");
for (const r of all("select caller_path, count(*) c from code_edges where edge_kind='POTENTIAL_CALL' and callee_name='Run' group by caller_path order by c desc limit 8"))
  console.log("  " + r.caller_path + " : " + r.c);

console.log("== POTENTIAL_CALL edges named 'CreateWorkspace' ==");
for (const r of all("select caller_path, caller_sbs, caller_sbe, unresolved from code_edges where edge_kind='POTENTIAL_CALL' and callee_name='CreateWorkspace'"))
  console.log("  " + r.caller_path + " " + r.caller_sbs + "-" + r.caller_sbe + "  " + r.unresolved);

console.log("== POTENTIAL_CALL edges naming PatchMatchController ==");
for (const r of all("select caller_path, callee_name, unresolved from code_edges where edge_kind='POTENTIAL_CALL' and callee_name like '%PatchMatchController%' limit 10"))
  console.log("  " + r.caller_path + "  -> " + r.callee_name + "  " + r.unresolved);

console.log("== definitions named PatchMatchController / PatchMatch (symbol rows) ==");
for (const r of all("select f.path, s.kind, s.name, s.start_line, s.start_col from symbols s join files f on f.id=s.file_id where s.name in ('PatchMatchController','PatchMatch','PatchMatchCuda') order by s.name, f.path, s.start_line"))
  console.log("  " + r.name + "  " + r.kind + "  " + r.path + ":" + r.start_line + ":" + r.start_col);

console.log("== definitions named 'Print' (first 12) ==");
for (const r of all("select f.path, s.kind, s.name, s.start_line, s.start_col from symbols s join files f on f.id=s.file_id where s.name='Print' order by f.path limit 12"))
  console.log("  " + r.path + ":" + r.start_line + ":" + r.start_col + "  " + r.kind);

console.log("== is max_triangulation_angle indexed as a symbol? ==");
console.log("  count = " + all("select count(*) c from symbols where name='max_triangulation_angle'")[0].c);
console.log("== symbols with kind 'field'/'variable'/'member' (sample) ==");
for (const r of all("select kind, count(*) c from symbols group by kind order by c desc"))
  console.log("  " + r.kind + " : " + r.c);
db.close();
