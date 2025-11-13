# Implementation Tasks: Multi-Platform Release Automation

**Date:** 2025-11-13
**Related PRD:** [prd-multi-platform-release.md](./2025-11-13_github-actions-release-prd.md)
**Status:** Ready for Implementation

---

## Overview

This document breaks down the implementation of automated multi-platform releases into specific, actionable tasks. Tasks are organized by phase and include acceptance criteria for each item.

**Estimated Total Time:** 5 weeks (part-time development, ~20 hours total)

---

## Phase 1: Research & Setup (Week 1) - 4 hours

### Task 1.1: Research ONNX Runtime Build Requirements
**Priority:** High
**Estimated Time:** 2 hours

**Description:**
Investigate how the `ort` crate (v2.0.0-rc.9) handles ONNX Runtime dependencies on different platforms. Determine if special build configuration is needed for GitHub Actions runners.

**Steps:**
1. Read `ort` crate documentation: https://docs.rs/ort/
2. Check if GitHub Actions runners (ubuntu-latest, windows-latest) include ONNX Runtime by default
3. Review `ort` features and configuration options (especially `ort-sys` with `default-features = false`)
4. Test local build on clean Ubuntu and Windows VMs if possible
5. Document findings in this file or create separate notes

**Acceptance Criteria:**
- [ ] Know whether ONNX Runtime is available on GitHub runners by default
- [ ] Know which system packages (if any) need to be installed
- [ ] Know which `ort` features/configuration to use in workflow
- [ ] Have fallback plan if default setup doesn't work

**Resources:**
- ort documentation: https://docs.rs/ort/
- GitHub Actions runner images: https://github.com/actions/runner-images

---

### Task 1.2: Research Changelog Generation Tools
**Priority:** Medium
**Estimated Time:** 1 hour

**Description:**
Evaluate options for automatically generating changelogs from conventional commits.

**Options to Evaluate:**
1. **GitHub's built-in release notes generator** (via API)
   - Pros: No external dependencies, simple
   - Cons: Basic grouping, less customization

2. **release-changelog-builder-action** by mikepenz
   - Pros: Highly customizable, good conventional commit support
   - Cons: Additional action dependency

3. **git-cliff** action
   - Pros: Very powerful, keeps changelog in repo
   - Cons: More complex setup

**Steps:**
1. Review each tool's documentation
2. Test with example commits from this repo
3. Choose tool that best balances simplicity and functionality
4. Document choice with rationale

**Acceptance Criteria:**
- [ ] One changelog tool selected
- [ ] Understand how to configure it for conventional commits
- [ ] Know how to group by feat/fix/chore/etc.
- [ ] Know how to format output for GitHub release body

**Recommendation:** Start with GitHub's built-in generator, enhance later if needed

---

### Task 1.3: Create Initial Workflow File Structure
**Priority:** High
**Estimated Time:** 1 hour

**Description:**
Create the skeleton GitHub Actions workflow YAML file with basic structure and triggers.

**Steps:**
1. Create `.github/workflows/` directory if it doesn't exist
2. Create `release.yml` workflow file
3. Set up triggers (git tag push + manual dispatch)
4. Add job placeholders for build-windows, build-linux, create-release
5. Add basic checkout step
6. Test workflow triggers (push test tag, verify workflow runs)

**File Path:** `.github/workflows/release.yml`

**Basic Structure:**
```yaml
name: Multi-Platform Release Build

on:
  push:
    tags:
      - 'v*.*.*'
  workflow_dispatch:

jobs:
  validate-version:
    # Validate tag matches Cargo.toml

  build-windows:
    # Build Windows binary

  build-linux:
    # Build Linux binary

  create-release:
    # Package artifacts and create GitHub release
    needs: [validate-version, build-windows, build-linux]
```

