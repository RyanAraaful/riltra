// SPDX-License-Identifier: Apache-2.0
// CP7-A deterministic CLI reference + parity comparator. Re-runs the pinned CLI
// through the read-only wrapper, then compares every representative result with
// the direct MCP result and the frozen golden. Key order is ignored.
"use strict";
const fs = require("node:fs");
const path = require("node:path");
const crypto = require("node:crypto");
const { spawnSync } = require("node:child_process");

const ROOT = "E:\\dev\\Riltra\\.cp7a";
const HOME = path.join(ROOT, "home-template");
const PRISTINE = path.join(ROOT, "home-pristine");
const OUT = path.join(ROOT, "reference");
const WRAPPER = path.join(ROOT, "bin", "riltra-ro.cjs");
const GOLDENS = path.join(ROOT, "goldens-raw");
const MAIN = ["registry.json", "index\\riltra.db", "index\\other.db", "knowledge\\riltra.db", "knowledge\\other.db"];

const commands = [
  { key: "repo-list", args: ["repo", "list"], golden: "repo-list.json" },
  { key: "repo-status-repo", args: ["repo", "status", "--repo", "riltra"], golden: "repo-status.json" },
  { key: "repo-status-index", args: ["index", "status", "--repo", "riltra"], golden: "index-status.json" },
  { key: "search-validate-id", args: ["search", "--repo", "riltra", "--symbol", "validate_id"], golden: "search-validate-id.json" },
  { key: "callers-validate-id", args: ["callers", "--repo", "riltra", "--path", "crates/cli/src/knowledge.rs", "--line", "89", "--col", "1"], golden: "callers-validate-id.json" },
  { key: "callees-validate-new", args: ["callees", "--repo", "riltra", "--path", "crates/cli/src/knowledge.rs", "--line", "199", "--col", "1"], golden: "callees-validate-new.json" },
  { key: "trace-validate-new-to-id", args: ["trace", "--repo", "riltra", "--path", "crates/cli/src/knowledge.rs", "--line", "199", "--col", "1", "--to", "crates/cli/src/knowledge.rs:89:1"], golden: "trace-validate-new-to-id.json" },
  { key: "impact-validate-id", args: ["impact", "--repo", "riltra", "--path", "crates/cli/src/knowledge.rs", "--line", "89", "--col", "1"], golden: "impact-validate-id.json" },
  { key: "potential-vs-calls", args: ["callers", "--repo", "other", "--path", "c/main.c", "--line", "6", "--col", "1"], golden: "callers-other-add.json" },
  { key: "knowledge-search-readonly", args: ["knowledge", "search", "--repo", "riltra", "--query", "read-only"], golden: "knowledge-search-readonly.json" },
  { key: "knowledge-revision-r1", args: ["knowledge", "show", "--repo", "riltra", "--id", "cp7a-decision-health", "--revision", "1"], golden: "knowledge-show-health-r1.json" },
  { key: "knowledge-revision-current", args: ["knowledge", "show", "--repo", "riltra", "--id", "cp7a-decision-health"], golden: "knowledge-show-health.json" },
  { key: "knowledge-related", args: ["knowledge", "related", "--repo", "riltra", "--id", "cp7a-decision-health"], golden: "knowledge-related-health.json" },
  { key: "knowledge-evidence-readonly", args: ["knowledge", "show", "--repo", "riltra", "--id", "cp7a-constraint-readonly"], golden: "knowledge-show-readonly.json" },
  { key: "stale-evidence", args: ["knowledge", "show", "--repo", "other", "--id", "cp7a-other-stale"], golden: "knowledge-show-stale.json" },
  { key: "missing-index-empty", args: ["search", "--repo", "empty", "--symbol", "main"], golden: "search-empty-main.json" },
  { key: "missing-knowledge-store-empty", args: ["knowledge", "search", "--repo", "empty", "--query", "anything"], golden: "knowledge-search-empty.json" },
  { key: "status", args: ["status"], golden: "status.json" },
  { key: "doctor", args: ["doctor"], golden: "doctor.json" }
];

