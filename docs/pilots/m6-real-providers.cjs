// SPDX-License-Identifier: Apache-2.0
// Opt-in Windows pilot. No downloads, installations, package scripts or source execution.
// node docs/pilots/m6-real-providers.cjs <scip-typescript main.js> <rust-analyzer.exe>
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const cp = require('node:child_process');
const crypto = require('node:crypto');
const assert = require('node:assert/strict');
const { DatabaseSync } = require('node:sqlite');
const root = path.resolve(__dirname, '../..');
const scip = path.resolve(process.argv[2] || '');
const ra = path.resolve(process.argv[3] || '');
assert(process.argv.length === 4 && fs.existsSync(scip) && fs.existsSync(ra), 'supply two existing provider paths');
const work = fs.mkdtempSync(path.join(root, 'target/m6-pilot-'));
const exe = path.join(root, 'target/debug/riltra.exe');
const records = [];
const write = (file, text) => { fs.mkdirSync(path.dirname(file), { recursive: true }); fs.writeFileSync(file, text); };
function launch(label, binary, args, cwd, home, expected = 0) {
  const result = cp.spawnSync('pwsh', ['-NoProfile', '-File', path.join(__dirname, 'm6-measure.ps1')], {
    input: JSON.stringify({ exe: binary, args, cwd }), encoding: 'utf8', timeout: 180000,
    env: { ...process.env, RILTRA_HOME: home, CARGO_NET_OFFLINE: 'true', CARGO_TERM_COLOR: 'never' },
    maxBuffer: 16 * 1024 * 1024,
  });
  assert.equal(result.status, 0, result.stderr);
  const measured = JSON.parse(result.stdout);
  write(path.join(work, label + '.json'), JSON.stringify(measured, null, 2));
  records.push({ label, exe: binary, args, cwd, home, exit: measured.exit,
    elapsed_ms: measured.elapsed_ms, peak_working_set_bytes: measured.peak_working_set_bytes,
    output_bytes: Buffer.byteLength(measured.stdout) });
  assert.equal(measured.exit, expected, `${label}: ${measured.stderr}`);
  return measured.stdout.trim() ? JSON.parse(measured.stdout) : { error: measured.stderr, exit: measured.exit };
}
function native(label, project, args, expected = 0) {
  return launch(label, exe, args, project.repo, project.home, expected);
}
function project(name, files) {
  const repo = path.join(work, name, 'code'), home = path.join(work, name, 'home');
  fs.mkdirSync(home, { recursive: true });
  for (const [file, text] of Object.entries(files)) write(path.join(repo, file), text);
  const p = { repo, home, name };
  native(name + '-init', p, ['init', '--repo', repo, '--name', name]);
  return p;
}
function dbState(p) {
  // The on-disk name is discovered instead of relying on a private filename.
  function find(dir) { for (const ent of fs.readdirSync(dir, { withFileTypes: true })) {
    const f = path.join(dir, ent.name); if (ent.isDirectory()) { const hit = find(f); if (hit) return hit; }
    else if (/\.(sqlite|sqlite3|db)$/.test(f)) return f;
  } }
  const db = new DatabaseSync(find(p.home), { readOnly: true });
  const result = {};
  for (const table of ['meta', 'snapshots', 'files', 'semantic_imports', 'semantic_facts', 'code_edges'])
    result[table] = db.prepare(`SELECT * FROM ${table} ORDER BY rowid`).all();
  db.close();
  return result;
}
const ts = project('ts', {
  'package.json': '{"name":"riltra-m6-real-pilot","version":"1.0.0","private":true}\n',
  'tsconfig.json': '{"compilerOptions":{"strict":true,"target":"ES2020","module":"commonjs","noEmit":true},"include":["*.ts"]}\n',
  '.gitignore': 'index.scip\n',
  'lib.ts': 'export function leaf(): number { return 7; }\nexport function middle(): number { return leaf(); }\nexport function referenceOnly() { return leaf; }\nexport function ambiguous(leaf: () => number) { return leaf(); }\n',
  'main.ts': 'import { middle, leaf } from "./lib";\nexport function entry(): number { return middle(); }\nexport function borrowed() { const alias = leaf; return alias(); }\nexport function localOwner() { function local() { return 1; } return local(); }\n',
  'other.ts': 'export function leaf() { return 99; }\nexport function localOwner() { function local() { return 2; } return local(); }\n',
});
const full = native('ts-full', ts, ['index', '--repo', 'ts', '--full']);
const before = dbState(ts);
// External producer output is text rather than Riltra JSON; preserve it separately.
const producer = cp.spawnSync('pwsh', ['-NoProfile', '-File', path.join(__dirname, 'm6-measure.ps1')], {
  input: JSON.stringify({ exe: process.execPath, args: [scip, 'index', '--cwd', ts.repo, '--output', 'index.scip', '--no-progress-bar'], cwd: ts.repo }),
  encoding: 'utf8', timeout: 180000, maxBuffer: 16 * 1024 * 1024,
});
assert.equal(producer.status, 0, producer.stderr);
const produced = JSON.parse(producer.stdout);
write(path.join(work, 'ts-producer.json'), JSON.stringify(produced, null, 2));
assert.equal(produced.exit, 0, produced.stderr);
const artifact = path.join(ts.repo, 'index.scip');
const imported = native('ts-import', ts, ['semantic', 'import', '--repo', 'ts', '--scip', artifact]);
const after = dbState(ts);
assert.deepEqual(after.snapshots, before.snapshots);
assert.deepEqual(after.files, before.files);
const query = (label, command, file, line, col, extra = []) => native(label, ts,
  [command, '--repo', 'ts', '--path', file, '--line', String(line), '--col', String(col), ...extra]);
