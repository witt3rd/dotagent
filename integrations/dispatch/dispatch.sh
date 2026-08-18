#!/bin/sh
#
# dispatch — the git-event orchestrator, as a post-commit hook.
#
# When a commit touches the repo's event log, this checks whether it created work that
# needs an agent, acquires a single-flight lock, and spawns a fresh agent to do it. It is
# the "push" half of the hybrid: the git commit is the signal, this hook is the
# dispatcher, and the spawned agent pulls (`agent state`, `agent inbox`).
#
# Nothing here is required. If this hook is absent, the repo is still an active
# intelligence — a human or agent just pulls on wake instead of being pushed a fresh
# instance. This is the optional, co-located dispatch layer.
#
# Semantics
#   - Only `.agent/log/` commits are considered (feature commits never dispatch).
#   - Only *unclaimed inbound* work dispatches (a new I event with no resolve/claim).
#   - Single-flight: if a dispatched agent is still working (the busy lock is live), we
#     skip — its completion commit will re-fire this hook and advance the queue. One event
#     at a time, through completion.
#   - Self-healing: the lock stores spawner PID + timestamp; a dead or stale lock is
#     reclaimed so a crashed agent can't wedge the queue.
#
# Install (per repo, or via core.hooksPath):
#   ln -s "$PWD/integrations/dispatch/dispatch.sh" .git/hooks/post-commit
# Configure the spawn command via env in the hook (or in AGENT_DISPATCH below).
#
# Cross-host: post-commit fires only on LOCAL commits. A pulled-in event (agent A on host
# X → B on host Y) arrives via fetch and is a catch-up: install this as post-merge, wrap
# it in a polling watcher, or let B pull on wake. See README.md.

set -eu

# --- what to spawn (override with AGENT_DISPATCH, or edit here) ------------------------
# Default: a headless opencode run with a bounded prompt, in the repo, detached.
AGENT_DISPATCH="${AGENT_DISPATCH:-opencode run}"
# The prompt handed to the fresh agent. It pulls state/inbox, acts, releases the lock.
# Passed to the spawned command as a SINGLE argument (never eval'd), so shell
# metacharacters in it are safe.
DISPATCH_PROMPT="${DISPATCH_PROMPT:-Inbound mail needs a caretaker. Run the agent commands (agent inbox then agent state), triage what is actionable, resolve what you close, and release the dispatch lock.}"

ROOT="$(git rev-parse --show-toplevel 2>/dev/null)" || exit 0
LOG_DIR="$ROOT/.agent/log"
LOCK="$ROOT/.agent/.busy"
LOCK_TTL=86400            # reclaim a lock older than this (seconds)
AGENT_LOCK_TTL="${AGENT_LOCK_TTL:-$LOCK_TTL}"

# --- 1. did this commit touch the event log? ------------------------------------------
case "$(git diff-tree --name-only --no-commit-id -r HEAD 2>/dev/null || true)" in
    *".agent/log/"*) ;;
    *) exit 0 ;;          # not a log event — never dispatch
esac

[ -d "$LOG_DIR" ] || exit 0

# --- 2. is there unclaimed inbound work? ----------------------------------------------
# An inbound event with no resolve marker and no active claim. Empty → nothing to do.
has_work() {
    for f in "$LOG_DIR"/I--*.md; do
        [ -f "$f" ] || continue
        id="$(sed -n 's/^id:[[:space:]]*//p' "$f" | head -1)"
        claimed=""
        for r in "$LOG_DIR"/R--*.md "$LOG_DIR"/C--*.md; do
            [ -f "$r" ] || continue
            [ "$(sed -n 's/^re:[[:space:]]*//p' "$r" | head -1)" = "$id" ] && claimed=1
        done
        [ -z "$claimed" ] && return 0
    done
    return 1
}
has_work || exit 0

# --- 3. single-flight busy lock (atomic mkdir), self-healing --------------------------
if [ -d "$LOCK" ]; then
    lock_pid="$(cat "$LOCK/pid" 2>/dev/null || echo 0)"
    lock_ts="$(cat "$LOCK/ts" 2>/dev/null || echo 0)"
    alive=0
    [ "$lock_pid" -gt 1 ] && kill -0 "$lock_pid" 2>/dev/null && alive=1
    stale=0
    now="$(date +%s)"
    [ $(( now - lock_ts )) -ge "$AGENT_LOCK_TTL" ] && stale=1
    # Reclaim only if the worker is gone or hopelessly stale; otherwise another agent is
    # mid-flight and will advance the queue on its own completion commit.
    if [ "$alive" = 1 ] && [ "$stale" = 0 ]; then
        exit 0
    fi
    rm -rf "$LOCK"
fi
if ! mkdir "$LOCK" 2>/dev/null; then
    exit 0                # lost the race — another dispatcher holds the lock
fi
printf '%s\n' "$$" > "$LOCK/pid"
printf '%s\n' "$(date +%s)" > "$LOCK/ts"

# --- 4. spawn a fresh agent, detached, so the commit returns immediately --------------
# setsid detaches from this commit's session so the child survives the parent exit
# (a plain background subshell is SIGHUP-killed mid-body — it loses the race to rm the
# lock and never reliably writes its log). Release the lock in the child on success.
# The command is WORD-SPLIT, never eval'd — the prompt must not be re-parsed (it may
# contain shell metacharacters like parentheses).
export AGENT_DISPATCH DISPATCH_PROMPT ROOT LOCK AGENT_DISPATCH_LOG
setsid sh -c '
    cd "$ROOT"
    set -- $AGENT_DISPATCH
    cmd="$1"; shift
    if "$cmd" "$@" "$DISPATCH_PROMPT"; then
        rm -rf "$LOCK"
    fi
' >"${AGENT_DISPATCH_LOG:-$ROOT/.agent/.dispatch.log}" 2>&1 </dev/null &

exit 0