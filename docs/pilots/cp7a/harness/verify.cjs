// CP7-A final independent verification: frozen hashes, source pin, raw/aggregate
// consistency, isolation exposure, no-empty-store, docs evidence sanity, credentials scan.
// Reads the frozen .cp7a state and the committed docs/pilots/cp7a evidence copies.
"use strict";
const fs = require("node:fs");
const path = require("node:path");
const crypto = require("node:crypto");
const { execSync } = require("node:child_process");

const REPO = "E:/dev/Riltra";
const CP = path.join(REPO, ".cp7a");
const DOCS = path.join(REPO, "docs", "pilots", "cp7a");
const checks = [];
function check(name, ok, detail) {
  checks.push({ name, ok: Boolean(ok), detail: detail === undefined ? "" : String(detail) });
  console.log((ok ? "PASS  " : "FAIL  ") + name + (detail !== undefined ? "  :: " + detail : ""));
}
function sha256File(p) {
  return crypto.createHash("sha256").update(fs.readFileSync(p)).digest("hex").toUpperCase();
}
function readJson(p) {
  return JSON.parse(fs.readFileSync(p, "utf8"));
}
function walk(dir, out) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) walk(p, out);
    else out.push(p);
  }
  return out;
}

const env = readJson(path.join(CP, "frozen", "environment.json"));
const summary = readJson(path.join(CP, "summary.json"));

// ---------- 1. frozen hashes ----------
const corpusHash = sha256File(path.join(CP, "corpus", "corpus.json"));
check("corpus.sha256 file hash", corpusHash === "D9F33A765A98723B0ACFD314865C976FB8BD58A680C911932B64C83E8530C824", corpusHash);
check("summary.corpus_sha256", summary.corpus_sha256 === corpusHash, summary.corpus_sha256);
check("environment.corpus_sha256", env.corpus_sha256 === corpusHash, env.corpus_sha256);
check("docs corpus copy hash", sha256File(path.join(DOCS, "corpus", "corpus.json")) === corpusHash, "docs/pilots/cp7a/corpus/corpus.json");

const tokenRuleHash = sha256File(path.join(CP, "frozen", "token-rules.md"));
check("summary.token_rule_sha256", summary.token_rule_sha256 === tokenRuleHash, summary.token_rule_sha256);
check("docs token-rules copy hash", sha256File(path.join(DOCS, "method", "token-rules.md")) === tokenRuleHash, "docs/pilots/cp7a/method/token-rules.md");

// ---------- 2. source pin ----------
const sourceHead = execSync("git rev-parse HEAD", { cwd: path.join(CP, "source") }).toString().trim();
const sourceStatus = execSync("git status --porcelain", { cwd: path.join(CP, "source") }).toString().trim();
check("source HEAD == baseline", sourceHead === "0fd22a1134274c1e92f741fc0d4e3b4886ceeaf3", sourceHead);
check("source HEAD == corpus.source_head", sourceHead === readJson(path.join(CP, "corpus", "corpus.json")).source_head, "");
check("source worktree clean", sourceStatus === "", sourceStatus || "(clean)");

// ---------- 3. binary + configs used for measurement ----------
const binHash = sha256File(path.join(REPO, "target", "debug", "riltra.exe"));
check("binary sha256", binHash === env.binary_sha256, binHash);
check("docs global config == frozen global config", sha256File(path.join(DOCS, "method", "global-opencode.cp7a.jsonc")) === env.global_config_sha256, env.global_config_sha256);
for (const arm of ["a", "b"]) {
  const dir = path.join(DOCS, "harness", "configs", `arm-${arm}`);
  check(`docs arm-${arm} opencode.jsonc == frozen arm config`, sha256File(path.join(dir, "opencode.jsonc")) === env.arm_config_sha256[arm.toUpperCase()], env.arm_config_sha256[arm.toUpperCase()]);
}
check("docs arm-a plugin == frozen", sha256File(path.join(DOCS, "harness", "configs", "arm-a", "plugin.ts")) === env.plugins.A, env.plugins.A);
check("docs arm-b plugin == frozen", sha256File(path.join(DOCS, "harness", "configs", "arm-b", "plugin.ts")) === env.plugins.B, env.plugins.B);
check("docs wrapper cmd == frozen", sha256File(path.join(DOCS, "harness", "bin", "riltra-ro.cmd")) === env.wrapper.cmd, env.wrapper.cmd);
check("docs wrapper cjs == frozen", sha256File(path.join(DOCS, "harness", "bin", "riltra-ro.cjs")) === env.wrapper.cjs, env.wrapper.cjs);