const responses = {
  callers: query('ts-callers', 'callers', 'lib.ts', 1, 17),
  callees: query('ts-callees', 'callees', 'lib.ts', 2, 17),
  trace: query('ts-trace', 'trace', 'main.ts', 2, 17),
  impact: query('ts-impact', 'impact', 'lib.ts', 1, 17),
  references: query('ts-reference', 'callees', 'lib.ts', 3, 17),
  ambiguous: query('ts-parameter', 'callees', 'lib.ts', 4, 17),
  alias: query('ts-alias', 'callees', 'main.ts', 3, 17),
  other: query('ts-other', 'callers', 'other.ts', 1, 17),
  fallbackCallees: query('ts-fallback-callees', 'callees', 'lib.ts', 2, 8),
  fallbackTrace: query('ts-fallback-trace', 'trace', 'main.ts', 2, 8),
  fallbackParameter: query('ts-fallback-parameter', 'callees', 'lib.ts', 4, 8),
  fallbackAlias: query('ts-fallback-alias', 'callees', 'main.ts', 3, 8),
};
const meta = state => Object.fromEntries(state.meta.map(row => [row.key, row.value]));
assert.equal(meta(after).semantic_generation, '1');
assert.equal(meta(after).graph_proof, `m6-task7-source-proof-1:${after.snapshots[0].nonce}:1`);
assert.equal(imported.verification, 'Unverified');
assert(responses.callers.edges.some(e => e.kind === 'CALLS' && e.precision === 'Exact' && e.verification === 'Unverified'));
assert.deepEqual(responses.fallbackCallees.edges[0].evidence.terminal_span, [87, 91]);
assert.equal(fs.readFileSync(path.join(ts.repo, 'lib.ts'), 'utf8').slice(87, 91), 'leaf');
assert(responses.callers.edges.some(e => e.kind === 'REFERENCES' && e.evidence.span[0] === 138));
for (const key of ['fallbackParameter', 'fallbackAlias']) {
  assert(responses[key].edges.some(e => e.kind === 'POTENTIAL_CALL' && e.callee === null));
  assert(!responses[key].edges.some(e => e.kind === 'CALLS'));
}
assert.equal(responses.other.edges.length, 0, 'same name in another file cannot link');
assert.equal(responses.impact.verdict, 'UNKNOWN');
assert.equal(responses.impact.completeness.status, 'partial');
const localCalls = after.code_edges.filter(e => e.edge_kind === 'CALLS' && e.callee_uri.startsWith('local '));
assert.equal(localCalls.length, 2);
for (const edge of localCalls) assert.equal(edge.callee_doc, edge.caller_file);
assert.deepEqual(after.code_edges.filter(e => e.edge_kind === 'CALLS')
  .map(e => `${e.caller_file}:${e.callee_name}`).sort(),
['lib.ts:leaf', 'main.ts:local', 'main.ts:middle', 'other.ts:local']);
assert(responses.fallbackTrace.paths.some(p => p.edges.some(e => e.kind === 'CALLS'
  && e.caller_path === 'main.ts' && e.callee.symbol_uri.endsWith('`lib.ts`/middle().')
  && e.evidence.terminal_span[0] === 79 && e.evidence.terminal_span[1] === 85)));
