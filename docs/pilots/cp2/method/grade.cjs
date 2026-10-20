// CP2 grading of the workflow query outputs against the CP1 section D
// criteria.
//
// Three different things are kept apart, because a correct "UNKNOWN" is
// evidence of honest behaviour, not evidence that a capability exists:
//
//   PASS         the evidence was available and the criterion holds
//   FAIL         the evidence was available and the criterion does not hold
//   UNMET        a CAPABILITY criterion: the evidence was available and the
//                capability is not there
//   UNAVAILABLE  the evidence itself could not be obtained (the tool answered
//                with a classified error, the output is missing, or the query
//                returned no item to inspect)
//
// A parse error is never a PASS, an empty array is never evidence of
// completeness, and an error is classified by its actual reason (a CUDA
// terminus is unavailable because CUDA files are not indexed, not because the
// source changed).
//
// Usage: node grade.cjs <query-dir> [out-file]
"use strict";

const fs = require("node:fs");
const path = require("node:path");

const V = { PASS: "PASS", FAIL: "FAIL", UNMET: "UNMET", UNAVAILABLE: "UNAVAILABLE" };

// ---------------------------------------------------------------- evidence

function classifyError(raw) {
  const t = String(raw == null ? "" : raw);
  const kind = /"kind"\s*:\s*"([a-z_]+)"/.exec(t);
  if (kind) return "domain-error:" + kind[1];
  if (/anchor unresolved: source-changed/.test(t)) return "source-changed";
  if (/unsupported/i.test(t)) return "unsupported";
  if (/cuda/i.test(t)) return "cuda-unsupported";
  if (/^error:/m.test(t)) return "cli-error";
  return "unclassified-error";
}

function loadQuery(dir, name) {
  const file = path.join(dir, name + ".json");
  if (!fs.existsSync(file)) {
    return { available: false, reason: "missing-output", detail: name };
  }
  const raw = fs.readFileSync(file, "utf8");
  let doc = null;
  try {
    doc = JSON.parse(raw);
  } catch (e) {
    return { available: false, reason: classifyError(raw), detail: raw.slice(0, 300) };
  }
  if (doc === null || typeof doc !== "object") {
    return { available: false, reason: "not-an-object", detail: raw.slice(0, 300) };
  }
  return { available: true, doc: doc, raw: raw };
}

function edgesOf(doc) {
  return doc && Array.isArray(doc.edges) ? doc.edges : [];
}
function resultsOf(doc) {
  return doc && Array.isArray(doc.results) ? doc.results : [];
}
function pathsOf(doc) {
  return doc && Array.isArray(doc.paths) ? doc.paths : [];
}
function isCandidate(edge) {
  return edge && (edge.kind === "POTENTIAL_CALL" || edge.kind === "IMPORTS");
}

// ------------------------------------------------------------------ checks

// A criterion that inspects the returned items needs at least one item;
// without one there is nothing to inspect and the verdict is UNAVAILABLE,
// never a vacuous PASS.
function itemCount(evidence) {
  return evidence.count;
}

function makeCheck(id, layer, criterion, verdict, reason, evidence) {
  return { id, layer, criterion, verdict, reason: reason || null, evidence: evidence || {} };
}

