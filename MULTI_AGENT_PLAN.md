# MULTI_AGENT_PLAN.md - ag-botkit Polymarket MVP Architecture

**Project Mission:** Build a minimal Polymarket trading bot framework with real-time monitoring, risk management, and time-series metrics storage.

**Target Platform:** Polymarket CLOB + RTDS (Real-Time Data Service)
- RTDS WebSocket: `wss://ws-live-data.polymarket.com`
- RTDS Message Format: `{ topic: string, type: string, timestamp: number, payload: object }`
- Dynamic subscription model with ~5s ping interval

---

## 1. SYSTEM ARCHITECTURE OVERVIEW

```
┌──────────────────────────────────────────────────────────────────┐
│                         ag-botkit System                          │
├──────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌─────────────────┐         ┌──────────────────┐               │
│  │ examples/       │         │  monitor/        │               │
│  │   minibot/      │────────▶│  (Go Dashboard)  │               │
│  │                 │ metrics │                  │               │
│  │  - RTDS client  │ via WS  │  - WebSocket srv │               │
│  │  - Risk eval    │         │  - uPlot charts  │               │
│  │  - Metrics gen  │         │  - HTTP static   │               │
│  └────────┬────────┘         └──────────────────┘               │
│           │                                                       │
│           │ uses (FFI)       ┌──────────────────┐               │
│           └─────────────────▶│  risk/           │               │
│                               │  (Rust Library)  │               │
│                               │                  │               │
│                               │  - Policy engine │               │
│                               │  - Simulator     │               │
│                               └────────┬─────────┘               │
│                                        │                          │
│                                        │ uses (FFI)               │
│                                        ▼                          │
│                               ┌──────────────────┐               │
│                               │  core/           │               │
│                               │  (C Library)     │               │
│                               │                  │               │
│                               │  - Ring buffers  │               │
│                               │  - Time-series   │               │
│                               └──────────────────┘               │
│                                                                   │
└──────────────────────────────────────────────────────────────────┘
```

**Data Flow:**
1. `minibot` connects to Polymarket RTDS WebSocket
2. `minibot` receives market data, evaluates risk policies
3. `minibot` generates metrics (lag, msgs/sec, inventory, risk decisions)
4. `minibot` pushes metrics to `monitor` via WebSocket
5. `monitor` renders real-time charts in browser

---

## 2. METRICS PROTOCOL (minibot → monitor)

### 2.1 WebSocket Connection
- **Endpoint:** `ws://localhost:8080/metrics`
- **Protocol:** JSON messages, one per line (newline-delimited)
- **Direction:** Unidirectional (minibot → monitor)

### 2.2 Metrics Message Format

**Base Message Schema:**
```json
{
  "timestamp": 1735689600000,
  "metric_type": "string",
  "metric_name": "string",
  "value": number,
  "labels": {
    "key": "value"
  }
}
```

**Field Definitions:**
- `timestamp` (required, int64): Unix milliseconds
- `metric_type` (required, string): One of `"counter"`, `"gauge"`, `"histogram"`
- `metric_name` (required, string): Metric identifier (see below)
- `value` (required, float64): Metric value
- `labels` (optional, object): Key-value pairs for metric dimensions

### 2.3 Defined Metrics

**RTDS Connection Metrics:**
```json
// WebSocket message received
{
  "timestamp": 1735689600000,
  "metric_type": "counter",
  "metric_name": "polymarket.rtds.messages_received",
  "value": 1,
  "labels": {
    "topic": "market",
    "message_type": "book"
  }
}

// WebSocket lag (received timestamp - server timestamp)
{
  "timestamp": 1735689600100,
  "metric_type": "gauge",
  "metric_name": "polymarket.rtds.lag_ms",
  "value": 45.3,
  "labels": {
    "topic": "market"
  }
}

// Messages per second (computed over 1s window)
{
  "timestamp": 1735689601000,
  "metric_type": "gauge",
  "metric_name": "polymarket.rtds.msgs_per_second",
  "value": 23.5,
  "labels": {}
}
```

**Mock Inventory Metrics:**
```json
// Position size for a market
{
  "timestamp": 1735689600200,
  "metric_type": "gauge",
  "metric_name": "polymarket.position.size",
  "value": 150.0,
  "labels": {
    "market_id": "0x123abc",
    "side": "long"
  }
}

// Total inventory value (USD)
{
  "timestamp": 1735689600200,
  "metric_type": "gauge",
  "metric_name": "polymarket.inventory.value_usd",
  "value": 1250.75,
  "labels": {}
}
```

