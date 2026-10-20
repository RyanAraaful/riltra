"use strict";
// CP3 harness: E8 include-resolution baseline + compiler-equivalent ground truth.
//
// Reads the pilot index read-only. For every C/C++ import row it classifies the
// quoted include literal under the rules that matter to the E8 decision:
//
//   code_rule   - the rule the product actually applies today
//                 (crates/cli/src/index.rs import_mentions_path): the literal is
//                 resolved RELATIVE TO THE IMPORTER'S DIRECTORY and must equal
//                 the target's repository-relative path.
//   probe_rule  - the rule CP2's db-include-probe.cjs measured: the literal
//                 itself must equal a repository-relative path. Recorded
//                 separately because it is NOT the product's rule.
//   suffix      - the Option A rule: the indexed paths that equal the literal
//                 or end with "/" + literal. Unique / ambiguous / none.
//   compiler    - what a C++ compiler would actually open for a QUOTED
//                 include: first the including file's own directory, then each
//                 -I root in order. The roots are passed in (repo-relative) so
//                 the ground truth is auditable, not guessed.
//
// Usage:
//   node e8-baseline.cjs <db> [root1,root2,...]
//
// Output: JSON on stdout.

const { DatabaseSync } = require("node:sqlite");

const dbPath = process.argv[2];
if (!dbPath) {
  console.error("usage: node e8-baseline.cjs <db> [root1,root2,...]");
  process.exit(2);
}
const roots = (process.argv[3] || "")
  .split(",")
  .map((r) => r.trim().replace(/\\/g, "/").replace(/\/+$/, ""))
  .filter(Boolean);

const db = new DatabaseSync(dbPath, { readOnly: true });
const all = (sql, ...a) => db.prepare(sql).all(...a);

const snapshotId = all("SELECT MAX(id) AS id FROM snapshots")[0].id;
if (!snapshotId) {
  console.error("no snapshot in " + dbPath);
  process.exit(3);
}

const files = all(
  "SELECT path FROM files WHERE snapshot_id = ?1 ORDER BY path",
  snapshotId
).map((f) => f.path);
const byPath = new Set(files);

const imports = all(
  `SELECT f.path AS importer, f.language AS language, i.text AS text
     FROM imports i JOIN files f ON f.id = i.file_id
    WHERE i.snapshot_id = ?1 AND f.language IN ('c','cpp')
    ORDER BY f.path, i.byte_start`,
  snapshotId
);

// The quoted literal of a preprocessor include. Angle-bracket and unquoted
// forms are not quoted includes and name a root/system header.
function quotedLiteral(text) {
  const m = /["']([^"']+)["']/.exec(text || "");
  return m ? m[1] : null;
}

// Normalised repository-relative path of "dir(importer)/literal", the way the
// product's C/C++ branch composes it.
function relativeToImporter(importer, literal) {
  const parts = importer.split("/").slice(0, -1);
  for (const part of literal.split("/")) {
    if (part === "." || part === "") continue;
    if (part === "..") {
      if (!parts.length) return null;
      parts.pop();
      continue;
    }
    parts.push(part);
  }
  return parts.join("/");
}

// What a compiler opens for a quoted include: the including file's own
// directory first, then each -I root in order. Returns the first indexed
// repository path, or null when the compiler would leave the repository
// (a system header, or no such file).
function compilerResolves(importer, literal) {
  const candidates = [relativeToImporter(importer, literal)];
  for (const root of roots) candidates.push(root + "/" + literal);
  for (const c of candidates) {
    if (c && byPath.has(c)) return c;
  }
  return null;
}

function suffixHits(literal) {
  const needle = "/" + literal;
  return files.filter((f) => f === literal || f.endsWith(needle));
}