// ---------- 4. home pristine + no empty store ----------
for (const [rel, want] of Object.entries(env.home_pristine)) {
  const got = sha256File(path.join(CP, "home-pristine", rel));
  check(`home-pristine ${rel}`, got === want, got);
}
const emptyStoreChecks = [];
for (const home of ["home-pristine", "home-template"]) {
  for (const rel of [path.join("index", "empty.db"), path.join("knowledge", "empty.db")]) {
    const p = path.join(CP, home, rel);
    emptyStoreChecks.push(`${home}/${rel}:${fs.existsSync(p) ? "EXISTS" : "absent"}`);
  }
}
check("no empty store in pristine/template homes", emptyStoreChecks.every((s) => s.endsWith("absent")), emptyStoreChecks.join(", "));

// ---------- 5. raw runs + aggregate recomputation ----------
const tasksDir = path.join(CP, "runs", "tasks");
const taskNames = fs.readdirSync(tasksDir).sort();
check("16 task directories", taskNames.length === 16, taskNames.length);
const runs = [];
let missing = [];
for (const t of taskNames) {
  for (const arm of ["A", "B"]) {
    const d = path.join(tasksDir, t, arm);
    const mp = path.join(d, "meta.json");
    const sp = path.join(d, "score.json");
    const ap = path.join(d, "messages.json");
    if (![mp, sp, ap].every((p) => fs.existsSync(p))) { missing.push(t + "/" + arm); continue; }
    const meta = readJson(mp);
    const score = readJson(sp);
    const api = readJson(ap);
    let input = 0, cacheRead = 0, cacheWrite = 0, output = 0, reasoning = 0;
    for (const m of api.data || []) {
      if (m.type !== "assistant" || !m.tokens) continue;
      input += m.tokens.input || 0;
      cacheRead += (m.tokens.cache && m.tokens.cache.read) || 0;
      cacheWrite += (m.tokens.cache && m.tokens.cache.write) || 0;
      output += m.tokens.output || 0;
      reasoning += m.tokens.reasoning || 0;
    }
    runs.push({ task: t, arm, meta, score, gross: input + cacheRead + cacheWrite + output, grossUpper: input + cacheRead + cacheWrite + output + reasoning, input, cacheRead, cacheWrite, output, reasoning, messages: (api.data || []).length });
  }
}
check("32 run dirs with meta+score+messages", missing.length === 0 && runs.length === 32, missing.length ? "missing: " + missing.join(",") : runs.length);
check("zero retries (one run per task/arm)", runs.length === 32, "");
for (const arm of ["A", "B"]) {
  const rr = runs.filter((r) => r.arm === arm);
  const attempted = rr.length;
  const statusOk = rr.filter((r) => r.meta.status === "ok" && r.meta.outcome === "succeeded" && !r.meta.timed_out).length;
  const completed = rr.filter((r) => r.meta.status === "ok" && r.meta.outcome === "succeeded" && !r.meta.timed_out && r.meta.exposure_ok && r.meta.home_unchanged && r.score.correct && r.score.evidence_valid).length;
  const correct = rr.filter((r) => r.score.correct).length;
  const evidence = rr.filter((r) => r.score.evidence_valid).length;
  const gross = rr.reduce((s, r) => s + r.gross, 0);
  const grossUpper = rr.reduce((s, r) => s + r.grossUpper, 0);
  const agg = summary.aggregate[arm];
  check(`arm ${arm}: attempted/completed/correct/evidence`, attempted === agg.attempted && completed === agg.completed && correct === agg.correct && evidence === agg.evidence, `${attempted}/${completed}/${correct}/${evidence} vs ${agg.attempted}/${agg.completed}/${agg.correct}/${agg.evidence}`);
  check(`arm ${arm}: gross tokens recomputed`, gross === agg.gross_total, `${gross} vs ${agg.gross_total}`);
  check(`arm ${arm}: gross+reasoning recomputed`, grossUpper === agg.gross_upper_total, `${grossUpper} vs ${agg.gross_upper_total}`);
  const perCompleted = gross / agg.completed;
  check(`arm ${arm}: gross per completed`, Math.abs(perCompleted - agg.gross_per_completed) < 1e-6, perCompleted.toFixed(4) + " vs " + agg.gross_per_completed);
  check(`arm ${arm}: session status ok for all attempts`, statusOk === attempted, `${statusOk}/${attempted}`);
  check(`arm ${arm}: zero timeouts/errors/exposure/home failures`, rr.every((r) => !r.meta.timed_out && !r.meta.error && r.meta.exposure_ok && r.meta.home_unchanged), "");
  const blocked = rr.reduce((s, r) => s + (r.meta.plugin_blocked || 0), 0);
  check(`arm ${arm}: measured plugin blocked == ${agg.blocked}`, blocked === agg.blocked, `${blocked} vs ${agg.blocked}`);
}

