#!/usr/bin/env python3
"""
Test script to send sample metrics to the monitor dashboard.
Usage: python3 test_metrics.py
"""

import json
import time
import random
from websocket import create_connection

def send_test_metrics():
    """Send various test metrics to the monitor."""
    print("Connecting to ws://localhost:8080/metrics...")

    try:
        ws = create_connection("ws://localhost:8080/metrics")
        print("Connected! Sending test metrics...\n")

        # Send metrics for 30 seconds
        start_time = time.time()
        msg_count = 0

        while time.time() - start_time < 30:
            timestamp = int(time.time() * 1000)

            # RTDS lag metric
            lag_metric = {
                "timestamp": timestamp,
                "metric_type": "gauge",
                "metric_name": "polymarket.rtds.lag_ms",
                "value": random.uniform(10, 100),
                "labels": {"topic": "market"}
            }
            ws.send(json.dumps(lag_metric))
            msg_count += 1

            # Messages per second metric
            mps_metric = {
                "timestamp": timestamp,
                "metric_type": "gauge",
                "metric_name": "polymarket.rtds.msgs_per_second",
                "value": random.uniform(10, 50),
                "labels": {}
            }
            ws.send(json.dumps(mps_metric))
            msg_count += 1

            # Messages received counter
            msg_received = {
                "timestamp": timestamp,
                "metric_type": "counter",
                "metric_name": "polymarket.rtds.messages_received",
                "value": 1,
                "labels": {"topic": "market", "message_type": "book"}
            }
            ws.send(json.dumps(msg_received))
            msg_count += 1

            # Position size metric
            position_metric = {
                "timestamp": timestamp,
                "metric_type": "gauge",
                "metric_name": "polymarket.position.size",
                "value": random.uniform(100, 500),
                "labels": {"market_id": "0x123abc", "side": "long"}
            }
            ws.send(json.dumps(position_metric))
            msg_count += 1

            # Risk decision (random allow/block)
            risk_metric = {
                "timestamp": timestamp,
                "metric_type": "gauge",
                "metric_name": "polymarket.risk.decision",
                "value": random.choice([0, 1, 1, 1]),  # 75% allowed
                "labels": {"policy": "position_limit", "market_id": "0x123abc"}
            }
            ws.send(json.dumps(risk_metric))
            msg_count += 1

            # Occasionally toggle kill switch
            if random.random() < 0.05:  # 5% chance
                kill_switch_metric = {
                    "timestamp": timestamp,
                    "metric_type": "gauge",
                    "metric_name": "polymarket.risk.kill_switch",
                    "value": random.choice([0, 1]),
                    "labels": {}
                }
                ws.send(json.dumps(kill_switch_metric))
                msg_count += 1

            print(f"Sent {msg_count} metrics... (elapsed: {int(time.time() - start_time)}s)", end='\r')
            time.sleep(0.2)  # Send metrics 5 times per second

        print(f"\n\nTest complete! Sent {msg_count} metrics in {int(time.time() - start_time)} seconds.")
        print("Check http://localhost:8080 to view the dashboard.")

        ws.close()

    except ConnectionRefusedError:
        print("ERROR: Could not connect to monitor server.")
        print("Make sure the monitor is running: ./bin/monitor")
    except Exception as e:
        print(f"ERROR: {e}")

if __name__ == "__main__":
    print("=" * 60)
    print("Monitor Dashboard Test Script")
    print("=" * 60)
    print()
    print("This script will send test metrics to the monitor for 30s.")
    print("Open http://localhost:8080 in your browser to view the charts.")
    print()

    send_test_metrics()