**Acceptance Criteria:**
- [ ] `.github/workflows/release.yml` file created
- [ ] Workflow triggers on `v*.*.*` tags
- [ ] Workflow can be manually triggered from GitHub UI
- [ ] Workflow shows up in Actions tab
- [ ] Basic structure includes all required jobs

**Testing:**
- Push test tag: `git tag v0.0.0-test && git push origin v0.0.0-test`
- Delete test tag after: `git tag -d v0.0.0-test && git push origin :refs/tags/v0.0.0-test`

---

## Phase 2: Core Build Implementation (Week 2) - 6 hours

### Task 2.1: Implement Version Validation Job
**Priority:** High
**Estimated Time:** 1 hour
**Depends On:** Task 1.3

**Description:**
Create a job that extracts the version from the git tag and verifies it matches the version in `Cargo.toml`. This prevents releases with mismatched versions.

**Steps:**
1. Add `validate-version` job to workflow
2. Extract version from `GITHUB_REF` (e.g., `refs/tags/v1.2.3` → `1.2.3`)
3. Read version from `Cargo.toml` using `cargo metadata` or `grep`
4. Compare versions and fail if mismatch
5. Output version for use in later jobs

**Implementation Notes:**
- Use `github.ref_name` to get tag name
- Use `cargo metadata --format-version 1 | jq -r '.packages[0].version'` to get Cargo version
- Store version in `GITHUB_OUTPUT` for downstream jobs

**Acceptance Criteria:**
- [ ] Job extracts version from git tag correctly
- [ ] Job reads version from Cargo.toml correctly
- [ ] Job fails workflow if versions don't match
- [ ] Job outputs version variable for other jobs to use
- [ ] Clear error message explains mismatch if it occurs

**Test Cases:**
- Tag `v0.1.0` with Cargo.toml `version = "0.1.0"` → Pass
- Tag `v0.2.0` with Cargo.toml `version = "0.1.0"` → Fail with clear message

---

### Task 2.2: Implement Git LFS Checkout
**Priority:** High
**Estimated Time:** 30 minutes
**Depends On:** Task 1.3

**Description:**
Configure the repository checkout step to properly download Git LFS files (ONNX model).

**Steps:**
1. Update checkout action to use `actions/checkout@v4`
2. Add `lfs: true` parameter
3. Verify LFS files are downloaded (check file size)
4. Add verification step to ensure model file exists and is not a pointer

**Implementation:**
```yaml
- name: Checkout repository
  uses: actions/checkout@v4
  with:
    lfs: true

- name: Verify LFS files
  run: |
    if [ ! -f speakers/en_US-amy-medium.onnx ]; then
      echo "Error: ONNX model file not found"
      exit 1
    fi
    SIZE=$(stat -f%z speakers/en_US-amy-medium.onnx 2>/dev/null || stat -c%s speakers/en_US-amy-medium.onnx)
    if [ $SIZE -lt 60000000 ]; then
      echo "Error: ONNX model file too small ($SIZE bytes), may be LFS pointer"
      exit 1
    fi
    echo "LFS files verified successfully"
```

**Acceptance Criteria:**
- [ ] Git LFS files downloaded during checkout
- [ ] ONNX model file is 63MB, not a small pointer file
- [ ] Workflow fails early if LFS files missing/invalid
- [ ] Works on both ubuntu-latest and windows-latest runners

**Testing:**
- Check workflow logs to confirm LFS download messages
- Verify file size in verification step output

---

### Task 2.3: Implement Windows Build Job
**Priority:** High
**Estimated Time:** 2 hours
**Depends On:** Tasks 1.1, 2.2

**Description:**
Create the complete Windows build job that compiles the binary, runs tests, and performs smoke tests.

**Steps:**
1. Configure job to run on `windows-latest` runner
2. Set up Rust toolchain (stable, `x86_64-pc-windows-msvc` target)
3. Configure caching for dependencies using `Swatinem/rust-cache`
4. Run `cargo test`
5. Run `cargo build --release`
6. Verify binary exists at `target/release/quarm_announce.exe`
7. Run smoke test: `./target/release/quarm_announce.exe --help` (or similar)
8. Upload binary as artifact for later packaging

