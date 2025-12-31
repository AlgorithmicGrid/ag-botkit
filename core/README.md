# core/ - Time-Series Ring Buffer Library

High-performance C11 library for storing and querying time-series metrics with zero allocations in hot paths.

## Features

- **Zero-allocation hot paths**: No allocations in `append()` or query operations
- **Ring buffer design**: Fixed capacity with automatic oldest-data eviction
- **Opaque handle API**: ABI-stable interface hiding implementation details
- **Thread-safe by design**: Caller controls synchronization (no hidden mutexes)
- **Defensive programming**: NULL pointer checks, clear error codes
- **Portable**: C11 standard, works on macOS/Linux

## Quick Start

```c
#include "ag_timeseries.h"

// Create buffer with capacity for 1000 points
ag_timeseries_t* ts = ag_timeseries_create(1000);

// Append data points (timestamp in ms, value)
ag_timeseries_append(ts, 1735689600000, 42.5);
ag_timeseries_append(ts, 1735689601000, 43.2);
ag_timeseries_append(ts, 1735689602000, 44.1);

// Query last 10 points (newest first)
int64_t timestamps[10];
double values[10];
size_t count = ag_timeseries_query_last(ts, 10, timestamps, values);

// Query range [start, end] inclusive
count = ag_timeseries_query_range(
    ts,
    1735689600000,  // start_ms
    1735689602000,  // end_ms
    100,            // max_points
    timestamps,
    values
);

// Cleanup
ag_timeseries_destroy(ts);
```

## Building

### Build Static Library

```bash
make
# Output: lib/libag_core.a
```

### Run Unit Tests

```bash
make test
# Runs comprehensive test suite
```

### Run with Memory Leak Detection

```bash
make test-valgrind
# Requires valgrind installed
```

### Clean Build Artifacts

```bash
make clean
```

## API Reference

### Types

#### `ag_timeseries_t`
Opaque handle to time-series buffer. Internal structure is hidden for ABI stability.

### Error Codes

```c
#define AG_OK               0   // Success
#define AG_ERR_INVALID_ARG -1   // Invalid argument (NULL pointer, zero capacity, etc.)
#define AG_ERR_NOMEM       -2   // Memory allocation failed
#define AG_ERR_FULL        -3   // Buffer is full (unused in ring buffer)
#define AG_ERR_EMPTY       -4   // Buffer is empty
```

### Functions

#### `ag_timeseries_create`

```c
ag_timeseries_t* ag_timeseries_create(size_t capacity);
```

Create time-series buffer with fixed capacity.

- **Parameters:**
  - `capacity`: Maximum number of data points (must be > 0)
- **Returns:** Pointer to buffer, or NULL on failure
- **Memory:** Allocates memory for buffer. Call `ag_timeseries_destroy()` to free.
- **Thread Safety:** Safe to call concurrently

**Example:**
```c
ag_timeseries_t* ts = ag_timeseries_create(1000);
if (ts == NULL) {
    // Handle allocation failure
}
```

#### `ag_timeseries_destroy`

```c
void ag_timeseries_destroy(ag_timeseries_t* ts);
```

Destroy time-series buffer and free all memory.

- **Parameters:**
  - `ts`: Buffer handle (NULL safe - no-op if NULL)
- **Thread Safety:** NOT safe. Ensure no other threads are accessing buffer.

**Example:**
```c
ag_timeseries_destroy(ts);
```

#### `ag_timeseries_append`

```c
int ag_timeseries_append(ag_timeseries_t* ts, int64_t timestamp_ms, double value);
```

Append data point to buffer. Ring buffer overwrites oldest data when full.

- **Parameters:**
  - `ts`: Buffer handle
  - `timestamp_ms`: Timestamp in milliseconds (Unix epoch)
  - `value`: Metric value
- **Returns:** `AG_OK` on success, `AG_ERR_INVALID_ARG` if `ts` is NULL
- **Performance:** O(1), zero allocations
- **Thread Safety:** NOT safe. Caller must serialize access.

