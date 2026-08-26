"""agnix - Python API.

Programmatic access to the agnix linter. Mirrors the Node API in npm/index.js.
"""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
import sysconfig
from pathlib import Path
from typing import Any, Mapping, Sequence

__version__ = "0.51.0"

__all__ = ["AgnixError", "binary_path", "lint", "run", "version", "__version__"]


class AgnixError(RuntimeError):
    """agnix ran but did not produce the output the caller asked for."""


_BINARY_NAME = "agnix" + (sysconfig.get_config_var("EXE") or "")


def binary_path() -> Path:
    """Absolute path of the agnix binary installed with this package.

    The wheels ship the binary in the `.data/scripts` directory, which is the
    only place a wheel can put an executable and keep its exec bit, so it lands
    in the environment's `bin`/`Scripts` next to the interpreter rather than
    inside the package.
    """
    candidates = [Path(sysconfig.get_path("scripts")) / _BINARY_NAME]

    # pip install --user puts scripts under the user scheme instead.
    if sys.version_info >= (3, 10):
        user_scheme = sysconfig.get_preferred_scheme("user")
    elif sys.platform == "win32":
        user_scheme = "nt_user"
    elif sys.platform == "darwin" and sys._framework:
        user_scheme = "osx_framework_user"
    else:
        user_scheme = "posix_user"
    candidates.append(Path(sysconfig.get_path("scripts", scheme=user_scheme)) / _BINARY_NAME)

    for candidate in candidates:
        if candidate.is_file():
            return candidate

    # Last resort: whatever agnix is on PATH, which covers layouts this
    # package cannot predict (relocated venvs, zipapps, distro repackaging).
    on_path = shutil.which(_BINARY_NAME)
    if on_path:
        return Path(on_path)

    raise FileNotFoundError(
        f"Could not find the {_BINARY_NAME} binary. Try reinstalling:\n"
        "  pip install --force-reinstall agnix"
    )


def run(
    args: Sequence[str] = (),
    **kwargs: Any,
) -> subprocess.CompletedProcess:
    """Run agnix and return the completed process.

    Extra keyword arguments are passed through to `subprocess.run`. Output is
    captured as text unless the caller overrides `capture_output`/`text`.
    """
    kwargs.setdefault("capture_output", True)
    kwargs.setdefault("text", True)
    return subprocess.run([str(binary_path()), *args], **kwargs)  # noqa: S603


def lint(
    target: str,
    tool: str | None = None,
    fmt: str = "json",
) -> Mapping[str, Any]:
    """Lint a file or directory and return the parsed diagnostics.

    `tool` selects the agent target, using the same values as `--target`:
    `generic`, `claude-code`, `cursor`, `codex`, `kiro`. A format other than
    `json` is returned unparsed under a "raw" key.

    Raises `AgnixError` if agnix produced no parseable JSON, which is what a
    rejected argument looks like. Returning an empty report there would read as
    a clean lint.
    """
    args = ["--format", fmt]
    if tool:
        args += ["--target", tool]
    args.append(target)

    result = run(args)

    if fmt != "json":
        return {"raw": result.stdout}

    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise AgnixError(
            f"agnix exited {result.returncode} without JSON output"
            f"{': ' + result.stderr.strip() if result.stderr else ''}"
        ) from error


def version() -> str:
    """Version string reported by the binary itself."""
    return run(["--version"]).stdout.strip()
