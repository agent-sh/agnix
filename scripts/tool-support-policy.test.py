"""Check roster consistency and exercise monitoring dispatch without network writes."""
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import unittest

ROOT = Path(os.environ.get('AGNIX_POLICY_ROOT', Path(__file__).resolve().parents[1]))
EXPECTED = {
    'claude-code': 's', 'codex': 's',
    'kiro': 'a', 'opencode': 'a', 'opencode-v2': 'a', 'pi': 'a',
    'antigravity': 'b', 'grok-code': 'b',
    'muse-code': 'c', 'kimi-code': 'c',
    'copilot': 'd', 'cline': 'd', 'cursor': 'd', 'gemini-cli': 'd',
    'agentskills-spec': 's',
}


class ToolSupportPolicy(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.baselines = json.loads((ROOT / '.github/tool-release-baselines.json').read_text())

    def test_release_roster_and_documented_tiers_agree(self):
        tools = self.baselines['tools']
        self.assertEqual({key: value['tier'] for key, value in tools.items()}, EXPECTED)
        research = (ROOT / 'knowledge-base/RESEARCH-TRACKING.md').read_text()
        for tool_id, tool in tools.items():
            if tool_id in {'opencode-v2', 'agentskills-spec'}:
                continue
            tier = tool['tier'].upper()
            section = re.search(rf'^### {tier} Tier .*?(?=^### |\Z)', research, re.M | re.S)
            self.assertIsNotNone(section, tier)
            name = 'OpenCode' if tool_id == 'opencode' else tool['display_name']
            self.assertIn('| ' + name + ' |', section[0], tool_id)

    def test_dropped_tools_cannot_be_polled(self):
        dropped = {'continue', 'aider', 'amazon-q', 'tabnine', 'codeium',
                   'devin', 'windsurf', 'amp', 'roo-code'}
        self.assertEqual(set(self.baselines['excluded_tools']), dropped)
        self.assertFalse(dropped.intersection(self.baselines['tools']))
        for tool in self.baselines['retired_tools'].values():
            self.assertFalse(tool['tracked'])
            self.assertNotIn('tier', tool)

    def test_new_tools_have_native_validation_and_release_tracking(self):
        for tool_id, validator in [('pi', 'pi'), ('antigravity', 'antigravity'), ('grok-code', 'grok')]:
            tool = self.baselines['tools'][tool_id]
            self.assertEqual(tool['validation_status'], 'implemented', tool_id)
            self.assertIn(validator, tool['validators'], tool_id)
            self.assertTrue(tool['tracked'], tool_id)
            self.assertIsNotNone(tool['last_known_version'], tool_id)

    def test_community_priorities_do_not_claim_unimplemented_validators(self):
        for tool_id in ['muse-code', 'kimi-code']:
            tool = self.baselines['tools'][tool_id]
            self.assertEqual(tool['validation_status'], 'pending', tool_id)
            self.assertEqual(tool['validators'], [], tool_id)
            if not tool['tracked']:
                self.assertIn('https://', tool['untracked_reason'], tool_id)
                self.assertIsNone(tool['last_known_version'], tool_id)

    def test_spec_sources_follow_current_maintenance_priorities(self):
        sources = json.loads((ROOT / '.github/spec-baselines.json').read_text())['sources']
        self.assertEqual(set(sources), {'s-tier', 'a-tier', 'b-tier', 'd-tier'})
        self.assertTrue({'opencode-rules', 'pi-settings', 'pi-models', 'pi-keybindings', 'pi-theme'}.issubset(sources['a-tier']))
        self.assertTrue({'antigravity-settings', 'antigravity-plugin', 'antigravity-rules', 'grok-build-reference'}.issubset(sources['b-tier']))
        for source in sources['s-tier']:
            self.assertFalse(source.startswith(('opencode', 'cursor', 'cline', 'github-copilot')))
        self.assertEqual(set(sources['d-tier']), {
            'cline-rules', 'github-copilot', 'cursor-environment', 'cursor-environment-schema',
            'cursor-hooks', 'cursor-mcp', 'cursor-rules', 'cursor-subagents',
        })
        names = [name for group in sources.values() for name in group]
        self.assertEqual(len(names), len(set(names)), 'a source must not be polled twice')

    def test_spec_schedule_and_manual_dispatch(self):
        script = ROOT / 'scripts/spec-drift-tiers.sh'
        for event, requested, cron, expected in [
            ('schedule', 'all', '0 0 * * 0', 's-tier'),
            ('schedule', 'all', '0 0 * * 3', 'a-tier b-tier'),
            ('schedule', 'all', '0 0 1 * *', 'd-tier'),
            ('workflow_dispatch', 's-tier', '', 's-tier'),
            ('workflow_dispatch', 'a-tier', '', 'a-tier'),
            ('workflow_dispatch', 'b-tier', '', 'b-tier'),
            ('workflow_dispatch', 'd-tier', '', 'd-tier'),
            ('workflow_dispatch', 'all', '', 's-tier a-tier b-tier d-tier'),
            ('push', 'all', '', 's-tier a-tier b-tier d-tier'),
        ]:
            with self.subTest(event=event, requested=requested, cron=cron):
                result = subprocess.run(
                    ['bash', '-c', 'source "$1"; select_spec_tiers "$2" "$3" "$4"',
                     '_', str(script), event, requested, cron],
                    capture_output=True, text=True, timeout=10)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(result.stdout.strip(), expected)
        workflow = (ROOT / '.github/workflows/spec-drift.yml').read_text()
        self.assertIn('source scripts/spec-drift-tiers.sh', workflow)
        self.assertIn('for tier in $selected_tiers;', workflow)
        for cron in ['0 0 * * 0', '0 0 * * 3', '0 0 1 * *']:
            self.assertIn("cron: '" + cron + "'", workflow)

    def test_release_poll_visits_pi_and_current_kimi_but_not_retired_sources(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            baseline = dict(self.baselines)
            baseline['tools'] = {
                key: dict(self.baselines['tools'][key], last_known_version=version)
                for key, version in [('pi', 'v1.4.0'), ('kimi-code', '@moonshot-ai/kimi-code@2.4.0')]
            }
            (root / 'baseline.json').write_text(json.dumps(baseline))
            (root / 'bin').mkdir()
            (root / 'bin/gh').write_text(
                '#!/bin/sh\nprintf "%s\\n" "$*" >> "$CALL_LOG"\n'
                'case "$*" in\n'
                '"api repos/earendil-works/pi/releases?per_page=100") '
                'printf \'[{"tag_name":"v1.4.1-beta.1","prerelease":true},'
                '{"tag_name":"v1.4.0","prerelease":false}]\' ;;\n'
                '"api repos/MoonshotAI/kimi-code/releases?per_page=100") '
                'printf \'[{"tag_name":"desktop-v9.0.0","prerelease":false},'
                '{"tag_name":"@moonshot-ai/kimi-code@2.4.0","prerelease":false}]\' ;;\n'
                '*) exit 99 ;;\nesac\n')
            (root / 'bin/curl').write_text('#!/bin/sh\nexit 99\n')
            for program in (root / 'bin').iterdir():
                program.chmod(0o755)
            env = dict(os.environ)
            env.pop('GLM_API_KEY', None)
            env.update(PATH=str(root / 'bin') + os.pathsep + env['PATH'],
                       BASELINES_FILE=str(root / 'baseline.json'), UPDATE_BASELINES='true',
                       TOOL_FILTER='', CALL_LOG=str(root / 'calls'),
                       GITHUB_STEP_SUMMARY=str(root / 'summary'))
            result = subprocess.run(['bash', str(ROOT / 'scripts/check-tool-releases.sh')],
                                    cwd=ROOT, env=env, capture_output=True, text=True, timeout=30)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn('ok=2 new=0 skipped=0 errors=0', result.stdout)
            self.assertCountEqual((root / 'calls').read_text().splitlines(), [
                'api repos/earendil-works/pi/releases?per_page=100',
                'api repos/MoonshotAI/kimi-code/releases?per_page=100',
            ])


if __name__ == '__main__':
    unittest.main()
