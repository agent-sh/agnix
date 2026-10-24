import * as assert from 'assert';
import { createRequire } from 'module';
import { runInNewContext } from 'vm';

// Resolve the same copies used by the test runner and publishing tool, including
// nested dependencies selected by the scoped Azure Identity override.
const fromMocha = createRequire(require.resolve('mocha'));
const fromVsce = createRequire(require.resolve('@vscode/vsce'));
const fromIdentity = createRequire(fromVsce.resolve('@azure/identity'));
const serialize = fromMocha('serialize-javascript') as (
  value: unknown, options?: { space: number }
) => string;

type Fields = Record<string, unknown>;
type CodeFlow = (request: Fields, payload?: Fields) => Promise<unknown>;
interface Application {
  acquireTokenByCode: CodeFlow;
  getAuthCodeUrl(request: Fields): Promise<string>;
}
interface Configuration {
  auth: Fields;
  system?: Fields;
}
interface ApplicationConstructor {
  new(configuration: Configuration): Application;
  prototype: Application;
}
const msal = fromIdentity('@azure/msal-node') as {
  ConfidentialClientApplication: ApplicationConstructor;
  PublicClientApplication: new(configuration: Configuration) => unknown;
};
const confidential = msal.ConfidentialClientApplication;

function configuration(): Configuration {
  const noNetwork = async () => { throw new Error('Unexpected authentication network request'); };
  return {
    auth: {
      clientId: '11111111-1111-1111-1111-111111111111',
      authority: 'https://login.microsoftonline.com/common',
      clientSecret: 'offline-test-fixture',
    },
    system: { networkClient: { sendGetRequestAsync: noNetwork, sendPostRequestAsync: noNetwork } },
  };
}

