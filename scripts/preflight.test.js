// Exercise the entrypoint with fake tools: no Rust compilation or network.
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

function fixture(t) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'agnix preflight '));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  for (const dir of ['bin', 'scripts', 'nested', 'schemas', 'editors/vscode/schemas', 'tmp']) {
    fs.mkdirSync(path.join(root, dir), { recursive: true });
  }
  fs.copyFileSync(path.join(__dirname, 'preflight.sh'), path.join(root, 'scripts/preflight.sh'));
  for (const file of ['CLAUDE.md', 'AGENTS.md', 'schemas/agnix.json', 'editors/vscode/schemas/agnix.json']) {
    fs.writeFileSync(path.join(root, file), 'same\n');
  }
  // Preserve only the shell utilities the real entrypoint needs. In particular,
  // missing node cannot accidentally resolve to the developer's global node.
  for (const tool of ['bash', 'dirname', 'cmp', 'mktemp', 'rm']) {
    const location = spawnSync('sh', ['-c', `command -v ${tool}`], { encoding: 'utf8' });
    assert.equal(location.status, 0);
    fs.symlinkSync(location.stdout.trim(), path.join(root, 'bin', tool));
  }
  const stub = `#!/bin/sh
tool=\${0##*/}
printf '%s|%s|%s|%s|%s\\n' "$tool" "$*" "$CARGO_BUILD_JOBS" "$RUST_TEST_THREADS" "$RAYON_NUM_THREADS" >> "$PREFLIGHT_LOG"
if [ "$tool $*" = "\${FAIL_COMMAND:-}" ]; then exit 17; fi
if [ "$tool" = cargo ]; then
  previous=''
  for arg in "$@"; do
    if [ "$previous" = --output ]; then printf '%s\\n' "\${SCHEMA_CONTENT:-same}" > "$arg"; fi
    previous=$arg
  done
fi
`;
  for (const tool of ['cargo', 'node', 'python3']) {
    fs.writeFileSync(path.join(root, 'bin', tool), stub, { mode: 0o755 });
  }
  // The real locale and glibc scripts are tested elsewhere. Here verify dispatch.
  for (const script of ['check-locale-sync.sh', 'check-glibc-floor.test.sh']) {
    fs.writeFileSync(path.join(root, 'scripts', script), 'exit 0\n');
  }
  const log = path.join(root, 'commands.log');
  return {
    root,
    run(args = [], overrides = {}) {
      return spawnSync('bash', [path.join(root, 'scripts/preflight.sh'), ...args], {
        cwd: path.join(root, 'nested'), encoding: 'utf8',
        env: { ...process.env, PATH: path.join(root, 'bin'), TMPDIR: path.join(root, 'tmp'),
          CARGO_BUILD_JOBS: '', RUST_TEST_THREADS: '', RAYON_NUM_THREADS: '',
          PREFLIGHT_LOG: log, ...overrides },
      });
    },
    commands() { return fs.existsSync(log) ? fs.readFileSync(log, 'utf8') : ''; },
  };
}

test('default quick mode works from a nested directory with spaces and never compiles', (t) => {
  const f = fixture(t);
  const result = f.run();
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /Quick preflight/);
  assert.match(f.commands(), /cargo\|fmt --check --all\|2\|2\|2/);
  assert.match(f.commands(), /node\|scripts\/sync-rule-bookkeeping.js --check --skip-docs/);
  assert.match(f.commands(), /python3\|scripts\/check-rule-counts.py/);
  assert.doesNotMatch(f.commands(), /clippy|cargo\|test|cargo\|run/);
});

test('full mode retains validation gates and honors explicit resource limits', (t) => {
  const f = fixture(t);
  const result = f.run(['--full'], { CARGO_BUILD_JOBS: '3', RUST_TEST_THREADS: '1', RAYON_NUM_THREADS: '4' });
  assert.equal(result.status, 0, result.stderr);
  const commands = f.commands();
  for (const command of [
    'clippy --locked --workspace --all-targets --all-features -- -D warnings',
    'test --locked --workspace', 'eval tests/eval.yaml',
    'kiro_ci_gate -- --include-ignored', 'npm/test/install-helpers.test.js',
    '-m unittest discover -s pypi/test', '--test scripts/glm-extract.test.js',
    '--test scripts/preflight.test.js', 'schema --output', '. --config .agnix.toml',
  ]) assert.ok(commands.includes(command), command);
  assert.match(commands, /\|3\|1\|4/);
  assert.deepEqual(fs.readdirSync(path.join(f.root, 'tmp')), []);
  assert.match(result.stdout, /Full local preflight/);
});

test('cheap check failure stops before compilation and propagates its status', (t) => {
  const f = fixture(t);
  const result = f.run(['--full'], { FAIL_COMMAND: 'node scripts/sync-rule-bookkeeping.js --check --skip-docs' });
  assert.equal(result.status, 17);
  assert.match(result.stderr, /FAIL.*Rule bookkeeping/);
  assert.doesNotMatch(f.commands(), /clippy|cargo\|test/);
});

test('a missing prerequisite fails instead of skipping a gate', (t) => {
  const f = fixture(t);
  fs.unlinkSync(path.join(f.root, 'bin/node'));
  const result = f.run();
  assert.equal(result.status, 1);
  assert.match(result.stderr, /Missing prerequisite: node/);
  assert.equal(f.commands(), '');
});

test('schema drift fails without overwriting files and removes its temporary output', (t) => {
  const f = fixture(t);
  const result = f.run(['--full'], { SCHEMA_CONTENT: 'changed' });
  assert.equal(result.status, 1);
  assert.match(result.stderr, /FAIL.*Generated schema matches/);
  assert.equal(fs.readFileSync(path.join(f.root, 'schemas/agnix.json'), 'utf8'), 'same\n');
  assert.equal(fs.readFileSync(path.join(f.root, 'editors/vscode/schemas/agnix.json'), 'utf8'), 'same\n');
  assert.deepEqual(fs.readdirSync(path.join(f.root, 'tmp')), []);
  assert.doesNotMatch(f.commands(), /--config .agnix.toml/);
});

test('unknown or extra options fail before running checks', (t) => {
  const f = fixture(t);
  for (const args of [['--typo'], ['--quick', '--full']]) {
    assert.equal(f.run(args).status, 2);
  }
  assert.equal(f.commands(), '');
});
