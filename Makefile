.PHONY: all core risk exec storage monitor strategies minibot test clean help

all: core risk exec storage monitor strategies minibot

core:
	@echo "Building core C library..."
	cd core && make

risk:
	@echo "Building risk Rust library..."
	cd risk && cargo build --release

exec: risk
	@echo "Building exec gateway Rust library..."
	cd exec && cargo build --release

storage:
	@echo "Building storage Rust library..."
	cd storage && cargo build --release

monitor:
	@echo "Building monitor Go dashboard..."
	cd monitor && go build -o bin/monitor ./cmd/monitor

strategies: risk exec
	@echo "Building strategies Rust library..."
	cd strategies && cargo build --release

minibot: risk
	@echo "Building minibot..."
	cd examples/minibot && cargo build --release

test: test-core test-risk test-exec test-storage test-monitor test-strategies
	@echo "✓ All tests passed"

test-core:
	@echo "Testing core..."
	cd core && make test

test-risk:
	@echo "Testing risk..."
	cd risk && cargo test

test-exec:
	@echo "Testing exec..."
	cd exec && cargo test

test-storage:
	@echo "Testing storage..."
	cd storage && cargo test

test-monitor:
	@echo "Testing monitor..."
	cd monitor && go test ./...

test-strategies:
	@echo "Testing strategies..."
	cd strategies && cargo test

clean:
	@echo "Cleaning all build artifacts..."
	cd core && make clean
	cd risk && cargo clean
	cd exec && cargo clean
	cd storage && cargo clean
	cd monitor && rm -rf bin
	cd strategies && cargo clean
	cd examples/minibot && cargo clean

help:
	@echo "ag-botkit Makefile"
	@echo ""
	@echo "Targets:"
	@echo "  all        - Build all components (core, risk, exec, storage, monitor, strategies, minibot)"
	@echo "  core       - Build core C library"
	@echo "  risk       - Build risk Rust library"
	@echo "  exec       - Build execution gateway Rust library"
	@echo "  storage    - Build storage Rust library"
	@echo "  monitor    - Build monitor Go dashboard"
	@echo "  strategies - Build strategies Rust library"
	@echo "  minibot    - Build minibot demo"
	@echo "  test       - Run all tests"
	@echo "  clean      - Remove all build artifacts"
	@echo "  help       - Show this help message"
