# Implementation Tasks: Multi-Platform Release Automation (Prescriptive)

**Date:** 2025-11-13
**Related PRD:** [prd-multi-platform-release.md](./2025-11-13_github-actions-release-prd.md)
**Status:** Ready for Implementation
**Target Audience:** Junior Developer

---

## Relevant Files

- `.github/workflows/release.yml` - Main GitHub Actions workflow file for automated releases
- `docs/RELEASING.md` - Documentation for maintainers on how to create releases
- `README.md` - Project README (will be updated with installation instructions)
- `.gitattributes` - Git LFS configuration (already exists, may need verification)
- `Cargo.toml` - Contains version field that must match git tags

### Notes

- This is a DevOps/CI/CD implementation focused on GitHub Actions workflow configuration
- No application code changes required
- Testing will be performed by triggering the workflow with test tags
- Workflow will be tested incrementally as each job is implemented

---

## Tasks

- [ ] 1.0 **Workflow Infrastructure Setup**
  - [x] 1.1 Create `.github/workflows/` directory if it doesn't exist using `mkdir -p .github/workflows`
  - [x] 1.2 Create empty `release.yml` file in `.github/workflows/` directory
  - [x] 1.3 Add workflow metadata: name (`Multi-Platform Release Build`), and description comment at top of file
  - [x] 1.4 Configure `on:` triggers section with `push.tags: ['v*.*.*']` pattern for semantic version tags
  - [x] 1.5 Add `workflow_dispatch:` trigger to allow manual workflow execution from GitHub UI
  - [x] 1.6 Define `permissions:` section with `contents: write` (needed for creating releases) and `actions: read`
  - [x] 1.7 Create job structure outline with placeholder jobs: `validate-version`, `build-windows`, `build-linux`, `package-artifacts`, `create-release`
  - [x] 1.8 Set up job dependency chain using `needs:` keyword (build jobs need validate-version, package needs build jobs, release needs package)
  - [x] 1.9 Add workflow-level environment variables if needed (e.g., `CARGO_TERM_COLOR: always`)
  - [ ] 1.10 Test workflow triggers by pushing test tag `v0.0.0-test` and verifying workflow appears in Actions tab
  - **Acceptance Criteria**: Workflow file exists, triggers on tag push, shows up in GitHub Actions UI, job dependencies configured correctly

- [ ] 2.0 **Version Management and Validation**
  - [ ] 2.1 Add `validate-version` job running on `ubuntu-latest` runner
  - [ ] 2.2 Add checkout step using `actions/checkout@v4` (without LFS, just need Cargo.toml)
  - [ ] 2.3 Extract version from git tag: use `${{ github.ref_name }}` to get tag name (e.g., `v1.2.3`)
  - [ ] 2.4 Strip 'v' prefix from tag using shell parameter expansion or `sed`: `TAG_VERSION="${GITHUB_REF_NAME#v}"`
  - [ ] 2.5 Extract version from `Cargo.toml` using `cargo metadata --format-version 1 | jq -r '.packages[0].version'`
  - [ ] 2.6 Store both versions in environment variables: `TAG_VERSION` and `CARGO_VERSION`
  - [ ] 2.7 Compare versions using shell conditional: `if [ "$TAG_VERSION" != "$CARGO_VERSION" ]; then echo "Error: Version mismatch"; exit 1; fi`
  - [ ] 2.8 Add clear error message that includes both versions and instructs how to fix (update Cargo.toml)
  - [ ] 2.9 Output validated version using `echo "version=$TAG_VERSION" >> $GITHUB_OUTPUT` with step `id: version`
  - [ ] 2.10 Add `outputs:` section to job definition to expose `version` to other jobs
  - [ ] 2.11 Test with matching versions (should pass) and mismatched versions (should fail with clear error)
  - **Acceptance Criteria**: Job extracts both versions, compares correctly, fails on mismatch with helpful error, outputs version for downstream jobs

