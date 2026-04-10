.PHONY: build test install run demo headless clean lint fmt check venv venv-update sim-test

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
	@echo "Agent:   ~/.polit/agent_debug.jsonl"
	@echo "Source:  $$(pwd)"

# === Python Simulation ===
# Note: PyO3 needs Homebrew Python, not Xcode system Python.
# Set PYO3_PYTHON=/opt/homebrew/bin/python3.12 if builds fail with dylib errors.

venv:
	python3 -m venv sim/.venv
	sim/.venv/bin/pip install -e sim/

venv-update:
	sim/.venv/bin/pip install -e sim/

sim-test:
	PYO3_PYTHON=/opt/homebrew/bin/python3.12 cargo test --features simulation -- sim_bridge
	/opt/homebrew/bin/python3.12 -m pytest sim/tests/ -v

# === Debug ===

# Show last N agent turns from debug log (default 5)
debug:
	@echo "=== Agent Debug Log ==="
	@tail -5 ~/.polit/agent_debug.jsonl 2>/dev/null | python3 -m json.tool --no-ensure-ascii 2>/dev/null || tail -5 ~/.polit/agent_debug.jsonl 2>/dev/null || echo "No agent log yet. Run polit first."

# Show full agent debug log
debug-full:
	@cat ~/.polit/agent_debug.jsonl 2>/dev/null | python3 -m json.tool --no-ensure-ascii 2>/dev/null || cat ~/.polit/agent_debug.jsonl 2>/dev/null || echo "No agent log yet."

# Show last N lines of game log
log:
	@tail -50 ~/.polit/polit.log 2>/dev/null || echo "No log yet."

# Clear debug logs for fresh run
debug-clear:
	@rm -f ~/.polit/agent_debug.jsonl ~/.polit/polit.log
	@echo "Debug logs cleared."
