"use strict";
// CP3 harness: summarise the real OpenCode session (E10 / G-MCP agent side).
//
// The raw session transcript is archived next to this summary. The summary
// records only what the session did, so a reader can check the claims
// without parsing the transcript.
//
// Usage:
//   node opencode-summary.cjs <session-stdout.json> <out.json>

const fs = require("node:fs");

const raw = fs.readFileSync(process.argv[2], "utf8");
const lines = raw.split("\n").filter((l) => l.trim());
const msgs = [];
for (const l of lines) {
  try {
    msgs.push(JSON.parse(l));
  } catch (e) {
    /* not a JSON line */
  }
}

const uses = msgs.filter((m) => m.type === "tool_use").map((m) => m.part);
const calls = [];
for (const p of uses) {
  const input = JSON.stringify((p.state && p.state.input) || {});
  const match = /riltra_[a-z_]+/.exec(input);
  const output = String((p.state && p.state.output) || "").replace(/\s+/g, " ");
  calls.push({
    tool: match ? match[0] : "opencode.list_mcp_resources",
    status: (p.state && p.state.status) || null,
    mentioned: match ? match[0] : null,
    output_head: output.slice(0, 400),
    output_length: output.length,
    classified_error: (() => {
      // The tool error may arrive plain or JSON-escaped inside a wrapper.
      const m = /not_registered.{0,120}/.exec(output);
      return m ? m[0] : null;
    })(),
  });
}

const texts = msgs
  .map((m) => (m.part && m.part.text) || m.text)
  .filter((t) => typeof t === "string" && t.length > 200);
const finalText = texts.length ? texts[texts.length - 1] : null;

// The classified error the agent was asked to surface verbatim.
let classifiedError = null;
for (const c of calls) {
  if (c.classified_error) {
    classifiedError = c.classified_error;
    break;
  }
}

// Every repository-relative path the agent cited in its final answer.
const cited = finalText
  ? [...new Set((finalText.match(/colmap\/src\/[A-Za-z0-9_./-]+\.[a-z]+/g) || []))]
  : [];

// Did the agent claim a fact it could not support? Look for the explicit
// "I could not" statements; they are the honesty signal.
const disclaimers = finalText
  ? (finalText.match(/[^.\n]*\b(could not|cannot|not resolve[ds]?|unverified|not verified|no tool can)[^.\n]*\./gi) || [])
      .map((s) => s.trim())
      .slice(0, 12)
  : [];

const summary = {
  transcript_lines: lines.length,
  tool_use_parts: uses.length,
  riltra_tools_used: [...new Set(calls.map((c) => c.mentioned).filter(Boolean))],
  calls,
  classified_error_surfaced: classifiedError,
  cited_repository_relative_paths: cited,
  cited_path_count: cited.length,
  explicit_disclaimers: disclaimers,
  final_text_length: finalText ? finalText.length : 0,
};

if (process.argv[3]) {
  fs.writeFileSync(process.argv[3], JSON.stringify(summary, null, 2));
}
console.log(JSON.stringify(summary, null, 2));
