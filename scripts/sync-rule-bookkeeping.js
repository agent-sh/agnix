#!/usr/bin/env node
/**
 * sync-rule-bookkeeping.js
 *
 * Syncs every derived location downstream of `knowledge-base/rules.json`
 * so adding a rule becomes "write rules.json entry + validator + tests"
 * and the tool keeps the rest in sync automatically.
 *
 * What it touches:
 *
 * 1. `knowledge-base/rules.json`
 *    - `total_rules` is set to `rules.length`
 *    - `last_updated` is set to today (YYYY-MM-DD) when `--bump-date`
 *      is passed OR when `rules.length` changed from prior state.
 *
 * 2. `crates/agnix-rules/rules.json`
 *    - Byte-identical mirror of `knowledge-base/rules.json`.
 *
 * 3. Count phrases in every file in `COUNT_FILES`:
 *    - `CLAUDE.md`, `AGENTS.md`, `README.md`
 *    - `knowledge-base/README.md`, `knowledge-base/INDEX.md`
 *    - `plugin/commands/agnix.md`, `plugin/skills/agnix/SKILL.md`,
 *      `skills/agnix/SKILL.md`
 *    - `plugin/.claude-plugin/plugin.json` (description string)
 *
 *    "N rules" and "N validation rules" phrases are updated to match
 *    `rules.length`. Singular forms ("N rule sourced/across") are not
 *    currently used anywhere in these files; if they appear in future
 *    prose, add the pattern to `countPatterns` below.
 *    Validator count phrase "N validators" is updated when --validators N
 *    is passed (the count isn't automatically derivable without parsing
 *    Rust source).
 *
 *    CHANGELOG.md is intentionally excluded - its "N rules at release"
 *    phrases are historical and must stay frozen per-version.
 *
 * 3b. Package versions synced from `Cargo.toml` workspace version:
 *    - `plugin/.claude-plugin/plugin.json`
 *    - `npm/package.json`
 *
 *    The Rust workspace version is the single source of truth. Release
 *    commits bump Cargo.toml; this script pushes the new version into
 *    the plugin manifest and the npm wrapper so they can't drift behind.
 *
 * 4. `knowledge-base/VALIDATION-RULES.md` footer stats
 *    - `**Total Coverage**: N validation rules across M categories`
 *    - `**Certainty**: H HIGH, M MEDIUM, L LOW`
 *    - `**Auto-Fixable**: N rules (P%)`
 *    All derived from the rules.json entries. A parity test in
 *    crates/agnix-cli/tests/rule_parity.rs asserts these match, so
 *    stale values fail CI.
 *
 * 5. Regenerates website docs via `scripts/generate-docs-rules.py`
 *    unless --skip-docs is passed.
 *
 * What it does NOT do (intentional - these need human judgment):
 * - Adding rule entries to rules.json itself
 * - Writing the validator Rust source
 * - Updating rule_parity.rs valid_prefixes / categories / fixture dirs
 * - Updating registry.rs EXPECTED_BUILTIN_COUNT or MiscProvider tests
 * - Updating api_contract.rs FileType variant lists
 *
 * Those are flagged with --check which reports any stale numbers and
 * exits non-zero so CI can gate on drift.
 *
 * Usage:
 *   node scripts/sync-rule-bookkeeping.js              # apply updates
 *   node scripts/sync-rule-bookkeeping.js --check      # report drift, exit 1 if any
 *   node scripts/sync-rule-bookkeeping.js --validators=42  # also bump validator phrase
 *   node scripts/sync-rule-bookkeeping.js --bump-date  # force last_updated to today
 *   node scripts/sync-rule-bookkeeping.js --skip-docs  # don't regenerate website docs
 */

'use strict';

