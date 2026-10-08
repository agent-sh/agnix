# agnix

Linter for AI agent configurations. Validates SKILL.md, CLAUDE.md, hooks, MCP, and more.

**455 rules** | **Real-time validation** | **Auto-fix** | **Multi-tool support**

## Installation

```bash
pip install agnix
```

Or run it without installing:

```bash
uvx agnix .
```

The wheels bundle a prebuilt binary, so there is no Rust toolchain to install
and nothing is downloaded after `pip install`. Supported platforms:

| Platform | Wheel |
|----------|-------|
| Linux x86_64 (glibc) | `manylinux_*_x86_64` |
| Linux x86_64 (musl, e.g. Alpine) | `musllinux_1_2_x86_64` |
| Linux aarch64 (glibc) | `manylinux_*_aarch64` |
| macOS Apple silicon | `macosx_11_0_arm64` |
| Windows x86_64 | `win_amd64` |

On any other platform, install from source with `cargo install agnix-cli`.

## Usage

### Command line

```bash
# Lint current directory
agnix .

# Lint specific file
agnix CLAUDE.md

# Auto-fix issues
agnix . --fix

# JSON output
agnix . --format json

# Target a specific tool
agnix . --target claude-code
```

`python -m agnix` works the same way as the `agnix` script.

### Python API

```python
import agnix

# Version reported by the bundled binary
print(agnix.version())

# Lint a path and get parsed diagnostics
report = agnix.lint(".", tool="claude-code")
print(report["summary"])

# Anything else: pass arguments straight through
result = agnix.run(["--fix", "CLAUDE.md"])
print(result.returncode, result.stdout)
```

## Links

- [Documentation](https://agent-sh.github.io/agnix/)
- [Repository](https://github.com/agent-sh/agnix)
- [Issues](https://github.com/agent-sh/agnix/issues)
- [Changelog](https://github.com/agent-sh/agnix/blob/main/CHANGELOG.md)

## License

MIT OR Apache-2.0
