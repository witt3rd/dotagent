# Codex — integration

Codex reads `AGENTS.md` at the repo root as its standard instructions file, so **no thin
delta is needed** — the universal contract already applies. What Codex adds is a native
`SessionStart` hook mechanism for the mechanical wake.

## 1. The contract

Nothing to copy. Codex reads `AGENTS.md`; if it says "on wake run `agent state` + `agent
inbox`, on finish run `agent handoff`", Codex follows it. If your repo needs the delta
inline, point at AGENTS.md the same way as `CLAUDE.md` (see `claude-code/`).

## 2. The mechanical wake — lifecycle hooks

Codex hooks are configured in `.codex/hooks.json` (project) or `~/.codex/hooks.json`
(user), and require `[features] hooks = true` in `config.toml`. Copy `hook-session.sh`
next to the repo, then register:

```json
{
  "hooks": {
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "bash <repo>/hook-session.sh session",
            "timeout": 10
          }
        ]
      }
    ]
  }
}
```

```toml
# config.toml (required for hooks to fire)
[features]
hooks = true
```

`SessionStart` prints the repo's `agent state` + `agent inbox` into the session, waking the
agent oriented. For a `Stop` reminder, add the same entry with `hook-session.sh stop`.

## Notes

- Codex `SessionStart` accepts a `matcher` (e.g. `"startup|resume"`) to scope when the
  wake fires — omit it to run on every session start.
- This is the live, working schema for the command-hook shape.