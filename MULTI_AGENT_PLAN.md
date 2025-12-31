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

## 12. ROADMAP ARCHITECTURE - PRODUCTION FEATURES

### 12.1 Overview

The following sections define the architecture for production roadmap features that extend beyond the MVP. Each feature introduces new specialized agents and integration points while maintaining strict module boundaries.

**Roadmap Items:**
1. CLOB API Integration (Order Placement)
2. Persistent Storage (TimescaleDB)
3. Advanced Risk Models (VaR, Greeks)
4. Multi-Market Strategy Support
5. Production Deployment Tooling

**Guiding Principles:**
- Architect as orchestrator - all work coordinated through MULTI_AGENT_PLAN.md
- Specialized agents work exclusively in assigned directories
- Clear interface contracts between all modules
- Implementation order follows dependency graph
- Each feature has measurable definition of done

---

### 12.2 Feature: CLOB API Integration (Order Placement)

**Purpose:** Enable real order placement on Polymarket CLOB and other exchanges.

**New Agent:** `exec-gateway`
- **Location:** `.claude/agents/exec-gateway.md`
- **Working Directory:** `exec/`
- **Responsibilities:**
  - Implement venue-specific API clients (Polymarket CLOB, CEX, DEX)
  - Design order management system (OMS) with state tracking
  - Build rate limiting and throttling infrastructure
  - Create unified venue adapter interface
  - Integrate pre-trade risk checks from risk/ module
  - Emit execution metrics to monitor/ module

**Module Structure:**
```
exec/
├── src/
│   ├── lib.rs              # ExecutionEngine API
│   ├── engine.rs           # Main execution orchestration
│   ├── order.rs            # Order types
│   └── error.rs            # Error handling
├── venues/
│   ├── polymarket.rs       # Polymarket CLOB adapter
│   ├── binance.rs          # CEX adapter (example)
│   └── uniswap.rs          # DEX adapter (example)
├── oms/
│   ├── tracker.rs          # Order state tracking
│   └── validator.rs        # Order validation
├── ratelimit/
│   ├── limiter.rs          # Rate limiter implementation
│   └── config.rs           # Per-venue rate configs
├── adapters/
│   ├── trait.rs            # VenueAdapter trait
│   └── normalize.rs        # Data normalization
└── tests/
```

**Interface Contracts:**

**Input from strategies/:**
```rust
pub struct Order {
    pub venue: VenueId,
    pub market: MarketId,
    pub side: Side,
    pub order_type: OrderType,
    pub price: Option<f64>,
    pub size: f64,
    pub time_in_force: TimeInForce,
}
```

**Output to strategies/:**
```rust
pub struct OrderAck {
    pub order_id: OrderId,
    pub status: OrderStatus,
    pub timestamp: DateTime<Utc>,
}

pub struct Fill {
    pub order_id: OrderId,
    pub price: f64,
    pub size: f64,
    pub fee: f64,
    pub timestamp: DateTime<Utc>,
}
```

**Integration with risk/:**
```rust
// Pre-trade risk check before order submission
let risk_decision = risk_engine.evaluate(&RiskContext {
    market_id: order.market.clone(),
    current_position: get_position(&order.market),
    proposed_size: order.size,
    inventory_value_usd: get_total_inventory(),
}).await?;

if !risk_decision.allowed {
    return Err(ExecError::RiskRejected(risk_decision.violated_policies));
}
```

**Integration with monitor/:**
```json
// Execution metrics emitted to monitor WebSocket
{
  "timestamp": 1735689600000,
  "metric_type": "gauge",
  "metric_name": "exec.latency_ms",
  "value": 45.2,
  "labels": {"venue": "polymarket", "market": "0x123abc"}
}

{
  "timestamp": 1735689600100,
  "metric_type": "counter",
  "metric_name": "exec.orders_placed",
  "value": 1,
  "labels": {"venue": "polymarket", "side": "buy"}
}
```

**Dependencies:**
- Depends on: risk/ (pre-trade checks), monitor/ (metrics)
- Depended on by: strategies/ (order execution)

