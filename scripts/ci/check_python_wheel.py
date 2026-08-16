"""Validate ipckit wheel tags, metadata, and native-extension contents.

Self-contained wheel contract with three profiles:
  abi3        -> cp38-abi3 wheel with the compiled ipckit.ipckit extension
  lite_py37   -> py3-none-any stubs-only wheel with no compiled extension
  native_py37 -> cp37-cp37m wheel with the compiled ipckit.ipckit extension
"""

from __future__ import annotations

import argparse
from email.parser import Parser
from pathlib import Path
import sys
import zipfile

DISTRIBUTION = "ipckit"
PROFILE_TAGS = {
    "abi3": ("cp38", "abi3"),
    "lite_py37": ("py3", "none"),
    "native_py37": ("cp37", "cp37m"),
}
PLATFORM_ALLOWED = {
    "linux-x86_64": ("manylinux", None),
    "windows-x86_64": ("win_amd64", None),
    "macos-universal2": ("macosx", "universal2"),
    "any": ("any", None),
}


def _parse_filename(path: Path):
    name = path.name
    if name.endswith(".whl"):
        name = name[: -len(".whl")]
    parts = name.split("-")
    if len(parts) < 5:
        return None
    return parts[-3], parts[-2], parts[-1]


def _single_member(archive, suffix):
    matches = [n for n in archive.namelist() if n.endswith(suffix)]
    if len(matches) != 1:
        raise ValueError("expected exactly one *{}, found {}".format(suffix, len(matches)))
    return archive.read(matches[0]).decode("utf-8")


def _has_extension(names):
    for name in names:
        if "ipckit/ipckit." in name and name.endswith((".so", ".pyd")):
            return True
    return False


def validate(path: Path, profile: str, platform: str):
    errors = []
    if profile not in PROFILE_TAGS:
        return ["unknown profile {!r}".format(profile)]
    if platform not in PLATFORM_ALLOWED:
        return ["unknown platform {!r}".format(platform)]

    expected_py, expected_abi = PROFILE_TAGS[profile]
    expects_extension = profile != "lite_py37"

    parsed = _parse_filename(path)
    if parsed is None:
        return ["cannot parse wheel filename {!r}".format(path.name)]
    py_tag, abi_tag, plat_tag = parsed

    try:
        with zipfile.ZipFile(str(path)) as archive:
            names = archive.namelist()
            metadata = Parser().parsestr(_single_member(archive, ".dist-info/METADATA"))
            wheel_meta = Parser().parsestr(_single_member(archive, ".dist-info/WHEEL"))
    except (OSError, ValueError, zipfile.BadZipFile, UnicodeDecodeError) as exc:
        return ["cannot inspect wheel: {}".format(exc)]

    actual_name = str(metadata.get("Name", "")).lower().replace("_", "-")
    if actual_name != DISTRIBUTION:
        errors.append("Name is {!r}, expected {!r}".format(actual_name, DISTRIBUTION))

    requires_python = str(metadata.get("Requires-Python", ""))
    if not requires_python.startswith(">=3.7"):
        errors.append("Requires-Python is {!r}, expected >=3.7".format(requires_python))

    if py_tag != expected_py:
        errors.append("python tag {!r}, expected {!r}".format(py_tag, expected_py))
    if abi_tag != expected_abi:
        errors.append("abi tag {!r}, expected {!r}".format(abi_tag, expected_abi))

    plat_prefix, plat_sub = PLATFORM_ALLOWED[platform]
    if not plat_tag.startswith(plat_prefix) or (plat_sub and plat_sub not in plat_tag):
        errors.append("platform tag {!r} not allowed for {}".format(plat_tag, platform))

    has_ext = _has_extension(names)
    if has_ext != expects_extension:
        errors.append(
            "compiled extension presence is {}, expected {}".format(has_ext, expects_extension)
        )

    root_is_pure = str(wheel_meta.get("Root-Is-Purelib", "")).lower()
    expected_pure = "true" if profile == "lite_py37" else "false"
    if root_is_pure != expected_pure:
        errors.append("Root-Is-Purelib is {!r}, expected {!r}".format(root_is_pure, expected_pure))

    return errors


def main(argv=None):
    parser = argparse.ArgumentParser()
    parser.add_argument("--profile", required=True)
    parser.add_argument("--platform", required=True)
    parser.add_argument("wheels", nargs="+")
    args = parser.parse_args(argv)

    failed = False
    for raw in args.wheels:
        raw_path = Path(raw)
        parent = raw_path.parent
        matches = list(parent.glob(raw_path.name)) if str(parent) != "." else []
        paths = matches or [raw_path]
        for path in paths:
            errors = validate(path, args.profile, args.platform)
            if errors:
                failed = True
                for error in errors:
                    sys.stderr.write("{}: {}\n".format(path, error))
            else:
                sys.stdout.write("{}: {} wheel contract OK\n".format(path, args.profile))
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
