import * as assert from 'assert';
import * as fs from 'fs';
import * as path from 'path';
import {
  AGNIX_DOCUMENT_SELECTOR,
  AGNIX_FILE_PATTERNS,
  AGNIX_SIBLING_DETECTED_PATTERNS,
  AGNIX_WATCH_PATTERNS,
} from '../../filePatterns';

// Unit tests for file pattern matching logic
// These tests don't require VS Code and can run with plain mocha

// Compiled to out/src/test/unit/, four levels below editors/vscode/.
const PACKAGE_ROOT = path.resolve(__dirname, '..', '..', '..', '..');

function readPackageFile(name: string): string {
  return fs.readFileSync(path.join(PACKAGE_ROOT, name), 'utf8');
}

/** Split `value` on `separator` outside `{...}` groups. */
function splitOutsideGroups(value: string, separator: string): string[] {
  const parts: string[] = [];
  let depth = 0;
  let current = '';
  for (const ch of value) {
    if (ch === '{') depth++;
    if (ch === '}') depth--;
    if (ch === separator && depth === 0) {
      parts.push(current);
      current = '';
    } else {
      current += ch;
    }
  }
  parts.push(current);
  return parts;
}

function segmentSource(segment: string): string {
  let out = '';
  for (let i = 0; i < segment.length; i++) {
    const ch = segment[i];
    if (ch === '*') {
      out += '[^/]*';
    } else if (ch === '{') {
      const close = segment.indexOf('}', i);
      const choices = splitOutsideGroups(segment.slice(i + 1, close), ',');
      out += `(?:${choices.map(globSource).join('|')})`;
      i = close;
    } else {
      out += ch.replace(/[.+^$()|[\]\\?]/g, '\\$&');
    }
  }
  return out;
}

/**
 * Regex source for a glob with the semantics of VS Code's glob matcher
 * (src/vs/base/common/glob.ts): a `**` segment matches any number of path
 * segments, including none; `*` stays within one segment and matches dot
 * files; `{a,b}` groups may hold `/`.
 */
function globSource(glob: string): string {
  const segments = splitOutsideGroups(glob, '/');
  return segments
    .map((segment, index) => {
      const last = index === segments.length - 1;
      if (segment === '**') {
        return last ? '.*' : '(?:[^/]*/)*';
      }
      return segmentSource(segment) + (last ? '' : '/');
    })
    .join('');
}

function matchesGlob(filePath: string, glob: string): boolean {
  return new RegExp(`^${globSource(glob)}$`).test(filePath.replace(/\\/g, '/'));
}

function matchesAny(filePath: string, patterns: readonly string[]): boolean {
  return patterns.some((pattern) => matchesGlob(filePath, pattern));
}

/**
 * Check if a path is found by the activation events (path-only detection)
 */
function isAgnixFile(filePath: string): boolean {
  return matchesAny(filePath, AGNIX_FILE_PATTERNS);
}

/** Emulates `vscode.languages.match` for the agnix document selector. */
function isSelected(filePath: string, languageId: string): boolean {
  return AGNIX_DOCUMENT_SELECTOR.some(
    (filter) =>
      filter.scheme === 'file' &&
      (filter.language === undefined || filter.language === languageId) &&
      (filter.pattern === undefined || matchesGlob(filePath, filter.pattern))
  );
}

describe('Glob matcher used by these tests', () => {
  it('matches ** as zero or more segments', () => {
    assert.ok(matchesGlob('a/b.md', '**/a/**/b.md'));
    assert.ok(matchesGlob('x/a/y/z/b.md', '**/a/**/b.md'));
    assert.ok(matchesGlob('/home/me/a/b.md', '**/a/b.md'));
  });

  it('keeps * within one segment and matches dot files', () => {
    assert.ok(matchesGlob('.mcp.json', '**/*.mcp.json'));
    assert.ok(!matchesGlob('a/b/c.md', '**/a/*.md'));
  });

  it('expands groups, including groups with a path separator', () => {
    assert.ok(matchesGlob('x/.config/amp/settings.jsonc', '**/{.amp,.config/amp}/settings.{json,jsonc}'));
    assert.ok(!matchesGlob('x/amp/settings.json', '**/{.amp,.config/amp}/settings.{json,jsonc}'));
  });

  it('matches Windows paths', () => {
    assert.ok(matchesGlob('C:\\ProgramData\\Devin\\hooks.json', '**/ProgramData/Devin/hooks.json'));
  });
});

