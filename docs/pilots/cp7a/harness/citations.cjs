// SPDX-License-Identifier: Apache-2.0
// CP7-A citation resolver. Every repo path/line and cp7a record id cited in an
// answer must resolve against the pinned source, fixtures, or known records.
"use strict";
const fs = require("node:fs");
const path = require("node:path");

const os = require("node:os");
const REPO_ROOT = path.resolve(__dirname, "..", "..", "..", "..");
const CP7A_ROOT = path.join(REPO_ROOT, ".cp7a");
const SOURCE_ROOT = path.join(CP7A_ROOT, "source");
const MAIN_ROOT = REPO_ROOT;
const OTHER_ROOT = path.join(CP7A_ROOT, "other-repo");
const EMPTY_ROOT = path.join(CP7A_ROOT, "empty-repo");
const BIN_ROOT = path.join(CP7A_ROOT, "bin");
const ARM_A = path.join(os.tmpdir(), "opencode", "cp7a", "arm-a");
const ARM_B = path.join(os.tmpdir(), "opencode", "cp7a", "arm-b");
const TARGET_DEBUG = path.join(REPO_ROOT, "target", "debug");

const KNOWN_RECORDS = new Set([
  "cp7a-decision-health",
  "cp7a-constraint-readonly",
  "cp7a-constraint-trace",
  "cp7a-other-stale",
  "cp7a-rel-health-readonly",
  "cp7a-rel-health-trace"
]);
const NON_RECORD_CP7A = new Set(["cp7a-bench"]);

const INDEX = new Map();
function addIndex(p) {
  const base = path.basename(p).toLowerCase();
  if (!INDEX.has(base)) INDEX.set(base, []);
  INDEX.get(base).push(p);
}
function walk(root) {
  let entries;
  try { entries = fs.readdirSync(root, { withFileTypes: true }); } catch { return; }
  for (const e of entries) {
    if (e.name === ".git" || e.name === "target" || e.name === "node_modules") continue;
    const p = path.join(root, e.name);
    if (e.isDirectory()) walk(p);
    else if (e.isFile()) addIndex(p);
  }
}
function addRoot(root) {
  try {
    for (const e of fs.readdirSync(root, { withFileTypes: true })) {
      if (e.isFile()) addIndex(path.join(root, e.name));
    }
  } catch {}
}
for (const root of [SOURCE_ROOT, OTHER_ROOT, EMPTY_ROOT, BIN_ROOT, ARM_A, ARM_B]) walk(root);
addRoot(MAIN_ROOT);
for (const name of ["riltra.exe", "riltra.pdb"]) {
  const p = path.join(TARGET_DEBUG, name);
  if (fs.existsSync(p)) addIndex(p);
}

function normalizeAbsolute(p) {
  let s = p.trim();
  s = s.replace(/^\\\\\?\\/, "");
  s = s.replace(/^\/\/\?\/?/, "");
  s = s.replace(/[),;]+$/, "");
  return s;
}

function splitLine(raw) {
  const s = raw.trim();
  const m = /^(.*?)(?::(\d+))(?::(\d+))?$/.exec(s);
  if (m) return { path: m[1], line: Number(m[2]), col: m[3] ? Number(m[3]) : null };
  return { path: s, line: null, col: null };
}

function resolveRelative(p) {
  const roots = [SOURCE_ROOT, OTHER_ROOT, EMPTY_ROOT, BIN_ROOT, ARM_A, ARM_B, MAIN_ROOT];
  for (const root of roots) {
    const cand = path.join(root, p);
    try { if (fs.existsSync(cand)) return { path: cand, reason: null }; } catch {}
  }
  if (!p.includes("/") && !p.includes("\\")) {
    const list = INDEX.get(p.toLowerCase());
    if (list && list.length === 1) return { path: list[0], reason: null };
    if (list && list.length > 1) return { path: list[0], reason: "ambiguous-basename", candidates: list.slice(0, 5) };
  }
  return { path: null, reason: "not-found" };
}

function resolveAbsolute(p) {
  const variants = [p, normalizeAbsolute(p), p.replace(/\\/g, "/"), normalizeAbsolute(p).replace(/\\/g, "/")];
  for (const v of variants) {
    try { if (fs.existsSync(v)) return { path: v, reason: null }; } catch {}
  }
  return { path: null, reason: "not-found" };
}