**Example:**
```c
int64_t now_ms = 1735689600000;
int result = ag_timeseries_append(ts, now_ms, 42.5);
if (result != AG_OK) {
    // Handle error
}
```

#### `ag_timeseries_query_last`

```c
size_t ag_timeseries_query_last(
    const ag_timeseries_t* ts,
    size_t max_points,
    int64_t* out_timestamps,
    double* out_values
);
```

Query last N points, ordered newest to oldest.

- **Parameters:**
  - `ts`: Buffer handle
  - `max_points`: Maximum number of points to retrieve
  - `out_timestamps`: Output array (must have space for `max_points`)
  - `out_values`: Output array (must have space for `max_points`)
- **Returns:** Number of points written (0 to `max_points`)
- **Performance:** O(n), zero allocations
- **Thread Safety:** NOT safe. Caller must serialize with append operations.

**Example:**
```c
int64_t timestamps[100];
double values[100];
size_t count = ag_timeseries_query_last(ts, 100, timestamps, values);

for (size_t i = 0; i < count; i++) {
    printf("%lld: %f\n", (long long)timestamps[i], values[i]);
}
```

#### `ag_timeseries_query_range`

```c
size_t ag_timeseries_query_range(
    const ag_timeseries_t* ts,
    int64_t start_ms,
    int64_t end_ms,
    size_t max_points,
    int64_t* out_timestamps,
    double* out_values
);
```

Query points in time range [start_ms, end_ms] inclusive, ordered oldest to newest.

- **Parameters:**
  - `ts`: Buffer handle
  - `start_ms`: Start timestamp (inclusive)
  - `end_ms`: End timestamp (inclusive)
  - `max_points`: Maximum number of points to retrieve
  - `out_timestamps`: Output array (must have space for `max_points`)
  - `out_values`: Output array (must have space for `max_points`)
- **Returns:** Number of points written (0 to `max_points`)
- **Performance:** O(n), zero allocations
- **Thread Safety:** NOT safe. Caller must serialize with append operations.

**Example:**
```c
int64_t start = 1735689600000;
int64_t end = 1735689700000;
int64_t timestamps[1000];
double values[1000];

size_t count = ag_timeseries_query_range(ts, start, end, 1000, timestamps, values);
printf("Found %zu points in range\n", count);
```

#### `ag_timeseries_size`

```c
size_t ag_timeseries_size(const ag_timeseries_t* ts);
```

Get current number of data points in buffer.

- **Parameters:**
  - `ts`: Buffer handle
- **Returns:** Number of points (0 to capacity), or 0 if `ts` is NULL
- **Thread Safety:** Safe to call, but result may be stale if other threads modify buffer.

**Example:**
```c
size_t size = ag_timeseries_size(ts);
printf("Buffer contains %zu points\n", size);
```

#### `ag_timeseries_capacity`

```c
size_t ag_timeseries_capacity(const ag_timeseries_t* ts);
```

Get buffer capacity.

- **Parameters:**
  - `ts`: Buffer handle
- **Returns:** Maximum capacity, or 0 if `ts` is NULL
- **Thread Safety:** Safe to call (capacity is immutable after creation).

**Example:**
```c
size_t capacity = ag_timeseries_capacity(ts);
printf("Buffer capacity: %zu\n", capacity);
```

## Usage Examples

### Example 1: Basic Metrics Storage

```c
#include "ag_timeseries.h"
#include <stdio.h>
#include <time.h>

int main(void) {
    // Create buffer for last 60 seconds of data (1 point per second)
    ag_timeseries_t* latency_ts = ag_timeseries_create(60);

    // Simulate recording latency metrics
    for (int i = 0; i < 100; i++) {
        int64_t timestamp = 1735689600000 + (i * 1000);  // Every 1 second
        double latency_ms = 10.0 + (i % 20);             // Mock latency

        ag_timeseries_append(latency_ts, timestamp, latency_ms);
    }

    // Get last 10 data points
    int64_t timestamps[10];
    double values[10];
    size_t count = ag_timeseries_query_last(latency_ts, 10, timestamps, values);

    printf("Last 10 latency measurements:\n");
    for (size_t i = 0; i < count; i++) {
        printf("  %lld ms: %.2f ms\n", (long long)timestamps[i], values[i]);
    }

    ag_timeseries_destroy(latency_ts);
    return 0;
}
```

