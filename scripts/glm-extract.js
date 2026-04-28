#!/usr/bin/env node
/*
 * Thin GLM chat-completion client for the tool-release watcher.
 *
 * Two modes, selected via --mode=<name>:
 *
 *   --mode=extract (default)
 *     Reads HTML/text from stdin, asks GLM to extract release notes for
 *     the named tool + version. Output is markdown with:
 *       ## What changed
 *       ## Likely impact on agnix rules
 *     Used by kiro/windsurf where upstream provides an HTML changelog
 *     page rather than a structured GitHub release body.
 *
 *   --mode=agnix-triage
 *     Reads release-notes markdown from stdin (already extracted), plus
 *     a `--interests-json=<file>` that describes what agnix cares about
 *     for this tool (which config files, which change-types matter).
 *     Asks GLM to filter the notes down to just validator-relevant items
 *     + rule candidates. Output is markdown with:
 *       ## Agnix-relevant changes  (each bullet cites the matching interest)
 *       ## Already-covered changes (when --rules-manifest is provided)
 *       ## Rule candidates (if any)
 *       ## Irrelevant (UI, perf, telemetry, etc.)
 *     Used by tool-release-watch.yml to pre-triage the per-tool issue so
 *     a human reads a summary instead of the full changelog.
 *
 *     Optional `--rules-manifest=<file>` passes a compact rule catalog
 *     (built by scripts/build-rule-manifest.js) so GLM can (a) classify
 *     changes as already-covered instead of listing them as new, and
 *     (b) propose next-in-sequence rule ids (e.g. MCP-025 after
 *     MCP-001..024) instead of inventing freeform names. The workflow
 *     builds this manifest per-tool and passes it automatically; the
 *     prompt degrades gracefully to the un-grounded form when absent.
 *
 * Surface and defaults distilled from github.com/avifenesh/cairn
 * internal/llm/glm.go (z.ai coding-paas endpoint, Bearer auth, OpenAI-
 * compatible chat/completions shape).
 *
 * Usage:
 *   echo "<html>" | node scripts/glm-extract.js <tool_display_name> <version> <source_url>
 *   echo "<notes>" | node scripts/glm-extract.js --mode=agnix-triage --interests-json=/tmp/interests.json [--rules-manifest=/tmp/rules.json] <tool_display_name> <version> <source_url>
 *
 * Env:
 *   GLM_API_KEY  required - z.ai key in "id.secret" format
 *   GLM_MODEL    optional - defaults to glm-5 (mid-tier coding model, ~22s for
 *                this workload). Other options:
 *                  - glm-4.7  - older; observed >120s timeouts in 2026-04 testing
 *                  - glm-5.1  - current flagship; heaviest on quota
 *                  - glm-5-turbo - per cairn's default; not separately benchmarked here
 *   GLM_BASE_URL optional - defaults to https://api.z.ai/api/coding/paas/v4
 *
 * Exit codes:
 *   0 - success, markdown on stdout
 *   1 - GLM HTTP error (message on stderr)
 *   2 - missing GLM_API_KEY, required argv, or --interests-json when mode=agnix-triage
 *
 * The watcher treats any non-zero exit OR empty stdout as "fall back to stub".
 */

'use strict';

const fs = require('fs');

// Char budget for stdin payload sent to GLM - applies to both HTML
// (extract mode) and markdown release notes (agnix-triage mode).
const INPUT_BUDGET = 80_000;
const MAX_TOKENS = 4096;
const TEMPERATURE = 0.3; // extraction task, prefer determinism

// Module-scoped argv/env holders. Populated by runCli() when invoked
// directly; left undefined when required as a library (so the prompt
// builders can be unit-tested without triggering the CLI's process.exit
// on "missing positional arg"). Builders default their `ctx` param to
// these so the CLI path stays a one-liner.
let flags = {};
let toolDisplay;
let version;
let sourceUrl;
let mode;
let apiKey;
let model;
let baseUrl;

/**
 * Load the changes_of_interest descriptor for the target tool. Schema:
 *   {
 *     "config_surfaces": ["path/to/config.toml", ".tool/settings.json"],
 *     "relevant": ["new/renamed config keys", "hook event names", ...],
 *     "irrelevant": ["UI polish", "model additions", ...]
 *   }
 * The prompt builder passes these verbatim so the operator can tune per
 * tool without editing this script.
 */
function loadInterests() {
  const raw = flags['interests-json'];
  // Must be a non-empty string path. Bare `--interests-json` with no `=`
  // would parse to boolean `true`; reject that so we fail loudly instead
  // of letting `fs.readFileSync(true)` crash with an obscure error.
  if (!raw || typeof raw !== 'string') {
    console.error('--mode=agnix-triage requires --interests-json=<path> with a file path value');
    process.exit(2);
  }
  try {
    return JSON.parse(fs.readFileSync(raw, 'utf8'));
  } catch (err) {
    console.error(`failed to read ${raw}: ${err.message}`);
    process.exit(2);
  }
}

