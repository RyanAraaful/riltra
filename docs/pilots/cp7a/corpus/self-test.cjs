// SPDX-License-Identifier: Apache-2.0
// Corpus self-test: every required regex must match a synthetic correct answer,
// and every evidence marker must match the frozen raw CLI document.
"use strict";
const fs = require("node:fs");
const path = require("node:path");
const corpus = JSON.parse(fs.readFileSync(path.join(__dirname, "corpus.json"), "utf8"));
const { scoreTask } = require("./score.cjs");

const gold = path.join(__dirname, "..", "goldens-raw");
const raw = (name) => fs.readFileSync(path.join(gold, name), "utf8");
const status = raw("status.json");
const repoList = raw("repo-list.json");
const repoStatus = raw("repo-status.json");
const indexStatus = raw("index-status.json");
const searchId = raw("search-validate-id.json");
const callersId = raw("callers-validate-id.json");
const calleesNew = raw("callees-validate-new.json");
const trace = raw("trace-validate-new-to-id.json");
const impact = raw("impact-validate-id.json");
const callersOther = raw("callers-other-add.json");
const kSearch = raw("knowledge-search-readonly.json");
const kRev1 = raw("knowledge-show-health-r1.json");
const kRev2 = raw("knowledge-show-health.json");
const kRelated = raw("knowledge-related-health.json");
const kReadonly = raw("knowledge-show-readonly.json");
const kStale = raw("knowledge-show-stale.json");
const searchEmpty = raw("search-empty-main.json");
const kEmpty = raw("knowledge-search-empty.json");
const doctor = raw("doctor.json");
const healthMcp = '{"operation":"health","versions":{"riltra":"0.1.0"},"repo_count":3,"doctor":{"pass":true},"capabilities":{"read_only":true}}';

const answers = {
  "repo-list": "Registered repositories: empty, other, riltra (3 total).",
  "repo-status": "HEAD 0fd22a1134274c1e92f741fc0d4e3b4886ceeaf3; worktree not dirty; index freshness fresh.",
  "search-validate-id": "One match: crates/cli/src/knowledge.rs, start line 89, kind function.",
  "callers-validate-id": "4 caller edges, all POTENTIAL_CALL, verification Unverified.",
  "callees-validate-new": "9 callee edges; distinct names: validate_id, validate_kind, validate_new_status, is_empty, Err, Ok, to_string.",
  "trace-validate-new-to-id": "0 paths; it does not claim an execution path; verification Unverified.",
  "impact-validate-id": "Verdict UNKNOWN; it does not prove execution; verification Unverified.",
  "potential-vs-calls": "One caller edge; kind POTENTIAL_CALL; none are emitted as CALLS.",
  "knowledge-search-readonly": "One hit; ID cp7a-constraint-readonly; title MCP tools are read-only.",
  "knowledge-revision": "Revision 1 title: Health composes bootstrap and doctor. Current revision 2 title: Health composes bootstrap, doctor, and repository count.",
  "knowledge-related": "2 relations: cp7a-rel-health-readonly (supports cp7a-constraint-readonly); cp7a-rel-health-trace (relates_to cp7a-constraint-trace).",
  "knowledge-evidence-readonly": "Title: MCP tools are read-only; verification Unverified; evidence freshness fresh.",
  "stale-evidence": "Record cp7a-other-stale: freshness stale; verification Unverified; it means the source file changed after the evidence was captured.",
  "missing-index-empty": "0 results; reason no-index; Riltra did not create an index.",
  "missing-knowledge-store-empty": "0 hits; generation 0; no error.",
  "health": "Version 0.1.0; 3 registered repositories; the environment doctor passes."
};