### Example 2: Range Query for Monitoring

```c
#include "ag_timeseries.h"
#include <stdio.h>

int main(void) {
    ag_timeseries_t* msgs_per_sec = ag_timeseries_create(3600);  // 1 hour at 1s resolution

    // Simulate message rate data
    int64_t base_time = 1735689600000;
    for (int i = 0; i < 120; i++) {  // 2 minutes of data
        ag_timeseries_append(msgs_per_sec, base_time + (i * 1000), 50.0 + (i % 30));
    }

    // Query last 60 seconds
    int64_t start = base_time + 60000;  // 1 minute ago
    int64_t end = base_time + 120000;   // now

    int64_t timestamps[100];
    double values[100];
    size_t count = ag_timeseries_query_range(msgs_per_sec, start, end, 100, timestamps, values);

    // Calculate average
    double sum = 0.0;
    for (size_t i = 0; i < count; i++) {
        sum += values[i];
    }
    double avg = (count > 0) ? sum / count : 0.0;

    printf("Last 60 seconds: %zu data points, avg %.2f msgs/sec\n", count, avg);

    ag_timeseries_destroy(msgs_per_sec);
    return 0;
}
```

### Example 3: Thread-Safe Wrapper

```c
#include "ag_timeseries.h"
#include <pthread.h>
#include <stdio.h>

typedef struct {
    ag_timeseries_t* ts;
    pthread_mutex_t lock;
} thread_safe_ts_t;

thread_safe_ts_t* ts_create_safe(size_t capacity) {
    thread_safe_ts_t* safe_ts = malloc(sizeof(thread_safe_ts_t));
    if (safe_ts == NULL) return NULL;

    safe_ts->ts = ag_timeseries_create(capacity);
    if (safe_ts->ts == NULL) {
        free(safe_ts);
        return NULL;
    }

    pthread_mutex_init(&safe_ts->lock, NULL);
    return safe_ts;
}

void ts_destroy_safe(thread_safe_ts_t* safe_ts) {
    if (safe_ts == NULL) return;
    ag_timeseries_destroy(safe_ts->ts);
    pthread_mutex_destroy(&safe_ts->lock);
    free(safe_ts);
}

int ts_append_safe(thread_safe_ts_t* safe_ts, int64_t timestamp_ms, double value) {
    pthread_mutex_lock(&safe_ts->lock);
    int result = ag_timeseries_append(safe_ts->ts, timestamp_ms, value);
    pthread_mutex_unlock(&safe_ts->lock);
    return result;
}

size_t ts_query_last_safe(thread_safe_ts_t* safe_ts, size_t max_points,
                          int64_t* out_timestamps, double* out_values) {
    pthread_mutex_lock(&safe_ts->lock);
    size_t count = ag_timeseries_query_last(safe_ts->ts, max_points,
                                            out_timestamps, out_values);
    pthread_mutex_unlock(&safe_ts->lock);
    return count;
}
```

## Integration with Other Languages

### Rust FFI Example

