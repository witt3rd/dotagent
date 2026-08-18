# GitHub Agentic Workflows (GH-AW) — the native repository for dotagent's dispatch

dotagent works on any git repo. On GitHub this is not just supported — it's the *native*
home for the pattern, because **GitHub Agents Workflows (gh-aw) is the dispatch we built
locally as a `post-commit` hook, provided as a platform.**

GH-AW runs event-triggered workflows as **fresh headless agents**. That is exactly the
hybrid we shipped in `integrations/dispatch/`:

| dotagent concept            | local (`post-commit` hook)              | GitHub (gh-aw)                                   |
|-----------------------------|------------------------------------------|--------------------------------------------------|
| **Charter**                 | `AGENTS.md`                              | `AGENTS.md` (already in the engine manifest)     |
| **The mind (event log)**    | `.agent/` committed                      | `.agent/` committed, read by the workflow        |
| **Control plane**           | `scripts/agent`                          | `scripts/agent` (checked out with the repo)      |
| **Wake / dispatch**         | hook fires on a local commit → spawn     | **GitHub event → workflow → fresh `opencode run`** |
| **Pull**                    | `agent state` + `agent inbox`            | the workflow prompt tells opencode to run them   |
| **Act + close**             | `agent resolve` + commit                 | `agent resolve` + commit, pushed back            |
| **Single-flight**           | `.agent/.busy` lock                      | gh-aw workflow concurrency (per-event)           |

The big win over local: on GitHub the push is **real and cross-host**. GitHub is the shared
orchestrator — an event on the repo (an issue, a PR, a comment, a schedule) triggers a fresh
engine run, wherever that machine lives. There is no "catch-up on pull" here; the workflow
*is* the wake.

## Parity with local: the same repo, the same loop

dotagent and gh-aw are designed as close to **1:1** as the platforms allow — the only
difference is **WHERE** the wake runs (your machine vs GitHub). Everything else is the
same:

- **Same mind.** `AGENTS.md` (charter), `.agent/` (event log), `scripts/agent` (control
  plane), `skills/` (lived experience) — one definition of the repo, shared by both paths.
- **Same caretaker loop.** The local `post-commit` dispatch prompt and the gh-aw
  caretaker workflow body are the SAME 5-step sequence: orient (state + inbox) → claim →
  act → record (resolve/reply) → hand off (check + commit). Only the mechanics differ:
  locally you hold the `.agent/.busy` lock and run `git config agent.dispatch`; on GitHub
  the workflow `concurrency` is the lock and `engine:` is the dispatcher.
- **Same value:** the activity-to-agent is pushed either way.

## Bring your agent, we take care of the rest

dotagent is agent-agnostic on both sides. You bring the agent; dotagent provisions
everything around it:

```
./inhabit.sh --repo ~/src/thing --identity thing \
  --dispatch --github \
  --launcher 'oc run'        # the LOCAL agent launcher (your opencode/claude/codex)
# --github provisions .github/workflows/shared/dotagent.md + caretaker.md from the batteries
```

- **Local:** `--launcher 'oc run'` picks which agent the `post-commit` wake spawns.
- **Cloud:** the provisioned caretaker workflow's `engine:` picks which gh-aw engine runs
  (the opencode engine is the shipped battery; `dotagent-engine.md` is the template for
  wiring your own agent's engine).

The user never authors a hook, a workflow, a ledger, or a charter from scratch — they bring
an agent and dotagent establishes the active intelligence, on the ground or in the cloud,
identically.

## The two artifacts

Vendor both into the repo gh-aw can find:

- **`dotagent-engine.md`** → `.github/workflows/shared/dotagent.md` — the engine
  definition that registers opencode (with the `scripts/agent` CLI + `.agent/` in its
  manifest) as the headless caretaker.
- **`caretaker-workflow.md`** → `.github/workflows/caretaker.md` — an example workflow that
  wakes a fresh opencode on an event and runs the caretaker loop (state → inbox → act →
  resolve → commit).

## Wire it up

1. **Vendor the engine** (a gh-aw engine is a `.md` file with an `engine.behaviors`
   frontmatter block; gh-aw compiles it — no binary changes). Treat it as a vendored sample,
   not an officially-supported GitHub integration, and pin a version.
2. **Add the LLM key secret**: **Settings → Secrets and variables → Actions**, add e.g.
   `ANTHROPIC_API_KEY` (or the key for your provider — the engine's
   `provider-env-mode: universal-llm-consumer` reads it from the environment).
3. **Write workflows** that `imports: shared/dotagent.md` and set `engine: dotagent`,
   triggered on the events that are your inbound.
4. **Recompile after workflow edits** with `gh aw compile <workflow>.md --watch`.

## Design guidance (the dispatch vocabulary)

- **The event is the message.** An issue/PR/comment is an `agent inbox` item. Your workflow
  prompt is the revival of the caretaker: "run `agent state` + `agent inbox`, triage, act,
  `agent resolve`, commit."
- **Prefer repo-relative, pinned imports** — `imports: shared/dotagent.md` lets you evolve
  the engine in place; a remote pin (`owner/repo/...@v1.2.14`) lets you control upgrades.
- **Single-flight per event.** Use gh-aw workflow concurrency so one inbound is handled at
  a time, mirroring the `.agent/.busy` lock.
- **The ledger stays the source of truth.** The workflow commits its `resolve`/`handoff`
  back to `.agent/`, so the git history is still the audit trail — local and GitHub share
  the same mind.

## Footnotes on scope

- This is **gh-aw (GitHub Agentic Workflows)**, the markdown-workflow agent engine — not
  classic GitHub Actions YAML. It needs the gh-aw tooling/GA to be available on your
  account; it is evolving, so pin versions and treat the engine file as a vendored sample.
- The engine definition extends the opencode example from GitHub's own
  third-party-agent guide, adding the dotagent manifest (`scripts/agent`, `.agent/`,
  `skills/`) and a caretaker config.