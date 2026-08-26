#!/usr/bin/env python3
"""Build platform-specific agnix wheels from the release archives.

The wheels are repacks, not source builds: release.yml already cross-builds
`agnix` for every supported target, checksums each archive, and attests it.
This script takes those archives and wraps each binary in a wheel whose
platform tag matches the target, so `pip install agnix` and `uvx agnix` get a
prebuilt binary with no Rust toolchain and no post-install download.

Usage:
    python3 pypi/build_wheels.py --artifacts artifacts --out pypi/dist
    python3 pypi/build_wheels.py --artifacts artifacts --version 0.49.0

Every archive named in TARGETS must be present unless --allow-missing is passed.
"""

from __future__ import annotations

import argparse
import base64
import csv
import hashlib
import io
import re
import shutil
import stat
import sys
import tarfile
import tempfile
import zipfile
from dataclasses import dataclass
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - Python < 3.11
    print("build_wheels.py needs Python 3.11+ for tomllib", file=sys.stderr)
    raise SystemExit(2) from None

REPO_ROOT = Path(__file__).resolve().parent.parent
PYPI_DIR = REPO_ROOT / "pypi"

# Wheels are pure-Python plus a binary: no CPython ABI is linked, so the
# interpreter tag stays py3 and only the platform tag varies.
PYTHON_TAG = "py3"
ABI_TAG = "none"
GENERATOR = "agnix build_wheels.py"

# Zip entries get a fixed timestamp so rebuilding the same release produces
# byte-identical wheels.
ZIP_DATE_TIME = (1980, 1, 1, 0, 0, 0)

LICENSE_FILES = ("LICENSE-MIT", "LICENSE-APACHE")


@dataclass(frozen=True)
class Target:
    """One release target and the wheel it turns into."""

    triple: str
    archive: str
    binary: str
    # Platform tag, or None when it has to be derived from the binary.
    platform_tag: str | None = None


TARGETS = (
    Target("x86_64-unknown-linux-gnu", "agnix-x86_64-unknown-linux-gnu.tar.gz", "agnix"),
    Target("aarch64-unknown-linux-gnu", "agnix-aarch64-unknown-linux-gnu.tar.gz", "agnix"),
    Target(
        "x86_64-unknown-linux-musl",
        "agnix-x86_64-unknown-linux-musl.tar.gz",
        "agnix",
        # musl builds are static, so the only floor is musl's own ABI version.
        "musllinux_1_2_x86_64",
    ),
    Target(
        "aarch64-apple-darwin",
        "agnix-aarch64-apple-darwin.tar.gz",
        "agnix",
        # Apple silicon starts at macOS 11.
        "macosx_11_0_arm64",
    ),
    Target(
        "x86_64-pc-windows-msvc",
        "agnix-x86_64-pc-windows-msvc.zip",
        "agnix.exe",
        "win_amd64",
    ),
)

GNU_ARCH_TAGS = {
    "x86_64-unknown-linux-gnu": "x86_64",
    "aarch64-unknown-linux-gnu": "aarch64",
}

GLIBC_REF = re.compile(rb"GLIBC_2\.(\d{1,3})\b")


def glibc_platform_tags(binary: Path, arch: str) -> list[str]:
    """Platform tags for a glibc-linked binary, from its own version refs.

    The floor is read off the binary instead of hardcoded so a toolchain or
    cross-image bump cannot silently ship a wheel that claims wider
    compatibility than the binary has. `manylinux_2_17` also gets the legacy
    `manylinux2014` alias, which older pip versions are the only ones to match.
    """
    minors = {int(match.group(1)) for match in GLIBC_REF.finditer(binary.read_bytes())}
    if not minors:
        raise ValueError(
            f"{binary} has no GLIBC_2.x version references; "
            "it is not a glibc build and needs an explicit platform tag"
        )

    # PEP 600: manylinux_${GLIBCMAJOR}_${GLIBCMINOR} means "needs at most this
    # glibc", so the tag has to be the highest symbol version referenced.
    minor = max(minors)
    tags = [f"manylinux_2_{minor}_{arch}"]
    if minor <= 17:
        tags.append(f"manylinux2014_{arch}")
    return tags


def read_metadata(pyproject: Path) -> dict:
    with pyproject.open("rb") as handle:
        return tomllib.load(handle)["project"]