**Implementation Notes:**
- Install any Windows-specific dependencies (likely none needed, WASAPI is built-in)
- Rust cache action speeds up subsequent runs significantly
- Set working directory to ensure paths resolve correctly

**Example Job:**
```yaml
build-windows:
  name: Build Windows Binary
  runs-on: windows-latest
  needs: validate-version

  steps:
    - name: Checkout repository
      uses: actions/checkout@v4
      with:
        lfs: true

    - name: Set up Rust toolchain
      uses: dtolnay/rust-toolchain@stable
      with:
        targets: x86_64-pc-windows-msvc

    - name: Cache Rust dependencies
      uses: Swatinem/rust-cache@v2

    - name: Run tests
      run: cargo test

    - name: Build release binary
      run: cargo build --release

    - name: Smoke test binary
      run: |
        ./target/release/quarm_announce.exe --version
        # Or just check it can execute without error

    - name: Upload binary artifact
      uses: actions/upload-artifact@v4
      with:
        name: windows-binary
        path: target/release/quarm_announce.exe
```

**Acceptance Criteria:**
- [ ] Job runs on windows-latest runner
- [ ] All 14 unit tests pass
- [ ] Release binary builds successfully
- [ ] Binary is optimized (--release flag)
- [ ] Smoke test executes binary without errors
- [ ] Binary uploaded as artifact
- [ ] Job fails if any step fails

**Testing:**
- Trigger workflow and verify Windows job completes
- Download artifact and verify it's a valid PE executable

---

### Task 2.4: Implement Linux Build Job
**Priority:** High
**Estimated Time:** 2.5 hours
**Depends On:** Tasks 1.1, 2.2

**Description:**
Create the Linux build job with proper dependency installation for ONNX Runtime and audio libraries.

**Steps:**
1. Configure job to run on `ubuntu-latest` runner
2. Install system dependencies (ALSA, build tools, pkg-config)
3. Set up Rust toolchain (stable, `x86_64-unknown-linux-gnu` target)
4. Configure dependency caching
5. Run `cargo test`
6. Run `cargo build --release`
7. Verify binary exists at `target/release/quarm_announce`
8. Run smoke test: `./target/release/quarm_announce --help`
9. Upload binary as artifact

**System Dependencies to Install:**
- `build-essential` - GCC, make, etc.
- `pkg-config` - Build configuration
- `libasound2-dev` - ALSA development headers (for rodio)
- Any ONNX Runtime dependencies (research from Task 1.1)

**Implementation Notes:**
- Use `apt-get update && apt-get install -y <packages>`
- Document that this is for Ubuntu; SteamOS may need different packages
- Consider adding comment about known SteamOS requirements (glibc, make)

**Example Job:**
```yaml
build-linux:
  name: Build Linux Binary
  runs-on: ubuntu-latest
  needs: validate-version

  steps:
    - name: Checkout repository
      uses: actions/checkout@v4
      with:
        lfs: true

    - name: Install system dependencies
      run: |
        sudo apt-get update
        sudo apt-get install -y \
          build-essential \
          pkg-config \
          libasound2-dev
        # Note: SteamOS may require additional packages (glibc, make)

    - name: Set up Rust toolchain
      uses: dtolnay/rust-toolchain@stable
      with:
        targets: x86_64-unknown-linux-gnu

    - name: Cache Rust dependencies
      uses: Swatinem/rust-cache@v2

    - name: Run tests
      run: cargo test

    - name: Build release binary
      run: cargo build --release

    - name: Smoke test binary
      run: |
        ./target/release/quarm_announce --version
        # Verify it's an ELF binary
        file ./target/release/quarm_announce

    - name: Upload binary artifact
      uses: actions/upload-artifact@v4
      with:
        name: linux-binary
        path: target/release/quarm_announce
```

