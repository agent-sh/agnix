<div align="center">
  <img src="editors/vscode/icon.png" alt="agnix" width="128">
  <h1>agnix</h1>
  <p><strong>Lint agent configurations before they break your workflow</strong></p>
  <p>
    <a href="https://www.npmjs.com/package/agnix"><img src="https://img.shields.io/npm/v/agnix.svg?color=3C873A" alt="npm"></a>
    <a href="https://crates.io/crates/agnix-cli"><img src="https://img.shields.io/crates/v/agnix-cli.svg?color=E57324" alt="Crates.io"></a>
    <a href="https://github.com/agent-sh/agnix/releases"><img src="https://img.shields.io/github/v/release/agent-sh/agnix?color=0A7E8C" alt="Release"></a>
    <a href="https://github.com/agent-sh/agnix/actions/workflows/ci.yml"><img src="https://github.com/agent-sh/agnix/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
    <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/license-MIT%2FApache--2.0-64748B.svg" alt="License"></a>
    <a href="https://github.com/hesreallyhim/awesome-claude-code"><img src="https://awesome.re/mentioned-badge.svg" alt="Mentioned in Awesome Claude Code"></a>
  </p>
</div>

> Running open models for your company? [**Tiyuvta**](https://tiyuvta.ai/services/) helps with model choice, deployment and optimization, and fine-tuning on your hardware or cloud account.

<p align="center">
  <a href="https://github.com/sponsors/avifenesh"><img src="https://img.shields.io/badge/Sponsor-%E2%9D%A4-db61a2?style=for-the-badge&logo=githubsponsors&logoColor=white" alt="Sponsor"></a>
</p>
<p align="center"><b>Built and maintained with a lot of agent tokens and hours. If it saves you time, <a href="https://github.com/sponsors/avifenesh">sponsoring</a> helps keep it going.</b></p>

<p align="center">Catch broken agent configs before your AI tools silently ignore them.<br>757 rules across Claude Code, Codex CLI, OpenCode, Cursor, Copilot, and more -<br>validating CLAUDE.md, SKILL.md, hooks, MCP configs, and other agent files.</p>

<p align="center"><strong>Auto-fix</strong> | <strong><a href="https://github.com/marketplace/actions/agnix-ci">GitHub Action</a></strong> | <strong><a href="https://marketplace.visualstudio.com/items?itemName=avifenesh.agnix">VS Code</a> + <a href="https://plugins.jetbrains.com/plugin/30087-agnix">JetBrains</a> + <a href="https://github.com/agent-sh/agnix/tree/main/editors/neovim">Neovim</a> + <a href="https://zed.dev/extensions?query=agnix">Zed</a></strong></p>

<p align="center">
  <a href="https://agent-sh.github.io/agnix/"><img src="https://img.shields.io/badge/Docs-Website-2563EB?style=for-the-badge" alt="Website"></a>
  <a href="https://agent-sh.github.io/agnix/playground"><img src="https://img.shields.io/badge/Try_it-Playground-E91E63?style=for-the-badge" alt="Playground"></a>
  <a href="https://dev.to/avifenesh/your-ai-agent-configs-are-probably-broken-and-you-dont-know-it-16n1"><img src="https://img.shields.io/badge/Blog-Post-0A0A0A?style=for-the-badge&logo=dev.to" alt="Blog Post"></a>
</p>

<p align="center"><em>New rules and tool support ship constantly. Follow for real-time updates:</em></p>
<p align="center">
  <a href="https://x.com/avi_fenesh"><img src="https://img.shields.io/badge/Follow-@avi__fenesh-1DA1F2?style=for-the-badge&logo=x&logoColor=white" alt="Follow on X"></a>
</p>

## Why agnix?

**Your skills don't trigger.** Vercel's research found skills [invoke at 0%](https://vercel.com/blog/agents-md-outperforms-skills-in-our-agent-evals) without correct syntax. One wrong field and your skill is invisible.

**"Almost right" is the worst outcome.** [66% of developers](https://survey.stackoverflow.co/2025/ai) cite it as their biggest AI frustration. Misconfigured agents produce exactly this.

**Multi-tool stacks fail silently.** Cursor + Claude Code + Copilot each want different formats. A config that works in one tool [breaks in another](https://arnav.tech/beyond-copilot-cursor-and-claude-code-the-unbundled-coding-ai-tools-stack).

**Bad patterns get amplified.** AI assistants don't ignore wrong configs - they [learn from them](https://www.augmentcode.com/guides/enterprise-coding-standards-12-rules-for-ai-ready-teams).

agnix validates all of it - 757 rules sourced from official specs, academic research, and real-world breakage patterns. Auto-fix included.

> **Want to try it first?** [Open the playground](https://agent-sh.github.io/agnix/playground) - paste any agent config, see diagnostics instantly. No install, runs in your browser.

## Quick Start

```console
$ npx agnix .
Validating: .

CLAUDE.md:15:1 warning: Generic instruction 'Be helpful and accurate' [fixable]
  help: Remove generic instructions. Claude already knows this.

.claude/skills/review/SKILL.md:3:1 error: Invalid name 'Review-Code' [fixable]
  help: Use lowercase letters and hyphens only (e.g., 'code-review')

Found 1 error, 1 warning
  2 issues are automatically fixable

hint: Run with --fix, --fix-safe, or --fix-unsafe to apply fixes
```

https://github.com/user-attachments/assets/72d5fe7c-476f-46ea-be64-5785cf6d5600

## Install

```bash
# npm (recommended, all platforms)
npm install -g agnix

# Homebrew (macOS/Linux)
brew tap agent-sh/agnix && brew install agnix

# pip (or `uvx agnix .` to run without installing)
pip install agnix

# Cargo
cargo install agnix-cli
```

[Pre-built binaries](https://github.com/agent-sh/agnix/releases) | [All install options](https://agent-sh.github.io/agnix/docs/installation)

### Editor Extensions

| Editor | Install |
|--------|---------|
| **VS Code** | [VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=avifenesh.agnix) |
| **JetBrains** | [JetBrains Marketplace](https://plugins.jetbrains.com/plugin/30087-agnix) |
| **Neovim** | `{ "agent-sh/agnix", config = function() require("agnix").setup() end }` |
| **Zed** | Search "agnix" in Extensions |

[Editor setup guide](https://agent-sh.github.io/agnix/docs/editor-integration)

### GitHub Action

```yaml
- name: Validate agent configs
  uses: agent-sh/agnix@v0
  with:
    target: 'claude-code'
```

## Usage

```bash
agnix .              # Validate current directory
agnix --fix .        # Apply only safe fixes
agnix --fix-safe .   # Explicit safe-only fix mode
agnix --fix-unsafe . # Apply all fixes, including medium and LOW confidence
agnix --dry-run --show-fixes .  # Preview fixes with inline diff output
agnix --strict .     # Strict mode (warnings = errors)
agnix --format github .  # GitHub Actions annotations
agnix explain MCP-018    # Explain a rule and its evidence
agnix --target claude-code .  # Only Claude Code rules plus generic ones, like tools = ["claude-code"]
agnix --target kiro .         # Only Kiro rules plus generic ones, like tools = ["kiro"]
```

[Full CLI reference](https://agent-sh.github.io/agnix/docs/configuration) | [Full rules reference](https://agent-sh.github.io/agnix/docs/rules)

Kiro permission YAML templates at `.kiro/settings/permissions.yaml` are lintable. The actual user policy is `~/.kiro/settings/permissions.yaml`; workspace policies live outside the repository at `~/.kiro/workspace-roots/<hash>/permissions.yaml`. Lint those paths explicitly to check the active policies.

## Supported Tools

Maintenance priorities:

| Tier | Tools |
|------|-------|
| S | Claude Code, Codex CLI |
| A | Kiro, OpenCode, Pi |
| B | Antigravity CLI, Grok Code |
| C | Muse Code, Kimi Code |
| D | GitHub Copilot, Cline, Cursor, legacy Gemini CLI |

Pi, Antigravity CLI and Grok Code have native static validators and automated release tracking. Muse Code and Kimi Code validation remains pending. See the [coverage and monitoring inventory](knowledge-base/RESEARCH-TRACKING.md) and [native contracts](knowledge-base/NATIVE-TOOL-CONTRACTS.md) for supported formats and qualified versions.

Implemented validators:

| Tool | Rules | Count | Config Files |
|------|-------|-------|--------------|
| [Pi](https://pi.dev) | PI-\* | 17 | .pi/settings.json, ~/.pi/agent/, models, keybindings, MCP, packages, themes and prompts |
| [Antigravity CLI](https://antigravity.google/docs/getting-started?tab=cli) | AGY-\* | 18 | CLI settings, mcp_config.json, hooks, plugins, rules, registries and agents |
| [Grok Code](https://docs.x.ai/build/cli/reference) | GRK-\* | 20 | .grok/config.toml, sandbox, requirements, roles, personas, hooks, plugins, LSP, agents and skills |
| [Agent Skills](https://agentskills.io) | AS-\*, XP-SK-\* | 26 | SKILL.md |
| [Claude Code](https://code.claude.com/docs) | CC-\* | 170 | CLAUDE.md, .claude/rules/\*.md, settings.json, hooks, agents, skills, output styles, plugins, marketplace.json, .mcp.json |
| [GitHub Copilot](https://docs.github.com/en/copilot) | COP-\*, CP-SK-\* | 35 | .github/copilot-instructions.md, .github/instructions/\*.instructions.md, .github/agents/, .github/prompts/, .github/hooks/\*.json, copilot-setup-steps.yml, plugin.json, .github/skills/ |
| [Cursor](https://cursor.com) | CUR-\*, CR-SK-\* | 40 | .cursor/rules/\*.mdc, .cursorrules, .cursor/hooks.json, .cursor/agents/, .cursor/environment.json, .cursor/cli.json, permissions.json, .cursor-plugin/, .cursorignore, .cursor/skills/ |
| [Codex CLI](https://developers.openai.com/codex) | CDX-\*, CX-SK-\* | 89 | AGENTS.md, .codex/config.toml, .codex/hooks.json, .codex/agents/\*.toml, .codex/rules/\*.rules, requirements.toml, .codex-plugin/plugin.json, .agents/skills/ |
| [OpenCode](https://opencode.ai) | OC-\* | 67 | opencode.json(c), .opencode/agents/, .opencode/commands/, .opencode/tools/, .opencode/plugins/, .opencode/skills/ |
| [Kiro](https://kiro.dev) | KIRO-\*, KR-SK-\*, KR-AG-\*, KR-HK-\*, KR-PW-\*, KR-MCP-\*, KR-SET-\*, KR-PM-\*, KR-WF-\*, KR-IGN-\* | 141 | .kiro/steering/\*\*/\*.md, .kiro/skills/\*\*/SKILL.md, .kiro/agents/\*\*/\*.{json,md}, .kiro/hooks/\*.kiro.hook, .kiro/hooks/\*.json, .kiro/settings/mcp.json, .kiro/settings/cli.json, .kiro/settings/permissions.yaml (or .json), .kiro/powers/\*/POWER.md and published POWER.md, power plugin.json and mcp.json, .kiro/workflows/\*.workflow.{json,yaml,yml}, .kiroignore |
| [MCP](https://modelcontextprotocol.io) | MCP-\* | 30 | .mcp.json and the MCP server blocks of client configs |
| [AGENTS.md](https://agents.md) | AGM-\*, XP-\* | 13 | AGENTS.md, AGENTS.local.md, AGENTS.override.md |
| [Cline](https://docs.cline.bot) | CLN-\*, CL-SK-\* | 16 | .clinerules/, .cline/rules/, .cline/workflows/, .cline/hooks/, .clineignore, cline_mcp_settings.json, .cline/skills/ |
| [Gemini CLI](https://github.com/google-gemini/gemini-cli) | GM-\* | 35 | GEMINI.md, .gemini/settings.json, gemini-extension.json, extension hooks and agents, .gemini/commands/\*.toml, policies, .gemini/agents/, .geminiignore |

Continue, Aider, Amazon Q, Tabnine, Codeium, Devin/Windsurf and Amp are outside active support and monitoring. Shipped Amp and Windsurf/Devin validators remain available for legacy compatibility. Roo Code support was removed in 0.59.0.

## Architecture

Rust workspace crates:
- `agnix-rules` - rule metadata generated from `knowledge-base/rules.json`
- `agnix-core` - shared validation engine library
- `agnix-cli` - command-line interface binary
- `agnix-lsp` - language server binary
- `agnix-mcp` - MCP server binary
- `agnix-wasm` - WebAssembly bindings for browser/runtime integrations

## Contributing

Contributions welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for the development guide.

### Local development loop

Run `bash scripts/preflight.sh` while editing. It checks formatting, rule
bookkeeping, count tables, locale copies, instruction copies, and schema copies
before any compilation. It requires Rust with rustfmt, Node.js, Python 3, and
Bash. Missing tools fail the check instead of silently skipping it.

Before pushing, run `bash scripts/preflight.sh --full`. This adds Clippy, workspace
tests, the rule efficacy evaluation, the Kiro gate, packaging script tests,
generated schema comparison, and source-built agent configuration validation.
It reports each check's elapsed time and stops at the first failure. Schema
comparison uses a temporary file and never overwrites local edits. Cargo builds,
Rust tests, and Rayon default to two workers each; set `CARGO_BUILD_JOBS`,
`RUST_TEST_THREADS`, or `RAYON_NUM_THREADS` to override these local defaults.

The existing optional hook installer, `sh scripts/setup-hooks.sh`, installs a
pre-push hook that calls the same full preflight. Git worktrees share the hooks
directory, and the installer replaces the existing pre-push hook.

Track the change in an issue, then open a PR with the tested commit, commands,
results, and any limits of the evidence. Local preflight does not replace hosted
Linux, Windows, macOS, CodeQL, dependency, or review gates. Review the diff and
record a factual self-review comment, wait for successful `revuto-review` and
CI, and address every review comment before merging through the PR. A changed
head needs fresh checks and review. Report a specific blocker when a required
gate cannot finish; keep branch protection intact.

CI and Security cancel superseded runs of the same PR. Their main and scheduled
runs use separate groups so newer runs cannot cancel them. The docs deployment
workflow keeps its existing shared concurrency group. CodeQL uses Rust's buildless
extraction, which runs build scripts and proc macros itself; release binaries are
still built by the release workflow.

[Report a bug](https://github.com/agent-sh/agnix/issues/new) | [Request a rule](https://github.com/agent-sh/agnix/issues/new) | [Good first issues](https://github.com/agent-sh/agnix/labels/good%20first%20issue)

## License

MIT OR Apache-2.0

---

<p align="center">
  <a href="https://github.com/agent-sh/agnix/stargazers">Star this repo</a> to help other developers find agnix.
</p>
