"""Exercise the live watcher with local source fixtures and no GitHub writes."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


class ReleaseSources(unittest.TestCase):
    def poll(self, kiro, devin, fetch_fails=False):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            tools = json.loads((ROOT / '.github/tool-release-baselines.json').read_text())['tools']
            tools = {name: tools[name] for name in ['kiro', 'windsurf']}
            for tool in tools.values():
                tool['last_known_version'] = '0.0.0'
            (root / 'baseline.json').write_text(json.dumps({'tools': tools}))
            (root / 'kiro.html').write_text(kiro)
            (root / 'devin.md').write_text(devin)
            (root / 'bin').mkdir()
            (root / 'bin/curl').write_text(
                '#!/bin/sh\ncase "$*" in\n'
                '*kiro*) cat "$SOURCE_FIXTURES/kiro.html"; if [ "$SOURCE_FETCH_FAILS" = 1 ]; then exit 22; fi ;;\n'
                '*devin*|*windsurf*) cat "$SOURCE_FIXTURES/devin.md" ;;\n'
                '*) exit 22 ;;\nesac\n'
            )
            (root / 'bin/gh').write_text('#!/bin/sh\necho "$*" >> "$SOURCE_FIXTURES/gh-calls"\nexit 99\n')
            for program in (root / 'bin').iterdir():
                program.chmod(0o755)
            env = dict(os.environ)
            env.pop('GLM_API_KEY', None)
            env.update(PATH=str(root / 'bin') + os.pathsep + env['PATH'],
                       SOURCE_FIXTURES=str(root), BASELINES_FILE=str(root / 'baseline.json'),
                       UPDATE_BASELINES='true', TOOL_FILTER='',
                       SOURCE_FETCH_FAILS='1' if fetch_fails else '0',
                       GITHUB_STEP_SUMMARY=str(root / 'summary.md'))
            result = subprocess.run(['bash', str(ROOT / 'scripts/check-tool-releases.sh')],
                                    cwd=ROOT, env=env, text=True, capture_output=True, timeout=30)
            self.assertFalse((root / 'gh-calls').exists(), 'dry run must not call GitHub')
            summary = (root / 'summary.md').read_text() if (root / 'summary.md').exists() else ''
            return result, summary

    def test_release_markers_include_kiro_patches_and_devin_versions(self):
        result, summary = self.poll(
            '<h2>2.34.0 CLI</h2><a href="/changelog/cli/2-34/">Release</a>'
            '<a href="/changelog/cli/2-34#patch-2-34-2">2.34.2</a>'
            '<a href="/changelog/ide/99-0/">Other product</a>'
            '<a href="https://example.com/changelog/cli/99-0/">Other host</a>'
            '<p>Future rollout: 3.0.0. Unrelated dependency: 20.10.0</p>',
            '# Changelog\n\n**v3.11.228**\nOctober 26, 2026\n\n**v3.11.216**\nOld Wave 14\n',
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        versions = {entry['id']: entry['version'] for entry in json.loads(summary.split('```json\n')[1].split('```')[0])}
        self.assertEqual(versions['kiro'], '2.34.2')
        self.assertEqual(versions['windsurf'], 'v3.11.228')

    def test_missing_source_markers_fail_the_poll(self):
        result, summary = self.poll('<p>Source unavailable</p>', '# No releases available\n')
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertNotIn('```json', summary, 'incomplete polls must not suggest baseline updates')

    def test_partial_http_response_does_not_count_as_a_successful_source(self):
        result, summary = self.poll(
            '<a href="/changelog/cli/2-34#patch-2-34-2">2.34.2</a>',
            '**v3.11.228**\n', fetch_fails=True,
        )
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertNotIn('```json', summary)


if __name__ == '__main__':
    unittest.main()
