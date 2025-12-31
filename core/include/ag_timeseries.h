/*
 * ag_timeseries.h - Time-Series Ring Buffer API
 *
 * Purpose: Lock-free, zero-allocation ring buffer for storing time-series metrics.
 *
 * Thread Safety: NOT thread-safe. Caller must provide external synchronization.
 * Memory Model: Fixed capacity allocated at creation time, no allocations in hot paths.
 * ABI Stability: Opaque handle design allows internal changes without breaking API.
 *
 * Copyright (c) 2025 AlgorithmicGrid
 */

#ifndef AG_TIMESERIES_H
#define AG_TIMESERIES_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque handle - internal structure hidden from users */
typedef struct ag_timeseries_t ag_timeseries_t;

/* Error codes - consistent with MULTI_AGENT_PLAN.md Section 3.1 */
#define AG_OK               0   /* Success */
#define AG_ERR_INVALID_ARG -1   /* Invalid argument (NULL pointer, invalid capacity, etc.) */
#define AG_ERR_NOMEM       -2   /* Memory allocation failed */
#define AG_ERR_FULL        -3   /* Buffer is full (not used in ring buffer, kept for compatibility) */
#define AG_ERR_EMPTY       -4   /* Buffer is empty */

/*
 * Create time-series buffer with fixed capacity.
 *
 * Parameters:
 *   capacity - Maximum number of data points to store (must be > 0)
 *
 * Returns:
 *   Pointer to allocated time-series buffer, or NULL on failure.
 *
 * Memory:
 *   Allocates memory for handle and ring buffer. Call ag_timeseries_destroy() to free.
 *   This is the ONLY allocation point - no allocations in append/query operations.
 *
 * Thread Safety:
 *   Safe to call concurrently from multiple threads.
 */
ag_timeseries_t* ag_timeseries_create(size_t capacity);

/*
 * Destroy time-series buffer.
 *
 * Parameters:
 *   ts - Time-series buffer handle (NULL safe - no-op if NULL)
 *
 * Memory:
 *   Frees all memory associated with the buffer.
 *
 * Thread Safety:
 *   NOT safe. Caller must ensure no other threads are accessing this buffer.
 */
void ag_timeseries_destroy(ag_timeseries_t* ts);

/*
 * Append data point to time-series buffer.
 *
 * Parameters:
 *   ts           - Time-series buffer handle
 *   timestamp_ms - Timestamp in milliseconds (Unix epoch)
 *   value        - Metric value
 *
 * Returns:
 *   AG_OK on success
 *   AG_ERR_INVALID_ARG if ts is NULL
 *
 * Behavior:
 *   Ring buffer implementation - oldest data is overwritten when full.
 *   No validation of timestamp ordering (caller's responsibility).
 *   NO ALLOCATIONS - constant time O(1) operation.
 *
 * Thread Safety:
 *   NOT safe. Caller must serialize access.
 */
int ag_timeseries_append(ag_timeseries_t* ts, int64_t timestamp_ms, double value);

/*
 * Query last N points (newest first).
 *
 * Parameters:
 *   ts              - Time-series buffer handle
 *   max_points      - Maximum number of points to retrieve
 *   out_timestamps  - Output array for timestamps (must have space for max_points)
 *   out_values      - Output array for values (must have space for max_points)
 *
 * Returns:
 *   Number of points written to output arrays (0 to max_points).
 *   Returns 0 if ts is NULL or buffer is empty.
 *
 * Behavior:
 *   Returns up to max_points, ordered newest to oldest.
 *   If buffer has fewer than max_points, returns all available.
 *   NO ALLOCATIONS - output written to caller-provided buffers.
 *
 * Thread Safety:
 *   NOT safe. Caller must serialize access with append operations.
 */
size_t ag_timeseries_query_last(
    const ag_timeseries_t* ts,
    size_t max_points,
    int64_t* out_timestamps,
    double* out_values
);

/*
 * Query points in time range [start_ms, end_ms] inclusive.
 *
 * Parameters:
 *   ts              - Time-series buffer handle
 *   start_ms        - Start timestamp (inclusive)
 *   end_ms          - End timestamp (inclusive)
 *   max_points      - Maximum number of points to retrieve
 *   out_timestamps  - Output array for timestamps (must have space for max_points)
 *   out_values      - Output array for values (must have space for max_points)
 *
 * Returns:
 *   Number of points written to output arrays (0 to max_points).
 *   Returns 0 if ts is NULL, buffer is empty, or no points in range.
 *
 * Behavior:
 *   Returns points where start_ms <= timestamp <= end_ms, up to max_points.
 *   Results ordered oldest to newest (chronological).
 *   If more than max_points match, returns first max_points chronologically.
 *   NO ALLOCATIONS - output written to caller-provided buffers.
 *
 * Thread Safety:
 *   NOT safe. Caller must serialize access with append operations.
 */
size_t ag_timeseries_query_range(
    const ag_timeseries_t* ts,
    int64_t start_ms,
    int64_t end_ms,
    size_t max_points,
    int64_t* out_timestamps,
    double* out_values
);

/*
 * Get current number of data points in buffer.
 *
 * Parameters:
 *   ts - Time-series buffer handle
 *
 * Returns:
 *   Number of points currently stored (0 to capacity).
 *   Returns 0 if ts is NULL.
 *
 * Thread Safety:
 *   Safe to call, but result may be stale if other threads are modifying buffer.
 */
size_t ag_timeseries_size(const ag_timeseries_t* ts);

/*
 * Get buffer capacity.
 *
 * Parameters:
 *   ts - Time-series buffer handle
 *
 * Returns:
 *   Maximum capacity of buffer.
 *   Returns 0 if ts is NULL.
 *
 * Thread Safety:
 *   Safe to call (capacity is immutable after creation).
 */
size_t ag_timeseries_capacity(const ag_timeseries_t* ts);

#ifdef __cplusplus
}
#endif

#endif /* AG_TIMESERIES_H */