function hash(p) { return crypto.createHash("sha256").update(fs.readFileSync(p)).digest("hex").toUpperCase(); }
function writeJson(p, v) { fs.writeFileSync(p, JSON.stringify(v, null, 2) + "\n"); }
function rm(p) { try { fs.rmSync(p, { recursive: true, force: true }); } catch {} }
function resetHome() {
  for (const d of ["index", "knowledge"]) { rm(path.join(HOME, d)); fs.mkdirSync(path.join(HOME, d), { recursive: true }); }
  for (const rel of MAIN) fs.copyFileSync(path.join(PRISTINE, rel), path.join(HOME, rel));
}
function homeState() {
  const main = {};
  for (const rel of MAIN) { const p = path.join(HOME, rel); main[rel] = fs.existsSync(p) ? hash(p) : null; }
  return { main, empty_index: fs.existsSync(path.join(HOME, "index\\empty.db")), empty_knowledge: fs.existsSync(path.join(HOME, "knowledge\\empty.db")) };
}
function changed(a, b) {
  return !a || !b || MAIN.some((rel) => a.main[rel] !== b.main[rel]) || b.empty_index || b.empty_knowledge;
}
function canon(x) {
  if (Array.isArray(x)) return x.map(canon);
  if (x && typeof x === "object") { const o = {}; for (const k of Object.keys(x).sort()) o[k] = canon(x[k]); return o; }
  return x;
}
function deepEqual(a, b) { return JSON.stringify(canon(a)) === JSON.stringify(canon(b)); }
function diff(a, b, p = "") {
  if (a === b) return null;
  if (a === null || b === null || typeof a !== typeof b || typeof a !== "object") return `${p || "/"} mcp/cli=${JSON.stringify(a)} expected=${JSON.stringify(b)}`;
  const keys = new Set([...Object.keys(a || {}), ...Object.keys(b || {})]);
  for (const k of keys) { const d = diff(a ? a[k] : undefined, b ? b[k] : undefined, p + "/" + k); if (d) return d; }
  return null;
}
function readJson(p) { return JSON.parse(fs.readFileSync(p, "utf8")); }

resetHome();
const before = homeState();
writeJson(path.join(OUT, "cli-home-before.json"), before);
const results = [];
for (const c of commands) {
  const t0 = Number(process.hrtime.bigint() / 1000000n);
  const r = spawnSync(process.execPath, [WRAPPER, ...c.args], {
    cwd: "E:\\dev\\Riltra",
    env: { ...process.env, RILTRA_HOME: HOME },
    encoding: "utf8",
    windowsHide: true,
    timeout: 90000,
    maxBuffer: 64 * 1024 * 1024
  });
  const elapsed = Number(process.hrtime.bigint() / 1000000n) - t0;
  let json = null, parse_error = null;
  try { json = JSON.parse(r.stdout); } catch (e) { parse_error = String(e && e.message || e); }
  results.push({ key: c.key, args: c.args, golden: c.golden, exit_code: r.status, elapsed_ms: elapsed, stdout: r.stdout, stderr: r.stderr, json, parse_error });
}
const after = homeState();
writeJson(path.join(OUT, "cli-home-after.json"), after);
writeJson(path.join(OUT, "cli-results.json"), { schema: "cp7a-cli-reference-v1", generated_at: new Date().toISOString(), wrapper: WRAPPER, home_before: before, home_after: after, home_changed: changed(before, after), results });

