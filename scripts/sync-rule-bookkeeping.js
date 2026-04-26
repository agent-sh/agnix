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
 * 3. `CLAUDE.md`, `AGENTS.md`, `README.md`
 *    - "N rules" and "N validation rules" phrases are updated to match
 *      `rules.length`. Singular forms ("N rule sourced/across") are not
 *      currently used anywhere in these files; if they appear in future
 *      prose, add the pattern to `countPatterns` below.
 *    - Validator count phrase "N validators" is updated when --validators N
 *      is passed (the count isn't automatically derivable without parsing
 *      Rust source).
 *
 * 4. Regenerates website docs via `scripts/generate-docs-rules.py`
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

const COUNT_FILES = [
  path.join(ROOT, 'CLAUDE.md'),
  path.join(ROOT, 'AGENTS.md'),
  path.join(ROOT, 'README.md'),
];

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

if (rulesJsonDirty) {
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

// ---- 3. Count phrases in CLAUDE.md / AGENTS.md / README.md ----

// Patterns to update. The pattern must be specific enough that we don't
// match unrelated numbers. We look for " <N> rules" and
// " <N> validation rules" with word-boundary anchoring.
const countPatterns = [
  { re: /(\b)(\d+)( rules\b)/g },
  { re: /(\b)(\d+)( validation rules\b)/g },
];

for (const file of COUNT_FILES) {
  if (!fs.existsSync(file)) continue;
  const original = readText(file);
  let updated = original;
  for (const { re } of countPatterns) {
    updated = updated.replace(re, (match, pre, num, post) => {
      const current = parseInt(num, 10);
      if (current === actualRuleCount) return match;
      note(
        `${path.relative(ROOT, file)}: "${current}${post}" should be "${actualRuleCount}${post}"`
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

// ---- 4. Regenerate website docs ----

if (!skipDocs && !checkMode) {
  try {
    const output = execFileSync('python', [path.join(ROOT, 'scripts', 'generate-docs-rules.py')], {
      cwd: ROOT,
      encoding: 'utf8',
    });
    console.log('[OK] Regenerated website docs:');
    console.log(output.trim().split('\n').map((l) => `     ${l}`).join('\n'));
  } catch (err) {
    console.error('[WARN] Could not regenerate docs (python unavailable?):', err.message);
    // Not a hard failure - the docs test will catch drift separately.
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
