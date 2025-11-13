# PRD: Multi-Platform Release Automation via GitHub Actions

**Date:** 2025-11-13
**Status:** Draft
**Author:** Product Management
**Target Audience:** Engineering Team

---

## Introduction/Overview

This PRD defines requirements for implementing automated multi-platform release builds using GitHub Actions. The system will build the `quarm_announce` Rust application for Windows and Linux (SteamOS), package release artifacts with required dependencies, and create GitHub releases with automatically generated changelogs.

**Problem:** Currently, creating releases is a manual process requiring:
- Building the application separately on each platform
- Manually packaging binaries with configuration files and TTS models
- Creating GitHub releases and writing release notes by hand
- Risk of inconsistent builds or missing files across platforms

**Goal:** Fully automate the release process from git tag creation to published GitHub release with downloadable platform-specific archives.

---

## Goals

1. **Eliminate manual release overhead** - Reduce release creation from ~30 minutes to <5 minutes of human time
2. **Ensure build consistency** - Every release built in clean CI environment with reproducible configurations
3. **Support multiple platforms** - Windows (x86_64-pc-windows-msvc) and Linux/SteamOS (x86_64-unknown-linux-gnu) builds
4. **Automate version management** - Sync git tags with Cargo.toml version automatically
5. **Generate accurate changelogs** - Parse conventional commits to create structured release notes
6. **Maintain quality gates** - Run tests and smoke tests before releasing binaries

---

## User Stories

### Story 1: Maintainer Creating a Release
**As a** project maintainer
**I want to** create a new release by pushing a git tag
**So that** I can publish new versions without manual build/packaging steps

**Acceptance Criteria:**
- Push git tag `v1.2.3` triggers full release workflow
- Workflow builds for both Windows and Linux automatically
- Release artifacts appear in GitHub Releases page
- Can review draft release before publishing to users

### Story 2: User Downloading Platform-Specific Build
**As a** Windows user
**I want to** download a `.zip` file with everything needed to run the application
**So that** I can start using it immediately without installing dependencies

**Acceptance Criteria:**
- Windows `.zip` contains: `quarm_announce.exe`, `speakers/` directory with models, `config.json`
- Archive structure matches expected paths (speakers in subdirectory)
- Binary runs successfully on Windows x86_64 systems

### Story 3: SteamOS/Linux User Downloading Build
**As a** SteamOS user
**I want to** download a `.tar.gz` file with a working Linux binary
**So that** I can run the application on my Steam Deck

**Acceptance Criteria:**
- Linux `.tar.gz` contains: `quarm_announce` binary, `speakers/` directory, `config.json`
- Binary is dynamically linked to standard glibc libraries
- Binary has execute permissions preserved

### Story 4: Maintainer Reviewing Changelog
**As a** project maintainer
**I want to** see automatically generated release notes grouped by change type
**So that** I can quickly verify what's included and edit if needed before publishing

**Acceptance Criteria:**
- Changelog includes all commits since last release
- Commits grouped by conventional commit type (feat, fix, chore, etc.)
- Can edit draft release notes before publishing

---

## Functional Requirements

### FR1: Workflow Trigger
**Priority:** Must-Have
The GitHub Actions workflow MUST:
1.1. Trigger when a git tag matching pattern `v*.*.*` is pushed (semantic versioning)
1.2. NOT trigger on regular commits to main branch
1.3. Support manual triggering via workflow_dispatch for testing

**Verification:** Push tag `v0.2.0` → workflow runs; push to main → workflow does not run

---

### FR2: Version Management
**Priority:** Must-Have
The workflow MUST:
2.1. Extract version number from git tag (e.g., `v1.2.3` → `1.2.3`)
2.2. Verify git tag version matches `version` field in `Cargo.toml`
2.3. Fail workflow if versions are mismatched (prevents accidental version drift)

**Verification:** Tag `v0.2.0` with Cargo.toml showing `version = "0.2.0"` → succeeds; mismatched versions → fails with clear error

---

### FR3: Git LFS Handling
**Priority:** Must-Have
The workflow MUST:
3.1. Properly check out Git LFS files during repository checkout
3.2. Successfully download `speakers/en_US-amy-medium.onnx` (63MB) from LFS
3.3. Verify LFS files exist before building

