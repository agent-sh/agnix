#!/usr/bin/env node

const https = require('https');
const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');
const os = require('os');
const crypto = require('crypto');

const GITHUB_REPO = 'agent-sh/agnix';
const VERSION = require('./package.json').version;

function removeIfExists(filePath) {
  try {
    if (fs.existsSync(filePath)) {
      fs.unlinkSync(filePath);
    }
  } catch (_) {
    // best-effort cleanup
  }
}

/**
 * Get platform-specific asset name and binary name.
 */
function getPlatformInfo() {
  const platform = os.platform();
  const arch = os.arch();

  const mapping = {
    'darwin-arm64': {
      asset: 'agnix-aarch64-apple-darwin.tar.gz',
      extractedName: 'agnix',
      binary: 'agnix-binary',
    },
    'darwin-x64': {
      // x64 Mac uses ARM binary via Rosetta 2
      asset: 'agnix-aarch64-apple-darwin.tar.gz',
      extractedName: 'agnix',
      binary: 'agnix-binary',
    },
    'linux-x64': {
      asset: 'agnix-x86_64-unknown-linux-gnu.tar.gz',
      extractedName: 'agnix',
      binary: 'agnix-binary',
    },
    'linux-arm64': {
      asset: 'agnix-aarch64-unknown-linux-gnu.tar.gz',
      extractedName: 'agnix',
      binary: 'agnix-binary',
    },
    'win32-x64': {
      asset: 'agnix-x86_64-pc-windows-msvc.zip',
      extractedName: 'agnix.exe',
      binary: 'agnix-binary.exe',
    },
  };

  const key = `${platform}-${arch}`;
  const info = mapping[key];

  if (!info) {
    console.error(`Unsupported platform: ${platform}-${arch}`);
    console.error('Supported platforms: darwin-arm64, darwin-x64, linux-x64, win32-x64');
    process.exit(1);
  }

  return info;
}

/**
 * Download a file from URL, following redirects.
 */
function downloadFile(url, destPath) {
  return new Promise((resolve, reject) => {
    let file = null;
    let settled = false;

    const fail = (err) => {
      if (settled) {
        return;
      }
      settled = true;
      if (file) {
        file.destroy();
      }
      removeIfExists(destPath);
      reject(err);
    };

    const request = https.get(url, (response) => {
      if (response.statusCode === 302 || response.statusCode === 301) {
        const redirectUrl = response.headers.location;
        if (redirectUrl) {
          response.resume();
          const nextUrl = new URL(redirectUrl, url).href;
          downloadFile(nextUrl, destPath).then(resolve).catch(reject);
          return;
        }
      }

      if (response.statusCode !== 200) {
        response.resume();
        fail(new Error(`Download failed with status ${response.statusCode}`));
        return;
      }

      file = fs.createWriteStream(destPath);
      response.pipe(file);
      response.on('error', fail);

      file.on('finish', () => {
        if (settled) {
          return;
        }
        settled = true;
        file.close((err) => {
          if (err) {
            removeIfExists(destPath);
            reject(err);
          } else {
            resolve();
          }
        });
      });
      file.on('error', fail);
    });

    request.on('error', fail);
  });
}

/**
 * Extract archive based on platform.
 */
function extractArchive(archivePath, destDir) {
  const platform = os.platform();

  if (platform === 'win32') {
    // Use PowerShell to extract zip
    execSync(
      `powershell -Command "Expand-Archive -Path '${archivePath}' -DestinationPath '${destDir}' -Force"`,
      { stdio: 'inherit' }
    );
  } else {
    // Use tar for .tar.gz
    execSync(`tar -xzf "${archivePath}" -C "${destDir}"`, { stdio: 'inherit' });
  }
}

/**
 * Compute the SHA-256 of a file as a lowercase hex string, streaming the file
 * so large archives are not read fully into memory.
 */
function sha256File(filePath) {
  return new Promise((resolve, reject) => {
    const hash = crypto.createHash('sha256');
    const stream = fs.createReadStream(filePath);
    stream.on('error', reject);
    stream.on('data', (chunk) => hash.update(chunk));
    stream.on('end', () => resolve(hash.digest('hex')));
  });
}

