// Verify whether C++ direct-initialization declarations are captured as calls.
"use strict";
const { DatabaseSync } = require("node:sqlite");
const db = new DatabaseSync(process.argv[2], { readOnly: true });
const all = (sql, ...a) => db.prepare(sql).all(...a);

console.log("== calls recorded in colmap/src/colmap/exe/mvs.cc around L265-L280 ==");
for (const r of all("select c.name, c.start_line, c.start_col, c.end_line, c.tcallee_start_line, c.tcallee_start_col from calls c join files f on f.id=c.file_id where f.path='colmap/src/colmap/exe/mvs.cc' and c.start_line between 255 and 285 order by c.start_line"))
  console.log("  L" + r.start_line + ":" + r.start_col + "  name=" + r.name + "  tcallee=L" + r.tcallee_start_line + ":" + r.tcallee_start_col);

console.log("== calls in automatic_reconstruction.cc around L425-L440 ==");
for (const r of all("select c.name, c.start_line, c.start_col from calls c join files f on f.id=c.file_id where f.path='colmap/src/colmap/controllers/automatic_reconstruction.cc' and c.start_line between 420 and 445 order by c.start_line"))
  console.log("  L" + r.start_line + ":" + r.start_col + "  name=" + r.name);

console.log("== calls in dense_reconstruction_widget.cc around L390-L405 ==");
for (const r of all("select c.name, c.start_line, c.start_col from calls c join files f on f.id=c.file_id where f.path='colmap/src/colmap/ui/dense_reconstruction_widget.cc' and c.start_line between 390 and 405 order by c.start_line"))
  console.log("  L" + r.start_line + ":" + r.start_col + "  name=" + r.name);

console.log("== symbols with kind='method' (all 51) ==");
for (const r of all("select f.path, s.name, s.start_line, s.start_col from symbols s join files f on f.id=s.file_id where s.kind='method' order by f.path limit 15"))
  console.log("  " + r.path + ":" + r.start_line + ":" + r.start_col + "  " + r.name);
console.log("  total = " + all("select count(*) c from symbols where kind='method'")[0].c);
console.log("== method-kind symbols by language ==");
for (const r of all("select language, count(*) c from symbols where kind='method' group by language"))
  console.log("  " + r.language + " : " + r.c);

console.log("== are C++ member functions captured as 'function' instead? sample PatchMatch methods ==");
for (const r of all("select f.path, s.kind, s.name, s.start_line, s.start_col from symbols s join files f on f.id=s.file_id where f.path='colmap/src/colmap/mvs/patch_match.h' order by s.start_line limit 30"))
  console.log("  L" + r.start_line + ":" + r.start_col + "  " + r.kind + "  " + r.name);
db.close();
