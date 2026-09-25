# docprims Makefile
# GPL-free document text extraction primitives
#
# Quick Reference:
#   make help       - Show all available targets
#   make bootstrap  - Install tools (sfetch -> goneat)
#   make check      - Run all quality checks (fmt, lint, test, deny)
#   make fmt        - Format code (cargo fmt)
#   make build      - Build all crates

.PHONY: all help bootstrap bootstrap-force tools check test fmt lint build clean version install
.PHONY: precommit prepush deps-check audit deny miri msrv fmt-check npm-publish-prereqs-check
.PHONY: build-release build-ffi cbindgen
.PHONY: release-clean release-download release-checksums release-sign release-tag release-push-tag
.PHONY: release-verify-tag release-verify-remote-tag release-insert-anchors
.PHONY: release-export-keys release-verify-checksums release-verify-signatures
.PHONY: release-verify-keys release-notes release-upload release
.PHONY: build-local-go go-test go-test-shared
.PHONY: version-patch version-minor version-major version-set version-sync version-check
.PHONY: release-check release-crates-list release-crates-dry-run release-crates-verify
.PHONY: release-tooling-test release-preflight release-guard-tag-version
.PHONY: check-windows check-windows-msvc check-windows-gnu

# -----------------------------------------------------------------------------
# Configuration
# -----------------------------------------------------------------------------

.NOTPARALLEL: release

# Version from VERSION file (SSOT)
VERSION := $(shell cat VERSION 2>/dev/null || echo "dev")

# Tool installation directory
BIN_DIR := $(CURDIR)/bin

# Pinned tool versions for reproducibility
SFETCH_VERSION := v0.4.11
GONEAT_VERSION ?= v0.6.0

# Tool paths
SFETCH = $(shell [ -x "$(BIN_DIR)/sfetch" ] && echo "$(BIN_DIR)/sfetch" || command -v sfetch 2>/dev/null)
GONEAT = $(shell command -v goneat 2>/dev/null)

# Rust toolchain
CARGO = cargo
# Build against the committed Cargo.lock; fail rather than re-resolve.
CARGO_LOCKED = --locked
# Build every feature (including the `cli` binary) in workspace-wide targets.
CARGO_ALL = --all-features
# Minimum supported Rust version (workspace rust-version); verified by `make msrv`.
MSRV = 1.88.0

# -----------------------------------------------------------------------------
# Default and Help
# -----------------------------------------------------------------------------

all: check

help: ## Show available targets
	@echo "docprims - GPL-free Document Text Extraction"
	@echo ""
	@awk 'BEGIN { FS = ":.*## " } /^[a-zA-Z0-9_-]+:.*## / { printf "  %-26s %s\n", $$1, $$2 }' $(MAKEFILE_LIST)
	@echo ""
	@echo "Current version: $(VERSION)"

# -----------------------------------------------------------------------------
# Bootstrap - Trust Anchor Chain
# -----------------------------------------------------------------------------

