# Antigravity CLI 1.4.10 and OpenCode 2.2.1

Reviewed on 2026-10-29 for #1747 and #1748. The rule catalog remains at 757 IDs. This maintenance change adds no rule and does not cut an Agnix release.

## Antigravity CLI 1.4.10

The [official installer manifest](https://antigravity-cli-auto-updater-974169037036.us-central1.run.app/manifests/linux_amd64.json) reports 1.4.10. Release notes were read with the official quota-free `agy changelog` command, documented in [CLI changelog](https://antigravity.google/docs/cli/commands/changelog).

All six entries concern runtime behavior: terminal streaming, startup memory reclamation, compaction-size accounting, subagent context retention, image request preparation and isolated browser snapshots. None changes a static agent configuration field. The published [settings](https://antigravity.google/schemas/v1/settings.json) and [plugin](https://antigravity.google/schemas/v1/plugin.json) schemas are byte-identical to the sources used for Agnix 0.60.0:

| Contract | SHA-256 |
|---|---|
| Settings source | `6d4ab641fff729c0ec416afc8c96ccb414d9f438fe3f28fec294086a8f824877` |
| Plugin source | `0eea5e483af6f03fa099f32b065d2924a323f1a616ed0323cfa0b7a74ff9543c` |

No validator change is needed. Advance the release baseline to 1.4.10 and replace the unavailable `/docs/release-notes` publication link with the official CLI changelog documentation. This is a static-contract review, not a claim that Agnix tests those runtime fixes.

## OpenCode 2.2.1

Read the [published release](https://github.com/anomalyco/opencode/releases/tag/v2.2.1), the complete v2.2.0/v2.2.1 Git trees and changed configuration sources. The trees contain 9,961 and 10,098 entries and are not truncated. The ordinary compare endpoint returns only 300 files, so it is insufficient for this range.

The tagged [root config](https://github.com/anomalyco/opencode/blob/v2.2.1/packages/schema/src/config.ts) adds `timezone`, an optional string for scheduled tasks. The [normalizer](https://github.com/anomalyco/opencode/blob/v2.2.1/packages/core/src/config/normalize.ts) loads it. Agnix previously reported OC-004 for this valid key. Add it to the known-key and V2-marker sets, preserving the warning on OpenCode 1 and for misspelled unknown keys. The vendor schema does not enumerate valid IANA names, so this change does not invent timezone-name validation.

The [experimental schema](https://github.com/anomalyco/opencode/blob/v2.2.1/packages/schema/src/config/experimental.ts) adds `image_generate` (boolean or object with an optional model selection) and boolean `schedules`. Agnix already accepts these fields in its open experimental object. Compatibility coverage exercises both image-generation forms alongside `timezone`.

The [provider schema](https://github.com/anomalyco/opencode/blob/v2.2.1/packages/schema/src/config/provider.ts) removes the known prompt-cache setting from variant-specific settings while retaining the open JSON record. Agnix does not currently validate those nested variant settings, so this creates no unknown-key false positive or additional enforced constraint. Policy/service reload and provider SDK export changes are runtime or SDK behavior, not new recipe field shapes.

The bot's relevance summary missed `timezone`; it was not used as evidence for accepting the release. Advance only the reviewed V2 baseline to v2.2.1. OpenCode 1 remains on its separate release line.
