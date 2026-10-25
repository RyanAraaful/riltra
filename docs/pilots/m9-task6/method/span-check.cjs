// SPDX-License-Identifier: Apache-2.0
// Source-grounded check of a Riltra index database against its source tree.
// Read-only. Usage: node span-check.cjs <index.db> <source-root> <out.json> [inl-path...]
"use strict";
const { DatabaseSync } = require("node:sqlite");
const crypto = require("node:crypto");
const fs = require("node:fs");
const path = require("node:path");

const [dbPath, root, outFile, ...extra] = process.argv.slice(2);
if (!dbPath || !root || !outFile) {
  console.error("usage: node span-check.cjs <index.db> <source-root> <out.json> [inl-path...]");
  process.exit(2);
}
const db = new DatabaseSync(dbPath, { readOnly: true });
const all = (sql, ...args) => db.prepare(sql).all(...args);
const sid = all("select id from snapshots order by id desc limit 1")[0].id;
const squash = (s) => String(s).replace(/\s+/g, "");

// Riltra's content digest: FNV-1a 64-bit, lowercase hex (crates/cli/src/store.rs content_digest).
function fnv1a64(bytes) {
  let h = 0xcbf29ce484222325n;
  const prime = 0x100000001b3n;
  const mask = 0xffffffffffffffffn;
  for (let i = 0; i < bytes.length; i++) {
    h ^= BigInt(bytes[i]);
    h = (h * prime) & mask;
  }
  return h.toString(16).padStart(16, "0");
}

const cache = new Map();
function source(rel) {
  if (!cache.has(rel)) {
    const file = path.join(root, rel);
    let value = null;
    if (fs.existsSync(file)) {
      const bytes = fs.readFileSync(file);
      const starts = [0];
      for (let i = 0; i < bytes.length; i++) if (bytes[i] === 0x0a) starts.push(i + 1);
      value = { bytes, starts };
    }
    cache.set(rel, value);
  }
  return cache.get(rel);
}

// 1-based line and 0-based byte column of a byte offset.
function locate(src, off) {
  const s = src.starts;
  let lo = 0;
  let hi = s.length - 1;
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1;
    if (s[mid] <= off) lo = mid;
    else hi = mid - 1;
  }
  return { line: lo + 1, col0: off - s[lo] };
}

const newTally = () => ({
  rows: 0, missing_source: 0, line_mismatch: 0, start_col_delta: {}, end_col_delta: {},
  name_checked: 0, name_mismatch: 0, nameless: 0, samples: [],
});

// Checks one fact row: its byte span must map to the reported lines and
// columns, and its byte slice must contain the reported name (whitespace ignored).
function checkRow(t, src, row, label, nameKey, sliceTest) {
  t.rows++;
  if (row[nameKey] === null || row[nameKey] === undefined) t.nameless++;
  if (!src) { t.missing_source++; return; }
  const a = locate(src, row.byte_start);
  const b = locate(src, row.byte_end);
  const sd = row.start_col - a.col0;
  const ed = row.end_col - b.col0;
  t.start_col_delta[sd] = (t.start_col_delta[sd] || 0) + 1;
  t.end_col_delta[ed] = (t.end_col_delta[ed] || 0) + 1;
  const lineOk = a.line === row.start_line && b.line === row.end_line;
  if (!lineOk) t.line_mismatch++;
  const slice = src.bytes.subarray(row.byte_start, row.byte_end).toString("utf8");
  let nameOk = true;
  if (row[nameKey] !== null && row[nameKey] !== undefined) {
    t.name_checked++;
    nameOk = sliceTest(squash(slice), squash(row[nameKey]));
    if (!nameOk) t.name_mismatch++;
  }
  if ((!lineOk || !nameOk) && t.samples.length < 10) {
    t.samples.push({ label, start_line: row.start_line, end_line: row.end_line, computed: [a.line, b.line], name: row[nameKey] ?? null, slice: slice.slice(0, 120) });
  }
}

const out = { snapshot_id: sid, db: path.basename(dbPath) };

// --- files: content identity, errors, languages, unsupported extensions
const files = all("select id, path, digest, bytes, language, errors from files where snapshot_id=?", sid);
const fileInfo = { rows: files.length, digest_match: 0, digest_mismatch: 0, bytes_mismatch: 0, missing_source: 0,
  errors_sum: 0, files_with_errors: 0, by_language: {}, by_extension_processed: {}, cu_cuh_rows: 0 };
