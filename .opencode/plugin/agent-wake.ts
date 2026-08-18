// .opencode/plugin/agent-wake.ts
//
// Mechanical wake for an active-intelligence repo: on the first `session.created`
// event, run `agent state` + `agent inbox` against the repo root and log the result.
//
// The plugin is belt-and-suspenders. The wake that reaches the model's context is the
// AGENTS.md convention — opencode reads AGENTS.md, which instructs the agent to run
// `agent state` + `agent inbox` on wake, and it does. This plugin makes the wake
// mechanical too (it actually runs the commands) and surfaces the result in the log.
//
// Verified against opencode 1.18.18: Hooks.event receives { event } with a `type`
// discriminator; the plugin runs once per opencode process (not per session created).

import type { Plugin } from "@opencode-ai/plugin"

let woke = false

export default (async ({ directory, $ }) => {
  return {
    async event({ event }) {
      if (event.type !== "session.created") return
      if (woke) return
      woke = true

      const out = await $`bash -c '
        cd "$(git rev-parse --show-toplevel 2>/dev/null)" 2>/dev/null || exit 0
        # Prefer the repo-local control plane; never let an ambient `agent` on PATH shadow it.
        A="scripts/agent"; [ -x "$A" ] || A="$(command -v agent 2>/dev/null)" || exit 0
        echo "── [agent] state ──"; "$A" state 2>/dev/null
        echo "── [agent] inbox ──"; "$A" inbox 2>/dev/null
      '`
        .cwd(directory)
        .nothrow()
        .quiet()

      const text = await out.text()
      if (text.trim()) console.log(text.trim())
    },
  }
}) satisfies Plugin