for (const edge of after.code_edges.filter(e => e.edge_kind === 'CALLS')) {
  assert(after.semantic_facts.some(f => f.import_id === edge.callee_import_id
    && f.symbol_uri === edge.callee_uri && (f.roles & 1) !== 0
    && (edge.callee_doc === null || f.rel_path === edge.callee_doc)), 'same-import target definition proof');
}
for (let repeat = 1; repeat <= 5; repeat++) {
  for (const [key, command, file, line, col] of [
    ['callers', 'callers', 'lib.ts', 1, 17],
    ['fallbackCallees', 'callees', 'lib.ts', 2, 8],
    ['fallbackTrace', 'trace', 'main.ts', 2, 8],
    ['impact', 'impact', 'lib.ts', 1, 17],
  ]) assert.deepEqual(query(`ts-warm-${command}-${repeat}`, command, file, line, col), responses[key]);
}
assert.deepEqual(query('ts-deterministic', 'callers', 'lib.ts', 1, 17), responses.callers);
const firstPage = query('ts-page-1', 'callers', 'lib.ts', 1, 17, ['--limit', '1']);
const secondPage = query('ts-page-2', 'callers', 'lib.ts', 1, 17, ['--limit', '1', '--offset', '1']);
assert.deepEqual(firstPage.edges, responses.callers.edges.slice(0, 1));
assert.deepEqual(secondPage.edges, responses.callers.edges.slice(1, 2));
const broken = path.join(work, 'truncated.scip');
const rawArtifact = fs.readFileSync(artifact);
write(broken, rawArtifact.subarray(0, rawArtifact.length - 3));
native('ts-failed-import', ts, ['semantic', 'import', '--repo', 'ts', '--scip', broken], 3);
assert.deepEqual(dbState(ts), after, 'mid-stream import failure rolls back all graph and semantic state');
const noop = native('ts-noop', ts, ['index', '--repo', 'ts']);
assert.equal(noop.reparsed, 0);
fs.appendFileSync(path.join(ts.repo, 'main.ts'), '// one-file refresh\n');
const stale = query('ts-stale', 'callers', 'lib.ts', 1, 17);
assert.equal(stale.verification, 'Stale');
const delta = native('ts-delta', ts, ['index', '--repo', 'ts']);
assert.equal(delta.reparsed, 1);
const deltaState = dbState(ts);
const reimport = native('ts-reimport', ts, ['semantic', 'import', '--repo', 'ts', '--scip', artifact]);
const reimportState = dbState(ts);
assert.deepEqual(reimportState.snapshots, deltaState.snapshots);
assert.deepEqual(reimportState.files, deltaState.files);
assert.equal(Number(meta(reimportState).semantic_generation), 2);
const ambiguousImport = native('ts-duplicate-import-anchor', ts,
  ['callers', '--repo', 'ts', '--path', 'lib.ts', '--line', '1', '--col', '17'], 3);