const byId = new Map();
for (const f of files) {
  byId.set(f.id, f);
  fileInfo.errors_sum += f.errors;
  if (f.errors > 0) fileInfo.files_with_errors++;
  fileInfo.by_language[f.language] = (fileInfo.by_language[f.language] || 0) + 1;
  const ext = path.extname(f.path).toLowerCase() || "(none)";
  fileInfo.by_extension_processed[ext] = (fileInfo.by_extension_processed[ext] || 0) + 1;
  if (/\.cuh?$/i.test(f.path)) fileInfo.cu_cuh_rows++;
  const src = source(f.path);
  if (!src) { fileInfo.missing_source++; continue; }
  if (src.bytes.length !== f.bytes) fileInfo.bytes_mismatch++;
  if (fnv1a64(src.bytes) === f.digest) fileInfo.digest_match++;
  else fileInfo.digest_mismatch++;
}
out.files = fileInfo;

// --- symbols, imports, calls: span and name grounding
const symbols = newTally();
for (const r of all("select file_id, kind, name, start_line, start_col, end_line, end_col, byte_start, byte_end from symbols where snapshot_id=?", sid)) {
  const f = byId.get(r.file_id);
  checkRow(symbols, f ? source(f.path) : null, r, `${f.path}:${r.start_line} ${r.kind}`, "name",
    (slice, name) => slice.includes(name));
}
out.symbols = symbols;

const imports = newTally();
for (const r of all("select file_id, text, start_line, start_col, end_line, end_col, byte_start, byte_end from imports where snapshot_id=?", sid)) {
  const f = byId.get(r.file_id);
  checkRow(imports, f ? source(f.path) : null, r, `${f.path}:${r.start_line} import`, "text",
    (slice, text) => slice.includes(text));
}
out.imports = imports;

const calls = newTally();
const callRows = all("select file_id, name, start_line, start_col, end_line, end_col, byte_start, byte_end from calls where snapshot_id=?", sid);
for (const r of callRows) {
  const f = byId.get(r.file_id);
  checkRow(calls, f ? source(f.path) : null, r, `${f.path}:${r.start_line} call`, "name",
    (slice, name) => slice.includes(name));
}
out.calls = calls;

// --- the .inl files: errors, facts and name shapes (the Task 2 table, per file)
const inlPaths = extra.length ? extra : files.filter((f) => f.path.endsWith(".inl")).map((f) => f.path);
const inl = [];
for (const p of inlPaths) {
  const f = files.find((x) => x.path === p);
  if (!f) { inl.push({ path: p, indexed: false }); continue; }
  const syms = all("select name from symbols where snapshot_id=? and file_id=?", sid, f.id);
  const cls = all("select name from calls where snapshot_id=? and file_id=?", sid, f.id);
  const shapes = {};
  for (const c of cls) {
    const k = c.name === null ? "(null)" : /^[A-Za-z_][\w:]*$/.test(c.name) ? "identifier" : c.name.slice(0, 24);
    shapes[k] = (shapes[k] || 0) + 1;
  }
  inl.push({
    path: p, indexed: true, language: f.language, errors: f.errors,
    definitions: syms.length, definitions_nameless: syms.filter((s) => s.name === null).length,
    calls: cls.length, calls_nameless: cls.filter((c) => c.name === null).length,
    calls_non_identifier: cls.filter((c) => c.name !== null && !/^[A-Za-z_][\w:]*$/.test(c.name)).length,
    call_name_shapes_top: Object.entries(shapes).sort((a, b) => b[1] - a[1]).slice(0, 8),
  });
}
out.inl = inl;
out.inl_summary = {
  files_indexed: files.filter((f) => f.path.endsWith(".inl")).length,
  zero_error: files.filter((f) => f.path.endsWith(".inl") && f.errors === 0).length,
  with_error: files.filter((f) => f.path.endsWith(".inl") && f.errors > 0).length,
  errors_total: files.filter((f) => f.path.endsWith(".inl")).reduce((s, f) => s + f.errors, 0),
};

fs.writeFileSync(outFile, JSON.stringify(out, null, 2));
console.log("span-check written: " + outFile);
db.close();
