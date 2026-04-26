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
 *       ## Agnix-relevant changes
 *       ## Rule candidates (if any)
 *       ## Irrelevant (UI, perf, telemetry, etc.)
 *     Used by tool-release-watch.yml to pre-triage the per-tool issue so
 *     a human reads a summary instead of the full changelog.
 *
 * Surface and defaults distilled from github.com/avifenesh/cairn
 * internal/llm/glm.go (z.ai coding-paas endpoint, Bearer auth, OpenAI-
 * compatible chat/completions shape).
 *
 * Usage:
 *   echo "<html>" | node scripts/glm-extract.js <tool_display_name> <version> <source_url>
 *   echo "<notes>" | node scripts/glm-extract.js --mode=agnix-triage --interests-json=/tmp/interests.json <tool_display_name> <version> <source_url>
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

// Parse argv: collect flags into an options object + leave positional args.
const argv = process.argv.slice(2);
const flags = {};
const positional = [];
for (const arg of argv) {
  if (arg.startsWith('--')) {
    const eq = arg.indexOf('=');
    if (eq >= 0) {
      flags[arg.slice(2, eq)] = arg.slice(eq + 1);
    } else {
      flags[arg.slice(2)] = true;
    }
  } else {
    positional.push(arg);
  }
}

const mode = flags.mode || 'extract';
const [toolDisplay, version, sourceUrl] = positional;
if (!toolDisplay || !version || !sourceUrl) {
  console.error('usage: glm-extract.js [--mode=extract|agnix-triage] [--interests-json=<path>] <tool_display_name> <version> <source_url>');
  process.exit(2);
}
if (!['extract', 'agnix-triage'].includes(mode)) {
  console.error(`unknown --mode=${mode}; expected 'extract' or 'agnix-triage'`);
  process.exit(2);
}

const apiKey = process.env.GLM_API_KEY;
if (!apiKey) {
  console.error('GLM_API_KEY env var is required');
  process.exit(2);
}

const model = process.env.GLM_MODEL || 'glm-5';
const baseUrl = (process.env.GLM_BASE_URL || 'https://api.z.ai/api/coding/paas/v4').replace(/\/$/, '');

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

async function readStdin() {
  return new Promise((resolve, reject) => {
    let buf = '';
    process.stdin.setEncoding('utf8');
    process.stdin.on('data', (chunk) => { buf += chunk; });
    process.stdin.on('end', () => resolve(buf));
    process.stdin.on('error', reject);
  });
}

function buildExtractPrompt(html) {
  return [
    `Extract the release notes for ${toolDisplay} ${version} from the page below.`,
    '',
    'Reply as concise markdown with these sections:',
    '## What changed',
    '## Likely impact on agnix rules (one line; "none likely" if unclear)',
    '',
    `If the exact version is not on the page, summarize the most recent release instead and note that ${version} was not present.`,
    'No preamble, no commentary about the HTML structure or extraction process.',
    '',
    `Source URL: ${sourceUrl}`,
    '',
    'Page content:',
    html,
  ].join('\n');
}

function buildAgnixTriagePrompt(notes, interests) {
  const surfaces = (interests.config_surfaces || []).map((s) => `- \`${s}\``).join('\n') || '- (none declared)';
  const relevant = (interests.relevant || []).map((s) => `- ${s}`).join('\n') || '- (none declared)';
  const irrelevant = (interests.irrelevant || []).map((s) => `- ${s}`).join('\n') || '- (none declared)';
  return [
    `You are triaging release notes for ${toolDisplay} ${version} from the perspective of agnix, a linter for AI coding-tool config files.`,
    '',
    `Agnix validates these ${toolDisplay} config surfaces:`,
    surfaces,
    '',
    'Items that matter to agnix (would likely require a validator update):',
    relevant,
    '',
    'Items that do NOT matter to agnix (safe to ignore):',
    irrelevant,
    '',
    'Read the release notes below and classify each bullet point. Reply as concise markdown with these sections in order. Use #### (four hashes) for section headers so they nest under the outer ### Agnix Triage section that wraps this output in the GitHub issue body:',
    '',
    '#### Agnix-relevant changes',
    'List each change-type relevant item as a single bullet. Quote the exact phrase from the notes in backticks. If none, write `_None - this release is agnix-irrelevant._`.',
    '',
    '#### Rule candidates',
    'If any relevant change suggests a new validation rule, propose it as `- <rule name>: <one-line description>`. If none, write `_None._`.',
    '',
    '#### Irrelevant changes (not reviewed)',
    'One-line summary like `5 items: model additions, UI polish, bug fixes`. Do not enumerate.',
    '',
    'Rules: no preamble, no commentary about extraction, be strict about relevance - when in doubt, classify as irrelevant. Do not fabricate items that are not in the notes.',
    '',
    `Source URL: ${sourceUrl}`,
    '',
    'Release notes:',
    notes,
  ].join('\n');
}

(async () => {
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
    prompt = buildAgnixTriagePrompt(stdin, interests);
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
})();
