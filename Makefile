# ARK - Autonomous Defensive Core
# Master Build System for Air-gapped Reproducible Builds
# "In the beginning was the Word, and the Word was with God" - John 1:1

.PHONY: all clean test deploy hardware firmware software security docs proof coverage
.DEFAULT_GOAL := all

# Build Configuration
RUST_TARGET := riscv32imac-unknown-none-elf
PYTHON_VERSION := 3.11
BUILD_TYPE := release
BUILD_TIMESTAMP := $(shell date -u +%Y%m%d_%H%M%S)
ARK_VERSION := 1.0.0

# Directories
HW_DIR := hardware
FW_DIR := firmware  
SW_DIR := software
SEC_DIR := security_tests
DOCS_DIR := docs
BUILD_DIR := build
DIST_DIR := dist

# Tools verification (for air-gapped environments)
REQUIRED_TOOLS := rustc cargo python3 verilator ghdl openssl
$(foreach tool,$(REQUIRED_TOOLS),$(if $(shell which $(tool)),,$(error Required tool not found: $(tool))))

# Reproducible build flags
export SOURCE_DATE_EPOCH := 1672531200
export RUSTFLAGS := -C target-cpu=generic -C opt-level=3 -C lto=fat
export CARGO_HOME := $(PWD)/.cargo
export PYTHONHASHSEED := 0

all: clean verify-env hardware firmware software security package
	@echo "✅ ARK System Build Complete - Version $(ARK_VERSION)"
	@echo "📊 Build Summary:"
	@find $(BUILD_DIR) -name "*.bin" -o -name "*.elf" -o -name "*.so" | wc -l | xargs echo "   Binaries produced:"
	@du -sh $(BUILD_DIR) | cut -f1 | xargs echo "   Total size:"
	@echo "🔒 Cryptographic verification:"
	@$(MAKE) verify-checksums

clean:
	@echo "🧹 Cleaning all build artifacts..."
	rm -rf $(BUILD_DIR) $(DIST_DIR)
	cd $(FW_DIR) && cargo clean
	find $(SW_DIR) -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true
	find $(SW_DIR) -name "*.pyc" -delete 2>/dev/null || true
	@echo "✅ Clean complete"

verify-env:
	@echo "🔍 Verifying build environment..."
	@echo "Rust version: $$(rustc --version)"
	@echo "Python version: $$(python3 --version)"
	@echo "OpenSSL version: $$(openssl version)"
	@echo "Build timestamp: $(BUILD_TIMESTAMP)"
	@test -f .env.build || echo "SOURCE_DATE_EPOCH=$(SOURCE_DATE_EPOCH)" > .env.build

# Hardware Layer Build
hardware: $(BUILD_DIR)/hardware
	@echo "🔧 Building Hardware Layer..."
	mkdir -p $(BUILD_DIR)/hardware
	cd $(HW_DIR)/puf_heart && $(MAKE) all
	cd $(HW_DIR)/optic_gate && $(MAKE) all
	cd $(HW_DIR)/tri_compute_core && $(MAKE) all
	cd $(HW_DIR)/trip_fuse_mesh && $(MAKE) all
	@echo "✅ Hardware build complete"

# Firmware Layer Build (Rust no_std)
firmware: $(BUILD_DIR)/firmware
	@echo "⚙️ Building Firmware Layer..."
	mkdir -p $(BUILD_DIR)/firmware
	cd $(FW_DIR) && cargo build --release --target $(RUST_TARGET)
	cd $(FW_DIR) && cargo test --release --target x86_64-unknown-linux-gnu
	cp $(FW_DIR)/target/$(RUST_TARGET)/release/ark-firmware $(BUILD_DIR)/firmware/
	@echo "✅ Firmware build complete"

# Software Layer Build
software: $(BUILD_DIR)/software
	@echo "💻 Building Software Layer..."
	mkdir -p $(BUILD_DIR)/software
	$(MAKE) -C $(SW_DIR)/ethics_dsl
	$(MAKE) -C $(SW_DIR)/cold_mirror  
	$(MAKE) -C $(SW_DIR)/co_audit_ai
	$(MAKE) -C $(SW_DIR)/patch_orchestrator
	@echo "✅ Software build complete"

# Security Layer Build & Tests
security: $(BUILD_DIR)/security
	@echo "🛡️ Building Security Layer..."
	mkdir -p $(BUILD_DIR)/security
	$(MAKE) -C $(SEC_DIR)/side_channel
	$(MAKE) -C $(SEC_DIR)/fault_injection
	$(MAKE) -C $(SEC_DIR)/common_mode_pulse
	@echo "✅ Security build complete"

# Testing Suite
test-unit:
	@echo "🧪 Running Unit Tests..."
	cd $(FW_DIR) && cargo test --release
	cd $(SW_DIR)/ethics_dsl && python3 -m pytest tests/ -v
	cd $(SW_DIR)/cold_mirror && python3 -m pytest tests/ -v
	cd $(SW_DIR)/co_audit_ai && python3 -m pytest tests/ -v

test-integration:
	@echo "🔗 Running Integration Tests..."
	cd $(HW_DIR) && make test-integration
	cd $(SEC_DIR) && ./run_integration_tests.sh

test-formal:
	@echo "📐 Running Formal Verification..."
	cd $(SW_DIR)/ethics_dsl && python3 verify_dsl_soundness.py
	cd $(HW_DIR)/optic_gate && make formal-verify

