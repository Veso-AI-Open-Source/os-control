BIN := $(HOME)/.local/bin

.PHONY: build debug test live install register bar status demo

# Release build of the tool, the bar, and the gate panel.
build:
	cargo build --release --workspace
	swiftc -O helpers/ask.swift -o target/release/osctl-ask

debug:
	cargo build --workspace
	swiftc -O helpers/ask.swift -o target/debug/osctl-ask

# Unit tests. They touch nothing on screen.
test:
	cargo test --workspace

# Live checks on the real desktop. Calculator only, in the background.
live: debug
	python3 tests/live.py --pointer

# Puts osctl and its panel on the PATH. Sessions already running keep the
# old binary until they restart.
install: build
	install -d $(BIN)
	install -m 755 target/release/osctl target/release/osctl-ask target/release/os-bar $(BIN)/

# Gives the tools to every Claude Code session of this user.
register:
	claude mcp add --scope user os -- $(BIN)/osctl mcp

# Opens the bar. Started from a terminal, it uses the terminal's grants.
bar:
	$(BIN)/os-bar &

status:
	$(BIN)/osctl status

# Records docs/demo.gif. See demo/README.md.
demo:
	uv run demo/record.py