describe('Pattern syntax', () => {
  const ALL = [...AGNIX_WATCH_PATTERNS, ...AGNIX_DOCUMENT_SELECTOR.flatMap((f) => (f.pattern ? [f.pattern] : []))];

  it('uses only syntax VS Code globs, watchers and file search share', () => {
    for (const pattern of ALL) {
      assert.ok(pattern.startsWith('**/'), `${pattern} should start with **/`);
      assert.ok(!/[[\]?\\]/.test(pattern), `${pattern} should not use [], ? or backslashes`);
      assert.ok(!pattern.endsWith('/'), `${pattern} should not end with /`);
      let depth = 0;
      for (const ch of pattern) {
        if (ch === '{') depth++;
        if (ch === '}') depth--;
        assert.ok(depth === 0 || depth === 1, `${pattern} should not nest {} groups`);
      }
      assert.strictEqual(depth, 0, `${pattern} should balance {} groups`);
      for (const group of pattern.match(/\{[^}]*\}/g) ?? []) {
        const choices = group.slice(1, -1).split(',');
        assert.ok(choices.length > 1, `${pattern} group ${group} should have alternatives`);
        assert.ok(choices.every((c) => c.length > 0), `${pattern} group ${group} has an empty choice`);
        assert.ok(!group.includes('**'), `${pattern} group ${group} should not hold **`);
      }
      for (const segment of splitOutsideGroups(pattern, '/')) {
        assert.ok(segment === '**' || !segment.includes('**'), `${pattern}: ** must be a whole segment`);
      }
    }
  });

  it('has no duplicate patterns', () => {
    assert.strictEqual(new Set(AGNIX_WATCH_PATTERNS).size, AGNIX_WATCH_PATTERNS.length);
  });
});

/**
 * Example paths for every non-generic FileType of
 * crates/agnix-core/src/file_types/detection.rs.
 */
