"""Run the contracted import smoke on a real Python 3.7 interpreter.

Verifies that the installed ipckit package behaves correctly:
  native_py37 -> the compiled ipckit.ipckit extension is importable
  lite_py37   -> the package imports but the compiled extension is absent
"""

from __future__ import annotations

import argparse
import importlib
import sys


def _verify_py37() -> None:
    if sys.version_info[:2] != (3, 7):
        raise RuntimeError(
            "smoke requires Python 3.7, got {}.{}".format(
                sys.version_info[0], sys.version_info[1],
            )
        )


def main(argv=None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--profile", choices=("native_py37", "lite_py37"), required=True)
    args = parser.parse_args(argv)

    try:
        _verify_py37()
        import ipckit

        version = getattr(ipckit, "__version__", None)
        if not version:
            raise RuntimeError("ipckit.__version__ is missing")

        if args.profile == "native_py37":
            importlib.import_module("ipckit.ipckit")
            if not getattr(ipckit, "_native_available", False):
                raise RuntimeError("native_py37 must report _native_available=True")
        else:
            if getattr(ipckit, "_native_available", True):
                raise RuntimeError("lite_py37 must report _native_available=False")
            try:
                importlib.import_module("ipckit.ipckit")
            except ImportError:
                pass
            else:
                raise RuntimeError("lite_py37 must not contain ipckit.ipckit")
    except Exception as exc:  # noqa: BLE001
        sys.stderr.write("python37-runtime-smoke: {}\n".format(exc))
        return 1

    sys.stdout.write("python37-runtime-smoke: {} OK\n".format(args.profile))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