- [ ] 3.0 **Repository Checkout and LFS Configuration**
  - [ ] 3.1 Add checkout step to all build jobs using `actions/checkout@v4` with `lfs: true` parameter
  - [ ] 3.2 Add `lfs: true` specifically (not just default checkout) to ensure Git LFS files download
  - [ ] 3.3 Create verification step named "Verify Git LFS files" that runs after checkout
  - [ ] 3.4 Check if `speakers/en_US-amy-medium.onnx` file exists using `test -f` or `[ -f ]`
  - [ ] 3.5 Get file size of ONNX model using `stat` command (platform-specific: `stat -f%z` on macOS, `stat -c%s` on Linux)
  - [ ] 3.6 Add size verification: file must be at least 60,000,000 bytes (60MB) to ensure it's not an LFS pointer
  - [ ] 3.7 Add conditional to fail job if file doesn't exist or is too small: `if [ $SIZE -lt 60000000 ]; then exit 1; fi`
  - [ ] 3.8 Add success message when verification passes: `echo "✓ LFS files verified: ONNX model is $SIZE bytes"`
  - [ ] 3.9 Also verify JSON config file exists: `test -f speakers/en_US-amy-medium.onnx.json`
  - [ ] 3.10 Test by temporarily breaking LFS and verifying workflow fails gracefully
  - **Acceptance Criteria**: All build jobs download 63MB ONNX model (not pointer), verification step catches missing/incomplete LFS files, clear error messages on failure

- [ ] 4.0 **Multi-Platform Build Jobs**
  - [ ] 4.1 Create `build-windows` job with `runs-on: windows-latest`
  - [ ] 4.2 Add `needs: validate-version` dependency to Windows build job
  - [ ] 4.3 Add checkout step with LFS (reuse pattern from task 3.0)
  - [ ] 4.4 Add LFS verification step to Windows job
  - [ ] 4.5 Install Rust toolchain on Windows using `dtolnay/rust-toolchain@stable` action
  - [ ] 4.6 Specify target `x86_64-pc-windows-msvc` in toolchain action using `targets:` parameter
  - [ ] 4.7 Add Rust dependency caching using `Swatinem/rust-cache@v2` action (no configuration needed)
  - [ ] 4.8 Add step "Run unit tests" that executes `cargo test --verbose`
  - [ ] 4.9 Add step "Build release binary" that executes `cargo build --release --verbose`
  - [ ] 4.10 Add step "Verify binary exists" that checks for `target/release/quarm_announce.exe`
  - [ ] 4.11 Add smoke test step that runs `./target/release/quarm_announce.exe --version` (or `--help` if no version flag)
  - [ ] 4.12 If binary doesn't support --version, use PowerShell to check it executes: `if (Test-Path target/release/quarm_announce.exe) { exit 0 } else { exit 1 }`
  - [ ] 4.13 Upload Windows binary as artifact using `actions/upload-artifact@v4` with name `windows-binary` and path `target/release/quarm_announce.exe`
  - [ ] 4.14 Create `build-linux` job with `runs-on: ubuntu-latest`
  - [ ] 4.15 Add `needs: validate-version` dependency to Linux build job
  - [ ] 4.16 Add checkout step with LFS to Linux job
  - [ ] 4.17 Add LFS verification step to Linux job
  - [ ] 4.18 Add step "Install system dependencies" that runs `sudo apt-get update && sudo apt-get install -y build-essential pkg-config libasound2-dev`
  - [ ] 4.19 Add comment in workflow noting SteamOS may need additional packages: glibc, make (documented as known limitation)
  - [ ] 4.20 Install Rust toolchain on Linux using `dtolnay/rust-toolchain@stable` action
  - [ ] 4.21 Specify target `x86_64-unknown-linux-gnu` in toolchain action
  - [ ] 4.22 Add Rust dependency caching using `Swatinem/rust-cache@v2` action
  - [ ] 4.23 Add step "Run unit tests" that executes `cargo test --verbose`
  - [ ] 4.24 Add step "Build release binary" that executes `cargo build --release --verbose`
  - [ ] 4.25 Add step "Verify binary exists" that checks for `target/release/quarm_announce`
  - [ ] 4.26 Add smoke test step that runs `./target/release/quarm_announce --version` and verifies exit code 0
  - [ ] 4.27 Add step to verify binary is ELF format using `file target/release/quarm_announce` (should show "ELF 64-bit")
  - [ ] 4.28 Upload Linux binary as artifact using `actions/upload-artifact@v4` with name `linux-binary` and path `target/release/quarm_announce`
  - [ ] 4.29 Test Windows job independently by temporarily commenting out other jobs
  - [ ] 4.30 Test Linux job independently by temporarily commenting out other jobs
  - [ ] 4.31 Verify both jobs run in parallel (not sequential) by checking workflow run timeline
  - **Acceptance Criteria**: Windows builds .exe successfully, Linux builds ELF binary successfully, all 14 tests pass on both platforms, smoke tests execute binaries, artifacts uploaded, builds run in parallel

