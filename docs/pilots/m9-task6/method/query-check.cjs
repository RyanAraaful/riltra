// SPDX-License-Identifier: Apache-2.0
// Citation, span and label checks over frozen query outputs (read-only).
// Usage: node query-check.cjs <query-dir> <source-root> <out.json> [index.db]
"use strict";
const fs = require("node:fs");
const path = require("node:path");

const [qdir, root, outFile, dbPath] = process.argv.slice(2);
if (!qdir || !root || !outFile) {
  console.error("usage: node query-check.cjs <query-dir> <source-root> <out.json> [index.db]");
  process.exit(2);
}
const squash = (s) => String(s).replace(/\s+/g, "");

const srcCache = new Map();
function src(rel) {
  if (!srcCache.has(rel)) {
    const f = path.join(root, rel);
    srcCache.set(rel, fs.existsSync(f) ? fs.readFileSync(f) : null);
  }
  return srcCache.get(rel);
}
function sliceOf(rel, span) {
  const b = src(rel);
  if (!b || !Array.isArray(span) || span.length !== 2) return null;
  if (span[0] < 0 || span[1] > b.length || span[0] > span[1]) return null;
  return b.subarray(span[0], span[1]).toString("utf8");
}

// Every object that carries an edge: it has caller_path and a kind.
function collectEdges(node, acc) {
  if (Array.isArray(node)) {
    for (const n of node) collectEdges(n, acc);
  } else if (node && typeof node === "object") {
    if (typeof node.kind === "string" && "caller_path" in node) acc.push(node);
    for (const v of Object.values(node)) collectEdges(v, acc);
  }
  return acc;
}

let db = null;
let symbolRows = null;
if (dbPath) {
  const { DatabaseSync } = require("node:sqlite");
  db = new DatabaseSync(dbPath, { readOnly: true });
  const sid = db.prepare("select id from snapshots order by id desc limit 1").get().id;
  symbolRows = new Set(
    db.prepare("select f.path p, s.byte_start b from symbols s join files f on f.id=s.file_id where s.snapshot_id=?").all(sid)
      .map((r) => r.p + "\u0001" + r.b)
  );
}

const tot = {
  files: 0, parsed: 0, edges: 0, kinds: {}, precision: {}, verification: {}, unresolved_reasons: {},
  calls_edges: 0, exact_edges: 0, candidate_edges: 0,
  citation_checked: 0, citation_ok: 0, citation_bad: [],
  endpoint_checked: 0, endpoint_ok: 0, endpoint_bad: [],
  search_results_checked: 0, search_results_ok: 0, search_results_bad: [],
};
const perFile = {};
const bump = (obj, k) => { obj[k] = (obj[k] || 0) + 1; };

for (const f of fs.readdirSync(qdir).filter((x) => x.endsWith(".json") && !x.startsWith("_")).sort()) {
  tot.files++;
  const name = f.replace(/\.json$/, "");
  let doc;
  try { doc = JSON.parse(fs.readFileSync(path.join(qdir, f), "utf8")); } catch (e) { perFile[name] = { parsed: false }; continue; }
  tot.parsed++;
  const info = { parsed: true, edges: 0, completeness: doc.completeness ? doc.completeness.status : null,
    unresolved: doc.completeness && Array.isArray(doc.completeness.unresolved) ? doc.completeness.unresolved : null,
    verdict: doc.verdict ?? null, disclaimer: doc.disclaimer ?? null, truncated: doc.truncated ?? null };
  for (const e of collectEdges(doc, [])) {
    tot.edges++; info.edges++;
    bump(tot.kinds, e.kind);
    if (e.precision !== undefined) bump(tot.precision, String(e.precision));
    if (e.verification !== undefined) bump(tot.verification, String(e.verification));
    if (e.unresolved !== undefined) bump(tot.unresolved_reasons, String(e.unresolved));
    if (e.kind === "CALLS") tot.calls_edges++;
    if (e.precision === "Exact") tot.exact_edges++;
    if (e.kind === "POTENTIAL_CALL") tot.candidate_edges++;
    // Citation: the terminal identifier span of the edge must be the callee name as written.
    const term = e.evidence && e.evidence.terminal_span;
    if (e.callee_name != null && term) {
      tot.citation_checked++;
      const got = sliceOf(e.caller_path, term);
      const inside = Array.isArray(e.caller_span) && term[0] >= e.caller_span[0] && term[1] <= e.caller_span[1];
      if (got !== null && squash(got) === squash(e.callee_name) && inside) tot.citation_ok++;
      else if (tot.citation_bad.length < 20) tot.citation_bad.push({ file: name, caller: e.caller_path, span: term, name: e.callee_name, got });
    }
    // Endpoint: a syntactic definition endpoint must be a symbol the index holds.
    if (symbolRows && e.callee && e.callee.type === "syntax_def") {
      tot.endpoint_checked++;
      const key = e.callee.path + "\u0001" + e.callee.byte_start;
      if (symbolRows.has(key)) tot.endpoint_ok++;
      else if (tot.endpoint_bad.length < 20) tot.endpoint_bad.push({ file: name, endpoint: e.callee });
    }
  }
  // Search results: a definition's line must contain its last name segment.
  if (Array.isArray(doc.results)) {
    for (const r of doc.results) {
      if (!r.span || !r.name || !r.path) continue;
      tot.search_results_checked++;
      const b = src(r.path);
      const last = String(r.name).split("::").pop();
      let lineText = null;
      if (b) {
        let line = 1; let start = 0;
        for (let i = 0; i < b.length && line < r.span.start_line; i++) if (b[i] === 0x0a) { line++; start = i + 1; }
        let end = b.indexOf(0x0a, start); if (end < 0) end = b.length;
        lineText = b.subarray(start, end).toString("utf8");
      }
      if (lineText !== null && lineText.includes(last)) tot.search_results_ok++;
      else if (tot.search_results_bad.length < 20) tot.search_results_bad.push({ file: name, path: r.path, line: r.span.start_line, name: r.name });
    }
  }
  perFile[name] = info;
}

const out = { totals: tot, files: perFile };
fs.writeFileSync(outFile, JSON.stringify(out, null, 2));
console.log("query-check written: " + outFile);
if (db) db.close();
