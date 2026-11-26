# Release compatibility review - 2026-11-25

For the later markers and Pi v1.6.1 qualification, see the [November 26 follow-up](2026-11-26-release-triage.md).

The Wednesday sweep re-ran the release poll (`UPDATE_BASELINES=true bash
scripts/check-tool-releases.sh`) on 2026-11-25: `ok=5 new=9 errors=0
skipped=1`. Each marker below was read from its live source before
writing, and each finding was checked against the released CLI's own
text (changelog, published schema, bundled guide or binary), not only
the watcher's quoted marker.

| Issue | Tool | Claimed | Baseline now | Primary live evidence |
| --- | --- | --- | --- | --- |
| #1885 | Antigravity CLI | 1.5.16 | 1.5.17 | linux_amd64 auto-updater manifest, plus the 1.5.17 and 1.5.16 notes fetched live with `agy changelog` |
| #1886 | Claude Code | v2.1.331 | v2.1.332 | npm registry latest record and the GitHub release notes |
| #1887 | Codex CLI | rust-v0.172.0 | rust-v0.172.0 | tagged `config.schema.json`, diffed against rust-v0.171.0, plus the release notes |
| #1888 | Kimi Code | 2.7.3 | 2.7.3 | npm dist-tags and the quoted-key set of the main bundle |
| none | Kiro CLI | 3.6.0 | 3.6.0 | kiro.dev CLI changelog, hooks types and settings reference |
| none | OpenCode 2 | v2.4.1 | v2.4.1 | opencode.ai/config.json and the v2.4.0/v2.4.1 release notes |
| none | Grok Code | 1.0.92 | 1.0.92 | x.ai changelog, the bundled user guide and the installed CLI's own error text |
| none | Cursor | 3.32.4 | 3.32.4 | update endpoint on five platforms, published `environment.schema.json`, docs diffed against Wayback copies of the baseline content |
| none | Gemini CLI | v0.71.1 | v0.71.1 | GitHub release and the published settings schema (byte-identical to v0.70.0) |

## Triage

Validator changes, each covered by a test in `release_triage.rs`:

- Claude Code v2.1.332 adds the `PreSkillShellCommand` hook event
  ("Requires Claude Code v2.1.332 or later"). CC-HK-001 accepts it, CC-HK-003
  asks for a matcher on it, and CC-HK-021 lists it as a tool event for `if`.
  A pin below v2.1.332 keeps the CC-HK-001 error: "on 2.1.331 and older, a
  settings file naming it opens a dialog at start".
- Claude Code v2.1.331 adds `optionalDependencies` to plugin manifests.
  CC-PL-017 accepts the key, CC-PL-019 checks its versions like
  `dependencies`, and CC-MP-005 accepts it on marketplace entries.
- Codex rust-v0.172.0 adds `approvals_reviewer` on one connected app tool
  (`[apps.<app_id>.tools.<tool_id>]`): CDX-CFG-024 validates the value and
  CDX-CFG-006 accepts the key. CDX-CFG-011 accepts `connector_tool_result_passthrough`
  and `realtime_frames_local`, absent from the release notes.
- Kiro CLI 3.6.0 adds the `SubagentStop` trigger (V3 sessions): KR-HK-012
  accepts it. The settings reference documents `chat.notificationSound`
  (boolean, global only): KR-SET-005 accepts it.
- OpenCode v2.4.0 adds the root `automations` boolean: OC-004 accepts it.
- Cursor 3.32.4's published `environment.schema.json` carries a root `clone`
  object: CUR-016 accepts it. The other four drifted Cursor docs change no
  validated surface.
- Grok CLI 1.0.90: "unknown keys in sandbox.toml are reported as errors", and
  the bundled guide says Grok "ignores the whole `sandbox.toml`". GRK-SBX-002
  now reports an unknown key at the root or in a profile as an error for
  grok-code pinned to 1.0.90 or later, or unpinned. `workspace_write` is
  typed as a boolean and `minimal` joins the `extends` values (both from CLI
  1.0.82).

Baseline-only advances:

- Antigravity CLI 1.5.16 and 1.5.17: the published settings and plugin
  schemas hash to the `source_sha256` values recorded in `sources.json`
  on 2026-11-23, so the vendored constraint bundles stand. 1.5.16 changes
  `pluginControls.pluginsDisabled` in managed admin settings, which agnix
  does not read. User-global skills in
  `~/.agents/skills` are already a detected skills root. The remainder is
  UI, ACP, OAuth and Remote Control behavior.
- Gemini CLI v0.71.1: the settings schema is byte-identical to v0.70.0.
- Kimi Code 2.7.3: no validator yet; the bundle's quoted-key set grows by 21
  feedback and MCP OAuth runtime fields.
- Pi v1.6.0 was the latest release at the time of this poll.

Spec sentinels (#1890 and the sources the same day's reproduction found
drifted on top of it): `pi-models` (two description changes against
v1.6.0, no constraint change), `antigravity-discovery`, `antigravity-rules`,
`antigravity-settings`, `pi-settings`, and the Cursor `hooks`, `mcp`,
`subagents`, `environment` and `environment-schema` docs. `pi-settings`
changed on `main` after v1.6.0: a `notices` object replaces
`showCacheMissNotices`, which is kept as a deprecated legacy setting. That
is unreleased, and PI-CFG-002 permits unknown settings, so no vendored
constraint moves until a release carries it.

## Not done

- Codex rust-v0.172.0 deprecates `features.code_mode_host_grpc` and ignores
  it with a warning. agnix has no deprecated-key rule for Codex, so the key
  is still accepted silently. A new rule is a separate change.
- `SubagentStop` has no version info for a Kiro CLI pin below 3.6.0, unlike
  KR-HK-031 and KR-HK-032.

## Verification

- `cargo fmt --all --check`: pass
- `cargo clippy -p agnix-core --all-targets -D warnings`: pass
- `cargo test -p agnix-core --test release_triage`: pass
- `cargo test -p agnix-core`: pass
- `cargo test -p agnix-cli --test rule_parity` (reads the baselines and RESEARCH-TRACKING): pass
- `node scripts/sync-rule-bookkeeping.js --check`: clean