**Implementation Order:**
1. Define VenueAdapter trait and ExecutionEngine API
2. Implement Polymarket CLOB adapter
3. Build OMS state tracking
4. Integrate risk engine pre-trade checks
5. Add rate limiting
6. Implement metrics emission
7. Integration tests with mock venues
8. End-to-end tests with Polymarket testnet

**Definition of Done:**
- [ ] VenueAdapter trait defined and documented
- [ ] Polymarket CLOB adapter fully functional
- [ ] Order state machine tracks full lifecycle
- [ ] Pre-trade risk checks integrated
- [ ] Rate limiting prevents API violations
- [ ] Execution metrics emitted to monitor
- [ ] Integration tests pass
- [ ] README with API usage examples

---

### 12.3 Feature: Persistent Storage (TimescaleDB)

**Purpose:** Store metrics, execution history, and positions in persistent database.

**New Agent:** `storage-layer`
- **Location:** `.claude/agents/storage-layer.md`
- **Working Directory:** `storage/`
- **Responsibilities:**
  - Implement TimescaleDB connection and pooling
  - Design hypertable schemas for metrics and execution data
  - Build high-throughput ingestion pipeline
  - Create query API for historical data
  - Implement data retention and compression policies
  - Design database migration system

**Module Structure:**
```
storage/
├── src/
│   ├── lib.rs              # StorageEngine API
│   ├── engine.rs           # Main storage engine
│   ├── execution.rs        # ExecutionStore
│   ├── config.rs           # Configuration
│   └── error.rs            # Error types
├── timescale/
│   ├── connection.rs       # Connection pooling
│   ├── query.rs            # Query builders
│   └── migrations.rs       # Migration runner
├── schemas/
│   ├── metrics.sql         # Metrics hypertable
│   ├── execution.sql       # Orders/fills/positions
│   └── migrations/         # Versioned migrations
├── ingest/
│   ├── buffer.rs           # Buffered insertion
│   └── pipeline.rs         # Ingestion pipeline
├── query/
│   ├── metrics.rs          # Metric queries
│   ├── execution.rs        # Execution queries
│   └── cache.rs            # Query caching
├── retention/
│   ├── policy.rs           # Retention policies
│   └── cleanup.rs          # Cleanup jobs
└── docker-compose.yml      # Local TimescaleDB
```

**Schema Design:**

```sql
-- Metrics hypertable
CREATE TABLE metrics (
    timestamp TIMESTAMPTZ NOT NULL,
    metric_name TEXT NOT NULL,
    value DOUBLE PRECISION NOT NULL,
    labels JSONB
);
SELECT create_hypertable('metrics', 'timestamp');

-- Orders table
CREATE TABLE orders (
    id UUID PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    venue TEXT NOT NULL,
    market TEXT NOT NULL,
    side TEXT NOT NULL,
    price DOUBLE PRECISION,
    size DOUBLE PRECISION NOT NULL,
    status TEXT NOT NULL
);
SELECT create_hypertable('orders', 'timestamp');

-- Fills table
CREATE TABLE fills (
    id UUID PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    order_id UUID REFERENCES orders(id),
    price DOUBLE PRECISION NOT NULL,
    size DOUBLE PRECISION NOT NULL,
    fee DOUBLE PRECISION NOT NULL
);
SELECT create_hypertable('fills', 'timestamp');
```

**Interface Contracts:**

**Integration with monitor/:**
```rust
// Monitor pushes metrics to storage
storage_engine.insert_metrics_batch(vec![
    MetricPoint {
        timestamp: Utc::now(),
        metric_name: "polymarket.rtds.lag_ms".to_string(),
        value: 45.3,
        labels: hashmap!{"topic" => "market"},
    }
]).await?;
```

**Integration with exec/:**
```rust
// Exec stores order and fill data
execution_store.store_order(order).await?;
execution_store.store_fill(fill).await?;
```

**Query API for analysis:**
```rust
// Query historical metrics
let metrics = storage_engine.query_metrics(
    "polymarket.rtds.lag_ms",
    start_time,
    end_time,
    Some(hashmap!{"topic" => "market"}),
).await?;

// Query execution history
let orders = execution_store.query_orders(
    start_time,
    end_time,
    OrderFilters { venue: Some("polymarket"), ..Default::default() },
).await?;
```

