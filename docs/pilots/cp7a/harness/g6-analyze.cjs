// SPDX-License-Identifier: Apache-2.0
// CP7-A G6 post-hoc analysis correction. The initial smoke capture inspected
// only tool state.content and missed the structured state.error field. This
// reads the untouched raw session and rewrites only the derived checks object.
"use strict";
const fs = require("node:fs");
const path = require("node:path");
const REPO_ROOT = path.resolve(__dirname, "..", "..", "..", "..");
const OUT = path.join(REPO_ROOT, ".cp7a", "g6-smoke");
const api = JSON.parse(fs.readFileSync(path.join(OUT, "messages.json"), "utf8"));
const meta = JSON.parse(fs.readFileSync(path.join(OUT, "meta.json"), "utf8"));
const tools = [];
let answer = "";
for (const m of api.data || []) {
  if (m.type !== "assistant") continue;
  for (const c of m.content || []) {
    if (c.type === "tool") tools.push({ name: c.name, state: c.state });
    if (c.type === "text" && c.text) answer += (answer ? "\n" : "") + c.text;
  }
}
const invalid = tools.find((t) => t.name === "riltra_riltra_code_callers");
const health = tools.find((t) => t.name === "riltra_riltra_health");
const healthJson = health && health.state.content && health.state.content[0] ? JSON.parse(health.state.content[0].text) : null;
const idle = (api.data || []).filter((m) => m.type === "idle").pop() || null;
const sessionFile = path.join(OUT, "session.json");
const session = fs.existsSync(sessionFile) ? JSON.parse(fs.readFileSync(sessionFile, "utf8")) : null;
const outcome = (idle && idle.outcome) || (session && session.data && session.data.outcome) || null;
const MAIN = ["registry.json", "index\\riltra.db", "index\\other.db", "knowledge\\riltra.db", "knowledge\\other.db"];
const mainUnchanged = MAIN.every((rel) => meta.home_before && meta.home_after && meta.home_before.main[rel] === meta.home_after.main[rel]);
const homeUnchanged = mainUnchanged && !(meta.home_after && (meta.home_after.empty_index || meta.home_after.empty_knowledge));
const errorMessage = invalid && invalid.state && invalid.state.error ? invalid.state.error.message : null;
let errorJson = null;
if (errorMessage) { try { errorJson = JSON.parse(errorMessage); } catch {} }
const checks = {
  analysis_note: "Post-hoc extraction of tool state.error from the untouched raw session. The initial smoke capture inspected only state.content, so its invalid_content/mentions_error fields were incomplete; no measurement field and no raw message was changed.",
  invalid_call_found: Boolean(invalid),
  invalid_status: invalid ? invalid.state.status : null,
  invalid_error: invalid && invalid.state ? invalid.state.error || null : null,
  invalid_structured_error: Boolean(errorJson && errorJson.kind && errorJson.message),
  invalid_error_kind: errorJson ? errorJson.kind : null,
  invalid_error_message: errorJson ? errorJson.message : null,
  invalid_mentions_error: Boolean(errorJson),
  health_call_found: Boolean(health),
  health_status: health ? health.state.status : null,
  health_version: healthJson && healthJson.status ? healthJson.status.version : null,
  health_doctor_pass: healthJson && healthJson.doctor ? healthJson.doctor.pass : null,
  health_responded: Boolean(healthJson),
  server_alive_after_error: Boolean(health && health.state.status !== "error" && healthJson),
  tool_names: tools.map((t) => t.name),
  exposure_ok: Boolean(meta.exposure_ok),
  context_kept: meta.context_kept || [],
  context_removed: meta.context_removed || [],
  blocked: meta.plugin_blocked || 0,
  home_unchanged: homeUnchanged,
  empty_index: meta.home_after ? Boolean(meta.home_after.empty_index) : null,
  empty_knowledge: meta.home_after ? Boolean(meta.home_after.empty_knowledge) : null,
  status: meta.timed_out ? "timeout" : (outcome === "succeeded" ? "ok" : "error"),
  outcome,
  model_answer_excerpt: answer.slice(0, 1200)
};
meta.checks = checks;
meta.checks_analysis_note = checks.analysis_note;
fs.writeFileSync(path.join(OUT, "checks.json"), JSON.stringify(checks, null, 2) + "\n");
fs.writeFileSync(path.join(OUT, "meta.json"), JSON.stringify(meta, null, 2) + "\n");
console.log(JSON.stringify({
  invalid_status: checks.invalid_status,
  invalid_error_kind: checks.invalid_error_kind,
  invalid_error_message: checks.invalid_error_message,
  invalid_structured_error: checks.invalid_structured_error,
  health_status: checks.health_status,
  health_version: checks.health_version,
  server_alive_after_error: checks.server_alive_after_error,
  exposure_ok: checks.exposure_ok,
  home_unchanged: checks.home_unchanged,
  status: checks.status
}, null, 2));
