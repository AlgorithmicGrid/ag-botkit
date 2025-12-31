# RTDS Subscription Format Fix

**Date:** 2025-12-31
**Issue:** Minibot was not receiving messages from Polymarket RTDS

## Root Cause

The subscription format was incorrect. We were using:

```json
{
  "type": "subscribe",
  "channel": "market",
  "market": "0x..."
}
```

This format is **not** supported by Polymarket RTDS.

## Correct Format

According to [Polymarket RTDS Documentation](https://docs.polymarket.com/developers/RTDS/RTDS-overview), the correct subscription format is:

```json
{
  "action": "subscribe",
  "subscriptions": [
    {
      "topic": "topic_name",
      "type": "message_type",
      "filters": "optional_filter_string"
    }
  ]
}
```

## Supported Topics

According to the documentation, RTDS officially supports:

1. **crypto_prices** - Real-time cryptocurrency price updates ✅ Fully supported
2. **comments** - Comment-related events ✅ Fully supported
3. **activity** - Trade activity ⚠️ Unsupported (but may work)

**Note:** Market-specific order book data requires the separate [CLOB WebSocket](https://docs.polymarket.com/developers/CLOB/websocket/wss-overview) at `wss://ws-subscriptions-clob.polymarket.com`, not RTDS.

## Changes Made

### 1. Updated `examples/minibot/src/rtds.rs`

**New structures:**

```rust
#[derive(Debug, Serialize)]
pub struct Subscription {
    pub topic: String,
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SubscriptionMessage {
    pub action: String,
    pub subscriptions: Vec<Subscription>,
}
```

### 2. Updated `examples/minibot/src/main.rs`

Now subscribes to:

1. **crypto_prices** with type `"*"` (all crypto price updates)
2. **activity** with type `"trades"` (trade events, unsupported but worth trying)

**Subscription messages:**

```rust
// Crypto prices (fully supported)
{
  "action": "subscribe",
  "subscriptions": [
    {
      "topic": "crypto_prices",
      "type": "*"
    }
  ]
}

// Activity trades (unsupported but may work)
{
  "action": "subscribe",
  "subscriptions": [
    {
      "topic": "activity",
      "type": "trades"
    }
  ]
}
```

### 3. Updated `examples/minibot/config.yaml`

Removed the `subscribe_topics` field since subscriptions are now hardcoded.

## Expected Behavior

With these changes, minibot should:

1. ✅ Connect to `wss://ws-live-data.polymarket.com`
2. ✅ Send correct subscription messages
3. ✅ Receive crypto price updates
4. ⚠️ Possibly receive trade activity (unsupported topic)
5. ✅ Generate and send metrics to monitor

**Debug output should show:**

```
DEBUG: Connecting to Polymarket RTDS at wss://ws-live-data.polymarket.com
DEBUG: Successfully connected to RTDS
DEBUG: Sending subscription: {"action":"subscribe","subscriptions":[{"topic":"crypto_prices","type":"*"}]}
DEBUG: Sending subscription: {"action":"subscribe","subscriptions":[{"topic":"activity","type":"trades"}]}
DEBUG: Subscriptions sent, waiting for messages...
Received msg: {"topic":"crypto_prices","type":"price","timestamp":1735...,"payload":{...}}
Sending metric: "polymarket.rtds.messages_received" = 1 (type: Counter)
```

## Future: CLOB WebSocket for Order Books

If you need market-specific order book data, you'll need to:

1. Switch endpoint to `wss://ws-subscriptions-clob.polymarket.com`
2. Use CLOB subscription format:
   ```json
   {
     "assets_ids": ["asset_id_1", "asset_id_2"],
     "type": "market"
   }
   ```
3. Optionally add authentication for user-specific data

See: [CLOB WebSocket Documentation](https://docs.polymarket.com/developers/CLOB/websocket/wss-overview)

## Sources

- [Polymarket RTDS Overview](https://docs.polymarket.com/developers/RTDS/RTDS-overview)
- [Polymarket RTDS GitHub Client](https://github.com/Polymarket/real-time-data-client)
- [CLOB WebSocket Overview](https://docs.polymarket.com/developers/CLOB/websocket/wss-overview)
- [Market Channel Documentation](https://docs.polymarket.com/developers/CLOB/websocket/market-channel)

## Testing

Run minibot and check for messages:

```bash
cd /Users/borkiss../ag-botkit
./run_local.sh
```

Look for "Received msg" logs with crypto_prices data.
