.PHONY: all core risk monitor minibot test clean help

all: core risk monitor minibot

core:
	@echo "Building core C library..."
	cd core && make

risk:
	@echo "Building risk Rust library..."
	cd risk && cargo build --release

monitor:
	@echo "Building monitor Go dashboard..."
	cd monitor && go build -o bin/monitor ./cmd/monitor

minibot: risk
	@echo "Building minibot..."
	cd examples/minibot && cargo build --release

test: test-core test-risk test-monitor
	@echo "✓ All tests passed"

test-core:
	@echo "Testing core..."
	cd core && make test

test-risk:
	@echo "Testing risk..."
	cd risk && cargo test

test-monitor:
	@echo "Testing monitor..."
	cd monitor && go test ./...

clean:
	@echo "Cleaning all build artifacts..."
	cd core && make clean
	cd risk && cargo clean
	cd monitor && rm -rf bin
	cd examples/minibot && cargo clean

help:
	@echo "ag-botkit Makefile"
	@echo ""
	@echo "Targets:"
	@echo "  all       - Build all components (core, risk, monitor, minibot)"
	@echo "  core      - Build core C library"
	@echo "  risk      - Build risk Rust library"
	@echo "  monitor   - Build monitor Go dashboard"
	@echo "  minibot   - Build minibot demo"
	@echo "  test      - Run all tests"
	@echo "  clean     - Remove all build artifacts"
	@echo "  help      - Show this help message"