**Dependencies:**
- Depends on: None (infrastructure layer)
- Depended on by: monitor/ (metrics persistence), exec/ (execution history)

**Implementation Order:**
1. Set up TimescaleDB docker-compose
2. Design and create hypertable schemas
3. Implement connection pooling
4. Build batch ingestion pipeline
5. Create query API
6. Add compression and retention policies
7. Build migration system
8. Integration tests with real database

**Definition of Done:**
- [ ] TimescaleDB running in docker-compose
- [ ] Metrics and execution schemas created
- [ ] Batch insertion >10k metrics/sec
- [ ] Query API with time-range and filters
- [ ] Compression after 7 days
- [ ] Retention policy (90 days metrics, 365 days execution)
- [ ] Migration system with up/down scripts
- [ ] Integration tests pass
- [ ] README with setup and usage

---

### 12.4 Feature: Advanced Risk Models (VaR, Greeks)

**Purpose:** Implement quantitative risk models for sophisticated risk assessment.

**New Agent:** `advanced-risk`
- **Location:** `.claude/agents/advanced-risk.md`
- **Working Directory:** `risk/src/advanced/`
- **Responsibilities:**
  - Implement Value at Risk (VaR) models (Historical, Parametric, Monte Carlo)
  - Build Greeks calculation engine (Delta, Gamma, Vega, Theta, Rho)
  - Create portfolio risk analytics
  - Design stress testing framework
  - Calculate performance metrics (Sharpe, Sortino, max drawdown)
  - Integrate with base risk engine

**Module Structure:**
```
risk/src/advanced/
├── mod.rs              # Advanced module exports
├── var.rs              # VaR engines
├── greeks.rs           # Greeks calculation
├── portfolio.rs        # Portfolio analytics
├── stress.rs           # Stress testing
├── metrics.rs          # Performance metrics
├── models.rs           # Model integration
└── error.rs            # Advanced error types

risk/tests/advanced/
├── var_tests.rs        # VaR validation tests
├── greeks_tests.rs     # Greeks accuracy tests
└── portfolio_tests.rs  # Portfolio analytics tests

risk/docs/advanced/
├── VAR_METHODOLOGY.md
├── GREEKS_GUIDE.md
└── STRESS_TESTING.md
```

**Interface Contracts:**

**VaR Calculation:**
```rust
pub struct VarEngine {
    config: VarConfig,
    historical_returns: Vec<f64>,
}

impl VarEngine {
    pub fn calculate_historical_var(
        &self,
        portfolio_value: f64,
        confidence_level: f64,  // e.g., 0.95, 0.99
        time_horizon_days: u32,
    ) -> Result<VarResult, RiskError>;

    pub fn calculate_monte_carlo_var(
        &self,
        portfolio_value: f64,
        mean_return: f64,
        volatility: f64,
        confidence_level: f64,
        time_horizon_days: u32,
        num_simulations: usize,
    ) -> Result<VarResult, RiskError>;
}
```

**Greeks Calculation:**
```rust
pub struct GreeksEngine {
    config: GreeksConfig,
}

impl GreeksEngine {
    pub fn calculate_greeks(
        &self,
        option: &Option,
        underlying_price: f64,
        volatility: f64,
        risk_free_rate: f64,
    ) -> Result<Greeks, RiskError>;

    pub fn calculate_portfolio_greeks(
        &self,
        positions: &[OptionPosition],
        market_data: &MarketData,
    ) -> Result<PortfolioGreeks, RiskError>;
}

#[derive(Debug, Clone)]
pub struct Greeks {
    pub delta: f64,
    pub gamma: f64,
    pub vega: f64,
    pub theta: f64,
    pub rho: f64,
}
```

**Integration with base risk engine:**
```rust
// Extend PolicyRule enum
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum PolicyRule {
    // Existing policies
    PositionLimit { ... },
    InventoryLimit { ... },
    KillSwitch { ... },

    // Advanced risk policies
    VarLimit {
        max_var_usd: f64,
        confidence_level: f64,
        time_horizon_days: u32,
    },
    GreeksLimit {
        max_delta: f64,
        max_gamma: f64,
        max_vega: f64,
    },
}
```

