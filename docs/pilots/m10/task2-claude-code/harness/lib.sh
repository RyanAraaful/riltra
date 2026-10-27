# SPDX-License-Identifier: Apache-2.0
# Isolated Claude Code session for the M10 Task 2 pilot. Sourced by probe.sh and session.sh.
# Paths come from the environment only: ROOT, CACHE, CLAUDE_BIN, and FREEZE (set by the caller).

: "${ROOT:?set ROOT to the scratch root outside the repository}"
: "${CACHE:?set CACHE to the dependency cache outside every workspace}"
: "${CLAUDE_BIN:?set CLAUDE_BIN to the Claude Code executable}"

PROFILE="$ROOT/claude-profile"   # benchmark profile supplied by the owner; holds the login
BIN="$ROOT/bin"                  # Arm R binary built from the pinned tree (PROCEDURE.md section 3)
BASE_TOOLS="Read,Glob,Grep,Edit,Write,Bash"
RILTRA_TOOLS="mcp__riltra__riltra_code_callees,mcp__riltra__riltra_code_callers,mcp__riltra__riltra_code_impact,mcp__riltra__riltra_code_search,mcp__riltra__riltra_code_trace,mcp__riltra__riltra_health,mcp__riltra__riltra_knowledge_related,mcp__riltra__riltra_knowledge_search,mcp__riltra__riltra_knowledge_show,mcp__riltra__riltra_repo_list,mcp__riltra__riltra_repo_status"
SEMANTIC_TOOL="mcp__riltra__riltra_code_semantic"   # advertised by the server; denied at the client (owner instruction 4)

# claude_session <arm R|B> <workdir> <outdir> <prompt-file> <timeout-seconds>
# One isolated session. Writes transcript.jsonl, stderr.log, debug.log and home/ under <outdir>;
# for arm R also mcp.json and riltra-home/. Prints the client exit code (124 = time cap).
claude_session() {
  local arm=$1 wd=$2 out=$3 prompt=$4 secs=$5
  local home="$out/home" path_env="$PATH" allow="$BASE_TOOLS" envx=() flags=()
  mkdir -p "$home/AppData/Roaming" "$home/AppData/Local" "$out"
  if [ "$arm" = R ]; then
    mkdir -p "$out/riltra-home"
    printf '{"mcpServers":{"riltra":{"type":"stdio","command":"%s","args":["mcp"],"env":{"RILTRA_HOME":"%s"}}}}\n' \
      "$(cygpath -m "$BIN/riltra.exe")" "$(cygpath -m "$out/riltra-home")" > "$out/mcp.json"
    path_env="$BIN:$PATH"
    allow="$BASE_TOOLS,$RILTRA_TOOLS"
    envx=(RILTRA_HOME="$(cygpath -w "$out/riltra-home")")
    flags=(--mcp-config "$(cygpath -w "$out/mcp.json")" --disallowedTools "$SEMANTIC_TOOL")
  fi
  (
    cd "$wd" && timeout "$secs" env -i \
      PATH="$path_env" SystemRoot="$SYSTEMROOT" SYSTEMROOT="$SYSTEMROOT" ComSpec="$ComSpec" \
      PATHEXT="$PATHEXT" USERNAME="$USERNAME" TEMP="$(cygpath -w "$TEMP")" TMP="$(cygpath -w "$TMP")" \
      HOME="$home" USERPROFILE="$(cygpath -w "$home")" \
      APPDATA="$(cygpath -w "$home/AppData/Roaming")" LOCALAPPDATA="$(cygpath -w "$home/AppData/Local")" \
      CLAUDE_CONFIG_DIR="$(cygpath -w "$PROFILE")" \
      CARGO_HOME="$(cygpath -w "$HOME/.cargo")" RUSTUP_HOME="$(cygpath -w "$HOME/.rustup")" \
      CARGO_TARGET_DIR="$CACHE" ${envx[@]+"${envx[@]}"} \
      "$CLAUDE_BIN" -p --output-format stream-json --verbose --no-session-persistence \
        --restricted --tools "$BASE_TOOLS" --allowedTools "$allow" \
        --permission-mode dontAsk --permission-prompts none \
        --strict-mcp-config ${flags[@]+"${flags[@]}"} --disable-slash-commands \
        --debug-file "$(cygpath -w "$out/debug.log")" \
        < "$prompt" > "$out/transcript.jsonl" 2> "$out/stderr.log"
  )
  echo $?
}

# config_hash: SHA-256 of the sanitized client configuration of the benchmark profile. It covers
# config-bearing paths (names only) and the MCP entries of .claude.json. It never reads
# .credentials.json, account fields, or session transcripts.
config_hash() {
  node -e '
    const fs = require("fs"), path = require("path"), crypto = require("crypto");
    const P = process.argv[1];
    const names = [];
    const walk = (rel) => {
      const abs = path.join(P, rel);
      if (!fs.existsSync(abs)) return;
      names.push(rel.split(path.sep).join("/"));
      if (fs.statSync(abs).isDirectory()) for (const f of fs.readdirSync(abs).sort()) walk(path.join(rel, f));
    };
    for (const rel of ["settings.json", "CLAUDE.md", "skills", "plugins", "agents", "commands", "hooks"]) walk(rel);
    const projects = path.join(P, "projects");
    if (fs.existsSync(projects)) for (const d of fs.readdirSync(projects).sort()) walk(path.join("projects", d, "memory"));
    let mcp = null;
    try {
      const c = JSON.parse(fs.readFileSync(path.join(P, ".claude.json"), "utf8"));
      mcp = { mcpServers: c.mcpServers ?? null,
              projects: Object.fromEntries(Object.entries(c.projects ?? {}).map(([k, v]) => [k, v?.mcpServers ?? null])) };
    } catch {}
    console.log(crypto.createHash("sha256").update(JSON.stringify({ names, mcp })).digest("hex"));
  ' "$(cygpath -w "$PROFILE")"
}
