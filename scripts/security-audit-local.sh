#!/bin/bash
# Local Security Audit Script
# Run this to check security issues locally before pushing

set -e

echo "=== Security Audit ==="
echo ""

# Check if cargo-audit is installed
if ! command -v cargo-audit &> /dev/null; then
    echo "Installing cargo-audit..."
    cargo install cargo-audit --version 0.22.0
fi

# Check if cargo-deny is installed
if ! command -v cargo-deny &> /dev/null; then
    echo "Installing cargo-deny..."
    cargo install cargo-deny --version 0.14.3
fi

# Check if pip-audit is installed
if ! command -v pip-audit &> /dev/null; then
    echo "Installing pip-audit..."
    python3 -m pip install --upgrade pip
    pip install pip-audit
fi

echo ""
echo "=== Running Cargo Audit ==="
cargo audit

echo ""
echo "=== Running Cargo Deny ==="
cargo deny check --config cargo-deny.toml

echo ""
echo "=== Running Pip Audit ==="
pip-audit --desc || echo "Note: pip-audit may report vulnerabilities but won't fail"

echo ""
echo "=== Security Audit Complete ==="
