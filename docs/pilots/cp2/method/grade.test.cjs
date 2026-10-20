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

const { grade, classifyError, loadQuery, V } = require("./grade.cjs");

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
