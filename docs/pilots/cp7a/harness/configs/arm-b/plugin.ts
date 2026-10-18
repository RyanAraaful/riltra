import { Plugin } from "@opencode/plugin"
import { appendFileSync } from "node:fs"

const LOG = "E:\\dev\\Riltra\\.cp7a\\plugin-events.jsonl"
const ALLOWED = new Set([
  "riltra_riltra_repo_list",
  "riltra_riltra_repo_status",
  "riltra_riltra_code_search",
  "riltra_riltra_code_callers",
  "riltra_riltra_code_callees",
  "riltra_riltra_code_trace",
  "riltra_riltra_code_impact",
  "riltra_riltra_knowledge_search",
  "riltra_riltra_knowledge_show",
  "riltra_riltra_knowledge_related",
  "riltra_riltra_health",
])

function log(entry) {
  try {
    appendFileSync(LOG, JSON.stringify(entry) + "\n")
  } catch {}
}

export default Plugin.define({
  id: "cp7a.mcp",
  async setup(ctx) {
    await ctx.session.hook("context", (event) => {
      const before = Object.keys(event.tools)
      for (const name of before) {
        if (name !== "read" && !ALLOWED.has(name)) delete event.tools[name]
      }
      log({
        kind: "context",
        sessionID: event.sessionID,
        agent: event.agent?.id ?? event.agent,
        kept: Object.keys(event.tools).sort(),
        removed: before.filter((name) => name !== "read" && !ALLOWED.has(name)).sort(),
      })
    })

    await ctx.tool.hook("execute.before", (event) => {
      if (event.tool === "read") {
        log({ kind: "blocked", sessionID: event.sessionID, tool: "read" })
        throw new Error("CP7A_MCP_READ_BLOCKED")
      }
      if (!ALLOWED.has(event.tool)) {
        log({ kind: "blocked", sessionID: event.sessionID, tool: event.tool })
        throw new Error(`CP7A_MCP_TOOL_BLOCKED:${event.tool}`)
      }
      log({ kind: "allowed", sessionID: event.sessionID, tool: event.tool })
    })
  },
})
