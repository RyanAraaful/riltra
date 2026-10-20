// Order-independent fact digest of a Riltra index database.
//
// The digest covers the facts (files with their content digests, symbols,
// imports, calls with their spans and provenance) and the derived edge set.
// Rows are sorted before hashing, so the digest does not depend on the order
// SQLite happens to return them in (the ORDER BY of a non-total order is
// not stable between two builds).
"use strict";
const { DatabaseSync } = require("node:sqlite");
const crypto = require("node:crypto");
const fs = require("node:fs");

const flat = (row) =>
  Object.values(row)
    .map((v) => (v === null || v === undefined ? "" : String(v)))
    .join("\u0001");

function digestRows(rows) {
  const lines = rows.map(flat).sort();
  const h = crypto.createHash("sha256");
  for (const l of lines) h.update(l + "\n");
  return { rows: lines.length, digest: h.digest("hex") };
}

function factTables(dbPath) {
  const db = new DatabaseSync(dbPath, { readOnly: true });
  const all = (sql, ...a) => db.prepare(sql).all(...a);
  const snap = all("select * from snapshots order by id desc limit 1")[0];
  if (!snap) {
    db.close();
    return null;
  }
  const tables = {
    files: digestRows(all("select path, digest, bytes, language, errors from files where snapshot_id=?", snap.id)),
    symbols: digestRows(all("select f.path, s.kind, s.name, s.start_line, s.start_col, s.end_line, s.end_col, s.byte_start, s.byte_end, s.provider, s.grammar, s.language from symbols s join files f on f.id=s.file_id where s.snapshot_id=?", snap.id)),
    imports: digestRows(all("select f.path, i.start_line, i.start_col, i.end_line, i.end_col, i.byte_start, i.byte_end, i.text, i.provider, i.grammar, i.language from imports i join files f on f.id=i.file_id where i.snapshot_id=?", snap.id)),
    calls: digestRows(all("select f.path, c.start_line, c.start_col, c.end_line, c.end_col, c.byte_start, c.byte_end, c.name, c.unresolved, c.provider, c.grammar, c.language from calls c join files f on f.id=c.file_id where c.snapshot_id=?", snap.id)),
    edges: digestRows(all("select edge_kind, precision, provider, caller_path, caller_sbs, caller_sbe, callee_name, callee_kind, coalesce(unresolved,'') from code_edges where snapshot_id=?", snap.id)),
  };
  db.close();
  return tables;
}

function combinedDigest(tables) {
  const h = crypto.createHash("sha256");
  for (const k of Object.keys(tables).sort()) h.update(k + "=" + tables[k].digest + "\n");
  return h.digest("hex");
}

function snapshotMeta(dbPath) {
  const db = new DatabaseSync(dbPath, { readOnly: true });
  const snap = db.prepare("select * from snapshots order by id desc limit 1").get();
  db.close();
  if (!snap) return null;
  return {
    id: snap.id,
    head: snap.head,
    dirty: snap.dirty,
    status_at_write: snap.status_at_write,
  };
}

// --- the comparisons ------------------------------------------------------
//
// A parity check that cannot fail proves nothing, so each comparison
// reports WHY it rejects and never reports a match it did not measure.

const FACT_TABLES = ["files", "symbols", "imports", "calls", "edges"];

// Compare two digest objects (the output of db-mutation-check.cjs).
function compareDigests(a, b) {
  if (!a || typeof a !== "object" || !b || typeof b !== "object") {
    return { identical: false, reason: "missing-digest" };
  }
  if (!a.tables || !b.tables) {
    return { identical: false, reason: "missing-tables" };
  }
  // The two indexes must come from the same source tree: the recorded
  // commit must match, and the metadata must be there to prove it.
  if (!a.snapshot || !b.snapshot) {
    return { identical: false, reason: "missing-snapshot-metadata" };
  }
  if (a.snapshot.head !== b.snapshot.head) {
    return { identical: false, reason: "snapshot-head-differs" };
  }
  const differing = [];
  for (const t of FACT_TABLES) {
    const ta = a.tables[t];
    const tb = b.tables[t];
    if (!ta || !tb || !ta.digest || !tb.digest) {
      return { identical: false, reason: "missing-table:" + t };
    }
    if (ta.digest !== tb.digest) differing.push(t);
    else if (ta.rows !== tb.rows) differing.push(t + ":rows");
  }
  if (differing.length > 0) {
    return { identical: false, reason: "tables-differ:" + differing.join(","), tables: differing };
  }
  if (!a.combined_digest || !b.combined_digest) {
    return { identical: false, reason: "missing-combined-digest" };
  }
  if (a.combined_digest !== b.combined_digest) {
    return { identical: false, reason: "combined-digest-differs" };
  }
  return { identical: true, reason: null, tables: FACT_TABLES };
}

