# Cursor — integration

Cursor reads `AGENTS.md` at the repo root (adopted as a standard instructions file) and
also honors `.cursor/rules/*.mdc` for repo-scoped rules. There is no command-hook mechanism
in the open product, so the integration is the thin rule file plus the convention.

## 1. The thin delta — `.cursor/rules/agent.mdc`

Thin, pointing back to AGENTS.md:

```markdown
---
description: Active-intelligence operating loop — read AGENTS.md and follow it.
globs: 
alwaysApply: true
---

This repo is an **active intelligence** — read `AGENTS.md` for the full doctrine. The loop:

- **On wake:** run `agent state` then `agent inbox`; pick up where the last caretaker left off.
- **On finish:** run `agent handoff '<what changed>'`.
- **Keep it clean:** the log must pass `agent check` (exit 0). Never edit events in place.

The `agent` CLI is `scripts/agent` (or on PATH); if absent, `agent init <identity>` scaffolds `.agent/`.
```

## 2. The convention

Without hooks, the wake is convention: the rule states the loop and Cursor follows it on
every request (the `.mdc` `alwaysApply: true` keeps it in context).

## Notes

- Keep the rule thin; `AGENTS.md` is the source of truth. Cursor reads AGENTS.md directly,
  so this file only reinforces the loop in the Cursor context.