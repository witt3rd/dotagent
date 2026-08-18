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
engine run, wherever God that machine lives. There is no "catch-up on pull" here; the
workflow *is* the wake.

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