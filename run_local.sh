#!/usr/bin/env bash
# run_local.sh - Start monitor and minibot for local development

set -e

PROJECT_ROOT="/Users/borkiss../ag-botkit"
MONITOR_BIN="$PROJECT_ROOT/monitor/bin/monitor"
MINIBOT_BIN="$PROJECT_ROOT/examples/minibot/target/release/minibot"
MINIBOT_CONFIG="$PROJECT_ROOT/examples/minibot/config.yaml"

echo "========================================"
echo "  ag-botkit Local Development Stack    "
echo "========================================"
echo ""

# Check if binaries exist
if [ ! -f "$MONITOR_BIN" ] || [ ! -f "$MINIBOT_BIN" ]; then
    echo "==> Building all components..."
    make -C "$PROJECT_ROOT" all
    echo ""
fi

# Start monitor
echo "==> Starting monitor dashboard..."
"$MONITOR_BIN" -web "$PROJECT_ROOT/monitor/web" &
MONITOR_PID=$!
echo "Monitor started (PID: $MONITOR_PID)"

# Wait for monitor to start
echo "Waiting for monitor to start..."
sleep 3

# Check if monitor is still running
if ! kill -0 $MONITOR_PID 2>/dev/null; then
    echo "ERROR: Monitor failed to start"
    exit 1
fi

# Start minibot
echo ""
echo "==> Starting minibot..."
"$MINIBOT_BIN" --config "$MINIBOT_CONFIG" &
MINIBOT_PID=$!
echo "Minibot started (PID: $MINIBOT_PID)"

echo ""
echo "=========================================="
echo "  Stack is running!                      "
echo "=========================================="
echo ""
echo "Dashboard:    http://localhost:8080"
echo "Monitor PID:  $MONITOR_PID"
echo "Minibot PID:  $MINIBOT_PID"
echo ""
echo "Logs:"
echo "  - Monitor: stdout"
echo "  - Minibot: stdout"
echo ""
echo "Press Ctrl+C to stop all services..."
echo ""

# Trap Ctrl+C and cleanup
cleanup() {
    echo ""
    echo "==> Shutting down..."
    kill $MINIBOT_PID 2>/dev/null || true
    kill $MONITOR_PID 2>/dev/null || true
    echo "Stopped."
    exit 0
}

trap cleanup INT TERM

# Wait for processes
wait
