# Monitor - Real-Time Polymarket Dashboard

Lightweight Go-based monitoring dashboard for ag-botkit trading system with WebSocket streaming and live charts.

## Architecture

```
┌─────────────┐     WebSocket      ┌─────────────┐     WebSocket     ┌─────────────┐
│   minibot   │ ──────────────────▶│   monitor   │──────────────────▶│   Browser   │
│   (Python)  │  /metrics endpoint │  (Go Server)│ /dashboard endpoint│  (uPlot UI) │
└─────────────┘                    └─────────────┘                   └─────────────┘
                                          │
                                          ▼
                                   In-Memory Storage
                                   (Ring Buffers)
```

## Features

- **WebSocket Metrics Ingestion**: High-performance endpoint for receiving metrics from trading bots
- **Real-Time Broadcasting**: Live metric distribution to dashboard clients
- **Efficient Storage**: Ring buffer implementation with configurable capacity per metric
- **Lightweight UI**: Vanilla JavaScript with uPlot for minimal overhead
- **Zero Dependencies**: Standard library + gorilla/websocket only
- **Auto-Reconnection**: Built-in reconnection logic for resilient connections

## Quick Start

### Build

```bash
cd /Users/borkiss../ag-botkit/monitor
go mod download
go build -o bin/monitor ./cmd/monitor
```

### Run

```bash
# Default (localhost:8080, auto-detects web directory)
./bin/monitor

# Custom configuration
./bin/monitor -addr localhost:9090 -capacity 5000

# Specify web directory (useful when running from project root)
./bin/monitor -web /Users/borkiss../ag-botkit/monitor/web

# From project root
cd /Users/borkiss../ag-botkit
./monitor/bin/monitor -web ./monitor/web
```

**Flags:**
- `-addr` - HTTP server address (default: `localhost:8080`)
- `-capacity` - Metric storage capacity per metric (default: `10000`)
- `-web` - Web directory path (auto-detects `./web` or `../../web` if not specified)

### Test

```bash
go test ./...
```

## API Endpoints

### HTTP Endpoints

- `GET /` - Dashboard UI (HTML/CSS/JS)
- `GET /static/*` - Static assets

### WebSocket Endpoints

#### 1. Metrics Ingestion: `ws://localhost:8080/metrics`

**Purpose**: Receive metrics from trading bots

**Protocol**: JSON messages (one per message)

**Message Format**:
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

**Field Definitions**:
- `timestamp` (int64): Unix milliseconds
- `metric_type` (string): "counter", "gauge", or "histogram"
- `metric_name` (string): Metric identifier (see Metrics section)
- `value` (float64): Metric value
- `labels` (object, optional): Key-value dimensions

**Example Client** (Go):
```go
import (
    "encoding/json"
    "github.com/gorilla/websocket"
)

conn, _, err := websocket.DefaultDialer.Dial("ws://localhost:8080/metrics", nil)
if err != nil {
    log.Fatal(err)
}

metric := map[string]interface{}{
    "timestamp":   time.Now().UnixMilli(),
    "metric_type": "gauge",
    "metric_name": "polymarket.rtds.lag_ms",
    "value":       23.5,
    "labels":      map[string]string{"topic": "market"},
}

data, _ := json.Marshal(metric)
conn.WriteMessage(websocket.TextMessage, data)
```

**Example Client** (Python):
```python
import json
import time
import websocket

ws = websocket.create_connection("ws://localhost:8080/metrics")

metric = {
    "timestamp": int(time.time() * 1000),
    "metric_type": "gauge",
    "metric_name": "polymarket.rtds.lag_ms",
    "value": 23.5,
    "labels": {"topic": "market"}
}

ws.send(json.dumps(metric))
```

#### 2. Dashboard Broadcast: `ws://localhost:8080/dashboard`

**Purpose**: Subscribe to real-time metric updates

**Protocol**: Server-to-client JSON messages (receive-only)

**Behavior**:
- On connection: Receives last 60 seconds of metrics
- Real-time: Receives all new metrics as they arrive
- Auto-ping: Keep-alive every 54 seconds

**Message Format**: Same as metrics ingestion

## Supported Metrics

### RTDS Connection Metrics

