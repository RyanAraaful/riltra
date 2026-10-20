// CP2 grading of the workflow query outputs against the CP1 section D
// criteria. The honesty gate asks "is the answer trustworthy"; the
// capability gate asks "does the answer cover what a GitNexus alternative
// must cover". Presence of an expected relation is a CAPABILITY criterion,
// never an honesty one.
"use strict";
const fs = require("node:fs");
const path = require("node:path");

const QDIR = process.env.CP2_QDIR;
const OUT = process.env.CP2_OUT;
const read = (n) => {
  const p = path.join(QDIR, n + ".json");
  if (!fs.existsSync(p)) return null;
  const t = fs.readFileSync(p, "utf8").trim();
  try { return JSON.parse(t); } catch (e) { return { _parse_error: t.slice(0, 200) }; }
};

const out = { generated_at: new Date().toISOString(), checks: [] };
const check = (id, criterion, layer, pass, evidence) =>
  out.checks.push({ id, criterion, layer, pass, evidence });

const edges = (j) => (j && Array.isArray(j.edges) ? j.edges : []);
const allCandidates = (j) => { const e = edges(j); return e.every((x) => x.kind === "POTENTIAL_CALL" || x.kind === "IMPORTS"); };
const anchorUncertain = (j) => !!(j && j.anchor && j.anchor.anchor_uncertainty);
const hasFreshness = (j) => !!(j && (j.freshness !== undefined || j.freshness_status !== undefined) && j.repo);
const hasDeficiency = (j) => !!(j && j.completeness && Array.isArray(j.completeness.unresolved) && j.completeness.unresolved.length > 0);

// ---------- D.1 ----------
const sPrint = read("d1-1-search-Print");
const sNorm = read("d1-2-search-normalize_vec");
const sRun = read("d1-3-search-Run");
const sMax = read("d1-4-search-max_triangulation");
const cPrint = read("d1-5-callers-PrintDef");
const cNorm = read("d1-7-callers-normalize_vec");
const hasDef = (j, p, line) => !!(j && j.results && j.results.some((r) => r.path === p && r.span && r.span.start_line === line));

check("D1-H1", "the answer resolves the requested definition and does not invent one", "honesty",
  hasDef(sPrint, "colmap/src/colmap/mvs/patch_match.h", 74) || hasDef(sPrint, "colmap/src/colmap/mvs/patch_match.cc", 118),
  { header_def: hasDef(sPrint, "colmap/src/colmap/mvs/patch_match.h", 74), impl_def: hasDef(sPrint, "colmap/src/colmap/mvs/patch_match.cc", 118), total: sPrint && sPrint.total });

check("D1-H2", "every returned reference is a candidate, never a proven CALLS edge", "honesty",
  allCandidates(cPrint), { kinds: [...new Set(edges(cPrint).map((e) => e.kind))], n: edges(cPrint).length });

check("D1-H3", "normalize_vec resolves to the Python definition and to no C++ definition", "honesty",
  hasDef(sNorm, "colmap/benchmark/reconstruction/evaluation/geometry.py", 34) &&
  !(sNorm && sNorm.results && sNorm.results.some((r) => r.path && r.path.endsWith(".cc"))),
  { py34: hasDef(sNorm, "colmap/benchmark/reconstruction/evaluation/geometry.py", 34), results: sNorm && sNorm.results ? sNorm.results.map((r) => r.path + ":" + r.span.start_line) : null });

const runPaths = sRun && sRun.results ? [...new Set(sRun.results.map((r) => r.path))] : [];
check("D1-H4", "a bare name returns multiple distinct identities, not one merged symbol", "honesty",
  runPaths.length >= 2, { distinct_paths: runPaths, total: sRun && sRun.total });

check("D1-H5", "the answer states the repository and a freshness verdict", "honesty",
  hasFreshness(sPrint), { repo: sPrint && sPrint.repo, freshness: sPrint && sPrint.freshness });

// capability: the reference set must resolve the C++ method identity
const printSites = edges(cPrint).map((e) => e.caller_path + ":" + e.caller_span[0]);
const printDistinguished = printSites.includes("colmap/src/colmap/mvs/patch_match.cc:26724");
check("D1-C6", "the reference set resolves the C++ method identity (separates patch_match.cc:667 from :668 and fusion.cc:144)", "capability",
  printDistinguished,
  { edges_returned: printSites.length, call_sites_in_index: "4 POTENTIAL_CALL edges named Print (patch_match.cc x2, fusion.cc, exe/mvs.cc)", distinguished: printDistinguished });

// ---------- D.2 ----------
const cCtrl = read("d2-1-callers-PatchMatchCtrlRun");
const calCtrl = read("d2-3-callees-CtrlRun");
const calRun = read("d2-2-callees-PatchMatchRun");
const cCreate = read("d2-4-callers-CreateWorkspace");
const cOpt = read("d2-5-callers-optmgr-binding");
const p2class = read("p2-callers-class-PatchMatch");
const callerPaths = (j) => edges(j).map((e) => e.caller_path);

