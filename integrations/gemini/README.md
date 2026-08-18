# Gemini CLI — integration

Gemini reads `GEMINI.md` (repo root) as its instructions file and reads `AGENTS.md`. It has
no lifecycle-hook mechanism (as of writing), so the integration is the **thin delta** plus
the convention.

## 1. The thin delta — `GEMINI.md` (repo root)

Thin, pointing back to AGENTS.md:

```markdown
# Agent operating notes

This repo is an **active intelligence** — read `AGENTS.md` for the full doctrine. The loop:

- **On wake:** run `agent state` then `agent inbox`; pick up where the last caretaker left off.
- **On finish:** run `agent handoff '<what changed>'`.
- **Keep it clean:** the log must pass `agent check` (exit 0). Never edit events in place.

The `agent` CLI is `scripts/agent` (or on PATH); if absent, `agent init <identity>` scaffolds `.agent/`.
```

## 2. The convention

Without hooks, the wake is convention: `GEMINI.md` states the loop and Gemini follows it.
To make it mechanical anyway, add a shell wrapper you run before starting the session that
prints the wake:

```bash
alias gemini-wake='{ scripts/agent state 2>/dev/null; scripts/agent inbox 2>/dev/null; } && gemini'
```

That surfaces state + inbox into the session the same way a hook would, without needing a
hook mechanism.

## Notes

- Gemini reads `AGENTS.md` too — keep `GEMINI.md` thin and let `AGENTS.md` be the source of
  truth (single source of truth).