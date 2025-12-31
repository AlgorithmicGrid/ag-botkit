# Monitor Dashboard - Quick Start Guide

Get the Polymarket monitoring dashboard up and running in 2 minutes.

## Prerequisites

- Go 1.21+ installed
- Python 3.x (optional, for testing)

## Step 1: Build

```bash
cd /Users/borkiss../ag-botkit/monitor
make build
```

Expected output:
```
Building monitor...
Build complete: bin/monitor
```

## Step 2: Run

```bash
./bin/monitor
```

Expected output:
```
2025/12/31 00:26:00 Serving static files from: /Users/borkiss../ag-botkit/monitor/web
2025/12/31 00:26:00 Starting monitor server on localhost:8080
2025/12/31 00:26:00 Dashboard: http://localhost:8080
2025/12/31 00:26:00 Metrics WS: ws://localhost:8080/metrics
2025/12/31 00:26:00 Dashboard WS: ws://localhost:8080/dashboard
```

## Step 3: View Dashboard

Open your browser to:
```
http://localhost:8080
```

You should see:
- 6 chart panels (RTDS Lag, Messages/sec, Position Size, Risk Decisions, Messages Received, Kill Switch)
- Status indicator showing "Connecting..." or "Connected"
- Empty charts waiting for data

## Step 4: Send Test Metrics (Optional)

Install Python WebSocket library:
```bash
pip3 install websocket-client
```

Run the test script:
```bash
python3 test_metrics.py
```

The dashboard charts should immediately start updating with live data!

## Verify Everything Works

### Checklist

- [ ] `make build` completes without errors
- [ ] `make test` shows all tests passing
- [ ] `./bin/monitor` starts server on :8080
- [ ] Browser opens dashboard at http://localhost:8080
- [ ] Dashboard shows "Connected" status
- [ ] Test script sends metrics successfully
- [ ] Charts update in real-time

## Common Issues

### Issue: "Could not find web directory"

**Solution**: Run from the monitor directory or specify web path:
```bash
./bin/monitor -web /Users/borkiss../ag-botkit/monitor/web
```

### Issue: Port 8080 already in use

**Solution**: Use a different port:
```bash
./bin/monitor -addr localhost:9090
```

### Issue: WebSocket connection refused

**Check**:
1. Server is running: `ps aux | grep monitor`
2. Server logs show no errors
3. Browser console (F12) for JavaScript errors

## Next Steps

### Integrate with minibot

Connect your trading bot to send metrics:

```python
import websocket
import json
import time

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

### Customize Configuration

```bash
# Run on different port with custom capacity
./bin/monitor \
  -addr localhost:9090 \
  -capacity 5000 \
  -web ./web
```

### Run in Background

```bash
# Using nohup
nohup ./bin/monitor > monitor.log 2>&1 &

# Check it's running
tail -f monitor.log
```

## Performance Tips

1. **High-frequency metrics**: Default capacity (10,000 points) handles ~3 hours at 1 msg/sec
2. **Memory usage**: Each metric uses ~240 KB for 10K points
3. **Dashboard clients**: No limit, but each adds network overhead
4. **Slow clients**: Automatically disconnected if send buffer fills

## Development

### Make Targets

```bash
make build          # Build binary
make test           # Run tests
make clean          # Remove artifacts
make run            # Build and run
make fmt            # Format code
make lint           # Lint code
```

### File Locations

```
monitor/
├── bin/monitor              # Compiled binary
├── cmd/monitor/main.go      # Entry point
├── internal/
│   ├── server/              # HTTP/WebSocket handlers
│   └── storage/             # Metrics storage
├── web/                     # Dashboard UI
│   ├── index.html
│   └── static/
│       ├── app.js
│       └── style.css
├── Makefile
└── README.md                # Full documentation
```

## Support

For detailed documentation, see [README.md](README.md)

For architecture details, see [MULTI_AGENT_PLAN.md](../MULTI_AGENT_PLAN.md) section 3.3
