// SPDX-License-Identifier: Apache-2.0
// CP7-A frozen corpus generator. Writes corpus.json; the JSON is the artifact.
"use strict";
const fs = require("node:fs");
const path = require("node:path");

const R = String.raw;
const shell = (command, output) => ({ tool: R`^shell$`, command, output });
const mcp = (name, output) => ({ tool: R`^riltra_riltra_${name}$`, output });

const CP7A_ROOT = process.env.RILTRA_CP7A_ROOT || "<CP7A_ROOT>";

const access = {
  A: R`Access: use only the shell tool, running the read-only wrapper ${CP7A_ROOT}\bin\riltra-ro.cmd. Run that wrapper with --help once if you need CLI syntax. Do not use read or any other tool. Answer the question with the requested values and the evidence you observed.`,
  B: R`Access: use only the direct Riltra MCP tools named riltra_riltra_*. Their input schemas describe the arguments. Do not use read, shell, or any other tool. Answer the question with the requested values and the evidence you observed.`
};

const tasks = [
  {
    id: "repo-list",
    order: "A",
    question: "In the frozen Riltra registry, list every registered repository name and state how many are registered.",
    golden: { repos: ["empty", "other", "riltra"], count: 3 },
    required: [R`\bempty\b`, R`\bother\b`, R`\briltra\b`, R`\b(3|three)\b`],
    evidence: {
      A: [shell(R`repo\s+list`, [R`"riltra"`, R`"other"`, R`"empty"`])],
      B: [mcp("repo_list", [R`"riltra"`, R`"other"`, R`"empty"`])]
    }
  },
  {
    id: "repo-status",
    order: "B",
    question: "For repository `riltra`, report the git HEAD, whether the worktree is dirty, and the code-index freshness.",
    golden: { head: "0fd22a1134274c1e92f741fc0d4e3b4886ceeaf3", dirty: false, freshness: "fresh" },
    required: [R`0fd22a1`, R`(?:\bdirty\b[^.\n]{0,30}\b(?:false|no)\b|\bnot dirty\b|\bclean\b)`, R`\bfresh\b`],
    evidence: {
      A: [
        shell(R`repo\s+status[\s\S]*riltra`, [R`0fd22a1`, R`dirty[^,]{0,20}false`]),
        shell(R`index\s+status[\s\S]*riltra`, [R`freshness[^,]{0,20}fresh`])
      ],
      B: [mcp("repo_status", [R`0fd22a1`, R`dirty[^,]{0,20}false`, R`freshness[^,]{0,20}fresh`])]
    }
  },
  {
    id: "search-validate-id",
    order: "A",
    question: "In repository `riltra`, how many indexed symbols match `validate_id`, and what are the path, start line, and kind of the exact function named `validate_id`?",
    golden: { total: 1, path: "crates/cli/src/knowledge.rs", start_line: 89, kind: "function" },
    required: [R`\b(1|one)\b`, R`knowledge\.rs`, R`\b89\b`, R`\bfunction\b`],
    evidence: {
      A: [shell(R`search[\s\S]*riltra[\s\S]*validate_id`, [R`name[^,]{0,20}validate_id`, R`start_line[^0-9]{0,10}89`, R`kind[^,]{0,20}function`, R`total[^0-9]{0,10}1`])],
      B: [mcp("code_search", [R`name[^,]{0,20}validate_id`, R`start_line[^0-9]{0,10}89`, R`kind[^,]{0,20}function`, R`total[^0-9]{0,10}1`])]
    }
  },
  {
    id: "callers-validate-id",
    order: "B",
    question: "For the function `validate_id` anchored at `crates/cli/src/knowledge.rs:89:1` in `riltra`, how many caller edges are returned, what edge kind do they have, and what is their verification status?",
    golden: { count: 4, kind: "POTENTIAL_CALL", verification: "Unverified" },
    required: [R`\b(4|four)\b`, R`POTENTIAL_CALL`, R`Unverified`],
    evidence: {
      A: [shell(R`callers[\s\S]*riltra`, [R`kind[^,]{0,20}POTENTIAL_CALL`, R`Unverified`])],
      B: [mcp("code_callers", [R`kind[^,]{0,20}POTENTIAL_CALL`, R`Unverified`])]
    }
  },
  {
    id: "callees-validate-new",
    order: "A",
    question: "For the function `validate_new` anchored at `crates/cli/src/knowledge.rs:199:1` in `riltra`, how many callee edges are returned, and what are the distinct callee names?",
    golden: { count: 9, distinct: ["Err", "Ok", "is_empty", "to_string", "validate_id", "validate_kind", "validate_new_status"] },
    required: [R`\b(9|nine)\b`, R`validate_id`, R`validate_kind`, R`validate_new_status`, R`is_empty`, R`\bErr\b`, R`to_string`, R`\bOk\b`],
    evidence: {
      A: [shell(R`callees[\s\S]*riltra`, [
        R`callee_name[^,]{0,20}validate_id`, R`callee_name[^,]{0,20}validate_kind`,
        R`callee_name[^,]{0,20}validate_new_status`, R`callee_name[^,]{0,20}is_empty`,
        R`callee_name[^,]{0,20}Err`, R`callee_name[^,]{0,20}to_string`, R`callee_name[^,]{0,20}Ok`
      ])],
      B: [mcp("code_callees", [
        R`callee_name[^,]{0,20}validate_id`, R`callee_name[^,]{0,20}validate_kind`,
        R`callee_name[^,]{0,20}validate_new_status`, R`callee_name[^,]{0,20}is_empty`,
        R`callee_name[^,]{0,20}Err`, R`callee_name[^,]{0,20}to_string`, R`callee_name[^,]{0,20}Ok`
      ])]
    }
  },
  {
    id: "trace-validate-new-to-id",
    order: "B",
    question: "Trace from `validate_new` at `crates/cli/src/knowledge.rs:199:1` to `validate_id` at `crates/cli/src/knowledge.rs:89:1` in `riltra`. How many paths are returned, does Riltra claim this is an execution path, and what is the verification status?",
    golden: { paths: 0, execution: false, verification: "Unverified", disclaimer: "plausible-path-not-execution" },
    required: [R`(?:\b0\b|\bzero\b|\bno paths?\b)`, R`Unverified`, R`(?:not|no|never|does not|doesn't)[^.\n]{0,60}execution`],
    evidence: {
      A: [shell(R`trace[\s\S]*riltra`, [R`paths[^,]{0,10}\[\]`, R`plausible-path-not-execution`, R`Unverified`])],
      B: [mcp("code_trace", [R`paths[^,]{0,10}\[\]`, R`plausible-path-not-execution`, R`Unverified`])]
    }
  },
  {
    id: "impact-validate-id",
    order: "A",
    question: "For `validate_id` at `crates/cli/src/knowledge.rs:89:1` in `riltra`, what is the impact verdict, does it prove execution, and what is the verification status?",
    golden: { verdict: "UNKNOWN", execution: false, verification: "Unverified", disclaimer: "plausible-impact-not-execution" },
    required: [R`UNKNOWN`, R`Unverified`, R`(?:not|no|never|does not|doesn't)[^.\n]{0,60}(?:execution|proof|prove)`],
    evidence: {
      A: [shell(R`impact[\s\S]*riltra`, [R`verdict[^,]{0,20}UNKNOWN`, R`plausible-impact-not-execution`, R`Unverified`])],
      B: [mcp("code_impact", [R`verdict[^,]{0,20}UNKNOWN`, R`plausible-impact-not-execution`, R`Unverified`])]
    }
  },
  {
    id: "potential-vs-calls",
    order: "B",
    question: "At `c/main.c:6:1` in repository `other`, how many caller edges are returned, what kind are they, and are any emitted as `CALLS`?",
    golden: { count: 1, kind: "POTENTIAL_CALL", calls: 0 },
    required: [R`(?:\b1\b|\bone\b)[^.\n]{0,30}\bedge`, R`POTENTIAL_CALL`, R`(?:no|none|not|never)[^.\n]{0,40}CALLS`],
    evidence: {
      A: [shell(R`callers[\s\S]*other`, [R`callee_name[^,]{0,20}add`, R`kind[^,]{0,20}POTENTIAL_CALL`, R`c/main\.c`])],
      B: [mcp("code_callers", [R`callee_name[^,]{0,20}add`, R`kind[^,]{0,20}POTENTIAL_CALL`, R`c/main\.c`])]
    }
  },
  {
    id: "knowledge-search-readonly",
    order: "A",
    question: "Search the knowledge store of `riltra` for the literal text `read-only`. How many hits are returned, and what are the record ID and title?",
    golden: { total: 1, id: "cp7a-constraint-readonly", title: "MCP tools are read-only" },
    required: [R`\b(1|one)\b`, R`cp7a-constraint-readonly`, R`MCP tools are read-only`],
    evidence: {
      A: [shell(R`knowledge\s+search[\s\S]*riltra`, [R`total[^0-9]{0,10}1`, R`cp7a-constraint-readonly`, R`MCP tools are read-only`])],
      B: [mcp("knowledge_search", [R`total[^0-9]{0,10}1`, R`cp7a-constraint-readonly`, R`MCP tools are read-only`])]
    }
  },
  {
    id: "knowledge-revision",
    order: "B",
    question: "Show record `cp7a-decision-health` in `riltra` at revision 1 and at its current revision. What is the title at each revision, and what is the current revision number?",
    golden: {
      revision_1_title: "Health composes bootstrap and doctor",
      current_revision: 2,
      current_title: "Health composes bootstrap, doctor, and repository count"
    },
    required: [R`Health composes bootstrap and doctor`, R`Health composes bootstrap,? doctor,? and repository count`, R`\b(2|two)\b`],
    evidence: {
      A: [
        shell(R`knowledge\s+show[\s\S]*riltra`, [R`Health composes bootstrap and doctor`, R`revision[^0-9]{0,10}1`]),
        shell(R`knowledge\s+show[\s\S]*riltra`, [R`Health composes bootstrap,? doctor,? and repository count`, R`revision[^0-9]{0,10}2`])
      ],
      B: [
        mcp("knowledge_show", [R`Health composes bootstrap and doctor`, R`revision[^0-9]{0,10}1`]),
        mcp("knowledge_show", [R`Health composes bootstrap,? doctor,? and repository count`, R`revision[^0-9]{0,10}2`])
      ]
    }
  },
  {
    id: "knowledge-related",
    order: "A",
    question: "For record `cp7a-decision-health` in `riltra`, list its outbound relations: how many, and for each give the relation ID, kind, and destination record ID.",
    golden: {
      count: 2,
      relations: [
        { relation_id: "cp7a-rel-health-readonly", kind: "supports", dst_id: "cp7a-constraint-readonly" },
        { relation_id: "cp7a-rel-health-trace", kind: "relates_to", dst_id: "cp7a-constraint-trace" }
      ]
    },
    required: [R`\b(2|two)\b`, R`cp7a-rel-health-readonly`, R`supports`, R`cp7a-constraint-readonly`, R`cp7a-rel-health-trace`, R`relates_to`, R`cp7a-constraint-trace`],
    evidence: {
      A: [shell(R`knowledge\s+related[\s\S]*riltra`, [
        R`relation_id[\s\S]{0,40}cp7a-rel-health-readonly`, R`supports`, R`cp7a-constraint-readonly`,
        R`relation_id[\s\S]{0,40}cp7a-rel-health-trace`, R`relates_to`, R`cp7a-constraint-trace`
      ])],
      B: [mcp("knowledge_related", [
        R`relation_id[\s\S]{0,40}cp7a-rel-health-readonly`, R`supports`, R`cp7a-constraint-readonly`,
        R`relation_id[\s\S]{0,40}cp7a-rel-health-trace`, R`relates_to`, R`cp7a-constraint-trace`
      ])]
    }
  },
  {
    id: "knowledge-evidence-readonly",
    order: "B",
    question: "Show record `cp7a-constraint-readonly` in `riltra`. What is its title, verification state, and evidence freshness status?",
    golden: { id: "cp7a-constraint-readonly", title: "MCP tools are read-only", verification: "Unverified", freshness: "fresh" },
    required: [R`MCP tools are read-only`, R`Unverified`, R`\bfresh\b`],
    evidence: {
      A: [shell(R`knowledge\s+show[\s\S]*riltra`, [R`cp7a-constraint-readonly`, R`verification[^,]{0,30}Unverified`, R`status[^,]{0,20}fresh`, R`MCP tools are read-only`])],
      B: [mcp("knowledge_show", [R`cp7a-constraint-readonly`, R`verification[^,]{0,30}Unverified`, R`status[^,]{0,20}fresh`, R`MCP tools are read-only`])]
    }
  },
  {
    id: "stale-evidence",
    order: "A",
    question: "Show record `cp7a-other-stale` in `other`. What is its freshness status, verification state, and what does that freshness status mean?",
    golden: { id: "cp7a-other-stale", title: "Stale evidence fixture", verification: "Unverified", freshness: "stale", meaning: "the source file changed after the evidence was captured" },
    required: [R`\bstale\b`, R`Unverified`, R`cp7a-other-stale`, R`(?:changed|modified|differs|mismatch)`],
    evidence: {
      A: [shell(R`knowledge\s+show[\s\S]*other`, [R`cp7a-other-stale`, R`status[^,]{0,20}stale`, R`Unverified`, R`observed_digest`])],
      B: [mcp("knowledge_show", [R`cp7a-other-stale`, R`status[^,]{0,20}stale`, R`Unverified`, R`observed_digest`])]
    }
  },
  {
    id: "missing-index-empty",
    order: "B",
    question: "Search symbols for `main` in repository `empty`. How many results are returned, what reason is reported, and did Riltra create an index?",
    golden: { total: 0, reason: "no-index", freshness: "unknown", index_created: false },
    required: [R`(?:\b0\b|\bzero\b|\bno results?\b)`, R`no-index`, R`(?:no|not|none|nothing)[^.\n]{0,60}(?:creat|writ|stor|index)`],
    evidence: {
      A: [shell(R`search[\s\S]*empty`, [R`total[^0-9]{0,10}0`, R`no-index`, R`freshness[^,]{0,20}unknown`])],
      B: [mcp("code_search", [R`total[^0-9]{0,10}0`, R`no-index`, R`freshness[^,]{0,20}unknown`])]
    }
  },
  {
    id: "missing-knowledge-store-empty",
    order: "A",
    question: "Search knowledge for `anything` in repository `empty`. How many hits are returned, what generation is reported, and is it an error?",
    golden: { total: 0, generation: 0, error: false },
    required: [R`(?:0 (?:results?|hits?|total)|(?:total|results?|hits?)[^0-9]{0,15}0)`, R`generation[^0-9]{0,10}0`, R`(?:no error|not an error|succe|not an? error)`],
    evidence: {
      A: [shell(R`knowledge\s+search[\s\S]*empty`, [R`generation[^0-9]{0,10}0`, R`total[^0-9]{0,10}0`, R`hits[^,]{0,10}\[\]`])],
      B: [mcp("knowledge_search", [R`generation[^0-9]{0,10}0`, R`total[^0-9]{0,10}0`, R`hits[^,]{0,10}\[\]`])]
    }
  },
  {
    id: "health",
    order: "B",
    question: "Report Riltra's version, the number of registered repositories, and whether the environment doctor passes.",
    golden: { version: "0.1.0", repo_count: 3, doctor_pass: true },
    required: [R`0\.1\.0`, R`\b(3|three)\b`, R`(?:doctor|environment)[^.\n]{0,60}(?:pass|true|ok|healthy)`],
    evidence: {
      A: [
        shell(R`(?:^|\s)status(?:\s|$)`, [R`0\.1\.0`]),
        shell(R`repo\s+list`, [R`"riltra"`, R`"other"`, R`"empty"`]),
        shell(R`\bdoctor\b`, [R`pass[^,]{0,10}true`])
      ],
      B: [mcp("health", [R`0\.1\.0`, R`repo_count[^0-9]{0,10}3`, R`pass[^,]{0,10}true`])]
    }
  }
];

const corpus = {
  schema: "cp7a-corpus-v1",
  frozen_at: new Date().toISOString(),
  source_head: "0fd22a1134274c1e92f741fc0d4e3b4886ceeaf3",
  model: { providerID: "opencode", id: "deepseek-v4.1-flash-free", paid: false, backend_revision: "unverifiable" },
  access,
  scoring: {
    rule: "A task is correct only when every required regex matches the concatenated assistant text. Evidence is valid only when every evidence item matches a successful tool call: tool name, optional CLI command regex, and all output regexes. No retries. Startup/warm-up tokens are excluded from task totals.",
    regex_flags: "i",
    order: "Alternating A-first/B-first by task index; fixed before measurement.",
    retry: "zero automatic retries"
  },
  tasks
};

const dir = __dirname;
fs.writeFileSync(path.join(dir, "corpus.json"), JSON.stringify(corpus, null, 2) + "\n");
console.log(JSON.stringify({ tasks: tasks.length, out: path.join(dir, "corpus.json") }));