- [ ] 5.0 **Artifact Packaging**
  - [ ] 5.1 Create `package-artifacts` job with `runs-on: ubuntu-latest`
  - [ ] 5.2 Add `needs: [validate-version, build-windows, build-linux]` dependencies
  - [ ] 5.3 Add checkout step (need `config.json` and `speakers/` directory) with LFS enabled
  - [ ] 5.4 Download Windows binary artifact using `actions/download-artifact@v4` with name `windows-binary` to path `./artifacts/windows/`
  - [ ] 5.5 Download Linux binary artifact using `actions/download-artifact@v4` with name `linux-binary` to path `./artifacts/linux/`
  - [ ] 5.6 Add step "Display artifact structure" to debug: `ls -la artifacts/`
  - [ ] 5.7 Create Windows staging directory: `mkdir -p staging-windows`
  - [ ] 5.8 Copy Windows binary to staging: `cp artifacts/windows/quarm_announce.exe staging-windows/`
  - [ ] 5.9 Copy config.json to staging: `cp config.json staging-windows/`
  - [ ] 5.10 Copy speakers directory to staging: `cp -r speakers staging-windows/`
  - [ ] 5.11 Verify staging directory structure matches requirements (quarm_announce.exe, config.json, speakers/ with both files)
  - [ ] 5.12 Create Windows .zip archive with version in filename: `cd staging-windows && zip -r ../quarm_announce-v${{ needs.validate-version.outputs.version }}-windows-x64.zip . && cd ..`
  - [ ] 5.13 Upload Windows package artifact using `actions/upload-artifact@v4` with name `windows-package` and path `quarm_announce-v*.zip`
  - [ ] 5.14 Create Linux staging directory: `mkdir -p staging-linux`
  - [ ] 5.15 Copy Linux binary to staging: `cp artifacts/linux/quarm_announce staging-linux/`
  - [ ] 5.16 Set executable permissions on binary: `chmod +x staging-linux/quarm_announce`
  - [ ] 5.17 Copy config.json to staging: `cp config.json staging-linux/`
  - [ ] 5.18 Copy speakers directory to staging: `cp -r speakers staging-linux/`
  - [ ] 5.19 Verify staging directory structure matches requirements
  - [ ] 5.20 Create Linux .tar.gz archive preserving permissions: `tar -czf quarm_announce-v${{ needs.validate-version.outputs.version }}-linux-x64.tar.gz -C staging-linux .`
  - [ ] 5.21 Upload Linux package artifact using `actions/upload-artifact@v4` with name `linux-package` and path `quarm_announce-v*.tar.gz`
  - [ ] 5.22 Add step to display final package sizes: `ls -lh quarm_announce-v*.zip quarm_announce-v*.tar.gz`
  - [ ] 5.23 Test package job by downloading artifacts and manually extracting to verify structure
  - **Acceptance Criteria**: Both archives created with correct filenames, Windows .zip contains all files in flat structure with speakers subdirectory, Linux .tar.gz preserves execute permissions, archives are approximately 65-70MB each

