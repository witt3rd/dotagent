---
name: git
description: >-
  House git discipline — applies to ALL repos. Two modes: work on main during active
  debugging/iteration (commit frequently), or use worktrees for parallel feature work. Never
  accumulate uncommitted work. Always sync with origin/main before starting. Covers repo
  hygiene and the clean end-state: no stale worktrees, no local branches beyond the mainline,
  mainline at tip of origin, primary clone clean.
metadata:
  aliases: [worktree, repo-hygiene, clean-end-state]
  deps: [caretaker]
---

# House git discipline (all repos)

## Two modes, one rule

**Mode 1: Active debugging/iteration** — work on main. Commit small batches frequently.
Push to a branch when stable. The git history is a breadcrumb trail.

**Mode 2: Parallel feature work** — use worktrees. Different agents building different
features that don't touch the same files. If they do touch the same files, coordinate:
one agent works at a time on that file.

The rule: **never accumulate uncommitted work.** Commit every 30 minutes during debugging.
Small, revertable commits. "Fix X" not "down to one last bug?"

## Active debugging workflow

```bash
# 1. Start: sync with origin/main
git checkout main
git pull origin main

# 2. Work: commit small batches frequently
git add <files>
git commit -m "fix: describe what changed"
# ... repeat every 30 minutes ...

# 3. When ready for PR: push to a branch
git checkout -b feat/my-feature
git push origin feat/my-feature
gh pr create --repo <owner>/<repo> --base main --head feat/my-feature

# 4. After merge: clean up
git checkout main
git pull origin main
git branch -d feat/my-feature
```

## Parallel feature workflow (worktrees)

Use worktrees when two agents are building **distinct features** that don't touch the
same files. If they do touch the same files, coordinate — one agent works at a time on
that file.

```bash
# Create a worktree for a parallel feature
git wt-new feat/my-feature
cd <repo>.wt/feat--my-feature
# ... work, commit, push, open PR ...

# After merge: clean up
git wt-rm feat/my-feature
```

## The two commands

```bash
# Create a worktree + branch for a change (run from anywhere in the repo)
git wt-new docs/foo
#   -> <parent>/<repo>.wt/docs--foo/ on branch docs/foo
#   forks from the mainline (--start <ref> overrides)
cd <parent>/<repo>.wt/docs--foo
# ...work, commit, push, open PR from inside the worktree...

# After the merge: remove the worktree AND delete the branch, together
git wt-rm docs/foo
#   safe-delete: refuses dirty trees / unmerged branches unless --force
```

Scripts live at `~/.local/bin/git-wt-new` and `~/.local/bin/git-wt-rm`.

## Conventions

- Branch `docs/<x>` / `fix/<x>` / `feat/<x>` / `task/<x>` → folder `docs--<x>` (kebab-case,
  `/` → `--`). Match the repo's existing branch shape (`git branch -a`) when unsure.
- **Before opening a PR, sync with origin/main** — always `git pull origin main` first,
  rebase if needed, then push.
- Commit messages: imperative mood, ≤72 chars. "fix: ..." / "feat: ..." / "chore: ..."

## Before starting any session

```bash
git checkout main
git pull origin main
```

Always start from the latest origin/main. Never start from a stale state.

## Never accumulate uncommitted work

- Commit every 30 minutes during debugging
- Small, revertable commits: "fix smoke test", "add debug logging", "revert that"
- The git history is a breadcrumb trail, not a pristine record
- If you need to roll back, `git revert <commit>` — don't lose work

## State and repair

- `git worktree list` — see every worktree and its branch.
- If a worktree got moved out-of-band with a plain `mv`, git's registration still points at
  the old path (listed `prunable`). Repair from inside the moved worktree: `git worktree
  repair` then `git worktree prune`.

## Repo hygiene — the clean end-state (a contract, not a nicety)

"Clean" is a **checkable end-state**, not a feeling. A repo is clean — ready for the next
agent to pick up cold — when ALL of these hold:

1. **No stale worktrees.** Every worktree under `<repo>.wt/` belongs to a branch whose work
   is still in flight. Once merged, remove it: `git wt-rm`.
2. **No local branches beyond the mainline.** `git branch` shows only the mainline plus any
   branch with a live worktree.
3. **The mainline is at the tip of origin** (or deliberately ahead/behind and recorded).
   `git status -sb` on the mainline should read `## main...origin/main` with no
   `ahead`/`behind` — unless a push/PR is intentionally deferred.
4. **The primary clone is clean.** No uncommitted work. All work committed.

## Integrity sweep

Run these checks IN ORDER. Fix what fails, then move on.

```bash
git fetch origin
```

1. **Unpushed commits** (`git status -sb`, `git log --oneline origin/main..HEAD`)
2. **Uncommitted work** (`git status --porcelain`)
3. **Local branches — merged or abandoned?** (`git branch --merged origin/main`)
4. **Stale worktrees** (`git worktree list`)
5. **The mainline is at origin tip** (`git fetch origin && git status -sb`)

## The handoff channel

Where the repo has `scripts/agent` (dotagent-inhabited), write it via `agent handoff
<subject>`. Elsewhere, `.agent/HANDOFF.md`. Never hand-edit a growing `.agent/HANDOFF.md`
in a ledger repo — the ledger replaces it.

## As an agent

- **During debugging:** work on main, commit frequently, push when stable.
- **For parallel features:** use worktrees. Different agents, different files.
- **Before any session:** `git pull origin main`.
- **Before any PR:** sync with origin/main, rebase if needed.
- Read each repo's AGENTS.md for repo-specific rules.