**Integration with exec/:**
```rust
// Provide hedging recommendations
let hedge_recommendations = greeks_engine.suggest_hedge(
    &current_portfolio_greeks,
    &target_greeks,
    &available_instruments,
).await?;
```

**Integration with monitor/:**
```json
// Emit advanced risk metrics
{
  "timestamp": 1735689600000,
  "metric_type": "gauge",
  "metric_name": "risk.var_95",
  "value": 1250.50,
  "labels": {"time_horizon": "1d", "method": "historical"}
}

{
  "timestamp": 1735689600000,
  "metric_type": "gauge",
  "metric_name": "risk.portfolio_delta",
  "value": 0.45,
  "labels": {}
}
```

**Dependencies:**
- Depends on: base risk/ (extends existing policies)
- Depended on by: exec/ (hedging recommendations), strategies/ (risk-aware trading)

**Implementation Order:**
1. Implement Historical VaR
2. Add Parametric and Monte Carlo VaR
3. Build Black-Scholes Greeks calculation
4. Create portfolio analytics
5. Design stress testing scenarios
6. Integrate with base PolicyRule enum
7. Add VaR backtesting framework
8. Mathematical validation tests

**Definition of Done:**
- [ ] VaR models (Historical, Parametric, Monte Carlo) implemented
- [ ] Greeks calculation validated against benchmarks
- [ ] Portfolio volatility and correlation matrix calculation
- [ ] Stress testing with 3+ scenarios
- [ ] Performance metrics (Sharpe, Sortino, max drawdown)
- [ ] Integration with base risk engine
- [ ] VaR backtesting shows <5% violations at 95% confidence
- [ ] Mathematical validation tests pass
- [ ] Documentation of all formulas
- [ ] README with methodology

---

### 12.5 Feature: Multi-Market Strategy Support

**Purpose:** Enable trading strategies across multiple markets and venues simultaneously.

**New Agent:** `strategy-engine`
- **Location:** `.claude/agents/strategy-engine.md`
- **Working Directory:** `strategies/`
- **Responsibilities:**
  - Design base Strategy trait with lifecycle hooks
  - Build multi-market coordinator
  - Create signal generation framework
  - Implement reference strategies (market making, arbitrage, trend following)
  - Build backtesting engine
  - Design strategy metrics and monitoring

**Module Structure:**
```
strategies/
├── src/
│   ├── lib.rs              # Strategy trait and core types
│   ├── context.rs          # StrategyContext
│   ├── coordinator.rs      # MultiMarketCoordinator
│   ├── error.rs            # Strategy errors
│   └── metrics.rs          # Strategy metrics
├── framework/
│   ├── lifecycle.rs        # Lifecycle management
│   ├── params.rs           # Parameter handling
│   └── versioning.rs       # Strategy versioning
├── multimarket/
│   ├── arbitrage.rs        # Arbitrage detection
│   ├── routing.rs          # Order routing
│   └── inventory.rs        # Cross-market inventory
├── signals/
│   ├── technical.rs        # Technical indicators
│   ├── microstructure.rs   # Market microstructure
│   └── composite.rs        # Signal composition
├── impl/
│   ├── market_maker.rs     # Market making
│   ├── trend.rs            # Trend following
│   ├── mean_reversion.rs   # Mean reversion
│   ├── stat_arb.rs         # Statistical arbitrage
│   └── execution.rs        # TWAP/VWAP
└── backtest/
    ├── engine.rs           # Backtesting engine
    ├── fill_sim.rs         # Fill simulation
    └── optimizer.rs        # Parameter optimization
```

**Interface Contracts:**

**Strategy Trait:**
```rust
#[async_trait]
pub trait Strategy: Send + Sync {
    async fn initialize(&mut self, ctx: &mut StrategyContext) -> Result<(), StrategyError>;

    async fn on_market_tick(
        &mut self,
        market_id: &str,
        tick: &MarketTick,
        ctx: &mut StrategyContext,
    ) -> Result<(), StrategyError>;

    async fn on_fill(
        &mut self,
        fill: &Fill,
        ctx: &mut StrategyContext,
    ) -> Result<(), StrategyError>;

    async fn on_cancel(
        &mut self,
        order_id: &OrderId,
        ctx: &mut StrategyContext,
    ) -> Result<(), StrategyError>;

    fn metadata(&self) -> StrategyMetadata;
}
```

