# opencode — integration

opencode reads `AGENTS.md` natively as its project instructions, so the **universal
contract applies with nothing to copy** — no thin delta. Two optional affordances.

## 1. The contract

Free. opencode auto-loads `AGENTS.md` and the repo's skills (it scans `**/SKILL.md` under
the skills root), so a repo that follows `templates/AGENTS.md` and ships `skills/` is fully
wired for opencode with zero config. To make the repo's own skills available, they're
already discoverable in `skills/`.

## 2. Optional — a session-start wake plugin

opencode hooks are plugin-based (a small `.ts` module). If you want a *mechanical* wake
that prints `agent state` + `agent inbox` at session start, drop this into `.opencode/plugin/`:

```ts
// .opencode/plugin/agent-wake.ts
import type { Plugin } from "@opencode-ai/plugin"

const wake = {
  session: async ({ project, $ }: any) => {
    const script = project === "agent" ? "agent" : "scripts/agent"
    // Best-effort: if the repo has no agent CLI, stay silent.
    const { stdout } = await $`bash -c '
      command -v '"'"'agent'"'"' >/dev/null 2>&1 || [ -x scripts/agent ] || exit 0
      echo "── [agent] state ──";  agent state 2>/dev/null || scripts/agent state 2>/dev/null
      echo "── [agent] inbox ──"; agent inbox 2>/dev/null || scripts/agent inbox 2>/dev/null
    '`.nothrow().quiet()
    if (stdout) console.log(stdout)
  },
}

export default (async (input) => wake) satisfies Plugin
```

> The plugin hook surface varies by opencode version. If `session` isn't the right event in
> yours, hook the closest start-of-session bus event (the plugin `event(input)` receives all
> of them). Prefer the convention below for most setups.

## 3. Recommended convention (most robust)

Because opencode reads AGENTS.md natively and the doctrine already says "on wake run `agent
state` + `agent inbox`", the most robust opencode integration is **no extra wiring at all**:
the agent reads the charter and follows the loop. The plugin is a convenience for teams that
want the wake to be mechanical rather than convention. Use whichever matches your taste —
the store is the same either way.