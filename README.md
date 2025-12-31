# ag-botkit

> **Low-latency Polymarket trading infrastructure monorepo**

A modular trading bot framework for Polymarket with real-time monitoring, risk management, and time-series metrics. Built for performance with C core, Rust services, and a lightweight Go dashboard.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│  Polymarket RTDS (WebSocket)                            │
│  wss://ws-live-data.polymarket.com                      │
└──────────────────┬──────────────────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────────────────┐
│  examples/minibot (Rust)                                │
│  • Connects to RTDS                                     │
│  • Evaluates risk policies                              │
│  • Generates metrics                                    │
└──────────────────┬──────────────────────────────────────┘
                   │ metrics (WebSocket)
                   ▼
┌─────────────────────────────────────────────────────────┐
│  monitor/ (Go + uPlot)                                  │
│  • WebSocket server (:8080)                             │
│  • Real-time dashboard                                  │
│  • Metrics storage & charting                           │
└─────────────────────────────────────────────────────────┘
```

**Supporting Libraries:**
- `core/` - C time-series ring buffer (zero-copy, lock-free)
- `risk/` - Rust risk engine (policies, simulator, evaluator)

## Quick Start

### Prerequisites

- **C compiler** (gcc or clang) - for core/
- **Rust** 1.70+ - for risk/ and minibot
- **Go** 1.21+ - for monitor/
- **Make** - for build orchestration

### Build & Run

```bash
# Clone the repository
git clone <repo-url>
cd ag-botkit

# Build all components
make all

# Start the stack (monitor + minibot)
./run_local.sh
```

Open your browser to **http://localhost:8080** to view the dashboard.

### What You'll See

The dashboard displays 6 real-time charts:
1. **RTDS Lag** - WebSocket latency to Polymarket
2. **Messages/Second** - Throughput from RTDS
3. **Position Size** - Simulated position tracking
4. **Risk Decisions** - Policy evaluation results
5. **Messages Received** - Cumulative counter
6. **Kill Switch** - Risk kill-switch status

## Repository Structure

```
ag-botkit/
├── core/                    # C library: ring-buffer time-series
│   ├── include/            # Public headers
│   ├── src/                # Implementation
│   ├── tests/              # Unit tests
│   ├── lib/                # Built library (libag_core.a)
│   └── Makefile
│
├── risk/                    # Rust library: risk engine
│   ├── src/                # Source code
│   ├── examples/           # Example policies
│   ├── policies/           # Policy templates
│   ├── tests/              # Tests
│   └── Cargo.toml
│
├── monitor/                 # Go dashboard
│   ├── cmd/monitor/        # Entry point
│   ├── internal/           # Server & storage
│   ├── web/                # HTML/CSS/JS
│   ├── bin/                # Built binary
│   └── go.mod
│
├── examples/
│   └── minibot/            # Demo Polymarket bot
│       ├── src/            # Rust source
│       ├── config.yaml     # Configuration
│       └── Cargo.toml
│
├── MULTI_AGENT_PLAN.md     # System architecture doc
├── CLAUDE.md               # Claude Code guidance
├── Makefile                # Root build system
└── run_local.sh            # Start script
```

## Development

### Building Individual Components

```bash
# Core C library
make core
cd core && make test

# Risk Rust library
make risk
cd risk && cargo test

# Monitor Go dashboard
make monitor
cd monitor && go test ./...

# Minibot
make minibot
```

### Running Components Separately

```bash
# Monitor only (from project root)
./monitor/bin/monitor -web ./monitor/web

# Monitor only (from monitor directory)
cd monitor && ./bin/monitor

# Minibot only (requires monitor running)
./examples/minibot/target/release/minibot --config examples/minibot/config.yaml
```

### Configuration

Edit `examples/minibot/config.yaml` to configure:
- RTDS endpoint and market subscriptions
- Monitor WebSocket endpoint
- Risk policy file path

Edit risk policies in `risk/policies/` or create custom YAML files.

### Testing

```bash
# Run all tests
make test

# Individual test suites
cd core && make test
cd risk && cargo test
cd monitor && go test ./...
```

## Component Documentation

Each component has detailed documentation:

- **[core/README.md](core/README.md)** - C API reference, usage examples
- **[risk/README.md](risk/README.md)** - Policy format, API guide
- **[monitor/README.md](monitor/README.md)** - WebSocket protocol, dashboard
- **[examples/minibot/README.md](examples/minibot/README.md)** - Bot setup, metrics

## Metrics Protocol

Minibot sends metrics to monitor via WebSocket (`ws://localhost:8080/metrics`):

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

**Metric Types:**
- `counter` - Cumulative values (messages received)
- `gauge` - Point-in-time values (lag, position size)
- `histogram` - Distributions (not yet implemented)

## Risk Policies

Risk engine supports YAML-based policies:

```yaml
policies:
  - type: PositionLimit
    market_id: "0x123abc"
    max_size: 1000.0

  - type: InventoryLimit
    max_value_usd: 10000.0

  - type: KillSwitch
    enabled: false
```

See `risk/examples/example_policy.yaml` for more examples.

## Performance

- **Core** ring buffer: 35M ops/sec (append)
- **Monitor** metrics ingestion: 7.8M metrics/sec
- **Risk** policy evaluation: <1µs per check
- **Minibot** RTDS throughput: 1000+ msgs/sec

## Polymarket Integration

- **RTDS WebSocket:** `wss://ws-live-data.polymarket.com`
- **Message Format:** `{ topic, type, timestamp, payload }`
- **Subscriptions:** Dynamic, no reconnect required
- **Ping Interval:** 5 seconds (recommended)

## Roadmap

**MVP (Current):**
- ✅ Real-time RTDS connection
- ✅ Risk policy evaluation
- ✅ Metrics dashboard
- ✅ Mock position tracking

**Future:**
- CLOB API integration (order placement)
- Persistent storage (TimescaleDB)
- Advanced risk models (VaR, Greeks)
- Multi-market strategy support
- Production deployment tooling

## Architecture Document

For detailed system architecture, interfaces, and contracts between components, see **[MULTI_AGENT_PLAN.md](MULTI_AGENT_PLAN.md)**.

## License

MIT

## Contributing

This is an MVP demonstration project. For production use, implement:
- Proper error recovery and reconnection logic
- Persistent storage for metrics
- Authentication for monitor dashboard
- Rate limiting and backpressure handling
- Comprehensive logging and alerting