**Strategy Context:**
```rust
pub struct StrategyContext {
    pub strategy_id: String,
    pub exec_engine: Arc<Mutex<ExecutionEngine>>,
    pub risk_engine: Arc<Mutex<RiskEngine>>,
    pub positions: HashMap<String, Position>,
    pub orders: HashMap<OrderId, Order>,
}

impl StrategyContext {
    pub async fn submit_order(&mut self, order: Order) -> Result<OrderId, StrategyError>;
    pub async fn cancel_order(&mut self, order_id: &OrderId) -> Result<(), StrategyError>;
    pub fn get_position(&self, market_id: &str) -> Option<&Position>;
}
```

**Multi-Market Coordination:**
```rust
pub struct MultiMarketCoordinator {
    strategies: HashMap<String, Box<dyn Strategy>>,
    market_subscriptions: HashMap<String, Vec<String>>,
}

impl MultiMarketCoordinator {
    pub async fn register_strategy(
        &mut self,
        strategy_id: String,
        strategy: Box<dyn Strategy>,
        markets: Vec<String>,
    ) -> Result<(), StrategyError>;

    pub async fn route_market_tick(
        &mut self,
        market_id: &str,
        tick: &MarketTick,
    ) -> Result<(), StrategyError>;
}
```

**Integration with exec/:**
```rust
// Submit orders via ExecutionEngine
let order_id = ctx.exec_engine.lock().await.submit_order(order).await?;
```

**Integration with risk/:**
```rust
// Risk checks performed before order submission
let risk_decision = ctx.risk_engine.lock().await.evaluate(&risk_ctx).await?;
if !risk_decision.allowed {
    return Err(StrategyError::RiskRejected);
}
```

**Integration with monitor/:**
```json
// Strategy metrics
{
  "timestamp": 1735689600000,
  "metric_type": "gauge",
  "metric_name": "strategy.pnl_usd",
  "value": 125.50,
  "labels": {"strategy_id": "mm_strategy_1", "market": "0x123abc"}
}

{
  "timestamp": 1735689600000,
  "metric_type": "counter",
  "metric_name": "strategy.signals_generated",
  "value": 1,
  "labels": {"strategy_id": "arb_strategy_1", "signal_type": "long"}
}
```

**Dependencies:**
- Depends on: exec/ (order execution), risk/ (pre-trade checks), monitor/ (metrics)
- Depended on by: User strategies (extend Strategy trait)

**Implementation Order:**
1. Define Strategy trait and StrategyContext
2. Build MultiMarketCoordinator
3. Implement market making strategy
4. Add cross-market arbitrage strategy
5. Create signal generation framework
6. Build backtesting engine
7. Add strategy metrics emission
8. Integration tests with mock exec/risk

**Definition of Done:**
- [ ] Strategy trait defined with lifecycle hooks
- [ ] StrategyContext integrates exec and risk
- [ ] MultiMarketCoordinator routes market data
- [ ] Market making strategy implemented
- [ ] Cross-market arbitrage strategy implemented
- [ ] Signal framework with 3+ indicators
- [ ] Backtesting engine functional
- [ ] Strategy metrics emitted to monitor
- [ ] Integration tests pass
- [ ] README with strategy development guide

---

### 12.6 Feature: Production Deployment Tooling

**Purpose:** Enable reliable, scalable production deployments with monitoring and observability.

**New Agent:** `devops-infra`
- **Location:** `.claude/agents/devops-infra.md`
- **Working Directories:** `deploy/`, `infra/`
- **Responsibilities:**
  - Create Docker images for all components
  - Design Kubernetes manifests with autoscaling
  - Build CI/CD pipelines (GitHub Actions)
  - Implement Terraform for infrastructure
  - Deploy monitoring stack (Prometheus/Grafana)
  - Create deployment runbooks and disaster recovery

