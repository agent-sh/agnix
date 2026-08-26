---
title: Installation
description: "Install agnix via npm, Homebrew, pip, Cargo, or download pre-built binaries."
---

# Installation

## npm (recommended)

Works on all platforms. Includes pre-built binaries.

```bash
npm install -g agnix
```

Or run without installing:

```bash
npx agnix .
```

## Homebrew (macOS / Linux)

```bash
brew tap agent-sh/agnix && brew install agnix
```

## pip / uv

```bash
pip install agnix
```

Or run without installing:

```bash
uvx agnix .
```

The wheels bundle a pre-built binary, so no Rust toolchain is needed and nothing
is downloaded after install. Wheels are published for Linux x86_64 (glibc and
musl), Linux aarch64, macOS Apple silicon, and Windows x86_64; on any other
platform, use `cargo install agnix-cli`.

`python -m agnix` runs the same binary, and a small Python API is available:

```python
import agnix

report = agnix.lint(".", tool="ClaudeCode")
print(agnix.version(), report["summary"])
```

## Cargo (Rust toolchain)

```bash
cargo install agnix-cli
```

## Pre-built binaries

Download from [GitHub Releases](https://github.com/agent-sh/agnix/releases) for your platform.

Two Linux x86_64 archives are published:

| Archive | Use it when |
|---------|-------------|
| `agnix-x86_64-unknown-linux-gnu.tar.gz` | Default. Releases are gated to require no more than glibc 2.31; current builds need only 2.18. |
| `agnix-x86_64-unknown-linux-musl.tar.gz` | Statically linked, no glibc dependency. Use it on musl distros such as Alpine, or on any host where the gnu build reports a missing `GLIBC_...` version. |

The npm installer and the GitHub Action pick the gnu build and switch to the
static musl build automatically if it cannot run on the host.

## Verify installation

```bash
agnix --version
```

## Editor extensions

agnix ships editor integrations powered by the `agnix-lsp` server:

| Editor | Install |
|--------|---------|
| VS Code | [Marketplace](https://marketplace.visualstudio.com/items?itemName=avifenesh.agnix) |
| JetBrains | [Plugin](https://plugins.jetbrains.com/plugin/30087-agnix) |
| Neovim | [Plugin](https://github.com/agent-sh/agnix.nvim) |
| Zed | [Extension](https://zed.dev/extensions?query=agnix) |

See [Editor Integration](./editor-integration.md) for setup details.
