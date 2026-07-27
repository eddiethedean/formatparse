#!/bin/bash
# Release script for formatparse
# Usage: ./scripts/release.sh <version>
# Example: ./scripts/release.sh 0.1.0

set -e

if [ -z "$1" ]; then
    echo "Usage: $0 <version>"
    echo "Example: $0 0.1.0"
    exit 1
fi

VERSION="$1"
TAG="v${VERSION}"

# Validate version format (basic check)
if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9]+)?$ ]]; then
    echo "Error: Invalid version format. Use semantic versioning (e.g., 0.1.0)"
    exit 1
fi

echo "🚀 Preparing release ${VERSION}..."

# Check if we're on main branch
CURRENT_BRANCH=$(git branch --show-current)
if [ "$CURRENT_BRANCH" != "main" ]; then
    echo "⚠️  Warning: You're not on the main branch (currently on ${CURRENT_BRANCH})"
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Check for uncommitted changes
if ! git diff-index --quiet HEAD --; then
    echo "❌ Error: You have uncommitted changes. Please commit or stash them first."
    exit 1
fi

# Check if tag already exists
if git rev-parse "$TAG" >/dev/null 2>&1; then
    echo "❌ Error: Tag ${TAG} already exists"
    exit 1
fi

# Update version in Cargo.toml (workspace package + formatparse-core dependency)
echo "📝 Updating version in Cargo.toml to ${VERSION}..."
python3 - "$VERSION" <<'PY'
import re
import sys
from pathlib import Path

version = sys.argv[1]
path = Path("Cargo.toml")
text = path.read_text(encoding="utf-8")
text, n1 = re.subn(
    r'(?m)^version = "[^"]+"',
    f'version = "{version}"',
    text,
    count=1,
)
text, n2 = re.subn(
    r'(formatparse-core = \{ path = "formatparse-core", version = ")[^"]+("\s*\})',
    rf"\g<1>{version}\2",
    text,
    count=1,
)
if n1 != 1 or n2 != 1:
    raise SystemExit(
        f"Failed to update Cargo.toml versions (workspace={n1}, formatparse-core={n2})"
    )
path.write_text(text, encoding="utf-8")
PY

if ! grep -q "^version = \"${VERSION}\"" Cargo.toml; then
    echo "❌ Error: Could not confirm workspace version in Cargo.toml (expected version = \"${VERSION}\")."
    exit 1
fi

if ! grep -q "formatparse-core = { path = \"formatparse-core\", version = \"${VERSION}\" }" Cargo.toml; then
    echo "❌ Error: formatparse-core dependency version in Cargo.toml does not match ${VERSION}."
    exit 1
fi

echo "🔒 Refreshing Cargo.lock workspace package versions..."
cargo metadata --format-version 1 >/dev/null

# Commit the version change
echo "💾 Committing version change..."
git add Cargo.toml Cargo.lock
git commit -m "Bump version to ${VERSION}"

# Create and push tag
echo "🏷️  Creating tag ${TAG}..."
git tag -a "$TAG" -m "Release ${VERSION}"

# Push changes and tag
echo "📤 Pushing to remote..."
git push origin main
git push origin "$TAG"

echo ""
echo "✅ Release ${VERSION} prepared!"
echo ""
echo "🚀 The GitHub Actions workflow will automatically:"
echo "   - Build wheels for all platforms and Python versions"
echo "   - Publish to PyPI"
echo ""
echo "📋 Optional: Create a GitHub release for better visibility:"
echo "   1. Go to https://github.com/eddiethedean/formatparse/releases/new"
echo "   2. Select tag ${TAG}"
echo "   3. Fill in release title and notes"
echo "   4. Click 'Publish release'"

