# Dispatch — the git-event orchestrator (the "push" half)

The hybrid wake: when a commit carries a new message into the event log, a `post-commit`
hook **spawns a fresh agent** to pull it. It answers the "which of my dozens of agents is
woken, and when" question centrally: **the hook fires on a real event, the spawned agent
owns the work, and the lock guarantees one at a time.**

```
commit touches .agent/log  →  post-commit hook  →  single-flight lock  →  spawn agent  →  pulls
```

**Nothing here is required.** The baseline is manual: a human or agent opens the repo and
runs `agent state` + `agent inbox`. This is the optional, co-located dispatch layer for
teams that want work *pushed* to a fresh instance instead of waiting for one to arrive.

## Install (co-located, same host)

The hook is `integrations/dispatch/dispatch.sh`. Wire it as `post-commit` (per repo, or via
`git config core.hooksPath`):

```bash
# per repo
mkdir -p .git/hooks
ln -s "$PWD/integrations/dispatch/dispatch.sh" .git/hooks/post-commit
# or globally, then enable in the repo:
git config core.hooksPath .githooks
ln -s "$PWD/integrations/dispatch/dispatch.sh" .githooks/post-commit
```

Configure the spawn command via env in the hook (or `export` before committing):

```bash
AGENT_DISPATCH='opencode run'        # what to spawn; defaults to `opencode run`
DISPATCH_PROMPT='...'                # the bounded prompt for the fresh agent
AGENT_DISPATCH_LOG=/var/log/agent-dispatch.log   # where the spawned agent's output goes
```

Defaults: spawn `opencode run` with a caretaker prompt that tells it to pull `agent
inbox`/`agent state`, act, resolve, and release the lock. `opencode run` must be on the
hook's PATH — set it in the hook if it isn't.

## How it decides to spawn (three filters)

1. **The commit touched `.agent/log/`.** Feature commits never dispatch — only event-log
   commits.
2. **There is *unclaimed inbound* work.** An `I` event with no resolve marker and no claim
   (`C` marker). Handoffs, outbound, and already-handled or already-owned threads never
   spawn.
3. **No agent is already working.** The single-flight lock (below).

Before spawning, dispatch **claims** each unclaimed inbound it's about to hand off (`agent
claim <id>`), so the work is marked owned and won't be re-dispatched if the spawned agent
dies before resolving. The claim drops the event out of `inbox`/`outbox` and out of future
dispatch — the "owned but not yet closed" window.

## Single-flight: one event at a time, through completion

The dispatcher takes a **busy lock** (`.agent/.busy`) before spawning. The spawned agent
holds it and releases it when it finishes (its prompt says so). Because the agent's own
resolve/handoff commit re-fires the hook, completion **advances the queue**:

```
event A → lock → spawn → resolve → release → (its commit fires hook) → event B → …
```

One in flight, strictly serialized, **no daemon** — the completion commit is what picks up
the next event. Self-healing: the lock stores the spawner's PID + timestamp; a dead or
stale (older than `AGENT_LOCK_TTL`, default 1 day) lock is reclaimed instead of wedging the
queue. Manual recovery, if ever needed: `rm -rf .agent/.busy`.

## Cross-host — a catch-up scenario

`post-commit` fires only on **local** commits, never on `git pull`/`fetch`. When agent A on
host X commits into B's repo on host Y, B's awareness arrives with the pull — so on Y it is
always **catch-up**, one of three ways (in increasing immediacy):

- **post-merge / post-fetch** — install the same script as `post-merge` (or wrap a
  `git fetch` + run in a polling watcher) to dispatch on pull. Closest to push, still no
  daemon.
- **a polling watcher** — the daemon case: `git fetch` + dispatch on a timer (true
  cross-host push).
- **next-session pull** — the plain model: B pulls on wake and processes whatever caught
  up. No new machinery.

The git ledger is the single source of truth in all of them; *pull is the notification* on
the receiving host.

## Design notes / gotchas

- **The hook spawns asynchronously** via `setsid`, detached from the commit's session and
  stdio, so the *commit returns immediately* — it never blocks on the spawned agent. A
  plain `(...) &` is SIGHUP-killed on hook exit; `setsid` survives.
- **The spawned command is word-split, never `eval`'d** — the prompt is passed as a single
  argument, so shell metacharacters (parentheses, etc.) in it are safe.
- **`AGENT_DISPATCH` must be a simple command line** (command + fixed args, e.g.
  `opencode run`). Complex quoting in it is not supported — that's the `eval` trap.
- **Loop safety**: the spawned agent's own commits re-fire the hook, but they're not "new
  unclaimed inbound," so there's no infinite re-dispatch.
- **Each repo owns its dispatcher.** Install it per repo; the hook is stateless and reads
  the event log as its only source of truth.

## Relationship to the other integrations

- `_shared/hook-session.sh` wakes the *same* agent in an existing session (convention +
  log). This dispatch **spawns a fresh one** on a git event. They're complementary: session
  wake for the interactive agent, dispatch for pushing work to a new instance.
- The manual mode (`agent inbox` + `agent resolve`) is always the baseline; dispatch layers
  on top of it and uses exactly the same commands.