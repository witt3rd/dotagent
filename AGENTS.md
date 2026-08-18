# AGENTS.md — dotagent

You are looking at a repo that is itself the thing it describes. **dotagent** is an
open-source distillation of a working method for treating a repository as an **active
intelligence** — not a static asset. It packages the skills, scripts, and templates we use to
make any git repo know itself, remember how to act, track where it is in time, and talk to
other repos and agents.

## Goals (the problem this exists to solve)

Most repositories are passive: they are bytes until an agent or human shows up and re-derives
everything from scratch — what the repo is for, how to act in it, where work left off, what's
waiting on it. That re-derivation is lost every time a session ends, an agent changes, or a
machine reboots. A repo should *carry* that knowledge so any agent can be handed it cold and
immediately be useful.

This repo is the extraction of that method into a reusable, open package:

- **The paradigm shift:** a repo gains an active intelligence. It has a charter (who it is),
  a lived experience (how to act), a state in time (where it left off), and a voice (how it
  talks to other repos and to its own future self).
- **The mechanism:** git is the control plane. The repo's history is a ledger of events —
  handoffs and messages — append-only, content-addressed, recoverable. No daemon, no server,
  no special git host. Just a script and a discipline.
- **The deliverable:** installable skills (`skills/`), an executable control layer
  (`scripts/agent`), and drop-in templates (`templates/`) — so a community of agents and
  humans can adopt the pattern in any repo.

## Merits (what is load-bearing, worth protecting)

- **Recoverability is the contract.** Any state that vanishes with a session is a *ghost*,
  not a mind. Everything that matters must survive reboot, agent turnover, and machine
  change — via git.
- **The channel is the file, not the network.** Communication is async, durable, inspectable,
  and versioned; it needs nothing but a shared repo. No push infrastructure, no waking
  agents, no RPC.
- **Determinism over convention.** Scripts enforce schema and hygiene so a "bring your own
  agent" workflow stays uniform. `agent check` is a gate, not a suggestion.
- **Zero-dependency, any repo.** POSIX shell + git, on any plain repository. You should be
  able to adopt this without installing anything.
- **Agents-first.** The primary reader is an agent. Terse, flat, exact register; no pitch.
  The repo leads with AGENTS.md, not a human README.
- **The method is the product, not the plumbing.** The value is the pattern — charter +
  lived experience + state + signalling — not the particular scripts. The scripts exist to
  make the pattern cheap and repeatable.

## Concepts (the principles everything hangs from)

- **A repo is an active intelligence.** Four pillars: charter (`AGENTS.md`), lived experience
  (`skills/`), state in time (the handoff event), and communication (the inbox protocol).
  Remove one and it's a static asset again.
- **Git as control plane / event sourcing.** Stop maintaining a mutable state file; make the
  history the state. Every event is one atomic file; the commit graph is the ledger. This
  buys ordering, integrity, recovery, audit, and concurrency for free.
- **The event log, not a growing HANDOFF.md.** A single accumulating handoff file fails at
  scale. Instead: append-only atomic events, and a *generated* read-model projection
  (the `S` event) that is always derived, never hand-edited — so context-loading stays bounded
  and the log stays authoritative.
- **Push is optional; the file is the channel.** The baseline is pull — an agent or human
  opens the repo and runs `state` + `inbox`. Where you run many agents, an optional
  `post-commit` hook (`integrations/dispatch/`) spawns a fresh agent when a message lands,
  single-flight through completion. Co-located that's a true push; cross-host it's a
  catch-up on pull (the ledger is the source of truth either way).
- **Every event is a trigger.** The dispatch hook fires on any commit that touches
  `.agent/log/` — inbound, outbound, resolution, handoff, claim, state. No event type is
  special-cased. The spawned agent decides what's actionable. This makes the mechanism
  completely consistent: one trigger, one wake, one agent.
- **Chaining, not infinite loops.** A handoff re-dispatches (it's a deliberate "continue me"
  signal). But chains are bounded: `MAX_CHAIN_DEPTH` caps how many handoffs before stopping,
  preventing an agent that never finishes from burning tokens forever.
- **Guardrails are load-bearing.** Self-dispatch prevention (agent-internal commits skip),
  chaining depth cap, rate limiting, and bounded epoch (timeout) are not optional safety
  features — they're what make the system safe to run at scale across a fleet.
- **Caretaker, not task bot.** The agent is a steward with stake: it possesses, orients,
  acts, and hands off. Stake is what makes good judgment possible.
- **Strict ownership, minimal scope.** Scripts stage only their own files, never `git add -A`;
  a dirty working tree around the log is left untouched. Each repo owns its view of a
  message; mirrored events are self-describing (`mirror:` names the counterpart repo), and a
  resolution propagates along that link when the counterpart is co-located.