def urlsafe_b64_nopad(data: bytes) -> str:
    return base64.urlsafe_b64encode(data).rstrip(b"=").decode("ascii")


def record_hash(data: bytes) -> str:
    return f"sha256={urlsafe_b64_nopad(hashlib.sha256(data).digest())}"


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def parse_sha256_sidecar(contents: str, expected_name: str) -> str:
    """Read the hash for `expected_name` out of a shasum-style sidecar.

    Mirrors npm/install.js so both wrappers reject a tampered archive the same
    way.
    """
    for line in contents.splitlines():
        parts = line.strip().split(maxsplit=1)
        if len(parts) != 2:
            continue
        raw_hash, raw_name = parts
        name = raw_name.lstrip("*").replace("\\", "/").rsplit("/", 1)[-1]
        if name != expected_name:
            continue
        expected = raw_hash.lower()
        if not re.fullmatch(r"[0-9a-f]{64}", expected):
            raise ValueError(f"Invalid checksum entry for {expected_name}")
        return expected
    raise ValueError(f"Checksum file has no entry for {expected_name}")


def verify_archive(archive: Path) -> None:
    """Verify an archive against its `.sha256` sidecar when one is present."""
    sidecar = archive.with_name(archive.name + ".sha256")
    if not sidecar.is_file():
        print(f"  no .sha256 sidecar for {archive.name}, skipping verification")
        return

    expected = parse_sha256_sidecar(sidecar.read_text(encoding="utf-8"), archive.name)
    actual = sha256_file(archive)
    if actual != expected:
        raise ValueError(
            f"Checksum mismatch for {archive.name}: expected {expected}, got {actual}"
        )
    print(f"  checksum verified: {archive.name}")


def extract_binary(archive: Path, member: str, dest_dir: Path) -> Path:
    """Extract one named binary out of a release archive."""
    dest = dest_dir / member

    if archive.name.endswith(".zip"):
        with zipfile.ZipFile(archive) as zf:
            with zf.open(member) as src, dest.open("wb") as out:
                shutil.copyfileobj(src, out)
    else:
        with tarfile.open(archive, "r:gz") as tf:
            extracted = tf.extractfile(member)
            if extracted is None:
                raise ValueError(f"{archive.name} has no regular file entry {member}")
            with extracted as src, dest.open("wb") as out:
                shutil.copyfileobj(src, out)

    dest.chmod(0o755)
    return dest


def metadata_document(meta: dict, long_description: str) -> str:
    """RFC 822 METADATA per the core metadata spec (version 2.4)."""
    lines = [
        "Metadata-Version: 2.4",
        f"Name: {meta['name']}",
        f"Version: {meta['version']}",
        f"Summary: {meta['description']}",
        f"Requires-Python: {meta['requires-python']}",
        f"License-Expression: {meta['license']}",
    ]
    for author in meta.get("authors", []):
        lines.append(f"Author: {author['name']}")
        if author.get("email"):
            lines.append(f"Author-email: {author['name']} <{author['email']}>")
    for license_file in LICENSE_FILES:
        lines.append(f"License-File: {license_file}")
    if meta.get("keywords"):
        lines.append(f"Keywords: {','.join(meta['keywords'])}")
    for classifier in meta.get("classifiers", []):
        lines.append(f"Classifier: {classifier}")
    for label, url in meta.get("urls", {}).items():
        lines.append(f"Project-URL: {label}, {url}")
    lines.append("Description-Content-Type: text/markdown")
    lines.append("")
    lines.append(long_description)
    return "\n".join(lines)


def wheel_document(tags: list[str]) -> str:
    lines = [
        "Wheel-Version: 1.0",
        f"Generator: {GENERATOR}",
        # A binary in platlib, so not purelib.
        "Root-Is-Purelib: false",
    ]
    lines += [f"Tag: {PYTHON_TAG}-{ABI_TAG}-{tag}" for tag in tags]
    lines.append("")
    return "\n".join(lines)