assert(ambiguousImport.error.includes('ambiguous'));
const revalidationDb = new DatabaseSync((function find(dir) {
  for (const ent of fs.readdirSync(dir, { withFileTypes: true })) {
    const f = path.join(dir, ent.name); if (ent.isDirectory()) { const hit = find(f); if (hit) return hit; }
    else if (/\.(sqlite|sqlite3|db)$/.test(f)) return f;
  }
})(ts.home));
revalidationDb.prepare("DELETE FROM meta WHERE key='graph_proof'").run();
revalidationDb.close();
const incompatible = native('ts-incompatible', ts, ['callers', '--repo', 'ts', '--path', 'lib.ts', '--line', '1', '--col', '17'], 3);
assert(incompatible.error.includes('graph-proof-incompatible'));
const revalidated = native('ts-revalidate', ts, ['semantic', 'import', '--repo', 'ts', '--scip', artifact]);
const validatedState = dbState(ts);
assert.deepEqual(validatedState.snapshots, deltaState.snapshots);
assert.equal(Number(meta(validatedState).semantic_generation), 3);
const rs = project('rs', {
  'Cargo.toml': '[workspace]\n\n[package]\nname = "riltra-m6-lsp-pilot"\nversion = "0.1.0"\nedition = "2021"\n',
  '.gitignore': '/target/\nCargo.lock\n',
  'src/lib.rs': 'pub fn leaf() -> i32 { 7 }\npub fn caller() -> i32 { leaf() }\npub fn reference_only() -> fn() -> i32 { leaf }\n',
});
native('rs-full', rs, ['index', '--repo', 'rs', '--full']);
const rsBefore = dbState(rs);
const lspArgs = ['semantic', 'query', '--repo', 'rs', '--path', 'src/lib.rs', '--line', '2', '--col', '26'];
const rsSyntax = native('rs-core', rs, lspArgs, 3);
const rsDefinition = native('rs-definition', rs, [...lspArgs, '--lsp-server', ra]);
const rsReferences = native('rs-references', rs,
  ['semantic', 'query', '--repo', 'rs', '--path', 'src/lib.rs', '--line', '1', '--col', '8', '--refs', '--lsp-server', ra]);
const rsUnavailable = native('rs-unavailable', rs, [...lspArgs, '--lsp-server', path.join(work, 'absent.exe')]);
const rsAfter = dbState(rs);
assert.deepEqual(rsAfter, rsBefore, 'LSP must not write persisted facts, edges or metadata');
assert.equal(rsDefinition.provider, 'lsp-subprocess');
assert(rsDefinition.definitions.length > 0, 'real definition must be non-empty');
assert(rsReferences.references.some(hit => hit.span.byte_start === 52), 'real call reference');
assert(rsReferences.references.some(hit => hit.span.byte_start === 102), 'real non-call reference');
for (const hit of [...rsDefinition.definitions, ...rsReferences.references]) {
  assert.equal(hit.symbol_uri, null);
  assert.equal(hit.precision, 'Heuristic');
  assert.equal(hit.verification, 'Unverified');
  const text = fs.readFileSync(path.join(rs.repo, hit.path), 'utf8');
  assert.equal(text.slice(hit.span.byte_start, hit.span.byte_end), 'leaf');
}
assert.equal(rsAfter.semantic_imports.length, 0);
assert.equal(rsAfter.semantic_facts.length, 0);
assert(!rsAfter.code_edges.some(e => e.edge_kind === 'CALLS' || e.provider === 'lsp-subprocess'));
native('rs-syntax-after-unavailable', rs, ['callees', '--repo', 'rs', '--path', 'src/lib.rs', '--line', '2', '--col', '8']);
write(path.join(work, 'rust.json'), JSON.stringify({ rsSyntax, rsDefinition, rsReferences, rsUnavailable, rsBefore, rsAfter }, null, 2));
write(path.join(work, 'source-state-before.json'), JSON.stringify(before, null, 2));
write(path.join(work, 'source-state-after.json'), JSON.stringify(after, null, 2));
write(path.join(work, 'responses.json'), JSON.stringify(responses, null, 2));
write(path.join(work, 'run.json'), JSON.stringify({ work, environment: { platform: os.platform(), release: os.release(), arch: os.arch(), cpus: os.cpus(), memory_bytes: os.totalmem(), node: process.version }, full, imported, noop, delta, reimport, revalidated,
  artifact_sha256: crypto.createHash('sha256').update(fs.readFileSync(artifact)).digest('hex'), produced, records }, null, 2));
console.log(work);
