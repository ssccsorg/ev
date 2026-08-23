.PHONY: build test lint fmt check coverage clean

build:
	cargo build --release

test:
	cargo test --release

lint:
	cargo clippy --all-targets -- -D warnings

fmt:
	cargo fmt --all

check:
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings
	cargo build --release
	cargo test --release

# Code coverage gate (cargo-llvm-cov via scripts/coverage.sh). The Spike,
# Yosys, and simulation backends are exercised with the instrumented binary
# (local tools or the ev image), then merged into the report. Thresholds:
# 80% lines and 80% regions on the full crate.
coverage:
	bash run.sh --coverage

clean:
	cargo clean