**Module Structure:**
```
deploy/
├── docker/
│   ├── Dockerfile.exec
│   ├── Dockerfile.monitor
│   ├── Dockerfile.strategy
│   ├── docker-compose.yml
│   └── .dockerignore
└── k8s/
    ├── namespace.yaml
    ├── exec-deployment.yaml
    ├── monitor-deployment.yaml
    ├── strategy-deployment.yaml
    ├── timescaledb-statefulset.yaml
    ├── configmaps.yaml
    ├── secrets.yaml
    └── ingress.yaml

infra/
├── terraform/
│   ├── main.tf
│   ├── vpc.tf
│   ├── eks.tf
│   └── rds.tf
├── monitoring/
│   ├── prometheus/
│   ├── grafana/
│   └── alerts/
└── ops/
    ├── runbooks/
    ├── disaster-recovery.md
    └── backup-restore.sh
```

**Kubernetes Architecture:**
```yaml
# Exec Gateway Deployment with HPA
apiVersion: apps/v1
kind: Deployment
metadata:
  name: exec-gateway
spec:
  replicas: 3
  template:
    spec:
      containers:
      - name: exec-gateway
        image: ghcr.io/org/ag-exec:latest
        resources:
          requests:
            cpu: 500m
            memory: 512Mi
          limits:
            cpu: 2000m
            memory: 2Gi
        livenessProbe:
          httpGet:
            path: /health
            port: 8081
        readinessProbe:
          httpGet:
            path: /ready
            port: 8081
---
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: exec-gateway-hpa
spec:
  scaleTargetRef:
    kind: Deployment
    name: exec-gateway
  minReplicas: 3
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        averageUtilization: 70
```

**CI/CD Pipeline:**
```yaml
# .github/workflows/deploy.yml
jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo clippy --all-targets

  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo test --workspace

  build:
    needs: [lint, test]
    runs-on: ubuntu-latest
    steps:
      - uses: docker/build-push-action@v5
        with:
          push: true
          tags: ghcr.io/org/ag-exec:${{ github.sha }}

  deploy:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - run: kubectl apply -f deploy/k8s/
      - run: kubectl rollout status deployment/exec-gateway
```

**Monitoring Stack:**
- Prometheus: Scrape metrics from all services
- Grafana: Dashboards for system health, execution metrics, risk metrics
- AlertManager: Alert on critical conditions
- Loki: Log aggregation

**Integration with all modules:**
```yaml
# All modules expose Prometheus metrics
# All pods have health/readiness endpoints
# All services log to stdout (collected by Loki)
# All deployments have resource limits
```

**Dependencies:**
- Depends on: All modules (deploys everything)
- Depended on by: None (infrastructure layer)

**Implementation Order:**
1. Create Dockerfiles for each component
2. Build docker-compose for local testing
3. Design Kubernetes manifests
4. Implement CI/CD pipeline
5. Create Terraform for AWS/GCP
6. Deploy Prometheus/Grafana
7. Write deployment runbooks
8. Test disaster recovery procedures

**Definition of Done:**
- [ ] Docker images for all components (<500MB)
- [ ] docker-compose for local development
- [ ] Kubernetes manifests with HPA
- [ ] CI/CD pipeline (build, test, deploy)
- [ ] Terraform provisions infrastructure
- [ ] Prometheus/Grafana deployed
- [ ] Alerting rules configured
- [ ] Deployment runbooks written
- [ ] Backup/restore tested
- [ ] Zero-downtime deployment verified
- [ ] README with deployment guide

---

### 12.7 Implementation Roadmap and Dependencies

**Dependency Graph:**
```
Phase 1 (Foundation):
- storage-layer (no dependencies)
  └─> Enables persistent metrics and execution history

Phase 2 (Execution):
- exec-gateway (depends on: risk/, monitor/)
  └─> Enables real order placement
  └─> Integrates with storage-layer for execution history

Phase 3 (Advanced Risk):
- advanced-risk (depends on: base risk/)
  └─> Extends risk engine with quantitative models
  └─> Integrates with exec-gateway for hedging

Phase 4 (Strategies):
- strategy-engine (depends on: exec-gateway, advanced-risk)
  └─> Enables multi-market trading strategies
  └─> Integrates all previous layers

Phase 5 (Production):
- devops-infra (depends on: all modules)
  └─> Deploys entire stack to production
```

**Implementation Timeline:**

