#!/usr/bin/env node
/*
 * Unit tests for `scripts/build-rule-manifest.js`.
 *
 * Run locally (no npm install needed):
 *   node --test scripts/build-rule-manifest.test.js
 *
 * Covered behaviors:
 *   - unfiltered mode returns every rule as a trimmed entry
 *   - tool filter keeps unscoped rules (tool: null) alongside matches
 *   - tool filter drops rules that belong to a different tool
 *   - shape is always {id, name, category, tool, version_range} with
 *     missing fields surfaced as `null` (so GLM sees a consistent shape)
 *   - parseArgs rejects unknown flags
 */

'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');

const { buildManifest, parseArgs } = require('./build-rule-manifest.js');

const sampleRules = [
  {
    id: 'AGM-001',
    name: 'Valid Markdown Structure',
    category: 'agents-md',
    evidence: { applies_to: {} },
  },
  {
    id: 'CC-SK-001',
    name: 'Some Claude Skill Rule',
    category: 'claude-skills',
    evidence: { applies_to: { tool: 'claude-code' } },
  },
  {
    id: 'MCP-025',
    name: 'Non-boolean alwaysLoad',
    category: 'mcp',
    evidence: {
      applies_to: { tool: 'claude-code', version_range: '>=2.1.121' },
    },
  },
  {
    id: 'CUR-001',
    name: 'A Cursor Rule',
    category: 'cursor',
    evidence: { applies_to: { tool: 'cursor' } },
  },
  {
    id: 'NO-EVIDENCE',
    name: 'Pathological rule with no evidence key',
    category: 'mcp',
    // `evidence` is deliberately absent - the code path must not crash.
  },
];

test('unfiltered manifest keeps every rule', () => {
  const out = buildManifest(sampleRules, null);
  assert.equal(out.length, sampleRules.length);
  assert.deepEqual(
    out.map((r) => r.id),
    ['AGM-001', 'CC-SK-001', 'MCP-025', 'CUR-001', 'NO-EVIDENCE'],
  );
});

test('tool filter keeps unscoped rules alongside matches', () => {
  const out = buildManifest(sampleRules, 'claude-code');
  const ids = out.map((r) => r.id).sort();
  // Unscoped rules (AGM-001, NO-EVIDENCE) survive the filter so GLM
  // sees cross-tool rules and doesn't miss coverage checks.
  assert.deepEqual(ids, ['AGM-001', 'CC-SK-001', 'MCP-025', 'NO-EVIDENCE']);
});

test('tool filter drops rules that belong to a different tool', () => {
  const out = buildManifest(sampleRules, 'claude-code');
  assert.ok(!out.some((r) => r.id === 'CUR-001'), 'CUR-001 must be filtered out');
});

test('every entry has the same shape with null for missing fields', () => {
  const out = buildManifest(sampleRules, null);
  for (const entry of out) {
    assert.deepEqual(
      Object.keys(entry).sort(),
      ['category', 'id', 'name', 'tool', 'version_range'],
      `entry ${entry.id} has unexpected keys`,
    );
    // Tool and version_range must be explicitly null when absent
    // rather than undefined - JSON.stringify would drop undefined.
    assert.notEqual(entry.tool, undefined);
    assert.notEqual(entry.version_range, undefined);
  }
});

test('entry with version_range is surfaced correctly', () => {
  const out = buildManifest(sampleRules, null);
  const mcp025 = out.find((r) => r.id === 'MCP-025');
  assert.equal(mcp025.tool, 'claude-code');
  assert.equal(mcp025.version_range, '>=2.1.121');
});

test('pathological rule with missing evidence returns nulls, does not crash', () => {
  const out = buildManifest(sampleRules, null);
  const patho = out.find((r) => r.id === 'NO-EVIDENCE');
  assert.equal(patho.tool, null);
  assert.equal(patho.version_range, null);
});

test('parseArgs rejects unknown flags by exit(2)', () => {
  // node:test doesn't have a clean "assert process.exit" helper; spawn
  // the script in a child to observe. We test the function contract via
  // a stubbed process.exit so the test file stays single-process.
  const origExit = process.exit;
  const origErr = console.error;
  let exitCode = null;
  process.exit = (c) => { exitCode = c; throw new Error('exit_trap'); };
  console.error = () => {}; // swallow the expected "unknown flag" message
  try {
    assert.throws(() => parseArgs(['--nope']), /exit_trap/);
    assert.equal(exitCode, 2);
  } finally {
    process.exit = origExit;
    console.error = origErr;
  }
});

test('parseArgs accepts known flags and returns defaults for the rest', () => {
  const parsed = parseArgs(['--tool=claude-code', '--pretty']);
  assert.equal(parsed.tool, 'claude-code');
  assert.equal(parsed.pretty, true);
  assert.equal(parsed.out, null);
});

test('parseArgs rejects bare --tool (no value) with exit(2)', () => {
  // Regression guard: a bare `--tool` used to silently become the
  // unfiltered manifest, masking operator typos. Must now fail fast.
  const origExit = process.exit;
  const origErr = console.error;
  let exitCode = null;
  process.exit = (c) => { exitCode = c; throw new Error('exit_trap'); };
  console.error = () => {};
  try {
    assert.throws(() => parseArgs(['--tool']), /exit_trap/);
    assert.equal(exitCode, 2);
  } finally {
    process.exit = origExit;
    console.error = origErr;
  }
});

test('parseArgs rejects bare --out (no value) with exit(2)', () => {
  const origExit = process.exit;
  const origErr = console.error;
  let exitCode = null;
  process.exit = (c) => { exitCode = c; throw new Error('exit_trap'); };
  console.error = () => {};
  try {
    assert.throws(() => parseArgs(['--out']), /exit_trap/);
    assert.equal(exitCode, 2);
  } finally {
    process.exit = origExit;
    console.error = origErr;
  }
});

test('parseArgs rejects empty string --tool= with exit(2)', () => {
  // Edge case: `--tool=` (trailing equals, no value) parses `value`
  // as "". That's still ambiguous operator intent, so reject.
  const origExit = process.exit;
  const origErr = console.error;
  let exitCode = null;
  process.exit = (c) => { exitCode = c; throw new Error('exit_trap'); };
  console.error = () => {};
  try {
    assert.throws(() => parseArgs(['--tool=']), /exit_trap/);
    assert.equal(exitCode, 2);
  } finally {
    process.exit = origExit;
    console.error = origErr;
  }
});
