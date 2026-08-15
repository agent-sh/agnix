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
  binaryRuns,
  getPlatformInfo,
  parseSha256Sidecar,
  restoreWrapperBackup,
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

  await test('restoreWrapperBackup restores over partial extraction on failure', async () => {
    await withTempDir(async (tempDir) => {
      const wrapperPath = path.join(tempDir, 'agnix');
      const wrapperBackup = path.join(tempDir, 'agnix.backup');
      fs.writeFileSync(wrapperPath, 'partial extracted binary');
      fs.writeFileSync(wrapperBackup, 'npm wrapper');

      restoreWrapperBackup(wrapperPath, wrapperBackup, false);

      assert.strictEqual(fs.readFileSync(wrapperPath, 'utf8'), 'npm wrapper');
      assert.ok(!fs.existsSync(wrapperBackup), 'backup should be moved back into place');
    });
  });

  await test('restoreWrapperBackup removes stale backup after successful install', async () => {
    await withTempDir(async (tempDir) => {
      const wrapperPath = path.join(tempDir, 'agnix');
      const wrapperBackup = path.join(tempDir, 'agnix.backup');
      fs.writeFileSync(wrapperPath, 'installed wrapper');
      fs.writeFileSync(wrapperBackup, 'old wrapper backup');

      restoreWrapperBackup(wrapperPath, wrapperBackup, true);

      assert.strictEqual(fs.readFileSync(wrapperPath, 'utf8'), 'installed wrapper');
      assert.ok(!fs.existsSync(wrapperBackup), 'stale backup should be removed');
    });
  });

  await test('binaryRuns reports success only on a clean exit', () => {
    assert.strictEqual(
      binaryRuns('/does/not/matter', { spawnSync: () => ({ status: 0 }) }),
      true
    );
    assert.strictEqual(
      binaryRuns('/does/not/matter', { spawnSync: () => ({ status: 127 }) }),
      false
    );
  });

  await test('binaryRuns treats a loader failure as not runnable', () => {
    // What a glibc-too-old host produces: the process never starts.
    assert.strictEqual(
      binaryRuns('/does/not/matter', {
        spawnSync: () => ({ error: new Error('ENOEXEC'), status: null }),
      }),
      false
    );
    assert.strictEqual(
      binaryRuns('/does/not/matter', {
        spawnSync: () => {
          throw new Error('spawn blocked');
        },
      }),
      false
    );
  });

  await test('binaryRuns detects a real executable', () => {
    assert.strictEqual(binaryRuns(process.execPath), true);
  });

  await test('linux-x64 carries the static musl fallback asset', () => {
    // Guard for #1371: the gnu asset cannot load on hosts whose glibc is older
    // than the build's floor, so the installer needs a static alternative.
    const info = getPlatformInfo('linux', 'x64');

    assert.strictEqual(info.asset, 'agnix-x86_64-unknown-linux-gnu.tar.gz');
    assert.strictEqual(info.fallbackAsset, 'agnix-x86_64-unknown-linux-musl.tar.gz');
  });

  await test('platforms without a static build declare no fallback', () => {
    for (const [platform, arch] of [
      ['darwin', 'arm64'],
      ['linux', 'arm64'],
      ['win32', 'x64'],
    ]) {
      assert.strictEqual(
        getPlatformInfo(platform, arch).fallbackAsset,
        undefined,
        `${platform}-${arch} must not claim a fallback asset that is not published`
      );
    }
  });

  console.log(`\nResults: ${passed} passed, ${failed} failed`);
  process.exitCode = failed > 0 ? 1 : 0;
})();
