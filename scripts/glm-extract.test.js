#!/usr/bin/env node
/*
 * Unit tests for the prompt builders in `scripts/glm-extract.js`.
 *
 * Run:
 *   node --test scripts/glm-extract.test.js
 *
 * These lock in the prompt contract so subtle drift (missing the
 * `-- matches:` citation, dropping the `#### Already-covered` section
 * when the manifest is present, leaking `[object Object]` into the
 * embedded manifest) breaks the test instead of silently degrading
 * triage quality.
 */

'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');

const {
  buildExtractPrompt,
  buildAgnixTriagePrompt,
  buildRequestBody,
  parseArgv,
} = require('./glm-extract.js');

const CTX = {
  toolDisplay: 'Claude Code',
  version: 'v2.1.122',
  sourceUrl: 'https://example.com/release',
};

const SAMPLE_INTERESTS = {
  config_surfaces: ['CLAUDE.md', '.claude/settings.json'],
  relevant: ['MCP server definition shape', 'Hook event names'],
  irrelevant: ['UI polish', 'Model additions'],
};

const SAMPLE_MANIFEST = [
  {
    id: 'MCP-024',
    name: 'Empty MCP Server Configuration',
    category: 'mcp',
    tool: null,
    version_range: null,
  },
  {
    id: 'MCP-025',
    name: 'Non-boolean alwaysLoad in MCP Server Config',
    category: 'mcp',
    tool: 'claude-code',
    version_range: '>=2.1.121',
  },
];

const SAMPLE_NOTES = '- Added alwaysLoad option to MCP server config\n- UI scroll fix';

test('buildExtractPrompt inlines tool+version+source', () => {
  const out = buildExtractPrompt('<html>notes</html>', CTX);
  assert.match(out, /Claude Code v2\.1\.122/);
  assert.match(out, /Source URL: https:\/\/example\.com\/release/);
  assert.match(out, /## What changed/);
});

test('buildAgnixTriagePrompt without manifest omits Already-covered section', () => {
  const out = buildAgnixTriagePrompt(SAMPLE_NOTES, SAMPLE_INTERESTS, null, CTX);
  assert.ok(!out.includes('#### Already-covered'), 'Already-covered must not appear without manifest');
  assert.ok(!out.includes('manifest above'), 'must not reference a manifest that was not provided');
  // Core sections still there:
  assert.match(out, /#### Agnix-relevant changes/);
  assert.match(out, /#### Rule candidates/);
  assert.match(out, /#### Irrelevant changes \(not reviewed\)/);
});

test('buildAgnixTriagePrompt with manifest adds Already-covered section', () => {
  const out = buildAgnixTriagePrompt(SAMPLE_NOTES, SAMPLE_INTERESTS, SAMPLE_MANIFEST, CTX);
  assert.match(out, /#### Already-covered changes/);
  assert.match(out, /#### Rule candidates/);
  // Agnix-relevant + Already-covered + Rule candidates + Irrelevant = 4 sections
  const sectionCount = (out.match(/^#### /gm) || []).length;
  assert.equal(sectionCount, 4, 'expected 4 #### sections with manifest');
});

test('citation requirement is present with or without manifest', () => {
  // The `-- matches: ...` citation requirement was the core #837 fix
  // and must appear in both code paths so the reviewer can always
  // audit why a bullet was surfaced.
  const withManifest = buildAgnixTriagePrompt(SAMPLE_NOTES, SAMPLE_INTERESTS, SAMPLE_MANIFEST, CTX);
  const withoutManifest = buildAgnixTriagePrompt(SAMPLE_NOTES, SAMPLE_INTERESTS, null, CTX);
  for (const prompt of [withManifest, withoutManifest]) {
    assert.match(prompt, /-- matches:/, 'prompt must require "-- matches:" citation');
    assert.match(prompt, /Never list an item without naming a matching interest/);
  }
});

test('manifest is embedded as valid JSON that round-trips', () => {
  const out = buildAgnixTriagePrompt(SAMPLE_NOTES, SAMPLE_INTERESTS, SAMPLE_MANIFEST, CTX);
  const m = out.match(/```json\n(\[.*?\])\n```/s);
  assert.ok(m, 'expected a fenced JSON block');
  const roundTripped = JSON.parse(m[1]);
  assert.deepEqual(roundTripped, SAMPLE_MANIFEST, 'embedded manifest must round-trip exactly');
});

test('manifest prompt explains next-free id computation', () => {
  const out = buildAgnixTriagePrompt(SAMPLE_NOTES, SAMPLE_INTERESTS, SAMPLE_MANIFEST, CTX);
  assert.match(out, /highest existing number for the matching category prefix/);
  assert.match(out, /MCP-024/); // example used in the prompt
  assert.match(out, /MCP-025/);
});

test('interests are rendered as bulleted lists in source order', () => {
  const out = buildAgnixTriagePrompt(SAMPLE_NOTES, SAMPLE_INTERESTS, null, CTX);
  // Each interest should appear as "- <line>" with order preserved.
  const relevantBlock = out.slice(out.indexOf('Items that matter'), out.indexOf('Items that do NOT'));
  const mccPos = relevantBlock.indexOf('MCP server definition shape');
  const hookPos = relevantBlock.indexOf('Hook event names');
  assert.ok(mccPos >= 0 && hookPos > mccPos, 'relevant interests must appear in declared order');
});

test('missing interest arrays degrade to "(none declared)" rather than crashing', () => {
  const out = buildAgnixTriagePrompt(SAMPLE_NOTES, {}, null, CTX);
  assert.match(out, /\(none declared\)/);
});

test('parseArgv splits flags and positionals correctly', () => {
  const { flags, positional } = parseArgv([
    '--mode=agnix-triage',
    '--interests-json=/tmp/i.json',
    '--rules-manifest=/tmp/r.json',
    'Claude Code',
    'v2.1.122',
    'https://example.com',
  ]);
  assert.equal(flags.mode, 'agnix-triage');
  assert.equal(flags['interests-json'], '/tmp/i.json');
  assert.equal(flags['rules-manifest'], '/tmp/r.json');
  assert.deepEqual(positional, ['Claude Code', 'v2.1.122', 'https://example.com']);
});

test('parseArgv handles bare flags as boolean true', () => {
  const { flags } = parseArgv(['--pretty']);
  assert.equal(flags.pretty, true);
});

test('buildRequestBody keeps thinking enabled', () => {
  const body = buildRequestBody('glm-5.3', 'prompt text');
  // Reasoning improves the "does this touch a validated config surface?"
  // judgement, so it stays on.
  assert.deepEqual(body.thinking, { type: 'enabled' });
});

test('buildRequestBody budgets enough output for reasoning plus answer', () => {
  const body = buildRequestBody('glm-5.3', 'prompt text');
  // Reasoning tokens come out of max_tokens. Measured reasoning on real triage
  // prompts is 1.9k-5.2k; at 4096 the answer was intermittently truncated away
  // entirely (finish_reason "length", empty content).
  assert.ok(
    body.max_tokens >= 16_384,
    `max_tokens ${body.max_tokens} leaves too little room for reasoning + answer`
  );
});

test('buildRequestBody sends a non-streaming single-turn request', () => {
  const body = buildRequestBody('glm-5-turbo', 'prompt text');
  assert.equal(body.model, 'glm-5-turbo');
  assert.equal(body.stream, false);
  assert.deepEqual(body.messages, [{ role: 'user', content: 'prompt text' }]);
  assert.equal(typeof body.max_tokens, 'number');
  assert.ok(body.max_tokens > 0);
  assert.equal(typeof body.temperature, 'number');
});
