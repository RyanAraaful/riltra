// Regression tests for the CP2 grading validator.
//
// The validator must keep three things apart:
//   PASS         the evidence was available and the criterion holds
//   FAIL         the evidence was available and the criterion does not hold
//   UNMET        a capability criterion whose evidence was available
//   UNAVAILABLE  the evidence itself could not be obtained
//
// A parse error is never a PASS, an empty array is never proof of
// completeness, an impact verdict must be exactly UNKNOWN, and a CUDA
// terminus is unavailable because CUDA sources are not indexed - not
// because the source changed.
//
// Usage: node --test grade.test.cjs
"use strict";

const test = require("node:test");
const assert = require("node:assert");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const { grade, gradeChecks, classifyError, loadQuery, V } = require("./grade.cjs");

// --- helpers -------------------------------------------------------------

function writeQueries(files) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cp2-grade-"));
  for (const [name, content] of Object.entries(files)) {
    fs.writeFileSync(path.join(dir, name + ".json"), content);
  }
  return dir;
}

function find(checks, id) {
  const c = checks.find((x) => x.id === id);
  assert.ok(c, "the check " + id + " must exist");
  return c;
}

const CANDIDATE_EDGE = {
  kind: "POTENTIAL_CALL",
  callee_name: "CreateWorkspace",
  caller_path: "colmap/src/colmap/mvs/patch_match.cc",
  caller_span: [1, 2],
};

// --- the error classification -------------------------------------------

test("a parse error is classified by its actual reason", () => {
  assert.strictEqual(classifyError("error: anchor unresolved: source-changed; reindex before syntax fallback"), "source-changed");
  assert.strictEqual(classifyError('{"kind":"not_registered","message":"unknown repository"}'), "domain-error:not_registered");
  assert.strictEqual(classifyError("error: unsupported file type"), "unsupported");
  assert.strictEqual(classifyError("something went wrong"), "unclassified-error");
});

test("a missing output is not an answer", () => {
  const dir = writeQueries({});
  const q = loadQuery(dir, "does-not-exist");
  assert.strictEqual(q.available, false);
  assert.strictEqual(q.reason, "missing-output");
});

test("a missing CUDA output is not a clean failure", () => {
  // The file was never written: nothing was observed, so the honesty check
  // cannot pass.
  const dir = writeQueries({});
  const checks = grade(dir).checks;
  const honesty = find(checks, "D3-H4");
  assert.notStrictEqual(honesty.verdict, V.PASS);
  assert.strictEqual(honesty.verdict, V.UNAVAILABLE);
  assert.strictEqual(honesty.reason, "missing-output");
});

test("an unclassifiable error is not a clean failure", () => {
  const dir = writeQueries({
    "d3-2-trace-to-cuda": "something went wrong\n",
  });
  const checks = grade(dir).checks;
  const honesty = find(checks, "D3-H4");
  assert.strictEqual(honesty.verdict, V.UNAVAILABLE);
  assert.strictEqual(honesty.reason, "unclassified-error");
});

// --- the honesty gates ---------------------------------------------------

test("a parse error never becomes an honesty PASS", () => {
  const dir = writeQueries({
    "d1-1-search-Print": "error: anchor unresolved: source-changed; reindex before syntax fallback",
  });
  const result = grade(dir);
  const check = find(result.checks, "D1-H1");
  assert.notStrictEqual(check.verdict, V.PASS);
  assert.strictEqual(check.verdict, V.UNAVAILABLE);
  assert.strictEqual(check.reason, "source-changed");
});

test("an empty edge list is not evidence for the label check", () => {
  const dir = writeQueries({
    "d1-5-callers-PrintDef": JSON.stringify({ edges: [], anchor: {}, completeness: { unresolved: ["x"] } }),
    "d1-7-callers-normalize_vec": JSON.stringify({ edges: [], anchor: {}, completeness: { unresolved: ["x"] } }),
    "d2-3-callees-CtrlRun": JSON.stringify({ edges: [], anchor: {}, completeness: { unresolved: ["x"] } }),
    "d2-4-callers-CreateWorkspace": JSON.stringify({ edges: [], anchor: {}, completeness: { unresolved: ["x"] } }),
  });
  const result = grade(dir);
  const check = find(result.checks, "D1-H2");
  assert.strictEqual(check.verdict, V.UNAVAILABLE);
  assert.strictEqual(check.reason, "no-reference-returned");
});