const cli = (args) => `E:\\dev\\Riltra\\.cp7a\\bin\\riltra-ro.cmd ${args}`;
const commands = {
  "repo-list": ["repo list"],
  "repo-status": ["repo status --repo riltra", "index status --repo riltra"],
  "search-validate-id": ["search --repo riltra --symbol validate_id"],
  "callers-validate-id": ["callers --repo riltra --path crates/cli/src/knowledge.rs --line 89 --col 1"],
  "callees-validate-new": ["callees --repo riltra --path crates/cli/src/knowledge.rs --line 199 --col 1"],
  "trace-validate-new-to-id": ["trace --repo riltra --path crates/cli/src/knowledge.rs --line 199 --col 1 --to crates/cli/src/knowledge.rs:89:1"],
  "impact-validate-id": ["impact --repo riltra --path crates/cli/src/knowledge.rs --line 89 --col 1"],
  "potential-vs-calls": ["callers --repo other --path c/main.c --line 6 --col 1"],
  "knowledge-search-readonly": ["knowledge search --repo riltra --query read-only"],
  "knowledge-revision": ["knowledge show --repo riltra --id cp7a-decision-health --revision 1", "knowledge show --repo riltra --id cp7a-decision-health"],
  "knowledge-related": ["knowledge related --repo riltra --id cp7a-decision-health --direction outbound"],
  "knowledge-evidence-readonly": ["knowledge show --repo riltra --id cp7a-constraint-readonly"],
  "stale-evidence": ["knowledge show --repo other --id cp7a-other-stale"],
  "missing-index-empty": ["search --repo empty --symbol main"],
  "missing-knowledge-store-empty": ["knowledge search --repo empty --query anything"],
  "health": ["status", "repo list", "doctor"]
};

const outputs = {
  "repo-list": { A: [repoList], B: [repoList] },
  "repo-status": { A: [repoStatus, indexStatus], B: [repoStatus + "\n" + indexStatus] },
  "search-validate-id": { A: [searchId], B: [searchId] },
  "callers-validate-id": { A: [callersId], B: [callersId] },
  "callees-validate-new": { A: [calleesNew], B: [calleesNew] },
  "trace-validate-new-to-id": { A: [trace], B: [trace] },
  "impact-validate-id": { A: [impact], B: [impact] },
  "potential-vs-calls": { A: [callersOther], B: [callersOther] },
  "knowledge-search-readonly": { A: [kSearch], B: [kSearch] },
  "knowledge-revision": { A: [kRev1, kRev2], B: [kRev1, kRev2] },
  "knowledge-related": { A: [kRelated], B: [kRelated] },
  "knowledge-evidence-readonly": { A: [kReadonly], B: [kReadonly] },
  "stale-evidence": { A: [kStale], B: [kStale] },
  "missing-index-empty": { A: [searchEmpty], B: [searchEmpty] },
  "missing-knowledge-store-empty": { A: [kEmpty], B: [kEmpty] },
  "health": { A: [status, repoList, doctor], B: [healthMcp] }
};

let failures = 0;
for (const task of corpus.tasks) {
  const text = answers[task.id];
  if (!text) { console.log(`FAIL ${task.id}: no synthetic answer`); failures++; continue; }
  for (const arm of ["A", "B"]) {
    const evs = task.evidence[arm];
    const outs = outputs[task.id][arm];
    if (evs.length !== outs.length) { console.log(`FAIL ${task.id} ${arm}: evidence/output length`); failures++; continue; }
    const content = [{ type: "text", text }];
    evs.forEach((ev, i) => {
      if (arm === "A") {
        const cmd = commands[task.id][i];
        if (!new RegExp(ev.command, "i").test(cli(cmd))) { console.log(`FAIL ${task.id} A ev${i}: command regex ${ev.command} !~ ${cmd}`); failures++; }
      }
      for (const p of ev.output || []) {
        if (!new RegExp(p, "i").test(outs[i])) { console.log(`FAIL ${task.id} ${arm} ev${i}: output regex ${p}`); failures++; }
      }
      const name = arm === "A" ? "shell" : ev.tool.replace(/^\^/, "").replace(/\$$/, "");
      content.push({
        type: "tool",
        name,
        state: {
          input: arm === "A" ? { command: cli(commands[task.id][i]) } : {},
          content: [{ type: "text", text: outs[i] }],
          status: "completed",
          time: {}
        }
      });
    });
    const score = scoreTask(task, arm, { data: [{ type: "assistant", content }] });
    if (!score.correct) { console.log(`FAIL ${task.id} ${arm}: required ${JSON.stringify(score.required_failures)}`); failures++; }
    if (!score.evidence_valid) { console.log(`FAIL ${task.id} ${arm}: evidence ${JSON.stringify(score.evidence_failures)}`); failures++; }
  }
}
console.log(JSON.stringify({ tasks: corpus.tasks.length, failures }));
process.exit(failures === 0 ? 0 : 1);
