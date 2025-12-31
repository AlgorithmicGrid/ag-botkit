# Minibot - Polymarket RTDS Demo Bot

A demonstration bot that connects to Polymarket's real-time data service (RTDS), evaluates risk policies, and sends metrics to the monitoring dashboard.

## Features

- Connects to Polymarket RTDS WebSocket
- Subscribes to market data streams
- Calculates real-time metrics:
  - WebSocket lag (server timestamp vs received timestamp)
  - Messages per second
  - Mock position tracking
  - Risk policy evaluation
- Sends metrics to monitor dashboard via WebSocket
- Automatic reconnection handling
- Configurable via YAML

## Building

```bash
cargo build --release
```

## Configuration

Edit `config.yaml`:

```yaml
rtds:
  endpoint: "wss://ws-live-data.polymarket.com"
  ping_interval_sec: 5
  reconnect_delay_sec: 2
  subscribe_topics:
    - market: "0x..." # Replace with real market ID

monitor:
  endpoint: "ws://localhost:8080/metrics"
  buffer_size: 1000
  reconnect_delay_sec: 1

risk:
  policy_file: "risk/examples/example_policy.yaml"  # Path relative to project root
```

**Note:** Paths in `config.yaml` are relative to the project root (`/Users/borkiss../ag-botkit/`), not the minibot directory.

## Running

**Prerequisites:**
- Monitor dashboard must be running on port 8080
- Valid Polymarket market ID in config

```bash
# From project root (recommended)
cd /Users/borkiss../ag-botkit
./examples/minibot/target/release/minibot --config examples/minibot/config.yaml

# Or use the run_local.sh script
./run_local.sh

# Run with debug logging
RUST_LOG=debug ./examples/minibot/target/release/minibot --config examples/minibot/config.yaml
```

**Note:** Always run from the project root directory, as config paths are relative to the root.

## Metrics Generated

### RTDS Connection Metrics
- `polymarket.rtds.messages_received` (counter) - Total messages received
- `polymarket.rtds.lag_ms` (gauge) - WebSocket latency
- `polymarket.rtds.msgs_per_second` (gauge) - Message throughput

### Position Metrics
- `polymarket.position.size` (gauge) - Position size per market
- `polymarket.inventory.value_usd` (gauge) - Total inventory value

### Risk Metrics
- `polymarket.risk.decision` (gauge) - Risk check result (1=allowed, 0=blocked)

## Integration with Monitor

Minibot sends metrics to the monitor dashboard in the following JSON format:

```json
{
  "timestamp": 1735689600000,
  "metric_type": "gauge",
  "metric_name": "polymarket.rtds.lag_ms",
  "value": 45.3,
  "labels": {
    "topic": "market"
  }
}
```

The monitor receives these metrics on `ws://localhost:8080/metrics` and displays them in real-time charts.

## Architecture

```
RTDS (Polymarket)
    ↓ WebSocket
Minibot
    ↓ Risk Evaluation (ag_risk)
    ↓ Metrics Generation
    ↓ WebSocket
Monitor Dashboard
    ↓ Charts (uPlot)
Browser
```

## Error Handling

- **RTDS connection lost:** Logs error, will need manual restart (auto-reconnect coming)
- **Monitor connection lost:** Logs warning, metrics are dropped
- **Invalid message:** Logs warning, continues processing
- **Risk violation:** Logs warning, emits metric with value=0

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Check code
cargo clippy

# Format code
cargo fmt
```

## Notes

- This is a **demo bot** for testing the monitoring infrastructure
- Does **NOT** place real orders
- Position updates are simulated for demonstration
- Uses mock data for risk evaluation
- For production use, implement proper error recovery and reconnection logic
