#!/bin/bash
# Pre-publish security validation script
# Run this before publishing to ensure all security checks pass

set -e

echo "🔒 Running pre-publish security checks..."

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "❌ Error: Cargo.toml not found. Run this script from the repository root."
    exit 1
fi

# Install cargo-audit if not available
if ! command -v cargo-audit &> /dev/null; then
    echo "📦 Installing cargo-audit..."
    cargo install cargo-audit
fi

# Install cargo-deny if not available
if ! command -v cargo-deny &> /dev/null; then
    echo "📦 Installing cargo-deny..."
    cargo install cargo-deny
fi

# Run cargo audit
echo "🔍 Running cargo audit..."
cargo audit || {
    echo "❌ Cargo audit found vulnerabilities. Fix them before publishing."
    exit 1
}

# Run cargo deny
echo "🔍 Running cargo deny..."
cargo deny check || {
    echo "❌ Cargo deny found issues. Fix them before publishing."
    exit 1
}

# Check if pip-audit is available
if command -v pip-audit &> /dev/null || command -v pip3 &> /dev/null; then
    echo "🔍 Running pip-audit..."
    if command -v pip-audit &> /dev/null; then
        pip-audit --desc || {
            echo "⚠️  pip-audit found vulnerabilities. Review before publishing."
        }
    else
        pip3 install pip-audit
        pip-audit --desc || {
            echo "⚠️  pip-audit found vulnerabilities. Review before publishing."
        }
    fi
else
    echo "⚠️  pip-audit not available, skipping Python dependency check"
fi

echo "✅ All security checks passed!"

