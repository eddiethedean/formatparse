# Releasing to PyPI

This document explains how to release a new version of `formatparse` to PyPI.

## Current version

The canonical version is **`0.8.4`** in the workspace [`Cargo.toml`](Cargo.toml) (`[workspace.package] version` and `[workspace.dependencies] formatparse-core` must match). Member crates use `version.workspace = true`. PyPI metadata uses `dynamic = ["version"]`; Maturin reads the workspace version when building wheels and sdist.

**PyPI latest published:** `0.8.3` (as of the 0.8.4 prep). To ship 0.8.4, tag `v0.8.4` after the checklist below—do not re-run `release.sh` with a version bump unless you change the workspace version again.

Before tagging, confirm:

- `[workspace.package] version` and `formatparse-core` dependency version in root [`Cargo.toml`](Cargo.toml) are both **`0.8.4`**.
- [`CHANGELOG.md`](CHANGELOG.md) lists all 0.8.4 changes under `## [0.8.4]` (not only under `[Unreleased]`).
- CI on `main` is green.

If the workspace version is **already** `X.Y.Z` on `main` (for example after a merge that bumped it), you do **not** need `release.sh` to edit `Cargo.toml` again. Push the tag only (see [Tag without a version bump](#tag-without-a-version-bump) below).

## Quick release

Use the release script (from repo root, on `main`, clean working tree):

```bash
./scripts/release.sh X.Y.Z
```

Example for a patch after 0.8.0:

```bash
./scripts/release.sh 0.8.1
```

This will:

1. Set `version = "X.Y.Z"` on the `version` line in the root `Cargo.toml` (workspace package version).
2. Commit the change.
3. Create and push a git tag `vX.Y.Z`.
4. Push `main` to `origin`.

The [Release (PyPI and crates.io)](.github/workflows/publish.yml) workflow runs on tag push and will:

- Run security checks, then publish **`formatparse-core`** and **`formatparse-pyo3`** to [crates.io](https://crates.io/) using the repository secret **`CARGO_REGISTRY_TOKEN`** (crates.io API token with `publish-new` / `publish-update` scopes). `formatparse-core` is published first so the path dependency in `formatparse-pyo3` resolves on the registry.
- Build wheels for Linux (manylinux), macOS, and Windows (including Intel and Windows 11 ARM where applicable).
- Build Python **3.8 through 3.14** wheels per the publish matrix (where `setup-python` / manylinux provide those interpreters).
- Build an sdist, then publish to PyPI (trusted publishing).

**Wheel metadata:** `[tool.maturin] compatibility = "linux"` in `pyproject.toml` sets the manylinux / auditwheel policy for **Linux** wheels built during publish. It does not restrict wheels for other platforms or local `maturin develop`.

Publish workflows install **`maturin>=1.13.3,<2.0`** (see `pyproject.toml` `[build-system].requires`). Do not set `[tool.maturin] python-source` to the repo root for this layout: `maturin sdist` then expects a Python tree named `_formatparse` at the workspace root, which this PyO3-only crate never ships in git.

**Note:** Creating a GitHub **Release** (with notes) is optional but recommended; copy highlights from `CHANGELOG.md`.

## Tag without a version bump

Use this when `Cargo.toml` already has `[workspace.package] version = "X.Y.Z"` and you only need to publish:

```bash
git tag -a vX.Y.Z -m "Release X.Y.Z"
git push origin vX.Y.Z
```

The publish workflow runs on the tag push.

## Manual release

1. **Bump `[workspace.package] version` in the root `Cargo.toml`** and the matching **`[workspace.dependencies] formatparse-core` version** (required for `cargo publish` of `formatparse-pyo3`).
2. **Update `CHANGELOG.md`** for the new version.
3. **Commit:** `git add Cargo.toml CHANGELOG.md && git commit -m "Bump version to X.Y.Z"`
4. **Tag and push:**
   ```bash
   git tag -a vX.Y.Z -m "Release X.Y.Z"
   git push origin main
   git push origin vX.Y.Z
   ```
5. **Optional:** Draft a GitHub Release from the tag and attach release notes.

## Version numbering

Follow [Semantic Versioning](https://semver.org/):

- **MAJOR** — incompatible API changes
- **MINOR** — backwards-compatible functionality
- **PATCH** — backwards-compatible bug fixes

## Troubleshooting

- **Version already exists on PyPI or crates.io:** bump the workspace version and re-tag (never reuse a published version).
- **crates.io publish fails on `formatparse-pyo3`:** ensure `formatparse-core` for that version published successfully first; confirm `CARGO_REGISTRY_TOKEN` has the required scopes and the crate names are not owned by another account.
- **Workflow fails:** inspect the Actions tab; common issues are missing Python on a new runner image or transient PyPI/network errors.
- **Tag already exists locally:** `git tag -d vX.Y.Z` then recreate, or choose a new patch version.
- **Tag already on remote:** coordinate with maintainers before force-deleting remote tags.