- [ ] 6.0 **Release Creation and Changelog**
  - [ ] 6.1 Create `create-release` job with `runs-on: ubuntu-latest`
  - [ ] 6.2 Add `needs: [validate-version, package-artifacts]` dependencies
  - [ ] 6.3 Add `if: startsWith(github.ref, 'refs/tags/v')` condition to only run on tag pushes (not manual dispatch)
  - [ ] 6.4 Add checkout step (minimal, just for context)
  - [ ] 6.5 Add step "Generate changelog" using GitHub API's built-in release notes generator
  - [ ] 6.6 Use `actions/github-script@v7` action to call `github.rest.repos.generateReleaseNotes()`
  - [ ] 6.7 Configure generateReleaseNotes with `owner: context.repo.owner`, `repo: context.repo.repo`, `tag_name: github.ref_name`
  - [ ] 6.8 Store generated changelog in step output with `id: changelog` and return `release.body`
  - [ ] 6.9 Add step to save changelog to file: `echo "${{ steps.changelog.outputs.result }}" > changelog.md`
  - [ ] 6.10 Add step to display changelog preview in logs: `cat changelog.md`
  - [ ] 6.11 Download all package artifacts using `actions/download-artifact@v4` to `./release-artifacts/` directory
  - [ ] 6.12 Add step "Display release artifacts" to verify: `ls -R ./release-artifacts/`
  - [ ] 6.13 Create draft release using `softprops/action-gh-release@v2` action
  - [ ] 6.14 Configure release action with `draft: true` to create draft (not published)
  - [ ] 6.15 Set release `name:` to `${{ github.ref_name }}` (e.g., "v1.2.3")
  - [ ] 6.16 Set release `body_path:` to changelog.md or use `body:` with `${{ steps.changelog.outputs.result }}`
  - [ ] 6.17 Set release `files:` to glob pattern matching both archives: `./release-artifacts/**/*.{zip,tar.gz}`
  - [ ] 6.18 Set `GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}` in env for release action
  - [ ] 6.19 Verify release `tag_name:` is set correctly (should default to trigger tag)
  - [ ] 6.20 Test by triggering workflow with test tag and verifying draft release appears in GitHub Releases page
  - [ ] 6.21 Verify both archives attached as release assets with correct names
  - [ ] 6.22 Verify changelog displays properly in release body with conventional commit grouping
  - **Acceptance Criteria**: Draft release created automatically, both archives attached as assets, changelog generated with commits grouped by type, release title matches tag, release not published (draft state), accessible from GitHub Releases page