// The verified volatile snapshot metadata. Everything else - results, spans,
// counters, file digests - is content and must match byte for byte.
function normalizeSnapshot(snapshot) {
  const out = Object.assign({}, snapshot);
  if ("id" in out) out.id = "<SNAP>";
  if ("created_unix" in out) out.created_unix = "<TS>";
  if ("nonce" in out) out.nonce = "<NONCE>";
  if ("revision" in out) out.revision = "<REVISION>";
  return out;
}

function normalizeDoc(node) {
  if (Array.isArray(node)) return node.map(normalizeDoc);
  if (node && typeof node === "object") {
    const out = {};
    for (const [key, value] of Object.entries(node)) {
      if (key === "snapshot" && value && typeof value === "object" && !Array.isArray(value)) {
        // A result carries a numeric "snapshot" (the snapshot id): only an
        // object is the snapshot metadata.
        out[key] = normalizeSnapshot(value);
      } else if (key === "repos" && Array.isArray(value)) {
        out[key] = value.map((r) =>
          r && typeof r === "object" ? Object.assign({}, r, { added_unix: "<TS>" }) : r
        );
      } else {
        out[key] = normalizeDoc(value);
      }
    }
    return out;
  }
  return node;
}

// Normalize a query answer. A JSON document is normalized structurally, so
// no broad regex can erase a substantive difference; a plain text answer
// (a CLI error) is compared as it is.
function normalizeOutput(text) {
  const raw = String(text == null ? "" : text);
  let doc;
  try {
    doc = JSON.parse(raw);
  } catch (e) {
    return raw.trim();
  }
  return JSON.stringify(normalizeDoc(doc));
}

// Compare two query answers. A nonzero exit, a missing output or a
// substantive difference is never a match.
function compareOutputs(a, b) {
  if (!a || !b) return { identical: false, reason: "missing-output" };
  if (a.exitCode !== 0 || b.exitCode !== 0) {
    return { identical: false, reason: "nonzero-exit:" + a.exitCode + "/" + b.exitCode };
  }
  const ta = String(a.text == null ? "" : a.text);
  const tb = String(b.text == null ? "" : b.text);
  if (ta.trim() === "" || tb.trim() === "") {
    return { identical: false, reason: "empty-output" };
  }
  if (normalizeOutput(ta) === normalizeOutput(tb)) {
    return { identical: true, reason: null };
  }
  return { identical: false, reason: "content-differs" };
}

module.exports = {
  factTables,
  combinedDigest,
  digestRows,
  snapshotMeta,
  compareDigests,
  compareOutputs,
  normalizeOutput,
  FACT_TABLES,
};

// --- the command line -----------------------------------------------------

if (require.main === module) {
  const mode = process.argv[2] || "";
  const fail = (reason, detail) => {
    console.log("NOT IDENTICAL: " + reason + (detail ? " (" + detail + ")" : ""));
    process.exit(1);
  };
  if (mode === "compare-digests") {
    const read = (p) => {
      if (!p || !fs.existsSync(p)) return null;
      const text = fs.readFileSync(p, "utf8");
      if (!text.trim()) return null;
      try {
        return JSON.parse(text);
      } catch (e) {
        return null;
      }
    };
    const a = read(process.argv[3]);
    const b = read(process.argv[4]);
    if (!a || !b) {
      console.log("NOT IDENTICAL: missing-digest");
      process.exit(2);
    }
    const r = compareDigests(a, b);
    if (!r.identical) fail(r.reason);
    console.log("IDENTICAL: " + FACT_TABLES.join(","));
    process.exit(0);
  }
  if (mode === "compare-outputs") {
    const read = (p) => {
      if (!p || !fs.existsSync(p)) return "";
      return fs.readFileSync(p, "utf8");
    };
    const exitA = process.argv[5] === undefined ? 0 : Number(process.argv[5]);
    const exitB = process.argv[6] === undefined ? 0 : Number(process.argv[6]);
    const r = compareOutputs(
      { exitCode: exitA, text: read(process.argv[3]) },
      { exitCode: exitB, text: read(process.argv[4]) }
    );
    if (!r.identical) fail(r.reason);
    console.log("IDENTICAL");
    process.exit(0);
  }
  console.log("usage: node fact-digest.cjs compare-digests <a.json> <b.json>");
  console.log("       node fact-digest.cjs compare-outputs <a.txt> <b.txt> [exitA] [exitB]");
  process.exit(2);
}
