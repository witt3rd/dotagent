# Claude Code — integration

Claude Code reads `CLAUDE.md` (repo root) and supports `SessionStart` / `Stop` command
hooks. Wire the mechanical wake and drop in the thin delta.

## 1. The thin delta — `CLAUDE.md` (repo root)

Thin, pointing back to AGENTS.md. Copy the block below into your repo's `CLAUDE.md`:

```markdown
# Agent operating notes

This repo is an **active intelligence** — read `AGENTS.md` for the full doctrine (goals,
merits, concepts, mechanisms). The short loop:

- **On wake:** run `agent state` (where the repo is in time) then `agent inbox` (what's
  waiting on it). Pick up where the last caretaker left off.
- **On finish:** run `agent handoff '<what changed>'` so the next caretaker can pick up cold.
- **Keep it clean:** the log must pass `agent check` (exit 0). Never edit events in place.

The `agent` CLI is `scripts/agent` (or on PATH). If it's not present, run
`agent init <identity>` to scaffold `.agent/`.
```

## 2. The mechanical wake — lifecycle hooks

Claude Code hooks are configured in `.claude/settings.json` (project) or
`~/.claude/settings.json` (user). Copy `hook-session.sh` next to the repo, then register:

```json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "*",
        "hooks": [
          { "type": "command", "command": "bash <repo>/hook-session.sh session", "timeout": 10 }
        ]
      }
    ],
    "Stop": [
      {
        "matcher": "*",
        "hooks": [
          { "type": "command", "command": "bash <repo>/hook-session.sh stop", "timeout": 10 }
        ]
      }
    ]
  }
}
```

`SessionStart` runs `agent state` + `agent inbox`, printing the repo's state and what's
waiting on it into the session — so the agent wakes oriented. `Stop` prints a handoff
reminder. If hooks are disabled or the command isn't found, the session is unaffected (the
hook exits quietly).

## Notes

- This is the same mechanism a real install uses to wake an agent at session start — the
  schema above is the live, working shape.
- Keep the thin delta thin. `AGENTS.md` is the source of truth; `CLAUDE.md` only points at
  it and states the loop.