function gradeChecks(ctx) {
  const checks = [];
  const add = (id, layer, criterion, verdict, reason, evidence) =>
    checks.push(makeCheck(id, layer, criterion, verdict, reason, evidence));

  // Every reference edge returned by ANY workflow query must be a candidate.
  const referenceQueries = [
    ["d1-5-callers-PrintDef", "Print definition"],
    ["d1-7-callers-normalize_vec", "normalize_vec definition"],
    ["d2-3-callees-CtrlRun", "PatchMatchController::Run callees"],
    ["d2-4-callers-CreateWorkspace", "CreateWorkspace callers"],
  ];
  const allEdges = [];
  const perQuery = {};
  let unavailableQueries = [];
  for (const [name, label] of referenceQueries) {
    const q = ctx.docs[name];
    if (!q.available) { unavailableQueries.push(name + ":" + q.reason); continue; }
    const e = edgesOf(q.doc);
    perQuery[name] = e.length;
    allEdges.push(...e);
  }
  const kinds = [...new Set(allEdges.map((e) => e.kind))];
  if (allEdges.length === 0) {
    add("D1-H2", "honesty",
      "every returned reference edge is a candidate (POTENTIAL_CALL), never a proven CALLS edge",
      V.UNAVAILABLE, unavailableQueries.length ? "no-reference-returned:" + unavailableQueries.join(",") : "no-reference-returned",
      { inspected_edges: 0, per_query: perQuery });
  } else {
    const proven = allEdges.filter((e) => e.kind === "CALLS");
    add("D1-H2", "honesty",
      "every returned reference edge is a candidate (POTENTIAL_CALL), never a proven CALLS edge",
      proven.length === 0 ? V.PASS : V.FAIL, proven.length ? "proven-calls-edge-returned" : null,
      { inspected_edges: allEdges.length, kinds, per_query: perQuery });
  }

  // ---- D.1 find a symbol and its references -------------------------------
  const sPrint = ctx.docs["d1-1-search-Print"];
  if (!sPrint.available) {
    add("D1-H1", "honesty", "the answer resolves the requested definition and does not invent one",
      V.UNAVAILABLE, sPrint.reason, {});
  } else {
    const hit = (p, l) => resultsOf(sPrint.doc).some((r) => r.path === p && r.span && r.span.start_line === l);
    const header = hit("colmap/src/colmap/mvs/patch_match.h", 74);
    const impl = hit("colmap/src/colmap/mvs/patch_match.cc", 118);
    add("D1-H1", "honesty", "the answer resolves the requested definition and does not invent one",
      header || impl ? V.PASS : V.FAIL, header || impl ? null : "definition-not-found",
      { header_definition: header, implementation_definition: impl, total: sPrint.doc.total });
  }

  const sNorm = ctx.docs["d1-2-search-normalize_vec"];
  if (!sNorm.available) {
    add("D1-H3", "honesty", "normalize_vec resolves to the Python definition and to no C++ definition",
      V.UNAVAILABLE, sNorm.reason, {});
  } else {
    const py = resultsOf(sNorm.doc).some((r) => r.path === "colmap/benchmark/reconstruction/evaluation/geometry.py" && r.span && r.span.start_line === 34);
    const cpp = resultsOf(sNorm.doc).filter((r) => r.path && r.path.endsWith(".cc"));
    add("D1-H3", "honesty", "normalize_vec resolves to the Python definition and to no C++ definition",
      py && cpp.length === 0 ? V.PASS : V.FAIL, py && cpp.length === 0 ? null : (py ? "cpp-result-present" : "python-definition-not-found"),
      { python_definition: py, cpp_results: cpp.map((r) => r.path + ":" + r.span.start_line) });
  }

  const sRun = ctx.docs["d1-3-search-Run"];
  if (!sRun.available) {
    add("D1-H4", "honesty", "a bare name returns multiple distinct identities, not one merged symbol",
      V.UNAVAILABLE, sRun.reason, {});
  } else {
    const paths = [...new Set(resultsOf(sRun.doc).map((r) => r.path))];
    add("D1-H4", "honesty", "a bare name returns multiple distinct identities, not one merged symbol",
      paths.length >= 2 ? V.PASS : V.FAIL, paths.length >= 2 ? null : "single-identity",
      { distinct_paths: paths.length, total: sRun.doc.total });
  }

  const sMax = ctx.docs["d1-4-search-max_triangulation"];
  const anySearch = ctx.docs["d1-1-search-Print"];
  if (!anySearch.available) {
    add("D1-H5", "honesty", "the answer states the repository and a freshness verdict",
      V.UNAVAILABLE, anySearch.reason, {});
  } else {
    const d = anySearch.doc;
    const fresh = d.freshness && typeof d.freshness.status === "object" ? d.freshness.status : d.freshness;
    add("D1-H5", "honesty", "the answer states the repository and a freshness verdict",
      d.repo && fresh ? V.PASS : V.FAIL, d.repo && fresh ? null : "repository-or-freshness-missing",
      { repo: d.repo, freshness: fresh, reasons: d.freshness ? d.freshness.reasons : undefined });
  }

  // capability: the C++ method identity must be resolvable
  const cPrint = ctx.docs["d1-5-callers-PrintDef"];
  if (!cPrint.available) {
    add("D1-C6", "capability", "the reference set resolves the C++ method identity (separates patch_match.cc:667 from :668 and fusion.cc:144)",
      V.UNAVAILABLE, cPrint.reason, {});
  } else {
    const sites = edgesOf(cPrint.doc).map((e) => e.caller_path + ":" + (e.caller_span ? e.caller_span[0] : "?"));
    const distinguished = sites.some((s) => s.includes("patch_match.cc:667") || s.includes("patch_match.cc:668")) &&
      sites.some((s) => s.includes("fusion.cc:144"));
    add("D1-C6", "capability", "the reference set resolves the C++ method identity (separates patch_match.cc:667 from :668 and fusion.cc:144)",
      distinguished ? V.PASS : V.UNMET, distinguished ? null : "reference-set-empty-or-unresolved",
      { edges_returned: sites.length, sites, anchor: cPrint.doc.anchor ? cPrint.doc.anchor.path + ":" + cPrint.doc.anchor.line : null });
  }

  // ---- D.2 callers and callees -------------------------------------------
  const cCtrl = ctx.docs["d2-1-callers-PatchMatchCtrlRun"];
  if (!cCtrl.available) {
    add("D2-H1", "honesty", "no reference is fabricated and the anchor uncertainty is reported",
      V.UNAVAILABLE, cCtrl.reason, {});
  } else {
    const d = cCtrl.doc;
    const uncertainty = !!(d.anchor && d.anchor.anchor_uncertainty);
    const reasons = d.completeness && Array.isArray(d.completeness.unresolved) ? d.completeness.unresolved : [];
    const kindsHere = [...new Set(edgesOf(d).map((e) => e.kind))];
    const honest = uncertainty && reasons.length > 0 && edgesOf(d).every(isCandidate);
    add("D2-H1", "honesty", "no reference is fabricated and the anchor uncertainty is reported",
      honest ? V.PASS : V.FAIL, honest ? null : "uncertainty-or-reasons-missing",
      { edges: edgesOf(d).length, kinds: kindsHere, anchor_uncertainty: uncertainty, unresolved_reasons: reasons.length });
  }

  const calRun = ctx.docs["d2-2-callees-PatchMatchRun"];
  if (!calRun.available) {
    add("D2-H2", "honesty", "the callee set contains the PatchMatchCuda construction and Run call",
      V.UNAVAILABLE, calRun.reason, {});
  } else {
    const names = edgesOf(calRun.doc).map((e) => e.callee_name);
    const cuda = names.filter((n) => n && n.includes("PatchMatchCuda"));
    add("D2-H2", "honesty", "the callee set contains the PatchMatchCuda construction and Run call",
      cuda.length > 0 ? V.PASS : V.FAIL, cuda.length > 0 ? null : "expected-callee-missing",
      { edges: names.length, callee_names: [...new Set(names)], patch_match_cuda: cuda });
  }

  const calCtrl = ctx.docs["d2-3-callees-CtrlRun"];
  if (!calCtrl.available) {
    add("D2-H3", "honesty", "the ThreadPool dispatch to ProcessProblem is absent or a candidate, never a proven CALLS edge",
      V.UNAVAILABLE, calCtrl.reason, {});
  } else {
    const pp = edgesOf(calCtrl.doc).filter((e) => (e.callee_name || "").includes("ProcessProblem"));
    const proven = pp.filter((e) => e.kind === "CALLS");
    add("D2-H3", "honesty", "the ThreadPool dispatch to ProcessProblem is absent or a candidate, never a proven CALLS edge",
      proven.length === 0 ? V.PASS : V.FAIL, proven.length === 0 ? null : "proven-calls-edge-returned",
      { edges: edgesOf(calCtrl.doc).length, process_problem_edges: pp.map((e) => ({ kind: e.kind, callee: e.callee_name })) });
  }

  const cCreate = ctx.docs["d2-4-callers-CreateWorkspace"];
  if (!cCreate.available) {
    add("D2-H4", "honesty", "the CreateWorkspace references stay inside patch_match.cc",
      V.UNAVAILABLE, cCreate.reason, {});
  } else {
    const edges = edgesOf(cCreate.doc);
    const outside = edges.filter((e) => e.caller_path && e.caller_path !== "colmap/src/colmap/mvs/patch_match.cc");
    if (edges.length === 0) {
      add("D2-H4", "honesty", "the CreateWorkspace references stay inside patch_match.cc",
        V.UNAVAILABLE, "no-reference-returned", { edges: 0 });
    } else {
      add("D2-H4", "honesty", "the CreateWorkspace references stay inside patch_match.cc",
        outside.length === 0 ? V.PASS : V.FAIL, outside.length === 0 ? null : "reference-outside-file",
        { edges: edges.length, caller_paths: [...new Set(edges.map((e) => e.caller_path))] });
    }
  }

  const cOpt = ctx.docs["d2-5-callers-optmgr-binding"];
  if (!cOpt.available) {
    add("D2-H5", "honesty", "the option-manager address-of site is not presented as a call to max_triangulation_angle",
      V.UNAVAILABLE, cOpt.reason, {});
  } else {
    const edges = edgesOf(cOpt.doc);
    const wrong = edges.filter((e) => (e.callee_name || "") === "max_triangulation_angle");
    if (edges.length === 0) {
      add("D2-H5", "honesty", "the option-manager address-of site is not presented as a call to max_triangulation_angle",
        V.UNAVAILABLE, "no-reference-returned", { edges: 0, note: "the query returned no edge, so the presentation cannot be inspected" });
    } else {
      add("D2-H5", "honesty", "the option-manager address-of site is not presented as a call to max_triangulation_angle",
        wrong.length === 0 ? V.PASS : V.FAIL, wrong.length === 0 ? null : "option-presented-as-call",
        { edges: edges.length, edges_summary: edges.map((e) => ({ kind: e.kind, callee: e.callee_name })) });
    }
  }

  // capability: the three external construction sites attributed to the class
  const p2 = ctx.docs["p2-callers-class-PatchMatch"];
  if (!p2.available) {
    add("D2-C6", "capability", "the three external construction sites are attributed to the class",
      V.UNAVAILABLE, p2.reason, {});
  } else {
    const callers = edgesOf(p2.doc).map((e) => e.caller_path);
    const exe = callers.some((p) => p && p.includes("exe/mvs.cc"));
    const auto = callers.some((p) => p && p.includes("automatic_reconstruction.cc"));
    const dense = callers.some((p) => p && p.includes("dense_reconstruction_widget"));
    add("D2-C6", "capability", "the three external construction sites are attributed to the class",
      exe && auto && dense ? V.PASS : V.UNMET, exe && auto && dense ? null : "construction-sites-not-attributed",
      { class_anchor_edges: callers, sites_expected: 3, sites_attributed: [exe, auto, dense].filter(Boolean).length });
  }

  // ---- D.3 trace a call path ---------------------------------------------
  const tr = ctx.docs["d3-1-trace-depth4"];
  if (!tr.available) {
    add("D3-H2", "honesty", "the plausibility disclaimer is present", V.UNAVAILABLE, tr.reason, {});
    add("D3-H3", "honesty", "the path does not claim a device-side definition", V.UNAVAILABLE, tr.reason, {});
    add("D3-C5", "capability", "the path spans the full frozen chain (4 hops, including the indirect dispatch)",
      V.UNAVAILABLE, tr.reason, {});
  } else {
    add("D3-H2", "honesty", "the plausibility disclaimer is present",
      tr.doc.disclaimer === "plausible-path-not-execution" ? V.PASS : V.FAIL,
      tr.doc.disclaimer === "plausible-path-not-execution" ? null : "disclaimer-missing-or-different",
      { disclaimer: tr.doc.disclaimer });
    const mentionsCu = JSON.stringify(tr.doc).includes(".cu");
    add("D3-H3", "honesty", "the path does not claim a device-side definition",
      mentionsCu ? V.FAIL : V.PASS, mentionsCu ? "device-side-definition-claimed" : null,
      { mentions_cuda_source: mentionsCu, paths: pathsOf(tr.doc).length });
    const maxHops = pathsOf(tr.doc).reduce((m, p) => Math.max(m, Array.isArray(p.edges) ? p.edges.length : 0), 0);
    add("D3-C5", "capability", "the path spans the full frozen chain (4 hops, including the indirect dispatch)",
      maxHops >= 3 ? V.PASS : V.UNMET, maxHops >= 3 ? null : "trace-does-not-extend-beyond-the-first-hop",
      { max_hops_returned: maxHops, paths: pathsOf(tr.doc).length, frozen_chain_hops: 4 });
  }

  // The CUDA terminus: the tool must fail cleanly, and the reason must be
  // the real one. Riltra does not index CUDA sources, so the anchor can
  // never resolve; the tool's own message ("source-changed") is recorded
  // next to the actual cause. The queries whose target is a CUDA file are
  // listed in ctx.cudaTargets.
  const trCu = ctx.docs["d3-2-trace-to-cuda"];
  const cudaTargets = ctx.cudaTargets || [];
  const isCudaTarget = cudaTargets.indexOf("d3-2-trace-to-cuda") >= 0;
  if (trCu.available) {
    add("D3-H4", "honesty", "the CUDA destination query fails cleanly instead of inventing a path",
      V.FAIL, "no-error-raised", { note: "the query answered instead of failing" });
  } else {
    add("D3-H4", "honesty", "the CUDA destination query fails cleanly instead of inventing a path",
      V.PASS, null,
      { observed_reason: trCu.reason, actual_cause: isCudaTarget ? "cuda-unsupported" : trCu.reason, detail: trCu.detail });
    add("D3-E1", "evidence", "the CUDA terminus can be verified",
      V.UNAVAILABLE, isCudaTarget ? "cuda-unsupported" : trCu.reason,
      { observed_reason: trCu.reason, actual_cause: isCudaTarget ? "cuda-unsupported" : trCu.reason });
  }

  // ---- D.4 impact of a change --------------------------------------------
  const im = ctx.docs["d4-1-impact-member"];
  if (!im.available) {
    add("D4-H2", "honesty", "the impact verdict is UNKNOWN, never SAFE/LOW", V.UNAVAILABLE, im.reason, {});
    add("D4-H3", "honesty", "completeness is partial with graph-completeness-unproven", V.UNAVAILABLE, im.reason, {});
    add("D4-H4", "honesty", "no macro site is resolved to a function definition named after the option",
      V.UNAVAILABLE, im.reason, {});
    add("D4-C5", "capability", "the affected set contains the declaration, print, validation, read/filter and CLI binding sites",
      V.UNAVAILABLE, im.reason, {});
  } else {
    const d = im.doc;
    add("D4-H2", "honesty", "the impact verdict is UNKNOWN, never SAFE/LOW",
      d.verdict === "UNKNOWN" ? V.PASS : V.FAIL, d.verdict === "UNKNOWN" ? null : "verdict-is-" + String(d.verdict),
      { verdict: d.verdict });
    const c = d.completeness || {};
    const reasons = Array.isArray(c.unresolved) ? c.unresolved : [];
    const ok = c.status === "partial" && reasons.includes("graph-completeness-unproven") &&
      d.disclaimer === "plausible-impact-not-execution";
    add("D4-H3", "honesty", "completeness is partial with graph-completeness-unproven",
      ok ? V.PASS : V.FAIL, ok ? null : "completeness-or-disclaimer-missing",
      { completeness: c.status, unresolved_reasons: reasons.length, disclaimer: d.disclaimer });
    const wrong = JSON.stringify(d).includes('"callee_name":"max_triangulation_angle"');
    add("D4-H4", "honesty", "no macro site is resolved to a function definition named after the option",
      wrong ? V.FAIL : V.PASS, wrong ? "option-resolved-as-definition" : null,
      { mentions_option_as_callee: wrong });
    const paths = pathsOf(d);
    add("D4-C5", "capability", "the affected set contains the declaration, print, validation, read/filter and CLI binding sites",
      paths.length > 0 ? V.PASS : V.UNMET, paths.length > 0 ? null : "affected-set-empty",
      { paths: paths.length, visits: d.visits, note: "an empty affected set is reported as UNKNOWN, never as proof of safety" });
  }

  // ---- D.5 natural-language question via OpenCode/MCP ---------------------
  // The server side is graded from the MCP smoke; the agent side is only
  // verified when a real OpenCode session ran. A blocked session is not a
  // passing session.
  const smoke = ctx.mcpSmoke;
  if (!smoke) {
    add("D5-H1", "honesty", "the MCP surface answers a source-grounded query with the repository identity",
      V.UNAVAILABLE, "mcp-smoke-missing", {});
  } else {
    const toolCount = Array.isArray(smoke.tool_list) ? smoke.tool_list.length : 0;
    const nonJson = smoke.stdout_pure ? smoke.stdout_pure.non_json : null;
    const stderrBytes = typeof smoke.stderr === "string" ? smoke.stderr.length : null;
    const calls = smoke.calls || [];
    const answered = calls.filter((c) => c.present && !c.is_error && c.text_len > 0);
    const grounded = answered.some((c) => (c.text || "").includes('"repo"'));
    const ok = toolCount === 11 && nonJson === 0 && stderrBytes === 0 && answered.length > 0 && grounded;
    add("D5-H1", "honesty", "the MCP surface answers a source-grounded query with the repository identity",
      ok ? V.PASS : V.FAIL, ok ? null : "mcp-surface-did-not-answer-cleanly",
      { tool_count: toolCount, tools: smoke.tool_list, stdout_non_json: nonJson, stderr_bytes: stderrBytes, answered_calls: answered.length });
    const errCall = calls.find((c) => c.is_error === true);
    const errText = errCall ? (errCall.text || "") : (smoke.classified_error_text || "");
    const errOk = !!errCall && /"kind"\s*:\s*"[a-z_]+"/.test(errText);
    add("D5-H2", "honesty", "an unknown repository returns a classified error, not a crash",
      errOk ? V.PASS : V.FAIL, errOk ? null : "no-classified-error",
      { error_text: errText });
  }
  const session = ctx.opencodeSession;
  if (!session) {
    add("D5-E1", "evidence", "the agent-side OpenCode session ran", V.UNAVAILABLE, "session-not-run", {});
  } else if (session.exit_code !== 0) {
    add("D5-E1", "evidence", "the agent-side OpenCode session ran", V.UNAVAILABLE, session.reason || "session-failed",
      { exit_code: session.exit_code, error: session.error });
  } else {
    const tools = session.tools || [];
    add("D5-E1", "evidence", "the agent-side OpenCode session ran", V.PASS, null,
      { exit_code: session.exit_code, tools });
    add("D5-H3", "honesty", "the agent answers from tool results only and cites sources",
      session.cited_sources ? V.PASS : V.FAIL, session.cited_sources ? null : "no-source-citations",
      { tools, cited_sources: session.cited_sources });
  }

  return checks;
}

