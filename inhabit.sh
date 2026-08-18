#!/usr/bin/env bash
#
# inhabit — give a plain repo an active intelligence.
#
# "Installing" dotagent is really inhabiting: the moment a repo stops being a static
# collection of bytes and gains a mind — a charter (AGENTS.md), a ledger (.agent/), a
# voice (the signalling layer), and a wake (the dispatch hook). This script performs that
# moment. It is the on-ramp: run it once against a repo and it is alive; any agent handed
# it cold can `agent state` + `agent inbox` and pick up where the last caretaker left off.
#
# What it does (idempotent — safe to re-run; it never clobbers existing work):
#   1. copies the control plane  -> <repo>/scripts/agent
#   2. scaffolds the ledger       -> <repo>/.agent/  (via `agent init`)
#   3. keeps runtime state out    -> .gitignore (.agent/.busy, .dispatch.log)
#   4. [--dispatch]   wires the wake  -> .git/hooks/post-commit (git event -> fresh agent)
#   5. [--launcher]   picks the spawn  -> git config agent.dispatch (e.g. 'oc run')
#   6. [--agents]     writes the charter -> <repo>/AGENTS.md (only if none exists)
#
# Usage:
#   inhabit.sh [--repo PATH] [--identity NAME] [--dispatch] [--launcher 'CMD ARGS']
#              [--agents] [--yes] [--dry-run]
#
#   inhabit.sh --repo ~/src/thing --identity thing --dispatch --launcher 'oc run' --agents
#
# The defaults are the lean core (1-3); the wake (4-5) and charter (6) are the batteries
# you opt into. --dry-run shows what it would do without touching anything.

set -euo pipefail

DOTAGENT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

usage() {
    sed -n '1,28p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
    echo
    echo "  --repo PATH        target repo (default: current dir)"
    echo "  --identity NAME    the repo's agent identity (default: repo name)"
    echo "  --dispatch         wire the post-commit wake (git event -> fresh agent)"
    echo "  --launcher 'CMD'   spawn command for the wake (e.g. 'oc run', 'oc-work run')"
    echo "  --agents           write templates/AGENTS.md as the charter if none exists"
    echo "  --yes / -y         don't prompt; take every enabled action"
    echo "  --dry-run          print what would happen, change nothing"
}

REPO=""; IDENTITY=""; DISPATCH=0; LAUNCHER=""; AGENTS=0; YES=0; DRY=0
while [ $# -gt 0 ]; do
    case "$1" in
        --repo)     REPO="$2"; shift 2 ;;
        --identity) IDENTITY="$2"; shift 2 ;;
        --dispatch) DISPATCH=1; shift ;;
        --launcher) LAUNCHER="$2"; shift 2 ;;
        --agents)   AGENTS=1; shift ;;
        --yes|-y)   YES=1; shift ;;
        --dry-run)  DRY=1; shift ;;
        -h|--help)  usage; exit 0 ;;
        -*) echo "inhabit: unknown option $1" >&2; usage >&2; exit 2 ;;
        *)  REPO="$1"; shift ;;
    esac
done

REPO="${REPO:-$PWD}"
REPO="$(cd "$REPO" 2>/dev/null && git rev-parse --show-toplevel 2>/dev/null || echo "$REPO")"
[ -d "$REPO/.git" ] || { echo "inhabit: $REPO is not a git repo" >&2; exit 2; }

confirm() { # $1 = description; only prompts when interactive and not --yes
    [ "$YES" = 1 ] && return 0
    printf '%s [y/N] ' "$1"; read -r ans
    case "$ans" in y|Y|yes) return 0;; *) return 1;; esac
}

doit() { # $@ = command; runs it, or echoes it under --dry-run
    if [ "$DRY" = 1 ]; then printf '  would: %s\n' "$*"; else "$@"; fi
}

IDENTITY="${IDENTITY:-$(basename "$REPO")}"

echo "== inhabiting $REPO (as '$IDENTITY') =="

# --- 1. control plane -------------------------------------------------------
if [ -x "$REPO/scripts/agent" ]; then
    echo "  scripts/agent: present (keeping)"
else
    doit mkdir -p "$REPO/scripts"
    doit cp "$DOTAGENT/scripts/agent" "$REPO/scripts/agent"
    doit chmod +x "$REPO/scripts/agent"
    echo "  scripts/agent: copied"
