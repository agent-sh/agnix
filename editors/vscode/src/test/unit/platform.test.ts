import * as assert from 'assert';
import { getPlatformInfo } from '../../platform';

describe('macOS LSP download', () => {
  it('matches the architecture of the extension host', () => {
    assert.strictEqual(getPlatformInfo('darwin', 'x64')?.asset, 'agnix-lsp-x86_64-apple-darwin.tar.gz');
    assert.strictEqual(getPlatformInfo('darwin', 'arm64')?.asset, 'agnix-lsp-aarch64-apple-darwin.tar.gz');
    assert.strictEqual(getPlatformInfo('darwin', 'ia32'), null);
  });
});
