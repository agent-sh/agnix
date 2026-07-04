import * as assert from 'assert';
import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import {
  parseSha256Sidecar,
  sha256File,
  verifySha256File,
} from '../../checksum';

const HELLO_SHA256 =
  '2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824';

describe('checksum verification', () => {
  it('parses sha256 sidecars with matching artifact names', () => {
    const hash = HELLO_SHA256.toUpperCase();

    assert.strictEqual(
      parseSha256Sidecar(`${hash}  agnix-lsp.tar.gz\n`, 'agnix-lsp.tar.gz'),
      HELLO_SHA256
    );
  });

  it('rejects malformed hashes and mismatched artifact names', () => {
    assert.throws(
      () => parseSha256Sidecar('not-a-hash  agnix-lsp.tar.gz', 'agnix-lsp.tar.gz'),
      /valid SHA256/
    );
    assert.throws(
      () =>
        parseSha256Sidecar(
          `${HELLO_SHA256}  agnix-lsp-linux.tar.gz`,
          'agnix-lsp-macos.tar.gz'
        ),
      /expected agnix-lsp-macos.tar.gz/
    );
  });

  it('verifies a file against a matching sidecar', () => {
    const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'agnix-checksum-'));
    const filePath = path.join(tempDir, 'agnix-lsp.tar.gz');
    const checksumPath = `${filePath}.sha256`;

    try {
      fs.writeFileSync(filePath, 'hello');
      fs.writeFileSync(checksumPath, `${HELLO_SHA256}  agnix-lsp.tar.gz\n`);

      assert.strictEqual(sha256File(filePath), HELLO_SHA256);
      assert.doesNotThrow(() =>
        verifySha256File(filePath, checksumPath, 'agnix-lsp.tar.gz')
      );
    } finally {
      fs.rmSync(tempDir, { recursive: true, force: true });
    }
  });

  it('rejects a mismatched file hash', () => {
    const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'agnix-checksum-'));
    const filePath = path.join(tempDir, 'agnix-lsp.tar.gz');
    const checksumPath = `${filePath}.sha256`;

    try {
      fs.writeFileSync(filePath, 'tampered');
      fs.writeFileSync(checksumPath, `${HELLO_SHA256}  agnix-lsp.tar.gz\n`);

      assert.throws(
        () => verifySha256File(filePath, checksumPath, 'agnix-lsp.tar.gz'),
        /Checksum mismatch/
      );
    } finally {
      fs.rmSync(tempDir, { recursive: true, force: true });
    }
  });
});