/**
 * Load the compact rule manifest produced by
 * `scripts/build-rule-manifest.js`. Optional - when absent the prompt
 * still works but the model has no grounding for rule-id proposals or
 * "already covered by MCP-*" reasoning.
 *
 * Returns `null` when the flag is unset or the file can't be read
 * (non-fatal: we'd rather produce a degraded triage than lose the
 * release notes entirely). Invalid JSON is also non-fatal but logged.
 */
function loadRulesManifest() {
  const raw = flags['rules-manifest'];
  if (!raw || typeof raw !== 'string') {
    return null;
  }
  try {
    const data = JSON.parse(fs.readFileSync(raw, 'utf8'));
    if (!Array.isArray(data)) {
      console.error(`rules manifest at ${raw} is not a JSON array - ignoring`);
      return null;
    }
    return data;
  } catch (err) {
    console.error(`failed to read rules manifest ${raw}: ${err.message} - ignoring`);
    return null;
  }
}

async function readStdin() {
  return new Promise((resolve, reject) => {
    let buf = '';
    process.stdin.setEncoding('utf8');
    process.stdin.on('data', (chunk) => { buf += chunk; });
    process.stdin.on('end', () => resolve(buf));
    process.stdin.on('error', reject);
  });
}

function buildExtractPrompt(html, ctx = { toolDisplay, version, sourceUrl }) {
  return [
    `Extract the release notes for ${ctx.toolDisplay} ${ctx.version} from the page below.`,
    '',
    'Reply as concise markdown with these sections:',
    '## What changed',
    '## Likely impact on agnix rules (one line; "none likely" if unclear)',
    '',
    `If the exact version is not on the page, summarize the most recent release instead and note that ${ctx.version} was not present.`,
    'No preamble, no commentary about the HTML structure or extraction process.',
    '',
    `Source URL: ${ctx.sourceUrl}`,
    '',
    'Page content:',
    html,
  ].join('\n');
}

function buildAgnixTriagePrompt(notes, interests, rulesManifest, ctx = { toolDisplay, version, sourceUrl }) {
  const surfaces = (interests.config_surfaces || []).map((s) => `- \`${s}\``).join('\n') || '- (none declared)';
  const relevant = (interests.relevant || []).map((s) => `- ${s}`).join('\n') || '- (none declared)';
  const irrelevant = (interests.irrelevant || []).map((s) => `- ${s}`).join('\n') || '- (none declared)';

  // Rule-catalog grounding: give GLM the existing rule manifest so its
  // "Rule candidates" section proposes real next-in-sequence ids
  // (e.g., MCP-025 after MCP-001..024) instead of inventing freeform
  // names, and its "Agnix-relevant changes" section can recognize when
  // an existing rule already covers the change.
  //
  // When the manifest is absent (no --rules-manifest flag or load
  // failed) we fall back to the un-grounded prompt - the workflow
  // keeps working, just with the same quality as before.
  const rulesBlock = rulesManifest
    ? [
        '',
        'Agnix already ships these validation rules (trimmed to id + name + category + scope for brevity). When a release note describes behavior these rules already cover, classify the change as irrelevant and name the covering rule id. When it describes new behavior in an existing family, the next-free id in that family is a natural rule candidate - use the exact prefix and the next sequential number.',
        '',
        '```json',
        JSON.stringify(rulesManifest),
        '```',
      ].join('\n')
    : '';

  // Use `null` as a sentinel for "skip this line" (a conditional line
  // whose condition was false). Empty strings are meaningful paragraph
  // separators and must survive the join.
  const lines = [
    `You are triaging release notes for ${ctx.toolDisplay} ${ctx.version} from the perspective of agnix, a linter for AI coding-tool config files.`,
    '',
    `Agnix validates these ${ctx.toolDisplay} config surfaces:`,
    surfaces,
    '',
    'Items that matter to agnix (would likely require a validator update):',
    relevant,
    '',
    'Items that do NOT matter to agnix (safe to ignore):',
    irrelevant,
    rulesBlock || null,
    '',
    'Read the release notes below and classify each bullet point. Reply as concise markdown with these sections in order. Use #### (four hashes) for section headers so they nest under the outer ### Agnix Triage section that wraps this output in the GitHub issue body:',
    '',
    '#### Agnix-relevant changes',
    'List each change-type relevant item as ONE bullet shaped exactly like:',
    '  - `<exact phrase from notes>` -- matches: `<verbatim line from "Items that matter">`',
    'The double-dash ` -- ` separator and the `matches:` label are required so a human reviewer can audit the mapping. If no item matches ANY "Items that matter" line, write `_None - this release is agnix-irrelevant._`. Never list an item without naming a matching interest; never invent a new interest that is not in the list above.',
    rulesManifest
      ? 'If a change is already covered by an existing rule from the manifest above, do NOT list it here; mention it in the "Already-covered" section below instead.'
      : null,
    '',
    rulesManifest ? '#### Already-covered changes' : null,
    rulesManifest
      ? 'List each change whose behavior is already enforced by an existing rule, shaped `- <exact phrase from notes> -- covered by <RULE-ID>`. If none, write `_None._`.'
      : null,
    rulesManifest ? '' : null,
    '#### Rule candidates',
    rulesManifest
      ? 'If any relevant change suggests a NEW validation rule (not already in the manifest above), propose it as `- <NEXT-FREE-ID>: <one-line description>`. Pick the id by finding the highest existing number for the matching category prefix in the manifest and adding 1 (e.g., if the manifest ends at MCP-024, propose MCP-025). If none, write `_None._`.'
      : 'If any relevant change suggests a new validation rule, propose it as `- <rule name>: <one-line description>`. If none, write `_None._`.',
    '',
    '#### Irrelevant changes (not reviewed)',
    'One-line summary like `5 items: model additions, UI polish, bug fixes`. Do not enumerate.',
    '',
    rulesManifest
      ? 'Rules: no preamble, no commentary about extraction, be strict about relevance - when in doubt, classify as irrelevant. Do not fabricate items that are not in the notes. Do not cite manifest rule ids that are not in the manifest above.'
      : 'Rules: no preamble, no commentary about extraction, be strict about relevance - when in doubt, classify as irrelevant. Do not fabricate items that are not in the notes.',
    '',
    `Source URL: ${ctx.sourceUrl}`,
    '',
    'Release notes:',
    notes,
  ];
  return lines.filter((line) => line !== null).join('\n');
}

