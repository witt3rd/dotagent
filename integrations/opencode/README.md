# opencode — integration

opencode reads `AGENTS.md` natively as its project instructions and auto-loads `skills/`
(`**/SKILL.md`). So the **universal contract applies with nothing to copy** — no thin
delta. Two affordances: the convention (recommended) and an optional mechanical-wake
plugin.

## 1. The convention (recommended, most robust)

opencode reads `AGENTS.md` at session start. If it says "on wake run `agent state` + `agent
inbox`, on finish run `agent handoff`" (see `templates/AGENTS.md`), the agent follows it —
the wake reaches the model's context through the charter itself. This is the primary path
and it's version-proof.

## 2. The mechanical wake — plugin

opencode hooks are plugin-based (no `hook` config key; hooks are a `Plugin` returning a
`Hooks` object). A plugin runs at `session.created` and actually executes the wake:

`.opencode/plugin/agent-wake.ts` (also in this directory):

```ts
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
      '`.cwd(directory).nothrow().quiet()
      const text = await out.text()
      if (text.trim()) console.log(text.trim())
    },
  }
}) satisfies Plugin
```

Verified against opencode 1.18.18: the `event` hook receives `{ event }` with a `type`
discriminator; `session.created` fires on a new session; the BunShell `$` returns a promise
with `.cwd()/.nothrow()/.quiet()` and a `.text()` output. The plugin runs once per opencode
process and guards on `scripts/agent` presence (silent if absent).

**What the plugin does and doesn't do, honestly:** it runs the wake mechanically and logs it
(to the opencode log). It does **not** by default inject the text into the model's chat
context — the hook surface for injecting a non-turn part is version-fragile. The wake that
reaches the model is the convention in (1). The plugin is the belt-and-suspenders that makes
the wake real (the commands actually run) even if the model doesn't invoke them, and the
AGENTS.md charter guarantees the model is told to.

## Notes

- No thin delta needed — opencode's native AGENTS.md + skills/ loading is the whole wiring.
- The plugin is optional. If you want a purely convention-based setup, skip `.opencode/`
  entirely; AGENTS.md alone is a complete integration.
- If the plugin's `session.created` event or BunShell surface changes in a future opencode,
  the guard clauses keep it harmless (it silently no-ops), and the convention in (1) still
  wakes the agent.