**Risk Decision Metrics:**
```json
// Risk check result (0 = blocked, 1 = allowed)
{
  "timestamp": 1735689600300,
  "metric_type": "gauge",
  "metric_name": "polymarket.risk.decision",
  "value": 1,
  "labels": {
    "policy": "position_limit",
    "market_id": "0x123abc"
  }
}

// Kill-switch state (0 = off, 1 = triggered)
{
  "timestamp": 1735689600300,
  "metric_type": "gauge",
  "metric_name": "polymarket.risk.kill_switch",
  "value": 0,
  "labels": {}
}
```

### 2.4 Error Handling
- If monitor WebSocket is unavailable, minibot should buffer metrics (up to 1000 messages) and reconnect
- Malformed JSON should be logged and dropped by monitor
- Unknown metric_name should be accepted (forward compatibility)

---

## 3. MODULE CONTRACTS

### 3.1 core/ - C Time-Series Library

**Purpose:** Provide lock-free, zero-allocation ring buffer storage for time-series metrics.

**API Contract:**

```c
// core/include/ag_timeseries.h

#ifndef AG_TIMESERIES_H
#define AG_TIMESERIES_H

#include <stdint.h>
#include <stddef.h>

// Opaque handle
typedef struct ag_timeseries_t ag_timeseries_t;

// Error codes
#define AG_OK 0
#define AG_ERR_INVALID_ARG -1
#define AG_ERR_NOMEM -2
#define AG_ERR_FULL -3

// Create time-series buffer with fixed capacity
// Returns NULL on failure
ag_timeseries_t* ag_timeseries_create(size_t capacity);

// Destroy time-series buffer
void ag_timeseries_destroy(ag_timeseries_t* ts);

// Append data point (timestamp in ms, value)
// Returns AG_OK or error code
int ag_timeseries_append(ag_timeseries_t* ts, int64_t timestamp_ms, double value);

// Query last N points (newest first)
// Returns number of points written to out_timestamps/out_values
// Caller must provide buffers of size >= max_points
size_t ag_timeseries_query_last(
    const ag_timeseries_t* ts,
    size_t max_points,
    int64_t* out_timestamps,
    double* out_values
);

// Query points in time range [start_ms, end_ms] inclusive
// Returns number of points written
size_t ag_timeseries_query_range(
    const ag_timeseries_t* ts,
    int64_t start_ms,
    int64_t end_ms,
    size_t max_points,
    int64_t* out_timestamps,
    double* out_values
);

// Get current size
size_t ag_timeseries_size(const ag_timeseries_t* ts);

// Get capacity
size_t ag_timeseries_capacity(const ag_timeseries_t* ts);

#endif // AG_TIMESERIES_H
```

**Implementation Requirements:**
- C11 standard, no compiler-specific extensions
- Ring buffer: oldest data evicted when full
- Thread-safety: NOT thread-safe (caller must synchronize)
- No allocations after `ag_timeseries_create()`
- All functions must handle NULL pointers gracefully (return error)

**Build Contract:**
```bash
# In /Users/borkiss../ag-botkit/core/
make              # Build libag_core.a
make test         # Run tests (using criterion or custom harness)
make clean        # Remove build artifacts
```

**Outputs:**
- Static library: `core/lib/libag_core.a`
- Headers: `core/include/ag_timeseries.h`

**Definition of Done:**
- All API functions implemented
- Test coverage >80% (append, query, boundary conditions)
- No memory leaks (valgrind clean)
- README.md with usage examples
- Makefile builds successfully on macOS/Linux

---

### 3.2 risk/ - Rust Risk Engine Library

**Purpose:** Policy-based risk evaluation and market simulation for Polymarket.

**API Contract:**