bootstrap: ## Install required tools (sfetch -> goneat -> tools)
	@echo "Bootstrapping docprims development environment..."
	@echo ""
	@# Step 0: Verify prerequisites
	@if ! command -v curl >/dev/null 2>&1; then \
		echo "[!!] curl not found (required for bootstrap)"; \
		exit 1; \
	fi
	@echo "[ok] curl found"
	@if ! command -v cargo >/dev/null 2>&1; then \
		echo "[!!] cargo not found (required)"; \
		echo ""; \
		echo "Install Rust toolchain:"; \
		echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"; \
		exit 1; \
	fi
	@echo "[ok] cargo: $$(cargo --version)"
	@echo ""
	@# Step 1: Install sfetch (trust anchor)
	@mkdir -p "$(BIN_DIR)"
	@if [ ! -x "$(BIN_DIR)/sfetch" ] && ! command -v sfetch >/dev/null 2>&1; then \
		echo "[..] Installing sfetch (trust anchor)..."; \
		curl -fsSL https://github.com/3leaps/sfetch/releases/download/$(SFETCH_VERSION)/install-sfetch.sh | bash -s -- --dir "$(BIN_DIR)" --tag $(SFETCH_VERSION) --yes; \
	else \
		echo "[ok] sfetch already installed"; \
	fi
	@# Verify sfetch
	@SFETCH_BIN=""; \
	if [ -x "$(BIN_DIR)/sfetch" ]; then SFETCH_BIN="$(BIN_DIR)/sfetch"; \
	elif command -v sfetch >/dev/null 2>&1; then SFETCH_BIN="$$(command -v sfetch)"; fi; \
	if [ -z "$$SFETCH_BIN" ]; then echo "[!!] sfetch installation failed"; exit 1; fi; \
	echo "[ok] sfetch: $$SFETCH_BIN"
	@echo ""
	@# Step 2: Install goneat via sfetch
	@SFETCH_BIN=""; \
	if [ -x "$(BIN_DIR)/sfetch" ]; then SFETCH_BIN="$(BIN_DIR)/sfetch"; \
	elif command -v sfetch >/dev/null 2>&1; then SFETCH_BIN="$$(command -v sfetch)"; fi; \
	if [ "$(FORCE)" = "1" ] || ! command -v goneat >/dev/null 2>&1; then \
		echo "[..] Installing goneat $(GONEAT_VERSION) via sfetch..."; \
		$$SFETCH_BIN --repo fulmenhq/goneat --tag $(GONEAT_VERSION); \
	else \
		echo "[ok] goneat already installed"; \
	fi
	@if command -v goneat >/dev/null 2>&1; then \
		echo "[ok] goneat: $$(goneat version 2>&1 | head -n1)"; \
	else \
		echo "[!!] goneat installation failed"; exit 1; \
	fi
	@echo ""
	@# Step 3: Install tools via goneat doctor tools
	@echo "[..] Installing tools via goneat..."
	@if command -v goneat >/dev/null 2>&1; then \
		goneat doctor tools --scope rust --install || echo "[--] Some tools may need manual install"; \
	fi
	@echo ""
	@# Step 4: Add Windows cross-check targets (optional, no SDK needed)
	@echo "[..] Adding Windows cross-check targets..."
	@rustup target add x86_64-pc-windows-msvc 2>/dev/null || true
	@rustup target add x86_64-pc-windows-gnu 2>/dev/null || true
	@echo "[ok] Windows targets available (run 'make check-windows' to validate)"
	@echo ""
	@echo "[ok] Bootstrap complete"
	@echo ""
	@echo "Ensure $(BIN_DIR) is in your PATH, or tools will be found automatically."

bootstrap-force: ## Force reinstall all tools
	@$(MAKE) bootstrap FORCE=1

tools: ## Verify external tools are available
	@echo "Verifying tools..."
	@if command -v goneat >/dev/null 2>&1; then \
		echo "[ok] Using goneat doctor tools:"; \
		goneat doctor tools --scope rust; \
	else \
		echo "[--] goneat not found, checking manually..."; \
		if command -v cargo >/dev/null 2>&1; then \
			echo "[ok] cargo: $$(cargo --version)"; \
		else \
			echo "[!!] cargo not found"; \
		fi; \
		if cargo fmt --version >/dev/null 2>&1; then \
			echo "[ok] rustfmt: $$(cargo fmt --version)"; \
		else \
			echo "[!!] rustfmt not found"; \
		fi; \
		if cargo clippy --version >/dev/null 2>&1; then \
			echo "[ok] clippy: $$(cargo clippy --version)"; \
		else \
			echo "[!!] clippy not found"; \
		fi; \
		if command -v cargo-deny >/dev/null 2>&1; then \
			echo "[ok] cargo-deny: $$(cargo-deny --version)"; \
		else \
			echo "[!!] cargo-deny not found"; \
		fi; \
		if command -v cargo-audit >/dev/null 2>&1; then \
			echo "[ok] cargo-audit: $$(cargo-audit --version)"; \
		else \
			echo "[!!] cargo-audit not found"; \
		fi; \
	fi

# -----------------------------------------------------------------------------
# Quality Gates
# -----------------------------------------------------------------------------

check: fmt-check lint test lean-lib-check doc-check deny audit ## Run all quality checks
	@echo "[ok] All quality checks passed"

test: ## Run test suite
	@echo "Running tests..."
	$(CARGO) test --workspace $(CARGO_ALL) $(CARGO_LOCKED)
	@echo "[ok] Tests passed"

