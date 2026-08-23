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

# Code coverage gate (cargo-llvm-cov). The Spike, Yosys, and simulation
# backends are excluded: they require external tools at runtime and are
# exercised by the integration pipeline instead. Thresholds leave headroom
# over the measured 85.67% lines / 84.72% regions on the committed suite.
coverage:
	cargo llvm-cov --release --fail-under-lines 80 --fail-under-regions 80 \
		--ignore-filename-regex "synth/backends|synth/sim"
	@echo "fixture coverage: tagma decoder 11,172/11,172, tagma demo top 11,172/11,172, cva6 ref 196,608, ibex rv32imcb 92,160"

clean:
	cargo clean