**Technical Detail:** Use `actions/checkout@v4` with `lfs: true` parameter

**Verification:** Built artifacts include full 63MB ONNX model, not LFS pointer file

---

### FR4: Build Matrix - Multi-Platform Compilation
**Priority:** Must-Have
The workflow MUST:
4.1. Build for target `x86_64-pc-windows-msvc` (Windows 64-bit)
4.2. Build for target `x86_64-unknown-linux-gnu` (Linux/SteamOS 64-bit)
4.3. Use `cargo build --release` for optimized binaries
4.4. Execute builds in parallel using GitHub Actions matrix strategy

**Platform-Specific Requirements:**

**Windows Build:**
- Runner: `windows-latest`
- Toolchain: Rust stable with `x86_64-pc-windows-msvc` target
- Expected binary: `target/release/quarm_announce.exe`

**Linux Build:**
- Runner: `ubuntu-latest`
- Toolchain: Rust stable with `x86_64-unknown-linux-gnu` target
- System dependencies: Install required packages for ONNX Runtime and audio libraries
- Expected binary: `target/release/quarm_announce`

**Verification:** Both builds complete successfully; binaries exist at expected paths

---

### FR5: Dependency Installation
**Priority:** Must-Have
The workflow MUST:
5.1. Install all required system dependencies before building
5.2. For Linux builds, install packages: `build-essential`, `libasound2-dev`, `pkg-config`
5.3. Document additional SteamOS-specific dependencies (glibc, make) in workflow comments

**Note:** User reported needing glibc and make on SteamOS, but exact package list unknown. Workflow should install common dependencies and document this limitation.

**Verification:** Linux build completes without missing dependency errors

---

### FR6: Build Verification - Unit Tests
**Priority:** Must-Have
The workflow MUST:
6.1. Run `cargo test` before building release binaries
6.2. Fail entire workflow if any tests fail
6.3. Display test results in workflow logs

**Context:** Project has 14 unit tests covering audio engine and log monitoring

**Verification:** Passing tests → build proceeds; failing test → workflow stops with failure

---

### FR7: Build Verification - Smoke Tests
**Priority:** Must-Have
The workflow MUST:
7.1. Execute each compiled binary with `--help` flag or version check
7.2. Verify binary exits successfully (exit code 0)
7.3. Fail workflow if binary cannot execute

**Purpose:** Ensures binaries are not corrupted and can actually run on target platform

**Implementation Note:** May require minimal runtime environment setup (speakers directory present)

**Verification:** Each binary executes successfully; corrupted binary → workflow fails

---

### FR8: Artifact Packaging
**Priority:** Must-Have
The workflow MUST package artifacts with this structure:

**Windows Archive (`quarm_announce-v{version}-windows-x64.zip`):**
```
quarm_announce.exe
speakers/
  en_US-amy-medium.onnx
  en_US-amy-medium.onnx.json
config.json
```

**Linux Archive (`quarm_announce-v{version}-linux-x64.tar.gz`):**
```
quarm_announce
speakers/
  en_US-amy-medium.onnx
  en_US-amy-medium.onnx.json
config.json
```

**Requirements:**
8.1. Use `.zip` format for Windows artifacts
8.2. Use `.tar.gz` format for Linux artifacts
8.3. Preserve file permissions (especially execute bit on Linux binary)
8.4. Include both ONNX model files from `speakers/` directory
8.5. Include `config.json` as configuration template

**Verification:** Extract archive on target platform → all files present with correct structure

---

### FR9: Changelog Generation
**Priority:** Must-Have
The workflow MUST:
9.1. Generate changelog from commit messages since last release tag
9.2. Parse conventional commit format (`feat:`, `fix:`, `chore:`, etc.)
9.3. Group commits by type (Features, Bug Fixes, Chores, etc.)
9.4. Support manual editing of generated changelog (draft release)

