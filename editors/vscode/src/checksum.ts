import * as crypto from 'crypto';
import * as fs from 'fs';
import * as path from 'path';

const SHA256_HEX_RE = /^[0-9a-f]{64}$/i;

export function parseSha256Sidecar(
  contents: string,
  expectedFileName: string
): string {
  const line = contents
    .split(/\r?\n/)
    .map((candidate) => candidate.trim())
    .find((candidate) => candidate.length > 0);

  if (!line) {
    throw new Error('Checksum file is empty');
  }

  const parts = line.split(/\s+/);
  const hash = parts[0];
  if (!SHA256_HEX_RE.test(hash)) {
    throw new Error('Checksum file does not contain a valid SHA256 hash');
  }

  const sidecarName = parts[1]?.replace(/^\*/, '');
  if (sidecarName && path.basename(sidecarName) !== expectedFileName) {
    throw new Error(
      `Checksum file is for ${sidecarName}, expected ${expectedFileName}`
    );
  }

  return hash.toLowerCase();
}

export function sha256File(filePath: string): string {
  return crypto.createHash('sha256').update(fs.readFileSync(filePath)).digest('hex');
}

export function verifySha256File(
  filePath: string,
  checksumPath: string,
  expectedFileName = path.basename(filePath)
): void {
  const expected = parseSha256Sidecar(
    fs.readFileSync(checksumPath, 'utf8'),
    expectedFileName
  );
  const actual = sha256File(filePath);

  if (actual !== expected) {
    throw new Error(
      `Checksum mismatch for ${expectedFileName}: expected ${expected}, got ${actual}`
    );
  }
}