fmt: ## Format code (goneat assess or cargo fmt)
	@echo "Formatting..."
	@if command -v goneat >/dev/null 2>&1; then \
		goneat assess --categories format --fix; \
	else \
		echo "[--] goneat not found, using cargo fmt"; \
		$(CARGO) fmt --all; \
	fi
	@echo "[ok] Formatting complete"

fmt-check: ## Check formatting without modifying
	@echo "Checking formatting..."
	@if command -v goneat >/dev/null 2>&1; then \
		goneat assess --categories format; \
	else \
		echo "[--] goneat not found, using cargo fmt --check"; \
		$(CARGO) fmt --all -- --check; \
	fi
	@echo "[ok] Formatting check passed"

lint: ## Run linting (goneat assess + cargo clippy)
	@echo "Linting..."
	@bash scripts/check-npm-trusted-publish-runtime.sh
	@if command -v goneat >/dev/null 2>&1; then \
		goneat assess --categories lint; \
	fi
	@# goneat v0.6.0 does not run the Rust lint pass; clippy runs directly as the gap-filler.
	$(CARGO) clippy --workspace --all-targets $(CARGO_ALL) $(CARGO_LOCKED) -- -D warnings
	@echo "[ok] Linting passed"

npm-publish-prereqs-check: ## Verify npm trusted publishing runtime guard
	@bash scripts/check-npm-trusted-publish-runtime.sh

doc-check: ## Build docs for the published crates as docs.rs does, failing on warnings
	RUSTDOCFLAGS="--cfg docsrs -D warnings" $(CARGO) doc --no-deps $(CARGO_LOCKED) -p docprims --features text,ooxml
	RUSTDOCFLAGS="--cfg docsrs -D warnings" $(CARGO) doc --no-deps $(CARGO_LOCKED) -p docprims-core -p docprims-text -p docprims-ooxml
	@echo "[ok] Documentation builds cleanly"

lean-lib-check: ## Verify library builds of the docprims crate pull no CLI or unused format dependencies
	@bash scripts/check-lean-lib.sh
	@echo "[ok] Lean library dependency check passed"

deny: ## Run cargo-deny license and advisory checks
	@echo "Running cargo-deny..."
	@if command -v cargo-deny >/dev/null 2>&1; then \
		cargo-deny --locked check; \
	else \
		echo "[!!] cargo-deny not found (run 'make bootstrap')"; \
		exit 1; \
	fi
	@echo "[ok] cargo-deny passed"

audit: ## Run cargo-audit security scan
	@echo "Running cargo-audit..."
	@if command -v cargo-audit >/dev/null 2>&1; then \
		cargo-audit audit; \
	else \
		echo "[!!] cargo-audit not found (run 'make bootstrap')"; \
		exit 1; \
	fi
	@echo "[ok] cargo-audit passed"

miri: ## Run Miri to detect undefined behavior in unsafe code (requires nightly)
	@echo "Running Miri..."
	@if rustup run nightly cargo miri --version >/dev/null 2>&1; then \
		rustup run nightly cargo miri test -p docprims-core --lib && \
		rustup run nightly cargo miri test -p docprims-ffi --lib; \
	else \
		echo "[!!] Miri not installed. Install with:"; \
		echo "  rustup +nightly component add miri"; \
		exit 1; \
	fi
	@echo "[ok] Miri passed"

msrv: ## Verify build and tests with the Minimum Supported Rust Version
	@echo "Checking MSRV ($(MSRV))..."
	@if rustup run $(MSRV) cargo --version >/dev/null 2>&1; then \
		CARGO_TARGET_DIR=target/msrv rustup run $(MSRV) cargo build --workspace $(CARGO_ALL) $(CARGO_LOCKED) && \
		CARGO_TARGET_DIR=target/msrv rustup run $(MSRV) cargo test --workspace $(CARGO_ALL) $(CARGO_LOCKED); \
	else \
		echo "[!!] Rust $(MSRV) not installed. Install with:"; \
		echo "  rustup install $(MSRV)"; \
		exit 1; \
	fi
	@echo "[ok] MSRV check passed"

# -----------------------------------------------------------------------------
# Windows Cross-Check (no SDK required)
# See: .plans/active/v0.1.0/07-windows-cross-check-validation.md
# -----------------------------------------------------------------------------

