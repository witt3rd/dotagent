# AGENTS.md — <your repo>

> Template for adopting the active-intelligence pattern. Read the real example in
> `../AGENTS.md` and the `skills/agentsmd/` skill for how to author this well. Replace the
> placeholders below; keep the orientation-first structure (goals → merits → concepts →
> mechanisms).

## Goals (the problem this exists to solve)

<What status quo does this repo exist to change? What is the agent expected to uphold?>

## Merits (what is load-bearing, worth protecting)

<The invariants, design intent, and why behind each mechanism — the decision criteria when
two valid approaches exist.>

## Concepts (the principles everything hangs from)

<The organizational principles. An agent that knows a principle can flag a request that
violates it instead of obeying blindly.>

**Span of control (law — see the dotagent charter).** Send to the custodian of
another tree. Do not enter their working tree and act. Authority is not
license to bypass. Peers send; they do not dabble.

## Mechanisms (rules + exact commands)

<Concrete commands, paths, conventions, gotchas, and the testing/PR/git workflow for THIS
repo. Ground truth, not generic advice.>

### The caretaker loop

This repo is agent-operated. On wake, run `agent state` + `agent inbox` to pick up where the
last caretaker left off; on sleep, run `agent handoff` so the next one can. Keep the repo in
the clean end-state (no stale worktrees, mainline at origin tip, primary clone clean) and
the log passing `agent check`.

```
agent state         # where am I in time
agent inbox         # what's waiting on me
agent handoff <subject> [-m BODY]   # hand off on sleep
agent check         # integrity/conformity gate (exit 0/2/3)
```

## Scope and audience

<Universal vs. maintainer-only vs. external-contributor; how identity is verified.>

Last updated: <YYYY-MM-DD>.