const totals = {
  cpp_import_rows: imports.length,
  quoted_rows: 0,
  unquoted_rows: 0,
  code_rule_resolves: 0,
  probe_rule_resolves: 0,
  suffix_unique: 0,
  suffix_ambiguous: 0,
  suffix_none: 0,
  rooted_literal_rows: 0,
  bare_literal_rows: 0,
  // Ground truth: of the rows a unique suffix match would claim, how often the
  // compiler-equivalent resolution lands on that same file.
  suffix_unique_agrees_with_compiler: 0,
  suffix_unique_disagrees_with_compiler: 0,
  suffix_unique_compiler_unresolved: 0,
  suffix_unique_compiler_resolves_elsewhere: 0,
  // The rows a unique suffix match would ADD to today's candidate scope.
  suffix_unique_new_candidates: 0,
  suffix_unique_new_rooted: 0,
  suffix_unique_new_bare: 0,
  suffix_unique_new_agrees_with_compiler: 0,
  suffix_unique_new_disagrees_with_compiler: 0,
  suffix_unique_new_compiler_unresolved: 0,
  suffix_unique_new_compiler_resolves_elsewhere: 0,
};

const disagreement = [];
const agrees = [];
const ambiguousSamples = [];
const missingSamples = [];

for (const im of imports) {
  const literal = quotedLiteral(im.text);
  if (!literal) {
    totals.unquoted_rows++;
    continue;
  }
  totals.quoted_rows++;
  if (literal.includes("/")) totals.rooted_literal_rows++;
  else totals.bare_literal_rows++;

  const relative = relativeToImporter(im.importer, literal);
  if (relative && byPath.has(relative)) totals.code_rule_resolves++;
  if (byPath.has(literal)) totals.probe_rule_resolves++;

  const hits = suffixHits(literal);
  if (hits.length === 0) {
    totals.suffix_none++;
    if (missingSamples.length < 100) {
      missingSamples.push({ importer: im.importer, literal });
    }
    continue;
  }
  if (hits.length > 1) {
    totals.suffix_ambiguous++;
    if (ambiguousSamples.length < 12) {
      ambiguousSamples.push({ importer: im.importer, literal, hits: hits.slice(0, 4) });
    }
    continue;
  }

  totals.suffix_unique++;
  const claimed = hits[0];
  const compiler = compilerResolves(im.importer, literal);
  const alreadyResolved = relative !== null && byPath.has(relative);
  if (!alreadyResolved) {
    totals.suffix_unique_new_candidates++;
    if (literal.includes("/")) totals.suffix_unique_new_rooted++;
    else totals.suffix_unique_new_bare++;
  }
  const verdict = compiler === claimed ? "agrees" : compiler === null ? "unresolved" : "elsewhere";
  if (verdict === "agrees") {
    totals.suffix_unique_agrees_with_compiler++;
    if (!alreadyResolved) totals.suffix_unique_new_agrees_with_compiler++;
    if (agrees.length < 12 && !alreadyResolved) {
      agrees.push({ importer: im.importer, literal, file: claimed, new_candidate: true });
    }
  } else if (verdict === "unresolved") {
    totals.suffix_unique_compiler_unresolved++;
    if (!alreadyResolved) totals.suffix_unique_new_compiler_unresolved++;
    if (disagreement.length < 40) {
      disagreement.push({
        importer: im.importer,
        literal,
        claimed,
        compiler: null,
        new_candidate: !alreadyResolved,
        kind: "compiler-would-not-open",
      });
    }
  } else {
    totals.suffix_unique_compiler_resolves_elsewhere++;
    if (!alreadyResolved) totals.suffix_unique_new_compiler_resolves_elsewhere++;
    if (disagreement.length < 40) {
      disagreement.push({
        importer: im.importer,
        literal,
        claimed,
        compiler,
        new_candidate: !alreadyResolved,
        kind: "compiler-opens-another-file",
      });
    }
  }
}

// The literal form matters: a bare filename can be shadowed by a system
// header, a rooted one carries the project's namespace.
const bareClaimed = [];
for (const im of imports) {
  const literal = quotedLiteral(im.text);
  if (!literal || literal.includes("/")) continue;
  const hits = suffixHits(literal);
  if (hits.length === 1 && bareClaimed.length < 12) {
    bareClaimed.push({
      importer: im.importer,
      literal,
      claimed: hits[0],
      compiler: compilerResolves(im.importer, literal),
    });
  }
}

console.log(
  JSON.stringify(
    {
      db: dbPath,
      snapshot_id: snapshotId,
      indexed_files: files.length,
      indexed_paths: files,
      include_roots: roots,
      totals,
      samples: {
        agree: agrees,
        disagreement,
        ambiguous: ambiguousSamples,
        missing: missingSamples,
        bare_literal_claims: bareClaimed,
      },
    },
    null,
    2
  )
);
db.close();