function parseSha256Sidecar(contents, expectedFileName) {
  const expectedBase = path.basename(expectedFileName);
  for (const line of contents.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed) {
      continue;
    }

    const [rawHash, rawFileName] = trimmed.split(/\s+/, 2);
    if (!rawFileName) {
      continue;
    }

    const normalizedFileName = rawFileName.replace(/^\*/, '').replace(/\\/g, '/');
    const baseName = path.posix.basename(normalizedFileName);
    if (baseName !== expectedBase) {
      continue;
    }

    const expected = rawHash.toLowerCase();
    if (!/^[0-9a-f]{64}$/.test(expected)) {
      throw new Error(`Invalid checksum file for ${expectedBase}`);
    }

    return expected;
  }

  throw new Error(`Checksum file does not contain an entry for ${expectedBase}`);
}

/**
 * Download the `.sha256` sidecar for an archive and verify the archive against
 * it before extraction, so a tampered download is rejected (mirrors
 * scripts/download.sh). Throws on a missing/malformed sidecar or hash mismatch.
 */
async function verifyChecksum(archivePath, checksumUrl, deps = { downloadFile }) {
  const checksumPath = `${archivePath}.sha256`;
  try {
    await deps.downloadFile(checksumUrl, checksumPath);
    const raw = fs.readFileSync(checksumPath, 'utf8').trim();
    const expected = parseSha256Sidecar(raw, path.basename(archivePath));
    const actual = (await sha256File(archivePath)).toLowerCase();
    if (actual !== expected) {
      throw new Error(
        `Checksum mismatch for ${path.basename(archivePath)}: expected ${expected}, got ${actual}`
      );
    }
    console.log('Checksum verified.');
  } finally {
    removeIfExists(checksumPath);
  }
}

async function main() {
  const platformInfo = getPlatformInfo();
  const binDir = path.join(__dirname, 'bin');
  const binaryPath = path.join(binDir, platformInfo.binary);
  const extractedPath = path.join(binDir, platformInfo.extractedName);
  const wrapperPath = path.join(binDir, 'agnix');
  const wrapperBackup = path.join(binDir, 'agnix.backup');

  // Skip if binary already exists
  if (fs.existsSync(binaryPath)) {
    console.log('agnix binary already installed');
    return;
  }

  // Ensure bin directory exists
  if (!fs.existsSync(binDir)) {
    fs.mkdirSync(binDir, { recursive: true });
  }

  const downloadUrl = `https://github.com/${GITHUB_REPO}/releases/download/v${VERSION}/${platformInfo.asset}`;
  const archivePath = path.join(binDir, platformInfo.asset);

  console.log(`Downloading agnix v${VERSION} for ${os.platform()}-${os.arch()}...`);

  try {
    // Save wrapper script if it exists (npm places it during install)
    if (fs.existsSync(wrapperPath) && wrapperPath !== binaryPath) {
      fs.copyFileSync(wrapperPath, wrapperBackup);
    }

    await downloadFile(downloadUrl, archivePath);
    await verifyChecksum(archivePath, `${downloadUrl}.sha256`);
    console.log('Extracting...');
    extractArchive(archivePath, binDir);

    // Rename extracted binary to avoid conflict with wrapper script
    if (fs.existsSync(extractedPath) && extractedPath !== binaryPath) {
      fs.renameSync(extractedPath, binaryPath);
    }

    // Restore wrapper script if it was backed up
    if (fs.existsSync(wrapperBackup)) {
      fs.renameSync(wrapperBackup, wrapperPath);
    }

    // Make binary executable on Unix
    if (os.platform() !== 'win32') {
      fs.chmodSync(binaryPath, 0o755);
    }

    // Verify binary exists
    if (!fs.existsSync(binaryPath)) {
      throw new Error('Binary not found after extraction');
    }

    console.log('agnix installed successfully');
  } catch (error) {
    console.error(`Failed to install agnix: ${error.message}`);
    console.error('');
    console.error('You can install manually:');
    console.error('  cargo install agnix-cli');
    process.exitCode = 1;
  } finally {
    // Best-effort cleanup so a failed install leaves no stale archive or backup.
    removeIfExists(archivePath);
    try {
      if (fs.existsSync(wrapperBackup)) {
        if (!fs.existsSync(wrapperPath)) {
          fs.renameSync(wrapperBackup, wrapperPath);
        } else {
          removeIfExists(wrapperBackup);
        }
      }
    } catch (_) {
      // ignore
    }
  }
}

if (require.main === module) {
  main();
}

module.exports = {
  downloadFile,
  extractArchive,
  getPlatformInfo,
  main,
  parseSha256Sidecar,
  removeIfExists,
  sha256File,
  verifyChecksum,
};
