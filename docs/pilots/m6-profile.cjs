// SPDX-License-Identifier: Apache-2.0
// Build an instrumented archive in target/, leaving production source and binary untouched.
// node docs/pilots/m6-profile.cjs <completed pilot directory>
const fs = require('node:fs');
const path = require('node:path');
const cp = require('node:child_process');
const assert = require('node:assert/strict');
const { DatabaseSync } = require('node:sqlite');
const root = path.resolve(__dirname, '../..');
assert(process.argv[2], 'supply a completed pilot directory');
const pilot = path.resolve(process.argv[2]);
assert.equal(path.dirname(pilot), path.join(root, 'target'), 'profile only an isolated fixture under target/');
assert(path.basename(pilot).startsWith('m6-pilot-') && fs.existsSync(path.join(pilot, 'run.json')),
  'profile only a completed m6-real-providers run');
const dest = fs.mkdtempSync(path.join(root, 'target/m6-profile-'));
const source = path.join(dest, 'source');
fs.mkdirSync(source);
function command(exe, args, opts = {}) {
  const r = cp.spawnSync(exe, args, { cwd: root, encoding: 'utf8', maxBuffer: 32 * 1024 * 1024, ...opts });
  assert.equal(r.status, 0, r.stderr);
  return r.stdout;
}
const baseline = command('git', ['-c', 'safe.directory=' + root.replaceAll('\\', '/'), 'rev-parse', 'HEAD']).trim();
command('git', ['-c', 'safe.directory=' + root.replaceAll('\\', '/'), 'archive', '--format=tar', '--output=' + path.join(dest, 'source.tar'), baseline]);
command('tar', ['-xf', path.join(dest, 'source.tar'), '-C', source]);
const edgesFile = path.join(source, 'crates/cli/src/edges.rs');
let edges = fs.readFileSync(edgesFile, 'utf8');
// Drop guards also record early returns; total derivation includes nested proof work.
const guard = `
struct PilotTimer(&'static str, std::time::Instant);
impl PilotTimer { fn new(name: &'static str) -> Self { Self(name, std::time::Instant::now()) } }
impl Drop for PilotTimer { fn drop(&mut self) { eprintln!("M6_PROFILE {} {}", self.0, self.1.elapsed().as_nanos()); } }
`;
edges = edges.replace('pub fn unify(', guard + '\npub fn unify(');
for (const name of ['source_proofs', 'derive_syntax_edges', 'upsert_semantic_edges', 'rederive_after_carry']) {
  const start = edges.indexOf('fn ' + name + '(');
  assert(start >= 0);
  const body = edges.indexOf(' {', start) + 2;
  edges = edges.slice(0, body) + `\n    let _pilot_timer = PilotTimer::new("${name}");` + edges.slice(body);
}
const old = 'proofs.insert(id, crate::syntax::relationship_proof(lang, &raw));';
assert(edges.includes(old));
edges = edges.replace(old, `let pilot_parse = std::time::Instant::now();
                ${old}
                eprintln!("M6_PROFILE proof_parse {}", pilot_parse.elapsed().as_nanos());`);
fs.writeFileSync(edgesFile, edges);
const build = cp.spawnSync('cargo', ['build', '--locked', '--offline', '--manifest-path', path.join(source, 'Cargo.toml'), '--target-dir', path.join(dest, 'build'), '-p', 'riltra'], {
  cwd: root, encoding: 'utf8', timeout: 600000, maxBuffer: 32 * 1024 * 1024,
});
fs.writeFileSync(path.join(dest, 'build.log'), build.stdout + build.stderr);
assert.equal(build.status, 0, build.stderr);
const exe = path.join(dest, 'build/debug/riltra.exe');
const home = path.join(dest, 'home');
// Artifact metadata points to the pilot root; reproduce the exact source there,
// and keep the profile database isolated. No synthetic SCIP rewriting.
const original = path.join(pilot, 'ts/code');
const main = path.join(original, 'main.ts');
const saved = fs.readFileSync(main);
const originalText = saved.toString().replace('// one-file refresh\n', '');
fs.mkdirSync(home);
let rows = [];
function run(label, args) {
  const started = performance.now();
  const r = cp.spawnSync(exe, args, { cwd: original, env: { ...process.env, RILTRA_HOME: home }, encoding: 'utf8' });
  assert.equal(r.status, 0, r.stderr);
  const events = r.stderr.split(/\r?\n/).filter(s => s.startsWith('M6_PROFILE ')).map(s => {
    const [, name, ns] = s.split(' '); return { name, ns: Number(ns) };
  });
  const json = JSON.parse(r.stdout);
  rows.push({ label, elapsed_ms: performance.now() - started, events, json });
  fs.writeFileSync(path.join(dest, label + '.log'), r.stdout + r.stderr);
  return json;
}
function findDb(dir) { for (const ent of fs.readdirSync(dir, { withFileTypes: true })) {
  const f = path.join(dir, ent.name); if (ent.isDirectory()) { const hit = findDb(f); if (hit) return hit; }
  else if (/\.(sqlite|sqlite3|db)$/.test(f)) return f;
} }
try {
  fs.writeFileSync(main, originalText);
  run('init', ['init', '--repo', original, '--name', 'profile']);
  run('full', ['index', '--repo', 'profile', '--full']);
  run('import', ['semantic', 'import', '--repo', 'profile', '--scip', path.join(original, 'index.scip')]);
  for (const [command, file, line, col] of [
    ['callers', 'lib.ts', '1', '17'], ['callees', 'lib.ts', '2', '8'],
    ['trace', 'main.ts', '2', '8'], ['impact', 'lib.ts', '1', '17'],
  ]) {
    run(command, [command, '--repo', 'profile', '--path', file, '--line', line, '--col', col]);
    assert.equal(rows.at(-1).events.length, 0, 'queries must not derive/reparse');
  }
  run('noop', ['index', '--repo', 'profile']);
  fs.appendFileSync(main, '// one-file refresh\n');
  run('delta', ['index', '--repo', 'profile']);
  run('reimport', ['semantic', 'import', '--repo', 'profile', '--scip', path.join(original, 'index.scip')]);
  const db = new DatabaseSync(findDb(home));
  db.prepare("DELETE FROM meta WHERE key='graph_proof'").run();
  db.close();
  run('revalidate', ['semantic', 'import', '--repo', 'profile', '--scip', path.join(original, 'index.scip')]);
} finally { fs.writeFileSync(main, saved); }
fs.writeFileSync(path.join(dest, 'profile.json'), JSON.stringify({ baseline, pilot, rows }, null, 2));
console.log(dest);
