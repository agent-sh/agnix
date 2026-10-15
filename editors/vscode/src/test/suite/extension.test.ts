import * as assert from 'assert';
import * as vscode from 'vscode';
import { buildLspConfig } from '../../extension';

suite('Extension Test Suite', () => {
  vscode.window.showInformationMessage('Start all tests.');

  test('Extension should be present', () => {
    assert.ok(vscode.extensions.getExtension('avifenesh.agnix'));
  });

  test('Extension should activate', async () => {
    const ext = vscode.extensions.getExtension('avifenesh.agnix');
    assert.ok(ext);
    await ext.activate();
    assert.ok(ext.isActive);
  });

  suite('Commands', () => {
    test('agnix.restart command should be registered', async () => {
      const commands = await vscode.commands.getCommands(true);
      assert.ok(commands.includes('agnix.restart'));
    });

    test('agnix.showOutput command should be registered', async () => {
      const commands = await vscode.commands.getCommands(true);
      assert.ok(commands.includes('agnix.showOutput'));
    });

    test('agnix.validateFile command should be registered', async () => {
      const commands = await vscode.commands.getCommands(true);
      assert.ok(commands.includes('agnix.validateFile'));
    });

    test('agnix.validateWorkspace command should be registered', async () => {
      const commands = await vscode.commands.getCommands(true);
      assert.ok(commands.includes('agnix.validateWorkspace'));
    });

    test('agnix.showRules command should be registered', async () => {
      const commands = await vscode.commands.getCommands(true);
      assert.ok(commands.includes('agnix.showRules'));
    });

    test('agnix.fixAll command should be registered', async () => {
      const commands = await vscode.commands.getCommands(true);
      assert.ok(commands.includes('agnix.fixAll'));
    });
  });

  suite('Configuration', () => {
    test('agnix.lspPath should have default value', () => {
      const config = vscode.workspace.getConfiguration('agnix');
      const lspPath = config.get<string>('lspPath');
      assert.strictEqual(lspPath, 'agnix-lsp');
    });

    test('agnix.enable should default to true', () => {
      const config = vscode.workspace.getConfiguration('agnix');
      const enable = config.get<boolean>('enable');
      assert.strictEqual(enable, true);
    });

    test('agnix.trace.server should default to off', () => {
      const config = vscode.workspace.getConfiguration('agnix');
      const trace = config.get<string>('trace.server');
      assert.strictEqual(trace, 'off');
    });

    test('agnix.severity should default to Warning', () => {
      const config = vscode.workspace.getConfiguration('agnix');
      const severity = config.get<string>('severity');
      assert.strictEqual(severity, 'Warning');
    });

    test('agnix.target should default to Generic', () => {
      const config = vscode.workspace.getConfiguration('agnix');
      const target = config.get<string>('target');
      assert.strictEqual(target, 'Generic');
    });

    test('agnix.tools should default to empty array', () => {
      const config = vscode.workspace.getConfiguration('agnix');
      const tools = config.get<string[]>('tools');
      assert.deepStrictEqual(tools, []);
    });
  });

  suite('Rule Configuration', () => {
    const ruleCategories = [
      'skills',
      'hooks',
      'agents',
      'memory',
      'plugins',
      'xml',
      'mcp',
      'imports',
      'crossPlatform',
      'agentsMd',
      'copilot',
      'cursor',
      'promptEngineering',
    ];

    for (const category of ruleCategories) {
      test(`agnix.rules.${category} should default to true`, () => {
        const config = vscode.workspace.getConfiguration('agnix');
        const value = config.get<boolean>(`rules.${category}`);
        assert.strictEqual(value, true, `rules.${category} should be true by default`);
      });
    }

    test('agnix.rules.disabledRules should default to empty array', () => {
      const config = vscode.workspace.getConfiguration('agnix');
      const disabledRules = config.get<string[]>('rules.disabledRules');
      assert.deepStrictEqual(disabledRules, []);
    });
  });

  suite('Version Configuration', () => {
    const versions = ['claudeCode', 'codex', 'cursor', 'copilot'];

    for (const version of versions) {
      test(`agnix.versions.${version} should default to null`, () => {
        const config = vscode.workspace.getConfiguration('agnix');
        const value = config.get<string | null>(`versions.${version}`);
        assert.strictEqual(value, null, `versions.${version} should be null by default`);
      });
    }
  });

  suite('Spec Configuration', () => {
    const specs = ['mcpProtocol', 'agentSkills', 'agentsMd'];

    for (const spec of specs) {
      test(`agnix.specs.${spec} should default to null`, () => {
        const config = vscode.workspace.getConfiguration('agnix');
        const value = config.get<string | null>(`specs.${spec}`);
        assert.strictEqual(value, null, `specs.${spec} should be null by default`);
      });
    }
  });

  suite('Configuration Sync', () => {
    const changedKeys = new Set<string>();

    async function setUserConfig(values: Record<string, unknown>): Promise<void> {
      const config = vscode.workspace.getConfiguration('agnix');
      for (const [key, value] of Object.entries(values)) {
        changedKeys.add(key);
        await config.update(key, value, vscode.ConfigurationTarget.Global);
      }
    }

    teardown(async () => {
      const config = vscode.workspace.getConfiguration('agnix');
      for (const key of changedKeys) {
        await config.update(key, undefined, vscode.ConfigurationTarget.Global);
      }
      changedKeys.clear();
    });

    test('schema defaults do not override project configuration', () => {
      assert.deepStrictEqual(buildLspConfig(), {});
    });

    test('buildLspConfig transforms configured camelCase rules to snake_case', async () => {
      await setUserConfig({
        'rules.crossPlatform': false,
        'rules.agentsMd': true,
        'rules.promptEngineering': false,
      });
      const config = buildLspConfig();
      assert.deepStrictEqual(config.rules, {
        cross_platform: false,
        agents_md: true,
        prompt_engineering: false,
      });
    });

    test('buildLspConfig includes explicitly configured sections', async () => {
      await setUserConfig({
        severity: 'Error',
        target: 'Codex',
        tools: ['codex'],
        'rules.skills': false,
        'versions.claudeCode': '2.1.296',
        'specs.mcpProtocol': '2025-11-25',
      });
      const config = buildLspConfig();
      assert.strictEqual(config.severity, 'Error');
      assert.strictEqual(config.target, 'Codex');
      assert.deepStrictEqual(config.tools, ['codex']);
      assert.deepStrictEqual(config.rules, { skills: false });
      assert.deepStrictEqual(config.versions, { claude_code: '2.1.296' });
      assert.deepStrictEqual(config.specs, { mcp_protocol: '2025-11-25' });
    });

    test('buildLspConfig forwards all configured rule fields', async () => {
      const rules = {
        skills: 'skills', hooks: 'hooks', agents: 'agents', memory: 'memory',
        plugins: 'plugins', xml: 'xml', mcp: 'mcp', imports: 'imports',
        crossPlatform: 'cross_platform', agentsMd: 'agents_md', copilot: 'copilot',
        cursor: 'cursor', promptEngineering: 'prompt_engineering',
      };
      await setUserConfig({
        ...Object.fromEntries(Object.keys(rules).map(key => [`rules.${key}`, false])),
        'rules.disabledRules': ['AS-001'],
      });
      assert.deepStrictEqual(buildLspConfig().rules, {
        ...Object.fromEntries(Object.values(rules).map(field => [field, false])),
        disabled_rules: ['AS-001'],
      });
    });
  });

  suite('Language Support', () => {
    test('skill-markdown language should be registered', async () => {
      const languages = await vscode.languages.getLanguages();
      assert.ok(languages.includes('skill-markdown'));
    });
  });
});
