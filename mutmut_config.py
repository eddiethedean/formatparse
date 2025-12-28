"""Configuration for mutmut mutation testing

Run mutation tests with: mutmut run
View mutations with: mutmut show
"""

# Path to the package to mutate
paths_to_mutate = [
    "formatparse/",
]

# Command to run tests
# Note: Do not define tests_dir - mutmut will infer it from test_command
test_command = "python -m pytest tests -x"

# Timeout for test runs (in seconds)
test_timeout = 300

# Exclude certain files or patterns from mutation
exclude = [
    # Exclude test files
    "*/tests/*",
    "*/test_*.py",
    # Exclude __init__ files (usually just imports)
    "*/__init__.py",
    # Exclude setup/config files
    "*/setup.py",
    "*/conftest.py",
]

# Pre-mutation hook (optional)
# def pre_mutation(context):
#     pass

# Post-mutation hook (optional)
# def post_mutation(context):
#     pass

