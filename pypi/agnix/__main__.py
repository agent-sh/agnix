"""`python -m agnix`: hand the process over to the agnix binary.

`pip install agnix` already puts the binary itself on PATH as `agnix`; this
module is the importable path to the same thing.
"""

from __future__ import annotations

import os
import subprocess
import sys

from . import binary_path


def main() -> None:
    try:
        binary = binary_path()
    except FileNotFoundError as error:
        print(error, file=sys.stderr)
        print("Or build from source with:\n  cargo install agnix-cli", file=sys.stderr)
        raise SystemExit(1) from None

    argv = [str(binary), *sys.argv[1:]]

    # execv replaces this process so signals, exit codes, and terminal
    # behaviour are the binary's own. Windows has no execv that keeps the
    # parent's console attached to the child, so it gets a subprocess.
    if sys.platform == "win32":
        raise SystemExit(subprocess.call(argv))  # noqa: S603

    os.execv(str(binary), argv)


if __name__ == "__main__":
    main()