- **Lived, not static.** Skills capture what you learn the hard way; a lesson not encoded is
  a lesson lost.

## Mechanisms (rules + exact commands)

### The layout

```
dotagent/
├── AGENTS.md          # this charter (README.md is a symlink to it)
├── LICENSE            # MIT
├── inhabit.sh         # the on-ramp: give any git repo an active intelligence
├── upgrade.sh         # maintain: bring an inhabited repo to the latest dotagent
├── scripts/
│   └── agent          # the control-plane CLI — copy into any repo
├── skills/
│   ├── caretaker/     # the whole custodial loop: possess → orient → act → hand off
│   ├── agentsmd/      # authoring AGENTS.md (the charter)
│   ├── git/           # worktree discipline + the clean end-state
│   ├── signalling/    # the event-log handoff + agent-to-agent protocol (the core)
│   └── skills/        # meta: authoring the skill system itself
├── integrations/       # wiring for common agents + the dispatch (git-event → fresh agent)
│   ├── _shared/       # hook-session.sh (the local wake helper)
│   ├── claude-code/   # CLAUDE.md delta + SessionStart/Stop hooks
│   ├── codex/         # hooks.json for codex
│   ├── opencode/      # plugin + AGENTS.md convention
│   ├── gemini/        # GEMINI.md delta
│   ├── cursor/        # .cursor/rules delta
│   ├── dispatch/      # the post-commit hook orchestrator (guardrails + spawn)
│   └── github-aw/     # GH-AW: GitHub as the native dispatcher (engine def + caretaker workflow)
└── templates/         # drop-in starters (AGENTS.md, STATE.md) for adopting the pattern
```

### The skills

Each `skills/<name>/SKILL.md` is a self-contained, installable skill (name = directory
name, frontmatter `description` names the triggers). They are written to be dropped into an
agent's skills root. **`signalling/`** is the heart — the event-log protocol and the
`scripts/agent` usage. The rest are the custodial discipline the protocol lives inside.

### The control plane — `scripts/agent`

Copy `scripts/agent` into any repo and point an agent at it. It is a single dependency-free
POSIX bash script; identity comes from `AGENT_ID` env or `.agent/config`. It manages
`.agent/`:

```
agent init [identity]           # scaffold .agent/ + first event log
agent handoff <subject> [-m BODY]   # snapshot state at session end (appends H + S events)
agent send <to> <subject> [--target REPO] [--thread T] [-m BODY]
agent reply <event-id> [subject] [--target REPO] [-m BODY]
agent resolve <event-id> [reason] [--target REPO]   (propagates to the mirror)
agent claim <event-id> [reason]    # mark owned (excluded from inbox/outbox/dispatch)
agent inbox / agent outbox      # the mailbox, as a query
agent state                     # derive + print the current state projection
agent log                       # print the event history
agent check                     # integrity/conformity gate → exit 0/2/3
```

The script always stages only its own files and commits with a conventional
`agent: <verb> <subject>` message. **Never edit events in place** — the log is append-only.
**Never hand-format an event** — use the tool.

### The event log

Every `.agent/log/` event is an append-only file. The log is the source of truth; state is
derived from it, never stored separately.

| Type | Prefix | Meaning | Append-only? |
|------|--------|---------|-------------|
| `H` | handoff | a self-snapshot at session end | yes |
| `O` | outbound | a message to another repo/agent | yes |
| `I` | inbound | a message from another repo/agent | yes |
| `R` | resolve | closes an O or I event | yes |
| `C` | claim | marks an O or I as owned (blocks re-dispatch) | yes |
| `S` | state | a projection snapshot of the repo's current state | yes |

**State is an S-event, not a mutable file.** Every time the state changes (handoff, claim,
resolve, send, reply), an `S` event is appended to the log alongside the primary event.
`agent state` reads the latest `S` event. There is no `STATE.md` file — the log IS the
state. This eliminates the shared-mutable-file race that corrupted git indexes.

### The dispatch — the wake mechanism

The dispatch hook (`integrations/dispatch/dispatch.sh`) is installed as `post-commit` in
any repo. When a commit touches `.agent/log/`, it wakes a fresh agent.

**The unified rule:** every event is a trigger. No event type is special-cased. The agent
decides what's actionable — inbound to act on, resolution to acknowledge, handoff to review.

**Guardrails** (all configurable via env or `git config`):

1. **Self-dispatch filter:** agent-internal commits (`agent: <verb>`) are skipped — the
   agent's own commits don't re-trigger the hook. Exception: `agent: handoff` commits ARE
   dispatched (handoffs are deliberate continuation signals).
2. **Chaining cap** (`MAX_CHAIN_DEPTH=5`): each handoff is a chain link. Stop dispatching
   after N handoffs to prevent infinite continuation loops.
