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
  if (sidecarName) {
    const baseName = sidecarName.replace(/\\/g, '/').split('/').pop();
    if (baseName !== expectedFileName) {
      throw new Error(
        `Checksum file is for ${sidecarName}, expected ${expectedFileName}`
      );
    }
  }

  return hash.toLowerCase();
}

export function sha256File(filePath: string): Promise<string> {
  return new Promise((resolve, reject) => {
    const hasher = crypto.createHash('sha256');
    const stream = fs.createReadStream(filePath);

    stream.on('data', (chunk) => hasher.update(chunk));
    stream.on('error', reject);
    stream.on('end', () => resolve(hasher.digest('hex')));
  });
}

export async function verifySha256File(
  filePath: string,
  checksumPath: string,
  expectedFileName = path.basename(filePath)
): Promise<void> {
  const expected = parseSha256Sidecar(
    fs.readFileSync(checksumPath, 'utf8'),
    expectedFileName
  );
  const actual = await sha256File(filePath);

  if (actual !== expected) {
    throw new Error(
      `Checksum mismatch for ${expectedFileName}: expected ${expected}, got ${actual}`
    );
  }
}
