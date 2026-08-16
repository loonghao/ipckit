"""Build the py37-lite pure-Python (py3-none-any) wheel without maturin.

Maya 2022 / Blender 2.83 embed CPython 3.7, which cannot load the
cp38-abi3 wheels produced by the regular build. This script assembles a
stubs-only wheel from python/ipckit so those hosts can still install
ipckit for type-checking, mirroring the dcc-mcp-core py37-lite design.
"""

from __future__ import annotations

from email.message import EmailMessage
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
PYTHON_ROOT = ROOT / "python"
PACKAGE_DIR = PYTHON_ROOT / "ipckit"
DIST = ROOT / "dist"


def _ensure_wheel() -> None:
    try:
        import wheel.wheelfile  # noqa: F401
    except ImportError:
        subprocess.check_call([sys.executable, "-m", "pip", "install", "wheel"])


def _read_version() -> str:
    text = (ROOT / "pyproject.toml").read_text(encoding="utf-8")
    match = re.search(r'^version = "([^"]+)"', text, re.MULTILINE)
    if not match:
        raise RuntimeError("could not read project version from pyproject.toml")
    return match.group(1)


def main() -> int:
    """Assemble a pure-Python wheel from python/ipckit."""
    _ensure_wheel()
    from wheel.wheelfile import WheelFile

    version = _read_version()
    dist_name = "ipckit"
    dist_info = "{}-{}.dist-info".format(dist_name, version)
    wheel_name = "{}-{}-py3-none-any.whl".format(dist_name, version)
    DIST.mkdir(parents=True, exist_ok=True)
    wheel_path = DIST / wheel_name

    metadata = EmailMessage()
    metadata["Metadata-Version"] = "2.1"
    metadata["Name"] = "ipckit"
    metadata["Version"] = version
    metadata["Requires-Python"] = ">=3.7"
    metadata["License"] = "MIT OR Apache-2.0"
    metadata["Summary"] = "A cross-platform IPC library powered by Rust"

    wheel_meta = (
        "Wheel-Version: 1.0\n"
        "Generator: build_py37_pure_wheel\n"
        "Root-Is-Purelib: true\n"
        "Tag: py3-none-any\n"
    )

    with WheelFile(str(wheel_path), "w") as wf:
        for file_path in sorted(PACKAGE_DIR.rglob("*")):
            if not file_path.is_file():
                continue
            if "__pycache__" in file_path.parts:
                continue
            arcname = file_path.relative_to(PYTHON_ROOT).as_posix()
            wf.write(str(file_path), arcname)

        wf.writestr("{}/METADATA".format(dist_info), metadata.as_string())
        wf.writestr("{}/WHEEL".format(dist_info), wheel_meta)

    print("Built wheel: {}".format(wheel_path))
    return 0


if __name__ == "__main__":
    sys.exit(main())