```rust
// risk/src/lib.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// --- Policy Configuration ---

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RiskPolicyConfig {
    pub policies: Vec<PolicyRule>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum PolicyRule {
    PositionLimit {
        market_id: Option<String>, // None = global limit
        max_size: f64,
    },
    InventoryLimit {
        max_value_usd: f64,
    },
    KillSwitch {
        enabled: bool,
    },
}

// --- Risk Engine ---

pub struct RiskEngine {
    config: RiskPolicyConfig,
    kill_switch_active: bool,
}

impl RiskEngine {
    /// Load policies from YAML string
    pub fn from_yaml(yaml: &str) -> Result<Self, String>;

    /// Load policies from JSON string
    pub fn from_json(json: &str) -> Result<Self, String>;

    /// Evaluate if action is allowed
    pub fn evaluate(&self, ctx: &RiskContext) -> RiskDecision;

    /// Trigger kill-switch
    pub fn trigger_kill_switch(&mut self);

    /// Reset kill-switch
    pub fn reset_kill_switch(&mut self);
}

// --- Risk Context ---

#[derive(Debug, Clone)]
pub struct RiskContext {
    pub market_id: String,
    pub current_position: f64,
    pub proposed_size: f64, // Additional size (signed: +long, -short)
    pub inventory_value_usd: f64,
}

// --- Risk Decision ---

#[derive(Debug, Clone, PartialEq)]
pub struct RiskDecision {
    pub allowed: bool,
    pub violated_policies: Vec<String>, // Policy names that failed
}

// --- Simulator ---

pub struct PolymarketSimulator {
    // Simulates position updates and PnL
}

impl PolymarketSimulator {
    pub fn new() -> Self;

    /// Update position for a market
    pub fn update_position(&mut self, market_id: &str, size: f64, price: f64);

    /// Get current position
    pub fn get_position(&self, market_id: &str) -> f64;

    /// Get total inventory value
    pub fn get_inventory_value_usd(&self) -> f64;

    /// Reset all positions
    pub fn reset(&mut self);
}
```

**Policy File Format (YAML):**
```yaml
# risk/policies/example.yaml
policies:
  - type: PositionLimit
    market_id: "0x123abc"  # Optional, omit for global
    max_size: 1000.0

  - type: InventoryLimit
    max_value_usd: 10000.0

  - type: KillSwitch
    enabled: false
```

**FFI Considerations:**
- Export C-compatible functions for use from minibot (if needed)
- Use `cbindgen` to generate header if FFI required
- For MVP, minibot can link Rust statically

**Build Contract:**
```bash
# In /Users/borkiss../ag-botkit/risk/
cargo build --release        # Build library
cargo test                   # Run tests
cargo clippy                 # Lint
cargo doc --no-deps --open   # Generate docs
```

**Outputs:**
- Library: `risk/target/release/libag_risk.a` (static) or `.so` (dynamic)
- Rust crate for linking in minibot

**Definition of Done:**
- All policy types implemented
- Unit tests for each policy (>80% coverage)
- Simulator tracks positions correctly
- Example policy files in `risk/policies/`
- README.md with policy examples
- cargo build succeeds on macOS/Linux

---

### 3.3 monitor/ - Go Web Dashboard

**Purpose:** Real-time web dashboard for visualizing Polymarket bot metrics.

**API Contract:**

**WebSocket Server:**
```go
// monitor/internal/server/websocket.go

// Listens on ws://localhost:8080/metrics
// Accepts JSON metrics messages (see Section 2)
// Broadcasts to all connected dashboard clients
```

**HTTP Server:**
```go
// monitor/internal/server/http.go

// GET /               - Serve dashboard HTML
// GET /static/*       - Serve CSS/JS assets
// WS  /metrics        - Metrics ingestion endpoint
// WS  /dashboard      - Dashboard client subscription endpoint
```

**Dashboard Protocol (monitor → browser):**
```json
// Broadcast metrics to dashboard clients
{
  "timestamp": 1735689600000,
  "metric_name": "polymarket.rtds.lag_ms",
  "value": 45.3,
  "labels": {"topic": "market"}
}
```

**Frontend Requirements:**
- Single HTML page: `monitor/web/index.html`
- Use uPlot for charting (CDN link acceptable for MVP)
- WebSocket client connects to `ws://localhost:8080/dashboard`
- Charts to render:
  - RTDS lag (line chart, last 60s)
  - Messages/sec (line chart, last 60s)
  - Position size by market (bar chart, current)
  - Risk decisions (timeline, last 100 events)
- No React/Vue/Angular - plain JS or minimal framework

**Project Layout:**
```
monitor/
├── cmd/
│   └── monitor/
│       └── main.go          # Entry point
├── internal/
│   ├── server/
│   │   ├── http.go          # HTTP handlers
│   │   └── websocket.go     # WebSocket handlers
│   └── storage/
│       └── metrics.go       # In-memory metrics store
├── web/
│   ├── index.html           # Dashboard UI
│   └── static/
│       ├── style.css
│       └── app.js           # Chart rendering logic
├── go.mod
├── go.sum
└── README.md
```