test("a returned reference that is a candidate passes the label check", () => {
  const dir = writeQueries({
    "d1-5-callers-PrintDef": JSON.stringify({ edges: [], anchor: {}, completeness: { unresolved: ["x"] } }),
    "d1-7-callers-normalize_vec": JSON.stringify({ edges: [CANDIDATE_EDGE], anchor: {}, completeness: { unresolved: ["x"] } }),
    "d2-3-callees-CtrlRun": JSON.stringify({ edges: [CANDIDATE_EDGE], anchor: {}, completeness: { unresolved: ["x"] } }),
    "d2-4-callers-CreateWorkspace": JSON.stringify({ edges: [CANDIDATE_EDGE], anchor: {}, completeness: { unresolved: ["x"] } }),
  });
  const result = grade(dir);
  const check = find(result.checks, "D1-H2");
  assert.strictEqual(check.verdict, V.PASS);
  assert.strictEqual(check.evidence.inspected_edges, 3);
});

test("a proven CALLS edge fails the label check", () => {
  const proven = Object.assign({}, CANDIDATE_EDGE, { kind: "CALLS" });
  const dir = writeQueries({
    "d1-7-callers-normalize_vec": JSON.stringify({ edges: [proven], anchor: {}, completeness: { unresolved: ["x"] } }),
  });
  const result = grade(dir);
  const check = find(result.checks, "D1-H2");
  assert.strictEqual(check.verdict, V.FAIL);
  assert.strictEqual(check.reason, "proven-calls-edge-returned");
});

test("the impact verdict must be exactly UNKNOWN", () => {
  const base = {
    verdict: "SAFE",
    completeness: { status: "partial", unresolved: ["graph-completeness-unproven"] },
    disclaimer: "plausible-impact-not-execution",
    paths: [],
  };
  const dir = writeQueries({ "d4-1-impact-member": JSON.stringify(base) });
  let check = find(grade(dir).checks, "D4-H2");
  assert.strictEqual(check.verdict, V.FAIL);
  assert.strictEqual(check.reason, "verdict-is-SAFE");

  base.verdict = "LOW";
  const dir2 = writeQueries({ "d4-1-impact-member": JSON.stringify(base) });
  check = find(grade(dir2).checks, "D4-H2");
  assert.strictEqual(check.verdict, V.FAIL);
});

test("an UNKNOWN impact verdict with partial completeness passes the honesty gates", () => {
  const dir = writeQueries({
    "d4-1-impact-member": JSON.stringify({
      verdict: "UNKNOWN",
      completeness: { status: "partial", unresolved: ["graph-completeness-unproven", "insufficient-impact-evidence"] },
      disclaimer: "plausible-impact-not-execution",
      paths: [],
    }),
  });
  const checks = grade(dir).checks;
  assert.strictEqual(find(checks, "D4-H2").verdict, V.PASS);
  assert.strictEqual(find(checks, "D4-H3").verdict, V.PASS);
  // An empty affected set is a capability gap, never proof of safety.
  const cap = find(checks, "D4-C5");
  assert.strictEqual(cap.verdict, V.UNMET);
  assert.strictEqual(cap.reason, "affected-set-empty");
});

// --- the CUDA terminus ---------------------------------------------------

test("the CUDA terminus is unavailable because CUDA sources are not indexed", () => {
  const dir = writeQueries({
    "d3-2-trace-to-cuda": "error: anchor unresolved: source-changed; reindex before syntax fallback\n",
  });
  const checks = grade(dir).checks;
  const honesty = find(checks, "D3-H4");
  assert.strictEqual(honesty.verdict, V.PASS, "the tool failed cleanly instead of inventing a path");
  assert.strictEqual(honesty.evidence.observed_reason, "source-changed");
  assert.strictEqual(honesty.evidence.actual_cause, "cuda-unsupported");
  const evidence = find(checks, "D3-E1");
  assert.strictEqual(evidence.verdict, V.UNAVAILABLE);
  assert.strictEqual(evidence.reason, "cuda-unsupported");
});

test("a source-changed error on a non-CUDA target keeps its own reason", () => {
  const dir = writeQueries({
    "d3-2-trace-to-cuda": "error: anchor unresolved: source-changed; reindex before syntax fallback\n",
  });
  const checks = grade(dir, { cudaTargets: [] }).checks;
  const evidence = find(checks, "D3-E1");
  // The target of this query is not a CUDA file, so the observed reason is
  // not reclassified.
  assert.strictEqual(evidence.reason, "source-changed");
  assert.strictEqual(evidence.evidence.actual_cause, "source-changed");
});

