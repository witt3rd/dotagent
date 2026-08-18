#!/bin/sh
# hook-session — the "wake up to do work" lifecycle hook for an active-intelligence repo.
#
# Registered through an agent's hook mechanism (Claude Code and Codex both take command
# hooks; see the integration notes). It surfaces the repo's current state and inbox at
# session start so the agent picks up cold without relying on the model remembering the
# doctrine, and reminds it to hand off at session end.
#
#   hook-session.sh session     # start: print STATE + inbox
#   hook-session.sh stop        # end: remind to hand off
#
# The hook locates the repo's `agent` CLI (repo-local scripts/agent first, then PATH) and
# calls it against the repo root. It is a best-effort helper: if there's no git repo or no
# `agent` CLI, it exits quietly (exit 0) rather than disturbing the session.

set -eu

action="${1:-session}"

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
[ -n "$ROOT" ] || exit 0

# Prefer the repo-local copy, then a globally-installed `agent` on PATH.
if [ -x "$ROOT/scripts/agent" ]; then
    A="$ROOT/scripts/agent"
elif command -v agent >/dev/null 2>&1; then
    A="agent"
else
    exit 0
fi

case "$action" in
    session)
        echo "── [agent] repo state ──────────────────────────"
        ( cd "$ROOT" && "$A" state 2>/dev/null ) || true
        echo "── [agent] inbox ───────────────────────────────"
        ( cd "$ROOT" && "$A" inbox 2>/dev/null ) || true
        echo "─────────────────────────────────────────────────"
        ;;
    stop)
        echo "[agent] session ended — run \`agent handoff '<what changed>'\` to leave this repo pick-up-able for the next caretaker."
        ;;
esac
exit 0