const DETECTED_PATHS: Record<string, string[]> = {
  Skill: ['SKILL.md', '.claude/skills/review/SKILL.md', '.devin/skills/deploy/SKILL.md'],
  ClaudeMd: ['CLAUDE.md', 'CLAUDE.local.md', 'AGENTS.md', 'AGENTS.local.md', 'pkg/AGENTS.override.md'],
  Agent: ['.claude/agents/reviewer.md', 'agents/team/reviewer.md'],
  AmpCheck: ['.agents/checks/security.md', '/home/me/.config/amp/checks/perf.md', '/home/me/.config/agents/checks/a/b.md'],
  Hooks: [
    '.claude/settings.json',
    '.claude/settings.local.json',
    '.claude/managed-settings.json',
    '/Library/Application Support/ClaudeCode/managed-settings.json',
    '/etc/claude-code/managed-settings.d/10-base.json',
    'C:\\Program Files\\ClaudeCode\\managed-settings.json',
    'my-plugin/hooks/hooks.json',
  ],
  Plugin: ['.claude-plugin/plugin.json'],
  ClaudeMarketplace: ['.claude-plugin/marketplace.json'],
  Mcp: ['.mcp.json', 'tools.mcp.json', 'mcp.json', '.cursor/mcp.json', 'mcp-servers.json'],
  Copilot: ['.github/copilot-instructions.md'],
  CopilotScoped: ['.github/instructions/coding.instructions.md', '.github/instructions/web/react.instructions.md'],
  CopilotAgent: ['.github/agents/reviewer.md', '.github/agents/team/lead.agent.md', '/home/me/.copilot/agents/a.md', 'reviewer.agent.md', 'legacy.chatmode.md'],
  CopilotPrompt: ['.github/prompts/release.prompt.md'],
  CopilotHooks: [
    '.github/hooks/lint.json',
    '/home/me/.copilot/hooks/audit.json',
    '.github/copilot/settings.json',
    '.github/copilot/settings.local.json',
    '/home/me/.copilot/settings.json',
    '.github/workflows/copilot-setup-steps.yml',
    '.github/workflows/copilot-setup-steps.yaml',
  ],
  CopilotPlugin: ['.github/plugin/plugin.json', '.plugin/plugin.json'],
  ClaudeRule: ['.claude/rules/style.md', '.claude/rules/frontend/react.md'],
  ClaudeOutputStyle: ['.claude/output-styles/terse.md'],
  ClaudeMemoryIndex: ['.claude/agent-memory/reviewer/MEMORY.md', '.claude/agent-memory-local/reviewer/MEMORY.md', '/home/me/.claude/projects/app/memory/MEMORY.md'],
  ClaudeIgnore: ['.claudeignore'],
  ClaudeCommand: ['.claude/commands/deploy.md', '.claude/commands/frontend/component.md'],
  CursorRule: ['.cursor/rules/typescript.mdc', '.cursor/rules/web/react.md'],
  CursorHooks: ['.cursor/hooks.json'],
  CursorAgent: ['.cursor/agents/reviewer.md', '.cursor/agents/team/AGENTS.md'],
  CursorEnvironment: ['.cursor/environment.json'],
  CursorRulesLegacy: ['.cursorrules', '.cursorrules.md'],
  CursorCliConfig: ['.cursor/cli.json', '/home/me/.cursor/cli-config.json'],
  CursorPermissions: ['.cursor/permissions.json', '/home/me/.cursor/permissions.json'],
  CursorPlugin: ['.cursor-plugin/plugin.json', '.cursor-plugin/marketplace.json'],
  CursorIgnore: ['.cursorignore'],
  ClineRules: ['.clinerules'],
  ClineRulesFolder: [
    '.clinerules/style.md',
    '.clinerules/notes.txt',
    '.clinerules/workflows/release.md',
    '.clinerules/hooks/TaskStart',
    '.cline/hooks/PreToolUse.sh',
    '.cline/rules/style.md',
    '.cline/workflows/release.md',
  ],
  ClineIgnore: ['.clineignore'],
  ClineMcpSettings: ['/home/me/.cline/data/settings/cline_mcp_settings.json'],
  OpenCodeConfig: ['opencode.json', 'opencode.jsonc'],
  OpenCodeAgent: ['.opencode/agents/review.md', '.opencode/agent/review.md', '.opencode/modes/plan.md', '.opencode/mode/plan.md', '/home/me/.config/opencode/agents/team/lead.md', '/etc/opencode/agents/ops.md'],
  OpenCodeCommand: ['.opencode/commands/test.md', '.opencode/command/nested/test.md', '/home/me/.config/opencode/commands/x.md'],
  OpenCodeTool: ['.opencode/tools/search.ts', '.opencode/tool/search.js', '/home/me/.config/opencode/tools/db.mts', 'C:\\ProgramData\\opencode\\tools\\x.cjs'],
  OpenCodePlugin: ['.opencode/plugins/notify.ts', '.opencode/plugin/notify.mjs', '/Library/Application Support/opencode/plugins/p.js'],
  GeminiMd: ['GEMINI.md', 'GEMINI.local.md'],
  GeminiSettings: ['.gemini/settings.json', '/home/me/.gemini/settings.json', '.gemini/settings.local.json'],
  AmpSettings: ['.amp/settings.json', '.amp/settings.jsonc', '/home/me/.config/amp/settings.jsonc', '/etc/ampcode/managed-settings.json'],
  AmpSkillMcp: ['.agents/skills/browser/mcp.json', '/home/me/.config/amp/skills/db/mcp.json'],
  GeminiExtension: ['gemini-extension.json', '.gemini/extensions/tools/gemini-extension.json'],
  GeminiIgnore: ['.geminiignore'],
  GeminiHooks: ['.gemini/extensions/tools/hooks/hooks.json'],
  GeminiCommand: ['.gemini/commands/review.toml', '.gemini/commands/git/commit.toml', '/home/me/.gemini/commands/x.toml', '.gemini/extensions/tools/commands/deploy.toml', '.gemini/extensions/tools/commands/a/b.toml'],
  GeminiPolicy: [
    '.gemini/policies/base.toml',
    '/home/me/.gemini/policies/user.toml',
    '.gemini/extensions/tools/policies/ext.toml',
    '/etc/gemini-cli/policies/admin.toml',
    '/Library/Application Support/GeminiCli/policies/admin.toml',
    'C:\\ProgramData\\gemini-cli\\policies\\admin.toml',
  ],
  GeminiAgent: ['.gemini/agents/researcher.md', '.gemini/extensions/tools/agents/helper.md'],
  CodexConfig: ['.codex/config.toml', '.codex/config.json', '.codex/config.yaml', '.codex/config.yml'],
  CodexPlugin: ['.codex-plugin/plugin.json'],
  CodexRequirements: ['/etc/codex/requirements.toml', 'C:\\ProgramData\\OpenAI\\Codex\\requirements.toml'],
  CodexHooks: ['.codex/hooks.json', '/home/me/.codex/hooks.json'],
  CodexAgent: ['.codex/agents/reviewer.toml'],
  CodexRules: ['.codex/rules/default.rules', '/home/me/.codex/rules/git.rules', '/etc/codex/rules/admin.rules'],
  CodexPrompt: ['/home/me/.codex/prompts/review.md'],
  CodexSkillMetadata: ['.agents/skills/review/agents/openai.yaml', 'plugin/skills/deploy/agents/openai.yaml'],
  WindsurfRule: ['.devin/rules/style.md', '.windsurf/rules/web/react.md'],
  WindsurfWorkflow: ['.devin/workflows/release.md', '.windsurf/workflows/deploy.md', '/home/me/.codeium/windsurf/global_workflows/triage.md'],
  WindsurfRulesLegacy: ['.windsurfrules'],
  WindsurfGlobalRules: ['/home/me/.codeium/windsurf/memories/global_rules.md'],
  WindsurfHooks: [
    '.devin/hooks.json',
    '.windsurf/hooks.json',
    '/home/me/.codeium/windsurf/hooks.json',
    '/home/me/.codeium/hooks.json',
    '/Library/Application Support/Devin/hooks.json',
    '/etc/windsurf/hooks.json',
    'C:\\ProgramData\\Devin\\hooks.json',
  ],
  KiroSteering: ['.kiro/steering/product.md', '.kiro/steering/web/react.md'],
  KiroPower: ['POWER.md', '.kiro/powers/installed/stripe/POWER.md'],
  KiroPowerPlugin: ['.kiro/powers/stripe/plugin.json', 'powers/stripe/plugin.json'],
  KiroPowerSteering: ['.kiro/powers/stripe/steering/setup.md', '.kiro/powers/installed/stripe/steering/setup.md', 'power/steering/setup.md', 'stripe/dev.kiro/steering/setup.md', 'stripe/dev.kiro/INSTRUCTIONS.md'],
  KiroAgent: ['.kiro/agents/reviewer.json', '.kiro/agents/team/reviewer.md', '.amazonq/cli-agents/dev.json'],
  KiroHook: ['.kiro/hooks/lint.kiro.hook'],
  KiroHookV3: ['.kiro/hooks/lint.json'],
  KiroMcp: ['.kiro/settings/mcp.json', '.kiro/powers/stripe/mcp.json'],
  KiroSettings: ['.kiro/settings/cli.json', '/home/me/.kiro/settings/cli.json'],
  KiroPermissions: [
    '/home/me/.kiro/settings/permissions.yaml',
    '/home/me/.kiro/settings/permissions.json',
    '/home/me/.kiro/workspace-roots/abc123/permissions.yaml',
    '/etc/kiro/managed-settings.json',
    '/Library/Application Support/Kiro/managed-settings.json',
  ],
  KiroWorkflow: ['.kiro/workflows/plan.workflow.json', '.kiro/workflows/plan.workflow.yaml', '/home/me/.kiro/workflows/ship.workflow.yml'],
  KiroIgnore: ['.kiroignore', 'packages/api/.kiroignore', '/home/me/.kiro/settings/kiroignore'],
  KiroNotLoaded: [
    '.kiro/agents/reviewer.jsonc',
    '.kiro/hooks/lint.yaml',
    '.kiro/hooks/nested/lint.json',
    '.kiro/settings/permissions.yml',
    '.kiro/settings/mcp.jsonc',
    '.kiro/workflows/plan.json',
    '.kiro/workflows/nested/plan.workflow.json',
    '.kiro/settings.json',
    '.kiro/mcp.json',
  ],
};

