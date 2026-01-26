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
.PHONY: precommit prepush deps-check audit deny miri msrv fmt-check
.PHONY: build-release build-ffi cbindgen
.PHONY: build-local-go go-test
.PHONY: version-patch version-minor version-major version-set version-sync
.PHONY: check-windows check-windows-msvc check-windows-gnu

# -----------------------------------------------------------------------------
# Configuration
# -----------------------------------------------------------------------------

# Version from VERSION file (SSOT)
VERSION := $(shell cat VERSION 2>/dev/null || echo "dev")

# Tool installation directory
BIN_DIR := $(CURDIR)/bin

# Pinned tool versions for reproducibility
SFETCH_VERSION := latest
GONEAT_VERSION ?= v0.5.1

# Tool paths
SFETCH = $(shell [ -x "$(BIN_DIR)/sfetch" ] && echo "$(BIN_DIR)/sfetch" || command -v sfetch 2>/dev/null)
GONEAT = $(shell command -v goneat 2>/dev/null)

# Rust toolchain
CARGO = cargo

# -----------------------------------------------------------------------------
# Default and Help
# -----------------------------------------------------------------------------

all: check

help: ## Show available targets
	@echo "docprims - GPL-free Document Text Extraction"
	@echo "Extract text from documents without license contamination."
	@echo ""
	@echo "Development:"
	@echo "  help            Show this help message"
	@echo "  bootstrap       Install tools (sfetch -> goneat)"
	@echo "  build           Build all crates (debug)"
	@echo "  build-release   Build all crates (release)"
	@echo "  build-ffi       Build FFI library with C header"
	@echo "  install         Install docprims binary to ~/.local/bin"
	@echo "  clean           Remove build artifacts"
	@echo ""
	@echo "Go bindings:"
	@echo "  build-local-go  Build FFI for local Go development"
	@echo "  go-test         Run Go binding tests"
	@echo ""
	@echo "Quality gates:"
	@echo "  check           Run all quality checks (fmt, lint, test, deny)"
	@echo "  test            Run test suite"
	@echo "  fmt             Format code (cargo fmt)"
	@echo "  lint            Run linting (cargo clippy)"
	@echo "  precommit       Pre-commit checks (fast: fmt, clippy)"
	@echo "  prepush         Pre-push checks (thorough: fmt, clippy, test, deny)"
	@echo "  deny            Run cargo-deny license and advisory checks"
	@echo "  audit           Run cargo-audit security scan"
	@echo "  miri            Run Miri UB detection on unsafe code (nightly)"
	@echo "  msrv            Verify build with MSRV (Rust 1.85)"
	@echo "  check-windows   Cross-check Windows targets (no SDK required)"
	@echo ""
	@echo "Version management:"
	@echo "  version         Print current version"
	@echo "  version-patch   Bump patch version (0.1.0 -> 0.1.1)"
	@echo "  version-minor   Bump minor version (0.1.0 -> 0.2.0)"
	@echo "  version-major   Bump major version (0.1.0 -> 1.0.0)"
	@echo "  version-set     Set explicit version (V=X.Y.Z)"
	@echo "  version-sync    Sync VERSION to Cargo.toml"
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
		curl -fsSL https://github.com/3leaps/sfetch/releases/download/$(SFETCH_VERSION)/install-sfetch.sh | bash -s -- --dest "$(BIN_DIR)"; \
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

check: fmt-check lint test deny ## Run all quality checks
	@echo "[ok] All quality checks passed"

test: ## Run test suite
	@echo "Running tests..."
	$(CARGO) test --workspace
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

lint: ## Run linting (goneat assess or cargo clippy)
	@echo "Linting..."
	@if command -v goneat >/dev/null 2>&1; then \
		goneat assess --categories lint; \
	else \
		echo "[--] goneat not found, using cargo clippy"; \
		$(CARGO) clippy --workspace --all-targets -- -D warnings; \
	fi
	@echo "[ok] Linting passed"

deny: ## Run cargo-deny license and advisory checks
	@echo "Running cargo-deny..."
	@if command -v cargo-deny >/dev/null 2>&1; then \
		cargo-deny check; \
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

