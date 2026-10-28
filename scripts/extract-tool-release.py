#!/usr/bin/env python3
"""Extract release versions from canonical Kiro and Devin changelog content."""
import argparse
from html.parser import HTMLParser
import re
import sys
from urllib.parse import urlsplit


class KiroReleases(HTMLParser):
    def __init__(self):
        super().__init__()
        self.versions = set()

    def handle_starttag(self, tag, attrs):
        if tag != 'a':
            return
        href = dict(attrs).get('href', '')
        url = urlsplit(href)
        if url.netloc and url.netloc != 'kiro.dev':
            return
        release = re.fullmatch(r'/changelog/cli/(\d+)-(\d+)(?:-(\d+))?/?', url.path)
        if not release:
            return
        version = tuple(int(part or '0') for part in release.groups())
        self.versions.add(version)
        patch = re.fullmatch(r'patch-(\d+)-(\d+)-(\d+)', url.fragment)
        if patch:
            patched = tuple(map(int, patch.groups()))
            if patched[:2] == version[:2]:
                self.versions.add(patched)


def extract(kind, content):
    if kind == 'kiro-cli':
        parser = KiroReleases()
        parser.feed(content)
        versions = parser.versions
        prefix = ''
    elif kind == 'devin-desktop':
        # Ignore marketing waves, example versions and numbers in prose.
        matches = re.findall(r'^\*\*v(\d+)\.(\d+)\.(\d+)\*\*\s*$', content, re.MULTILINE)
        versions = {tuple(map(int, match)) for match in matches}
        prefix = 'v'
    else:
        raise ValueError('Unknown release extractor: ' + kind)
    if not versions:
        raise ValueError('No release versions found by ' + kind)
    return prefix + '.'.join(map(str, max(versions)))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('kind', choices=['kiro-cli', 'devin-desktop'])
    args = parser.parse_args()
    try:
        print(extract(args.kind, sys.stdin.read()))
    except ValueError as error:
        parser.exit(1, str(error) + '\n')