// --- the capability gates ------------------------------------------------

test("an available answer that does not cover the capability is UNMET, not FAIL", () => {
  const dir = writeQueries({
    "d3-1-trace-depth4": JSON.stringify({
      disclaimer: "plausible-path-not-execution",
      paths: [{ edges: [CANDIDATE_EDGE] }],
    }),
    "p2-callers-class-PatchMatch": JSON.stringify({ edges: [], anchor: {}, completeness: { unresolved: ["x"] } }),
  });
  const checks = grade(dir).checks;
  const cap = find(checks, "D3-C5");
  assert.strictEqual(cap.verdict, V.UNMET);
  assert.strictEqual(cap.reason, "trace-does-not-extend-beyond-the-first-hop");
  assert.strictEqual(cap.evidence.max_hops_returned, 1);
});

test("a missing capability evidence is UNAVAILABLE, not UNMET", () => {
  const dir = writeQueries({});
  const checks = grade(dir).checks;
  const cap = find(checks, "D2-C6");
  assert.strictEqual(cap.verdict, V.UNAVAILABLE);
  assert.strictEqual(cap.reason, "missing-output");
});

// --- the agent side ------------------------------------------------------

test("a blocked OpenCode session is not a passing agent session", () => {
  const dir = writeQueries({});
  const result = grade(dir, {
    opencodeSession: { exit_code: 1, reason: "provider-auth-403", error: "FreeTierError" },
  });
  const check = find(result.checks, "D5-E1");
  assert.strictEqual(check.verdict, V.UNAVAILABLE);
  assert.strictEqual(check.reason, "provider-auth-403");
});

test("the impact honesty check inspects the whole answer, including the empty pilot shape", () => {
  // CP1 D.4.5 pre-registers the empty path set as the expected pilot
  // result, so the answer is fully inspected: no edge is claimed for the
  // member read, the binding or the macro arguments.
  const dir = writeQueries({
    "d4-1-impact-member": JSON.stringify({
      verdict: "UNKNOWN",
      completeness: { status: "partial", unresolved: ["graph-completeness-unproven", "empty-result-not-coverage-proof"] },
      disclaimer: "plausible-impact-not-execution",
      paths: [],
      bridges: [],
      visits: { nodes: 1, edges: 0 },
    }),
  });
  const checks = grade(dir).checks;
  assert.strictEqual(find(checks, "D4-H4").verdict, V.PASS);
  // The capability gate stays unmet: the affected set is empty.
  assert.strictEqual(find(checks, "D4-C5").verdict, V.UNMET);
});

test("a reported edge that is not a candidate fails the D.2 edge-label check", () => {
  const dir = writeQueries({
    "d2-5-callers-optmgr-binding": JSON.stringify({
      edges: [{ kind: "CALLS", callee_name: "max_triangulation_angle", caller_path: "x.cc", caller_span: [1, 2] }],
      anchor: {},
      completeness: { unresolved: ["x"] },
    }),
  });
  const checks = grade(dir).checks;
  assert.strictEqual(find(checks, "D2-H5").verdict, V.FAIL);
});

test("a workflow without an evidence check reports no evidence gate", () => {  const dir = writeQueries({
    "d1-1-search-Print": JSON.stringify({
      repo: "mmsp-pin",
      freshness: { status: "fresh", reasons: [] },
      total: 1,
      results: [{ path: "colmap/src/colmap/mvs/patch_match.cc", span: { start_line: 118 } }],
    }),
  });
  const result = grade(dir);
  // The honesty checks that need the other queries are unavailable, but the
  // evidence layer has no check for D1 at all: that is "n/a", not
  // "unavailable".
  assert.strictEqual(result.summary.gates.D1.evidence.verdict, "n/a");
});

test("the workflow gate reports an unavailable check as unavailable", () => {
  const dir = writeQueries({});
  const result = grade(dir);
  assert.strictEqual(result.summary.gates.D4.honesty.verdict, V.UNAVAILABLE);
  assert.strictEqual(result.summary.gates.D4.capability.verdict, V.UNAVAILABLE);
});

test("a capability gate is never PASS while a capability check is UNMET", () => {
  const dir = writeQueries({
    "d3-1-trace-depth4": JSON.stringify({
      disclaimer: "plausible-path-not-execution",
      paths: [{ edges: [CANDIDATE_EDGE] }],
    }),
  });
  const result = grade(dir);
  assert.strictEqual(find(result.checks, "D3-C5").verdict, V.UNMET);
  assert.strictEqual(result.summary.gates.D3.capability.verdict, V.UNMET);
});