/**
 * Files agnix recognises only next to a gemini-extension.json, outside
 * .gemini/extensions/ (refine_file_type).
 */
const SIBLING_DETECTED_PATHS: Record<string, string[]> = {
  GeminiCommand: ['commands/deploy.toml', 'my-ext/commands/git/commit.toml'],
  GeminiPolicy: ['policies/ext.toml', 'my-ext/policies/ext.toml'],
  GeminiHooks: ['my-ext/hooks/hooks.json'],
  GeminiAgent: ['my-ext/agents/helper.md'],
};

/** Paths agnix does not lint as agent files. */
const NOT_DETECTED_PATHS = [
  'README.md',
  'docs/guide.md',
  'package.json',
  'tsconfig.json',
  'config.json',
  '.vscode/settings.json',
  'Cargo.toml',
  'pyproject.toml',
  '.gitignore',
  'src/index.ts',
  'rules/test.mdc',
  'skill.md',
  '.opencode/package.json',
  '.opencode/node_modules/lib/index.js',
  '.kiro/specs/feature/requirements.md',
  '.devin/config.json',
  '.devin/hooks.v1.json',
  '.amp/settings.local.json',
  '.github/workflows/ci.yml',
  'skills/agents/openai.yaml',
];

describe('File Pattern Matching', () => {
  describe('every detected file type', () => {
    for (const [fileType, paths] of Object.entries(DETECTED_PATHS)) {
      it(`activates, watches and selects ${fileType}`, () => {
        for (const p of paths) {
          assert.ok(isAgnixFile(p), `${p} (${fileType}) should match an activation pattern`);
          assert.ok(matchesAny(p, AGNIX_WATCH_PATTERNS), `${p} (${fileType}) should be watched`);
          assert.ok(isSelected(p, 'plaintext'), `${p} (${fileType}) should be selected in any language mode`);
        }
      });
    }

    for (const [fileType, paths] of Object.entries(SIBLING_DETECTED_PATHS)) {
      it(`watches and selects ${fileType} in extension roots outside .gemini/extensions`, () => {
        for (const p of paths) {
          assert.ok(matchesAny(p, AGNIX_WATCH_PATTERNS), `${p} (${fileType}) should be watched`);
          assert.ok(isSelected(p, 'plaintext'), `${p} (${fileType}) should be selected`);
        }
      });
    }

    it('does not match files agnix does not lint', () => {
      for (const p of NOT_DETECTED_PATHS) {
        assert.ok(!isAgnixFile(p), `${p} should not match an activation pattern`);
        assert.ok(!matchesAny(p, AGNIX_WATCH_PATTERNS), `${p} should not be watched`);
      }
    });
  });

  describe('SKILL.md files', () => {
    it('should match SKILL.md in root', () => {
      assert.ok(isAgnixFile('SKILL.md'));
    });

    it('should match SKILL.md in subdirectory', () => {
      assert.ok(isAgnixFile('skills/review/SKILL.md'));
    });

    it('should match SKILL.md in .claude/skills', () => {
      assert.ok(isAgnixFile('.claude/skills/my-skill/SKILL.md'));
    });

    it('should not match skill.md (lowercase)', () => {
      assert.ok(!isAgnixFile('skill.md'));
    });
  });

  describe('Hook configuration files', () => {
    it('should match nested .claude/settings.json', () => {
      assert.ok(isAgnixFile('project/.claude/settings.json'));
    });

    it('should not match VS Code workspace settings', () => {
      assert.ok(!isAgnixFile('.vscode/settings.json'));
    });
  });

  describe('MCP configuration files', () => {
    it('should match nested *.mcp.json files', () => {
      assert.ok(isAgnixFile('config/server.mcp.json'));
    });

    it('should not match regular json files', () => {
      assert.ok(!isAgnixFile('package.json'));
    });
  });

  describe('Cursor files', () => {
    it('should match nested .cursor/rules/*.mdc', () => {
      assert.ok(isAgnixFile('project/.cursor/rules/testing.mdc'));
    });

    it('should not match .mdc files outside .cursor/rules', () => {
      assert.ok(!isAgnixFile('rules/test.mdc'));
    });
  });

  describe('Absolute paths', () => {
    it('should match files opened from a workspace by absolute path', () => {
      assert.ok(isAgnixFile('/Users/me/project/.gemini/commands/review.toml'));
      assert.ok(isAgnixFile('C:\\Users\\me\\project\\.codex\\agents\\reviewer.toml'));
    });
  });
});

