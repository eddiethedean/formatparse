#!/bin/bash
# Release script for structparse
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

# Update version in Cargo.toml
echo "📝 Updating version in Cargo.toml to ${VERSION}..."
sed -i.bak "s/^version = \".*\"/version = \"${VERSION}\"/" Cargo.toml
rm Cargo.toml.bak

# Commit the version change
echo "💾 Committing version change..."
git add Cargo.toml
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
echo "   1. Go to https://github.com/eddiethedean/structparse/releases/new"
echo "   2. Select tag ${TAG}"
echo "   3. Fill in release title and notes"
echo "   4. Click 'Publish release'"