fi

# --- 2. ledger --------------------------------------------------------------
if [ -d "$REPO/.agent/log" ]; then
    echo "  .agent/: already inhabited (keeping)"
else
    doit bash "$REPO/scripts/agent" init "$IDENTITY"
    echo "  .agent/: scaffolded"
fi

# --- 3. runtime state out of the tree ---------------------------------------
GITIGNORE="$REPO/.gitignore"
if [ -f "$GITIGNORE" ] && grep -q '^\.agent/\.busy' "$GITIGNORE" 2>/dev/null; then
    echo "  .gitignore: already set"
else
    # append (or create) only the control-plane runtime lines
    {
        [ -f "$GITIGNORE" ] && [ -s "$GITIGNORE" ] && [ "$(tail -c1 "$GITIGNORE" | od -An -c | tr -d ' ')" != '\n' ] && echo
        echo ".agent/.busy/"
        echo ".agent/.dispatch.log"
    } >/tmp/inhabit-gi.$$; cat "$GITIGNORE" >>/tmp/inhabit-gi.$$ 2>/dev/null || true
    doit mv /tmp/inhabit-gi.$$ "$GITIGNORE"
    echo "  .gitignore: appended control-plane runtime state"
fi

# --- 4. wake (opt-in) -------------------------------------------------------
if [ "$DISPATCH" = 1 ]; then
    HOOK_DIR="$REPO/.agent/hooks"; HOOK="$HOOK_DIR/post-commit"
    if [ -e "$REPO/.git/hooks/post-commit" ]; then
        echo "  dispatch: .git/hooks/post-commit already exists (leaving it)"
    elif confirm "wire the post-commit wake (git event -> fresh agent)?"; then
        doit mkdir -p "$HOOK_DIR"
        doit cp "$DOTAGENT/integrations/dispatch/dispatch.sh" "$HOOK"
        doit chmod +x "$HOOK"
        doit ln -s "$HOOK" "$REPO/.git/hooks/post-commit"
        echo "  dispatch: wired (.git/hooks/post-commit)"
    else
        echo "  dispatch: skipped"
    fi
fi

# --- 5. launcher (opt-in) ---------------------------------------------------
if [ -n "$LAUNCHER" ]; then
    if confirm "set agent.dispatch launcher to '$LAUNCHER'?"; then
        doit git -C "$REPO" config agent.dispatch "$LAUNCHER"
        echo "  launcher: agent.dispatch = $LAUNCHER"
    else
        echo "  launcher: skipped"
    fi
fi

# --- 6. charter (opt-in) ----------------------------------------------------
if [ "$AGENTS" = 1 ]; then
    if [ -f "$REPO/AGENTS.md" ]; then
        echo "  AGENTS.md: exists (leaving it)"
    elif confirm "write a starter AGENTS.md charter (won't touch an existing one)?"; then
        doit cp "$DOTAGENT/templates/AGENTS.md" "$REPO/AGENTS.md"
        echo "  AGENTS.md: starter charter written — edit the <placeholders>"
    else
        echo "  AGENTS.md: skipped"
    fi
fi

# --- 7. commit the scaffold (leave it clean + recoverable) -------------------
FILES="scripts/agent .gitignore"
[ "$AGENTS" = 1 ] && [ -f "$REPO/AGENTS.md" ] && FILES="$FILES AGENTS.md"
[ "$DISPATCH" = 1 ] && FILES="$FILES .agent/hooks"
if [ -n "$(git -C "$REPO" status --porcelain $FILES 2>/dev/null)" ]; then
    if confirm "commit the scaffold (leaves the repo clean + recoverable)?"; then
        doit git -C "$REPO" add -- $FILES
        doit git -C "$REPO" commit -q -m "inhabit: $IDENTITY becomes an active intelligence"
        echo "  scaffold committed"
    else
        echo "  scaffold left uncommitted (commit it when ready)"
    fi
fi

echo
echo "== $REPO is inhabited. An agent handed it cold can: =="
echo "   agent state    # where it is in time"
echo "   agent inbox    # what's waiting on it"
echo "   agent handoff  # hand off on sleep"
[ -n "$LAUNCHER" ] || echo "Next: git config agent.dispatch 'oc run'   # (or oc-work run) — your launcher."