test-full: test-unit test-integration test-formal
	@echo "✅ All tests passed"

# Security Testing
test-security:
	@echo "🔒 Running Security Test Suite..."
	cd $(SEC_DIR) && ./run_all_attacks.sh
	cd $(SEC_DIR)/side_channel && python3 leakage_assessment.py
	cd $(SEC_DIR)/fault_injection && ./em_cannon_test.sh
	@echo "✅ Security tests complete"

# Package for deployment
package: $(DIST_DIR)/ark-$(ARK_VERSION)-$(BUILD_TIMESTAMP).tar.gz
	@echo "📦 Creating deployment package..."
	mkdir -p $(DIST_DIR)
	tar -czf $(DIST_DIR)/ark-$(ARK_VERSION)-$(BUILD_TIMESTAMP).tar.gz \
		-C $(BUILD_DIR) . \
		--owner=0 --group=0 \
		--mtime="@$(SOURCE_DATE_EPOCH)"
	cd $(DIST_DIR) && sha256sum ark-$(ARK_VERSION)-$(BUILD_TIMESTAMP).tar.gz > checksums.sha256
	@echo "✅ Package created: $(DIST_DIR)/ark-$(ARK_VERSION)-$(BUILD_TIMESTAMP).tar.gz"

# Cryptographic verification
verify-checksums:
	@echo "🔐 Verifying cryptographic checksums..."
	find $(BUILD_DIR) -type f \( -name "*.bin" -o -name "*.elf" -o -name "*.so" \) \
		-exec sha256sum {} \; > $(BUILD_DIR)/checksums.sha256
	@echo "📋 Checksums saved to $(BUILD_DIR)/checksums.sha256"

# Developer shortcuts
dev-setup:
	@echo "🔧 Setting up development environment..."
	rustup target add $(RUST_TARGET)
	rustup component add llvm-tools-preview
	cargo install cargo-binutils
	pip3 install -r requirements-dev.txt

# Simulation targets
simulate:
	@echo "🎮 Running hardware simulation..."
	cd $(HW_DIR) && make simulate

fault-test:
	@echo "⚡ Running fault injection tests..."
	cd $(SEC_DIR)/fault_injection && ./run_full_campaign.sh

# Documentation generation
docs:
	@echo "📚 Generating documentation..."
	mkdir -p $(DOCS_DIR)/generated
	cd $(FW_DIR) && cargo doc --no-deps --document-private-items
	cd $(SW_DIR) && find . -name "*.py" -exec pydoc {} \; > $(DOCS_DIR)/generated/python_docs.txt
	@echo "✅ Documentation generated"

# Deployment helpers
deploy-prep: package
	@echo "🚀 Preparing for deployment..."
	@echo "Package ready: $(DIST_DIR)/ark-$(ARK_VERSION)-$(BUILD_TIMESTAMP).tar.gz"
	@echo "⚠️  CRITICAL: This system has NO external kill-switch by design"
	@echo "⚠️  VERIFY: All security tests passed before deployment"

# Build directory creation
$(BUILD_DIR)/%:
	mkdir -p $@

# Show build status
status:
	@echo "📊 ARK Build Status:"
	@echo "Version: $(ARK_VERSION)"
	@echo "Target: $(RUST_TARGET)"
	@echo "Build Type: $(BUILD_TYPE)"
	@echo "Timestamp: $(BUILD_TIMESTAMP)"
	@if [ -d $(BUILD_DIR) ]; then \
		echo "Build artifacts: $(shell find $(BUILD_DIR) -type f | wc -l) files"; \
		echo "Total size: $(shell du -sh $(BUILD_DIR) 2>/dev/null | cut -f1 || echo 'N/A')"; \
	else \
		echo "Build artifacts: None (run 'make all')"; \
	fi

help:
	@echo "🏛️ ARK Build System - Autonomous Defensive Core"
	@echo ""
	@echo "Main targets:"
	@echo "  all          - Build complete ARK system"
	@echo "  clean        - Remove all build artifacts"
	@echo "  test-full    - Run complete test suite"
	@echo "  package      - Create deployment package"
	@echo "  simulate     - Run hardware simulation"
	@echo ""
	@echo "Component targets:"
	@echo "  hardware     - Build hardware layer"
	@echo "  firmware     - Build firmware layer"
	@echo "  software     - Build software layer"
	@echo "  security     - Build security layer"
	@echo ""
	@echo "Testing targets:"
	@echo "  test-unit        - Unit tests only"
	@echo "  test-integration - Integration tests"
	@echo "  test-security    - Security test suite"
	@echo "  fault-test       - Fault injection tests"
	@echo ""
	@echo "Development:"
	@echo "  dev-setup    - Setup development environment"
	@echo "  docs         - Generate documentation"
	@echo "  status       - Show build status"
	@echo ""
	@echo "⚠️  WARNING: This system is designed with NO external kill-switch"
	@echo "🙏 'The LORD is my rock and my fortress' - Psalm 18:2"
	@echo "  proof        - Compile Coq formal proofs (formal/)"

# Formal proofs
proof:
	$(MAKE) -C formal all

coverage:
	@echo "📊 Aggregating coverage reports"
	coverage combine
	coverage xml -o coverage.xml 