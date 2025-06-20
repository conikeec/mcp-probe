# 🚀 Release Automation Setup Guide

This guide helps you set up the complete automated release system for MCP Probe, including binary builds, crates.io publishing, and Homebrew distribution.

## 📋 Prerequisites

Before setting up automated releases, ensure you have:

- [x] A GitHub repository for the project
- [x] Access to repository settings and secrets
- [x] A crates.io account and API token
- [x] A separate GitHub repository for your Homebrew tap

## 🔑 Required GitHub Secrets

You need to configure the following secrets in your GitHub repository settings (`Settings > Secrets and variables > Actions`):

### 1. `CRATES_IO_TOKEN`

**Purpose**: Publish crates to crates.io automatically

**How to get it**:

1. Visit [crates.io](https://crates.io)
2. Log in with your GitHub account
3. Go to Account Settings > API Tokens
4. Create a new token with publish permissions
5. Copy the token (starts with `cio_`)

**Example**: `cio_1234567890abcdef...`

### 2. `HOMEBREW_TAP_TOKEN`

**Purpose**: Update Homebrew formula in your tap repository

**How to get it**:

1. Create a personal access token on GitHub
2. Go to Settings > Developer settings > Personal access tokens > Tokens (classic)
3. Generate new token with `repo` scope
4. Select your homebrew-tap repository or give full repo access
5. Copy the token (starts with `ghp_`)

**Example**: `ghp_1234567890abcdef...`

## 🍺 Homebrew Tap Setup

### 1. Create Homebrew Tap Repository

```bash
# Create a new repository named 'homebrew-tap'
# Repository should be public and named exactly: conikeec/homebrew-tap
# (replace 'conikeec' with your GitHub username)
```

### 2. Initialize Tap Structure

```bash
# Clone your new homebrew-tap repository
git clone https://github.com/conikeec/homebrew-tap.git
cd homebrew-tap

# Create Formula directory
mkdir -p Formula

# Copy the initial formula (already created in this project)
cp ../mcp-probe/Formula/mcp-probe.rb Formula/

# Commit and push
git add Formula/mcp-probe.rb
git commit -m "Add initial mcp-probe formula"
git push origin main
```

## 🔧 Repository Configuration

### 1. Enable GitHub Actions

Ensure GitHub Actions are enabled in your repository:

- Go to `Settings > Actions > General`
- Select "Allow all actions and reusable workflows"
- Save changes

### 2. Configure Branch Protection (Optional but Recommended)

```bash
# In your repository settings:
Settings > Branches > Add rule

Branch name pattern: main
☑️ Require status checks to pass before merging
☑️ Require branches to be up to date before merging
☑️ Status checks: CI (all jobs must pass)
```

## 🚀 Release Process

### Automated Release (Recommended)

1. **Create a Git Tag**:

   ```bash
   # Create and push a version tag
   git tag v0.1.0
   git push origin v0.1.0
   ```

2. **Release Automation Triggers**:
   - ✅ Creates GitHub release with binaries
   - ✅ Publishes to crates.io
   - ✅ Updates Homebrew formula
   - ✅ Generates release notes

### Manual Release Trigger

You can also trigger releases manually from GitHub Actions:

1. Go to `Actions > Release`
2. Click "Run workflow"
3. Enter the tag version (e.g., `v0.1.0`)
4. Click "Run workflow"

## 📦 What Gets Built

The release system automatically creates:

### 📥 Binary Distributions

- `mcp-probe-x86_64-unknown-linux-gnu.tar.gz` (+ .sha256)
- `mcp-probe-aarch64-unknown-linux-gnu.tar.gz` (+ .sha256)
- `mcp-probe-x86_64-apple-darwin.tar.gz` (+ .sha256)
- `mcp-probe-aarch64-apple-darwin.tar.gz` (+ .sha256)
- `mcp-probe-x86_64-pc-windows-msvc.zip` (+ .sha256)

### 📦 Package Distributions

- Published to crates.io: `mcp-core` and `mcp-cli`
- Updated Homebrew formula in your tap
- GitHub release with comprehensive release notes

## 🧪 Testing the Setup

### 1. Test CI Pipeline

```bash
# Create a test branch and push
git checkout -b test-ci
git push origin test-ci

# Check that CI runs successfully in GitHub Actions
```

### 2. Test Release Process

```bash
# Create a test release (use a pre-release version)
git tag v0.0.1-test
git push origin v0.0.1-test

# Monitor the release workflow in GitHub Actions
# Check that all jobs complete successfully
```

### 3. Verify Installations

After a successful release:

```bash
# Test curl install
curl -fsSL https://raw.githubusercontent.com/conikeec/mcp-probe/main/install.sh | bash

# Test Homebrew (after formula is updated)
brew install conikeec/tap/mcp-probe

# Test cargo install
cargo install mcp-cli
```

## 🐛 Troubleshooting

### Common Issues

#### 1. Crates.io Publishing Fails

- ❌ **Error**: "crate already exists"
- ✅ **Solution**: Increment version in Cargo.toml files

#### 2. Homebrew Formula Update Fails

- ❌ **Error**: Permission denied to homebrew-tap repository
- ✅ **Solution**: Check `HOMEBREW_TAP_TOKEN` has correct repository access

#### 3. Binary Build Fails

- ❌ **Error**: Compilation errors
- ✅ **Solution**: Run `./scripts/check.sh` locally first

#### 4. Cross-compilation Issues

- ❌ **Error**: "target not supported"
- ✅ **Solution**: Check Rust target availability, may need to update workflow

### Debug Steps

1. **Check GitHub Actions Logs**:

   - Go to Actions tab in your repository
   - Click on failed workflow
   - Examine logs for specific error messages

2. **Validate Secrets**:

   ```bash
   # Test crates.io token
   cargo login $CRATES_IO_TOKEN
   cargo publish --dry-run

   # Test GitHub token
   curl -H "Authorization: token $HOMEBREW_TAP_TOKEN" \
        https://api.github.com/repos/conikeec/homebrew-tap
   ```

3. **Test Locally**:

   ```bash
   # Run all checks
   ./scripts/check.sh

   # Test builds for different targets
   cargo build --target x86_64-unknown-linux-gnu
   cargo build --target x86_64-apple-darwin
   ```

## 📞 Support

If you encounter issues:

1. 📖 Check the [GitHub Actions documentation](https://docs.github.com/en/actions)
2. 🔍 Search existing [GitHub Issues](https://github.com/conikeec/mcp-probe/issues)
3. 💬 Open a new issue with:
   - Error logs from GitHub Actions
   - Steps to reproduce
   - Your repository/workflow configuration

---

## ✅ Setup Checklist

- [ ] GitHub repository created and configured
- [ ] GitHub Actions enabled
- [ ] `CRATES_IO_TOKEN` secret added
- [ ] `HOMEBREW_TAP_TOKEN` secret added
- [ ] Homebrew tap repository created (`conikeec/homebrew-tap`)
- [ ] Initial formula committed to tap repository
- [ ] CI pipeline tested with test branch
- [ ] Release process tested with test tag
- [ ] Installation methods verified

**🎉 Once complete, your release automation is ready!**

Simply push a version tag to trigger a complete release cycle:

```bash
git tag v0.1.0
git push origin v0.1.0
```
