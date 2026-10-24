#!/usr/bin/env node
'use strict';

const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '..');
const workflows = path.join(root, '.github', 'workflows');
const pins = [];
const actionReferences = new Set();
for (const name of fs.readdirSync(workflows)) {
  if (!/\.ya?ml$/.test(name)) continue;
  const content = fs.readFileSync(path.join(workflows, name), 'utf8');
  for (const match of content.matchAll(/^\s*(?:-\s*)?uses:\s*["']?([\w.-]+\/[\w./-]+)@([a-f0-9]{40})(?=[\s"']|$)/gm)) {
    actionReferences.add(`${match[1]}@${match[2]}`);
  }
  for (const match of content.matchAll(/github\/codeql-action\/([\w-]+)@([^\s"']+)/g)) {
    pins.push({ file: name, action: match[1], sha: match[2] });
  }
}

const errors = [];
if (!pins.length) errors.push('No CodeQL action pins found');
for (const pin of pins) {
  if (!/^[a-f0-9]{40}$/.test(pin.sha)) {
    errors.push(`${pin.file}: CodeQL ${pin.action} must use a full commit SHA`);
  }
}
if (new Set(pins.map(pin => pin.sha)).size > 1) {
  errors.push('CodeQL actions must use the same commit SHA');
}

const table = fs.readFileSync(path.join(workflows, 'README.md'), 'utf8');
const documented = new Set(
  [...table.matchAll(/^([\w.-]+\/[\w./-]+)@[^\s:]+: ([a-f0-9]{40})\r?$/gm)]
    .map(match => `${match[1]}@${match[2]}`),
);
for (const reference of actionReferences) {
  if (!reference.startsWith('github/codeql-action/') && !documented.has(reference)) {
    errors.push(`.github/workflows/README.md: missing current action pin ${reference}`);
  }
}
for (const pin of pins) {
  const row = new RegExp(`github/codeql-action/${pin.action}@v[^\\s:]+: ${pin.sha}(?:\\r?\\n|$)`);
  if (!row.test(table)) errors.push(`.github/workflows/README.md: missing current CodeQL ${pin.action} pin`);
}

if (errors.length) {
  for (const error of errors) console.error(error);
  process.exitCode = 1;
} else {
  console.log(`Action reference agrees with ${actionReferences.size} pinned actions; CodeQL pins agree across ${pins.length} steps`);
}