const mcp = readJson(path.join(OUT, "mcp-results.json"));
const mcpByKey = new Map(mcp.calls.map((c) => [c.key, c]));
const cliByKey = new Map(results.map((c) => [c.key, c]));
const golden = (name) => readJson(path.join(GOLDENS, name));
const compositeRepoStatus = {
  ...cliByKey.get("repo-status-repo").json,
  index_status: cliByKey.get("repo-status-index").json
};
const comparisons = [];
function addComparison(key, cliJson, mcpJson, goldenJson, note, composite = false) {
  const cliEqual = goldenJson === undefined ? null : deepEqual(cliJson, goldenJson);
  const mcpEqual = deepEqual(mcpJson, cliJson);
  comparisons.push({ key, composite, note, cli_vs_golden: cliEqual, mcp_vs_cli: mcpEqual, diff_cli_vs_golden: cliEqual === false ? diff(cliJson, goldenJson) : null, diff_mcp_vs_cli: mcpEqual ? null : diff(mcpJson, cliJson) });
}
for (const c of mcp.calls) {
  if (c.key === "repo-status") {
    const repoGolden = golden("repo-status.json");
    const indexGolden = golden("index-status.json");
    const mcpRepoPart = { ...c.json };
    delete mcpRepoPart.index_status;
    addComparison(c.key + ":repo", cliByKey.get("repo-status-repo").json, mcpRepoPart, repoGolden, "CLI repo status component");
    addComparison(c.key + ":index", cliByKey.get("repo-status-index").json, c.json.index_status, indexGolden, "CLI index status component");
    addComparison(c.key, compositeRepoStatus, c.json, undefined, "MCP repo_status combines CLI repo status + index status", true);
  } else if (c.key === "health") {
    const statusGolden = golden("status.json");
    const doctorGolden = golden("doctor.json");
    addComparison("health:status", cliByKey.get("status").json, c.json.status, statusGolden, "MCP health.status vs CLI status");
    addComparison("health:doctor", cliByKey.get("doctor").json, c.json.doctor, doctorGolden, "MCP health.doctor vs CLI doctor");
    const repoCount = (cliByKey.get("repo-list").json.repos || []).length;
    const countEqual = c.json.repo_count === repoCount;
    comparisons.push({ key: "health:repo_count", composite: true, note: "MCP health.repo_count vs CLI repo list count", cli_vs_golden: null, mcp_vs_cli: countEqual, diff_cli_vs_golden: null, diff_mcp_vs_cli: countEqual ? null : `repo_count mcp=${c.json.repo_count} cli=${repoCount}` });
  } else {
    const cli = cliByKey.get(c.key);
    addComparison(c.key, cli ? cli.json : null, c.json, golden(c.golden), "representative operation");
  }
}
const allMcpVsCli = comparisons.every((c) => c.mcp_vs_cli === true);
const allCliVsGolden = comparisons.filter((c) => c.cli_vs_golden !== null).every((c) => c.cli_vs_golden === true);
const cliHomeChanged = changed(before, after);
const pass = allMcpVsCli && allCliVsGolden && !cliHomeChanged && !mcp.home_changed && mcp.tools_exact_expected === true && mcp.server_alive_after_error === true;
const report = {
  schema: "cp7a-parity-v1",
  generated_at: new Date().toISOString(),
  mcp: {
    protocol: mcp.protocol_negotiated,
    tools_count: mcp.tools_count,
    tools_exact_expected: mcp.tools_exact_expected,
    server_alive_after_error: mcp.server_alive_after_error,
    home_changed: mcp.home_changed,
    binary_sha256: mcp.binary_sha256
  },
  cli: { commands: results.length, home_changed: cliHomeChanged, wrapper: WRAPPER },
  comparisons,
  summary: {
    compared: comparisons.length,
    mcp_vs_cli_equal: comparisons.filter((c) => c.mcp_vs_cli).length,
    cli_vs_golden_equal: comparisons.filter((c) => c.cli_vs_golden === true).length,
    cli_vs_golden_compared: comparisons.filter((c) => c.cli_vs_golden !== null).length,
    all_mcp_vs_cli: allMcpVsCli,
    all_cli_vs_golden: allCliVsGolden,
    pass,
    g5_note: "MCP repo_status and health are documented composites: repo_status = CLI repo status + index status; health.status/doctor are the CLI status/doctor outputs plus capabilities/limits. The core objects are deep-equal; key order ignored."
  }
};
writeJson(path.join(OUT, "compare.json"), report);
console.log(JSON.stringify(report.summary, null, 2));
if (!pass) {
  for (const c of comparisons.filter((x) => !x.mcp_vs_cli || x.cli_vs_golden === false)) console.log(JSON.stringify(c, null, 2));
  process.exitCode = 2;
}
