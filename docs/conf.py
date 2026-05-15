# Configuration file for the Sphinx documentation builder.
#
# For the full list of built-in configuration values, see the documentation:
# https://www.sphinx-doc.org/en/master/usage/configuration.html

from __future__ import annotations

import re
import sys
import types
from pathlib import Path

# Add the parent directory to the path so we can import formatparse
_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(_ROOT))


def _stub_type(name: str, bases: tuple[type, ...] = (object,)) -> type:
    def __init__(self, *args: object, **kwargs: object) -> None:  # noqa: D401
        """Accept construction like the real Rust-backed type (docs build stub)."""

    return type(
        name,
        bases,
        {
            "__module__": "_formatparse",
            "__doc__": f"Stub for {name} (extension not built).",
            "__init__": __init__,
        },
    )


def _stub_fn(name: str):
    def _fn(*_a, **_k):
        raise RuntimeError("_formatparse extension is not built (documentation stub).")

    _fn.__name__ = _fn.__qualname__ = name
    _fn.__module__ = "_formatparse"
    _fn.__doc__ = f"Stub for {name} (extension not built)."
    return _fn


# Mock _formatparse if the Rust extension is not built (e.g. Read the Docs without maturin).
# Must run before formatparse/__init__.py imports it. Use real types/callables (not
# MagicMock) so sphinx-autodoc-typehints can inspect annotations safely.
_FORMATPARSE_EXPORTS = (
    "parse",
    "parse_batch",
    "search",
    "findall",
    "findall_iter",
    "compile",
    "ParseResult",
    "FormatParser",
    "FindallIter",
    "FixedTzOffset",
    "PatternParseMismatch",
    "Results",
)

try:
    import _formatparse  # noqa: F401
except ImportError:
    from datetime import tzinfo

    _mod = types.ModuleType("_formatparse")
    for _name in _FORMATPARSE_EXPORTS:
        if _name in (
            "ParseResult",
            "FormatParser",
            "FindallIter",
            "FixedTzOffset",
            "PatternParseMismatch",
            "Results",
        ):
            bases: tuple[type, ...]
            if _name == "PatternParseMismatch":
                bases = (ValueError,)
            elif _name == "FixedTzOffset":
                bases = (tzinfo,)
            else:
                bases = (object,)
            setattr(_mod, _name, _stub_type(_name, bases))
        else:
            setattr(_mod, _name, _stub_fn(_name))
    sys.modules["_formatparse"] = _mod

# -- Project information -----------------------------------------------------

project = "formatparse"
copyright = "2024-2026, Odos Matthews"
author = "Odos Matthews"

# Version: match formatparse.__init__ (Cargo.toml workspace package version in checkout)
_release = "0.0.0-unknown"
try:
    _cargo_text = (_ROOT / "Cargo.toml").read_text(encoding="utf-8")
    _m = re.search(r'version\s*=\s*"([^"]+)"', _cargo_text)
    if _m:
        _release = _m.group(1)
except OSError:
    pass
release = _release
version = release

# -- General configuration ---------------------------------------------------

extensions = [
    "sphinx.ext.autodoc",
    "sphinx.ext.autosummary",
    "sphinx.ext.viewcode",
    "sphinx.ext.intersphinx",
    "sphinx.ext.extlinks",
    "sphinx.ext.napoleon",
    "sphinx_autodoc_typehints",
    "sphinx.ext.doctest",
    "sphinx_copybutton",
    "myst_parser",
]

templates_path = ["_templates"]
exclude_patterns = ["_build", "Thumbs.db", ".DS_Store"]

source_suffix = {
    ".rst": "restructuredtext",
    ".md": "markdown",
}

# -- Options for HTML output -------------------------------------------------

html_theme = "sphinx_rtd_theme"
html_static_path = ["_static"]
html_theme_options = {
    "navigation_depth": 4,
    "collapse_navigation": False,
    "titles_only": False,
}

# -- Extension configuration -------------------------------------------------

autodoc_mock_imports = ["_formatparse"]

autodoc_default_options = {
    "members": True,
    "member-order": "bysource",
    "special-members": "__init__",
    "undoc-members": False,
    "exclude-members": "__weakref__",
}

# Type hints in descriptions (readable on RTD)
autodoc_typehints = "description"
autodoc_typehints_description_target = "documented"

intersphinx_mapping = {
    "python": ("https://docs.python.org/3", None),
}

# Release links in CHANGELOG may not exist until publish day.
linkcheck_ignore = [
    r"https://github\.com/eddiethedean/formatparse/releases/tag/v0\.8\.0$",
]

extlinks = {
    "repo": ("https://github.com/eddiethedean/formatparse/blob/main/%s", "%s"),
}

copybutton_prompt_text = r">>> |\.\.\. |\$ "
copybutton_prompt_is_regexp = True

myst_enable_extensions = ["colon_fence"]

doctest_global_setup = """
try:
    from formatparse import parse, search, findall, compile, with_pattern
    from formatparse import ParseResult, FormatParser, BidirectionalPattern, BidirectionalResult
    from formatparse import FixedTzOffset, RepeatedNameError
except ImportError:
    pass
"""

doctest_test_doctest_blocks = "default"

napoleon_google_docstring = True
napoleon_numpy_docstring = True
napoleon_include_init_with_doc = False
napoleon_include_private_with_doc = False
napoleon_include_special_with_doc = True
