#!/usr/bin/env node
/*
 * Thin GLM chat-completion client for the tool-release watcher.
 *
 * Reads HTML/text from stdin, asks GLM to extract release notes, writes
 * markdown to stdout. Designed for one-shot extraction - no streaming,
 * no tool calls, no SSE, no retries (the watcher falls back to a stub
 * link if this fails, and tomorrow's run tries again).
 *
 * Surface and defaults distilled from github.com/avifenesh/cairn
 * internal/llm/glm.go (z.ai coding-paas endpoint, Bearer auth, OpenAI-
 * compatible chat/completions shape).
 *
 * Usage:
 *   echo "<html>" | node scripts/glm-extract.js <tool_display_name> <version> <source_url>
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
 *   2 - missing GLM_API_KEY or required argv
 *
 * The watcher treats any non-zero exit OR empty stdout as "fall back to stub".
 */

'use strict';

const HTML_BUDGET = 80_000; // chars of input HTML to send to GLM
const MAX_TOKENS = 4096;
const TEMPERATURE = 0.3; // extraction task, prefer determinism

const [, , toolDisplay, version, sourceUrl] = process.argv;
if (!toolDisplay || !version || !sourceUrl) {
  console.error('usage: glm-extract.js <tool_display_name> <version> <source_url>');
  process.exit(2);
}

const apiKey = process.env.GLM_API_KEY;
if (!apiKey) {
  console.error('GLM_API_KEY env var is required');
  process.exit(2);
}

const model = process.env.GLM_MODEL || 'glm-5';
const baseUrl = (process.env.GLM_BASE_URL || 'https://api.z.ai/api/coding/paas/v4').replace(/\/$/, '');

async function readStdin() {
  return new Promise((resolve, reject) => {
    let buf = '';
    process.stdin.setEncoding('utf8');
    process.stdin.on('data', chunk => { buf += chunk; });
    process.stdin.on('end', () => resolve(buf));
    process.stdin.on('error', reject);
  });
}

(async () => {
  const html = (await readStdin()).slice(0, HTML_BUDGET);
  if (!html.trim()) {
    console.error('stdin was empty - nothing to extract');
    process.exit(2);
  }

  const prompt = [
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

  let res;
  try {
    res = await fetch(`${baseUrl}/chat/completions`, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${apiKey}`,
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
