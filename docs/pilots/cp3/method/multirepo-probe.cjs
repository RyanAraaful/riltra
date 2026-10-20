"use strict";
// CP3 harness: the multi-repository isolation probe.
//
// Reads the two pilot indexes and reports, for a set of symbol names that
// exist in BOTH repositories, where each repository records them. It is the
// evidence for "identically named symbols do not contaminate the other
// repository" and for the E8 cross-repository candidate scope.
//
// Usage:
//   node multirepo-probe.cjs <db-a> <db-b> [name1,name2,...]

const { DatabaseSync } = require("node:sqlite");

const [dbA, dbB] = process.argv.slice(2, 4);
const wanted = (process.argv[4] || "").split(",").map((s) => s.trim()).filter(Boolean);

function repoFacts(dbPath) {
  const db = new DatabaseSync(dbPath, { readOnly: true });
  const all = (sql, ...a) => db.prepare(sql).all(...a);
  const snapshotId = all("SELECT MAX(id) AS id FROM snapshots")[0].id;
  const snapshot = all("SELECT * FROM snapshots WHERE id = ?1", snapshotId)[0];
  const symbols = all(
    "SELECT s.name AS name, f.path AS path FROM symbols s JOIN files f ON f.id = s.file_id WHERE s.snapshot_id = ?1 AND s.name IS NOT NULL",
    snapshotId
  );
  const files = all("SELECT path FROM files WHERE snapshot_id = ?1 ORDER BY path", snapshotId).map((f) => f.path);
  const imports = all(
    "SELECT f.path AS importer, i.text AS text FROM imports i JOIN files f ON f.id = i.file_id WHERE i.snapshot_id = ?1",
    snapshotId
  );
  db.close();
  return { snapshotId, snapshot, symbols, files, imports };
}

const a = repoFacts(dbA);
const b = repoFacts(dbB);

const names = wanted.length
  ? wanted
  : [...new Set(a.symbols.map((s) => s.name))].filter((n) => b.symbols.some((x) => x.name === n));

const rows = [];
for (const name of names) {
  const inA = a.symbols.filter((s) => s.name === name).map((s) => s.path);
  const inB = b.symbols.filter((s) => s.name === name).map((s) => s.path);
  rows.push({
    name,
    a_paths: [...new Set(inA)].sort(),
    b_paths: [...new Set(inB)].sort(),
    a_only: [...new Set(inA)].filter((p) => !b.files.includes(p)).length,
    b_only: [...new Set(inB)].filter((p) => !a.files.includes(p)).length,
  });
}

// Every repository-relative path recorded by one index must be absent from
// the other index's file table: the two stores are disjoint.
const sharedPaths = a.files.filter((p) => b.files.includes(p));

console.log(
  JSON.stringify(
    {
      a: {
        db: dbA,
        snapshot_id: a.snapshotId,
        head: a.snapshot ? a.snapshot.head : null,
        root: a.snapshot ? a.snapshot.root : null,
        files: a.files.length,
        symbols: a.symbols.length,
        imports: a.imports.length,
      },
      b: {
        db: dbB,
        snapshot_id: b.snapshotId,
        head: b.snapshot ? b.snapshot.head : null,
        root: b.snapshot ? b.snapshot.root : null,
        files: b.files.length,
        symbols: b.symbols.length,
        imports: b.imports.length,
      },
      shared_repository_relative_paths: sharedPaths.length,
      shared_sample: sharedPaths.slice(0, 10),
      common_symbols: rows,
    },
    null,
    2
  )
);