// ---------- 6. isolation exposure + plugin events joined to measured sessions ----------
const MCP = ["riltra_riltra_repo_list","riltra_riltra_repo_status","riltra_riltra_code_search","riltra_riltra_code_callers","riltra_riltra_code_callees","riltra_riltra_code_trace","riltra_riltra_code_impact","riltra_riltra_knowledge_search","riltra_riltra_knowledge_show","riltra_riltra_knowledge_related","riltra_riltra_health"];
const measured = new Set(runs.map((r) => r.meta.session));
const lines = fs.readFileSync(path.join(CP, "plugin-events.jsonl"), "utf8").trim().split(/\r?\n/).map(JSON.parse);
const measuredEvents = lines.filter((e) => measured.has(e.sessionID));
const mBlocked = measuredEvents.filter((e) => e.kind === "blocked").length;
const mAllowed = measuredEvents.filter((e) => e.kind === "allowed").length;
const expectedAllowed = summary.aggregate.A.plugin_allowed + summary.aggregate.B.plugin_allowed;
check("plugin log: measured allowed == aggregate allowed total", mAllowed === expectedAllowed, `${mAllowed} vs ${expectedAllowed}`);
check("plugin log: measured blocked == 2 (A) total", mBlocked === 2, mBlocked);
const ctx = measuredEvents.filter((e) => e.kind === "context");
const ctxSessions = new Set(ctx.map((e) => e.sessionID));
check("plugin log: every measured session has a context event", ctxSessions.size === 32, ctxSessions.size);
const ctxSets = new Set(ctx.map((e) => e.kept.slice().sort().join("+")));
check("plugin log: context kept sets are exactly the two frozen sets", ctxSets.size === 2 && ctxSets.has("read+shell") && ctxSets.has(["read", ...MCP].sort().join("+")), [...ctxSets].join(" | "));
let exposureBad = [];
for (const r of runs) {
  const ev = ctx.find((e) => e.sessionID === r.meta.session);
  if (!ev) { exposureBad.push(r.task + "/" + r.arm + ":no-context"); continue; }
  const want = r.arm === "A" ? ["read", "shell"] : ["read", ...MCP];
  const got = ev.kept.slice().sort();
  if (JSON.stringify(got) !== JSON.stringify(want.slice().sort())) exposureBad.push(`${r.task}/${r.arm}:${got.join("+")}`);
}
check("every measured session exposed exactly the frozen tool set", exposureBad.length === 0, exposureBad.join("; "));

