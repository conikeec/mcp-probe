# Development Scripts

This directory contains automation scripts for development, testing, and release management.

## Pre-commit Validation

### `pre-commit.sh`
Runs the exact same checks as GitHub Actions CI to catch issues early.

**Usage:**
```bash
./scripts/pre-commit.sh
```

**Checks performed:**
- `cargo fmt --all -- --check` (formatting)
- `cargo clippy --all-targets --all-features -- -D warnings` (linting)  
- `cargo test --all-targets --all-features` (testing)

Run before committing to prevent CI failures!

## Version Management

### `version-bump.sh`
Automated version management across the entire workspace.

**Usage:**
```bash
# Bump patch version (1.0.0 -> 1.0.1)
./scripts/version-bump.sh patch

# Bump minor version (1.0.0 -> 1.1.0)
./scripts/version-bump.sh minor

# Bump major version (1.0.0 -> 2.0.0)
./scripts/version-bump.sh major

# Set specific version
./scripts/version-bump.sh 1.5.0

# Dry run to see what would change
./scripts/version-bump.sh patch --dry-run

# Update version without committing
./scripts/version-bump.sh minor --no-commit

# Update and commit but don't tag
./scripts/version-bump.sh patch --no-tag
```

**Features:**
- ✅ Updates workspace version in `Cargo.toml`
- ✅ Updates dependency versions in `crates/mcp-cli/Cargo.toml`
- ✅ Verifies build and tests with new version
- ✅ Creates git commit and tag automatically
- ✅ Prevents inconsistencies across files
- ✅ Dry-run mode for safety

**Files updated:**
- `Cargo.toml` (workspace version)
- `crates/mcp-cli/Cargo.toml` (mcp-probe-core dependency version)

## Release Workflow

### Automated Release Process

1. **Version Bump**: Use `version-bump.sh` or GitHub Actions
2. **Push Tag**: Triggers automated release workflow
3. **GitHub Actions**: Builds binaries, publishes to crates.io, updates Homebrew

### GitHub Actions Version Bump

Trigger automated version bumps from GitHub UI:

1. Go to **Actions** → **Version Bump**
2. Click **Run workflow**
3. Choose version type (patch/minor/major) or specify exact version
4. GitHub Actions will:
   - Run all tests
   - Update version files
   - Create commit and tag
   - Push to repository
   - Trigger release workflow

### cargo-release Integration

For advanced users, `cargo-release` provides additional automation:

```bash
# Install cargo-release
cargo install cargo-release

# Release with cargo-release (uses release.toml config)
cargo release patch
cargo release minor
cargo release major
```

## Best Practices

1. **Always use version management scripts** instead of manual editing
2. **Run pre-commit checks** before pushing changes
3. **Use dry-run mode** to preview changes before applying
4. **Let GitHub Actions handle publishing** - don't manually publish to crates.io
5. **Keep workspace versions synchronized** across all Cargo.toml files

## Troubleshooting

### Version Mismatch Issues
If you encounter version inconsistencies:

```bash
# Check current versions
grep -r "version.*=" Cargo.toml crates/*/Cargo.toml

# Fix with version bump script
./scripts/version-bump.sh <current_version> --no-commit
```

### Build Issues After Version Change
```bash
# Verify everything works
./scripts/pre-commit.sh

# Clean build cache
cargo clean && cargo build
```

### CI Failures
The pre-commit script runs the same checks as CI:
```bash
./scripts/pre-commit.sh
```

If this passes locally but CI fails, there may be environment differences or caching issues. 