"""Exercise the live watcher with local source fixtures and no GitHub writes."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
import runpy

ROOT = Path(__file__).resolve().parents[1]
EXTRACT = runpy.run_path(str(ROOT / 'scripts/extract-tool-release.py'))['extract']


class NativeReleaseSources(unittest.TestCase):
    def test_antigravity_installer_manifest_requires_stable_version(self):
        self.assertEqual(EXTRACT('antigravity-manifest', '{"version":"1.4.9","url":"https://example.com/99.0.0"}'), '1.4.9')
        for content in ['{}', '{"version":149}', '{"version":"1.4.9-preview"}', '[]']:
            with self.subTest(content=content), self.assertRaises(ValueError):
                EXTRACT('antigravity-manifest', content)

    def test_grok_rss_uses_only_canonical_stable_build_release_items(self):
        def item(title, link):
            return f'<item><title>{title}</title><link>{link}</link></item>'
        content = '<rss><channel><title>Grok Build Changelog</title>' + ''.join([
            item('Grok Build 1.0.9', 'https://x.ai/changelog/build#v1.0.9-2026-01-01'),
            item('Grok Build 1.0.70', 'https://x.ai/changelog/build#v1.0.70-2026-10-23'),
            item('Grok Build 99.0.0', 'https://example.com/changelog/build#v99.0.0-2026-10-23'),
            item('Grok Build 2.0.0-preview', 'https://x.ai/changelog/build#v2.0.0-preview'),
            item('Grok Build 3.0.0', 'https://x.ai/changelog/api#v3.0.0-2026-10-23'),
        ]) + '</channel></rss>'
        self.assertEqual(EXTRACT('grok-build-rss', content), '1.0.70')
        with self.assertRaises(ValueError):
            EXTRACT('grok-build-rss', content.replace('Grok Build Changelog', 'Grok API Changelog'))



class ReleaseSources(unittest.TestCase):
    def poll(self, kiro, devin, fetch_fails=False):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            baselines = json.loads((ROOT / '.github/tool-release-baselines.json').read_text())
            # Retain extractor coverage using an explicit historical fixture.
            # The production watcher never traverses retired_tools.
            tools = {'kiro': baselines['tools']['kiro'],
                     'windsurf': baselines['retired_tools']['windsurf']}
            for tool in tools.values():
                tool['last_known_version'] = '0.0.0'
                tool['tracked'] = True
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
