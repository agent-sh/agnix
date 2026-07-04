#!/usr/bin/env node

/**
 * Tests for npm installer helper behavior.
 */

const assert = require('assert');
const crypto = require('crypto');
const fs = require('fs');
const os = require('os');
const path = require('path');

const {
  parseSha256Sidecar,
  sha256File,
  verifyChecksum,
} = require('../install');

let passed = 0;
let failed = 0;

async function test(name, fn) {
  try {
    await fn();
    console.log(`  PASS: ${name}`);
    passed++;
  } catch (err) {
    console.log(`  FAIL: ${name}`);
    console.log(`        ${err.message}`);
    failed++;
  }
}

function sha256(contents) {
  return crypto.createHash('sha256').update(contents).digest('hex');
}

async function withTempDir(fn) {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'agnix-install-test-'));
  try {
    await fn(tempDir);
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
}

(async () => {
  console.log('agnix npm installer helper tests\n');

  await test('parseSha256Sidecar selects the matching archive basename', () => {
    const expected = 'a'.repeat(64);
    const ignored = 'b'.repeat(64);
    const sidecar = [
      `${ignored}  other-archive.tar.gz`,
      `${expected}  nested/agnix-x86_64-unknown-linux-gnu.tar.gz`,
    ].join('\n');

    assert.strictEqual(
      parseSha256Sidecar(sidecar, 'agnix-x86_64-unknown-linux-gnu.tar.gz'),
      expected
    );
  });

  await test('parseSha256Sidecar accepts Windows-style paths and binary markers', () => {
    const expected = 'c'.repeat(64);
    const sidecar = `${expected}  *nested\\agnix-x86_64-pc-windows-msvc.zip\n`;

    assert.strictEqual(
      parseSha256Sidecar(sidecar, 'agnix-x86_64-pc-windows-msvc.zip'),
      expected
    );
  });

  await test('parseSha256Sidecar rejects a malformed matching hash', () => {
    assert.throws(
      () => parseSha256Sidecar('not-a-hash  agnix.tar.gz\n', 'agnix.tar.gz'),
      /Invalid checksum file/
    );
  });

  await test('parseSha256Sidecar rejects sidecars without the archive entry', () => {
    assert.throws(
      () => parseSha256Sidecar(`${'d'.repeat(64)}  other.tar.gz\n`, 'agnix.tar.gz'),
      /does not contain an entry/
    );
  });

  await test('sha256File hashes the archive asynchronously', async () => {
    await withTempDir(async (tempDir) => {
      const archivePath = path.join(tempDir, 'archive.tar.gz');
      fs.writeFileSync(archivePath, 'hello agnix');

      assert.strictEqual(await sha256File(archivePath), sha256('hello agnix'));
    });
  });

  await test('verifyChecksum awaits hashing and removes the sidecar', async () => {
    await withTempDir(async (tempDir) => {
      const archivePath = path.join(tempDir, 'agnix.tar.gz');
      const contents = 'verified archive';
      fs.writeFileSync(archivePath, contents);

      await verifyChecksum(archivePath, 'https://example.invalid/agnix.tar.gz.sha256', {
        downloadFile: async (_url, checksumPath) => {
          fs.writeFileSync(checksumPath, `${sha256(contents)}  agnix.tar.gz\n`);
        },
      });

      assert.ok(!fs.existsSync(`${archivePath}.sha256`), 'sidecar should be removed');
    });
  });

  await test('verifyChecksum removes the sidecar after mismatch failures', async () => {
    await withTempDir(async (tempDir) => {
      const archivePath = path.join(tempDir, 'agnix.tar.gz');
      fs.writeFileSync(archivePath, 'actual archive');

      await assert.rejects(
        verifyChecksum(archivePath, 'https://example.invalid/agnix.tar.gz.sha256', {
          downloadFile: async (_url, checksumPath) => {
            fs.writeFileSync(checksumPath, `${'e'.repeat(64)}  agnix.tar.gz\n`);
          },
        }),
        /Checksum mismatch/
      );

      assert.ok(!fs.existsSync(`${archivePath}.sha256`), 'sidecar should be removed');
    });
  });

  console.log(`\nResults: ${passed} passed, ${failed} failed`);
  process.exitCode = failed > 0 ? 1 : 0;
})();
