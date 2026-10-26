// Regression tests for the CP2 parity comparison.
//
// The comparison must be able to FAIL. A parity check that cannot fail
// proves nothing, so every branch that must reject is covered here:
//   - two factually identical indexes pass;
//   - a content change that the oracle does not have fails;
//   - an empty or missing digest never passes;
//   - a missing query output or a nonzero exit never passes;
//   - only the verified volatile snapshot metadata is normalized, and a
//     substantive difference is never normalized away.
//
// Usage: node --test fact-digest.test.cjs
"use strict";

const test = require("node:test");
const assert = require("node:assert");

const { compareDigests, compareOutputs, normalizeOutput } = require("./fact-digest.cjs");

// --- fixtures ------------------------------------------------------------

function digest(overrides) {
  const tables = {
    files: { rows: 2, digest: "f" },
    symbols: { rows: 3, digest: "s" },
    imports: { rows: 4, digest: "i" },
    calls: { rows: 5, digest: "c" },
    edges: { rows: 6, digest: "e" },
  };
  const d = { snapshot: { id: 1, head: "abc", dirty: true }, tables: tables, combined_digest: "combined" };
  return Object.assign(d, overrides || {});
}

// A query answer with the volatile snapshot metadata.
function answer(createdUnix, nonce) {
  return JSON.stringify({
    repo: "mmsp-pin",
    snapshot: { id: 7, created_unix: createdUnix, nonce: nonce, revision: "head=abc|nonce=" + nonce },
    total: 1,
    results: [{ path: "a.cc", name: "x", span: { start_line: 1 }, snapshot: 1 }],
  });
}

// --- the digest comparison ------------------------------------------------

test("two factually identical indexes pass", () => {
  const r = compareDigests(digest(), digest());
  assert.strictEqual(r.identical, true);
  assert.strictEqual(r.reason, null);
});

test("a content change the oracle does not have fails", () => {
  const oracle = digest();
  // The file's content digest differs: the oracle was built from another tree.
  oracle.tables.files = { rows: 2, digest: "other" };
  const r = compareDigests(digest(), oracle);
  assert.strictEqual(r.identical, false);
  assert.match(r.reason, /files/);
});

test("a changed edge set fails", () => {
  const oracle = digest();
  oracle.tables.edges = { rows: 6, digest: "other" };
  const r = compareDigests(digest(), oracle);
  assert.strictEqual(r.identical, false);
  assert.match(r.reason, /edges/);
});

test("a different row count fails", () => {
  const oracle = digest();
  oracle.tables.calls = { rows: 9, digest: "c" };
  const r = compareDigests(digest(), oracle);
  assert.strictEqual(r.identical, false);
});

test("an empty digest never passes", () => {
  const empty = { tables: { files: { rows: 0, digest: "" } } };
  const r = compareDigests(digest(), empty);
  assert.strictEqual(r.identical, false);
  assert.match(r.reason, /missing/);
});

test("a missing digest never passes", () => {
  assert.strictEqual(compareDigests(digest(), null).identical, false);
  assert.strictEqual(compareDigests(digest(), {}).identical, false);
  assert.strictEqual(compareDigests(null, digest()).identical, false);
});

test("a missing combined digest never passes", () => {
  const r = compareDigests(digest(), digest({ combined_digest: "" }));
  assert.strictEqual(r.identical, false);
  assert.match(r.reason, /combined/);
});

test("indexes from different source trees fail", () => {
  const oracle = digest();
  oracle.snapshot = { id: 1, head: "other", dirty: true };
  const r = compareDigests(digest(), oracle);
  assert.strictEqual(r.identical, false);
  assert.match(r.reason, /head/);
});

test("a missing source-tree record never passes", () => {
  const oracle = digest();
  delete oracle.snapshot;
  const r = compareDigests(digest(), oracle);
  assert.strictEqual(r.identical, false);
  assert.match(r.reason, /snapshot/);
});

// --- the query-output comparison -----------------------------------------

test("two identical answers pass", () => {
  const r = compareOutputs({ exitCode: 0, text: answer(1000, "n1") }, { exitCode: 0, text: answer(1000, "n1") });
  assert.strictEqual(r.identical, true);
});

test("only the volatile snapshot metadata is normalized", () => {
  const a = { exitCode: 0, text: answer(1000, "n1") };
  const b = { exitCode: 0, text: answer(2000, "n2") };
  const r = compareOutputs(a, b);
  assert.strictEqual(r.identical, true, "created_unix and nonce are volatile");
});

test("a substantive difference is never normalized away", () => {
  const a = JSON.parse(answer(1000, "n1"));
  const b = JSON.parse(answer(1000, "n1"));
  b.total = 2;
  b.results[0].span.start_line = 99;
  const r = compareOutputs({ exitCode: 0, text: JSON.stringify(a) }, { exitCode: 0, text: JSON.stringify(b) });
  assert.strictEqual(r.identical, false);
  assert.match(r.reason, /content/);
});

test("a changed result name is never normalized away", () => {
  const a = JSON.parse(answer(1000, "n1"));
  const b = JSON.parse(answer(1000, "n1"));
  b.results[0].name = "y";
  const r = compareOutputs({ exitCode: 0, text: JSON.stringify(a) }, { exitCode: 0, text: JSON.stringify(b) });
  assert.strictEqual(r.identical, false);
});

test("a missing query output never passes", () => {
  const r = compareOutputs({ exitCode: 0, text: answer(1000, "n1") }, { exitCode: 0, text: "" });
  assert.strictEqual(r.identical, false);
  assert.match(r.reason, /empty/);
  assert.strictEqual(compareOutputs(null, { exitCode: 0, text: "x" }).identical, false);
});

test("a nonzero exit code never passes", () => {
  const r = compareOutputs({ exitCode: 3, text: answer(1000, "n1") }, { exitCode: 0, text: answer(1000, "n1") });
  assert.strictEqual(r.identical, false);
  assert.match(r.reason, /exit/);
});

test("a plain text answer is compared as text", () => {
  const r = compareOutputs({ exitCode: 3, text: "error: anchor unresolved: source-changed" },
    { exitCode: 3, text: "error: anchor unresolved: source-changed" });
  // A nonzero exit never passes, even when the text is equal.
  assert.strictEqual(r.identical, false);
  assert.strictEqual(normalizeOutput("error: anchor unresolved"), "error: anchor unresolved");
});