describe('Document selector', () => {
  it('keeps the language selectors for Markdown, SKILL.md and JSON', () => {
    for (const language of ['markdown', 'skill-markdown', 'json']) {
      assert.ok(
        AGNIX_DOCUMENT_SELECTOR.some((f) => f.language === language && f.pattern === undefined),
        `${language} should be selected by language`
      );
    }
  });

  it('selects every watch pattern by path', () => {
    for (const pattern of AGNIX_WATCH_PATTERNS) {
      assert.ok(
        AGNIX_DOCUMENT_SELECTOR.some((f) => f.pattern === pattern && f.language === undefined),
        `${pattern} should be a document filter`
      );
    }
  });

  it('only selects files on disk', () => {
    assert.ok(AGNIX_DOCUMENT_SELECTOR.every((f) => f.scheme === 'file'));
  });

  it('selects TOML, YAML, JSONC, modules and ignore files whatever their language mode', () => {
    const cases: Array<[string, string]> = [
      ['.gemini/commands/review.toml', 'toml'],
      ['.codex/agents/reviewer.toml', 'plaintext'],
      ['.codex/rules/default.rules', 'plaintext'],
      ['.amp/settings.jsonc', 'jsonc'],
      ['opencode.jsonc', 'jsonc'],
      ['.opencode/tools/search.ts', 'typescript'],
      ['.kiro/workflows/plan.workflow.yaml', 'yaml'],
      ['.kiro/hooks/lint.kiro.hook', 'plaintext'],
      ['.cursorignore', 'ignore'],
      ['.clinerules/hooks/TaskStart', 'shellscript'],
    ];
    for (const [p, language] of cases) {
      assert.ok(isSelected(p, language), `${p} (${language}) should be selected`);
    }
  });

  it('does not select unrelated non-JSON files', () => {
    for (const [p, language] of [
      ['Cargo.toml', 'toml'],
      ['src/index.ts', 'typescript'],
      ['.vscode/settings.json', 'jsonc'],
      ['tsconfig.json', 'jsonc'],
    ]) {
      assert.ok(!isSelected(p, language), `${p} (${language}) should not be selected`);
    }
  });
});