def build_wheel(
    meta: dict,
    target: Target,
    binary: Path,
    tags: list[str],
    out_dir: Path,
) -> Path:
    name = meta["name"]
    version = meta["version"]
    dist_info = f"{name}-{version}.dist-info"
    data_scripts = f"{name}-{version}.data/scripts"
    tag_segment = ".".join(tags)
    wheel_path = out_dir / f"{name}-{version}-{PYTHON_TAG}-{ABI_TAG}-{tag_segment}.whl"

    # (archive path, bytes, executable) - collected first so RECORD covers
    # exactly what is written.
    entries: list[tuple[str, bytes, bool]] = []
    for source in sorted((PYPI_DIR / name).glob("*.py")):
        entries.append((f"{name}/{source.name}", source.read_bytes(), False))

    # The binary goes in the data scripts directory, not inside the package:
    # installers only carry the exec bit over for members under .data/scripts,
    # and this is what puts `agnix` itself on PATH.
    entries.append((f"{data_scripts}/{target.binary}", binary.read_bytes(), True))

    long_description = (PYPI_DIR / "README.md").read_text(encoding="utf-8")
    entries.append(
        (
            f"{dist_info}/METADATA",
            metadata_document(meta, long_description).encode("utf-8"),
            False,
        )
    )
    entries.append((f"{dist_info}/WHEEL", wheel_document(tags).encode("utf-8"), False))
    for license_file in LICENSE_FILES:
        entries.append(
            (
                f"{dist_info}/licenses/{license_file}",
                (REPO_ROOT / license_file).read_bytes(),
                False,
            )
        )

    record = io.StringIO(newline="")
    writer = csv.writer(record, lineterminator="\n")
    for arcname, data, _ in entries:
        writer.writerow([arcname, record_hash(data), len(data)])
    writer.writerow([f"{dist_info}/RECORD", "", ""])
    entries.append((f"{dist_info}/RECORD", record.getvalue().encode("utf-8"), False))

    out_dir.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(wheel_path, "w", zipfile.ZIP_DEFLATED) as zf:
        for arcname, data, executable in entries:
            info = zipfile.ZipInfo(arcname, date_time=ZIP_DATE_TIME)
            # create_system 3 (Unix) plus a full st_mode, S_IFREG included:
            # pip only carries the exec bit over for entries whose mode passes
            # S_ISREG, so permission bits alone leave the binary unrunnable.
            info.create_system = 3
            mode = stat.S_IFREG | (0o755 if executable else 0o644)
            info.external_attr = mode << 16
            info.compress_type = zipfile.ZIP_DEFLATED
            zf.writestr(info, data)

    return wheel_path


def platform_tags_for(target: Target, binary: Path) -> list[str]:
    if target.platform_tag:
        return [target.platform_tag]
    return glibc_platform_tags(binary, GNU_ARCH_TAGS[target.triple])


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--artifacts",
        type=Path,
        required=True,
        help="directory holding the release archives and their .sha256 sidecars",
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=PYPI_DIR / "dist",
        help="directory to write wheels into (default: pypi/dist)",
    )
    parser.add_argument(
        "--version",
        help="fail unless pypi/pyproject.toml declares this version",
    )
    parser.add_argument(
        "--allow-missing",
        action="store_true",
        help="skip targets whose archive is absent instead of failing",
    )
    args = parser.parse_args(argv)

    meta = read_metadata(PYPI_DIR / "pyproject.toml")
    if args.version and args.version != meta["version"]:
        print(
            f"Error: pypi/pyproject.toml is at {meta['version']}, expected {args.version}. "
            "Run scripts/sync-versions.sh.",
            file=sys.stderr,
        )
        return 1

    built: list[Path] = []
    with tempfile.TemporaryDirectory() as tmp:
        for target in TARGETS:
            archive = args.artifacts / target.archive
            print(f"{target.triple}:")
            if not archive.is_file():
                if args.allow_missing:
                    print(f"  {target.archive} missing, skipped")
                    continue
                print(f"  Error: {archive} not found", file=sys.stderr)
                return 1

            verify_archive(archive)
            work = Path(tmp) / target.triple
            work.mkdir()
            binary = extract_binary(archive, target.binary, work)
            tags = platform_tags_for(target, binary)
            wheel = build_wheel(meta, target, binary, tags, args.out)
            built.append(wheel)
            print(f"  {wheel.name}")

    if not built:
        print("Error: no wheels built", file=sys.stderr)
        return 1

    print(f"\n{len(built)} wheel(s) in {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