check-windows: check-windows-msvc check-windows-gnu ## Cross-check both Windows targets
	@echo "[ok] Windows cross-check passed"

check-windows-msvc: ## Cross-check Windows MSVC target (type checking only)
	@echo "Cross-checking Windows MSVC target..."
	@if ! rustup target list --installed | grep -q x86_64-pc-windows-msvc; then \
		echo "[..] Installing x86_64-pc-windows-msvc target..."; \
		rustup target add x86_64-pc-windows-msvc; \
	fi
	$(CARGO) check --target x86_64-pc-windows-msvc --workspace $(CARGO_ALL)
	@echo "[ok] Windows MSVC check passed"

check-windows-gnu: ## Cross-check Windows GNU target (type checking only)
	@echo "Cross-checking Windows GNU target..."
	@if ! rustup target list --installed | grep -q x86_64-pc-windows-gnu; then \
		echo "[..] Installing x86_64-pc-windows-gnu target..."; \
		rustup target add x86_64-pc-windows-gnu; \
	fi
	$(CARGO) check --target x86_64-pc-windows-gnu --workspace $(CARGO_ALL)
	@echo "[ok] Windows GNU check passed"

deps-check: ## Check dependencies for cooling violations
	@echo "Checking dev dependencies..."
	@if command -v goneat >/dev/null 2>&1; then \
		goneat dependencies check --cooling-days 7 --dev-deps-only 2>/dev/null || \
		echo "[--] Dependency cooling check not available"; \
	else \
		echo "[--] goneat not found, skipping dependency check"; \
	fi

# -----------------------------------------------------------------------------
# Build
# -----------------------------------------------------------------------------

build: ## Build all crates (debug)
	@echo "Building (debug)..."
	$(CARGO) build --workspace $(CARGO_ALL) $(CARGO_LOCKED)
	@echo "[ok] Build complete"

build-release: ## Build all crates (release)
	@echo "Building (release)..."
	$(CARGO) build --workspace --release $(CARGO_ALL) $(CARGO_LOCKED)
	@echo "[ok] Release build complete"

build-ffi: cbindgen ## Build FFI library with C header
	@echo "Building FFI library..."
	$(CARGO) build --package docprims-ffi --release $(CARGO_LOCKED)
	@echo "[ok] FFI build complete"
	@echo "Library: target/release/libdocprims_ffi.*"
	@echo "Header: ffi/docprims-ffi/docprims.h"

cbindgen: ## Generate C header from FFI crate
	@echo "Generating C header..."
	@if command -v cbindgen >/dev/null 2>&1; then \
		cbindgen --config cbindgen.toml --crate docprims-ffi --output ffi/docprims-ffi/docprims.h; \
		echo "[ok] Generated ffi/docprims-ffi/docprims.h"; \
	else \
		echo "[!!] cbindgen not found (cargo install cbindgen)"; \
		exit 1; \
	fi

clean: ## Remove build artifacts
	@echo "Cleaning..."
	$(CARGO) clean
	@rm -rf bin/
	@echo "[ok] Clean complete"

# -----------------------------------------------------------------------------
# Go Bindings
# -----------------------------------------------------------------------------

GO_BINDINGS_DIR := bindings/go/docprims