msrv: ## Verify build with Minimum Supported Rust Version (1.85)
	@echo "Checking MSRV (1.85)..."
	@if rustup run 1.85 cargo --version >/dev/null 2>&1; then \
		rustup run 1.85 cargo build --workspace && \
		rustup run 1.85 cargo test --workspace; \
	else \
		echo "[!!] Rust 1.85 not installed. Install with:"; \
		echo "  rustup install 1.85"; \
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
	$(CARGO) check --target x86_64-pc-windows-msvc --workspace
	@echo "[ok] Windows MSVC check passed"

check-windows-gnu: ## Cross-check Windows GNU target (type checking only)
	@echo "Cross-checking Windows GNU target..."
	@if ! rustup target list --installed | grep -q x86_64-pc-windows-gnu; then \
		echo "[..] Installing x86_64-pc-windows-gnu target..."; \
		rustup target add x86_64-pc-windows-gnu; \
	fi
	$(CARGO) check --target x86_64-pc-windows-gnu --workspace
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
	$(CARGO) build --workspace
	@echo "[ok] Build complete"

build-release: ## Build all crates (release)
	@echo "Building (release)..."
	$(CARGO) build --workspace --release
	@echo "[ok] Release build complete"

build-ffi: cbindgen ## Build FFI library with C header
	@echo "Building FFI library..."
	$(CARGO) build --package docprims-ffi --release
	@echo "[ok] FFI build complete"
	@echo "Library: target/release/libdocprims.*"
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

build-local-go: ## Build FFI for local Go development
	@echo "Building FFI for local Go development..."
	$(CARGO) build --release -p docprims-ffi
	@echo "[ok] FFI library built"

go-test: build-local-go ## Run Go binding tests
	@echo "Running Go tests..."
	cd $(GO_BINDINGS_DIR) && go test -v ./...
	@echo "[ok] Go tests passed"

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

prepush: check ## Run pre-push checks (thorough)
	@echo "[ok] Pre-push checks passed"

# -----------------------------------------------------------------------------
# Version Management
# -----------------------------------------------------------------------------

VERSION_FILE := VERSION

version: ## Print current version
	@echo "$(VERSION)"

version-patch: ## Bump patch version (0.1.0 -> 0.1.1)
	@current=$$(cat $(VERSION_FILE)); \
	major=$$(echo $$current | cut -d. -f1); \
	minor=$$(echo $$current | cut -d. -f2); \
	patch=$$(echo $$current | cut -d. -f3); \
	new_patch=$$((patch + 1)); \
	new_version="$$major.$$minor.$$new_patch"; \
	echo "$$new_version" > $(VERSION_FILE); \
	echo "Version bumped: $$current -> $$new_version"

version-minor: ## Bump minor version (0.1.0 -> 0.2.0)
	@current=$$(cat $(VERSION_FILE)); \
	major=$$(echo $$current | cut -d. -f1); \
	minor=$$(echo $$current | cut -d. -f2); \
	new_minor=$$((minor + 1)); \
	new_version="$$major.$$new_minor.0"; \
	echo "$$new_version" > $(VERSION_FILE); \
	echo "Version bumped: $$current -> $$new_version"

version-major: ## Bump major version (0.1.0 -> 1.0.0)
	@current=$$(cat $(VERSION_FILE)); \
	major=$$(echo $$current | cut -d. -f1); \
	new_major=$$((major + 1)); \
	new_version="$$new_major.0.0"; \
	echo "$$new_version" > $(VERSION_FILE); \
	echo "Version bumped: $$current -> $$new_version"

version-set: ## Set explicit version (V=X.Y.Z)
	@if [ -z "$(V)" ]; then \
		echo "Usage: make version-set V=1.2.3"; \
		exit 1; \
	fi
	@echo "$(V)" > $(VERSION_FILE)
	@echo "Version set to $(V)"

version-sync: ## Sync VERSION file to Cargo.toml (requires cargo-edit)
	@ver=$$(cat $(VERSION_FILE)); \
	if command -v cargo-set-version >/dev/null 2>&1; then \
		cargo set-version --workspace "$$ver"; \
		echo "[ok] Synced Cargo.toml to $$ver"; \
	else \
		echo "[!!] cargo-edit not installed (cargo install cargo-edit)"; \
		echo "Manual update required: set version = \"$$ver\" in Cargo.toml"; \
	fi