**Build Contract:**
```bash
# In /Users/borkiss../ag-botkit/monitor/
go build -o bin/monitor ./cmd/monitor   # Build binary
go test ./...                           # Run tests
go run ./cmd/monitor                    # Run server (port 8080)
```

**Outputs:**
- Binary: `monitor/bin/monitor`
- Listens on `http://localhost:8080`

**Definition of Done:**
- WebSocket ingestion working (accepts metrics)
- Dashboard loads and connects to `/dashboard` WS
- At least 2 charts rendering (lag, msgs/sec)
- Handles reconnection gracefully
- README.md with screenshots
- go build succeeds on macOS/Linux

---

### 3.4 examples/minibot/ - Demo Polymarket Bot

**Purpose:** Reference implementation demonstrating RTDS connection, risk evaluation, and metrics emission.

**Responsibilities:**
1. Connect to Polymarket RTDS WebSocket
2. Subscribe to at least one topic (e.g., `market` for order book updates)
3. Calculate metrics:
   - WebSocket lag: `server_timestamp - received_timestamp`
   - Messages/sec: count over 1s sliding window
   - Mock inventory: simulate position updates
   - Risk decisions: evaluate mock trades against policies
4. Send metrics to monitor via WebSocket

**RTDS Integration:**

**Connection:**
```
wss://ws-live-data.polymarket.com
```

**Message Format (Received):**
```json
{
  "topic": "market",
  "type": "book",
  "timestamp": 1735689600000,
  "payload": {
    "market": "0x123abc",
    "bids": [[0.51, 100], [0.50, 200]],
    "asks": [[0.52, 150], [0.53, 250]]
  }
}
```

**Subscription Message (Send):**
```json
{
  "type": "subscribe",
  "channel": "market",
  "market": "0x123abc"  // Optional filter
}
```

**Ping/Pong:**
```json
// Send every ~5 seconds
{"type": "ping"}

// Expect response
{"type": "pong"}
```

**Implementation Language:** Rust (preferred) or Go
- If Rust: can link `ag_risk` natively
- If Go: would need CGO bindings to risk library (acceptable for MVP)

**Configuration:**
```yaml
# examples/minibot/config.yaml
rtds:
  endpoint: "wss://ws-live-data.polymarket.com"
  ping_interval_sec: 5
  reconnect_delay_sec: 2
  subscribe_topics:
    - market: "0x123abc"  # Example market ID

monitor:
  endpoint: "ws://localhost:8080/metrics"
  buffer_size: 1000
  reconnect_delay_sec: 1

risk:
  policy_file: "../../risk/policies/example.yaml"
```

**Build Contract:**
```bash
# In /Users/borkiss../ag-botkit/examples/minibot/
cargo build --release   # If Rust
# OR
go build -o bin/minibot ./cmd/minibot  # If Go

# Run
./target/release/minibot --config config.yaml  # Rust
# OR
./bin/minibot --config config.yaml  # Go
```

**Outputs:**
- Binary: `examples/minibot/target/release/minibot` or `examples/minibot/bin/minibot`
- Connects to RTDS and monitor
- Logs activity to stdout

**Definition of Done:**
- Successfully connects to Polymarket RTDS
- Receives and parses at least one message type
- Calculates and emits all 5 metric types
- Connects to monitor WebSocket
- Handles reconnections (RTDS and monitor)
- README.md with setup instructions
- Builds successfully on macOS/Linux

---

## 4. BUILD AND ORCHESTRATION

### 4.1 Root Makefile

```makefile
# /Users/borkiss../ag-botkit/Makefile

.PHONY: all core risk monitor minibot test clean

all: core risk monitor minibot

core:
	cd core && make

risk:
	cd risk && cargo build --release

monitor:
	cd monitor && go build -o bin/monitor ./cmd/monitor

minibot: risk
	cd examples/minibot && cargo build --release

test: test-core test-risk test-monitor
	@echo "All tests passed"

test-core:
	cd core && make test

test-risk:
	cd risk && cargo test

test-monitor:
	cd monitor && go test ./...

clean:
	cd core && make clean
	cd risk && cargo clean
	cd monitor && rm -rf bin
	cd examples/minibot && cargo clean
```

