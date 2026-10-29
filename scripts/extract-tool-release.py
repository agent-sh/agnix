#!/usr/bin/env python3
"""Extract release versions from canonical release and installer sources."""
import argparse
from html.parser import HTMLParser
import re
import json
import sys
import xml.etree.ElementTree as ET
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
    if kind == 'antigravity-manifest':
        manifest = json.loads(content)
        version = manifest.get('version') if isinstance(manifest, dict) else None
        if not isinstance(version, str) or not re.fullmatch(r'\d+\.\d+\.\d+', version):
            raise ValueError('No stable Antigravity version in installer manifest')
        return version
    elif kind == 'grok-build-rss':
        channel = ET.fromstring(content).find('channel')
        if channel is None or channel.findtext('title') != 'Grok Build Changelog':
            raise ValueError('Not the Grok Build release feed')
        versions = set()
        for item in channel.findall('item'):
            match = re.fullmatch(r'Grok Build (\d+)\.(\d+)\.(\d+)', item.findtext('title', ''))
            link = urlsplit(item.findtext('link', ''))
            if match and link.hostname == 'x.ai' and link.path == '/changelog/build':
                version = tuple(map(int, match.groups()))
                if link.fragment.startswith('v' + '.'.join(map(str, version)) + '-'):
                    versions.add(version)
        prefix = ''
    elif kind == 'kiro-cli':
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
    parser.add_argument('kind', choices=['kiro-cli', 'devin-desktop', 'antigravity-manifest', 'grok-build-rss'])
    args = parser.parse_args()
    try:
        print(extract(args.kind, sys.stdin.read()))
    except (ValueError, ET.ParseError) as error:
        parser.exit(1, str(error) + '\n')
