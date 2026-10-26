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
// The line-based capability checks read the source the queries ran on. The
// search output names the snapshot root, and a span (a byte offset in Riltra's
// output) is mapped to its line there. A capability PASS needs a proven
// relation (CALLS or REFERENCES, Exact) at the frozen site: a name candidate
// never passes one. The one candidate a frozen criterion allows is the
// indirect thread-pool dispatch (B.2, B.3), which is never CALLS.
//
// Usage: node grade.cjs <query-dir> [out-file]
// CP2_SOURCE_ROOT=<snapshot> overrides the source root (for a re-grade of an
// archived output, whose search root is redacted).
"use strict";

const fs = require("node:fs");
const path = require("node:path");

const V = { PASS: "PASS", FAIL: "FAIL", UNMET: "UNMET", UNAVAILABLE: "UNAVAILABLE" };

// The approved MCP surface: twelve tools, the twelfth being the semantic
// tool (riltra_code_semantic). The count alone is not the contract: the
// catalog must be exactly this set, so a catalog of the right size that
// lacks an approved tool (or repeats one) is a different surface and must
// be rejected.
const MCP_TOOL_COUNT = 12;
const MCP_REQUIRED_TOOLS = [
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

// ---------------------------------------------------------------- evidence

function classifyError(raw) {
  const t = String(raw == null ? "" : raw);
  const kind = /"kind"\s*:\s*"([a-z_]+)"/.exec(t);
  if (kind) return "domain-error:" + kind[1];
  if (/anchor unresolved: source-changed/.test(t)) return "source-changed";
  if (/anchor unresolved: cuda-experimental/.test(t)) return "cuda-experimental";
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

// A relation the tool proved. This mirrors the rule the impact traversal applies
// before it follows an edge (crates/cli/src/edges.rs, traverse_impact): a CALLS
// or REFERENCES edge with Exact precision and no unresolved reason.
function isProven(edge) {
  return !!edge && (edge.kind === "CALLS" || edge.kind === "REFERENCES") &&
    edge.precision === "Exact" && edge.unresolved == null;
}

// The CUDA sources Riltra refuses to parse (.cu and .cuh, see syntax.rs classify).
const CUDA_SOURCE = /\.cuh?$/i;

// Endpoints that place a definition in a CUDA source. The frozen spec (B.3) has
// the path end at the CUDA boundary with that terminus labelled unsupported, so
// an endpoint typed "unsupported" is the label, not a claim. Text fields are
// not endpoints: a .cu name there is a mention, never a device-side claim.
function cudaDefinitionClaims(doc) {
  return pathsOf(doc)
    .flatMap((p) => (Array.isArray(p.edges) ? p.edges : []))
    .flatMap((e) => (e ? [e.caller, e.callee] : []))
    .filter((ep) => ep && ep.type !== "unsupported" && CUDA_SOURCE.test(String(ep.path || "")));
}

// Two endpoints name one definition: the same SCIP symbol, or the same
// syntactic definition span in the same file.
function sameDefinition(a, b) {
  if (!a || !b || a.type !== b.type) return false;
  if (a.symbol_uri || b.symbol_uri) return a.symbol_uri === b.symbol_uri;
  return a.path === b.path && a.byte_start === b.byte_start;
}

// The frozen MMSP files that the line-based checks name.
const FILES = {
  patchMatchCc: "colmap/src/colmap/mvs/patch_match.cc",
  patchMatchH: "colmap/src/colmap/mvs/patch_match.h",
  patchMatchCuda: "colmap/src/colmap/mvs/patch_match_cuda.cu",
  patchMatchCudaH: "colmap/src/colmap/mvs/patch_match_cuda.h",
  fusionCc: "colmap/src/colmap/mvs/fusion.cc",
  optionsH: "colmap/src/colmap/mvs/patch_match_options.h",
  optionsCc: "colmap/src/colmap/mvs/patch_match_options.cc",
  optionManagerCc: "colmap/src/colmap/controllers/option_manager.cc",
  mvsCc: "colmap/src/colmap/exe/mvs.cc",
  autoCc: "colmap/src/colmap/controllers/automatic_reconstruction.cc",
  denseCc: "colmap/src/colmap/ui/dense_reconstruction_widget.cc",
};

// ------------------------------------------------------- source grounding
//
// Riltra reports spans as byte offsets from the start of a file, and the frozen
// criteria name lines. A check that needs a line reads the file from the
// snapshot that the search queries ran against (their `root`). Without that
// file the check is UNAVAILABLE: a byte offset is never compared with a line.

// The snapshot root without the Windows extended-length prefix, or null.
function snapshotRootOf(ctx) {
  const s = ctx.docs["d1-1-search-Print"];
  const root = s && s.available && s.doc && typeof s.doc.root === "string" ? s.doc.root : null;
  return root ? root.replace(/^\\\\\?\\/, "") : null;
}

const sourceCache = new Map();
function sourceBytes(root, rel) {
  const key = root + "\n" + rel;
  if (!sourceCache.has(key)) {
    const file = path.join(root, rel);
    sourceCache.set(key, fs.existsSync(file) ? fs.readFileSync(file) : null);
  }
  return sourceCache.get(key);
}

// The 1-based line that holds a byte offset, with that line's text.
function lineAt(root, rel, byteOffset) {
  const bytes = root && typeof rel === "string" && Number.isInteger(byteOffset) ? sourceBytes(root, rel) : null;
  if (!bytes || byteOffset < 0 || byteOffset > bytes.length) return null;
  let line = 1;
  let start = 0;
  for (let i = 0; i < byteOffset; i++) {
    if (bytes[i] === 0x0a) {
      line += 1;
      start = i + 1;
    }
  }
  let end = bytes.indexOf(0x0a, start);
  if (end < 0) end = bytes.length;
  return { line, text: bytes.subarray(start, end).toString("utf8") };
}

// A reason that the tool actually reported. "missing-output" and
// "unclassified-error" mean nothing was observed, so a check that needs an
// observation must not pass on them.
function isObservedError(reason) {
  if (!reason) return false;
  return reason === "source-changed" || reason === "cli-error" ||
    reason === "unsupported" || reason === "cuda-unsupported" ||
    reason === "cuda-experimental" || reason.startsWith("domain-error:");
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

  // Source grounding for the line-based checks (see lineAt above). A test may
  // set ctx.sourceRoot; a run takes the root from the search output.
  const root = ctx.sourceRoot !== undefined ? ctx.sourceRoot : snapshotRootOf(ctx);
  const at = (rel, byteOffset) => lineAt(root, rel, byteOffset);
  // Where an edge's caller sits: its file, line and line text. Null without source.
  const siteOf = (edge) => {
    const where = edge && Array.isArray(edge.caller_span) ? at(edge.caller_path, edge.caller_span[0]) : null;
    return where ? Object.assign({ path: edge.caller_path }, where) : null;
  };
  const rowsOf = (doc) => edgesOf(doc).map((edge) => ({ edge, site: siteOf(edge) }));
  const unsourced = (rows) => rows.some((r) => !r.site);
  // Whether a definition endpoint starts on the given line of the given file.
  const definedAt = (endpoint, rel, line) => {
    const where = endpoint && endpoint.path === rel ? at(rel, endpoint.byte_start) : null;
    return !!where && where.line === line;
  };
  // The class, or its own constructor: patch_match.h:131 declares it and
  // patch_match.cc:219 defines it (checked at the frozen commit). A construction
  // site calls the constructor, so a proven edge to it attributes the site to
  // the class.
  const classOwns = (callee, classDef) => sameDefinition(callee, classDef) ||
    definedAt(callee, FILES.patchMatchCc, 219) || definedAt(callee, FILES.patchMatchH, 131);

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

  // honesty (A-3, amendment §8): a candidate keeps the frozen labels, names no
  // callee, and sits in a reference set that declares itself partial. The label
  // and callee rules are frozen B.1 and B.9; the partial rule is frozen B.8.
  const candidateRows = allEdges.filter((e) => e.kind === "POTENTIAL_CALL");
  const d1h6 = "a candidate keeps the POTENTIAL_CALL, Heuristic and Unverified labels, names no callee, and sits in a reference set that declares itself partial";
  if (candidateRows.length === 0) {
    add("D1-H6", "honesty", d1h6, V.UNAVAILABLE, "no-candidate-returned", { inspected_candidates: 0 });
  } else {
    const mislabelled = candidateRows.filter((e) => e.precision !== "Heuristic" || e.verification !== "Unverified");
    const named = candidateRows.filter((e) => e.callee != null);
    const undeclared = referenceQueries.map(([name]) => name).filter((name) => {
      const q = ctx.docs[name];
      if (!q.available || !edgesOf(q.doc).some((e) => e.kind === "POTENTIAL_CALL")) return false;
      return !(q.doc.completeness && q.doc.completeness.status === "partial");
    });
    const reason = mislabelled.length ? "candidate-mislabelled"
      : named.length ? "candidate-names-a-callee"
      : undeclared.length ? "candidate-set-not-declared-partial" : null;
    add("D1-H6", "honesty", d1h6, reason ? V.FAIL : V.PASS, reason, {
      inspected_candidates: candidateRows.length,
      mislabelled: mislabelled.length,
      named_callee: named.length,
      undeclared_queries: undeclared,
    });
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

  // capability: the reference set resolves the identity of the true reference.
  // Frozen D.1 #6: the answer separates patch_match.cc:667 from :668 and
  // fusion.cc:144. The reference at :667 must be a proven edge to the anchored
  // definition, and :668 or :144 must not be a proven edge to it. A candidate
  // name match at :667 is not identity.
  const cPrint = ctx.docs["d1-5-callers-PrintDef"];
  const d1Criterion = "the reference set resolves the C++ method identity (separates patch_match.cc:667 from :668 and fusion.cc:144)";
  if (!cPrint.available) {
    add("D1-C6", "capability", d1Criterion, V.UNAVAILABLE, cPrint.reason, {});
  } else {
    const anchor = cPrint.doc.anchor ? cPrint.doc.anchor.endpoint : null;
    const rows = rowsOf(cPrint.doc);
    const onLine = (rel, line) => rows.filter((r) => r.site && r.site.path === rel && r.site.line === line);
    const trueRef = onLine(FILES.patchMatchCc, 667).filter((r) => r.site.text.includes("Print"));
    const otherRef = [...onLine(FILES.patchMatchCc, 668), ...onLine(FILES.fusionCc, 144)];
    const proven = (r) => isProven(r.edge) && sameDefinition(r.edge.callee, anchor);
    const evidence = {
      edges_returned: rows.length,
      sites: rows.map((r) => (r.site ? r.site.path + ":" + r.site.line : null)),
      proven_at_667: trueRef.some(proven),
      proven_elsewhere: otherRef.some(proven),
    };
    if (unsourced(rows)) {
      add("D1-C6", "capability", d1Criterion, V.UNAVAILABLE, "source-unavailable", evidence);
    } else if (otherRef.some(proven)) {
      add("D1-C6", "capability", d1Criterion, V.FAIL, "other-site-proven-to-definition", evidence);
    } else if (trueRef.some(proven)) {
      add("D1-C6", "capability", d1Criterion, V.PASS, null, evidence);
    } else {
      add("D1-C6", "capability", d1Criterion, V.UNMET,
        trueRef.length ? "identity-not-proven-at-667" : "site-667-not-returned", evidence);
    }
  }

  // ---- D.2 callers and callees -------------------------------------------
  const cCtrl = ctx.docs["d2-1-callers-PatchMatchCtrlRun"];
  if (!cCtrl.available) {
    add("D2-H1", "honesty", "the answer states what it could not resolve and labels every reported edge as a candidate",
      V.UNAVAILABLE, cCtrl.reason, {});
  } else {
    const d = cCtrl.doc;
    const uncertainty = !!(d.anchor && d.anchor.anchor_uncertainty);
    const reasons = d.completeness && Array.isArray(d.completeness.unresolved) ? d.completeness.unresolved : [];
    const kindsHere = [...new Set(edgesOf(d).map((e) => e.kind))];
    // The label of the reported edges is checked by D1-H2 over every query
    // that returned edges; here the observable is the disclosure.
    const honest = uncertainty && reasons.length > 0;
    add("D2-H1", "honesty", "the answer states what it could not resolve and labels every reported edge as a candidate",
      honest ? V.PASS : V.FAIL, honest ? null : "uncertainty-or-reasons-missing",
      { edges: edgesOf(d).length, kinds: kindsHere, anchor_uncertainty: uncertainty, unresolved_reasons: reasons.length,
        note: "the edge labels are checked by D1-H2; the presence of the construction sites is the capability gate D2-C6" });
  }

  const calRun = ctx.docs["d2-2-callees-PatchMatchRun"];
  if (!calRun.available) {
    add("D2-H2", "honesty", "the callee set contains the PatchMatchCuda construction and Run call",
      V.UNAVAILABLE, calRun.reason, {});
  } else {
    const names = edgesOf(calRun.doc).map((e) => e.callee_name);
    const cuda = names.filter((n) => n && n.includes("PatchMatchCuda"));
    const runCall = names.filter((n) => n === "Run");
    const ok = cuda.length > 0 && runCall.length > 0;
    add("D2-H2", "honesty", "the callee set contains the PatchMatchCuda construction and Run call",
      ok ? V.PASS : V.FAIL, ok ? null : (cuda.length ? "the Run call is missing" : "expected-callee-missing"),
      { edges: names.length, callee_names: [...new Set(names)], patch_match_cuda: cuda, run_call: runCall });
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
    add("D2-H5", "honesty", "every reported edge carries a resolvable location and is labeled as a candidate",
      V.UNAVAILABLE, cOpt.reason, {});
  } else {
    const edges = edgesOf(cOpt.doc);
    const wrong = edges.filter((e) => (e.callee_name || "") === "max_triangulation_angle");
    if (edges.length === 0) {
      add("D2-H5", "honesty", "every reported edge carries a resolvable location and is labeled as a candidate",
        V.UNAVAILABLE, "no-reference-returned", { edges: 0, note: "the query reported no edge, so no reported edge can be inspected" });
    } else {
      add("D2-H5", "honesty", "every reported edge carries a resolvable location and is labeled as a candidate",
        wrong.length === 0 && edges.every(isCandidate) ? V.PASS : V.FAIL,
        wrong.length ? "option-presented-as-call" : (edges.every(isCandidate) ? null : "an-edge-is-not-a-candidate"),
        { edges: edges.length, edges_summary: edges.map((e) => ({ kind: e.kind, callee: e.callee_name })) });
    }
  }

  // capability: the three external construction sites are attributed to the
  // class by a proven edge. Frozen D.2 #5 requires the callers to be identity-
  // resolved, so a candidate at the right line is not attribution.
  const p2 = ctx.docs["p2-callers-class-PatchMatch"];
  const d2Criterion = "the three external construction sites are attributed to the class";
  if (!p2.available) {
    add("D2-C6", "capability", d2Criterion, V.UNAVAILABLE, p2.reason, {});
  } else {
    const classDef = p2.doc.anchor ? p2.doc.anchor.endpoint : null;
    const rows = rowsOf(p2.doc);
    // The construction lines of frozen B.2. The factory site is the argument of
    // make_shared / make_unique on line 398.
    const expected = [
      { site: "exe/mvs.cc:271", path: FILES.mvsCc, lines: [271, 271] },
      { site: "automatic_reconstruction.cc:431", path: FILES.autoCc, lines: [431, 431] },
      { site: "dense_reconstruction_widget.cc:397-398", path: FILES.denseCc, lines: [397, 398] },
    ];
    const sites = expected.map((s) => {
      const hits = rows.filter((r) => r.site && r.site.path === s.path &&
        r.site.line >= s.lines[0] && r.site.line <= s.lines[1] && r.site.text.includes("PatchMatchController"));
      return {
        site: s.site,
        present: hits.length > 0,
        proven: hits.some((r) => isProven(r.edge) && classOwns(r.edge.callee, classDef)),
      };
    });
    const evidence = { sites, sites_expected: 3, class_callers_returned: rows.length };
    if (unsourced(rows)) {
      add("D2-C6", "capability", d2Criterion, V.UNAVAILABLE, "source-unavailable", evidence);
    } else if (sites.some((s) => !s.present)) {
      add("D2-C6", "capability", d2Criterion, V.UNMET, "construction-sites-not-attributed", evidence);
    } else if (sites.some((s) => !s.proven)) {
      add("D2-C6", "capability", d2Criterion, V.UNMET, "attribution-not-proven-to-class", evidence);
    } else {
      add("D2-C6", "capability", d2Criterion, V.PASS, null, evidence);
    }
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
    const claims = cudaDefinitionClaims(tr.doc).length;
    add("D3-H3", "honesty", "the path does not claim a device-side definition",
      claims ? V.FAIL : V.PASS, claims ? "device-side-definition-claimed" : null,
      { cuda_definition_claims: claims, paths: pathsOf(tr.doc).length });
    const paths = pathsOf(tr.doc);
    const maxHops = paths.reduce((m, p) => Math.max(m, Array.isArray(p.edges) ? p.edges.length : 0), 0);
    const d3Criterion = "the path spans the full frozen chain (4 nodes, 3 edges, including the indirect dispatch)";
    const evidence = { max_hops_returned: maxHops, paths: paths.length, frozen_chain_nodes: 4, frozen_chain_edges: 3 };
    if (maxHops < 3) {
      add("D3-C5", "capability", d3Criterion, V.UNMET, "trace-does-not-extend-beyond-the-first-hop", evidence);
    } else {
      // Frozen B.3, hop by hop. Hop 1 is the indirect ThreadPool dispatch of
      // B.2: its address-of reference is a candidate at most, never CALLS. Hops
      // 2 and 3 are direct calls, so each must be a proven edge that calls from
      // the cited line and whose callee is the cited definition.
      const calledFrom = (e, rel, lines, token) => {
        const s = siteOf(e);
        return !!s && s.path === rel && lines.includes(s.line) && s.text.includes(token);
      };
      const hop1 = (e) => e.callee_name === "ProcessProblem" && e.kind === "POTENTIAL_CALL" &&
        calledFrom(e, FILES.patchMatchCc, [268, 292, 309], "ProcessProblem");
      const hop2 = (e) => isProven(e) && calledFrom(e, FILES.patchMatchCc, [671], "Run") &&
        definedAt(e.callee, FILES.patchMatchCc, 191);
      const hop3 = (e) => isProven(e) && calledFrom(e, FILES.patchMatchCc, [197], "Run") &&
        (definedAt(e.callee, FILES.patchMatchCuda, 1326) || definedAt(e.callee, FILES.patchMatchCudaH, 54));
      const rows = paths.flatMap((p) => (Array.isArray(p.edges) ? p.edges : []))
        .map((edge) => ({ edge, site: siteOf(edge) }));
      const spanned = paths.some((p) => Array.isArray(p.edges) && p.edges.length >= 3 &&
        hop1(p.edges[0]) && hop2(p.edges[1]) && hop3(p.edges[2]));
      evidence.chain_matched = spanned;
      if (unsourced(rows)) {
        add("D3-C5", "capability", d3Criterion, V.UNAVAILABLE, "source-unavailable", evidence);
      } else {
        add("D3-C5", "capability", d3Criterion, spanned ? V.PASS : V.UNMET,
          spanned ? null : "path-not-frozen-chain", evidence);
      }
    }
  }

  // The CUDA terminus: the tool must fail cleanly, and the reason must be
  // the real one. Riltra does not index CUDA sources, so the anchor can
  // never resolve. Current tool output names cuda-experimental; the archived
  // Task 6 output said source-changed, and that is recorded next to the
  // actual cause. The queries whose target is a CUDA file are listed in
  // ctx.cudaTargets.
  const trCu = ctx.docs["d3-2-trace-to-cuda"];
  const cudaTargets = ctx.cudaTargets || [];
  const isCudaTarget = cudaTargets.indexOf("d3-2-trace-to-cuda") >= 0;
  if (trCu.available) {
    add("D3-H4", "honesty", "the CUDA destination query fails cleanly instead of inventing a path",
      V.FAIL, "no-error-raised", { note: "the query answered instead of failing" });
  } else if (!isObservedError(trCu.reason)) {
    // Nothing was observed: a missing output or an unclassifiable error is
    // not a clean failure, it is absent evidence.
    add("D3-H4", "honesty", "the CUDA destination query fails cleanly instead of inventing a path",
      V.UNAVAILABLE, trCu.reason, { observed_reason: trCu.reason, note: "no classified failure was observed" });
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
    add("D4-H4", "honesty", "the two macro sites are not resolved to function definitions and no edge is claimed for the member read, the address-of binding or the macro arguments",
      wrong ? V.FAIL : V.PASS, wrong ? "option-resolved-as-definition" : null,
      { mentions_option_as_callee: wrong, paths: pathsOf(d).length, bridges: Array.isArray(d.bridges) ? d.bridges.length : 0,
        note: "the whole answer is inspected; the empty path set is the pre-registered pilot shape (CP1 D.4.5) and the answer states that it is not coverage proof" });
    const paths = pathsOf(d);
    const d4Criterion = "the affected set contains the declaration, print, validation, read/filter and CLI binding sites";
    // Frozen D.4 #5. The declaration is the anchor and is covered by its own
    // source line. Each other site needs a proven edge that sits on its frozen
    // lines and names the option, so a name match or an unrelated path cannot
    // pass.
    const TOKEN = "max_triangulation_angle";
    const frozenSites = [
      { site: "print", path: FILES.optionsCc, lines: [58, 58] },
      { site: "validation", path: FILES.optionsCc, lines: [91, 93] },
      { site: "read", path: FILES.patchMatchCc, lines: [361, 366] },
      { site: "filter", path: FILES.patchMatchCc, lines: [443, 447] },
      { site: "cli-binding", path: FILES.optionManagerCc, lines: [1018, 1019] },
    ];
    const rows = paths.flatMap((p) => (Array.isArray(p.edges) ? p.edges : []))
      .map((edge) => ({ edge, site: siteOf(edge) }));
    const anchorSite = d.anchor && d.anchor.endpoint ? at(d.anchor.path, d.anchor.endpoint.byte_start) : null;
    const declared = !!anchorSite && d.anchor.path === FILES.optionsH &&
      anchorSite.line === 56 && anchorSite.text.includes(TOKEN);
    const covered = frozenSites.map((s) => ({
      site: s.site,
      covered: rows.some((r) => r.site && r.site.path === s.path &&
        r.site.line >= s.lines[0] && r.site.line <= s.lines[1] &&
        r.site.text.includes(TOKEN) && isProven(r.edge)),
    }));
    const evidence = {
      paths: paths.length,
      visits: d.visits,
      declaration: declared,
      covered,
      note: "an empty affected set is reported as UNKNOWN, never as proof of safety",
    };
    if (paths.length === 0) {
      add("D4-C5", "capability", d4Criterion, V.UNMET, "affected-set-empty", evidence);
    } else if (unsourced(rows) || (d.anchor && d.anchor.endpoint && !anchorSite)) {
      add("D4-C5", "capability", d4Criterion, V.UNAVAILABLE, "source-unavailable", evidence);
    } else if (declared && covered.every((c) => c.covered)) {
      add("D4-C5", "capability", d4Criterion, V.PASS, null, evidence);
    } else {
      add("D4-C5", "capability", d4Criterion, V.UNMET, "affected-set-incomplete", evidence);
    }
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
    const toolList = Array.isArray(smoke.tool_list) ? smoke.tool_list : [];
    const toolCount = toolList.length;
    const sortedTools = toolList.slice().sort();
    const expectedTools = MCP_REQUIRED_TOOLS.slice().sort();
    const requiredToolsPresent =
      sortedTools.length === expectedTools.length && sortedTools.every((t, i) => t === expectedTools[i]);
    const surfaceOk = toolCount === MCP_TOOL_COUNT && requiredToolsPresent;
    const nonJson = smoke.stdout_pure ? smoke.stdout_pure.non_json : null;
    const stderrBytes = typeof smoke.stderr === "string" ? smoke.stderr.length : null;
    const calls = smoke.calls || [];
    const answered = calls.filter((c) => c.present && !c.is_error && c.text_len > 0);
    const grounded = answered.some((c) => (c.text || "").includes('"repo"'));
    const ok = surfaceOk && nonJson === 0 && stderrBytes === 0 && answered.length > 0 && grounded;
    const reason = !surfaceOk ? "mcp-tool-surface-mismatch" : (ok ? null : "mcp-surface-did-not-answer-cleanly");
    add("D5-H1", "honesty", "the MCP surface answers a source-grounded query with the repository identity",
      ok ? V.PASS : V.FAIL, reason,
      { tool_count: toolCount, required_tools: MCP_REQUIRED_TOOLS, required_tools_present: requiredToolsPresent,
        tools: smoke.tool_list, stdout_non_json: nonJson, stderr_bytes: stderrBytes, answered_calls: answered.length });
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
    if (tools.length === 0) {
      // The session exited cleanly but never reached a tool: it answered
      // nothing, so there is no agent-side evidence to grade.
      add("D5-E1", "evidence", "the agent-side OpenCode session ran", V.UNAVAILABLE, "session-reached-no-tool",
        { exit_code: session.exit_code });
    } else {
      add("D5-E1", "evidence", "the agent-side OpenCode session ran", V.PASS, null,
        { exit_code: session.exit_code, tools });
      add("D5-H3", "honesty", "the agent answers from tool results only and cites sources",
        session.cited_sources ? V.PASS : V.FAIL, session.cited_sources ? null : "no-source-citations",
        { tools, cited_sources: session.cited_sources });
    }
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
  const checks = gradeChecks({
    docs, mcpSmoke: opts.mcpSmoke, opencodeSession: opts.opencodeSession, cudaTargets: cudaTargets,
    sourceRoot: opts.sourceRoot,
  });
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
  // An archived output names a redacted snapshot root, so a re-grade names the
  // snapshot here. Without it the source comes from the search output's root.
  if (process.env.CP2_SOURCE_ROOT) opts.sourceRoot = process.env.CP2_SOURCE_ROOT;
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
  // A failed check is a harness finding and must not be reported as a
  // successful grading; an unmet capability is a result, not a failure.
  const failed = result.checks.filter((c) => c.verdict === V.FAIL);
  if (failed.length > 0) {
    console.error("FAILED CHECKS: " + failed.map((c) => c.id).join(", "));
    process.exit(1);
  }
}

module.exports = { grade, gradeChecks, summarize, classifyError, loadQuery, V };