**Suggested Format:**
```markdown
## What's Changed

### Features
- feat: add named pipe support for Zeal integration

### Bug Fixes
- fix: prevent duplicate announcements during buff cascades

### Other Changes
- chore: update dependencies to latest versions

**Full Changelog**: https://github.com/owner/repo/compare/v0.1.0...v0.2.0
```

**Verification:** Changelog includes all commits; grouped correctly; editable before publish

---

### FR10: Release Creation
**Priority:** Must-Have
The workflow MUST:
10.1. Create GitHub release as **draft** (not immediately published)
10.2. Use git tag as release title (e.g., `v1.2.3`)
10.3. Attach generated changelog as release body
10.4. Attach both platform artifacts (Windows `.zip` + Linux `.tar.gz`)
10.5. Set release target to the tagged commit

**Draft Rationale:** Allows maintainer to review artifacts, test downloads, and edit notes before publishing

**Verification:** Draft release created; both artifacts attached; changelog populated; not visible to public yet

---

### FR11: Failure Handling
**Priority:** Must-Have
The workflow MUST:
11.1. Fail entire workflow if ANY platform build fails
11.2. NOT create partial releases (e.g., Windows-only when Linux fails)
11.3. Provide clear error messages indicating which platform/step failed
11.4. Clean up any draft releases if workflow fails mid-execution

**Rationale:** Ensures users always have both platforms available; prevents confusion from incomplete releases

**Verification:** Simulate Linux build failure → no release created; error clearly logged

---

### FR12: Asset Naming Convention
**Priority:** Must-Have
Release assets MUST be named:
12.1. Windows: `quarm_announce-v{version}-windows-x64.zip`
12.2. Linux: `quarm_announce-v{version}-linux-x64.tar.gz`

**Examples:**
- `quarm_announce-v0.2.0-windows-x64.zip`
- `quarm_announce-v1.3.5-linux-x64.tar.gz`

**Verification:** Asset names match pattern; version matches tag

---

## Non-Goals (Out of Scope)

The following are explicitly OUT OF SCOPE for the initial implementation:

1. **macOS Builds** - Not required initially (users can build from source if needed)
2. **Pre-release Versioning** - No support for tags like `v1.2.3-beta.1` or `v2.0.0-rc.1` (future consideration)
3. **Manual Version Input** - Version MUST come from git tag, not manual workflow input
4. **32-bit Builds** - Only 64-bit platforms supported (Windows x64, Linux x64)
5. **Automatic Publishing** - Releases remain drafts for manual review/publishing
6. **Cargo.toml Auto-Updating** - Version in Cargo.toml must be updated manually before tagging (enforced via validation)
7. **Cross-Compilation from Single Runner** - Each platform built natively on respective runner OS
8. **Signed Binaries** - No code signing for Windows executables (future enhancement)
9. **Alternative TTS Models** - Only `en_US-amy-medium` model included in releases
10. **Installation Scripts** - Users extract and run manually (no installers/package managers)

---

## Technical Considerations

### ONNX Runtime Compilation
**Challenge:** The `ort` crate (v2.0.0-rc.9) has complex dependencies on ONNX Runtime native libraries

**Approach:**
- Use GitHub Actions runner default environments (should include ONNX Runtime build dependencies)
- If build fails, investigate `ort` documentation for platform-specific setup
- Consider using `ort`'s download features for pre-built binaries if available

**Fallback:** May need to install ONNX Runtime development libraries explicitly

### SteamOS Dependency Mystery
**Known Issue:** User needed to install `glibc`, `make`, and other packages on SteamOS but doesn't remember complete list

**Mitigation:**
- Install common development dependencies on Linux runner
- Document known requirements in release notes
- Provide troubleshooting guide for users encountering missing library errors

**Future Work:** Test built binary on actual SteamOS/Steam Deck to identify exact requirements

### Git LFS Performance
**Consideration:** 63MB ONNX model must be downloaded on every workflow run

**Impact:** Adds ~10-30 seconds to checkout time depending on GitHub's LFS bandwidth

**Acceptable:** Model is required for functionality; no optimization needed initially

### Audio Library Dependencies
**Linux:** May require ALSA development headers (`libasound2-dev`)
**Windows:** WASAPI built into Windows SDK (no extra deps)

