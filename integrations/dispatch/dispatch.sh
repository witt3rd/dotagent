#!/bin/sh
#
# dispatch — the git-event orchestrator, as a post-commit hook.
#
# Every commit that touches the event log triggers a dispatch. No filter on event type:
# inbound, outbound, resolution, handoff — every event is a trigger. The spawned agent
# decides what's actionable. The busy lock serializes dispatches; the self-dispatch
# filter prevents the agent's own commits from re-triggering; rate limiting prevents
# runaway spawning.
#
# Guardrails:
#   1. Self-dispatch prevention: agent-internal commits (message starts with "agent:")
#      are skipped — except handoffs, which are deliberate continuation signals ("chaining").
#   2. Chaining cap: max MAX_CHAIN_DEPTH handoffs before stopping (prevents infinite chains).
#   3. Rate limiting: max MAX_DISPATCHES dispatches per hour (configurable).
#   4. Bounded epoch: the spawned agent is killed after AGENT_TIMEOUT seconds. It is
#      told about the deadline in the prompt and must self-handoff if it can't finish.
#
# Nothing here is required. If this hook is absent, the repo is still an active
# intelligence — a human or agent just pulls on wake instead of being pushed.

set -eu

# --- configuration -----------------------------------------------------------
AGENT_DISPATCH="${AGENT_DISPATCH:-$(git config --get agent.dispatch 2>/dev/null || echo 'opencode run')}"
AGENT_TIMEOUT="${AGENT_TIMEOUT:-300}"          # seconds; 0 = no timeout
MAX_DISPATCHES="${MAX_DISPATCHES:-5}"           # per hour
MAX_CHAIN_DEPTH="${MAX_CHAIN_DEPTH:-5}"        # max handoffs before stopping
LOCK_TTL=86400                                 # reclaim stale locks (seconds)

DISPATCH_PROMPT="${DISPATCH_PROMPT:-You are the caretaker of this repo — an active intelligence (AGENTS.md, repository root). Something happened in the event log — check what is new. You are on a deadline (AGENT_TIMEOUT seconds); if you cannot finish in time, run scripts/agent handoff with state and release the lock. Use ONLY the repo-local CLI scripts/agent (never a bare agent on PATH). 1) Orient: run scripts/agent inbox then scripts/agent state. 2) Act on what is actionable: new inbound -> triage and do the work; a resolution propagating back -> acknowledge; a handoff -> review and resume. 3) Claim events you take (scripts/agent claim id). 4) Record: scripts/agent resolve id reason or scripts/agent reply. 5) Hand off if needed: scripts/agent handoff subject. Keep scripts/agent check clean (exit 0), commit + push. Remove the dispatch lock (rm -rf .agent/.busy) when done.}"

ROOT="$(git rev-parse --show-toplevel 2>/dev/null)" || exit 0
LOG_DIR="$ROOT/.agent/log"
LOCK="$ROOT/.agent/.busy"
COUNT_FILE="$ROOT/.agent/.dispatch_count"

[ -d "$LOG_DIR" ] || exit 0

# --- 1. did this commit touch the event log? ------------------------------------
case "$(git diff-tree --name-only --no-commit-id -r HEAD 2>/dev/null || true)" in
    *".agent/log/"*) ;;
    *) exit 0 ;;
esac

# --- 2. self-dispatch filter: skip agent-internal commits ----------------------
# Agent commits use the "agent:" prefix. Handoffs are the deliberate continuation
# signal — they re-dispatch. All other agent commits (claim, resolve, S events,
# acks) do not re-dispatch.
msg="$(git log -1 --format=%s HEAD 2>/dev/null || true)"
case "$msg" in
    "agent: handoff"*) ;;            # handoff — allow dispatch (continuation)
    "agent:"*)          exit 0 ;;    # other agent commit — don't re-dispatch
esac