**Acceptance Criteria:**
- [ ] Job runs on ubuntu-latest runner
- [ ] System dependencies install without errors
- [ ] All 14 unit tests pass
- [ ] Release binary builds successfully
- [ ] Binary is ELF format, dynamically linked to glibc
- [ ] Smoke test executes binary without errors
- [ ] Binary uploaded as artifact
- [ ] Job fails if any step fails

**Testing:**
- Trigger workflow and verify Linux job completes
- Download artifact and verify it's a valid ELF executable
- Check that binary can execute (may need Docker Ubuntu container locally)

---

## Phase 3: Packaging & Release (Week 3) - 5 hours

### Task 3.1: Create Windows Artifact Packaging
**Priority:** High
**Estimated Time:** 1.5 hours
**Depends On:** Task 2.3

**Description:**
Create a job step that downloads the Windows binary artifact and packages it with configuration files and TTS models into a `.zip` archive.

**Steps:**
1. Download `windows-binary` artifact
2. Create staging directory structure
3. Copy binary to staging directory
4. Copy `config.json` to staging directory
5. Copy `speakers/` directory (with models) to staging directory
6. Create `.zip` archive with proper naming
7. Upload packaged archive as artifact

**Directory Structure to Create:**
```
staging/
├── quarm_announce.exe
├── config.json
└── speakers/
    ├── en_US-amy-medium.onnx
    └── en_US-amy-medium.onnx.json
```

**Archive Naming:** `quarm_announce-v{version}-windows-x64.zip`

**Implementation Example:**
```yaml
- name: Download Windows binary
  uses: actions/download-artifact@v4
  with:
    name: windows-binary
    path: ./artifacts/windows

- name: Create Windows package
  run: |
    mkdir -p staging-windows
    cp artifacts/windows/quarm_announce.exe staging-windows/
    cp config.json staging-windows/
    cp -r speakers staging-windows/
    cd staging-windows
    7z a ../quarm_announce-v${{ needs.validate-version.outputs.version }}-windows-x64.zip .

- name: Upload Windows package
  uses: actions/upload-artifact@v4
  with:
    name: windows-package
    path: quarm_announce-v${{ needs.validate-version.outputs.version }}-windows-x64.zip
```

**Acceptance Criteria:**
- [ ] Windows binary downloaded from artifact
- [ ] All required files copied to staging directory
- [ ] Directory structure matches specification
- [ ] `.zip` archive created with correct naming convention
- [ ] Archive includes all files (binary, config, speakers)
- [ ] Archive size approximately 65-70MB
- [ ] Packaged archive uploaded as artifact

**Testing:**
- Download packaged archive from workflow artifacts
- Extract locally and verify all files present
- Test that binary runs with included config and models

---

### Task 3.2: Create Linux Artifact Packaging
**Priority:** High
**Estimated Time:** 1.5 hours
**Depends On:** Task 2.4

**Description:**
Create a job step that packages the Linux binary into a `.tar.gz` archive with proper file permissions preserved.

**Steps:**
1. Download `linux-binary` artifact
2. Create staging directory structure
3. Copy binary to staging directory and ensure execute permissions
4. Copy `config.json` to staging directory
5. Copy `speakers/` directory to staging directory
6. Create `.tar.gz` archive preserving permissions
7. Upload packaged archive as artifact

**Archive Naming:** `quarm_announce-v{version}-linux-x64.tar.gz`