### Changelog Generation Tools
**Options:**
- GitHub's release notes auto-generator API (built-in, simple)
- Third-party action like `mikepenz/release-changelog-builder-action`
- Custom script parsing git log

**Recommendation:** Start with GitHub's auto-generator, enhance if needed

---

## Dependencies & Assumptions

### External Dependencies
1. **GitHub Actions** - Workflow execution platform
2. **GitHub Releases** - Artifact hosting and distribution
3. **Git LFS** - Large file storage for ONNX models
4. **Rust Toolchain** - Stable channel with platform targets
5. **cargo** - Rust build system
6. **System Libraries** - Platform-specific (ALSA, build tools, etc.)

### Assumptions
1. Repository has Git LFS already configured (`.gitattributes` exists)
2. Maintainer will manually update `Cargo.toml` version before tagging
3. Commits follow conventional commit format (at least loosely)
4. GitHub Actions free tier has sufficient minutes for builds (~10-15 minutes per release)
5. Users have ability to extract `.zip` and `.tar.gz` archives on their platforms
6. SteamOS users can install missing system dependencies via `pacman` if needed

### Development Environment Requirements
- Workflow tested on GitHub-hosted runners (ubuntu-latest, windows-latest)
- No self-hosted runners required

---

## Timeline & Milestones

### Phase 1: Research & Setup (Week 1)
- Research ONNX Runtime cross-compilation requirements
- Investigate changelog generation tools
- Create initial workflow YAML file structure

### Phase 2: Core Build Implementation (Week 2)
- Implement Windows build job
- Implement Linux build job
- Set up Git LFS checkout
- Configure dependency installation

### Phase 3: Packaging & Testing (Week 3)
- Create artifact packaging scripts
- Implement smoke tests
- Test workflow end-to-end with test tag

### Phase 4: Release Automation (Week 4)
- Implement changelog generation
- Set up draft release creation
- Add asset upload logic
- Final testing with real release candidate

### Phase 5: Documentation & Handoff (Week 5)
- Document workflow for maintainers
- Create troubleshooting guide
- Write release process runbook

**Total Estimated Time:** 5 weeks (part-time development)

---

## Risk Assessment & Mitigation

### Risk 1: ONNX Runtime Build Failures
**Likelihood:** Medium
**Impact:** High (blocks releases)

**Mitigation:**
- Research `ort` crate documentation thoroughly
- Test builds on GitHub Actions runners early
- Have fallback plan to vendor pre-built ONNX Runtime binaries