# --- 3. chaining guard: max handoff depth ---------------------------------------
# A chain is dispatch -> agent handoffs -> dispatch -> agent handoffs -> ...
# Each H event is a chain link. Cap it to prevent infinite continuation loops.
h_count="$(ls -1 "$LOG_DIR"/H--*.md 2>/dev/null | wc -l)"
if [ "${h_count:-0}" -ge "$MAX_CHAIN_DEPTH" ]; then
    exit 0    # chain too long, stop
fi

# --- 4. rate limiting: max N dispatches per hour --------------------------------
now_hour="$(date +%Y%m%d%H)"
if [ -f "$COUNT_FILE" ]; then
    last_hour="$(sed -n '1p' "$COUNT_FILE")"
    count="$(sed -n '2p' "$COUNT_FILE")"
    if [ "$last_hour" = "$now_hour" ]; then
        if [ "${count:-0}" -ge "$MAX_DISPATCHES" ]; then
            exit 0    # rate limited
        fi
        printf '%s\n%s\n' "$now_hour" "$(( ${count:-0} + 1 ))" > "$COUNT_FILE"
    else
        printf '%s\n1\n' "$now_hour" > "$COUNT_FILE"
    fi
else
    printf '%s\n1\n' "$now_hour" > "$COUNT_FILE"
fi

# --- 5. single-flight busy lock (atomic mkdir), self-healing -------------------
if [ -d "$LOCK" ]; then
    lock_pid="$(cat "$LOCK/pid" 2>/dev/null || echo 0)"
    lock_ts="$(cat "$LOCK/ts" 2>/dev/null || echo 0)"
    alive=0
    [ "$lock_pid" -gt 1 ] && kill -0 "$lock_pid" 2>/dev/null && alive=1
    stale=0
    now="$(date +%s)"
    [ $(( now - lock_ts )) -ge "$LOCK_TTL" ] && stale=1
    if [ "$alive" = 1 ] && [ "$stale" = 0 ]; then
        exit 0    # another agent is mid-flight
    fi
    rm -rf "$LOCK"
fi
if ! mkdir "$LOCK" 2>/dev/null; then
    exit 0        # lost the race
fi
printf '%s\n' "$$" > "$LOCK/pid"
printf '%s\n' "$(date +%s)" > "$LOCK/ts"

# --- 6. spawn a fresh agent, detached, bounded by timeout ----------------------
# setsid detaches from this commit's session so the child survives the parent exit.
# timeout kills the agent if it exceeds AGENT_TIMEOUT. The prompt tells the agent
# about the deadline so it can self-handoff if needed.
export AGENT_DISPATCH DISPATCH_PROMPT ROOT LOCK AGENT_DISPATCH_LOG AGENT_TIMEOUT
setsid sh -c '
    cd "$ROOT"
    run_agent() {
        set -- $AGENT_DISPATCH
        cmd="$1"; shift
        "$cmd" "$@" "$DISPATCH_PROMPT"
    }
    if [ "${AGENT_TIMEOUT:-0}" -gt 0 ] 2>/dev/null; then
        timeout "$AGENT_TIMEOUT" sh -c "$( declare -f run_agent ); run_agent" 2>/dev/null \
            || true    # timeout or failure — lock is still held for self-healing
    else
        run_agent
    fi
    rm -rf "$LOCK"
' >"${AGENT_DISPATCH_LOG:-$ROOT/.agent/.dispatch.log}" 2>&1 </dev/null &

# --- 7. signal the dashboard (FIFO bus) -----------------------------------------
# If a dashboard is running on this host, notify it of the activity. Non-blocking;
# if the FIFO doesn't exist, the dashboard isn't running — exit silently.
BUS="${DOTAGENT_BUS:-/tmp/dotagent-bus}"
[ -p "$BUS" ] && printf '{"repo":"%s","ts":"%s"}\n' "$ROOT" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" >> "$BUS" 2>/dev/null

exit 0