test("an evidence check does not gate the capability of its workflow", () => {
  const dir = writeQueries({
    "d3-1-trace-depth4": JSON.stringify({
      disclaimer: "plausible-path-not-execution",
      paths: [{ edges: [CANDIDATE_EDGE] }],
    }),
    "d3-2-trace-to-cuda": "error: anchor unresolved: source-changed; reindex before syntax fallback\n",
  });
  const result = grade(dir);
  assert.strictEqual(result.summary.gates.D3.capability.verdict, V.UNMET);
  assert.strictEqual(result.summary.gates.D3.evidence.verdict, V.UNAVAILABLE);
});

// --- the MCP tool surface -------------------------------------------------

// The approved MCP surface: twelve tools. The twelfth is the semantic tool
// (riltra_code_semantic). A catalog is accepted only when it has exactly
// that many tools AND contains every required tool - the count alone is
// not the contract.
const TWELVE_TOOLS = [
  "riltra_code_callees",
  "riltra_code_callers",
  "riltra_code_impact",
  "riltra_code_search",
  "riltra_code_semantic",
  "riltra_code_trace",
  "riltra_health",
  "riltra_knowledge_related",
  "riltra_knowledge_search",
  "riltra_knowledge_show",
  "riltra_repo_list",
  "riltra_repo_status",
];

function smoke(overrides) {
  return Object.assign(
    {
      tool_list: TWELVE_TOOLS.slice(),
      stdout_pure: { lines: 6, parsed: 6, non_json: 0 },
      stderr: "",
      calls: [
        { id: 3, name: "riltra_code_search", present: true, is_error: false, text_len: 709, text: '{"repo":"mmsp-pin","freshness":"fresh"}' },
        { id: 5, name: "riltra_code_search", present: true, is_error: true, text_len: 74, text: '{"kind":"not_registered","message":"unknown repository"}' },
      ],
      classified_error: '{"kind":"not_registered","message":"unknown repository"}',
    },
    overrides || {},
  );
}

function gradeSmoke(mcpSmoke) {
  // No query evidence: only the D.5 surface checks are judged here.
  return grade(writeQueries({}), { mcpSmoke: mcpSmoke });
}

test("the MCP surface with the approved twelve-tool catalog passes", () => {
  const check = find(gradeSmoke(smoke()).checks, "D5-H1");
  assert.strictEqual(check.verdict, V.PASS);
  assert.strictEqual(check.evidence.tool_count, 12);
});

test("an eleven-tool catalog is rejected", () => {
  const check = find(gradeSmoke(smoke({ tool_list: TWELVE_TOOLS.slice(0, 11) })).checks, "D5-H1");
  assert.strictEqual(check.verdict, V.FAIL);
  assert.strictEqual(check.reason, "mcp-tool-surface-mismatch");
});

test("a thirteen-tool catalog is rejected", () => {
  const check = find(gradeSmoke(smoke({ tool_list: TWELVE_TOOLS.concat(["riltra_code_extra"]) })).checks, "D5-H1");
  assert.strictEqual(check.verdict, V.FAIL);
  assert.strictEqual(check.reason, "mcp-tool-surface-mismatch");
});

test("a twelve-tool catalog without the semantic tool is rejected", () => {
  const without = TWELVE_TOOLS.filter((t) => t !== "riltra_code_semantic").concat(["riltra_code_other"]);
  const check = find(gradeSmoke(smoke({ tool_list: without })).checks, "D5-H1");
  assert.strictEqual(check.verdict, V.FAIL);
  assert.strictEqual(check.reason, "mcp-tool-surface-mismatch");
});

test("a twelve-tool catalog missing another approved tool is rejected", () => {
  // The count alone is not the contract: a catalog of the right size that
  // lacks an approved tool is a different surface.
  const without = TWELVE_TOOLS.filter((t) => t !== "riltra_code_callers").concat(["riltra_code_other"]);
  const check = find(gradeSmoke(smoke({ tool_list: without })).checks, "D5-H1");
  assert.strictEqual(check.verdict, V.FAIL);
  assert.strictEqual(check.reason, "mcp-tool-surface-mismatch");
});

