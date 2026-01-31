# Release Checklist

This document walks maintainers through the build/sign/upload flow for each docprims release.

docprims uses the same high-level release posture as sysprims:

- CI builds unsigned artifacts.
- Maintainers sign checksum manifests locally and upload signatures + public keys.

## Prerequisites

- `gh` CLI authenticated with push access
- `minisign` installed (required)
- `gpg` installed (optional; only if you also PGP sign)
- Signing keys configured and available on your machine

## 1. Pre-Release Preparation

### Code Quality Gates

- [ ] Ensure `main` is clean: `git status` shows no uncommitted changes
- [ ] Run quality gates: `make check`
- [ ] Run full tests: `cargo test --workspace`
- [ ] Verify cargo-deny passes: `cargo deny check`

### Version & Documentation

- [ ] Update `VERSION` file with new semver (e.g. `0.1.1`)
- [ ] Ensure workspace version matches `VERSION` (if applicable)
- [ ] Update `CHANGELOG.md` (if present)

### Commit & Push

- [ ] Commit changes:
  ```bash
  git add -A
  git commit -m "release: prepare vX.Y.Z"
  ```
- [ ] Push to main:
  ```bash
  git push origin main
  ```

- [ ] Verify local/remote sync (required before running workflows):
  ```bash
  git fetch origin
  git log --oneline origin/main..HEAD
  git log --oneline HEAD..origin/main
  ```
  Both commands must show no output.

### Go Bindings Prep (Required)

Go bindings must be fetchable via `go get` without requiring Rust.

- [ ] Run the Go bindings prep workflow:
  ```bash
  VERSION=$(cat VERSION)
  gh workflow run "Go Bindings (Prep)" -f version="${VERSION}"
  ```
- [ ] Review and merge the created PR:
  ```bash
  VERSION=$(cat VERSION)
  gh pr list --search "go-bindings/v${VERSION}" --state open
  gh pr view --web "go-bindings/v${VERSION}"
  ```
- [ ] Confirm the PR updates the required files:
  - `bindings/go/docprims/lib/<platform>/libdocprims_ffi.a` (all platforms)
  - `bindings/go/docprims/lib-shared/<platform>/` (glibc/darwin/windows only; musl is static-only)
  - `bindings/go/docprims/include/docprims.h`
  - `ffi/docprims-ffi/docprims.h`
- [ ] After merge: ensure `main` is green again
- [ ] Test both static and shared modes locally:
  ```bash
  cd bindings/go/docprims
  go test -v ./...                      # static (default)
  make go-test-shared                   # shared (from repo root)
  ```

### Create and Push Tags

Create and push tags that point to the SAME commit (the commit that includes the merged Go bindings PR).

```bash
VERSION=$(cat VERSION)

# Canonical repo tag
git tag -a "v${VERSION}" -m "v${VERSION}: docprims release"

# Go submodule tag (required for subdir Go module)
git tag -a "bindings/go/docprims/v${VERSION}" -m "bindings/go/docprims/v${VERSION}"

git push origin "v${VERSION}" "bindings/go/docprims/v${VERSION}"
```

### CI Verification

- [ ] Wait for GitHub Actions workflows on the tag (release build, if configured)
- [ ] (Recommended) Run Go bindings staged validation:
  - `.github/workflows/go-bindings.yml` (glibc on PR; musl on manual)
- [ ] (Recommended) Validate the tag (includes Go + TypeScript):
  ```bash
  VERSION=$(cat VERSION)
  gh workflow run "Validate Release" -f tag="v${VERSION}"
  ```

### TypeScript N-API Prebuilds (Required for npm)

Run the prebuilds workflow on the release tag to build cross-platform native binaries:

- [ ] Trigger prebuilds workflow on the tag:
  ```bash
  VERSION=$(cat VERSION)
  gh workflow run "TypeScript N-API Prebuilds" --ref "v${VERSION}"
  ```
- [ ] Wait for completion (builds 6 platforms):
  ```bash
  gh run list --workflow="TypeScript N-API Prebuilds" --limit 3
  ```
- [ ] Verify all platforms built successfully

The prebuilds must complete before running the npm publish workflow in Section 3.

## 2. Manual Signing (Local Machine)

docprims signs checksum manifests for release artifacts.

### Set Environment Variables

```bash
export DOCPRIMS_RELEASE_TAG=v$(cat VERSION)

export DOCPRIMS_MINISIGN_KEY=/path/to/docprims.key
export DOCPRIMS_MINISIGN_PUB=/path/to/docprims.pub

# Optional PGP signing
export DOCPRIMS_PGP_KEY_ID="keyid!"
export DOCPRIMS_GPG_HOMEDIR=/path/to/gpg/homedir
```

### Signing Steps

1. Clean previous release artifacts:
   ```bash
   make release-clean
   ```

2. Download artifacts from the GitHub draft release:
   ```bash
   make release-download
   ```

3. Generate checksum manifests:
   ```bash
   make release-checksums
   ```
   Produces: `SHA256SUMS`, `SHA512SUMS`

4. Sign checksum manifests:
   ```bash
   make release-sign
   ```
   Produces: `.minisig` and optional `.asc` signatures

   > **Note (Ghostty users):** If GPG fails with "Screen or window too small", prefix with:
   > ```bash
   > TERM=xterm-256color make release-sign
   > ```

5. Export public keys into the release directory:
   ```bash
   make release-export-keys
   ```
   Produces:
   - `docprims-minisign.pub`
   - `docprims-release-signing-key.asc` (optional)

6. Verify everything before upload:
   ```bash
   make release-verify
   ```

7. Copy release notes (optional):
   ```bash
   make release-notes
   ```

8. Upload signed assets to GitHub and publish the release:
   ```bash
   make release-upload
   ```

## 3. TypeScript npm Publishing

After signing and undrafting the GitHub release, publish to npm.

### Prerequisites

- Prebuilds workflow completed successfully (Section 1)
- GitHub release is public (not draft)
- `publish-npm` environment configured on GitHub (for OIDC trusted publishing)

### Publish via Workflow (Recommended)

- [ ] Run the npm publish workflow from the release tag:
  ```bash
  VERSION=$(cat VERSION)
  gh workflow run "TypeScript npm Publish" --ref "v${VERSION}"
  ```
- [ ] Monitor workflow:
  ```bash
  gh run list --workflow="TypeScript npm Publish" --limit 3
  ```
- [ ] Verify publication:
  ```bash
  VERSION=$(cat VERSION)
  npm view "@3leaps/docprims@${VERSION}" version
  ```

### First-Time Setup (Manual Bootstrap)

For the first publish, the package must be created manually before OIDC publishing works:

```bash
cd bindings/typescript/docprims
npm login
npm publish --access public
```

This publishes with local platform binary only. Run the workflow publish afterward for cross-platform support.

## 4. Post-Release Verification

- [ ] Verify release is public: `gh release view v$(cat VERSION)`
- [ ] Download and verify checksums/signatures using the published public keys
- [ ] Verify npm package: `npm view @3leaps/docprims`
- [ ] Test Go module: `go get github.com/3leaps/docprims/bindings/go/docprims@v$(cat VERSION)`