### Risk 2: SteamOS Dependency Incompatibility
**Likelihood:** Medium
**Impact:** Medium (Linux users can't run binary)

**Mitigation:**
- Install comprehensive dev dependencies on Linux runner
- Consider static linking with musl target as alternative
- Provide clear error messages and dependency list in release notes

### Risk 3: Git LFS Quota Exhaustion
**Likelihood:** Low
**Impact:** Medium (workflow failures)

**Mitigation:**
- Monitor LFS bandwidth usage
- GitHub free tier includes 1GB/month bandwidth
- Each workflow run downloads ~63MB; ~15 releases/month within quota

### Risk 4: Workflow Execution Time Exceeds Free Tier
**Likelihood:** Low
**Impact:** Low (cost increase)

**Mitigation:**
- Optimize build caching using `Swatinem/rust-cache` action
- Free tier includes 2000 minutes/month
- Each release ~20 minutes; 100 releases/month within quota

### Risk 5: Version Mismatch Between Tag and Cargo.toml
**Likelihood:** Medium
**Impact:** Low (workflow fails, easily fixable)

**Mitigation:**
- Add validation step to compare versions
- Provide clear error message instructing how to fix
- Consider future automation to sync versions

### Risk 6: Binary Doesn't Execute on Target Platform
**Likelihood:** Low
**Impact:** High (broken release)

**Mitigation:**
- Implement smoke tests (FR7)
- Test on actual target platforms before first production release
- Collect user feedback and iterate

---

## Success Metrics

### Primary Metrics
1. **Time to Release:** Reduce from ~30 minutes manual to <5 minutes human time
2. **Build Success Rate:** >95% of triggered workflows complete successfully
3. **Platform Coverage:** 100% of releases include both Windows and Linux builds

### Quality Metrics
4. **Test Pass Rate:** 100% (no releases if tests fail)
5. **Smoke Test Success:** 100% (binaries must execute)
6. **User-Reported Issues:** <5% of downloads report "binary won't run" issues

### Operational Metrics
7. **Workflow Duration:** Complete release workflow in <20 minutes
8. **Artifact Size:** Windows + Linux archives totaling ~140-150MB (includes 63MB model × 2)
9. **Changelog Accuracy:** >90% of commits properly categorized by type

### Measurement Plan
- Monitor GitHub Actions workflow runs
- Track issue reports related to release downloads
- Survey users on download/setup experience

---

## Open Questions

1. **Exact SteamOS Dependencies:** What is the complete list of packages needed on SteamOS/Steam Deck?
   - **Action:** Test binary on actual Steam Deck and document results

2. **ONNX Runtime Static Linking:** Can we statically link ONNX Runtime to eliminate runtime dependencies?
   - **Action:** Research `ort` crate configuration options

3. **Model Size Optimization:** Is 63MB model file necessary or can we use smaller model?
   - **Action:** Test with other Piper voice models for size/quality tradeoff

4. **Automatic Version Bumping:** Should we automate Cargo.toml version updates?
   - **Decision:** Out of scope for v1, revisit if manual process becomes error-prone

5. **Pre-release Testing:** Should we implement automatic deployment to test environment?
   - **Decision:** Out of scope for v1, drafts provide manual testing opportunity

---

## Future Considerations

### Post-V1 Enhancements

1. **Pre-release Support**
   - Support tags like `v1.2.3-beta.1` for testing releases
   - Mark GitHub releases as "pre-release" automatically

2. **macOS Support**
   - Add `x86_64-apple-darwin` and `aarch64-apple-darwin` targets
   - Test on Intel and Apple Silicon Macs

3. **Code Signing**
   - Sign Windows executables to avoid SmartScreen warnings
   - Sign macOS binaries for Gatekeeper compatibility

4. **Automated Version Bumping**
   - Automatically update Cargo.toml version based on conventional commits
   - Commit version change back to repository

5. **Multiple TTS Models**
   - Package multiple voice models or allow users to download separately
   - Reduce default download size

6. **Installation Scripts**
   - Provide PowerShell script for Windows setup
   - Provide bash script for Linux setup
   - Consider system package managers (Chocolatey, AUR, etc.)

7. **Performance Optimizations**
   - Aggressive binary stripping to reduce size
   - Static linking where possible
   - Investigate `musl` target for ultra-portable Linux binary

8. **Continuous Deployment**
   - Automatically publish releases (remove draft step) after confidence builds

9. **Release Metrics Dashboard**
   - Track download counts per platform
   - Monitor user platform distribution
   - Inform future platform prioritization

---

## Appendix A: Example Workflow Trigger

```yaml
name: Release Build

on:
  push:
    tags:
      - 'v*.*.*'
  workflow_dispatch:
```

---

## Appendix B: Example Archive Contents

**Extracted Windows Archive:**
```
quarm_announce-v0.2.0-windows-x64/
├── quarm_announce.exe          (2-5 MB)
├── config.json                 (1 KB)
└── speakers/
    ├── en_US-amy-medium.onnx       (63 MB)
    └── en_US-amy-medium.onnx.json  (5 KB)
```

**Total Windows Archive Size:** ~65-70 MB compressed

---

## Appendix C: Conventional Commit Examples

```
feat: add support for reading Zeal named pipes
fix: prevent duplicate announcements during buff cascades
chore: update ort dependency to v2.0.0-rc.9
docs: update README with SteamOS installation instructions
test: add unit tests for batch deduplication logic
refactor: split audio module into separate file
perf: reduce log polling interval to 50ms
```

These will be parsed and grouped in the changelog generation.

---

## Approval & Sign-off

**Product Manager:** _____________________
**Engineering Lead:** _____________________
**QA Lead:** _____________________
**Release Date:** TBD (after implementation complete)

---

*End of PRD*
