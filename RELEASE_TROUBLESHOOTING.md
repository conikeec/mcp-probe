# 🔧 Release Troubleshooting Guide

This guide helps you resolve common issues with the automated release system.

## ❌ "Resource not accessible by integration" Error

This error occurs when GitHub Actions doesn't have sufficient permissions. Here are the solutions:

### ✅ Solution 1: Check Repository Settings

1. Go to your GitHub repository
2. Click **Settings** → **Actions** → **General**
3. Under "Workflow permissions", select:
   - ☑️ **"Read and write permissions"**
   - ☑️ **"Allow GitHub Actions to create and approve pull requests"**
4. Click **Save**

### ✅ Solution 2: Verify Token Permissions

The workflow now uses the built-in `GITHUB_TOKEN` which should have the right permissions. If you're using a custom token:

1. Go to **Settings** → **Developer settings** → **Personal access tokens**
2. Ensure your token has these scopes:
   - ☑️ `repo` (Full control of private repositories)
   - ☑️ `write:packages` (Upload packages to GitHub Package Registry)

### ✅ Solution 3: Check Branch Protection Rules

If you have branch protection enabled:

1. Go to **Settings** → **Branches**
2. Edit your branch protection rule
3. Under "Restrict pushes that create files":
   - ☑️ Allow GitHub Actions to create releases

## 🚀 Testing the Fixed Release Process

### 1. Test with a Pre-release

```bash
# Create a test tag
git tag v0.0.1-test
git push origin v0.0.1-test
```

### 2. Monitor the Workflow

1. Go to **Actions** tab in your repository
2. Click on the "Release" workflow
3. Watch each job complete:
   - ✅ Create Release
   - ✅ Build Binaries (all platforms)
   - ✅ Publish to crates.io
   - ✅ Update Homebrew

### 3. Verify Release Assets

After successful completion, check:

- ✅ GitHub Release is created with binaries
- ✅ All platform archives are uploaded
- ✅ SHA256 checksums are included
- ✅ Crates are published to crates.io

## 🐛 Other Common Issues

### Issue: Binary Build Fails

**Symptoms**: Compilation errors during cross-compilation

**Solution**:

```bash
# Test locally first
./scripts/check.sh

# Test specific target
cargo build --target x86_64-unknown-linux-gnu
```

### Issue: Crates.io Publishing Fails

**Symptoms**: "crate already exists" or authentication errors

**Solutions**:

1. **Version Conflict**: Increment version in `Cargo.toml`
2. **Token Issues**: Verify `CRATES_IO_TOKEN` secret is correct
3. **Dependencies**: Ensure `mcp-core` publishes before `mcp-cli`

### Issue: Homebrew Update Fails

**Symptoms**: Permission denied or repository not found

**Solutions**:

1. **Token Permissions**: Ensure `HOMEBREW_TAP_TOKEN` has repo access
2. **Repository**: Verify `conikeec/homebrew-tap` exists and is public
3. **Formula Path**: Check `Formula/mcp-probe.rb` exists in tap repo

## 📋 Release Checklist

Before creating a release:

- [ ] All tests pass locally (`./scripts/check.sh`)
- [ ] Version updated in `Cargo.toml` files
- [ ] `CHANGELOG.md` updated with new version
- [ ] GitHub repository settings allow Actions to write
- [ ] Required secrets are configured:
  - [ ] `CRATES_IO_TOKEN`
  - [ ] `HOMEBREW_TAP_TOKEN`
- [ ] Homebrew tap repository exists and is accessible

## 🔄 Manual Release (if automation fails)

If automation continues to fail, you can release manually:

### 1. Create GitHub Release

```bash
# Create release manually
gh release create v0.1.0 \
  --title "MCP Probe v0.1.0" \
  --notes-file CHANGELOG.md
```

### 2. Build and Upload Binaries

```bash
# Build for your platform
cargo build --release

# Upload binary
gh release upload v0.1.0 target/release/mcp-probe
```

### 3. Publish to Crates.io

```bash
# Publish core first
cd crates/mcp-core
cargo publish

# Wait 60 seconds, then publish CLI
cd ../mcp-cli
cargo publish
```

## 📞 Getting Help

If issues persist:

1. **Check Workflow Logs**: Look at the specific error in GitHub Actions
2. **Search Issues**: Check existing [GitHub Issues](https://github.com/conikeec/mcp-probe/issues)
3. **Create Issue**: Include:
   - Full error message from GitHub Actions
   - Repository settings screenshots
   - Steps you've already tried

---

## 🎯 Quick Fix Commands

```bash
# Re-run checks locally
./scripts/check.sh

# Force push a tag (if needed)
git tag -d v0.1.0  # Delete locally
git push origin :refs/tags/v0.1.0  # Delete remotely
git tag v0.1.0     # Recreate
git push origin v0.1.0  # Push again

# Test manual release
gh release create v0.1.0-manual --draft
```

The updated workflow should now work correctly with proper permissions!
