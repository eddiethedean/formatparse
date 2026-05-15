"""Package version resolution."""

from __future__ import annotations

import re
from importlib.metadata import PackageNotFoundError, version as _package_version
from pathlib import Path

try:
    __version__ = _package_version("formatparse")
except PackageNotFoundError:
    _cargo = Path(__file__).resolve().parent.parent / "Cargo.toml"
    try:
        _m = re.search(r'version\s*=\s*"([^"]+)"', _cargo.read_text(encoding="utf-8"))
        __version__ = _m.group(1) if _m else "0.0.0"
    except OSError:
        __version__ = "0.0.0"