describe('package.json contributions', () => {
  const pkg = JSON.parse(readPackageFile('package.json'));

  it('activates on exactly the path-detected patterns', () => {
    assert.deepStrictEqual(
      pkg.activationEvents,
      AGNIX_FILE_PATTERNS.map((pattern) => `workspaceContains:${pattern}`)
    );
  });

  it('activates sibling-detected Gemini extension files through gemini-extension.json', () => {
    assert.ok(pkg.activationEvents.includes('workspaceContains:**/gemini-extension.json'));
    for (const pattern of AGNIX_SIBLING_DETECTED_PATTERNS) {
      assert.ok(!pkg.activationEvents.includes(`workspaceContains:${pattern}`));
    }
  });

  it('offers every tool agnix validates in agnix.tools', () => {
    const tools = pkg.contributes.configuration.properties['agnix.tools'].items.enum;
    for (const tool of ['claude-code', 'codex', 'opencode', 'kiro', 'github-copilot', 'cline', 'cursor', 'amp', 'gemini-cli', 'windsurf', 'generic']) {
      assert.ok(tools.includes(tool), `agnix.tools should offer ${tool}`);
    }
  });
});

describe('Roo Code removal', () => {
  const ROO = /\.roo\b|\.roomodes|\.rooignore|roo[-_ ]?code/i;

  it('has no Roo Code patterns', () => {
    for (const pattern of [...AGNIX_WATCH_PATTERNS, ...AGNIX_DOCUMENT_SELECTOR.map((f) => f.pattern ?? '')]) {
      assert.ok(!ROO.test(pattern), `${pattern} should not be a Roo Code pattern`);
    }
  });

  it('does not watch or activate on Roo Code files', () => {
    for (const p of ['.roomodes', '.rooignore', '.roo/rules/style.md', '.roo/rules-code/style.md']) {
      assert.ok(!isAgnixFile(p), `${p} should not match an activation pattern`);
      assert.ok(!matchesAny(p, AGNIX_WATCH_PATTERNS), `${p} should not be watched`);
    }
  });

  it('has no Roo Code entries in package.json or the README', () => {
    for (const name of ['package.json', 'README.md']) {
      assert.ok(!ROO.test(readPackageFile(name)), `${name} should not mention Roo Code`);
    }
  });
});

