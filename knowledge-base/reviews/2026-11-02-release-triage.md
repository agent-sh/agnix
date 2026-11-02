# Release and spec-drift review - 2026-11-02

Covers #1777/#1778-lineage tool releases still open (#1778 Antigravity 1.4.12,
#1779 Kiro 2.35.2, #1785 OpenCode 2 v2.2.5) and the spec-drift notice #1784.
All versions were verified against the live upstream surfaces on 2026-11-02,
not just the issue markers. The catalog stays at 757 rule IDs; no rule was
added, removed or renumbered.

| Item | Result and primary evidence |
| --- | --- |
| Antigravity CLI 1.4.12 (#1778) | Live installer manifest reports `version: 1.4.12`. Read the full release notes with `agy changelog`: the only config-adjacent change is `"optional": true` for API key providers in a plugin's `auth.json`, a file agnix does not validate; the other entry is a terminal-rendering fix. The published settings and plugin schemas are byte-identical to the digests pinned in the 2026-10-30 review (`6d4ab641fff7…` settings, `0eea5e483af6…` plugin). Baseline-only advance. |
| Kiro CLI 2.35.2 -> 2.36.0 (#1779) | The canonical changelog's newest entry is 2.36.0 (Oct 29, 2026), which republishes the 2.35.x elevated-privilege warning. Two data updates: `app.warnOnElevatedPrivileges` joins the KR-SET-005 known-key list (boolean, evidence is the changelog's `kiro-cli settings app.warnOnElevatedPrivileges false`; the settings reference table had not listed it yet, and no workspace restriction is documented so none is inferred), and the `approve-for-me` policy preset joins KR-PM-006's valid list because Kiro itself writes it into workspace permissions files ("Kiro adds the `approve-for-me` policy preset to the workspace permissions file"); the permissions reference's preset table still lists the original six. Baseline advances to 2.36.0, superseding the 2.35.2 marker. |
| OpenCode 2 v2.2.5 (#1785) | Tagged release verified via the GitHub releases API (newest stable `v2.*` tag). The tagged schema (`packages/schema/src/config/compaction.ts`) types `buffer` as `Union([Percent, NonNegativeInt])` with `Percent` = `^([1-9]\d?(\.\d+)?)%$` ("from 1% up to but not including 100%"). OC-CFG-009 now accepts that string form when the file targets OpenCode 2; OpenCode 1 keeps the integer-only contract. The `bash`->`shell` tool-name aliasing is runtime dispatch, not configuration. Baseline advances to v2.2.5. |
| Spec drift (#1784) | All five changed sources were fetched and hashed with the sentinel's own normalization; the live hashes match the issue's current-hash prefixes exactly. Every rule-bearing claim of the 59 affected rules was checked against the live text: the 33 hook events, matcher value sets (Notification/StopFailure/DirectoryAdded), hook type/field contracts, skills frontmatter table (incl. `metadata`, `background`, `user-invocable` - all already known), subagent frontmatter table (incl. `color`, `omitClaudeMd`, `autoCompactWindow` range, permissionMode modes, 32-skill preload, 15,000-token description budget), MEMORY.md load limits, JSON-RPC 2.0, and the MCP security guidance are unchanged. No rule behavior change was needed; CC-SK-028's quoted vendor text was refreshed because the skills page reworded the "No shell command was run" explanation (the error reference now spells it "No shell command in \<name\> was run, because the \<tool\> tool does not accept one of them"; agnix's control-character check remains the documented subset). All five baseline hashes were refreshed. |

## Verification digests

| Source | SHA-256 (CRLF-normalized, sentinel-equivalent) |
| --- | --- |
| code.claude.com/docs/en/hooks.md | `4dbc59ce81a4b431d4fe819939760b1a5ac73d5aa41e27c47e5d7c06411bdf31` |
| code.claude.com/docs/en/skills.md | `d1cfcd4a5a898f34674a494cc0cbe14cc3b1613bb855133870f5ad9a0ab95bea` |
| code.claude.com/docs/en/sub-agents.md | `410d576b916a8dd6a313c3bdf169f9dd88152e9ac6d0887ec15f4c2db5fcd604` |
| modelcontextprotocol.io …/2026-07-28/index.md | `367fe3c143676f4cffcd01cbd9699c7ea7c58c3224e29c9a4432ffc60d017d96` |
| modelcontextprotocol.io …/basic/security_best_practices.md | `a38640214db9bd1253ffac3cfc1389710861746c8e4b2546879e6561a291a257` |
| antigravity.google/schemas/v1/settings.json | `6d4ab641fff729c0ec416afc8c96ccb414d9f438fe3f28fec294086a8f824877` (unchanged) |
| antigravity.google/schemas/v1/plugin.json | `0eea5e483af6f03fa099f32b065d2924a323f1a616ed0323cfa0b7a74ff9543c` (unchanged) |

This review concerns static configuration compatibility. Runtime dispatch,
model behavior and vendor runtime fixes are not executed or qualified by agnix.