### 4.2 run_local.sh

**Purpose:** Start monitor and minibot for local development.

```bash
#!/usr/bin/env bash
# /Users/borkiss../ag-botkit/run_local.sh

set -e

PROJECT_ROOT="/Users/borkiss../ag-botkit"

echo "==> Building all components..."
make -C "$PROJECT_ROOT" all

echo ""
echo "==> Starting monitor dashboard..."
"$PROJECT_ROOT/monitor/bin/monitor" &
MONITOR_PID=$!

# Wait for monitor to start
sleep 2

echo ""
echo "==> Starting minibot..."
"$PROJECT_ROOT/examples/minibot/target/release/minibot" \
  --config "$PROJECT_ROOT/examples/minibot/config.yaml" &
MINIBOT_PID=$!

echo ""
echo "=========================================="
echo "Dashboard: http://localhost:8080"
echo "Monitor PID: $MONITOR_PID"
echo "Minibot PID: $MINIBOT_PID"
echo "=========================================="
echo ""
echo "Press Ctrl+C to stop..."

# Trap Ctrl+C and cleanup
trap "kill $MONITOR_PID $MINIBOT_PID 2>/dev/null; exit" INT

# Wait for processes
wait
```

**Usage:**
```bash
chmod +x run_local.sh
./run_local.sh
```

### 4.3 Development Workflow

**Initial Setup:**
```bash
cd /Users/borkiss../ag-botkit
make all
./run_local.sh
```

**Iterative Development:**
```bash
# Rebuild specific module
make core      # or risk, monitor, minibot
./run_local.sh
```

**Testing:**
```bash
make test
```

---

## 5. DELEGATION PLAN FOR SUBAGENTS

### 5.1 Agent: core-c-implementer

**Assigned Directory:** `/Users/borkiss../ag-botkit/core/`

**Responsibilities:**
- Implement all functions in `ag_timeseries.h` contract (Section 3.1)
- Create ring buffer data structure
- Write unit tests (suggest criterion or custom test harness)
- Create Makefile with `all`, `test`, `clean` targets
- Write README.md with API usage examples

**Inputs:**
- API contract from Section 3.1
- Build contract from Section 3.1

**Outputs:**
- `core/lib/libag_core.a`
- `core/include/ag_timeseries.h`
- `core/tests/` (test files)
- `core/Makefile`
- `core/README.md`

**Definition of Done:**
- [ ] All API functions implemented
- [ ] `make` produces `lib/libag_core.a`
- [ ] `make test` runs and passes (>80% coverage)
- [ ] No memory leaks (run valgrind)
- [ ] README.md includes usage examples
- [ ] Code follows C11 standard

**Constraints:**
- C11 only, no C++ or extensions
- No allocations in append/query functions
- Caller is responsible for thread safety (document this)

---

### 5.2 Agent: risk-engine

**Assigned Directory:** `/Users/borkiss../ag-botkit/risk/`

**Responsibilities:**
- Implement `RiskEngine`, `PolymarketSimulator` per Section 3.2 contract
- Support YAML and JSON policy loading (use serde)
- Write unit tests for each policy type
- Create example policy files in `risk/policies/`
- Write README.md with policy examples

**Inputs:**
- API contract from Section 3.2
- Policy format from Section 3.2
- Build contract from Section 3.2

**Outputs:**
- `risk/src/lib.rs` (and supporting modules)
- `risk/target/release/libag_risk.a`
- `risk/policies/example.yaml`
- `risk/tests/` (unit tests)
- `risk/README.md`

**Definition of Done:**
- [ ] All policy types implemented (PositionLimit, InventoryLimit, KillSwitch)
- [ ] `cargo build --release` succeeds
- [ ] `cargo test` passes (>80% coverage)
- [ ] Simulator tracks positions correctly
- [ ] Example policy file validates
- [ ] README.md includes policy examples
- [ ] No clippy warnings

**Constraints:**
- Use tokio for async if needed (though sync is fine for MVP)
- Use thiserror for errors
- Keep dependencies minimal

---

### 5.3 Agent: monitor-ui

**Assigned Directory:** `/Users/borkiss../ag-botkit/monitor/`

**Responsibilities:**
- Implement WebSocket server per Section 3.3 contract
- Create HTTP server serving static dashboard
- Build frontend with uPlot charts
- Implement at least 2 charts (lag, msgs/sec)
- Write README.md with screenshots