test("a twelve-tool catalog with a duplicate is rejected", () => {
  const duplicated = TWELVE_TOOLS.slice(0, 11).concat([TWELVE_TOOLS[0]]);
  const check = find(gradeSmoke(smoke({ tool_list: duplicated })).checks, "D5-H1");
  assert.strictEqual(check.verdict, V.FAIL);
  assert.strictEqual(check.reason, "mcp-tool-surface-mismatch");
});

test("a session that reached no tool is not a passing agent session", () => {
  const dir = writeQueries({});
  const result = grade(dir, {
    opencodeSession: { exit_code: 0, reason: null, error: null, tools: [], cited_sources: false },
  });
  const check = find(result.checks, "D5-E1");
  assert.strictEqual(check.verdict, V.UNAVAILABLE);
  assert.strictEqual(check.reason, "session-reached-no-tool");
  // A session that reached nothing has no answer to judge: the honesty
  // check is not available either, exactly like a blocked session.
  assert.strictEqual(result.checks.filter((c) => c.id === "D5-H3").length, 0);
});

test("a surface whose stdout is not pure is rejected", () => {
  const check = find(gradeSmoke(smoke({ stdout_pure: { lines: 6, parsed: 5, non_json: 1 } })).checks, "D5-H1");
  assert.strictEqual(check.verdict, V.FAIL);
  assert.strictEqual(check.reason, "mcp-surface-did-not-answer-cleanly");
});

test("a surface that wrote to stderr is rejected", () => {
  const check = find(gradeSmoke(smoke({ stderr: "warning" })).checks, "D5-H1");
  assert.strictEqual(check.verdict, V.FAIL);
  assert.strictEqual(check.reason, "mcp-surface-did-not-answer-cleanly");
});

test("a surface that answered no call is rejected", () => {
  const check = find(gradeSmoke(smoke({ calls: [] })).checks, "D5-H1");
  assert.strictEqual(check.verdict, V.FAIL);
  assert.strictEqual(check.reason, "mcp-surface-did-not-answer-cleanly");
});

test("an answer without the repository identity is rejected", () => {
  const calls = [{ id: 3, name: "riltra_code_search", present: true, is_error: false, text_len: 12, text: '{"total":1}' }];
  const check = find(gradeSmoke(smoke({ calls: calls })).checks, "D5-H1");
  assert.strictEqual(check.verdict, V.FAIL);
  assert.strictEqual(check.reason, "mcp-surface-did-not-answer-cleanly");
});

test("the classified error check still requires a classified error", () => {
  const calls = [{ id: 5, name: "riltra_code_search", present: true, is_error: true, text_len: 20, text: "it exploded" }];
  const check = find(gradeSmoke(smoke({ calls: calls })).checks, "D5-H2");
  assert.strictEqual(check.verdict, V.FAIL);
  assert.strictEqual(check.reason, "no-classified-error");
});

// --- source-grounded capability predicates (correction of 2026-10-25) ---------
//
// Each fixture is a small source tree with known lines. The search output names
// the tree as its root, as a real run does, so a span can be placed on a line.
// The rule under test: a capability PASS needs a proven relation at the frozen
// site, and a line-based check reads the source instead of comparing a byte
// offset with a line number.

const FX = {
  pmCc: "colmap/src/colmap/mvs/patch_match.cc",
  pmH: "colmap/src/colmap/mvs/patch_match.h",
  pmCuda: "colmap/src/colmap/mvs/patch_match_cuda.cu",
  pmCudaH: "colmap/src/colmap/mvs/patch_match_cuda.h",
  fusionCc: "colmap/src/colmap/mvs/fusion.cc",
  optH: "colmap/src/colmap/mvs/patch_match_options.h",
  optCc: "colmap/src/colmap/mvs/patch_match_options.cc",
  optMgrCc: "colmap/src/colmap/controllers/option_manager.cc",
  mvsCc: "colmap/src/colmap/exe/mvs.cc",
  autoCc: "colmap/src/colmap/controllers/automatic_reconstruction.cc",
  denseCc: "colmap/src/colmap/ui/dense_reconstruction_widget.cc",
};

// A source tree: each file maps 1-based line numbers to text; other lines are
// blank. Returns the root directory.
function makeSnapshot(files) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "cp2-src-"));
  for (const [rel, lines] of Object.entries(files)) {
    const last = Math.max(...Object.keys(lines).map(Number));
    const text = [];
    for (let n = 1; n <= last; n++) text.push(lines[n] === undefined ? "" : lines[n]);
    fs.mkdirSync(path.dirname(path.join(root, rel)), { recursive: true });
    fs.writeFileSync(path.join(root, rel), text.join("\n") + "\n");
  }
  return root;
}