**Messages Received**:
```json
{
  "metric_type": "counter",
  "metric_name": "polymarket.rtds.messages_received",
  "value": 1,
  "labels": {"topic": "market", "message_type": "book"}
}
```

**WebSocket Lag**:
```json
{
  "metric_type": "gauge",
  "metric_name": "polymarket.rtds.lag_ms",
  "value": 45.3,
  "labels": {"topic": "market"}
}
```

**Messages Per Second**:
```json
{
  "metric_type": "gauge",
  "metric_name": "polymarket.rtds.msgs_per_second",
  "value": 23.5
}
```

### Position Metrics

**Position Size**:
```json
{
  "metric_type": "gauge",
  "metric_name": "polymarket.position.size",
  "value": 150.0,
  "labels": {"market_id": "0x123abc", "side": "long"}
}
```

**Inventory Value**:
```json
{
  "metric_type": "gauge",
  "metric_name": "polymarket.inventory.value_usd",
  "value": 1250.75
}
```

### Risk Metrics

**Risk Decision**:
```json
{
  "metric_type": "gauge",
  "metric_name": "polymarket.risk.decision",
  "value": 1,
  "labels": {"policy": "position_limit", "market_id": "0x123abc"}
}
```
- `value = 1`: Allowed
- `value = 0`: Blocked

**Kill Switch**:
```json
{
  "metric_type": "gauge",
  "metric_name": "polymarket.risk.kill_switch",
  "value": 0
}
```
- `value = 0`: Off
- `value = 1`: Triggered

## Dashboard Charts

The UI displays 6 real-time visualizations:

1. **RTDS Lag (ms)** - Line chart showing WebSocket latency
2. **Messages Per Second** - Throughput monitoring
3. **Position Size** - Current position tracking
4. **Risk Decisions** - Allow/block timeline
5. **Messages Received** - Cumulative counter
6. **Kill Switch** - Visual indicator (green=off, red=on)

All charts use **uPlot** for high-performance rendering with minimal CPU usage.

## Configuration

### Command-Line Flags

```bash
./bin/monitor \
  -addr localhost:8080 \    # HTTP server address
  -capacity 10000           # Ring buffer capacity per metric
```

### Environment Variables

Not currently supported. All configuration via flags.

## Performance

**Benchmarks** (M1 Mac):

```
BenchmarkRingBuffer_Append-8    50000000    28.5 ns/op
BenchmarkMetricStore_Append-8   10000000    127 ns/op
```

**Capacity**:
- Default: 10,000 points per metric
- Memory usage: ~240 KB per metric (10K points × 24 bytes/point)
- Tested with 100+ metrics at 1000 msgs/sec

**WebSocket**:
- No message buffering (real-time)
- Slow clients automatically disconnected
- Broadcast channel: 256 message buffer

## Project Structure

```
monitor/
├── cmd/
│   └── monitor/
│       └── main.go              # Entry point
├── internal/
│   ├── server/
│   │   ├── http.go              # HTTP handlers
│   │   └── websocket.go         # WebSocket hub and clients
│   └── storage/
│       ├── metrics.go           # Ring buffer storage
│       └── metrics_test.go      # Unit tests
├── web/
│   ├── index.html               # Dashboard UI
│   └── static/
│       ├── style.css            # Styling
│       └── app.js               # Chart rendering
├── go.mod
├── go.sum
└── README.md
```

## Testing

### Unit Tests

```bash
go test ./internal/storage -v
```

**Coverage**:
```
PASS: TestRingBuffer_Append
PASS: TestRingBuffer_GetLast
PASS: TestRingBuffer_GetRange
PASS: TestRingBuffer_Wrapping
PASS: TestMetricStore_Append
PASS: TestMetricStore_MultipleMetrics
PASS: TestMetricStore_GetRecentMetrics
PASS: TestMetricStore_GetNonExistent
```

### Manual Testing

**Test Script** (send test metrics):
```bash
# Send test metric using websocat
echo '{"timestamp":1735689600000,"metric_type":"gauge","metric_name":"polymarket.rtds.lag_ms","value":45.3}' | \
  websocat ws://localhost:8080/metrics
```

