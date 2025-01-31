.PHONY: build clean format test bench

# Default build in release mode
build:
	cargo build --release

# Clean build artifacts
clean:
	cargo clean

# Format code using rustfmt
format:
	cargo fmt --all

# Run tests
test:
	cargo test

# Run benchmarks
bench:
	cargo bench

# Check code without building
check:
	cargo check

# Additional useful target that combines format and check
lint: format check