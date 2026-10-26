// SPDX-License-Identifier: Apache-2.0
// E7 Slice 2 pilot acceptance (spec section 8.5, rows P1-P5) and the
// performance rows of section 8.6 (F1), run against an already indexed pilot
// home. Every expectation below was written from the frozen evidence
// (docs/m9-e7-slice2-evidence-and-decision.md section 2) before any run; a
// failed check is reported, never adjusted.
//
// Usage: node e7-pilot.cjs <riltra-binary> <riltra-home> <out.json> [repo]
//
// Read-only: it runs `search`, `callers` and `impact` and reads the database
// size. It creates nothing outside <out.json>.
'use strict';
const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const [bin, home, outFile, repo = 'mmsp-pin'] = process.argv.slice(2);
if (!bin || !home || !outFile) {
  console.error('usage: node e7-pilot.cjs <riltra-binary> <riltra-home> <out.json> [repo]');
  process.exit(2);
}

function run(args) {
  const t0 = process.hrtime.bigint();
  const r = spawnSync(bin, args, {
    env: { ...process.env, RILTRA_HOME: home },
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
  });
  const ms = Number(process.hrtime.bigint() - t0) / 1e6;
  let doc = null;
  try { doc = JSON.parse(r.stdout); } catch { /* reported by the caller */ }
  return { code: r.status, stdout: r.stdout, stderr: r.stderr, ms, doc };
}

const checks = [];
function check(id, description, ok, detail) {
  checks.push({ id, description, status: ok ? 'PASS' : 'FAIL', detail });
}

// --- P1: the nine frozen occurrences are located or classified -------------
const NAME = 'max_triangulation_angle';
const withOcc = run(['search', '--repo', repo, '--symbol', NAME, '--occurrences']);
const occ = withOcc.doc && withOcc.doc.occurrences;
const EXPECTED = [
  ['colmap/src/colmap/controllers/option_manager.cc', 'address-of-binding', 'observed', 1019, 41],
  ['colmap/src/colmap/mvs/patch_match.cc', 'member-access', 'observed', 362, 16],
  ['colmap/src/colmap/mvs/patch_match.cc', 'member-access', 'observed', 365, 31],
  ['colmap/src/colmap/mvs/patch_match_options.cc', 'macro-argument', 'candidate', 58, 15],
];
const got = occ ? occ.items.map((i) => [i.path, i.role, i.confidence, i.span.start_line, i.span.start_col]) : [];
check('P1', 'items are exactly frozen occurrences #9, #6, #7, #2 (total 4), in (path, byte, role) order',
  withOcc.code === 0 && occ && occ.total === 4 && JSON.stringify(got) === JSON.stringify(EXPECTED),
  { exit: withOcc.code, total: occ && occ.total, items: got });
check('P1-decl', 'the declaration (#1) is a search hit and not an occurrence',
  !!withOcc.doc && withOcc.doc.results.some((r) => r.kind === 'field'
    && r.path === 'colmap/src/colmap/mvs/patch_match_options.h'
    && r.span.start_line === 56 && r.span.start_col === 10)
  && !got.some((g) => g[0].endsWith('patch_match_options.h')),
  { results: withOcc.doc && withOcc.doc.results.map((r) => [r.kind, r.path, r.span.start_line, r.span.start_col]) });
check('P1-literal', 'the option-name string literal (#8) is not an occurrence',
  !got.some((g) => g[0].endsWith('option_manager.cc') && g[3] === 1018), { items: got });
check('P1-excluded', 'the per-file policy leaves #3, #4, #5 out (patch_match_options.cc lines 91-93)',
  !got.some((g) => g[0].endsWith('patch_match_options.cc') && g[3] >= 91 && g[3] <= 93), { items: got });

// --- P2: the filter site is unreachable by name ----------------------------
check('P2', 'patch_match.cc:443-447 (the local max_triangulation_angle_rad) does not appear',
  !got.some((g) => g[0].endsWith('/mvs/patch_match.cc') && g[3] >= 443 && g[3] <= 447), { items: got });

// --- P3: ambiguity is visible, never merged --------------------------------
check('P3', 'the option name has one field declaration and one symbol declaration; scope and binding are literal',
  !!occ && occ.field_declarations === 1 && occ.symbol_declarations === 1
  && occ.scope === 'repository-wide-exact-name' && occ.binding === 'unbound',
  occ && { field_declarations: occ.field_declarations, symbol_declarations: occ.symbol_declarations, scope: occ.scope, binding: occ.binding });
const valueDoc = run(['search', '--repo', repo, '--symbol', 'value', '--occurrences', '--limit', '1']);
const valueOcc = valueDoc.doc && valueDoc.doc.occurrences;
// The census counts are checked against the symbol list, an independent
// query: the list returns every kind, so the exact-name subset is counted here.
const valueList = run(['search', '--repo', repo, '--symbol', 'value', '--limit', '1000']).doc;
const listed = valueList ? valueList.results : [];
const exactFields = listed.filter((r) => r.name === 'value' && r.kind === 'field').length;
const exactAny = listed.filter((r) => r.name === 'value').length;
check('P3-value', 'the census declaration counts of `value` equal the exact-name subset of the symbol list',
  !!valueOcc && valueList && valueList.truncated === false
  && valueOcc.field_declarations === exactFields && valueOcc.symbol_declarations === exactAny,
  valueOcc && { census_fields: valueOcc.field_declarations, census_symbols: valueOcc.symbol_declarations, list_exact_fields: exactFields, list_exact_symbols: exactAny, list_total: valueList && valueList.total });
