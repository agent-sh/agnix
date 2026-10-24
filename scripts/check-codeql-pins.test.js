'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const test = require('node:test');

const codeql = 'a'.repeat(40);
const artifact = 'b'.repeat(40);

function check(t, mutate = () => {}) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'agnix-action-pins-'));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  fs.mkdirSync(path.join(root, 'scripts'));
  fs.mkdirSync(path.join(root, '.github', 'workflows'), { recursive: true });
  fs.copyFileSync(path.join(__dirname, 'check-codeql-pins.js'), path.join(root, 'scripts', 'check-codeql-pins.js'));
  const files = {
    workflow: `jobs:\n  check:\n    steps:\n      - uses: github/codeql-action/init@${codeql}\n      - uses: github/codeql-action/analyze@${codeql}\n      - uses: actions/upload-artifact@${artifact}\n`,
    readme: `github/codeql-action/init@v4: ${codeql}\ngithub/codeql-action/analyze@v4: ${codeql}\nactions/upload-artifact@v7: ${artifact}\n`,
  };
  mutate(files);
  fs.writeFileSync(path.join(root, '.github', 'workflows', 'ci.yml'), files.workflow);
  fs.writeFileSync(path.join(root, '.github', 'workflows', 'README.md'), files.readme);
  return spawnSync(process.execPath, ['scripts/check-codeql-pins.js'], {
    cwd: root, encoding: 'utf8', timeout: 30_000,
  });
}

test('accepts matching CodeQL and other action references', t => {
  const result = check(t);
  assert.equal(result.status, 0, result.stderr);
});

test('rejects a missing non-CodeQL action reference', t => {
  const result = check(t, files => {
    files.readme = files.readme.replace(`actions/upload-artifact@v7: ${artifact}\n`, '');
  });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /actions\/upload-artifact/);
});

test('rejects a stale non-CodeQL action reference', t => {
  const result = check(t, files => {
    files.workflow = files.workflow.replace(artifact, 'c'.repeat(40));
  });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /actions\/upload-artifact/);
});

test('still requires CodeQL actions to share one SHA', t => {
  const result = check(t, files => {
    files.workflow = files.workflow.replace(`analyze@${codeql}`, `analyze@${artifact}`);
    files.readme = files.readme.replace(`analyze@v4: ${codeql}`, `analyze@v4: ${artifact}`);
  });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /same commit SHA/);
});

test('still rejects abbreviated CodeQL pins', t => {
  const result = check(t, files => {
    files.workflow = files.workflow.replace(`init@${codeql}`, 'init@abcdef');
  });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /full commit SHA/);
});
