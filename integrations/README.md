# Integrations — wake your agent up

The `.agent/` log and the `agent` CLI are **tool-agnostic**: any agent, any framework, any
machine. What differs is *how an agent gets woken up to engage with them*. That's the
interop layer this directory provides.

The design is a single principle with two mechanisms:

## 1. The universal contract: `AGENTS.md`

Every modern agent already reads `AGENTS.md` (Claude Code, Codex, opencode, Gemini, Cursor,
and more have adopted it as the standard). So **the contract is already universal** — a repo
with a good `AGENTS.md` (see `templates/AGENTS.md`) that says "on wake run `agent state` +
`agent inbox`, on finish run `agent handoff`" is already wired for any of them.

## 2. Two optional affordances, per agent

**a. Thin per-agent deltas.** Where a framework looks for its own file first (`CLAUDE.md`,
`GEMINI.md`, `.cursor/rules`, …), drop in a *thin* file that points back to AGENTS.md and
states the wake/finish loop. It must stay thin — the doctrine lives in AGENTS.md, not
duplicated (single source of truth).

**b. Lifecycle hooks — the mechanical wake.** Claude Code and Codex support `SessionStart`
/ `Stop` command hooks. Register `integrations/_shared/hook-session.sh` and the agent is
*woken* at session start with the repo's actual state and inbox — no reliance on the model
remembering the doctrine — and reminded to hand off at the end.

The hooks are provided as snippets to register in *your* repo, not installed for you (your
agent setup is yours; we just hand you the batteries).

## Wiring guide

| Agent | Reads AGENTS.md | Thin delta | Lifecycle hook |
|---|---|---|---|
| Claude Code | yes | `CLAUDE.md` | yes — `SessionStart` / `Stop` |
| Codex | yes | — (none needed) | yes — `SessionStart` (in `.codex/hooks.json`) |
| opencode | yes | — (none needed) | advanced — plugin (see below) |
| Gemini CLI | yes | `GEMINI.md` | no hook mechanism |
| Cursor | yes | `.cursor/rules/*.mdc` | per-project |

Each subdirectory has a README with the exact, working registration. The quick start for
the two hook-capable agents (Claude Code and Codex):

```bash
# 1. Copy the shared hook next to your repo (or keep it on PATH).
cp dotagent/integrations/_shared/hook-session.sh /path/to/repo/hook-session.sh

# 2. Register it. Claude Code (your repo's .claude/settings.json):
cat > .claude/settings.json <<'JSON'
{
  "hooks": {
    "SessionStart": [{ "matcher": "*", "hooks": [
      { "type": "command", "command": "bash <repo>/hook-session.sh session", "timeout": 10 }
    ]}],
    "Stop": [{ "matcher": "*", "hooks": [
      { "type": "command", "command": "bash <repo>/hook-session.sh stop", "timeout": 10 }
    ]}]
  }
}
JSON

# 3. That's it. Next session, the agent wakes to your state and inbox.
```

## Why this is enough

We don't try to build an agent, an MCP server, a daemon, or a push mechanism. We keep the
store universal and hand you the three things you need to make *your* agent of choice engage
with it: the contract (AGENTS.md), a thin delta if your tool wants one, and a mechanical
wake where your tool supports it. Everything else — how the agent actually does the work —
stays with your agent, where it belongs.

## Adding an agent

Add a subdirectory here. The pattern is always the same:
1. The thin delta file (if the framework uses one) — pointing back to AGENTS.md.
2. The hook registration (if it supports hooks) — reusing `_shared/hook-session.sh`.
3. A README with the exact wiring.
4. A row in the table above.

Keep the delta thin and the registration exact — `ground truth, not generic advice`.