- [ ] 7.0 **Documentation and End-to-End Testing**
  - [ ] 7.1 Create `docs/RELEASING.md` file
  - [ ] 7.2 Add "Release Process" section with step-by-step instructions: 1) Update Cargo.toml version, 2) Commit version change, 3) Create git tag, 4) Push tag, 5) Monitor workflow, 6) Review draft release, 7) Publish release
  - [ ] 7.3 Add "Troubleshooting" section documenting common issues: version mismatches, LFS failures, build errors, missing dependencies
  - [ ] 7.4 Include examples of error messages and their solutions
  - [ ] 7.5 Add "Testing Releases" section explaining how to use test tags: `v0.0.0-test` pattern, how to delete test tags
  - [ ] 7.6 Document manual workflow dispatch usage for testing without creating releases
  - [ ] 7.7 Update `README.md` with new "Installation" section
  - [ ] 7.8 Add subsection for Windows: download link placeholder, extraction instructions, configuration steps
  - [ ] 7.9 Add subsection for Linux/SteamOS: download link placeholder, extraction with `tar -xzf`, configuration, note about potential missing dependencies
  - [ ] 7.10 Add note for Linux users about installing dependencies: `sudo apt install libasound2` (for Ubuntu/Debian)
  - [ ] 7.11 Link to Releases page in README: `[Releases page](https://github.com/username/quarm_announce/releases)`
  - [ ] 7.12 Optionally add GitHub Actions workflow status badge to README: `![Release Build](https://github.com/username/repo/actions/workflows/release.yml/badge.svg)`
  - [ ] 7.13 Perform end-to-end test: Update Cargo.toml to version `0.2.0` (or next appropriate version)
  - [ ] 7.14 Commit version change: `git commit -am "chore: bump version to 0.2.0"`
  - [ ] 7.15 Create git tag: `git tag v0.2.0`
  - [ ] 7.16 Push tag to trigger workflow: `git push origin v0.2.0`
  - [ ] 7.17 Monitor workflow execution in GitHub Actions tab, verify all jobs complete successfully
  - [ ] 7.18 Navigate to Releases page and verify draft release created
  - [ ] 7.19 Download Windows .zip archive and extract locally
  - [ ] 7.20 Verify Windows archive contents: quarm_announce.exe, config.json, speakers/ directory with both model files
  - [ ] 7.21 Test Windows binary on Windows 10/11 system (run `quarm_announce.exe --version` or similar)
  - [ ] 7.22 Download Linux .tar.gz archive and extract locally
  - [ ] 7.23 Verify Linux archive contents: quarm_announce binary (with execute permissions), config.json, speakers/ directory
  - [ ] 7.24 Test Linux binary on Ubuntu 22.04+ system or Docker container
  - [ ] 7.25 Review changelog in release body, verify commits are grouped correctly
  - [ ] 7.26 Edit release notes if needed (add any manual notes, formatting improvements)
  - [ ] 7.27 Publish the release if test successful, or delete draft if issues found
  - [ ] 7.28 Document any issues encountered during testing in RELEASING.md troubleshooting section
  - [ ] 7.29 Add workflow comments and cleanup: ensure all steps have clear names, remove any debug steps, add helpful comments
  - [ ] 7.30 Verify workflow is well-documented and maintainable for future updates
  - **Acceptance Criteria**: RELEASING.md created with clear instructions, README updated with installation instructions for both platforms, end-to-end test completes successfully, draft release created with working binaries for both platforms, documentation covers troubleshooting common issues

---

## Implementation Notes

### Workflow Execution Flow

The final workflow will execute in this sequence:

1. **Trigger**: Git tag `v*.*.*` pushed or manual workflow_dispatch
2. **Validate Version**: Extract tag version, compare with Cargo.toml
3. **Parallel Build Jobs**:
   - **Windows**: Checkout (LFS) → Install Rust → Run tests → Build → Smoke test → Upload artifact
   - **Linux**: Checkout (LFS) → Install deps → Install Rust → Run tests → Build → Smoke test → Upload artifact
4. **Package Artifacts**: Download build artifacts → Package as .zip/.tar.gz → Upload packages
5. **Create Release**: Generate changelog → Create draft release → Attach archives

### Estimated Timeline

- **Tasks 1.0-3.0**: 4-5 hours (workflow setup and validation)
- **Task 4.0**: 6-8 hours (build jobs are the most complex)
- **Task 5.0**: 3-4 hours (packaging logic)
- **Task 6.0**: 2-3 hours (release creation)
- **Task 7.0**: 2-3 hours (documentation and testing)

**Total Estimated Time**: ~20-25 hours over 2-3 weeks (part-time)

### Testing Strategy

- Test each job independently by commenting out dependent jobs
- Use test tags like `v0.0.1-test` for workflow validation
- Delete test tags after validation: `git push origin :refs/tags/v0.0.1-test`
- Verify artifacts manually before first production release

### Critical Success Criteria

- [ ] Workflow triggers only on `v*.*.*` tags
- [ ] Both Windows and Linux builds succeed
- [ ] All 14 unit tests pass on both platforms
- [ ] Smoke tests execute binaries successfully
- [ ] Archives contain all required files (binary, config, speakers)
- [ ] Draft release created with both archives attached
- [ ] Changelog generated with grouped commits

---
