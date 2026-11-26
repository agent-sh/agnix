interface PlatformInfo {
  asset: string;
  binary: string;
}

/**
 * Get platform-specific download info for agnix-lsp.
 */
export function getPlatformInfo(platform = process.platform, arch = process.arch): PlatformInfo | null {
  if (platform === 'darwin') {
    if (arch === 'arm64') {
      return {
        asset: 'agnix-lsp-aarch64-apple-darwin.tar.gz',
        binary: 'agnix-lsp',
      };
    }
    if (arch === 'x64') {
      return { asset: 'agnix-lsp-x86_64-apple-darwin.tar.gz', binary: 'agnix-lsp' };
    }
    return null;
  } else if (platform === 'linux') {
    if (arch === 'x64') {
      return {
        asset: 'agnix-lsp-x86_64-unknown-linux-gnu.tar.gz',
        binary: 'agnix-lsp',
      };
    }
    if (arch === 'arm64') {
      return {
        asset: 'agnix-lsp-aarch64-unknown-linux-gnu.tar.gz',
        binary: 'agnix-lsp',
      };
    }
    return null;
  } else if (platform === 'win32') {
    if (arch === 'x64') {
      return {
        asset: 'agnix-lsp-x86_64-pc-windows-msvc.zip',
        binary: 'agnix-lsp.exe',
      };
    }
    return null;
  }

  return null;
}