// The byte offset at which a 1-based line starts, as Riltra reports spans.
function lineStart(root, rel, line) {
  const bytes = fs.readFileSync(path.join(root, rel));
  let offset = 0;
  for (let n = 1; n < line; n++) offset = bytes.indexOf(0x0a, offset) + 1;
  return offset;
}

// A syntactic definition endpoint that starts on `line` of `rel`.
function defAt(root, rel, line) {
  return { type: "syntax_def", path: rel, byte_start: lineStart(root, rel, line), byte_end: lineStart(root, rel, line) + 1 };
}

// An edge whose caller starts on `line` of `rel`. The default is a candidate.
function edgeAt(root, rel, line, fields) {
  const start = lineStart(root, rel, line);
  return Object.assign(
    { kind: "POTENTIAL_CALL", precision: "Heuristic", unresolved: "dynamic", callee: null,
      caller_path: rel, caller_span: [start, start + 4], callee_name: "X" },
    fields || {},
  );
}

// A relation the tool proved, from `line` of `rel` to `callee`.
function proven(root, rel, line, callee, fields) {
  return edgeAt(root, rel, line, Object.assign(
    { kind: "CALLS", precision: "Exact", unresolved: null, callee: callee }, fields || {}));
}

// The search output that names the snapshot root, written as a real run writes it.
function searchNamingRoot(root) {
  return JSON.stringify({ root: "\\\\?\\" + root, results: [], total: 0 });
}

// --- D1-C6: the identity of the true reference at :667

function d1Root() {
  return makeSnapshot({
    [FX.pmCc]: { 118: "void PatchMatch::Problem::Print() const {", 667: "  problem.Print();", 668: "  patch_match_options.Print();" },
    [FX.fusionCc]: { 144: "  options_.Print();" },
  });
}

function d1Dir(root, buildEdges, withRoot) {
  const anchor = defAt(root, FX.pmCc, 118);
  const files = {
    "d1-5-callers-PrintDef": JSON.stringify({ anchor: { path: FX.pmCc, line: 118, endpoint: anchor }, edges: buildEdges(root, anchor) }),
  };
  if (withRoot !== false) files["d1-1-search-Print"] = searchNamingRoot(root);
  return writeQueries(files);
}

test("D1-C6 passes only when the reference at :667 is proven to the anchored definition", () => {
  const root = d1Root();
  const dir = d1Dir(root, (r, anchor) => [
    proven(r, FX.pmCc, 667, anchor, { callee_name: "Print" }),
    edgeAt(r, FX.pmCc, 668, { callee_name: "Print" }),
  ]);
  const check = find(grade(dir).checks, "D1-C6");
  assert.strictEqual(check.verdict, V.PASS);
  assert.strictEqual(check.evidence.proven_at_667, true);
});

test("D1-C6 does not pass on name candidates alone", () => {
  const root = d1Root();
  const dir = d1Dir(root, (r) => [
    edgeAt(r, FX.pmCc, 667, { callee_name: "Print" }),
    edgeAt(r, FX.pmCc, 668, { callee_name: "Print" }),
    edgeAt(r, FX.fusionCc, 144, { callee_name: "Print" }),
  ]);
  const check = find(grade(dir).checks, "D1-C6");
  assert.strictEqual(check.verdict, V.UNMET);
  assert.strictEqual(check.reason, "identity-not-proven-at-667");
});

test("D1-C6 fails when another reference is proven to the same definition", () => {
  const root = d1Root();
  const dir = d1Dir(root, (r, anchor) => [
    proven(r, FX.pmCc, 667, anchor, { callee_name: "Print" }),
    proven(r, FX.pmCc, 668, anchor, { callee_name: "Print" }),
  ]);
  const check = find(grade(dir).checks, "D1-C6");
  assert.strictEqual(check.verdict, V.FAIL);
  assert.strictEqual(check.reason, "other-site-proven-to-definition");
});

test("D1-C6 is UNAVAILABLE without the snapshot source, never decided by a byte offset", () => {
  const root = d1Root();
  const dir = d1Dir(root, (r, anchor) => [proven(r, FX.pmCc, 667, anchor, { callee_name: "Print" })], false);
  const check = find(grade(dir).checks, "D1-C6");
  assert.strictEqual(check.verdict, V.UNAVAILABLE);
  assert.strictEqual(check.reason, "source-unavailable");
});

// --- D2-C6: the three construction sites, proven to the class