**Inputs:**
- API contract from Section 3.3
- Metrics protocol from Section 2
- Dashboard protocol from Section 3.3
- Build contract from Section 3.3

**Outputs:**
- `monitor/cmd/monitor/main.go`
- `monitor/internal/server/` (http.go, websocket.go)
- `monitor/web/index.html`
- `monitor/web/static/app.js`
- `monitor/bin/monitor` (binary)
- `monitor/README.md`

**Definition of Done:**
- [ ] WebSocket `/metrics` accepts metrics messages
- [ ] WebSocket `/dashboard` broadcasts to clients
- [ ] Dashboard loads at `http://localhost:8080`
- [ ] At least 2 charts render (lag, msgs/sec)
- [ ] Handles reconnections gracefully
- [ ] `go build` succeeds
- [ ] `go test ./...` passes
- [ ] README.md includes screenshots

**Constraints:**
- No heavy frontend frameworks (React/Vue/Angular)
- Use uPlot for charts (lightweight)
- Plain Go, standard library preferred (gorilla/websocket acceptable)

---

### 5.4 Agent: minibot-implementer (OR user implements)

**Assigned Directory:** `/Users/borkiss../ag-botkit/examples/minibot/`

**Responsibilities:**
- Connect to Polymarket RTDS WebSocket
- Subscribe to at least one market
- Parse RTDS messages
- Calculate metrics (lag, msgs/sec, mock inventory)
- Integrate risk library
- Send metrics to monitor WebSocket
- Handle reconnections

**Inputs:**
- RTDS spec from Section 3.4
- Metrics protocol from Section 2
- Risk library from risk/
- Build contract from Section 3.4

**Outputs:**
- `examples/minibot/src/main.rs` (if Rust) or `cmd/minibot/main.go` (if Go)
- `examples/minibot/config.yaml`
- `examples/minibot/target/release/minibot` or `bin/minibot`
- `examples/minibot/README.md`

**Definition of Done:**
- [ ] Connects to RTDS successfully
- [ ] Receives and parses messages
- [ ] Emits all 5 metric types
- [ ] Connects to monitor WebSocket
- [ ] Uses risk library to evaluate mock trades
- [ ] Handles reconnections (RTDS + monitor)
- [ ] `cargo build --release` or `go build` succeeds
- [ ] README.md with setup instructions

**Constraints:**
- Must use real Polymarket RTDS endpoint (no mocking)
- Keep it simple - this is a demo, not production bot
- Log all activity to stdout

---

## 6. INTEGRATION SEQUENCES

### 6.1 Startup Sequence

```
1. User runs: ./run_local.sh
   ├─ Build all components (make all)
   ├─ Start monitor
   │  └─ Monitor listens on :8080 (HTTP + WS /metrics)
   └─ Start minibot
      ├─ Load risk policies (risk/policies/example.yaml)
      ├─ Connect to monitor WS (ws://localhost:8080/metrics)
      ├─ Connect to RTDS WS (wss://ws-live-data.polymarket.com)
      ├─ Subscribe to market(s)
      └─ Begin emitting metrics
```

### 6.2 Metrics Flow

```
RTDS ──(msg)──> minibot ──(process)──> minibot
                   │                      │
                   │                      │
              (calculate lag)       (evaluate risk)
                   │                      │
                   └──────┬───────────────┘
                          │
                   (emit metrics)
                          │
                          ▼
                    monitor WS (/metrics)
                          │
                    (store in-memory)
                          │
                   (broadcast to dashboard)
                          │
                          ▼
                    browser WS (/dashboard)
                          │
                    (render charts)
```

### 6.3 Error Handling Strategy

**RTDS Connection Lost:**
- minibot logs error
- minibot attempts reconnect after 2s delay
- minibot emits metric: `polymarket.rtds.connection_status = 0`

**Monitor Connection Lost:**
- minibot logs error
- minibot buffers metrics (up to 1000)
- minibot attempts reconnect after 1s delay
- On reconnect, flush buffered metrics

**Invalid Metrics Message:**
- monitor logs warning
- monitor drops message
- monitor continues processing

**Risk Policy Violation:**
- minibot logs violation
- minibot emits metric: `polymarket.risk.decision = 0`
- minibot does NOT execute trade (this is simulation)

---

## 7. TESTING STRATEGY

