# ev — ExaVerif

Exhaustive verification CLI for RISC-V custom instructions.

## Prerequisites

- Rust 1.85+ (install via [rustup](https://rustup.rs/))

## Quick Start

```bash
# Build
cargo build --release

# Run
ev check --target my_xif.yaml
ev certify --target my_xif.yaml --output certificate.pdf

# Test
cargo test --release

# Lint and format
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

Or use the Makefile:

```bash
make build
make test
make check  # fmt + clippy + build + test
```

## Project Structure

```
src/main.rs          CLI entry point (clap-based subcommands)
tests/cli_test.rs    Integration tests for CLI interface
docs/                Documentation (Quarto website)
```

## CI

Two workflows:

- `build-ev.yml` — Rust build, lint, format, test on push/PR to main
- Documentation workflows — build and deploy docs site