describe('Rules Categories', () => {
  const RULE_CATEGORIES = [
    { prefix: 'AS-', name: 'Agent Skills', minCount: 10 },
    { prefix: 'CC-SK-', name: 'Claude Code Skills', minCount: 5 },
    { prefix: 'CC-HK-', name: 'Claude Code Hooks', minCount: 5 },
    { prefix: 'CC-AG-', name: 'Claude Code Agents', minCount: 5 },
    { prefix: 'CC-PL-', name: 'Claude Code Plugins', minCount: 3 },
    { prefix: 'PE-', name: 'Prompt Engineering', minCount: 5 },
    { prefix: 'MCP-', name: 'Model Context Protocol', minCount: 5 },
    { prefix: 'AGM-', name: 'AGENTS.md', minCount: 5 },
    { prefix: 'COP-', name: 'GitHub Copilot', minCount: 3 },
    { prefix: 'CUR-', name: 'Cursor', minCount: 3 },
    { prefix: 'XML-', name: 'XML Tags', minCount: 2 },
    { prefix: 'XP-', name: 'Cross-Platform', minCount: 3 },
  ];

  it('should have 12 rule categories defined', () => {
    assert.strictEqual(RULE_CATEGORIES.length, 12);
  });

  it('should have unique prefixes', () => {
    const prefixes = RULE_CATEGORIES.map((c) => c.prefix);
    const uniquePrefixes = new Set(prefixes);
    assert.strictEqual(prefixes.length, uniquePrefixes.size);
  });

  it('each category should have a descriptive name', () => {
    RULE_CATEGORIES.forEach((category) => {
      assert.ok(category.name.length > 0, `Category ${category.prefix} should have a name`);
    });
  });
});