build-local-go: cbindgen ## Build FFI for local Go development
	@echo "Building FFI for local Go development..."
	$(CARGO) build --release -p docprims-ffi $(CARGO_LOCKED)
	@# Sync header and local static lib into Go module layout
	@PLATFORM=""; \
	UNAME_S="$$(uname -s)"; \
	UNAME_M="$$(uname -m)"; \
	if [ "$$UNAME_S" = "Darwin" ] && [ "$$UNAME_M" = "arm64" ]; then PLATFORM="darwin-arm64"; \
	elif [ "$$UNAME_S" = "Darwin" ] && [ "$$UNAME_M" = "x86_64" ]; then PLATFORM="darwin-amd64"; \
	elif [ "$$UNAME_S" = "Linux" ] && [ "$$UNAME_M" = "x86_64" ]; then PLATFORM="linux-amd64"; \
	elif [ "$$UNAME_S" = "Linux" ] && [ "$$UNAME_M" = "aarch64" ]; then PLATFORM="linux-arm64"; \
	else PLATFORM="unknown"; fi; \
	if [ "$$PLATFORM" != "unknown" ]; then \
		mkdir -p "bindings/go/docprims/include"; \
		mkdir -p "bindings/go/docprims/lib/local/$$PLATFORM"; \
		mkdir -p "bindings/go/docprims/lib-shared/local/$$PLATFORM"; \
		cp "ffi/docprims-ffi/docprims.h" "bindings/go/docprims/include/docprims.h"; \
		cp "target/release/libdocprims_ffi.a" "bindings/go/docprims/lib/local/$$PLATFORM/libdocprims_ffi.a"; \
		if [ "$$UNAME_S" = "Darwin" ]; then \
			cp "target/release/libdocprims_ffi.dylib" "bindings/go/docprims/lib-shared/local/$$PLATFORM/libdocprims_ffi.dylib"; \
		elif [ "$$UNAME_S" = "Linux" ]; then \
			cp "target/release/libdocprims_ffi.so" "bindings/go/docprims/lib-shared/local/$$PLATFORM/libdocprims_ffi.so"; \
		fi; \
	else \
		echo "[--] Unknown host platform for Go lib sync (uname: $$UNAME_S/$$UNAME_M)"; \
	fi
	@echo "[ok] FFI library built"

go-test: build-local-go ## Run Go binding tests
	@echo "Running Go tests..."
	@cd $(GO_BINDINGS_DIR) && go test -v ./...
	@echo "[ok] Go tests passed"

go-test-shared: build-local-go ## Run Go binding tests (shared library)
	@echo "Running Go tests (shared lib)..."
	@PLATFORM=""; \
	UNAME_S="$$(uname -s)"; \
	UNAME_M="$$(uname -m)"; \
	if [ "$$UNAME_S" = "Darwin" ] && [ "$$UNAME_M" = "arm64" ]; then PLATFORM="darwin-arm64"; \
	elif [ "$$UNAME_S" = "Darwin" ] && [ "$$UNAME_M" = "x86_64" ]; then PLATFORM="darwin-amd64"; \
	elif [ "$$UNAME_S" = "Linux" ] && [ "$$UNAME_M" = "x86_64" ]; then PLATFORM="linux-amd64"; \
	elif [ "$$UNAME_S" = "Linux" ] && [ "$$UNAME_M" = "aarch64" ]; then PLATFORM="linux-arm64"; \
	else PLATFORM="unknown"; fi; \
	if [ "$$PLATFORM" = "unknown" ]; then \
		echo "[!!] Unknown host platform for shared Go test (uname: $$UNAME_S/$$UNAME_M)"; \
		exit 1; \
	fi; \
	DYLD_LIBRARY_PATH="$(CURDIR)/bindings/go/docprims/lib-shared/local/$$PLATFORM" \
	LD_LIBRARY_PATH="$(CURDIR)/bindings/go/docprims/lib-shared/local/$$PLATFORM" \
	cd $(GO_BINDINGS_DIR) && go test -tags docprims_shared -v ./...
	@echo "[ok] Go tests (shared) passed"

# -----------------------------------------------------------------------------
# Install
# -----------------------------------------------------------------------------

INSTALL_BINDIR ?= $(HOME)/.local/bin

install: build-release ## Install docprims binary to INSTALL_BINDIR
	@echo "Installing docprims to $(INSTALL_BINDIR)..."
	@mkdir -p "$(INSTALL_BINDIR)"
	@cp target/release/docprims "$(INSTALL_BINDIR)/docprims"
	@chmod 755 "$(INSTALL_BINDIR)/docprims"
	@echo "[ok] Installed docprims to $(INSTALL_BINDIR)/docprims"

# -----------------------------------------------------------------------------
# Pre-commit / Pre-push Hooks
# -----------------------------------------------------------------------------

precommit: fmt-check lint ## Run pre-commit checks (fast)
	@echo "[ok] Pre-commit checks passed"

prepush: check version-check release-tooling-test ## Run pre-push checks (thorough)
	@echo "[ok] Pre-push checks passed"

# -----------------------------------------------------------------------------
# Release Governance (crates.io, version, tag guard)
# -----------------------------------------------------------------------------