describe('Resolved dependency security boundaries', () => {
  it('escapes a closing script tag split across serialized function bodies', () => {
    for (const ending of ['</script', '</ScRiPt']) {
      const first = () => undefined;
      first.toString = () => `function first(){}//${ending}`;
      const next = () => undefined;
      next.toString = () => 'function next(){} > <b data-probe>outside</b>';
      const output = serialize({ nested: { first }, next }, { space: 2 });
      assert.doesNotMatch(output, /<\/script(?=[\t\n\f\r />]|$)/i);
    }
  });

  it('preserves ordinary function and string serialization', () => {
    const output = serialize({ value: '</script>', fn: function ordinary() { return 42; } });
    const restored = runInNewContext(`(${output})`, {}, { timeout: 1000 }) as {
      value: string; fn(): number;
    };
    assert.strictEqual(restored.value, '</script>');
    assert.strictEqual(restored.fn(), 42);
    assert.doesNotMatch(output, /<\/script(?=[\t\n\f\r />]|$)/i);
  });

  it('removes untrusted auth-code response routing before delegation', async () => {
    const base = Object.getPrototypeOf(confidential.prototype) as Application;
    const original = base.acquireTokenByCode;
    let delegated: { request: Fields; payload?: Fields } | undefined;
    base.acquireTokenByCode = async (request, payload) => {
      delegated = { request, payload };
      return {};
    };
    try {
      const request = {
        scopes: ['https://graph.microsoft.com/.default'], redirectUri: 'http://localhost/callback',
        code: 'request-code', nonce: 'request-nonce',
      };
      const payload = {
        code: 'response-code', nonce: 'response-nonce', state: 'returned-state',
        cloud_instance_host_name: 'attacker.invalid', cloud_instance_name: 'untrusted-cloud',
        cloud_graph_host_name: 'graph.attacker.invalid', msgraph_host: 'other.attacker.invalid',
        client_info: 'untrusted-client-info', cloudInstanceHostName: 'alias.attacker.invalid',
      };
      await new confidential(configuration()).acquireTokenByCode(request, payload);
      assert.ok(delegated);
      assert.strictEqual(delegated.request, request);
      assert.deepStrictEqual(delegated.payload, {
        code: request.code, state: payload.state, nonce: request.nonce,
      });
    } finally {
      base.acquireTokenByCode = original;
    }
  });

  it('preserves an ordinary auth-code request without a response payload', async () => {
    const base = Object.getPrototypeOf(confidential.prototype) as Application;
    const original = base.acquireTokenByCode;
    const request = { scopes: ['scope'], redirectUri: 'http://localhost/callback', code: 'request-code' };
    base.acquireTokenByCode = async (actual, payload) => {
      assert.strictEqual(actual, request);
      assert.strictEqual(payload, undefined);
      return { marker: 'normal-request' };
    };
    try {
      assert.deepStrictEqual(await new confidential(configuration()).acquireTokenByCode(request), {
        marker: 'normal-request',
      });
    } finally {
      base.acquireTokenByCode = original;
    }
  });

  it('retains returned-state validation without sending a network request', async () => {
    const client = new confidential(configuration());
    await assert.rejects(client.acquireTokenByCode({
      scopes: ['scope'], redirectUri: 'http://localhost/callback', code: 'request-code', state: 'expected',
    }, { code: 'response-code', state: 'different' }), { errorCode: 'state_mismatch' });
  });

  it('rejects unsupported confidential-client instance-aware configuration', () => {
    const config = configuration();
    config.auth.instanceAware = true;
    assert.throws(() => new confidential(config), { errorCode: 'unsupported_cca_instance_aware' });
  });

  it('rejects all instance_aware query values before calling the URL builder', async () => {
    const base = Object.getPrototypeOf(confidential.prototype) as Application;
    const original = base.getAuthCodeUrl;
    base.getAuthCodeUrl = async () => { throw new Error('Unsupported request reached URL builder'); };
    try {
      for (const value of ['true', 'false', '']) {
        await assert.rejects(new confidential(configuration()).getAuthCodeUrl({
          scopes: ['scope'], redirectUri: 'http://localhost/callback',
          extraQueryParameters: { instance_aware: value },
        }), { errorCode: 'unsupported_cca_instance_aware' });
      }
    } finally {
      base.getAuthCodeUrl = original;
    }
  });

  it('retains public-client instance-aware configuration', () => {
    assert.doesNotThrow(() => new msal.PublicClientApplication({
      auth: { clientId: '11111111-1111-1111-1111-111111111111', instanceAware: true },
      system: configuration().system,
    }));
  });

  it('preserves the Azure Identity client-secret token adapter without real credentials', async () => {
    const prototype = confidential.prototype as unknown as {
      acquireTokenByClientCredential(request: Fields): Promise<unknown>;
    };
    const original = prototype.acquireTokenByClientCredential;
    const expiresOn = new Date(Date.now() + 60_000);
    const scopes = ['https://management.azure.com/.default'];
    prototype.acquireTokenByClientCredential = async request => {
      assert.deepStrictEqual(request.scopes, scopes);
      return { accessToken: 'offline-token', expiresOn, tokenType: 'Bearer' };
    };
    const identity = fromVsce('@azure/identity') as {
      ClientSecretCredential: new(tenant: string, client: string, secret: string, options: Fields) => {
        getToken(scopes: string[]): Promise<{ token: string; expiresOnTimestamp: number }>;
      };
    };
    try {
      const credential = new identity.ClientSecretCredential(
        '22222222-2222-2222-2222-222222222222', '11111111-1111-1111-1111-111111111111',
        'offline-test-fixture',
        { httpClient: { sendRequest: async () => { throw new Error('Unexpected network request'); } } },
      );
      assert.deepStrictEqual(await credential.getToken(scopes), {
        token: 'offline-token', expiresOnTimestamp: expiresOn.getTime(),
        refreshAfterTimestamp: undefined, tokenType: 'Bearer',
      });
    } finally {
      prototype.acquireTokenByClientCredential = original;
    }
  });
});
