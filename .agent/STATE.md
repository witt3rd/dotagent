# State

- Updated: 2026-08-18T08:09:42Z
- Identity: dotagent
- Repo: /home/dt/src/witt3rd/dotagent

## Latest handoff
- 2026-08-18T07:51:23Z — hybrid wake: git-event dispatch as a post-commit hook
Added integrations/dispatch/ — the 'push' half. post-commit hook checks (1) commit touched .agent/log, (2) unclaimed inbound, (3) single-flight .agent/.busy lock (PID+TTL self-healing), then setsid-spawns a fresh agent. Word-splits the command, never evals the prompt (found+fixed a real eval bug with parens). Verified: inbound-HEAD dispatches+releases; live lock skips; stale lock reclaims. Cross-host = catch-up (post-merge/watcher/pull) documented. AGENTS.md concept 'push is optional; the file is the channel' added. Next: consider installing the hook live on this repo, and whether resolve/claim should block re-dispatch of a thread mid-work.

## Open inbound (0)

## Open outbound (0)