const fs = require('fs');
const path = require('path');
const { execFileSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..');

const KNOWLEDGE_RULES = path.join(ROOT, 'knowledge-base', 'rules.json');
const CRATE_RULES = path.join(ROOT, 'crates', 'agnix-rules', 'rules.json');

// Files whose "N rules" / "N validation rules" phrases must track
// `rules.length`. Some files (notably `knowledge-base/INDEX.md`) also
// contain per-category counts like `| Agent Skills | 12 | ... | 15 rules |`
// that would false-positive under a naive `\d+ rules` regex, so we
// specify per-file patterns instead of globbing.
//
// Every entry describes:
//   - `path`: file location
//   - `patterns`: regexes to rewrite. If omitted, defaults to the
//     historical `(\b)(\d+)( rules\b)` + `(\b)(\d+)( validation rules\b)`
//     pair (safe for files that only mention the total count).
//
// CHANGELOG.md is intentionally NOT here because its "N rules at
// release" phrases are deliberately frozen per-version.
const DEFAULT_COUNT_PATTERNS = [
  { re: /(\b)(\d+)( rules\b)/g },
  { re: /(\b)(\d+)( validation rules\b)/g },
];

// Precise patterns for files that embed per-category counts or other
// unrelated N-rules mentions. Each pattern must include exactly three
// capture groups: (prefix, number, suffix) so the rewrite logic is
// identical to DEFAULT_COUNT_PATTERNS.
//
// knowledge-base/INDEX.md anchors:
//   - "N rules with detection logic" (in headline text)
//   - "Master validation reference (N rules)"
//   - "N rules with rule IDs"
//   - "**Total** ... **N rules**" (the totals row in the category table)
//   - "Validation Rules:  N rules" (in the stats block)
//   - "N validation rules across M categories" (tagline)
const INDEX_MD_PATTERNS = [
  { re: /(\b)(\d+)( rules with detection logic\b)/g },
  { re: /(Master validation reference \()(\d+)( rules\))/g },
  { re: /(\b)(\d+)( rules with rule IDs\b)/g },
  { re: /(\*\*Total\*\*[^|]*\|\s*\*\*)(\d+)( rules\*\*\s*\|)/g },
  { re: /(Validation Rules:\s+)(\d+)( rules\b)/g },
  { re: /(\b)(\d+)( validation rules across\b)/g },
];

const COUNT_FILES = [
  { path: path.join(ROOT, 'CLAUDE.md') },
  { path: path.join(ROOT, 'AGENTS.md') },
  { path: path.join(ROOT, 'README.md') },
  { path: path.join(ROOT, 'knowledge-base', 'README.md') },
  { path: path.join(ROOT, 'knowledge-base', 'INDEX.md'), patterns: INDEX_MD_PATTERNS },
  // Plugin + standalone skill/command files. These are the user-
  // facing descriptions that Claude Code, Codex, and the marketplace
  // index serve. They were the source of repeated drift audits (#828,
  // #829, #831) because no automation covered them. Each has a single
  // "validates ... N rules" tagline so default patterns are safe.
  { path: path.join(ROOT, 'plugin', 'commands', 'agnix.md') },
  { path: path.join(ROOT, 'plugin', 'skills', 'agnix', 'SKILL.md') },
  { path: path.join(ROOT, 'skills', 'agnix', 'SKILL.md') },
  { path: path.join(ROOT, 'plugin', '.claude-plugin', 'plugin.json') },
];

// Files whose top-level `version` field must track the Cargo.toml
// workspace version. We rewrite JSON in place - minimal diff, preserves
// everything else verbatim via JSON.stringify(data, null, 2).
// `plugin/.claude-plugin/plugin.json` is ALSO in COUNT_FILES above for
// its description-string count phrase; the two passes are independent.
const VERSION_SYNC_JSON_FILES = [
  path.join(ROOT, 'plugin', '.claude-plugin', 'plugin.json'),
  path.join(ROOT, 'npm', 'package.json'),
];

const WORKSPACE_CARGO_TOML = path.join(ROOT, 'Cargo.toml');

const args = process.argv.slice(2);
const checkMode = args.includes('--check');
const bumpDate = args.includes('--bump-date');
const skipDocs = args.includes('--skip-docs');
const validatorArg = args.find((a) => a.startsWith('--validators='));
let requestedValidatorCount = null;
if (validatorArg) {
  const raw = validatorArg.slice('--validators='.length);
  // Strict: digits only, no sign, no trailing garbage, at least 1.
  if (!/^\d+$/.test(raw)) {
    console.error(
      `[ERROR] --validators= must be a positive integer (digits only), got '${raw}'`
    );
    process.exit(2);
  }
  requestedValidatorCount = parseInt(raw, 10);
  if (requestedValidatorCount < 1) {
    console.error(`[ERROR] --validators= must be at least 1, got ${requestedValidatorCount}`);
    process.exit(2);
  }
}

// --check is read-only by contract. --bump-date would write to rules.json,
// which contradicts that. Reject the combination rather than silently
// ignoring --bump-date, so CI doesn't accidentally side-effect.
if (checkMode && bumpDate) {
  console.error(
    '[ERROR] --check is read-only; --bump-date would mutate rules.json. Pick one.'
  );
  process.exit(2);
}

function today() {
  return new Date().toISOString().slice(0, 10);
}

function readJSON(file) {
  return JSON.parse(fs.readFileSync(file, 'utf8'));
}

function writeJSONPretty(file, data) {
  const serialized = JSON.stringify(data, null, 2) + '\n';
  fs.writeFileSync(file, serialized);
}

const drift = []; // collected issues for --check mode
function note(msg) {
  drift.push(msg);
}

function readText(file) {
  return fs.readFileSync(file, 'utf8');
}

function writeText(file, content) {
  fs.writeFileSync(file, content);
}

// ---- 1. knowledge-base/rules.json: total_rules + optional last_updated ----

const rulesJson = readJSON(KNOWLEDGE_RULES);
const actualRuleCount = Array.isArray(rulesJson.rules) ? rulesJson.rules.length : 0;
if (actualRuleCount === 0) {
  console.error('[ERROR] rules.json contains no rules - aborting');
  process.exit(2);
}

let rulesJsonDirty = false;
if (rulesJson.total_rules !== actualRuleCount) {
  note(
    `knowledge-base/rules.json: total_rules is ${rulesJson.total_rules}, actual rule count is ${actualRuleCount}`
  );
  if (!checkMode) {
    rulesJson.total_rules = actualRuleCount;
    rulesJsonDirty = true;
  }
}

// Sync per-category counts in the top-level categories map. New
// categories used by rules but not declared in the map are flagged as
// drift so the maintainer can add the prefix + description entry.
if (syncCategoryCounts(rulesJson)) {
  rulesJsonDirty = true;
}

// last_updated: bump if explicitly requested OR if we're already changing
// total_rules (meaning a rule was added/removed).
if (bumpDate || rulesJsonDirty) {
  const t = today();
  if (rulesJson.last_updated !== t) {
    if (checkMode && !bumpDate) {
      // In --check mode without --bump-date, stale date is informational,
      // not a drift failure. It only becomes drift if someone actually
      // changed rules without running the script.
    } else {
      rulesJson.last_updated = t;
      rulesJsonDirty = true;
    }
  }
}

if (rulesJsonDirty && !checkMode) {
  // Defense-in-depth: --check is guaranteed read-only. The explicit
  // `!checkMode` guard here backs up the early exit on `--check + --bump-date`
  // so any future code path that sets rulesJsonDirty in check mode can't
  // accidentally mutate the file.
  writeJSONPretty(KNOWLEDGE_RULES, rulesJson);
  console.log(`[OK] Updated knowledge-base/rules.json (total_rules=${actualRuleCount}, last_updated=${rulesJson.last_updated})`);
}

// ---- 2. crates/agnix-rules/rules.json: byte-identical mirror ----

const knowledgeContent = fs.readFileSync(KNOWLEDGE_RULES);
const crateContent = fs.existsSync(CRATE_RULES) ? fs.readFileSync(CRATE_RULES) : null;
if (!crateContent || !knowledgeContent.equals(crateContent)) {
  note('crates/agnix-rules/rules.json is not a byte-identical mirror of knowledge-base/rules.json');
  if (!checkMode) {
    fs.writeFileSync(CRATE_RULES, knowledgeContent);
    console.log('[OK] Synced crates/agnix-rules/rules.json');
  }
}

// ---- 3. Count phrases in COUNT_FILES ----

for (const entry of COUNT_FILES) {
  const file = entry.path;
  if (!fs.existsSync(file)) continue;
  const original = readText(file);
  const patterns = entry.patterns || DEFAULT_COUNT_PATTERNS;
  let updated = original;
  for (const { re } of patterns) {
    updated = updated.replace(re, (match, pre, num, post) => {
      const current = parseInt(num, 10);
      if (current === actualRuleCount) return match;
      note(
        `${path.relative(ROOT, file)}: "${current}${post.trimEnd()}" should be "${actualRuleCount}${post.trimEnd()}"`
      );
      return `${pre}${actualRuleCount}${post}`;
    });
  }
  if (requestedValidatorCount !== null) {
    updated = updated.replace(/(\b)(\d+)( validators\b)/g, (match, pre, num, post) => {
      const current = parseInt(num, 10);
      if (current === requestedValidatorCount) return match;
      note(
        `${path.relative(ROOT, file)}: "${current}${post}" should be "${requestedValidatorCount}${post}"`
      );
      return `${pre}${requestedValidatorCount}${post}`;
    });
  }
  if (updated !== original && !checkMode) {
    writeText(file, updated);
    console.log(`[OK] Updated count phrases in ${path.relative(ROOT, file)}`);
  }
}

// ---- 3b. Package versions synced from Cargo.toml workspace version ----
//
// The Rust workspace version in `Cargo.toml` is the single source of
// truth for the binary. Release commits bump it (see v0.22.1, v0.22.0
// etc.) but historically missed `plugin/.claude-plugin/plugin.json`
// and `npm/package.json`, which drifted 4+ minor versions behind
// before the structural fix in this script landed. Now both are
// derived from Cargo.toml and enforced in --check mode so CI + the
// pre-push hook catch the drift before shipping.
//
// We parse Cargo.toml with a targeted regex rather than pulling in
// @iarna/toml - the workspace version line is consistently shaped
// (`version = "X.Y.Z"` at top level inside `[workspace.package]`)
// and adding a npm dep for one line of parsing is not worth the
// supply-chain surface.

function readCargoWorkspaceVersion() {
  const toml = readText(WORKSPACE_CARGO_TOML);
  // Match the `[workspace.package]` section then the first `version = "..."`
  // inside it. The `[^[]*` runs to the next section header.
  const m = toml.match(/\[workspace\.package\][^[]*?^\s*version\s*=\s*"([^"]+)"/m);
  if (!m) {
    console.error(
      '[ERROR] Could not find `version = "..."` under [workspace.package] in Cargo.toml'
    );
    process.exit(2);
  }
  return m[1];
}

const workspaceVersion = readCargoWorkspaceVersion();

for (const file of VERSION_SYNC_JSON_FILES) {
  if (!fs.existsSync(file)) continue;
  const json = readJSON(file);
  if (json.version === workspaceVersion) continue;
  note(
    `${path.relative(ROOT, file)}: version is "${json.version}", Cargo.toml workspace version is "${workspaceVersion}"`
  );
  if (!checkMode) {
    json.version = workspaceVersion;
    writeJSONPretty(file, json);
    console.log(`[OK] Synced ${path.relative(ROOT, file)} version to ${workspaceVersion}`);
  }
}

// ---- 4. VALIDATION-RULES.md footer stats ----

const VALIDATION_RULES_MD = path.join(ROOT, 'knowledge-base', 'VALIDATION-RULES.md');

function computeRuleStats(rulesJson) {
  const rules = rulesJson.rules;
  const autofixCount = rules.filter((r) => r.fix && r.fix.autofix === true).length;
  const autofixPct = Math.round((autofixCount / rules.length) * 100);
  // Category count comes from the structured `categories` map which is the
  // source of truth used by the docs generator (scripts/generate-docs-rules.py).
  // Using distinct rule.category values here would drift from the website.
  const categoriesMap = rulesJson.categories || {};
  const categoryCount = Object.keys(categoriesMap).length;
  const severityCounts = { HIGH: 0, MEDIUM: 0, LOW: 0 };
  for (const r of rules) {
    if (severityCounts[r.severity] !== undefined) {
      severityCounts[r.severity] += 1;
    }
  }
  return {
    total: rules.length,
    categoryCount,
    high: severityCounts.HIGH,
    medium: severityCounts.MEDIUM,
    low: severityCounts.LOW,
    autofixCount,
    autofixPct,
  };
}

/// Update per-category `count` fields in rulesJson.categories from the
/// live rule entries. Also notes any rules whose `category` value isn't
/// in the top-level map so the maintainer can add it.
/// Returns true if any count changed.
function syncCategoryCounts(rulesJson) {
  const rules = rulesJson.rules;
  const categoriesMap = rulesJson.categories;
  if (!categoriesMap) {
    note('knowledge-base/rules.json has no top-level `categories` map - skipping per-category count sync');
    return false;
  }
  // Tally categories from live rules.
  const observed = {};
  for (const r of rules) {
    observed[r.category] = (observed[r.category] || 0) + 1;
  }
  // Flag rules whose category isn't declared.
  for (const cat of Object.keys(observed)) {
    if (!categoriesMap[cat]) {
      note(
        `rules.json rule uses category "${cat}" that is not declared in the top-level categories map - add it with a prefix + description`
      );
    }
  }
  // Sync declared-category counts.
  let dirty = false;
  for (const [cat, meta] of Object.entries(categoriesMap)) {
    const expected = observed[cat] || 0;
    if (meta.count !== expected) {
      note(
        `rules.json categories.${cat}.count is ${meta.count}, actual is ${expected}`
      );
      if (!checkMode) {
        meta.count = expected;
        dirty = true;
      }
    }
  }
  return dirty;
}

if (fs.existsSync(VALIDATION_RULES_MD)) {
  const original = readText(VALIDATION_RULES_MD);
  const stats = computeRuleStats(rulesJson);

  // Three footer lines. Each has a fixed `**Label**:` prefix so we can
  // rewrite them in place without touching surrounding prose.
  const replacements = [
    {
      re: /(\*\*Total Coverage\*\*:\s*)(\d+)( validation rules across )(\d+)( categories)/,
      render: () => `${stats.total} validation rules across ${stats.categoryCount} categories`,
      label: 'Total Coverage',
    },
    {
      re: /(\*\*Certainty\*\*:\s*)(\d+)( HIGH,\s*)(\d+)( MEDIUM,\s*)(\d+)( LOW)/,
      render: () => `${stats.high} HIGH, ${stats.medium} MEDIUM, ${stats.low} LOW`,
      label: 'Certainty',
    },
    {
      re: /(\*\*Auto-Fixable\*\*:\s*)(\d+)( rules \()(\d+)(%\))/,
      render: () => `${stats.autofixCount} rules (${stats.autofixPct}%)`,
      label: 'Auto-Fixable',
    },
  ];

  let updated = original;
  for (const { re, render, label } of replacements) {
    const match = re.exec(updated);
    if (!match) {
      note(`VALIDATION-RULES.md: footer **${label}** line not found - regex drifted?`);
      continue;
    }
    const prefix = match[1];
    const rendered = `${prefix}${render()}`;
    const current = match[0];
    if (current !== rendered) {
      note(`VALIDATION-RULES.md: **${label}** line is stale (was "${current.trim()}")`);
      if (!checkMode) {
        updated = updated.replace(re, rendered);
      }
    }
  }

  if (updated !== original && !checkMode) {
    writeText(VALIDATION_RULES_MD, updated);
    console.log('[OK] Updated VALIDATION-RULES.md footer stats');
  }
}

// ---- 5. Regenerate website docs ----

if (!skipDocs && !checkMode) {
  // generate-docs-rules.py shebang is `#!/usr/bin/env python3`, so
  // python3 is the canonical interpreter. Fall back to `python` on
  // systems (typically Windows) where python3 isn't available but
  // `python` points at Python 3.
  const pythonCandidates = ['python3', 'python'];
  let regenerated = false;
  let lastErr = null;
  for (const bin of pythonCandidates) {
    try {
      const output = execFileSync(
        bin,
        [path.join(ROOT, 'scripts', 'generate-docs-rules.py')],
        { cwd: ROOT, encoding: 'utf8' }
      );
      console.log(`[OK] Regenerated website docs (via ${bin}):`);
      console.log(output.trim().split('\n').map((l) => `     ${l}`).join('\n'));
      regenerated = true;
      break;
    } catch (err) {
      lastErr = err;
      // Try next candidate.
    }
  }
  if (!regenerated) {
    console.error(
      '[WARN] Could not regenerate docs (python3/python unavailable?):',
      lastErr ? lastErr.message : 'unknown error'
    );
    // Not a hard failure - the docs_website_parity test catches drift.
  }
}

// ---- Report ----

if (drift.length > 0) {
  if (checkMode) {
    console.error(`\n[DRIFT] Found ${drift.length} bookkeeping drift issue(s):`);
    for (const msg of drift) {
      console.error(`  - ${msg}`);
    }
    console.error(`\nRun without --check to apply fixes.`);
    process.exit(1);
  } else {
    console.log(`\n[OK] Fixed ${drift.length} bookkeeping drift issue(s).`);
  }
} else {
  console.log('[OK] All rule bookkeeping is in sync.');
}