```rust
// In risk/src/metrics.rs or examples/minibot/src/metrics.rs

use std::os::raw::{c_int, c_double};
use std::ffi::c_void;

#[repr(C)]
struct AgTimeseries {
    _private: [u8; 0],
}

extern "C" {
    fn ag_timeseries_create(capacity: usize) -> *mut AgTimeseries;
    fn ag_timeseries_destroy(ts: *mut AgTimeseries);
    fn ag_timeseries_append(ts: *mut AgTimeseries, timestamp_ms: i64, value: f64) -> c_int;
    fn ag_timeseries_query_last(
        ts: *const AgTimeseries,
        max_points: usize,
        out_timestamps: *mut i64,
        out_values: *mut f64,
    ) -> usize;
}

pub struct TimeSeries {
    inner: *mut AgTimeseries,
}

impl TimeSeries {
    pub fn new(capacity: usize) -> Option<Self> {
        let ptr = unsafe { ag_timeseries_create(capacity) };
        if ptr.is_null() {
            None
        } else {
            Some(TimeSeries { inner: ptr })
        }
    }

    pub fn append(&mut self, timestamp_ms: i64, value: f64) -> Result<(), &'static str> {
        let result = unsafe { ag_timeseries_append(self.inner, timestamp_ms, value) };
        if result == 0 {
            Ok(())
        } else {
            Err("Append failed")
        }
    }
}

impl Drop for TimeSeries {
    fn drop(&mut self) {
        unsafe { ag_timeseries_destroy(self.inner) };
    }
}
```

## Performance Characteristics

| Operation | Time Complexity | Memory Allocations |
|-----------|----------------|-------------------|
| `create()` | O(n) | 1 (buffer allocation) |
| `append()` | O(1) | 0 |
| `query_last()` | O(n) | 0 |
| `query_range()` | O(n) | 0 |
| `size()` | O(1) | 0 |
| `capacity()` | O(1) | 0 |

Where `n` is the number of points requested, NOT the buffer capacity.

## Memory Management

### Ownership Semantics

- `ag_timeseries_create()` allocates memory; caller owns the returned handle
- Caller must call `ag_timeseries_destroy()` to free memory
- Query functions write to caller-provided buffers (no internal allocation)
- No reference counting or shared ownership

### Memory Footprint

For a buffer with capacity `N`:
- Handle: ~48 bytes (platform-dependent)
- Data: `N * sizeof(int64_t) + N * sizeof(double)` = `N * 16 bytes`
- Total: ~48 + 16N bytes

Example:
- Capacity 1000: ~16 KB
- Capacity 10000: ~160 KB
- Capacity 100000: ~1.6 MB

## Thread Safety

The library is **NOT thread-safe by design**. This design choice:

1. Eliminates mutex overhead in hot paths
2. Gives caller full control over synchronization strategy
3. Allows lock-free usage in single-threaded contexts

**Caller Responsibilities:**
- Serialize access using external locks (see thread-safe wrapper example)
- OR ensure single-threaded access
- OR use separate buffers per thread

## ABI Stability Guarantees

- Opaque handle design allows internal structure changes without API breaks
- Error codes are stable (won't change values)
- Function signatures are stable (won't change parameters)
- Safe to upgrade library without recompiling dependent code

**Future-Compatible Changes:**
- Adding new fields to internal struct (opaque to users)
- Optimizing ring buffer implementation
- Adding instrumentation/metrics

**Breaking Changes (would require major version bump):**
- Changing function signatures
- Changing error code values
- Changing timestamp/value data types

## Testing

The test suite (`tests/test_timeseries.c`) covers:

- Creation and destruction
- Append operations (single, multiple, wraparound)
- Query operations (last N, range, edge cases)
- NULL pointer handling
- Boundary conditions (empty, full, capacity 1)
- Ring buffer wraparound correctness
- Large capacity stress tests
- Negative timestamps
- Edge cases (zero max_points, invalid ranges)

Run tests with:
```bash
make test
```

For memory leak detection:
```bash
make test-valgrind
```

## License

Copyright (c) 2025 AlgorithmicGrid

## Related Documentation

- [MULTI_AGENT_PLAN.md](../MULTI_AGENT_PLAN.md) - System architecture
- [CLAUDE.md](../CLAUDE.md) - Development guidelines
- [risk/](../risk/) - Rust risk engine (uses this library via FFI)
- [examples/minibot/](../examples/minibot/) - Demo bot (uses this library)
