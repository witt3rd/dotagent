#!/usr/bin/env bash
#
# upgrade — bring an inhabited repo to the latest dotagent.
#
# inhabit establishes; upgrade maintains. When dotagent evolves (new guardrails, new event
# types, STATE.md removal), this script copies the latest tooling into an already-inhabited
# repo. Idempotent — safe to re-run; it only updates what's changed.
#
# Usage:
#   upgrade.sh [--repo PATH] [--yes] [--dry-run]

set -euo pipefail

DOTAGENT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

usage() {
    sed -n '1,12p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
    echo
    echo "  --repo PATH   target repo (default: current dir)"
    echo "  --yes / -y     don't prompt"
    echo "  --dry-run      show what would change, change nothing"
}

REPO=""; YES=0; DRY=0
while [ $# -gt 0 ]; do
    case "$1" in
        --repo)     REPO="$2"; shift 2 ;;
        --yes|-y)   YES=1; shift ;;
        --dry-run)  DRY=1; shift ;;
        -h|--help)  usage; exit 0 ;;
        -*) echo "upgrade: unknown option $1" >&2; usage >&2; exit 2 ;;
        *)  REPO="$1"; shift ;;
    esac
done

REPO="${REPO:-$PWD}"
REPO="$(cd "$REPO" 2>/dev/null && git rev-parse --show-toplevel 2>/dev/null || echo "$REPO")"
[ -d "$REPO/.git" ] || { echo "upgrade: $REPO is not a git repo" >&2; exit 2; }
[ -d "$REPO/.agent" ] || { echo "upgrade: $REPO is not inhabited (no .agent/)" >&2; exit 2; }

confirm() { [ "$DRY" = 1 ] && return 0; [ "$YES" = 1 ] && return 0; printf '%s [y/N] ' "$1"; read -r ans; case "$ans" in y|Y|yes) return 0;; *) return 1;; esac; }
doit() { if [ "$DRY" = 1 ]; then printf '  would: %s\n' "$*"; else "$@"; fi; }

echo "== upgrading $REPO to latest dotagent =="

# 1. CLI
if diff -q "$DOTAGENT/scripts/agent" "$REPO/scripts/agent" >/dev/null 2>&1; then
    echo "  scripts/agent: up to date"
else
    doit cp "$DOTAGENT/scripts/agent" "$REPO/scripts/agent"
    doit chmod +x "$REPO/scripts/agent"
    echo "  scripts/agent: updated"
fi

# 2. Dispatch hook
HOOK_SRC="$DOTAGENT/integrations/dispatch/dispatch.sh"
HOOK_DST="$REPO/.agent/hooks/post-commit"
if [ -f "$HOOK_DST" ] && diff -q "$HOOK_SRC" "$HOOK_DST" >/dev/null 2>&1; then
    echo "  dispatch hook: up to date"
else
    [ -d "$REPO/.agent/hooks" ] || doit mkdir -p "$REPO/.agent/hooks"
    doit cp "$HOOK_SRC" "$HOOK_DST"
    doit chmod +x "$HOOK_DST"
    echo "  dispatch hook: updated"
fi

# 3. Clean deprecated artifacts
if [ -f "$REPO/.agent/STATE.md" ] || git -C "$REPO" ls-files .agent/STATE.md 2>/dev/null | grep -q .; then
    doit git -C "$REPO" rm -f .agent/STATE.md 2>/dev/null || true
    doit rm -f "$REPO/.agent/STATE.md"
    echo "  STATE.md: removed (S-event migration)"
else
    echo "  STATE.md: already absent"
fi

if [ -f "$REPO/.agent/HANDOFF.md" ] || git -C "$REPO" ls-files .agent/HANDOFF.md 2>/dev/null | grep -q .; then
    doit git -C "$REPO" rm -f .agent/HANDOFF.md 2>/dev/null || true
    doit rm -f "$REPO/.agent/HANDOFF.md"
    echo "  HANDOFF.md: removed (event log replaces growing files)"
else
    echo "  HANDOFF.md: already absent"
fi

# 4. .gitignore
GITIGNORE="$REPO/.gitignore"
if [ -f "$GITIGNORE" ] && grep -q '.dispatch.log' "$GITIGNORE" 2>/dev/null; then
    echo "  .gitignore: already set"
else
    { [ -f "$GITIGNORE" ] && [ -s "$GITIGNORE" ] && printf '\n'; printf '.agent/.dispatch.log\n'; cat "$GITIGNORE" >>/tmp/upgrade-gi.$$ 2>/dev/null || true; } >/dev/null
    # simpler: just append if missing
    grep -q '.dispatch.log' "$GITIGNORE" 2>/dev/null || echo '.agent/.dispatch.log' >> "$GITIGNORE"
    echo "  .gitignore: updated"
fi

# 5. Skills merge (idempotent)
if [ -d "$DOTAGENT/skills" ]; then
    doit cp -rn "$DOTAGENT/skills"/. "$REPO/skills/"
    echo "  skills/: merged"
fi

# 6. Commit
echo
echo "== upgrade complete. Verify with: =="
echo "   $(dirname "$0")/scripts/agent -C '$REPO' check"