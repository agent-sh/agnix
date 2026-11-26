#!/usr/bin/env python3
"""Package a built Unix agnix-lsp in the current directory. Never publish."""

import argparse
import gzip
import hashlib
import io
from pathlib import Path
import subprocess
import tarfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('target', choices=(
        'aarch64-apple-darwin', 'x86_64-apple-darwin', 'aarch64-unknown-linux-gnu',
        'x86_64-unknown-linux-gnu', 'x86_64-unknown-linux-musl',
    ))
    parser.add_argument('binary', type=Path)
    args = parser.parse_args()
    if args.target.endswith('-apple-darwin'):
        expected = {'aarch64': 'arm64', 'x86_64': 'x86_64'}[args.target.split('-')[0]]
        arch = subprocess.check_output(['lipo', '-archs', args.binary], text=True).strip()
        if arch != expected:
            parser.error(f'{args.binary}: expected {expected} Mach-O, got {arch}')

    binary = args.binary.read_bytes()
    if not binary or not args.binary.stat().st_mode & 0o111:
        parser.error(f'{args.binary}: missing executable content or mode')
    stem = f'agnix-lsp-{args.target}'
    archive = Path(f'{stem}.tar.gz')
    raw_sum = Path(f'{stem}.sha256')
    archive_sum = Path(f'{archive}.sha256')
    if any(p.exists() for p in (archive, raw_sum, archive_sum)):
        parser.error('release outputs already exist; use a fresh directory')

    # Do not embed the builder's user, directory or wall clock in release assets.
    with archive.open('xb') as out, gzip.GzipFile(filename='', fileobj=out, mode='wb', mtime=0) as gz:
        with tarfile.open(fileobj=gz, mode='w|', format=tarfile.USTAR_FORMAT) as tar:
            member = tarfile.TarInfo('agnix-lsp')
            member.size, member.mode = len(binary), 0o755
            tar.addfile(member, io.BytesIO(binary))
    for path, name, content in ((raw_sum, 'agnix-lsp', binary), (archive_sum, archive.name, archive.read_bytes())):
        with path.open('x', encoding='ascii', newline='\n') as out:
            out.write(f'{hashlib.sha256(content).hexdigest()}  {name}\n')


if __name__ == '__main__':
    main()
