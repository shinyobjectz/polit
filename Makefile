.PHONY: build test install run demo headless clean lint fmt check

# === Development ===

build:
	cargo build

run:
	cargo run

demo:
	cargo run -- --demo

headless:
	cargo run -- --headless

# === Testing ===

test:
	cargo test

test-verbose:
	cargo test -- --nocapture

bench:
	cargo bench

# === Quality ===

lint:
	cargo clippy --all-targets -- -W clippy::all

fmt:
	cargo fmt

fmt-check:
	cargo fmt -- --check

check: fmt-check lint test
	@echo "All checks passed."

# === Install (updates global `polit` command) ===

install:
	cargo install --path .
	@echo ""
	@echo "✓ polit installed to $$(which polit)"
	@echo "  Run 'polit' from anywhere to launch."

# === Release build (optimized, no install) ===

release:
	cargo build --release

# === Clean ===

clean:
	cargo clean

clean-data:
	rm -rf ~/.polit/game.db
	@echo "Game database cleared. Config and saves preserved."

clean-all:
	rm -rf ~/.polit
	@echo "All POLIT data cleared."

# === Shortcuts ===

# Build + test + install in one command
update: check install
	@echo ""
	@echo "✓ Built, tested, and installed."

# Quick rebuild + install (skip tests)
quick: build install

# Show where things are
info:
	@echo "Binary:  $$(which polit 2>/dev/null || echo 'not installed')"
	@echo "Data:    ~/.polit/"
	@echo "Config:  ~/.polit/config/"
	@echo "Saves:   ~/.polit/saves/"
	@echo "Log:     ~/.polit/polit.log"
	@echo "Source:  $$(pwd)"