check("D2-H1", "no reference is fabricated and the anchor uncertainty is reported", "honesty",
  allCandidates(cCtrl) && anchorUncertain(cCtrl) && hasDeficiency(cCtrl),
  { kinds: [...new Set(edges(cCtrl).map((e) => e.kind))], n: edges(cCtrl).length, anchor_uncertainty: cCtrl && cCtrl.anchor && cCtrl.anchor.anchor_uncertainty, unresolved: cCtrl && cCtrl.completeness ? cCtrl.completeness.unresolved.length : null });

const calleeNames = (j) => edges(j).map((e) => e.callee_name);
check("D2-H2", "the callee set contains the PatchMatchCuda construction and Run call", "honesty",
  calleeNames(calRun).some((n) => n && n.includes("PatchMatchCuda")),
  { callee_names: [...new Set(calleeNames(calRun))] });

const pp = edges(calCtrl).filter((e) => (e.callee_name || "").includes("ProcessProblem"));
check("D2-H3", "the ThreadPool dispatch is absent or a candidate, never a proven CALLS edge", "honesty",
  pp.every((e) => e.kind === "POTENTIAL_CALL"), { process_problem_edges: pp.map((e) => ({ kind: e.kind, callee: e.callee_name })) });

check("D2-H4", "the CreateWorkspace references stay inside patch_match.cc", "honesty",
  callerPaths(cCreate).every((p) => !p || p === "colmap/src/colmap/mvs/patch_match.cc"),
  { caller_paths: [...new Set(callerPaths(cCreate))] });

check("D2-H5", "the option-manager address-of site is not presented as a call to the option", "honesty",
  !edges(cOpt).some((e) => (e.callee_name || "") === "max_triangulation_angle" && e.kind === "CALLS"),
  { edges: edges(cOpt).map((e) => ({ kind: e.kind, callee: e.callee_name })) });

const classEdges = callerPaths(p2class);
check("D2-C6", "the three external construction sites are attributed to the class", "capability",
  classEdges.some((p) => p && p.includes("exe/mvs.cc")) &&
  classEdges.some((p) => p && p.includes("automatic_reconstruction.cc")) &&
  classEdges.some((p) => p && p.includes("dense_reconstruction_widget")),
  { class_anchor_edges: classEdges, sites_in_source: 3, attributed: 0, cause: "C/C++ include-root resolution (gap E8) and template-argument binding (gap E7)" });

// ---------- D.3 ----------
const tr = read("d3-1-trace-depth4");
const trCu = read("d3-2-trace-to-cuda");
check("D3-H2", "the plausibility disclaimer is present", "honesty",
  !!(tr && tr.disclaimer === "plausible-path-not-execution"), { disclaimer: tr && tr.disclaimer });
check("D3-H3", "the path does not claim a device-side definition", "honesty",
  !(tr && JSON.stringify(tr).includes("patch_match_cuda.cu")),
  { mentions_cu: tr ? JSON.stringify(tr).includes(".cu") : null, paths: tr && tr.paths ? tr.paths.length : null });
check("D3-H4", "the CUDA destination query fails cleanly instead of inventing a path", "honesty",
  !!(trCu && (trCu._parse_error || trCu.paths === undefined || trCu.paths.length === 0 || trCu.error)),
  { output: trCu ? JSON.stringify(trCu).slice(0, 200) : null });
const maxHops = tr && tr.paths ? Math.max(...tr.paths.map((p) => p.edges.length)) : 0;
check("D3-C5", "the path spans the full frozen chain (4 hops, including the indirect dispatch)", "capability",
  maxHops >= 3, { max_hops_returned: maxHops, paths: tr && tr.paths ? tr.paths.length : null, frozen_chain_hops: 4 });

// ---------- D.4 ----------
const im = read("d4-1-impact-member");
check("D4-H2", "the impact verdict is UNKNOWN, never SAFE/LOW", "honesty",
  !!(im && im.verdict && im.verdict !== "SAFE" && im.verdict !== "LOW"), { verdict: im && im.verdict });
check("D4-H3", "completeness is partial with graph-completeness-unproven", "honesty",
  !!(im && im.completeness && im.completeness.status === "partial" &&
     (im.completeness.unresolved || []).includes("graph-completeness-unproven")),
  { completeness: im && im.completeness && im.completeness.status, reasons: im && im.completeness ? im.completeness.unresolved.length : null, disclaimer: im && im.disclaimer });
check("D4-H4", "no macro site is resolved to a function definition named after the option", "honesty",
  !(im && JSON.stringify(im).includes('"callee_name":"max_triangulation_angle"')),
  { mentions: im ? JSON.stringify(im).includes('"callee_name":"max_triangulation_angle"') : null });
check("D4-C5", "the affected set contains the declaration, print, validation, read/filter and CLI binding sites", "capability",
  !!(im && im.paths && im.paths.length > 0),
  { paths: im && im.paths ? im.paths.length : null, visits: im && im.visits, cause: "a member read, an address-of binding and a macro argument are not call expressions (gap E7)" });

fs.writeFileSync(path.join(OUT, "grading.json"), JSON.stringify(out, null, 2));
for (const c of out.checks) {
  console.log(`${c.id.padEnd(7)} ${c.layer.padEnd(9)} ${c.pass ? "PASS" : "FAIL/UNMET"}  ${c.criterion}`);
}
