# Makefile for ag_timeseries core library
#
# Targets:
#   all   - Build static library
#   test  - Build and run unit tests
#   clean - Remove all build artifacts
#
# Requirements:
#   - C11 compiler (gcc or clang)
#   - Standard math library
#
# Copyright (c) 2025 AlgorithmicGrid

# Compiler configuration
CC := gcc
CFLAGS := -std=c11 -Wall -Wextra -Wpedantic -O2 -fPIC
CFLAGS_DEBUG := -std=c11 -Wall -Wextra -Wpedantic -g -O0 -fPIC
LDFLAGS := -lm

# Directories
SRC_DIR := src
INC_DIR := include
TEST_DIR := tests
LIB_DIR := lib
BUILD_DIR := build

# Output files
STATIC_LIB := $(LIB_DIR)/libag_core.a
TEST_BIN := $(BUILD_DIR)/test_timeseries

# Source files
SRC_FILES := $(wildcard $(SRC_DIR)/*.c)
OBJ_FILES := $(patsubst $(SRC_DIR)/%.c,$(BUILD_DIR)/%.o,$(SRC_FILES))

# Test files
TEST_SRC := $(TEST_DIR)/test_timeseries.c
TEST_OBJ := $(BUILD_DIR)/test_timeseries.o

# Header dependencies
HEADERS := $(wildcard $(INC_DIR)/*.h)

# Platform detection for archive command
UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Darwin)
    AR := ar rcs
else
    AR := ar rcs
endif

# Default target
.PHONY: all
all: $(STATIC_LIB)

# Create directories
$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

$(LIB_DIR):
	mkdir -p $(LIB_DIR)

# Compile source files
$(BUILD_DIR)/%.o: $(SRC_DIR)/%.c $(HEADERS) | $(BUILD_DIR)
	$(CC) $(CFLAGS) -I$(INC_DIR) -c $< -o $@

# Build static library
$(STATIC_LIB): $(OBJ_FILES) | $(LIB_DIR)
	$(AR) $@ $(OBJ_FILES)
	@echo "Built static library: $(STATIC_LIB)"

# Compile test file
$(TEST_OBJ): $(TEST_SRC) $(HEADERS) | $(BUILD_DIR)
	$(CC) $(CFLAGS_DEBUG) -I$(INC_DIR) -c $< -o $@

# Build test binary
$(TEST_BIN): $(TEST_OBJ) $(OBJ_FILES)
	$(CC) $(CFLAGS_DEBUG) -o $@ $^ $(LDFLAGS)
	@echo "Built test binary: $(TEST_BIN)"

# Run tests
.PHONY: test
test: $(TEST_BIN)
	@echo ""
	@echo "Running unit tests..."
	@echo "====================="
	@$(TEST_BIN)
	@echo ""
	@echo "Test summary: All tests completed successfully"

# Run tests with valgrind (memory leak detection)
.PHONY: test-valgrind
test-valgrind: $(TEST_BIN)
	@echo ""
	@echo "Running tests with valgrind..."
	@echo "==============================="
	@valgrind --leak-check=full --show-leak-kinds=all --track-origins=yes \
	          --error-exitcode=1 $(TEST_BIN)
	@echo ""
	@echo "Valgrind: No memory leaks detected"

# Clean build artifacts
.PHONY: clean
clean:
	rm -rf $(BUILD_DIR) $(LIB_DIR)
	@echo "Cleaned build artifacts"

# Help target
.PHONY: help
help:
	@echo "ag_timeseries Core Library Makefile"
	@echo ""
	@echo "Targets:"
	@echo "  all            - Build static library (default)"
	@echo "  test           - Build and run unit tests"
	@echo "  test-valgrind  - Run tests with memory leak detection (requires valgrind)"
	@echo "  clean          - Remove all build artifacts"
	@echo "  help           - Show this help message"
	@echo ""
	@echo "Output:"
	@echo "  Library: $(STATIC_LIB)"
	@echo "  Tests:   $(TEST_BIN)"

# Phony targets
.PHONY: all test test-valgrind clean help