function d2Root() {
  return makeSnapshot({
    [FX.pmH]: { 129: "class PatchMatchController : public BaseController {" },
    [FX.mvsCc]: { 271: "  mvs::PatchMatchController controller(options," },
    [FX.autoCc]: { 431: "    mvs::PatchMatchController patch_match_controller(" },
    [FX.denseCc]: {
      397: "      std::make_unique<ControllerThread<mvs::PatchMatchController>>(",
      398: "          std::make_shared<mvs::PatchMatchController>(",
    },
  });
}

function d2Dir(root, buildEdges) {
  const classDef = defAt(root, FX.pmH, 129);
  return writeQueries({
    "d1-1-search-Print": searchNamingRoot(root),
    "p2-callers-class-PatchMatch": JSON.stringify({
      anchor: { path: FX.pmH, line: 129, endpoint: classDef },
      edges: buildEdges(root, classDef),
    }),
  });
}

test("D2-C6 passes only when all three construction sites are proven to the class", () => {
  const root = d2Root();
  const dir = d2Dir(root, (r, cls) => [
    proven(r, FX.mvsCc, 271, cls, { callee_name: "PatchMatchController" }),
    proven(r, FX.autoCc, 431, cls, { callee_name: "PatchMatchController" }),
    proven(r, FX.denseCc, 398, cls, { callee_name: "PatchMatchController" }),
  ]);
  assert.strictEqual(find(grade(dir).checks, "D2-C6").verdict, V.PASS);
});

test("D2-C6 does not pass on candidates at the three construction sites", () => {
  const root = d2Root();
  const dir = d2Dir(root, (r) => [
    edgeAt(r, FX.mvsCc, 271, { callee_name: "PatchMatchController" }),
    edgeAt(r, FX.autoCc, 431, { callee_name: "PatchMatchController" }),
    edgeAt(r, FX.denseCc, 398, { callee_name: "PatchMatchController", unresolved: "factory-template-argument" }),
  ]);
  const check = find(grade(dir).checks, "D2-C6");
  assert.strictEqual(check.verdict, V.UNMET);
  assert.strictEqual(check.reason, "candidate-only-attribution");
});

test("D2-C6 names a missing construction site instead of passing on file presence", () => {
  const root = d2Root();
  const dir = d2Dir(root, (r, cls) => [
    proven(r, FX.autoCc, 431, cls, { callee_name: "PatchMatchController" }),
    proven(r, FX.denseCc, 398, cls, { callee_name: "PatchMatchController" }),
  ]);
  const check = find(grade(dir).checks, "D2-C6");
  assert.strictEqual(check.verdict, V.UNMET);
  assert.strictEqual(check.reason, "construction-sites-not-attributed");
});

// --- D3-C5: the frozen chain, hop by hop

function d3Root() {
  return makeSnapshot({
    [FX.pmCc]: {
      191: "void PatchMatch::Run() {",
      197: "  patch_match_cuda_->Run();",
      268: "      thread_pool_->AddTask(&PatchMatchController::ProcessProblem,",
      671: "  patch_match.Run();",
    },
    [FX.pmCuda]: { 1326: "void PatchMatchCuda::Run() {" },
    [FX.pmCudaH]: { 54: "  void Run();" },
  });
}

function d3Dir(root, edges) {
  return writeQueries({
    "d1-1-search-Print": searchNamingRoot(root),
    "d3-1-trace-depth4": JSON.stringify({ disclaimer: "plausible-path-not-execution", paths: [{ edges: edges }] }),
  });
}

test("D3-C5 passes on the frozen chain: a candidate dispatch, then two proven direct calls", () => {
  const root = d3Root();
  const dir = d3Dir(root, [
    edgeAt(root, FX.pmCc, 268, { callee_name: "ProcessProblem" }),
    proven(root, FX.pmCc, 671, defAt(root, FX.pmCc, 191), { callee_name: "Run" }),
    proven(root, FX.pmCc, 197, defAt(root, FX.pmCuda, 1326), { callee_name: "Run" }),
  ]);
  assert.strictEqual(find(grade(dir).checks, "D3-C5").verdict, V.PASS);
});