**Test with Python**:
```python
import websocket
import json
import time
import random

ws = websocket.create_connection("ws://localhost:8080/metrics")

# Send 100 test metrics
for i in range(100):
    metric = {
        "timestamp": int(time.time() * 1000),
        "metric_type": "gauge",
        "metric_name": "polymarket.rtds.lag_ms",
        "value": random.uniform(10, 50),
        "labels": {"topic": "market"}
    }
    ws.send(json.dumps(metric))
    time.sleep(0.1)
```

## Integration with minibot

The minibot (examples/minibot/) connects to monitor and streams metrics:

```python
# In minibot
import websocket
import json

ws = websocket.create_connection("ws://localhost:8080/metrics")

def emit_metric(name, value, labels=None):
    metric = {
        "timestamp": int(time.time() * 1000),
        "metric_type": "gauge",
        "metric_name": name,
        "value": value,
        "labels": labels or {}
    }
    ws.send(json.dumps(metric))

# Example usage
emit_metric("polymarket.rtds.lag_ms", 23.5, {"topic": "market"})
emit_metric("polymarket.position.size", 150.0, {"market_id": "0x123"})
```

## Error Handling

### Client Disconnection

**Metrics endpoint**: Connection terminates, no retry (client should reconnect)

**Dashboard endpoint**: Server sends periodic pings (54s interval). Client disconnects if no pong received within 60s.

### Invalid Metrics

Malformed JSON messages are logged and dropped:

```
2025/12/31 12:00:00 Error parsing metric: invalid character '}' (message: {bad json})
```

### Slow Clients

Dashboard clients with full send buffers (256 messages) are automatically disconnected to prevent memory buildup.

## Deployment

### Standalone

```bash
# Build
go build -o bin/monitor ./cmd/monitor

# Run (foreground)
./bin/monitor -addr :8080

# Run (background)
nohup ./bin/monitor -addr :8080 > monitor.log 2>&1 &
```

### With Docker

```dockerfile
FROM golang:1.21-alpine AS builder
WORKDIR /app
COPY go.* ./
RUN go mod download
COPY . .
RUN go build -o monitor ./cmd/monitor

FROM alpine:latest
RUN apk --no-cache add ca-certificates
WORKDIR /root/
COPY --from=builder /app/monitor .
EXPOSE 8080
CMD ["./monitor", "-addr", "0.0.0.0:8080"]
```

```bash
docker build -t ag-botkit-monitor .
docker run -p 8080:8080 ag-botkit-monitor
```

### With systemd

```ini
# /etc/systemd/system/ag-monitor.service
[Unit]
Description=ag-botkit Monitor Dashboard
After=network.target

[Service]
Type=simple
User=botkit
WorkingDirectory=/opt/ag-botkit/monitor
ExecStart=/opt/ag-botkit/monitor/bin/monitor -addr localhost:8080
Restart=on-failure
RestartSec=5s

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl enable ag-monitor
sudo systemctl start ag-monitor
sudo systemctl status ag-monitor
```

## Troubleshooting

### Port Already in Use

```bash
# Check what's using port 8080
lsof -i :8080

# Use different port
./bin/monitor -addr localhost:9090
```

### WebSocket Connection Refused

```bash
# Verify server is running
curl http://localhost:8080/

# Check firewall
sudo ufw allow 8080
```

### Charts Not Updating

1. Open browser console (F12)
2. Check for WebSocket errors
3. Verify metrics are being sent to `/metrics` endpoint
4. Check server logs for parsing errors

### High Memory Usage

Reduce ring buffer capacity:
```bash
./bin/monitor -capacity 1000  # Default is 10000
```

## License

Part of ag-botkit project. See root LICENSE file.

## Contributing

This module follows the MULTI_AGENT_PLAN.md contract. Changes to WebSocket protocol or metric format require updating the plan document.

**Key Constraints**:
- No heavy frontend frameworks (React/Vue/Angular)
- Use uPlot for charting (lightweight requirement)
- Minimal Go dependencies (stdlib + gorilla/websocket only)
- All metrics must be prefixed with "polymarket.*"

## Links

- Project: https://github.com/your-org/ag-botkit
- uPlot Docs: https://github.com/leeoniya/uPlot
- WebSocket RFC: https://datatracker.ietf.org/doc/html/rfc6455