release-check: version-check ## Version consistency + crate package check (does not publish)
	@./scripts/check-packages.sh
	@echo "[ok] Package check passed; cargo publish was not run"

release-crates-list: ## Print the validated crates.io publication order
	@./scripts/release-crates.py list

release-crates-dry-run: ## Dry-run crates.io publishing from the guarded tag
	@./scripts/release-crates-dry-run.sh

release-crates-verify: ## Wait for the released version on crates.io and verify it (CRATE= for one)
	@CRATE="$(CRATE)" ./scripts/release-crates-verify.sh

release-guard-tag-version: ## Validate the canonical release tag
	@./scripts/release-guard-tag-version.sh

release-tooling-test: ## Run release guard, signing, asset, crate, version and notes tests
	@./scripts/release-decernor.test.sh
	@./scripts/release-tag-controls.test.sh
	@./scripts/verify-pinned-tag.test.sh
	@./scripts/release-crates.test.sh
	@./scripts/release-crates-verify.test.sh
	@./scripts/release-guard-tag-version.test.sh
	@./scripts/release-assets.test.sh
	@./scripts/release-github-state.test.sh
	@./scripts/release-safety.test.sh
	@./scripts/check-version.test.sh
	@./scripts/check-release-notes.test.sh
	@echo "[ok] Release tooling tests passed"

release-preflight: ## Verify clean-tree pre-tag requirements
	@./scripts/validate-release-anchors.sh
	@echo "Running release preflight checks..."
	@if [ -n "$$(git status --porcelain 2>/dev/null)" ]; then \
		echo "[!!] Working tree not clean - commit or stash changes first"; \
		git status --short; \
		exit 1; \
	fi
	@$(MAKE) prepush --silent
	@$(MAKE) release-check --silent
	@grep -Eq "^## v$(VERSION) — [0-9]{4}-[0-9]{2}-[0-9]{2}$$" RELEASE_NOTES.md || \
		{ echo "[!!] RELEASE_NOTES.md lacks the exact v$(VERSION) heading"; exit 1; }
	@grep -Eq "^## \[$(VERSION)\] - [0-9]{4}-[0-9]{2}-[0-9]{2}$$" CHANGELOG.md || \
		{ echo "[!!] CHANGELOG.md lacks the [$(VERSION)] heading"; exit 1; }
	@./scripts/check-release-notes.sh "v$(VERSION)"
	@git fetch origin main
	@test "$$(git rev-parse HEAD)" = "$$(git rev-parse origin/main)" || \
		{ echo "[!!] HEAD must equal fetched origin/main"; exit 1; }
	@test -z "$$(git status --porcelain)" || \
		{ echo "[!!] Preflight gates changed the working tree"; exit 1; }
	@echo "[ok] All preflight checks passed - ready to tag v$(VERSION)"

# -----------------------------------------------------------------------------
# Release Ceremony (maintainer, local keys; see RELEASE_CHECKLIST.md)
# -----------------------------------------------------------------------------
#
# CI builds the exact unsigned asset set and opens a draft release on a signed
# tag push. The maintainer signs checksum manifests locally and uploads them.
# Every target takes the tag from DOCPRIMS_RELEASE_TAG only.

RELEASE_DIR := $(CURDIR)/dist/release

release-tag: ## Create and verify a local signed version tag
	@./scripts/release-tag.sh

release-push-tag: ## Publish and verify the signed version tag
	@./scripts/release-push-tag.sh

release-verify-tag: ## Verify the tag using only the committed public pin
	@./scripts/release-verify-tag.sh

release-verify-remote-tag: ## Compare local and remote tag objects and GitHub verification
	@./scripts/release-verify-remote-tag.sh

release-insert-anchors: ## Maintainer-only: generate and review public fingerprint anchors
	@./scripts/release-insert-anchors.sh

release-clean: ## Safely empty the repository release staging directory
	@./scripts/release-clean.sh "$(RELEASE_DIR)"

release-download: ## Download exact unsigned assets from the trusted draft
	@./scripts/download-release-assets.sh "$(RELEASE_DIR)"