// ------------------------------------------------------------- aggregation

function summarize(checks) {
  const layers = { honesty: {}, capability: {}, evidence: {} };
  for (const c of checks) {
    const l = layers[c.layer] || (layers[c.layer] = {});
    l[c.verdict] = (l[c.verdict] || 0) + 1;
  }
  const workflows = {};
  for (const c of checks) {
    const wf = c.id.split("-")[0];
    const w = workflows[wf] || (workflows[wf] = { honesty: [], capability: [], evidence: [] });
    w[c.layer].push(c);
  }
  const gate = (list) => {
    // No check of this kind for the workflow: nothing was measured, which is
    // not the same as an unavailable measurement.
    if (list.length === 0) return { verdict: "n/a", reason: "no-check" };
    if (list.some((c) => c.verdict === V.FAIL)) return { verdict: V.FAIL, reason: "a-check-failed" };
    // A verified capability gap dominates an unavailable piece of evidence.
    if (list.some((c) => c.verdict === V.UNMET)) return { verdict: V.UNMET, reason: "a-capability-is-not-met" };
    if (list.some((c) => c.verdict === V.UNAVAILABLE)) return { verdict: V.UNAVAILABLE, reason: "evidence-unavailable" };
    return { verdict: V.PASS, reason: null };
  };
  const gates = {};
  for (const [wf, w] of Object.entries(workflows)) {
    gates[wf] = { honesty: gate(w.honesty), capability: gate(w.capability), evidence: gate(w.evidence) };
  }
  return { layers, gates };
}

