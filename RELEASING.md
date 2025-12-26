# Releasing to PyPI

This document explains how to release a new version of `structparse` to PyPI.

## Quick Release

Use the release script:

```bash
./scripts/release.sh 0.1.0
```

This will:
1. Update the version in `Cargo.toml`
2. Commit the change
3. Create and push a git tag (e.g., `v0.1.0`)
4. Push to the main branch

The GitHub Actions workflow will **automatically** trigger when you push the tag and will:
- Build wheels for all platforms and Python versions
- Publish to PyPI

**Note:** Creating a GitHub release is optional but recommended for better visibility and release notes.

## Manual Release

If you prefer to do it manually:

1. **Update version in `Cargo.toml`:**
   ```toml
   version = "0.1.0"  # Update this
   ```

2. **Commit the change:**
   ```bash
   git add Cargo.toml
   git commit -m "Bump version to 0.1.0"
   ```

3. **Create and push a tag:**
   ```bash
   git tag -a v0.1.0 -m "Release 0.1.0"
   git push origin main
   git push origin v0.1.0
   ```

4. **Create a GitHub release:**
   - Go to the releases page
   - Click "Draft a new release"
   - Select the tag
   - Fill in release notes
   - Click "Publish release"

## Version Numbering

Follow [Semantic Versioning](https://semver.org/):
- **MAJOR** version for incompatible API changes
- **MINOR** version for backwards-compatible functionality
- **PATCH** version for backwards-compatible bug fixes

Examples:
- `0.1.0` → `0.1.1` (patch)
- `0.1.0` → `0.2.0` (minor)
- `0.1.0` → `1.0.0` (major)

## What Gets Built

The workflow builds:
- **Wheels** for:
  - Linux (manylinux)
  - macOS (universal2)
  - Windows
  - Python versions: 3.9, 3.10, 3.11, 3.12, 3.13
- **Source distribution** (sdist)

All artifacts are automatically published to PyPI.

## Troubleshooting

- **Version already exists on PyPI**: Update the version number in `Cargo.toml`
- **Workflow fails**: Check the Actions tab for error details
- **Tag already exists**: Delete it with `git tag -d v0.1.0 && git push origin :refs/tags/v0.1.0`