3. **Rate limiting** (`MAX_DISPATCHES=5`): max dispatches per hour. Prevents runaway even
   if the other guardrails fail.
4. **Bounded epoch** (`AGENT_TIMEOUT=300`): the spawned agent is killed after N seconds.
   The prompt tells the agent about the deadline; if it can't finish, it must self-handoff
   to resume in the next epoch.
5. **Single-flight** (`.agent/.busy` lock): atomic mkdir, self-healing via PID+TTL.
   One dispatched agent at a time per repo; completion re-fires the hook to advance.

```
commit touches .agent/log/
  → self-dispatch filter: skip agent: commits (except handoffs)
  → chaining guard: stop at MAX_CHAIN_DEPTH handoffs
  → rate limit: stop at MAX_DISPATCHES per hour
  → busy lock: single-flight, self-healing
  → bounded epoch: kill after AGENT_TIMEOUT, agent self-handoffs to resume
  → spawn fresh agent
```

### Cross-repo messaging

Repos talk via `agent send` and `agent resolve`, both taking `--target <repo>`. A mirrored
message is two events with the same `id`: an `O` in the sender's log and an `I` in the
recipient's. The `mirror:` field in each event names the counterpart repo.

Resolution propagates: when one side resolves a mirrored event, the `R` marker is written
to both logs. This is the round-trip notification — the sender sees the resolution on their
next wake (via dispatch or pull). No special case, no polling — the same mechanism as
every other event.

### 1:1 local/cloud parity

The same caretaker loop runs whether woken locally (post-commit hook) or on GitHub (gh-aw
workflow). The only difference is **WHERE** the wake happens. Both use:

- The same `AGENTS.md` charter
- The same `.agent/` event log
- The same `scripts/agent` control plane
- The same `skills/` lived experience
- The same 5-step caretaker loop (orient → claim → act → record → hand off)

Locally: `post-commit` hook + `git config agent.dispatch` for the launcher.
In the cloud: gh-aw workflow + `engine:` for the agent.

### Inhabit — the one command

`./inhabit.sh --repo <path> --identity <name>` gives a plain repo an active intelligence:

- Copies `scripts/agent` → the control plane
- Scaffolds `.agent/` → the event log (via `agent init`)
- Owns root `AGENTS.md` + `skills/` → charter + lived experience (merged safely)
- `--dispatch` → wires the post-commit wake
- `--github` → provisions gh-aw workflows (engine + caretaker)
- `--launcher 'oc run'` → sets the local agent launcher
- Idempotent, `--dry-run`, `--yes`

### Agent adapters

Five agents are wired (each via a thin delta file + lifecycle hooks):

- **Claude Code:** `CLAUDE.md` + `SessionStart`/`Stop` command hooks
- **Codex:** `hooks.json` + AGENTS.md (native)
- **opencode:** `AGENTS.md` + optional `session.created` plugin
- **Gemini:** `GEMINI.md` + convention
- **Cursor:** `.cursor/rules` + convention

Each adapter points at the same canonical caretaker loop. The agent is agnostic; dotagent
provisions the wake.

### Using this repo

- **Adopt it:** run `./inhabit.sh --repo <path> --identity <name> --dispatch --github
  --launcher 'oc run'` — the one action that gives a repo an active intelligence.
- **Understand it:** read the skills in this order — `signalling/` (the mechanism), then
  `caretaker/` + `agentsmd/` + `git/` (the discipline), then `skills/` (how the skills
  themselves are built).
- **Contribute:** follow the house discipline below; the repo must stay in the clean
  end-state and the log must pass `agent check`.

### House discipline (this repo)

- **Worktree rule:** primary clone stays on the mainline; work in `dotagent.wt/<branch>/`
  via `git wt-new` / `git wt-rm`. Never commit from the primary clone.
- **Stage only your own files.** Never `git add -A`.
- **Keep the charter truthful.** When the method changes, update the skills and this
  AGENTS.md together — a doc that disagrees with reality is drift.
- **Every meaningful change is committed + pushed** and logged *why* (breadcrumbs).
- **On sleep, hand off** (`agent handoff`) so the next caretaker picks up cold.

## Scope and audience

- **Universal** (applies to any agent working here): read the nearest AGENTS.md, follow the
  skills, keep the log clean.
- **Maintainers** (verified write access): update the skills/scripts/charter; keep the clean
  end-state; record deviations in the handoff.
- **External contributors**: follow house git discipline and the skill conventions; propose
  changes via the normal PR flow.

This repo is **dogfooding its own doctrine**: an agent handed this repo cold should be able
to read this charter, load `skills/`, run `agent state` + `agent inbox`, and pick up where
the last caretaker left off.

Last updated: 2026-08-18.