**Implementation Example:**
```yaml
- name: Download Linux binary
  uses: actions/download-artifact@v4
  with:
    name: linux-binary
    path: ./artifacts/linux

- name: Create Linux package
  run: |
    mkdir -p staging-linux
    cp artifacts/linux/quarm_announce staging-linux/
    chmod +x staging-linux/quarm_announce
    cp config.json staging-linux/
    cp -r speakers staging-linux/
    tar -czf quarm_announce-v${{ needs.validate-version.outputs.version }}-linux-x64.tar.gz \
      -C staging-linux .

- name: Upload Linux package
  uses: actions/upload-artifact@v4
  with:
    name: linux-package
    path: quarm_announce-v${{ needs.validate-version.outputs.version }}-linux-x64.tar.gz
```

**Acceptance Criteria:**
- [ ] Linux binary downloaded from artifact
- [ ] All required files copied to staging directory
- [ ] Binary has execute permissions (755)
- [ ] `.tar.gz` archive created with correct naming convention
- [ ] Archive includes all files with correct permissions
- [ ] Archive size approximately 65-70MB
- [ ] Packaged archive uploaded as artifact

**Testing:**
- Download packaged archive from workflow artifacts
- Extract locally: `tar -xzf quarm_announce-v*.tar.gz`
- Verify binary has execute permissions
- Verify all files present

---

### Task 3.3: Implement Changelog Generation
**Priority:** High
**Estimated Time:** 1.5 hours
**Depends On:** Task 1.2

**Description:**
Implement automatic changelog generation from commit messages using the tool selected in Task 1.2.

**Steps:**
1. Add changelog generation step to workflow
2. Configure to parse conventional commits (feat, fix, chore, etc.)
3. Group commits by type in output
4. Format output as markdown suitable for GitHub release body
5. Include link to full commit comparison
6. Store changelog in workflow output or file for release creation