// ---------- 7. G6 smoke evidence ----------
const g6 = readJson(path.join(CP, "g6-smoke", "checks.json"));
check("G6 structured not_registered error", g6.invalid_structured_error === true && g6.invalid_error_kind === "not_registered", JSON.stringify(g6.invalid_error));
check("G6 server alive after error", g6.server_alive_after_error === true, "");
check("G6 exposure + home + status", g6.exposure_ok === true && g6.home_unchanged === true && g6.status === "ok", "");

// ---------- 8. G5 reference parity ----------
const cmp = readJson(path.join(CP, "reference", "compare.json"));
check("G5 all MCP vs CLI comparisons equal", cmp.summary.all_mcp_vs_cli === true && cmp.summary.mcp_vs_cli_equal === cmp.summary.compared, `${cmp.summary.mcp_vs_cli_equal}/${cmp.summary.compared}`);
check("G5 all CLI outputs byte-equal to goldens", cmp.summary.all_cli_vs_golden === true && cmp.summary.cli_vs_golden_equal === cmp.summary.cli_vs_golden_compared, `${cmp.summary.cli_vs_golden_equal}/${cmp.summary.cli_vs_golden_compared}`);
const mcpRes = readJson(path.join(CP, "reference", "mcp-results.json"));
check("direct MCP: 11 tools exact", mcpRes.tools_count === 11 && mcpRes.tools_exact_expected === true, "");
check("direct MCP: 17/17 calls ok, alive after invalid, home unchanged", mcpRes.calls.every((c) => !c.error) && mcpRes.server_alive_after_error === true && mcpRes.home_changed === false, "");
check("goldens: 20 frozen files", fs.readdirSync(path.join(CP, "goldens-raw")).length === 20, fs.readdirSync(path.join(CP, "goldens-raw")).length);

// ---------- 9. committed docs copy sanity ----------
const docsFiles = walk(DOCS, []);
check("docs evidence: summary.json + summary.md + measurements.json present", ["summary.json", "summary.md", "measurements.json"].every((f) => fs.existsSync(path.join(DOCS, f))), "");
check("docs evidence: raw runs copied (272 files)", walk(path.join(DOCS, "raw", "runs"), []).length === 272, walk(path.join(DOCS, "raw", "runs"), []).length);
check("docs evidence: gate logs present", ["gate.json", "gate-fmt.log", "gate-check.log", "gate-clippy.log", "gate-test.log"].every((f) => fs.existsSync(path.join(DOCS, "gate", f))), "");

// ---------- 10. credentials scan over the committed evidence ----------
const credRe = /sk-[A-Za-z0-9]{16,}|ghp_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,}|Bearer\s+[A-Za-z0-9_\-\.]{20,}|(api[_-]?key|apikey|secret|password)["']?\s*[:=]\s*["'][^"'\s]{6,}|ANTHROPIC_API_KEY|OPENAI_API_KEY/i;
const scannerSelf = path.resolve(path.join(DOCS, "harness", "verify.cjs"));
const credHits = [];
for (const f of docsFiles) {
  if (path.resolve(f) === scannerSelf) continue; // scanner source necessarily contains the pattern names
  let t;
  try { t = fs.readFileSync(f, "utf8"); } catch { continue; }
  if (credRe.test(t)) credHits.push(path.relative(DOCS, f));
}
check("no credential patterns in committed evidence", credHits.length === 0, credHits.join(", "));

const failed = checks.filter((c) => !c.ok);
const out = { schema: "cp7a-verification/1", generated_at: new Date().toISOString(), checks_total: checks.length, checks_failed: failed.length, credential_scan_skipped: ["harness/verify.cjs (scanner source contains the pattern names)"], checks };
fs.writeFileSync(path.join(CP, "verify.json"), JSON.stringify(out, null, 2) + "\n");
console.log(`\n${checks.length - failed.length}/${checks.length} checks passed`);
process.exit(failed.length ? 1 : 0);