### 7.1 Unit Tests

**core/:**
- Test ring buffer wrapping
- Test timestamp ordering
- Test query_range edge cases
- Test NULL pointer handling

**risk/:**
- Test each policy type independently
- Test policy combinations
- Test kill-switch state transitions
- Test simulator position tracking

**monitor/:**
- Test WebSocket message parsing
- Test metrics storage
- Test broadcast to multiple clients

### 7.2 Integration Tests

**minibot + monitor:**
```bash
# Start monitor
./monitor/bin/monitor &

# Run minibot for 10 seconds
timeout 10s ./examples/minibot/target/release/minibot

# Verify metrics appeared in monitor logs
```

**End-to-End:**
```bash
# Run full stack
./run_local.sh &

# Open browser to http://localhost:8080
# Verify charts are updating

# Stop stack
kill %1
```

### 7.3 Manual Testing Checklist

- [ ] RTDS connection establishes
- [ ] RTDS messages appear in minibot logs
- [ ] Metrics appear in monitor logs
- [ ] Dashboard charts update in real-time
- [ ] Disconnect RTDS, verify reconnect
- [ ] Disconnect monitor, verify minibot reconnects
- [ ] Trigger mock risk violation, verify metric emitted
- [ ] Load test: 100+ msgs/sec from RTDS

---

## 8. POLYMARKET-SPECIFIC CONSTRAINTS

### 8.1 What EXISTS
- RTDS WebSocket: `wss://ws-live-data.polymarket.com`
- Message format: `{ topic, type, timestamp, payload }`
- Topics: `market`, `user` (auth required)
- Dynamic subscriptions supported

### 8.2 What DOES NOT EXIST (Do Not Invent)
- No generic CEX/DEX abstractions
- No REST API for live data (CLOB API is separate, not used in MVP)
- No built-in reconnect logic (we must implement)
- No authentication for public market data

### 8.3 Naming Conventions
- All metrics must be prefixed: `polymarket.*`
- Use Polymarket terminology:
  - "market" not "symbol" or "pair"
  - "position" not "order" (for inventory)
  - "RTDS" not "websocket feed"

---

## 9. FUTURE EXTENSIONS (Out of Scope for MVP)

**Not Implementing Now:**
- CLOB API integration (order placement)
- Persistent storage (database)
- Distributed deployment
- Authentication/user-specific streams
- Advanced risk models (VaR, Greeks)
- Multi-agent coordination
- Configuration hot-reload

**These are explicitly deferred.**

---

## 10. DEFINITION OF DONE - SYSTEM LEVEL

**MVP is complete when:**

- [ ] `make all` builds all components without errors
- [ ] `make test` passes all unit tests
- [ ] `./run_local.sh` starts monitor and minibot
- [ ] Dashboard loads at `http://localhost:8080`
- [ ] At least 2 charts visible and updating
- [ ] minibot connects to Polymarket RTDS
- [ ] minibot emits all 5 metric types
- [ ] Risk policies load and evaluate
- [ ] All 4 module README.md files exist with examples
- [ ] No compiler warnings (C, Rust, Go)
- [ ] Repository includes this MULTI_AGENT_PLAN.md

**Success Criteria:**
- System runs for 5+ minutes without crashes
- RTDS lag metric shows realistic values (<500ms)
- Messages/sec metric tracks RTDS throughput
- Risk decisions reflect policy logic
- Dashboard remains responsive with 100+ metrics/sec

---

## 11. COMMUNICATION PROTOCOL BETWEEN AGENTS

**When to Update This Document:**
- Before implementing a new module interface
- After discovering integration issues
- When metrics protocol changes
- When adding new policy types

**How Agents Should Collaborate:**
1. Each agent works in assigned directory
2. Agents MUST NOT modify other agents' directories
3. Integration issues → update this document FIRST
4. Contract changes → notify affected agents via this document
5. All cross-module dependencies documented in Section 3

**Conflict Resolution:**
- Interface disputes → System Architect decides (this document is source of truth)
- Build conflicts → Root Makefile mediates
- Metric naming → Must follow Section 2.3 format

---

## 12. REVISION HISTORY

| Date       | Version | Changes                        | Author          |
|------------|---------|--------------------------------|-----------------|
| 2025-12-31 | 1.0     | Initial architecture           | System Architect|

---

**END OF MULTI_AGENT_PLAN.md**
