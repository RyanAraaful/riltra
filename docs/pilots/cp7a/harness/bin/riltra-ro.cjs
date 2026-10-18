// SPDX-License-Identifier: Apache-2.0
// CP7-A read-only CLI adapter. Validates an explicit allowlist of read-only
// Riltra commands and spawns the pinned binary without a shell.
"use strict";

const { spawnSync } = require("node:child_process");

const RILTRA = "E:\\dev\\Riltra\\target\\debug\\riltra.exe";
const HOME = "E:\\dev\\Riltra\\.cp7a\\home-template";
const REPOS = new Set(["riltra", "other", "empty"]);

function fail(message) {
  console.error(`riltra-ro: ${message}`);
  process.exit(64);
}

const args = process.argv.slice(2);
if (args.length === 0) fail("missing command");

function take(flag) {
  const i = args.indexOf(flag);
  if (i < 0) return null;
  if (i === args.length - 1) fail(`${flag} requires a value`);
  return args[i + 1];
}

function has(flag) {
  return args.includes(flag);
}

function requireRepo() {
  const repo = take("--repo");
  if (!repo) fail("--repo is required");
  if (!REPOS.has(repo)) fail(`repo not allowed: ${repo}`);
  return repo;
}

// Graph/index-status commands open the index live; never let them create the
// unindexed fixture's store.
function requireIndexedRepo() {
  if (requireRepo() === "empty") fail("repository has no index; command not allowed");
}

function requireValue(flag) {
  const value = take(flag);
  if (!value || value.startsWith("--")) fail(`${flag} requires a value`);
  if (/[\u0000-\u001f]/.test(value)) fail(`${flag} contains a control character`);
  return value;
}

function requireNumber(flag, min, max) {
  const raw = requireValue(flag);
  if (!/^\d+$/.test(raw)) fail(`${flag} must be an integer`);
  const n = Number(raw);
  if (n < min || n > max) fail(`${flag} out of range`);
  return n;
}

function optionalNumber(flag, min, max) {
  if (has(flag)) requireNumber(flag, min, max);
}

function rejectUnknownFlags(allowed) {
  for (let i = 0; i < args.length; i++) {
    if (!args[i].startsWith("--")) continue;
    if (!allowed.has(args[i])) fail(`flag not allowed: ${args[i]}`);
    if (args[i] === "--input") fail("stdin is not allowed");
  }
}

const [command, subcommand] = args;

if (command === "--help" || command === "--version") {
  // Read-only discovery.
} else if (command === "status") {
  rejectUnknownFlags(new Set());
  if (args.length !== 1) fail("status takes no arguments");
} else if (command === "doctor") {
  rejectUnknownFlags(new Set(["--repo"]));
  if (has("--repo")) requireRepo();
} else if (command === "repo" && subcommand === "list") {
  rejectUnknownFlags(new Set());
  if (args.length !== 2) fail("repo list takes no arguments");
} else if (command === "repo" && subcommand === "status") {
  rejectUnknownFlags(new Set(["--repo"]));
  requireRepo();
  if (args.length !== 4) fail("repo status requires --repo");
} else if (command === "index" && subcommand === "status") {
  rejectUnknownFlags(new Set(["--repo"]));
  requireIndexedRepo();
  if (args.length !== 4) fail("index status requires --repo");
} else if (command === "search") {
  rejectUnknownFlags(new Set(["--repo", "--symbol", "--limit", "--offset"]));
  requireRepo();
  requireValue("--symbol");
  optionalNumber("--limit", 1, 1000);
  optionalNumber("--offset", 0, 1000000);
} else if (["callers", "callees"].includes(command)) {
  rejectUnknownFlags(new Set(["--repo", "--path", "--line", "--col", "--symbol", "--limit", "--offset"]));
  requireIndexedRepo();
  requireValue("--path");
  requireNumber("--line", 1, 1000000);
  requireNumber("--col", 1, 1000000);
  optionalNumber("--limit", 1, 1000);
  optionalNumber("--offset", 0, 1000000);
} else if (command === "trace") {
  rejectUnknownFlags(new Set(["--repo", "--path", "--line", "--col", "--symbol", "--to", "--depth", "--limit", "--offset"]));
  requireIndexedRepo();
  requireValue("--path");
  requireNumber("--line", 1, 1000000);
  requireNumber("--col", 1, 1000000);
  if (has("--to")) requireValue("--to");
  optionalNumber("--depth", 1, 6);
  optionalNumber("--limit", 1, 1000);
  optionalNumber("--offset", 0, 1000000);
} else if (command === "impact") {
  rejectUnknownFlags(new Set(["--repo", "--path", "--line", "--col", "--symbol", "--depth", "--limit", "--offset"]));
  requireIndexedRepo();
  requireValue("--path");
  requireNumber("--line", 1, 1000000);
  requireNumber("--col", 1, 1000000);
  optionalNumber("--depth", 1, 6);
  optionalNumber("--limit", 1, 1000);
  optionalNumber("--offset", 0, 1000000);
} else if (command === "knowledge" && subcommand === "search") {
  rejectUnknownFlags(new Set(["--repo", "--query", "--kind", "--include-archived", "--include-superseded", "--include-retracted", "--limit", "--offset", "--generation"]));
  requireRepo();
  requireValue("--query");
  optionalNumber("--limit", 1, 1000);
  optionalNumber("--offset", 0, 1000000);
  optionalNumber("--generation", 0, Number.MAX_SAFE_INTEGER);
} else if (command === "knowledge" && subcommand === "show") {
  rejectUnknownFlags(new Set(["--repo", "--id", "--revision"]));
  requireRepo();
  requireValue("--id");
  optionalNumber("--revision", 1, Number.MAX_SAFE_INTEGER);
} else if (command === "knowledge" && subcommand === "related") {
  rejectUnknownFlags(new Set(["--repo", "--id", "--revision", "--direction", "--include-retracted", "--limit", "--offset", "--generation"]));
  requireRepo();
  requireValue("--id");
  optionalNumber("--revision", 1, Number.MAX_SAFE_INTEGER);
  if (has("--direction")) {
    const direction = requireValue("--direction");
    if (!["inbound", "outbound"].includes(direction)) fail("--direction must be inbound or outbound");
  }
  optionalNumber("--limit", 1, 1000);
  optionalNumber("--offset", 0, 1000000);
  optionalNumber("--generation", 0, Number.MAX_SAFE_INTEGER);
} else {
  fail(`command not allowed: ${args.join(" ")}`);
}

const result = spawnSync(RILTRA, args, {
  env: { ...process.env, RILTRA_HOME: HOME },
  encoding: "utf8",
  windowsHide: true,
  maxBuffer: 64 * 1024 * 1024,
});

if (result.error) {
  console.error(`riltra-ro: failed to start riltra: ${result.error.message}`);
  process.exit(1);
}
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
process.exit(result.status === null ? 1 : result.status);
