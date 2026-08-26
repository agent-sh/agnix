#!/usr/bin/env python3
"""Tests for the agnix Python API.

These exercise argument handling and output parsing with a stubbed subprocess,
so they run without the binary the wheels bundle.

Run with:
    python3 -m unittest discover -s pypi/test
"""

from __future__ import annotations

import subprocess
import sys
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import agnix  # noqa: E402


def completed(stdout: str = "", stderr: str = "", returncode: int = 0):
    return subprocess.CompletedProcess(
        args=["agnix"], returncode=returncode, stdout=stdout, stderr=stderr
    )


class LintTests(unittest.TestCase):
    def test_parses_json_report(self):
        report = '{"files": [], "summary": {"errors": 1, "warnings": 0}}'
        with mock.patch.object(agnix, "run", return_value=completed(report)) as run:
            result = agnix.lint(".", tool="claude-code")

        self.assertEqual(result["summary"]["errors"], 1)
        # --target takes the CLI's kebab-case values; clap rejects anything else.
        self.assertEqual(
            run.call_args.args[0],
            ["--format", "json", "--target", "claude-code", "."],
        )

    def test_omits_target_when_no_tool_given(self):
        with mock.patch.object(agnix, "run", return_value=completed("{}")) as run:
            agnix.lint("CLAUDE.md")

        self.assertEqual(run.call_args.args[0], ["--format", "json", "CLAUDE.md"])

    def test_non_json_output_raises_instead_of_reporting_clean(self):
        # A rejected argument exits non-zero with an empty stdout. Returning an
        # empty report here would read as "no issues found".
        rejected = completed(
            stderr="error: invalid value 'ClaudeCode' for '--target <TARGET>'",
            returncode=2,
        )
        with mock.patch.object(agnix, "run", return_value=rejected):
            with self.assertRaises(agnix.AgnixError) as caught:
                agnix.lint(".", tool="ClaudeCode")

        message = str(caught.exception)
        self.assertIn("exited 2", message)
        self.assertIn("invalid value 'ClaudeCode'", message)

    def test_non_json_format_is_returned_raw(self):
        with mock.patch.object(agnix, "run", return_value=completed("2 issues\n")):
            result = agnix.lint(".", fmt="sarif")

        self.assertEqual(result, {"raw": "2 issues\n"})


class VersionTests(unittest.TestCase):
    def test_version_strips_whitespace(self):
        with mock.patch.object(agnix, "run", return_value=completed("agnix 1.2.3\n")):
            self.assertEqual(agnix.version(), "agnix 1.2.3")

    def test_module_version_matches_pyproject(self):
        pyproject = (Path(agnix.__file__).resolve().parent.parent / "pyproject.toml").read_text()
        self.assertIn(f'version = "{agnix.__version__}"', pyproject)


class BinaryPathTests(unittest.TestCase):
    def test_prefers_the_scripts_directory(self):
        with mock.patch.object(agnix.sysconfig, "get_path", return_value="/venv/bin"):
            with mock.patch.object(Path, "is_file", return_value=True):
                self.assertEqual(agnix.binary_path(), Path("/venv/bin") / agnix._BINARY_NAME)

    def test_falls_back_to_path_lookup(self):
        with mock.patch.object(agnix.sysconfig, "get_path", return_value="/venv/bin"):
            with mock.patch.object(Path, "is_file", return_value=False):
                with mock.patch.object(agnix.shutil, "which", return_value="/usr/bin/agnix"):
                    self.assertEqual(agnix.binary_path(), Path("/usr/bin/agnix"))

    def test_raises_when_nothing_is_found(self):
        with mock.patch.object(agnix.sysconfig, "get_path", return_value="/venv/bin"):
            with mock.patch.object(Path, "is_file", return_value=False):
                with mock.patch.object(agnix.shutil, "which", return_value=None):
                    with self.assertRaises(FileNotFoundError):
                        agnix.binary_path()


if __name__ == "__main__":
    unittest.main()
