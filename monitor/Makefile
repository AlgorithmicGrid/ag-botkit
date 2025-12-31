.PHONY: all build test clean run help

# Default target
all: build

# Build the monitor binary
build:
	@echo "Building monitor..."
	@mkdir -p bin
	@go build -o bin/monitor ./cmd/monitor
	@echo "Build complete: bin/monitor"

# Run tests
test:
	@echo "Running tests..."
	@go test ./... -v

# Run tests with coverage
test-coverage:
	@echo "Running tests with coverage..."
	@go test ./... -coverprofile=coverage.out
	@go tool cover -html=coverage.out -o coverage.html
	@echo "Coverage report: coverage.html"

# Clean build artifacts
clean:
	@echo "Cleaning..."
	@rm -rf bin/
	@rm -f coverage.out coverage.html
	@echo "Clean complete"

# Run the monitor server
run: build
	@echo "Starting monitor server..."
	@./bin/monitor

# Format code
fmt:
	@echo "Formatting code..."
	@go fmt ./...

# Lint code
lint:
	@echo "Linting code..."
	@go vet ./...

# Download dependencies
deps:
	@echo "Downloading dependencies..."
	@go mod download
	@go mod tidy

# Help
help:
	@echo "Available targets:"
	@echo "  make build          - Build the monitor binary"
	@echo "  make test           - Run unit tests"
	@echo "  make test-coverage  - Run tests with coverage report"
	@echo "  make run            - Build and run the monitor"
	@echo "  make clean          - Remove build artifacts"
	@echo "  make fmt            - Format code"
	@echo "  make lint           - Lint code"
	@echo "  make deps           - Download dependencies"
	@echo "  make help           - Show this help message"