function citation(raw) {
  const s = splitLine(raw);
  const isAbsolute = /^[A-Za-z]:[\\/]/.test(s.path) || /^\\\\/.test(s.path) || /^\//.test(s.path);
  const r = isAbsolute ? resolveAbsolute(s.path) : resolveRelative(s.path);
  const out = {
    raw,
    path_text: s.path,
    line: s.line,
    col: s.col,
    resolved: Boolean(r.path),
    resolved_path: r.path || null,
    line_exists: null,
    col_exists: null,
    line_text: null,
    ambiguous: r.reason === "ambiguous-basename",
    candidates: r.candidates || null,
    reason: r.reason
  };
  if (r.path) {
    try {
      const st = fs.statSync(r.path);
      if (!st.isFile()) {
        if (s.line !== null) { out.line_exists = false; out.reason = "line-on-directory"; }
      } else if (s.line !== null) {
        const lines = fs.readFileSync(r.path, "utf8").split(/\r?\n/);
        out.line_exists = s.line >= 1 && s.line <= lines.length;
        if (out.line_exists) {
          const text = lines[s.line - 1];
          out.line_text = text.length > 200 ? text.slice(0, 200) + "..." : text;
          if (s.col !== null) out.col_exists = s.col >= 1 && s.col <= text.length + 1;
        } else {
          out.reason = "line-out-of-range";
        }
      }
    } catch (e) {
      out.reason = "unreadable";
    }
  }
  return out;
}

function extractPathTexts(answer) {
  const text = String(answer || "").replace(/https?:\/\/\S+/g, " ");
  const raws = new Set();
  const full = /(?:^|[^\w/\\])((?:crates|tests|fixtures|docs|proto|src|scripts)\/[A-Za-z0-9_.\/-]+(?::\d+(?::\d+)?)?)/g;
  const fixture = /(?:^|[^\w/\\])((?:c|cpp)\/[A-Za-z0-9_.\/-]+(?::\d+)?)/g;
  const abs = /(?<![A-Za-z])([A-Za-z]:[\\/][^\s`"'<>|)\]]+)/g;
  for (const re of [full, fixture, abs]) {
    let m;
    while ((m = re.exec(text)) !== null) raws.add(m[1]);
  }
  const bare = /(?:^|[^\w/\\-])([A-Za-z0-9_-]+\.(?:rs|md|toml|lock|json|jsonc|jsonl|c|h|cpp|py|ts|js|cjs|ps1|cmd|exe|yml|yaml)(?![A-Za-z0-9]))(?::(\d+)(?::(\d+))?)?/g;
  let b;
  while ((b = bare.exec(text)) !== null) {
    let raw = b[1];
    if (b[2]) raw += ":" + b[2];
    if (b[3]) raw += ":" + b[3];
    raws.add(raw);
  }
  return [...raws];
}

function extractRecords(answer) {
  const text = String(answer || "");
  const set = new Set();
  const re = /\bcp7a-[a-z0-9-]+\b/g;
  let m;
  while ((m = re.exec(text)) !== null) set.add(m[0]);
  return [...set];
}

function auditAnswer(answer) {
  const paths = extractPathTexts(answer).map(citation);
  const records = extractRecords(answer).map((id) => ({
    id,
    resolved: KNOWN_RECORDS.has(id) || NON_RECORD_CP7A.has(id),
    known_record: KNOWN_RECORDS.has(id),
    non_record_whitelisted: NON_RECORD_CP7A.has(id)
  }));
  const failures = [];
  for (const p of paths) {
    if (!p.resolved) failures.push({ kind: "path", text: p.raw, reason: p.reason || "not-found" });
    else if (p.line !== null && p.line_exists === false) failures.push({ kind: "line", text: p.raw, reason: p.reason || "line-out-of-range" });
    else if (p.col !== null && p.col_exists === false) failures.push({ kind: "column", text: p.raw, reason: "column-out-of-range" });
  }
  for (const r of records) {
    if (!r.resolved) failures.push({ kind: "record", text: r.id, reason: "unknown-record-id" });
  }
  return { paths, records, failures };
}

module.exports = { auditAnswer, KNOWN_RECORDS, NON_RECORD_CP7A };