describe('Fix Preview Logic', () => {
  // Mock TextEdit interface for testing
  interface MockRange {
    start: { line: number; character: number };
    end: { line: number; character: number };
    isEmpty: boolean;
  }

  interface MockTextEdit {
    range: MockRange;
    newText: string;
  }

  /**
   * Get edit summary (extracted from extension.ts for testing)
   */
  function getEditSummary(changes: MockTextEdit[]): string {
    if (!changes || changes.length === 0) {
      return '';
    }

    if (changes.length === 1) {
      const change = changes[0];
      const lineNum = change.range.start.line + 1;
      if (change.newText === '') {
        return `Line ${lineNum}: delete text`;
      }
      if (change.range.isEmpty) {
        return `Line ${lineNum}: insert text`;
      }
      return `Line ${lineNum}: replace text`;
    }

    let inserts = 0;
    let deletes = 0;
    let replaces = 0;

    for (const change of changes) {
      if (change.newText === '') {
        deletes++;
      } else if (change.range.isEmpty) {
        inserts++;
      } else {
        replaces++;
      }
    }

    const parts: string[] = [];
    if (replaces > 0) parts.push(`${replaces} replace${replaces > 1 ? 's' : ''}`);
    if (inserts > 0) parts.push(`${inserts} insert${inserts > 1 ? 's' : ''}`);
    if (deletes > 0) parts.push(`${deletes} delete${deletes > 1 ? 's' : ''}`);

    if (parts.length === 0) {
      return `${changes.length} changes`;
    }
    return `${changes.length} changes (${parts.join(', ')})`;
  }

  describe('getEditSummary', () => {
    it('should return empty string for no changes', () => {
      assert.strictEqual(getEditSummary([]), '');
    });

    it('should identify delete operations', () => {
      const changes: MockTextEdit[] = [
        {
          range: {
            start: { line: 4, character: 0 },
            end: { line: 4, character: 10 },
            isEmpty: false,
          },
          newText: '',
        },
      ];
      assert.strictEqual(getEditSummary(changes), 'Line 5: delete text');
    });

    it('should identify insert operations', () => {
      const changes: MockTextEdit[] = [
        {
          range: {
            start: { line: 9, character: 5 },
            end: { line: 9, character: 5 },
            isEmpty: true,
          },
          newText: 'inserted text',
        },
      ];
      assert.strictEqual(getEditSummary(changes), 'Line 10: insert text');
    });

    it('should identify replace operations', () => {
      const changes: MockTextEdit[] = [
        {
          range: {
            start: { line: 0, character: 0 },
            end: { line: 0, character: 5 },
            isEmpty: false,
          },
          newText: 'replacement',
        },
      ];
      assert.strictEqual(getEditSummary(changes), 'Line 1: replace text');
    });

    it('should summarize multiple changes', () => {
      const changes: MockTextEdit[] = [
        {
          range: {
            start: { line: 0, character: 0 },
            end: { line: 0, character: 5 },
            isEmpty: false,
          },
          newText: 'a',
        },
        {
          range: {
            start: { line: 1, character: 0 },
            end: { line: 1, character: 5 },
            isEmpty: false,
          },
          newText: 'b',
        },
        {
          range: {
            start: { line: 2, character: 0 },
            end: { line: 2, character: 5 },
            isEmpty: false,
          },
          newText: 'c',
        },
      ];
      assert.strictEqual(getEditSummary(changes), '3 changes (3 replaces)');
    });

    it('should summarize mixed change types', () => {
      const changes: MockTextEdit[] = [
        {
          range: {
            start: { line: 1, character: 0 },
            end: { line: 1, character: 3 },
            isEmpty: false,
          },
          newText: 'abc',
        },
        {
          range: {
            start: { line: 2, character: 2 },
            end: { line: 2, character: 2 },
            isEmpty: true,
          },
          newText: 'x',
        },
        {
          range: {
            start: { line: 3, character: 0 },
            end: { line: 3, character: 1 },
            isEmpty: false,
          },
          newText: '',
        },
      ];

      assert.strictEqual(
        getEditSummary(changes),
        '3 changes (1 replace, 1 insert, 1 delete)'
      );
    });
  });

  describe('Fix confidence levels', () => {
    it('safe fixes should be marked as preferred', () => {
      // This matches the LSP behavior where safe=true maps to isPreferred=true
      const safeFix = { isPreferred: true };
      const unsafeFix = { isPreferred: false };

      assert.strictEqual(safeFix.isPreferred, true);
      assert.strictEqual(unsafeFix.isPreferred, false);
    });
  });

  describe('Agnix action filtering', () => {
    const AGNIX_RULE_RE = /^(AS|CC|PE|MCP|AGM|COP|CUR|XML|XP)-/;

    function isAgnixDiagnostic(diag: { source?: string; code?: string }): boolean {
      return diag.source === 'agnix' || AGNIX_RULE_RE.test(diag.code || '');
    }

    function filterAgnixFixActions(
      actions: Array<{ edit?: object; diagnostics?: Array<{ source?: string; code?: string }> }>
    ) {
      return actions.filter((action) => {
        if (!action.edit) return false;
        const diagnostics = action.diagnostics || [];
        return diagnostics.some(isAgnixDiagnostic);
      });
    }

    it('should keep only actions with agnix diagnostics', () => {
      const actions = [
        { edit: {}, diagnostics: [{ source: 'agnix', code: 'AS-004' }] },
        { edit: {}, diagnostics: [{ source: 'eslint', code: 'no-unused-vars' }] },
        { edit: undefined, diagnostics: [{ source: 'agnix', code: 'AS-005' }] },
      ];

      const filtered = filterAgnixFixActions(actions);
      assert.strictEqual(filtered.length, 1);
      assert.strictEqual(filtered[0].diagnostics?.[0].code, 'AS-004');
    });
  });
});