function parseArgv(argv) {
  const parsedFlags = {};
  const positional = [];
  for (const arg of argv) {
    if (arg.startsWith('--')) {
      const eq = arg.indexOf('=');
      if (eq >= 0) {
        parsedFlags[arg.slice(2, eq)] = arg.slice(eq + 1);
      } else {
        parsedFlags[arg.slice(2)] = true;
      }
    } else {
      positional.push(arg);
    }
  }
  return { flags: parsedFlags, positional };
}

async function runCli() {
  const parsed = parseArgv(process.argv.slice(2));
  flags = parsed.flags;
  [toolDisplay, version, sourceUrl] = parsed.positional;
  mode = flags.mode || 'extract';

  if (!toolDisplay || !version || !sourceUrl) {
    console.error('usage: glm-extract.js [--mode=extract|agnix-triage] [--interests-json=<path>] [--rules-manifest=<path>] <tool_display_name> <version> <source_url>');
    process.exit(2);
  }
  if (!['extract', 'agnix-triage'].includes(mode)) {
    console.error(`unknown --mode=${mode}; expected 'extract' or 'agnix-triage'`);
    process.exit(2);
  }

  apiKey = process.env.GLM_API_KEY;
  if (!apiKey) {
    console.error('GLM_API_KEY env var is required');
    process.exit(2);
  }

  model = process.env.GLM_MODEL || 'glm-5';
  baseUrl = (process.env.GLM_BASE_URL || 'https://api.z.ai/api/coding/paas/v4').replace(/\/$/, '');

  const stdin = (await readStdin()).slice(0, INPUT_BUDGET);
  if (!stdin.trim()) {
    console.error('stdin was empty - nothing to process');
    process.exit(2);
  }

  let prompt;
  if (mode === 'extract') {
    prompt = buildExtractPrompt(stdin);
  } else {
    const interests = loadInterests();
    const rulesManifest = loadRulesManifest();
    prompt = buildAgnixTriagePrompt(stdin, interests, rulesManifest);
  }

  let res;
  try {
    res = await fetch(`${baseUrl}/chat/completions`, {
      method: 'POST',
      headers: {
        Authorization: `Bearer ${apiKey}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        model,
        messages: [{ role: 'user', content: prompt }],
        stream: false,
        max_tokens: MAX_TOKENS,
        temperature: TEMPERATURE,
      }),
      signal: AbortSignal.timeout(180_000),
    });
  } catch (err) {
    console.error(`GLM fetch failed: ${err.message}`);
    process.exit(1);
  }

  if (!res.ok) {
    const body = await res.text().catch(() => '');
    console.error(`GLM HTTP ${res.status}: ${body.slice(0, 500)}`);
    process.exit(1);
  }

  let data;
  try {
    data = await res.json();
  } catch (err) {
    console.error(`GLM response was not JSON: ${err.message}`);
    process.exit(1);
  }

  const content = data?.choices?.[0]?.message?.content || '';
  if (!content.trim()) {
    console.error('GLM returned empty content');
    process.exit(1);
  }
  process.stdout.write(content);
}

// Export the prompt builders for unit tests. `ctx` lets tests pass
// tool/version/source without triggering the CLI's argv parsing.
module.exports = {
  buildExtractPrompt,
  buildAgnixTriagePrompt,
  parseArgv,
};

if (require.main === module) {
  runCli();
}