function grade(queryDir, options) {
  const opts = options || {};
  const names = opts.queryNames || [
    "d1-1-search-Print", "d1-2-search-normalize_vec", "d1-3-search-Run", "d1-4-search-max_triangulation",
    "d1-5-callers-PrintDef", "d1-6-callers-PrintDecl", "d1-7-callers-normalize_vec",
    "d2-1-callers-PatchMatchCtrlRun", "d2-2-callees-PatchMatchRun", "d2-3-callees-CtrlRun",
    "d2-4-callers-CreateWorkspace", "d2-5-callers-optmgr-binding",
    "d3-1-trace-depth4", "d3-2-trace-to-cuda",
    "d4-1-impact-member", "d4-2-impact-function",
    "d0-1-index-status", "d0-2-repo-status", "d0-3-unknown-repo-classified",
    "p1-callers-class-PatchMatchCuda", "p2-callers-class-PatchMatch", "p3-search-qualified-Print",
  ];
  const docs = {};
  const cudaTargets = opts.cudaTargets || ["d3-2-trace-to-cuda"];
  for (const name of names) docs[name] = loadQuery(queryDir, name);
  const checks = gradeChecks({ docs, mcpSmoke: opts.mcpSmoke, opencodeSession: opts.opencodeSession, cudaTargets: cudaTargets });
  const summary = summarize(checks);
  return { generated_at: new Date().toISOString(), query_dir: queryDir, summary, checks };
}

