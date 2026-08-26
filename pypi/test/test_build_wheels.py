#!/usr/bin/env python3
"""Tests for the PyPI wheel builder.

Run with:
    python3 -m unittest discover -s pypi/test
"""

from __future__ import annotations

import csv
import io
import stat
import sys
import tarfile
import tempfile
import unittest
import zipfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import build_wheels as bw  # noqa: E402


def fake_glibc_binary(minors: tuple[int, ...]) -> bytes:
    refs = b"".join(b"GLIBC_2.%d\x00" % minor for minor in minors)
    return b"\x7fELF" + refs + b"payload"


class PlatformTagTests(unittest.TestCase):
    def test_highest_glibc_reference_wins(self):
        with tempfile.TemporaryDirectory() as tmp:
            binary = Path(tmp) / "agnix"
            binary.write_bytes(fake_glibc_binary((2, 17, 4, 14)))
            # Highest referenced symbol is the floor: a binary needing 2.17
            # must not claim to run on 2.4.
            self.assertEqual(
                bw.glibc_platform_tags(binary, "x86_64"),
                ["manylinux_2_17_x86_64", "manylinux2014_x86_64"],
            )

    def test_newer_glibc_gets_no_legacy_alias(self):
        with tempfile.TemporaryDirectory() as tmp:
            binary = Path(tmp) / "agnix"
            binary.write_bytes(fake_glibc_binary((28, 31)))
            self.assertEqual(
                bw.glibc_platform_tags(binary, "aarch64"), ["manylinux_2_31_aarch64"]
            )

    def test_binary_without_glibc_refs_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            binary = Path(tmp) / "agnix"
            binary.write_bytes(b"\x7fELFstatic")
            with self.assertRaises(ValueError):
                bw.glibc_platform_tags(binary, "x86_64")

    def test_explicit_tags_bypass_detection(self):
        musl = next(t for t in bw.TARGETS if t.triple == "x86_64-unknown-linux-musl")
        self.assertEqual(
            bw.platform_tags_for(musl, Path("/nonexistent")), ["musllinux_1_2_x86_64"]
        )


class SidecarTests(unittest.TestCase):
    digest = "a" * 64

    def test_matches_entry_by_basename(self):
        contents = f"{'b' * 64}  other.tar.gz\n{self.digest}  ./dist/agnix.tar.gz\n"
        self.assertEqual(
            bw.parse_sha256_sidecar(contents, "agnix.tar.gz"), self.digest
        )

    def test_accepts_binary_mode_asterisk(self):
        self.assertEqual(
            bw.parse_sha256_sidecar(f"{self.digest} *agnix.zip\n", "agnix.zip"),
            self.digest,
        )

    def test_missing_entry_raises(self):
        with self.assertRaises(ValueError):
            bw.parse_sha256_sidecar(f"{self.digest}  other.zip\n", "agnix.zip")

    def test_malformed_hash_raises(self):
        with self.assertRaises(ValueError):
            bw.parse_sha256_sidecar("nothex  agnix.zip\n", "agnix.zip")


