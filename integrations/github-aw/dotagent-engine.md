---
# Vendor to: .github/workflows/shared/dotagent.md
# A gh-aw engine definition for the dotagent caretaker pattern: headless opencode wired to
# the repo's control plane (scripts/agent) + mind (.agent/). Based on GitHub's opencode
# example; treat as a vendored sample, not an officially-supported integration. Pin the
# version to control upgrades.
engine:
  id: opencode-dotagent
  display-name: OpenCode caretaker (dotagent)
  description: >-
    Headless opencode running the dotagent caretaker loop on an active-intelligence repo:
    read AGENTS.md (the charter), run the agent CLI (state + inbox), act, resolve, commit.
  runtime-id: opencode
  experimental: true
  behaviors:
    secret-strategy: universal-llm-consumer
    capabilities:
      max-turns: true
    manifest:
      files:
        - opencode.jsonc
        - AGENTS.md
        - skills
        - .agent
        - scripts/agent
      path-prefixes:
        - .opencode/
        - .agent/
    network:
      defaults:
        - host.docker.internal
        - github.com
        - raw.githubusercontent.com
        - registry.npmjs.org
        - opencode.ai
        - models.dev
      provider-domains:
        copilot: api.githubcopilot.com
        anthropic: api.anthropic.com
        openai: api.openai.com
        google: generativelanguage.googleapis.com
    installation:
      package-manager: npm
      package-name: opencode-ai
      version: "1.2.14"
      step-name: Install OpenCode CLI
      binary-name: opencode
      include-node-setup: true
      cooldown: true
      verify-command: opencode --version
      verify-step-name: Verify OpenCode CLI
      docs-url: https://opencode.ai/docs
    config-file:
      path: opencode.jsonc
      step-name: Write caretaker config
      content: |-
        {
          "agent": {
            "build": {
              "permission": {
                "bash": "allow",
                "edit": "allow",
                "read": "allow",
                "glob": "allow",
                "grep": "allow",
                "webfetch": "allow",
                "websearch": "allow",
                "external_directory": "allow"
              }
            }
          },
          "autoupdate": false
        }
      merge-strategy: json-merge
    execution:
      command-name: opencode
      args:
        - run
        - --print-logs
        - --log-level
        - DEBUG
      step-name: Run the caretaker loop
      model-env-var: OPENCODE_MODEL
      mcp-config-env-var: GH_AW_MCP_CONFIG
      write-timestamp: true
      provider-env-mode: universal-llm-consumer
---