# Native tool source review

Reviewed on 2026-10-29 for Pi, Antigravity CLI and Grok Code onboarding. Implemented contracts and qualified versions are in [NATIVE-TOOL-CONTRACTS.md](NATIVE-TOOL-CONTRACTS.md). Rule-level references live in the canonical rule catalog.

## Inventory and method

The source inventory contains 359 records: 354 successful primary responses and five unavailable, guessed Antigravity schema endpoints. Those five probes do not establish a published schema. Antigravity publishes the settings and plugin schemas used here; hooks, rules and agents use their documentation contracts instead.

Documentation review accounts for 151 pages: 85 Antigravity, 42 Pi and 24 Grok. Contract-defining sections were compared to the implementation. Product, runtime and historical pages received content and configuration-reference triage. Historical release bodies and every runtime narrative were not exhaustively reread.

The Pi bucket also contains 31 implementation files, 16 fixtures, 15 examples, two test guides, README, changelog and two blog/site sources. Grok has 95 pinned source/guide files, supplemented by its marketplace index, LSP loader and video-storage parser. Release feeds are separate freshness evidence.

## Documentation coverage

Paths below are relative to their primary documentation root. They account for each documentation page in the review.

| Corpus | Contract/reference pages | Other pages and scope |
|---|---|---|
| [Antigravity docs](https://antigravity.google/docs/overview) | 26: `agent-settings`, `hooks`, `mcp`, `permissions`, `plugins`, `rules`, `sandbox`, `settings`, `skills`, `subagents`, `enterprise`, `install`, `remote-control`, `cli/best-practices`, `cli/credits`, `cli/gcli-migration`, `cli/headless`, `cli/modes`, `cli/reference`, `cli/statusline`, `cli/title`, `cli/vim-editor-mode`, `cli/commands/agents`, `cli/commands/permissions`, `cli/commands/statusline`, `cli/commands/title` | 11 App/IDE, 9 SDK and 39 runtime/product/history pages, listed below |
| [Pi v1.4.0 docs](https://github.com/earendil-works/pi/tree/v1.4.0/packages/coding-agent/docs) | 15: `configuration.md`, `settings.md`, `keybindings.md`, `mcp.md`, `models-configuration.md`, `models.md`, `packages.md`, `prompt-templates.md`, `skills.md`, `themes.md`, `providers.md`, `shell-aliases.md`, `environment-variables.md`, `security.md`, `compaction.md` | 14 usage/platform and 13 SDK/protocol/extension pages, listed below |
| [Grok Build docs](https://docs.x.ai/build/overview) | 12: `enterprise`, `settings`, `settings/reference`, `settings/zdr-video-storage`, `features/hooks`, `features/mcp-servers`, `features/permissions`, `features/project-rules`, `features/sandbox`, `features/skills-plugins-marketplaces`, `features/status-line`, `features/subagents` | 3 UI and 9 runtime/usage pages, listed below |

Antigravity App/IDE: `build-with-google`, `sidecars`, `ide/agent-side-panel`, `ide/allowlist-denylist`, `ide/browser`, `ide/browser-recordings`, `ide/extensions`, `ide/review-changes-editor`, `ide/separate-chrome-profile`, `ide/tab`, `ide/workflows`.

Antigravity SDK: `sdk/lifecycle`, `sdk/local-models`, `sdk/mcp`, `sdk/overview`, `sdk/personas`, `sdk/policies`, `sdk/structured-output`, `sdk/subagents`, `sdk/tools`.

Antigravity runtime/product/history: `artifact-review`, `artifacts`, `boost`, `changelog`, `cli/artifacts`, `cli/commands/btw`, `cli/commands/changelog`, `cli/commands/codesearch`, `cli/commands/context`, `cli/commands/credits`, `cli/commands/diff`, `cli/commands/effort`, `cli/commands/resume`, `cli/commands/usage`, `cli/commands/voice`, `cli/conversations`, `cli/prompting`, `cli/troubleshooting`, `cli/using`, `commands/browser`, `commands/goal`, `commands/grill-me`, `commands/learn`, `commands/schedule`, `faq`, `features`, `firebase-studio-migration`, `getting-started`, `implementation-plan`, `marketplace`, `models`, `overview`, `plan`, `plans`, `projects`, `screenshots`, `slash-commands`, `teamwork`, `walkthrough`.

Pi usage/platform: `quickstart.md`, `usage.md`, `cli.md`, `slash-commands.md`, `sessions.md`, `how-pi-works.md`, `index.md`, `containerization.md`, `llama-cpp.md`, `nvidia-rtx-llama-cpp.md`, `terminal-setup.md`, `termux.md`, `tmux.md`, `windows.md`.

Pi SDK/protocol/extensions: `cli-integration.md`, `codemode.md`, `custom-provider.md`, `extensions.md`, `json.md`, `message-types.md`, `rpc-commands.md`, `rpc-extension-ui.md`, `rpc.md`, `sdk.md`, `session-format.md`, `tui.md`, `virtual-models.md`.

Grok UI: `features/dashboard`, `features/theming`, `keyboard-shortcuts`. Grok runtime/usage: `cli/headless-scripting`, `cli/reference`, `cli/terminal-support`, `features/background-tasks`, `features/plan-mode`, `features/sessions`, `features/worktrees`, `modes-and-commands`, `overview`.

## Blog review

All 32 blog/discovery sources were read: 26 Antigravity articles, its RSS and index, two Grok launch articles, Pi's author article and pi.dev.

| Relevant source | Outcome |
|---|---|
| [Antigravity custom agents](https://antigravity.google/blog/introducing-custom-agents), [agents in plugins](https://antigravity.google/blog/custom-agents-in-google-plugins) | Preserve native agent fields and full model IDs. Do not invent a closed alias-only model enum. |
| [I/O deep dive](https://antigravity.google/blog/google-io-2026-feature-deep-dive), [I/O announcement](https://antigravity.google/blog/google-io-2026), [Antigravity 2](https://antigravity.google/blog/introducing-google-antigravity-2), [CLI launch](https://antigravity.google/blog/introducing-google-antigravity-cli) | Cross-check hooks, permissions and migration against current CLI-specific documentation. |
| [Antigravity SDK launch](https://antigravity.google/blog/introducing-google-antigravity-sdk) | SDK SSE support is not the CLI contract. Keep the CLI's explicit SSE rejection. |
| [Grok Build launch](https://x.ai/news/grok-build-cli), [open source launch](https://x.ai/news/grok-build-open-source) | Follow the linked native loader and bundled guides. Reconcile the source snapshot with the current released changelog. |
| [Pi author article](https://mariozechner.at/posts/2025-11-30-pi-coding-agent/), [Pi website](https://pi.dev) | The historical no-MCP position is superseded by Pi v1.4.0's built-in MCP documentation and current website. |

The remaining Antigravity articles contain no additional lintable CLI file contract. Their paths under [the official blog](https://antigravity.google/blog) are:

- Enterprise and access: `antigravity-enterprise`, `antigravity-ide-extensions`, `google-antigravity-for-enterprises`, `changes-to-antigravity-plans`.
- Models and examples: `gemini-3-flash-in-google-antigravity`, `gemini-3-1-pro-in-google-antigravity`, `gemini-3-5-flash-in-google-antigravity`, `gemini-3-6-flash-in-google-antigravity`, `gemini-3-7-flash-in-google-antigravity`, `gemini-3-8-flash-in-google-antigravity`, `gemini-4-argon-in-google-antigravity`, `nano-banana-pro`.
- Research and orchestration: `gemini-4-argon-trillion-edge-graphs-and-tpu-kernels`, `google-antigravity-built-an-os`, `teamwork-when-ai-becomes-a-research-partner`.
- UI/workflows: `introducing-google-antigravity`, `remote-control-for-antigravity`, `vcs-and-terminal`, `visualizing-with-the-help-of-antigravity`.
- Discovery: the blog index and `rss.xml` enumerate the reviewed corpus.

## Contract conflicts and boundaries

Native parser behavior takes precedence over a generic Agent Skills assumption. Published current CLI documentation takes precedence over an older launch article or a different SDK's API.

Antigravity registry paths are resolved relative to their declaring registry. This is inferred from the documented `.agents/rules.json` example using `path: "rules"`; no public implementation was found. File and directory filters follow the documented examples. Agnix does not claim an undocumented wildcard grammar for these filters.

Pi credential-store validation, including user-authored `auth.json` command-backed keys, is outside the current rule set. Grok `pager.toml` renderer settings, Antigravity App-only sidecars, SDK program configuration and runtime/session stores are excluded from CLI agent-file coverage. These exclusions are recorded so the research inventory cannot be mistaken for exhaustive validation of every file a tool can read.

The audit found and corrected native discovery, scope and type gaps: Pi resource subtypes and filters, Antigravity flat scanning/registries/plain-rule limits, and Grok marketplace/LSP formats, inline MCP types, unsigned timeouts, project MCP output caps and managed video-storage shapes. Independent reviews also checked the JSON depth guard, offline schemas and editor document overlay.