test("D3-C5 does not pass on three proven edges that are not the frozen chain", () => {
  const root = d3Root();
  const dir = d3Dir(root, [
    proven(root, FX.pmCc, 191, defAt(root, FX.pmCc, 191), { callee_name: "Run" }),
    proven(root, FX.pmCc, 671, defAt(root, FX.pmCc, 191), { callee_name: "Run" }),
    proven(root, FX.pmCc, 197, defAt(root, FX.pmCc, 191), { callee_name: "Run" }),
  ]);
  const check = find(grade(dir).checks, "D3-C5");
  assert.strictEqual(check.verdict, V.UNMET);
  assert.strictEqual(check.reason, "path-not-frozen-chain");
});

// --- D4-C5: the six frozen sites of max_triangulation_angle

function d4Root() {
  return makeSnapshot({
    [FX.optH]: { 56: "  double max_triangulation_angle = -1.0f;" },
    [FX.optCc]: {
      58: "  PrintOption(max_triangulation_angle);",
      91: "  if (max_triangulation_angle >= 0.0f) {",
      92: "    CHECK_OPTION_LE(max_triangulation_angle, 180.0f);",
      93: "    CHECK_OPTION_LE(min_triangulation_angle, max_triangulation_angle);",
    },
    [FX.pmCc]: {
      361: "  const bool has_max_triangulation_angle =",
      362: "      options_.max_triangulation_angle >= 0.0f;",
      444: "      (!has_max_triangulation_angle ||",
    },
    [FX.optMgrCc]: { 1019: "                   &patch_match_stereo->max_triangulation_angle);" },
  });
}

function d4Dir(root, buildEdges) {
  const anchor = defAt(root, FX.optH, 56);
  return writeQueries({
    "d1-1-search-Print": searchNamingRoot(root),
    "d4-1-impact-member": JSON.stringify({
      verdict: "UNKNOWN",
      anchor: { kind: "syntax-fallback", path: FX.optH, line: 56, col: 10, endpoint: anchor },
      completeness: { status: "partial", unresolved: ["graph-completeness-unproven"] },
      disclaimer: "plausible-impact-not-execution",
      paths: [{ edges: buildEdges(root, anchor) }],
      visits: { nodes: 1, edges: 0 },
    }),
  });
}

test("D4-C5 passes when each frozen site has a proven edge on its lines and the declaration is the anchor", () => {
  const root = d4Root();
  const dir = d4Dir(root, (r, anchor) => [
    proven(r, FX.optCc, 58, anchor, { kind: "REFERENCES", callee_name: "max_triangulation_angle" }),
    proven(r, FX.optCc, 92, anchor, { kind: "REFERENCES", callee_name: "max_triangulation_angle" }),
    proven(r, FX.pmCc, 362, anchor, { kind: "REFERENCES", callee_name: "max_triangulation_angle" }),
    proven(r, FX.pmCc, 444, anchor, { kind: "REFERENCES", callee_name: "max_triangulation_angle" }),
    proven(r, FX.optMgrCc, 1019, anchor, { kind: "REFERENCES", callee_name: "max_triangulation_angle" }),
  ]);
  assert.strictEqual(find(grade(dir).checks, "D4-C5").verdict, V.PASS);
});

test("D4-C5 does not pass on one unrelated affected row", () => {
  const root = d4Root();
  const dir = d4Dir(root, (r, anchor) => [
    proven(r, FX.pmCc, 362, anchor, { kind: "REFERENCES", callee_name: "max_triangulation_angle" }),
  ]);
  const check = find(grade(dir).checks, "D4-C5");
  assert.strictEqual(check.verdict, V.UNMET);
  assert.strictEqual(check.reason, "affected-set-incomplete");
});

test("D4-C5 does not count a candidate on a frozen site as covered", () => {
  const root = d4Root();
  const dir = d4Dir(root, (r, anchor) => [
    edgeAt(r, FX.optCc, 58, { callee_name: "PrintOption" }),
    proven(r, FX.optCc, 92, anchor, { kind: "REFERENCES", callee_name: "max_triangulation_angle" }),
    proven(r, FX.pmCc, 362, anchor, { kind: "REFERENCES", callee_name: "max_triangulation_angle" }),
    proven(r, FX.pmCc, 444, anchor, { kind: "REFERENCES", callee_name: "max_triangulation_angle" }),
    proven(r, FX.optMgrCc, 1019, anchor, { kind: "REFERENCES", callee_name: "max_triangulation_angle" }),
  ]);
  const check = find(grade(dir).checks, "D4-C5");
  assert.strictEqual(check.verdict, V.UNMET);
  assert.strictEqual(check.reason, "affected-set-incomplete");
  assert.strictEqual(check.evidence.covered.find((c) => c.site === "print").covered, false);
});