class ExtractTests(unittest.TestCase):
    def test_extracts_from_tar_gz_and_marks_executable(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            archive = tmp_path / "agnix.tar.gz"
            payload = tmp_path / "agnix"
            payload.write_bytes(b"binary")
            with tarfile.open(archive, "w:gz") as tf:
                tf.add(payload, arcname="agnix")

            dest = tmp_path / "out"
            dest.mkdir()
            extracted = bw.extract_binary(archive, "agnix", dest)
            self.assertEqual(extracted.read_bytes(), b"binary")
            self.assertTrue(extracted.stat().st_mode & 0o111)

    def test_extracts_from_zip(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            archive = tmp_path / "agnix.zip"
            with zipfile.ZipFile(archive, "w") as zf:
                zf.writestr("agnix.exe", b"windows binary")

            dest = tmp_path / "out"
            dest.mkdir()
            extracted = bw.extract_binary(archive, "agnix.exe", dest)
            self.assertEqual(extracted.read_bytes(), b"windows binary")


class WheelTests(unittest.TestCase):
    def setUp(self):
        self.meta = bw.read_metadata(bw.PYPI_DIR / "pyproject.toml")

    def build(self, target: bw.Target, tags: list[str], out_dir: Path) -> Path:
        binary = out_dir / "binary"
        binary.write_bytes(b"fake agnix binary")
        return bw.build_wheel(self.meta, target, binary, tags, out_dir / "dist")

    def test_wheel_layout_and_record(self):
        target = next(t for t in bw.TARGETS if t.triple == "x86_64-unknown-linux-gnu")
        with tempfile.TemporaryDirectory() as tmp:
            wheel = self.build(target, ["manylinux_2_17_x86_64"], Path(tmp))
            version = self.meta["version"]
            self.assertEqual(
                wheel.name, f"agnix-{version}-py3-none-manylinux_2_17_x86_64.whl"
            )

            with zipfile.ZipFile(wheel) as zf:
                names = set(zf.namelist())
                dist_info = f"agnix-{version}.dist-info"
                data_scripts = f"agnix-{version}.data/scripts"
                self.assertIn("agnix/__init__.py", names)
                self.assertIn("agnix/__main__.py", names)
                # The binary must live under .data/scripts: installers drop the
                # exec bit anywhere else, which leaves `agnix` unrunnable.
                self.assertIn(f"{data_scripts}/agnix", names)
                self.assertNotIn("agnix/agnix", names)
                self.assertIn(f"{dist_info}/METADATA", names)
                self.assertIn(f"{dist_info}/WHEEL", names)
                self.assertIn(f"{dist_info}/RECORD", names)
                # A console_scripts shim named agnix would collide with the
                # binary the wheel installs under the same name.
                self.assertNotIn(f"{dist_info}/entry_points.txt", names)
                self.assertIn(f"{dist_info}/licenses/LICENSE-MIT", names)
                self.assertIn(f"{dist_info}/licenses/LICENSE-APACHE", names)

                # RECORD must cover every archive member exactly once, with
                # hashes pip can verify.
                record = zf.read(f"{dist_info}/RECORD").decode("utf-8")
                rows = list(csv.reader(io.StringIO(record)))
                recorded = {row[0] for row in rows if row}
                self.assertEqual(recorded, names)
                for row in rows:
                    if not row or row[0] == f"{dist_info}/RECORD":
                        continue
                    self.assertEqual(row[1], bw.record_hash(zf.read(row[0])))
                    self.assertEqual(int(row[2]), len(zf.read(row[0])))

                info = zf.getinfo(f"{data_scripts}/agnix")
                mode = info.external_attr >> 16
                # pip's zip_item_is_executable() needs S_ISREG to pass before
                # it looks at the exec bits at all.
                self.assertTrue(stat.S_ISREG(mode))
                self.assertTrue(mode & 0o111)
                self.assertEqual(info.create_system, 3)

                wheel_meta = zf.read(f"{dist_info}/WHEEL").decode("utf-8")
                self.assertIn("Root-Is-Purelib: false", wheel_meta)
                self.assertIn("Tag: py3-none-manylinux_2_17_x86_64", wheel_meta)

                metadata = zf.read(f"{dist_info}/METADATA").decode("utf-8")
                self.assertIn("Metadata-Version: 2.4", metadata)
                self.assertIn("Name: agnix", metadata)
                self.assertIn(f"Version: {version}", metadata)
                self.assertIn("License-Expression: MIT OR Apache-2.0", metadata)
                self.assertIn("Description-Content-Type: text/markdown", metadata)

    def test_multiple_tags_join_in_filename(self):
        target = next(t for t in bw.TARGETS if t.triple == "x86_64-unknown-linux-gnu")
        with tempfile.TemporaryDirectory() as tmp:
            wheel = self.build(
                target, ["manylinux_2_17_x86_64", "manylinux2014_x86_64"], Path(tmp)
            )
            self.assertTrue(
                wheel.name.endswith(
                    "-py3-none-manylinux_2_17_x86_64.manylinux2014_x86_64.whl"
                ),
                wheel.name,
            )

    def test_windows_wheel_bundles_exe(self):
        target = next(t for t in bw.TARGETS if t.triple == "x86_64-pc-windows-msvc")
        with tempfile.TemporaryDirectory() as tmp:
            wheel = self.build(target, ["win_amd64"], Path(tmp))
            version = self.meta["version"]
            with zipfile.ZipFile(wheel) as zf:
                self.assertIn(f"agnix-{version}.data/scripts/agnix.exe", zf.namelist())

    def test_rebuild_is_reproducible(self):
        target = next(t for t in bw.TARGETS if t.triple == "aarch64-apple-darwin")
        with tempfile.TemporaryDirectory() as tmp:
            first = self.build(target, ["macosx_11_0_arm64"], Path(tmp)).read_bytes()
            second = self.build(target, ["macosx_11_0_arm64"], Path(tmp)).read_bytes()
            self.assertEqual(first, second)


class VersionTests(unittest.TestCase):
    def test_pyproject_matches_workspace_version(self):
        cargo = (bw.REPO_ROOT / "Cargo.toml").read_text(encoding="utf-8")
        section = cargo.split("[workspace.package]", 1)[1]
        workspace_version = next(
            line.split('"')[1]
            for line in section.splitlines()
            if line.startswith("version = ")
        )
        meta = bw.read_metadata(bw.PYPI_DIR / "pyproject.toml")
        self.assertEqual(meta["version"], workspace_version)

        init = (bw.PYPI_DIR / "agnix" / "__init__.py").read_text(encoding="utf-8")
        self.assertIn(f'__version__ = "{workspace_version}"', init)

    def test_version_mismatch_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            code = bw.main(
                ["--artifacts", tmp, "--out", tmp, "--version", "0.0.0-not-real"]
            )
            self.assertEqual(code, 1)


if __name__ == "__main__":
    unittest.main()
