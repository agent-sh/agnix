#!/usr/bin/env node
/*
 * Build a compact rule manifest for GLM triage grounding (#837).
 *
 * Reads `knowledge-base/rules.json` and emits a trimmed JSON array of
 * `{id, name, category, tool, version_range}` entries. The full file is
 * ~1.5 MB and includes evidence URLs, fix metadata, and code examples
 * that GLM doesn't need to judge "is this release agnix-relevant, and
 * does any existing rule already cover it?".
 *
 * Filtering options:
 *   --tool=<id>        Keep rules that apply to this tool OR to no
 *                      specific tool (unscoped / cross-tool). Unscoped
 *                      rules (e.g. AGENTS.md parsers, MCP core) always
 *                      matter regardless of which tool's release we're
 *                      triaging, so we never drop them.
 *   --out=<path>       Write JSON to this path instead of stdout.
 *   --pretty           Pretty-print (default: minified to save tokens).
 *
 * Output shape (always a JSON array):
 *   [
 *     {
 *       "id": "MCP-025",
 *       "name": "Non-boolean alwaysLoad in MCP Server Config",
 *       "category": "mcp",
 *       "tool": "claude-code",
 *       "version_range": ">=2.1.121"
 *     },
 *     ...
 *   ]
 *
 * Fields `tool` and `version_range` are `null` when absent rather than
 * omitted, so GLM sees the shape is consistent.
 *
 * Exit codes:
 *   0 - success
 *   1 - I/O or parse error
 *   2 - bad argv
 */

'use strict';

const fs = require('fs');
const path = require('path');

function parseArgs(argv) {
  const out = { tool: null, out: null, pretty: false };
  for (const raw of argv) {
    if (!raw.startsWith('--')) {
      console.error(`unexpected positional argument: ${raw}`);
      process.exit(2);
    }
    const eq = raw.indexOf('=');
    const key = eq >= 0 ? raw.slice(2, eq) : raw.slice(2);
    const value = eq >= 0 ? raw.slice(eq + 1) : true;
    if (key === 'tool') out.tool = typeof value === 'string' ? value : null;
    else if (key === 'out') out.out = typeof value === 'string' ? value : null;
    else if (key === 'pretty') out.pretty = true;
    else {
      console.error(`unknown flag: --${key}`);
      process.exit(2);
    }
  }
  return out;
}

function loadRules() {
  // Resolve rules.json relative to the repo root, not the CWD - the
  // sync/check workflows invoke scripts from various places and this
  // needs to work from all of them.
  const rulesPath = path.resolve(__dirname, '..', 'knowledge-base', 'rules.json');
  try {
    const raw = fs.readFileSync(rulesPath, 'utf8');
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed.rules)) {
      throw new Error('rules.json missing `rules` array');
    }
    return parsed.rules;
  } catch (err) {
    console.error(`failed to load ${rulesPath}: ${err.message}`);
    process.exit(1);
  }
}

function buildManifest(rules, toolFilter) {
  return rules
    .filter((rule) => {
      if (!toolFilter) return true;
      const ruleTool = rule.evidence?.applies_to?.tool ?? null;
      // Keep unscoped rules (no tool) and rules that match the filter.
      // Unscoped rules (AGENTS.md, cross-platform, MCP core) matter
      // across tools - we don't want GLM to miss them when deciding
      // "does any existing rule already cover this change?".
      return ruleTool === null || ruleTool === toolFilter;
    })
    .map((rule) => ({
      id: rule.id,
      name: rule.name,
      category: rule.category,
      tool: rule.evidence?.applies_to?.tool ?? null,
      version_range: rule.evidence?.applies_to?.version_range ?? null,
    }));
}

function main() {
  const { tool, out, pretty } = parseArgs(process.argv.slice(2));
  const rules = loadRules();
  const manifest = buildManifest(rules, tool);
  const json = pretty ? JSON.stringify(manifest, null, 2) : JSON.stringify(manifest);
  if (out) {
    fs.writeFileSync(out, json + '\n');
  } else {
    process.stdout.write(json + '\n');
  }
}

// Export for unit tests; run only when invoked directly.
module.exports = { parseArgs, buildManifest };

if (require.main === module) {
  main();
}