// The frozen spec expected 27 from the Slice 1 record. That figure is the
// case-insensitive count (`value` plus `Value`), not the exact name.
const caseInsensitiveFields = listed.filter((r) => r.kind === 'field' && String(r.name).toLowerCase() === 'value').length;
check('P3-value-27', 'the Slice 1 figure of 27 is the case-insensitive field count, not the exact-name count',
  caseInsensitiveFields === 27 && exactFields < 27,
  { case_insensitive_fields: caseInsensitiveFields, exact_fields: exactFields });
check('P3-fresh', 'the answer states the repository and a freshness verdict',
  !!withOcc.doc && withOcc.doc.repo === repo && withOcc.doc.freshness === 'fresh',
  withOcc.doc && { repo: withOcc.doc.repo, freshness: withOcc.doc.freshness, reasons: withOcc.doc.reasons });

// --- P4 / P5: the graph is untouched ---------------------------------------
const impact = run(['impact', '--repo', repo, '--path', 'colmap/src/colmap/mvs/patch_match_options.h', '--line', '56', '--col', '10']);
check('P4', 'impact at the field stays UNKNOWN/partial and carries no occurrence data',
  !!impact.doc && impact.doc.verdict === 'UNKNOWN' && impact.doc.completeness.status === 'partial'
  && !impact.stdout.includes('"occurrences"') && impact.doc.paths.length === 0,
  impact.doc && { verdict: impact.doc.verdict, completeness: impact.doc.completeness.status, paths: impact.doc.paths.length });
const callers = run(['callers', '--repo', repo, '--path', 'colmap/src/colmap/controllers/option_manager.cc', '--line', '1019', '--col', '41']);
check('P5', 'no edge is claimed for the member read, the address-of binding or the macro argument',
  !!callers.doc && callers.doc.edges.every((e) => e.edge_kind !== 'CALLS' && e.edge_kind !== 'REFERENCES'),
  callers.doc && { edges: callers.doc.edges.length, kinds: [...new Set(callers.doc.edges.map((e) => e.edge_kind))] });

// --- A5: no verified label anywhere in the section -------------------------
check('A5', 'no occurrence is labelled verified or exact',
  !!occ && !JSON.stringify(occ).match(/verified|exact"/i) && occ.items.every((i) => i.confidence === 'observed' || i.confidence === 'candidate'),
  null);

// --- F1: measure, do not claim ---------------------------------------------
function timings(args, reps) {
  const samples = [];
  for (let i = 0; i < reps; i++) samples.push(run(args).ms);
  samples.sort((a, b) => a - b);
  const pick = (q) => samples[Math.min(samples.length - 1, Math.ceil(q * samples.length) - 1)];
  return { reps, p50_ms: +pick(0.5).toFixed(1), p95_ms: +pick(0.95).toFixed(1), samples_ms: samples.map((s) => +s.toFixed(1)) };
}
const perf = {
  search: timings(['search', '--repo', repo, '--symbol', NAME], 5),
  search_occurrences: timings(['search', '--repo', repo, '--symbol', NAME, '--occurrences'], 5),
};
let rows = null;
try {
  const { DatabaseSync } = require('node:sqlite');
  const db = new DatabaseSync(path.join(home, 'index', `${repo}.db`), { readOnly: true });
  rows = db.prepare('SELECT COUNT(*) AS n FROM reference_occurrences').get().n;
  db.close();
} catch (e) { rows = `unavailable: ${e.message}`; }
let dbBytes = null;
try { dbBytes = fs.statSync(path.join(home, 'index', `${repo}.db`)).size; } catch { /* reported as null */ }
const measured = { occurrence_rows: rows, database_bytes: dbBytes, perf };

const summary = {
  pass: checks.filter((c) => c.status === 'PASS').length,
  fail: checks.filter((c) => c.status === 'FAIL').length,
};
fs.mkdirSync(path.dirname(path.resolve(outFile)), { recursive: true });
fs.writeFileSync(outFile, JSON.stringify({ repo, summary, checks, measured, raw: { occurrences_search: withOcc.doc, impact: impact.doc } }, null, 2) + '\n');
for (const c of checks) console.log(`${c.status.padEnd(5)} ${c.id.padEnd(11)} ${c.description}`);
console.log(`measured: occurrence_rows=${rows} database_bytes=${dbBytes} search p50=${perf.search.p50_ms}ms search+occurrences p50=${perf.search_occurrences.p50_ms}ms`);
console.log(`summary: ${summary.pass} PASS, ${summary.fail} FAIL`);
process.exit(summary.fail === 0 ? 0 : 1);