**Implementation (using GitHub's built-in generator):**
```yaml
- name: Generate changelog
  id: changelog
  uses: actions/github-script@v7
  with:
    script: |
      const { data: release } = await github.rest.repos.generateReleaseNotes({
        owner: context.repo.owner,
        repo: context.repo.repo,
        tag_name: '${{ github.ref_name }}',
      });
      return release.body;
    result-encoding: string

- name: Save changelog
  run: echo "${{ steps.changelog.outputs.result }}" > changelog.md

- name: Upload changelog
  uses: actions/upload-artifact@v4
  with:
    name: changelog
    path: changelog.md
```

**Alternative (using release-changelog-builder if more control needed):**
```yaml
- name: Build Changelog
  id: changelog
  uses: mikepenz/release-changelog-builder-action@v4
  with:
    configuration: .github/changelog-config.json
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

**Acceptance Criteria:**
- [ ] Changelog generated from commits since last tag
- [ ] Commits grouped by type (Features, Bug Fixes, etc.)
- [ ] Conventional commit format parsed correctly
- [ ] Output formatted as markdown
- [ ] Includes link to full commit comparison
- [ ] Changelog stored for use in release creation
- [ ] Works even if some commits don't follow conventional format

**Testing:**
- Review generated changelog in workflow output
- Verify grouping is correct
- Check that all commits are included

---

### Task 3.4: Implement Draft Release Creation
**Priority:** High
**Estimated Time:** 1.5 hours
**Depends On:** Tasks 3.1, 3.2, 3.3

**Description:**
Create the final job that downloads all packaged artifacts and creates a draft GitHub release with proper assets attached.

**Steps:**
1. Create `create-release` job that depends on both build jobs
2. Download Windows and Linux package artifacts
3. Download changelog artifact
4. Create draft release using `softprops/action-gh-release` or similar
5. Set release title to git tag name
6. Set release body to generated changelog
7. Attach both platform archives as release assets
8. Mark as draft (not published)

**Implementation Example:**
```yaml
create-release:
  name: Create Draft Release
  runs-on: ubuntu-latest
  needs: [validate-version, build-windows, build-linux]
  if: startsWith(github.ref, 'refs/tags/v')

  steps:
    - name: Checkout repository
      uses: actions/checkout@v4

    - name: Download all artifacts
      uses: actions/download-artifact@v4
      with:
        path: ./release-artifacts

    - name: Display artifact structure
      run: ls -R ./release-artifacts

    - name: Create draft release
      uses: softprops/action-gh-release@v2
      with:
        draft: true
        name: ${{ github.ref_name }}
        body_path: ./release-artifacts/changelog/changelog.md
        files: |
          ./release-artifacts/windows-package/*.zip
          ./release-artifacts/linux-package/*.tar.gz
      env:
        GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

**Acceptance Criteria:**
- [ ] Job runs after both build jobs complete successfully
- [ ] All artifacts downloaded correctly
- [ ] Draft release created in GitHub Releases
- [ ] Release title matches git tag
- [ ] Release body contains generated changelog
- [ ] Both Windows and Linux archives attached as assets
- [ ] Release marked as draft (not visible to public)
- [ ] Release associated with correct tag/commit

**Testing:**
- Trigger workflow with test tag
- Verify draft release appears in GitHub Releases page
- Verify both assets are attached and downloadable
- Verify changelog displays correctly

---

## Phase 4: Error Handling & Edge Cases (Week 4) - 3 hours

### Task 4.1: Implement Build Failure Handling
**Priority:** High
**Estimated Time:** 1 hour

**Description:**
Ensure the workflow properly handles failures and doesn't create partial releases.

**Steps:**
1. Verify `create-release` job has `needs: [build-windows, build-linux]` dependency
2. Test that release job doesn't run if either build fails
3. Add conditional to skip release on `workflow_dispatch` manual runs (optional)
4. Ensure clear error messages in logs for each failure type

**Scenarios to Test:**
- Windows build fails → No release created
- Linux build fails → No release created
- Tests fail on either platform → No release created
- Version validation fails → No builds run, no release created

**Implementation Notes:**
- Default GitHub Actions behavior: jobs with `needs:` won't run if dependencies fail
- Add explicit `if: success()` conditions if needed for clarity

**Acceptance Criteria:**
- [ ] Release job only runs if ALL build jobs succeed
- [ ] Failed builds produce clear error messages
- [ ] No partial releases created (e.g., Windows-only)
- [ ] Workflow status shows as failed if any job fails

**Testing:**
- Introduce intentional build failure (syntax error in code)
- Verify workflow fails and no release created
- Fix code and verify workflow succeeds

---

### Task 4.2: Add Workflow Status Notifications (Optional)
**Priority:** Low
**Estimated Time:** 1 hour

**Description:**
Add notifications or status badges to track workflow success/failure.

**Steps:**
1. Add workflow status badge to README.md
2. Optionally configure failure notifications (email, Slack, etc.)
3. Document how to view workflow results

**Badge Example:**
```markdown
![Release Build](https://github.com/username/quarm_announce/workflows/Multi-Platform%20Release%20Build/badge.svg)
```

**Acceptance Criteria:**
- [ ] Status badge added to README (optional)
- [ ] Badge reflects current workflow status
- [ ] Documentation includes link to Actions tab

---

### Task 4.3: Test End-to-End with Real Release
**Priority:** High
**Estimated Time:** 1 hour

**Description:**
Perform a complete end-to-end test of the release workflow with a real version bump.

**Steps:**
1. Update `Cargo.toml` version from `0.1.0` to `0.2.0` (or appropriate next version)
2. Commit version change: `git commit -am "chore: bump version to 0.2.0"`
3. Create and push git tag: `git tag v0.2.0 && git push origin v0.2.0`
4. Monitor workflow execution in GitHub Actions
5. Verify draft release created correctly
6. Download both platform archives and test on actual systems
7. Review and publish the release (or delete if test)

**Validation Checklist:**
- [ ] Workflow triggered by tag push
- [ ] Version validation passed
- [ ] Both builds completed successfully
- [ ] Tests passed on both platforms
- [ ] Smoke tests executed without errors
- [ ] Artifacts packaged correctly
- [ ] Changelog generated with accurate grouping
- [ ] Draft release created with correct title/body
- [ ] Both archives attached as assets
- [ ] Windows binary runs on Windows 10/11
- [ ] Linux binary runs on Ubuntu 22.04+ (or SteamOS if available)

**Acceptance Criteria:**
- [ ] Complete workflow executes without errors
- [ ] Draft release ready for publishing
- [ ] Both binaries execute successfully on target platforms
- [ ] All files included in archives
- [ ] Documentation updated if any issues found

**Post-Test:**
- If test successful: Publish the release!
- If issues found: Fix and repeat test with v0.2.1

---

## Phase 5: Documentation & Cleanup (Week 5) - 2 hours

### Task 5.1: Document Release Process for Maintainers
**Priority:** Medium
**Estimated Time:** 1 hour

**Description:**
Create maintainer documentation explaining how to create releases using the new automated workflow.

**Steps:**
1. Create `docs/RELEASING.md` or add section to existing README
2. Document the release process step-by-step
3. Include troubleshooting section for common issues
4. Document how to manually publish draft releases

**Documentation Outline:**
```markdown
# Release Process

## Creating a New Release

1. Update version in `Cargo.toml`
2. Commit the version change: `git commit -am "chore: bump version to X.Y.Z"`
3. Create git tag: `git tag vX.Y.Z`
4. Push tag: `git push origin vX.Y.Z`
5. Monitor GitHub Actions for workflow completion
6. Review draft release in GitHub Releases page
7. Test downloads if needed
8. Publish release when ready

## Troubleshooting

### Build Failures
- Check Actions logs for specific error
- Verify dependencies are correct
- Try building locally first

### Version Mismatch
- Ensure Cargo.toml version matches tag
- Tag format must be `vX.Y.Z`

### Missing LFS Files
- Verify `.gitattributes` configured correctly
- Check LFS quota hasn't been exceeded
```

**Acceptance Criteria:**
- [ ] Documentation file created
- [ ] Step-by-step instructions clear and accurate
- [ ] Common issues documented with solutions
- [ ] Examples provided for clarity

---

### Task 5.2: Add Workflow Comments and Cleanup
**Priority:** Low
**Estimated Time:** 30 minutes

**Description:**
Add helpful comments to the workflow file and clean up any temporary test artifacts.

**Steps:**
1. Add comments explaining each job and complex steps
2. Remove any debug/test steps no longer needed
3. Ensure consistent formatting
4. Add header comment with workflow description

**Acceptance Criteria:**
- [ ] Workflow file well-commented
- [ ] No unnecessary debug steps
- [ ] Consistent YAML formatting
- [ ] Easy to understand for future maintainers

---

### Task 5.3: Update Project README
**Priority:** Medium
**Estimated Time:** 30 minutes

**Description:**
Update the project README to mention automated releases and how users can download them.

**Steps:**
1. Add "Installation" section with download links
2. Mention platform support (Windows, Linux)
3. Add workflow status badge
4. Link to Releases page

**Example Addition:**
```markdown
## Installation

Pre-built binaries are available for Windows and Linux on the [Releases page](https://github.com/username/quarm_announce/releases).

### Windows
1. Download `quarm_announce-vX.Y.Z-windows-x64.zip`
2. Extract the archive
3. Configure `config.json` with your EverQuest log file path
4. Run `quarm_announce.exe`

### Linux (SteamOS)
1. Download `quarm_announce-vX.Y.Z-linux-x64.tar.gz`
2. Extract: `tar -xzf quarm_announce-vX.Y.Z-linux-x64.tar.gz`
3. Configure `config.json` with your log file path
4. Run: `./quarm_announce`

**Note:** Linux users may need to install dependencies: `sudo apt install libasound2`
```

**Acceptance Criteria:**
- [ ] README includes installation instructions
- [ ] Platform-specific instructions provided
- [ ] Link to releases page included
- [ ] Dependency notes for Linux users

---

## Post-Implementation Tasks

### Optional Enhancements (Future)

These tasks are out of scope for initial release but documented for future consideration:

1. **Pre-release Support** (2 hours)
   - Modify workflow to detect `-beta`, `-rc` tags
   - Mark GitHub releases as "pre-release" automatically

2. **macOS Builds** (4 hours)
   - Add `macos-latest` runner with `x86_64-apple-darwin` target
   - Research cross-compilation for Apple Silicon (`aarch64-apple-darwin`)
   - Package as `.tar.gz` or `.dmg`

3. **Automated Version Bumping** (3 hours)
   - Parse conventional commits to determine version bump (major/minor/patch)
   - Automatically update `Cargo.toml` and commit
   - Create tag programmatically

4. **Static Binary for Linux** (2 hours)
   - Investigate `x86_64-unknown-linux-musl` target
   - Statically link dependencies to eliminate runtime requirements
   - Trade-off: larger binary, but more portable

5. **Code Signing** (4 hours)
   - Set up Windows code signing certificate
   - Sign `.exe` files to avoid SmartScreen warnings
   - Research cost/logistics of certificates

---

## Testing Checklist

Before marking implementation complete, verify:

### Workflow Functionality
- [ ] Workflow triggers on `v*.*.*` tag push
- [ ] Workflow can be manually triggered via workflow_dispatch
- [ ] Version validation catches mismatches
- [ ] Git LFS files downloaded correctly
- [ ] Windows build completes successfully
- [ ] Linux build completes successfully
- [ ] Unit tests run and pass on both platforms
- [ ] Smoke tests execute binaries successfully
- [ ] Windows artifact packaged as `.zip`
- [ ] Linux artifact packaged as `.tar.gz`
- [ ] Changelog generated with commit grouping
- [ ] Draft release created automatically
- [ ] Both archives attached to release
- [ ] Workflow fails if any job fails (no partial releases)

### Artifact Quality
- [ ] Windows `.zip` extracts correctly on Windows 10/11
- [ ] Windows binary executes without errors
- [ ] Linux `.tar.gz` extracts correctly on Ubuntu 22.04
- [ ] Linux binary executes without errors (or lists required deps)
- [ ] Both archives include all required files (binary, config, speakers)
- [ ] ONNX models are full files, not LFS pointers
- [ ] File permissions correct (Linux binary executable)

### Documentation
- [ ] Release process documented for maintainers
- [ ] Installation instructions in README
- [ ] Troubleshooting guide created
- [ ] Workflow file has helpful comments

### Edge Cases
- [ ] Works with fresh clone (no local state required)
- [ ] Handles first release (no previous tag to compare)
- [ ] Handles manual workflow dispatch (no tag)
- [ ] Provides clear errors when failures occur

---

## Success Criteria

Implementation is complete when:

1. ✅ Pushing a `v*.*.*` tag triggers the full release workflow
2. ✅ Workflow builds binaries for Windows and Linux
3. ✅ All tests pass on both platforms
4. ✅ Binaries execute successfully (smoke tests)
5. ✅ Artifacts packaged correctly with all required files
6. ✅ Changelog generated from conventional commits
7. ✅ Draft GitHub release created with both archives attached
8. ✅ Maintainer can review and publish draft release
9. ✅ Process documented for future releases
10. ✅ At least one successful end-to-end test release completed

---

## Notes

- Estimated times are for a developer familiar with GitHub Actions
- Some tasks may take longer if unexpected issues arise (especially ONNX Runtime setup)
- Test frequently on actual GitHub Actions runners (not just local Docker)
- Keep test tags organized: delete test tags after workflow validation
- Consider using a test repository first if nervous about breaking main repo

---

**Last Updated:** 2025-11-13
**Next Review:** After Phase 3 completion (re-estimate remaining time)