release-notes: ## Add the exact per-cut notes to the signable asset set
	@DOCPRIMS_REQUIRE_TAG=1 ./scripts/release-guard-tag-version.sh >/dev/null
	@test -f "docs/releases/$${DOCPRIMS_RELEASE_TAG}.md" || \
		{ echo "[!!] Exact per-cut release notes are missing"; exit 1; }
	@./scripts/validate-release-assets.sh "$(RELEASE_DIR)" base >/dev/null
	@cp "docs/releases/$${DOCPRIMS_RELEASE_TAG}.md" \
		"$(RELEASE_DIR)/release-notes-$${DOCPRIMS_RELEASE_TAG}.md"
	@./scripts/stage-release-anchors.sh "$(RELEASE_DIR)"
	@./scripts/validate-release-assets.sh "$(RELEASE_DIR)" signable >/dev/null
	@echo "[ok] Per-cut release notes added to the signed set"

release-checksums: ## Generate exact SHA256 and SHA512 manifests
	@./scripts/generate-checksums.sh "$(RELEASE_DIR)"

release-sign: ## Sign checksum manifests with local MFA-held keys
	@./scripts/sign-release-assets.sh "$(RELEASE_DIR)"

release-export-keys: ## Export and prove public verification material
	@./scripts/export-release-keys.sh "$(RELEASE_DIR)"

release-verify-checksums: ## Verify exact dual checksum manifests
	@./scripts/verify-checksums.sh "$(RELEASE_DIR)"

release-verify-signatures: ## Verify every configured signature
	@./scripts/verify-signatures.sh "$(RELEASE_DIR)"

release-verify-keys: ## Verify exported public keys against staged anchors
	@./scripts/verify-public-keys.sh "$(RELEASE_DIR)"

release-verify: release-verify-checksums release-verify-signatures release-verify-keys ## Verify signed release set
	@./scripts/validate-release-assets.sh "$(RELEASE_DIR)" signed >/dev/null
	@echo "[ok] Signed release set verified"

release-upload: ## Verify once and update the exact trusted draft release
	@./scripts/upload-release-assets.sh "$(RELEASE_DIR)"

release: release-guard-tag-version ## Run the serialized local signing ceremony
	@DOCPRIMS_REQUIRE_TAG=1 ./scripts/release-guard-tag-version.sh >/dev/null
	@$(MAKE) release-clean
	@$(MAKE) release-download
	@$(MAKE) release-notes
	@$(MAKE) release-checksums
	@$(MAKE) release-sign
	@$(MAKE) release-export-keys
	@$(MAKE) release-upload
	@echo "[ok] Release assets signed and uploaded; GitHub release remains draft"

# -----------------------------------------------------------------------------
# Version Management
# -----------------------------------------------------------------------------

VERSION_FILE := VERSION

version: ## Print current version
	@echo "$(VERSION)"

version-patch: ## Bump patch version (0.1.0 -> 0.1.1)
	@current=$$(tr -d ' \t\r\n' < $(VERSION_FILE)); \
	IFS=. read -r major minor patch <<< "$$current"; \
	echo "$$major.$$minor.$$((patch + 1))" > $(VERSION_FILE); \
	./scripts/version-sync.py

version-minor: ## Bump minor version (0.1.0 -> 0.2.0)
	@current=$$(tr -d ' \t\r\n' < $(VERSION_FILE)); \
	IFS=. read -r major minor patch <<< "$$current"; \
	echo "$$major.$$((minor + 1)).0" > $(VERSION_FILE); \
	./scripts/version-sync.py

version-major: ## Bump major version (0.1.0 -> 1.0.0)
	@current=$$(tr -d ' \t\r\n' < $(VERSION_FILE)); \
	IFS=. read -r major minor patch <<< "$$current"; \
	echo "$$((major + 1)).0.0" > $(VERSION_FILE); \
	./scripts/version-sync.py

version-set: ## Set explicit version (V=X.Y.Z)
	@if [ -z "$(V)" ]; then echo "Usage: make version-set V=1.2.3"; exit 1; fi
	@echo "$(V)" > $(VERSION_FILE)
	@./scripts/version-sync.py

version-sync: ## Write VERSION to Cargo.toml, Cargo.lock and the npm manifests
	@./scripts/version-sync.py

version-check: ## Validate version consistency across Cargo and npm manifests
	@./scripts/check-version.sh
