# Configuration file for the Sphinx documentation builder.
#
# For the full list of built-in configuration values, see the documentation:
# https://www.sphinx-doc.org/en/master/usage/configuration.html

import os
import sys
from pathlib import Path

# Add the parent directory to the path so we can import formatparse
sys.path.insert(0, str(Path(__file__).parent.parent))

# -- Project information -----------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#project-information

project = 'formatparse'
copyright = '2024, Odos Matthews'
author = 'Odos Matthews'

# Get version from Cargo.toml
try:
    import re
    cargo_toml = Path(__file__).parent.parent / 'Cargo.toml'
    with open(cargo_toml) as f:
        content = f.read()
        match = re.search(r'version\s*=\s*"([^"]+)"', content)
        if match:
            release = match.group(1)
        else:
            release = '0.4.2'
except Exception:
    release = '0.4.2'

version = release

# -- General configuration ---------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#general-configuration

extensions = [
    'sphinx.ext.autodoc',
    'sphinx.ext.autosummary',
    'sphinx.ext.viewcode',
    'sphinx.ext.intersphinx',
    'sphinx.ext.napoleon',
    'sphinx.ext.doctest',
]

templates_path = ['_templates']
exclude_patterns = ['_build', 'Thumbs.db', '.DS_Store']

# -- Options for HTML output -------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#options-for-html-output

html_theme = 'sphinx_rtd_theme'
html_static_path = ['_static']

# -- Extension configuration -------------------------------------------------

# Autodoc configuration
autodoc_default_options = {
    'members': True,
    'member-order': 'bysource',
    'special-members': '__init__',
    'undoc-members': False,
    'exclude-members': '__weakref__'
}

# Intersphinx configuration
intersphinx_mapping = {
    'python': ('https://docs.python.org/3', None),
}

# Mock imports for autodoc (since we can't build the Rust extension on ReadTheDocs)
autodoc_mock_imports = ['_formatparse']

# Doctest configuration
# Note: Doctests will be skipped on ReadTheDocs if the package isn't available
doctest_global_setup = """
try:
    from formatparse import parse, search, findall, compile, with_pattern
    from formatparse import ParseResult, FormatParser, BidirectionalPattern, BidirectionalResult
    from formatparse import FixedTzOffset, RepeatedNameError
except ImportError:
    # Skip doctests if package isn't available
    pass
"""

doctest_test_doctest_blocks = 'default'

# Napoleon configuration (for Google/NumPy style compatibility)
napoleon_google_docstring = True
napoleon_numpy_docstring = True
napoleon_include_init_with_doc = False
napoleon_include_private_with_doc = False
napoleon_include_special_with_doc = True

