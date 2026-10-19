import { Plugin } from "@opencode/plugin"
import { appendFileSync } from "node:fs"

const LOG = "E:\\dev\\Riltra\\.cp7a\\plugin-events.jsonl"
const ALLOWED = new Set(["read", "shell"])
const WRAPPER =
  /^&?\s*["']?E:[\\/]+dev[\\/]+Riltra[\\/]+\.cp7a[\\/]+bin[\\/]+riltra-ro\.cmd["']?\s+(.+)$/i
const FORBIDDEN = /[;&|<>`$(){}\[\]\n\r*?]/

function log(entry) {
  try {
    appendFileSync(LOG, JSON.stringify(entry) + "\n")
  } catch {}
}

function validWrapperCommand(command) {
  const match = WRAPPER.exec(command.trim())
  if (!match) return false
  return !FORBIDDEN.test(match[1])
}

export default Plugin.define({
  id: "cp7a.cli",
  async setup(ctx) {
    await ctx.session.hook("context", (event) => {
      const before = Object.keys(event.tools)
      for (const name of before) {
        if (!ALLOWED.has(name)) delete event.tools[name]
      }
      log({
        kind: "context",
        sessionID: event.sessionID,
        agent: event.agent?.id ?? event.agent,
        kept: Object.keys(event.tools).sort(),
        removed: before.filter((name) => !ALLOWED.has(name)).sort(),
      })
    })

    await ctx.tool.hook("execute.before", (event) => {
      if (event.tool === "read") {
        log({ kind: "blocked", sessionID: event.sessionID, tool: "read" })
        throw new Error("CP7A_CLI_READ_BLOCKED")
      }
      if (event.tool !== "shell") {
        log({ kind: "blocked", sessionID: event.sessionID, tool: event.tool })
        throw new Error(`CP7A_CLI_TOOL_BLOCKED:${event.tool}`)
      }
      const command = String(event.input?.command ?? "")
      if (!validWrapperCommand(command)) {
        log({ kind: "blocked", sessionID: event.sessionID, tool: "shell", command })
        throw new Error("CP7A_CLI_COMMAND_BLOCKED")
      }
      log({ kind: "allowed", sessionID: event.sessionID, tool: "shell", command })
    })
  },
})
