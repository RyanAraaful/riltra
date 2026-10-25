// SPDX-License-Identifier: Apache-2.0
// Before/after comparison of two CP2 harness runs on the same pinned snapshot
// and the same host: the cold index, the incremental refresh, the warm
// queries, the graded workflows and the answers of the workflow queries.
//
// Usage:
//   node compare.cjs <base-log> <base-raw> <after-log> <after-raw> <out.md>
//
// It only reads the two archives and writes <out.md>. Nothing is graded here:
// the gradings come from the frozen grader (docs/pilots/cp2/method/grade.cjs)
// as archived in each run's grading.json, and a difference is reported, not
// judged.
'use strict';
const fs = require('node:fs');
const path = require('node:path');

const [baseLog, baseRaw, afterLog, afterRaw, outFile] = process.argv.slice(2);
if (!afterRaw || !outFile) {
  console.error('usage: node compare.cjs <base-log> <base-raw> <after-log> <after-raw> <out.md>');
  process.exit(2);
}
const read = (p) => fs.readFileSync(p, 'utf8');
const json = (p) => JSON.parse(read(p));
const exists = (p) => fs.existsSync(p);

function logFacts(file) {
  const text = read(file);
  const colds = [...text.matchAll(/index exit=0 wall_s=([\d.]+)/g)].map((m) => +m[1]);
  const dbs = [...text.matchAll(/db=mmsp-pin\.db bytes=(\d+)/g)].map((m) => +m[1]);
  const section = text.split('6. incremental correctness')[1] || '';
  const walls = [...section.matchAll(/exit=0 wall_ms=(\d+)/g)].map((m) => +m[1]);
  return { colds, dbs, noop: walls[0], delta: walls[1], oracle: walls[2] };
}

const pct = (a, b) => (a ? `${(((b - a) / a) * 100).toFixed(1)} %` : 'n/a');
const sec = (ms) => (ms == null ? 'n/a' : (ms / 1000).toFixed(1) + ' s');
const lines = [];
const out = (s = '') => lines.push(s);

const A = logFacts(baseLog);
const B = logFacts(afterLog);

out('# M9 release-candidate comparison: before vs after');
out();
out('Same host, same pinned snapshot, debug profile, the frozen CP2 harness. "Before" is the M9');
out('tip `f65c183`, "after" the release-candidate code. Single-host figures: no claim beyond them.');
out();
out('## Cold index (two independent runs each)');
out();
out('| | before | after | change |');
out('|---|---|---|---|');
for (let i = 0; i < 2; i++) {
  out(`| run ${i + 1} wall | ${A.colds[i]} s | ${B.colds[i]} s | ${pct(A.colds[i], B.colds[i])} |`);
}
const mean = (xs) => xs.reduce((s, x) => s + x, 0) / xs.length;
out(`| mean wall | ${mean(A.colds).toFixed(2)} s | ${mean(B.colds).toFixed(2)} s | ${pct(mean(A.colds), mean(B.colds))} |`);
out(`| database bytes | ${A.dbs[0]} | ${B.dbs[0]} | ${pct(A.dbs[0], B.dbs[0])} |`);
out();
out('## Incremental refresh and the independent full rebuild');
out();
out('| step | before | after | change |');
out('|---|---|---|---|');
out(`| refresh, nothing changed | ${sec(A.noop)} | ${sec(B.noop)} | ${pct(A.noop, B.noop)} |`);
out(`| refresh, one file changed | ${sec(A.delta)} | ${sec(B.delta)} | ${pct(A.delta, B.delta)} |`);
out(`| independent full rebuild | ${sec(A.oracle)} | ${sec(B.oracle)} | ${pct(A.oracle, B.oracle)} |`);
out();
for (const [label, raw] of [['before', baseRaw], ['after', afterRaw]]) {
  const f = path.join(raw, 'incremental', 'delta.json');
  if (exists(f)) {
    const d = json(f);
    out(`- ${label}: delta mode \`${d.mode}\`, reparsed ${d.reparsed}, reused ${d.reused}`);
  }
}
out();
out('## Warm queries (5 repeats each, p50 / p95 in ms)');
out();
out('| query | before p50 | before p95 | after p50 | after p95 | p50 change |');
out('|---|---|---|---|---|---|');
const pa = json(path.join(baseRaw, 'perf', 'warm-queries.json'));
const pb = json(path.join(afterRaw, 'perf', 'warm-queries.json'));
for (const a of pa) {
  const b = pb.find((x) => x.query === a.query);
  if (!b) continue;
  out(`| ${a.query} | ${a.ms_p50} | ${a.ms_p95} | ${b.ms_p50} | ${b.ms_p95} | ${pct(a.ms_p50, b.ms_p50)} |`);
}
out();
out('## The five workflows, graded by the frozen grader');
out();
const ga = json(path.join(baseRaw, 'grading.json'));
const gb = json(path.join(afterRaw, 'grading.json'));
const checks = (g) => Object.fromEntries(g.checks.map((c) => [c.id, c]));
const ca = checks(ga);
const cb = checks(gb);
const verdict = (c) => (c ? `${c.verdict}${c.reason ? ' (' + c.reason + ')' : ''}` : 'n/a');
out('| check | layer | before | after |');
out('|---|---|---|---|');
for (const id of Object.keys(cb)) {
  const a = ca[id];
  const b = cb[id];
  const mark = a && (a.verdict !== b.verdict || a.reason !== b.reason) ? ' **changed**' : '';
  out(`| ${id} | ${b.layer} | ${verdict(a)} | ${verdict(b)}${mark} |`);
}
out();
out(`Summary before: ${JSON.stringify(ga.summary.layers)}`);
out();
out(`Summary after: ${JSON.stringify(gb.summary.layers)}`);
out();
out('## What the workflow queries return');
out();
out('| query | before | after |');
out('|---|---|---|');
const queries = [
  'd1-1-search-Print', 'd1-4-search-max_triangulation', 'd1-5-callers-PrintDef', 'd1-6-callers-PrintDecl',
  'd2-1-callers-PatchMatchCtrlRun', 'd2-3-callees-CtrlRun', 'd2-4-callers-CreateWorkspace',
  'd2-5-callers-optmgr-binding', 'd3-1-trace-depth4', 'd4-1-impact-member', 'd4-2-impact-function',
  'p2-callers-class-PatchMatch',
];
function summarize(file) {
  if (!exists(file)) return 'missing';
  let d;
  try { d = JSON.parse(read(file)); } catch { return 'not JSON'; }
  if (Array.isArray(d.results)) return `${d.total} hits`;
  if (Array.isArray(d.edges)) {
    const files = [...new Set(d.edges.map((e) => path.basename(e.caller_path || '')))];
    return `${d.edges.length} edges${files.length ? ' (' + files.slice(0, 4).join(', ') + (files.length > 4 ? ', …' : '') + ')' : ''}; anchor ${JSON.stringify(d.anchor && d.anchor.span)}`;
  }
  if (Array.isArray(d.paths)) return `${d.verdict ? d.verdict + ', ' : ''}${d.paths.length} paths; anchor ${JSON.stringify(d.anchor && d.anchor.span)}`;
  return 'other';
}
for (const q of queries) {
  out(`| ${q} | ${summarize(path.join(baseRaw, 'queries', q + '.json'))} | ${summarize(path.join(afterRaw, 'queries', q + '.json'))} |`);
}
fs.writeFileSync(outFile, lines.join('\n') + '\n');
console.log(lines.join('\n'));