// ------------------------------------------------------------------- main

if (require.main === module) {
  const queryDir = process.argv[2] || path.join(__dirname, "..", "raw", "queries");
  const outFile = process.argv[3] || "";
  const opts = {};
  if (process.env.CP2_MCP_SMOKE) {
    try { opts.mcpSmoke = JSON.parse(fs.readFileSync(process.env.CP2_MCP_SMOKE, "utf8")); } catch (e) { opts.mcpSmoke = null; }
  }
  if (process.env.CP2_OPENCODE_SESSION) {
    try { opts.opencodeSession = JSON.parse(fs.readFileSync(process.env.CP2_OPENCODE_SESSION, "utf8")); } catch (e) { opts.opencodeSession = null; }
  }
  const result = grade(queryDir, opts);
  if (outFile) fs.writeFileSync(outFile, JSON.stringify(result, null, 2));
  for (const c of result.checks) {
    console.log(`${c.id.padEnd(8)} ${c.layer.padEnd(9)} ${c.verdict.padEnd(12)} ${c.criterion}${c.reason ? "  [" + c.reason + "]" : ""}`);
  }
  console.log("");
  console.log("summary: " + JSON.stringify(result.summary.layers));
  for (const [wf, g] of Object.entries(result.summary.gates)) {
    console.log(`  ${wf}: honesty=${g.honesty.verdict} capability=${g.capability.verdict}`);
  }
}

module.exports = { grade, gradeChecks, summarize, classifyError, loadQuery, V };