**Week 1-2: Storage Layer**
- Set up TimescaleDB
- Implement schemas and ingestion
- Build query API
- Test with monitor metrics

**Week 3-4: Execution Gateway**
- Define ExecutionEngine API
- Implement Polymarket CLOB adapter
- Integrate risk checks
- Add rate limiting and metrics

**Week 5-6: Advanced Risk**
- Implement VaR models
- Build Greeks calculation
- Create portfolio analytics
- Validate with known benchmarks

**Week 7-8: Strategy Engine**
- Define Strategy trait
- Build MultiMarketCoordinator
- Implement 2-3 reference strategies
- Create backtesting framework

**Week 9-10: Production Deployment**
- Containerize all components
- Create Kubernetes manifests
- Build CI/CD pipeline
- Deploy monitoring stack
- Production readiness testing

---

### 12.8 Coordination Strategy

**Architect as Orchestrator:**

The system-architect agent is the single source of truth for all integration and coordination:

1. **Before Implementation:**
   - Architect defines module interfaces in MULTI_AGENT_PLAN.md
   - Architect creates integration contracts
   - Architect approves agent task allocation

2. **During Implementation:**
   - Specialized agents work in assigned directories only
   - Agents NEVER modify other modules
   - Integration issues escalated to architect
   - Architect updates MULTI_AGENT_PLAN.md with decisions

3. **After Implementation:**
   - Architect validates integration points
   - Architect updates system architecture diagrams
   - Architect documents lessons learned

**Agent Boundaries:**

| Agent | Directory | Can Read | Can Write | Must Coordinate With |
|-------|-----------|----------|-----------|----------------------|
| core-c-implementer | core/ | core/ | core/ | architect (API changes) |
| risk-engine | risk/ | risk/ | risk/ | architect (contracts) |
| advanced-risk | risk/src/advanced/ | risk/ | risk/src/advanced/ | risk-engine |
| monitor-ui | monitor/ | monitor/ | monitor/ | architect (protocols) |
| exec-gateway | exec/ | exec/, risk/, monitor/ | exec/ | architect, risk-engine |
| storage-layer | storage/ | storage/ | storage/ | architect (schemas) |
| strategy-engine | strategies/ | strategies/, exec/, risk/ | strategies/ | architect, exec-gateway |
| devops-infra | deploy/, infra/ | all | deploy/, infra/ | architect (all modules) |

**Communication Protocol:**

1. **Interface Changes:**
   - Agent proposes change in implementation
   - Architect reviews and updates MULTI_AGENT_PLAN.md
   - All dependent agents notified via plan update
   - Implementation proceeds after approval

2. **Integration Issues:**
   - Agent documents issue with affected modules
   - Architect analyzes and redesigns integration
   - Architect updates contracts in MULTI_AGENT_PLAN.md
   - Agents implement per updated contracts

3. **New Features:**
   - User requests feature
   - Architect assigns to appropriate agent(s)
   - Architect defines integration points
   - Agents implement within boundaries
   - Architect validates integration

---

### 12.9 Quality Gates

**Code Quality:**
- All agents: No compiler warnings
- Rust: `cargo clippy` passes
- C: Compiles with `-Wall -Wextra -Wpedantic`
- Go: `go vet` passes

**Testing:**
- Unit test coverage: >80%
- Integration tests: All critical paths covered
- E2E tests: Happy path + error scenarios
- Performance tests: Meet defined targets

**Documentation:**
- Every module has README.md
- All public APIs documented
- Integration points described
- Examples provided

**Security:**
- No secrets in code
- All dependencies scanned
- Container images hardened
- Network policies defined

---

## 13. REVISION HISTORY

| Date       | Version | Changes                        | Author          |
|------------|---------|--------------------------------|-----------------|
| 2025-12-31 | 1.0     | Initial architecture           | System Architect|
| 2025-12-31 | 2.0     | Roadmap architecture added     | System Architect|
|            |         | - CLOB API integration         |                 |
|            |         | - TimescaleDB storage          |                 |
|            |         | - Advanced risk models         |                 |
|            |         | - Multi-market strategies      |                 |
|            |         | - Production deployment        |                 |
|            |         | - 5 new specialized agents     |                 |

---

**END OF MULTI_AGENT_PLAN.md**
