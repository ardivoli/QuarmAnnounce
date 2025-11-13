# Release Process

This document describes the process for creating new releases of Quarm Announce.

## Table of Contents

- [Overview](#overview)
- [Release Process](#release-process)
- [Testing Releases](#testing-releases)
- [Troubleshooting](#troubleshooting)

## Overview

Releases are automated via GitHub Actions. When you push a git tag matching the pattern `v*.*.*`, the workflow will:

1. Validate that the tag version matches the version in `Cargo.toml`
2. Build binaries for Windows (x86_64-pc-windows-msvc) and Linux (x86_64-unknown-linux-gnu)
3. Run all unit tests on both platforms
4. Package binaries with required files (`config.json` and `speakers/` directory)
5. Generate a changelog from commit messages
6. Create a draft GitHub release with attached archives

## Release Process

Follow these steps to create a new release:

### 1. Update Cargo.toml Version

Edit `Cargo.toml` and update the `version` field:

```toml
[package]
name = "quarm_announce"
version = "1.2.3"  # Update this
```

### 2. Commit the Version Change

Commit your version change with a conventional commit message:

```bash
git add Cargo.toml
git commit -m "chore: bump version to 1.2.3"
```

### 3. Create and Push Git Tag

Create a git tag matching the version you just set:

```bash
git tag v1.2.3
git push origin v1.2.3
```

**Important:** The tag version (without the `v` prefix) must exactly match the version in `Cargo.toml`. If they don't match, the workflow will fail with a clear error message.

### 4. Monitor Workflow Execution

1. Navigate to your repository's **Actions** tab on GitHub
2. Find the "Multi-Platform Release Build" workflow run
3. Monitor the progress of all jobs:
   - Validate Version
   - Build Windows Binary
   - Build Linux Binary
   - Package Release Artifacts
   - Create GitHub Release

The workflow typically takes 10-20 minutes to complete.

### 5. Review Draft Release

Once the workflow completes:

1. Navigate to the **Releases** page in your repository
2. Find the draft release for your version (e.g., "v1.2.3")
3. Verify the release includes:
   - Both platform archives (`quarm_announce-v1.2.3-windows-x64.zip` and `quarm_announce-v1.2.3-linux-x64.tar.gz`)
   - Auto-generated changelog with grouped commits
4. Optionally edit the release notes to add additional context or highlights

### 6. Test the Release (Recommended)

Download and test the archives on their target platforms before publishing:

**Windows:**
```powershell
# Extract the archive
Expand-Archive quarm_announce-v1.2.3-windows-x64.zip -DestinationPath test-release

# Verify contents
dir test-release
# Should contain: quarm_announce.exe, config.json, speakers/

# Test the binary
cd test-release
.\quarm_announce.exe
```

**Linux:**
```bash
# Extract the archive
tar -xzf quarm_announce-v1.2.3-linux-x64.tar.gz -C test-release

# Verify contents and permissions
ls -la test-release
# Should contain: quarm_announce (executable), config.json, speakers/

# Test the binary
cd test-release
./quarm_announce
```

### 7. Publish the Release

Once you've verified everything looks good:

1. Go to the draft release on GitHub
2. Click **Edit**
3. Click **Publish release**

The release is now public and users can download it from the Releases page.

## Testing Releases

You can test the release workflow without creating a real release using test tags or manual dispatch.

### Using Test Tags

Use version tags with a `-test` suffix to test the full release workflow:

```bash
# Update Cargo.toml to a test version (e.g., 0.0.0-test)
git commit -am "test: testing release workflow"
git tag v0.0.0-test
git push origin v0.0.0-test
```

This will trigger the full workflow including draft release creation.

**Cleanup test tags:**

```bash
# Delete local tag
git tag -d v0.0.0-test

# Delete remote tag
git push origin :refs/tags/v0.0.0-test

# Delete the draft release from GitHub UI
```

### Using Manual Dispatch

You can also trigger the workflow manually without creating a release:

1. Go to **Actions** tab in GitHub
2. Select "Multi-Platform Release Build" workflow
3. Click **Run workflow**
4. Select the branch to run from
5. Click **Run workflow**

**Note:** Manual dispatch will run all jobs except `create-release` (which only runs on tag pushes).

## Troubleshooting

### Version Mismatch Error

**Error Message:**
```
Error: Version mismatch! Git tag is v1.2.3 but Cargo.toml has version 1.2.2
Please update Cargo.toml version to 1.2.3 before creating this release
```

**Solution:**
1. Update `Cargo.toml` to match the git tag version
2. Commit the change
3. Delete the old tag: `git tag -d v1.2.3 && git push origin :refs/tags/v1.2.3`
4. Create a new tag after committing: `git tag v1.2.3 && git push origin v1.2.3`

### Git LFS File Download Failure

**Error Message:**
```
Error: ONNX model file is too small (132 bytes). Expected at least 60MB. This may be an LFS pointer file.
Ensure Git LFS is properly configured and the model was downloaded.
```

**Solution:**
This usually means Git LFS is not properly configured for the repository. Ensure:
1. Git LFS is installed on your system: `git lfs install`
2. The `.gitattributes` file includes: `*.onnx filter=lfs diff=lfs merge=lfs -text`
3. The ONNX model is actually tracked by LFS: `git lfs ls-files` should show the model
4. Force re-push LFS files if needed: `git lfs push --all origin`

### Build Errors on Windows

**Error Message:**
```
error: linking with `link.exe` failed: exit code: 1181
```

**Solution:**
This typically indicates missing Windows build tools. The workflow uses `windows-latest` runners which should have all required tools pre-installed. If you see this error:
1. Check the workflow logs for more specific error messages
2. Verify `Cargo.toml` dependencies are correct
3. Ensure no platform-specific dependencies are missing their Windows support

### Build Errors on Linux

**Error Message:**
```
error: failed to run custom build command for `alsa-sys v0.3.1`
```

**Solution:**
This means system dependencies are missing. The workflow installs required packages via:
```bash
sudo apt-get install -y build-essential pkg-config libasound2-dev
```

If you see this error, the workflow job may need additional packages added.

### Missing Dependencies on SteamOS

**Known Limitation:**
SteamOS may require additional system packages beyond what Ubuntu provides:
- `glibc` (GNU C Library)
- `make` (build tools)

**Solution:**
The Linux binaries are built on Ubuntu but should work on SteamOS. If you encounter issues running on SteamOS:
1. Install missing packages: `sudo steamos-readonly disable && sudo pacman -S glibc make`
2. Report the issue for documentation updates
3. Consider building directly on SteamOS for full compatibility

### Test Failures

**Error Message:**
```
test result: FAILED. 13 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

**Solution:**
1. Review the specific test that failed in the workflow logs
2. Fix the failing test in your code
3. Run `cargo test` locally to verify the fix
4. Commit and push the fix
5. Delete and recreate the release tag

### Workflow Doesn't Trigger

**Issue:** Pushed a tag but workflow didn't run.

**Solution:**
1. Verify tag format matches `v*.*.*` pattern (e.g., `v1.2.3`)
2. Check that the tag was pushed to the remote: `git ls-remote --tags origin`
3. Verify workflow file is committed to `main` branch: `.github/workflows/release.yml`
4. Check **Actions** tab is enabled in repository settings

### Draft Release Not Created

**Issue:** Workflow completes but no draft release appears.

**Solution:**
1. Verify the `create-release` job ran (it only runs on tag pushes, not manual dispatch)
2. Check the job logs for errors
3. Verify the repository has `contents: write` permission for the workflow
4. Ensure `GITHUB_TOKEN` has proper permissions (should be automatic)

## Additional Resources

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [GitHub Releases Documentation](https://docs.github.com/en/repositories/releasing-projects-on-github)
- [Semantic Versioning](https://semver.org/)
- [Conventional Commits](https://www.conventionalcommits.org/)
