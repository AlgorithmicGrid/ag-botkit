/*
 * ag_timeseries.c - Time-Series Ring Buffer Implementation
 *
 * Implementation Strategy:
 *   - Ring buffer with head/tail pointers
 *   - Fixed capacity, circular overwrite
 *   - Zero allocations after create()
 *   - Defensive programming with NULL checks
 *
 * Copyright (c) 2025 AlgorithmicGrid
 */

#include "ag_timeseries.h"
#include <stdlib.h>
#include <string.h>
#include <assert.h>

/*
 * Internal structure - opaque to users
 *
 * Ring Buffer Layout:
 *   [0] [1] [2] ... [capacity-1]
 *    ^               ^
 *    tail           head (next write position)
 *
 * Invariants:
 *   - size <= capacity
 *   - head, tail < capacity
 *   - When size == capacity, buffer is full
 *   - When size == 0, buffer is empty
 */
struct ag_timeseries_t {
    size_t capacity;        /* Maximum number of points */
    size_t size;            /* Current number of points */
    size_t head;            /* Next write position */
    size_t tail;            /* Oldest data position (only valid when full) */
    int64_t* timestamps;    /* Timestamp array */
    double* values;         /* Value array */
};

/* Helper: Advance index with wraparound */
static inline size_t advance_index(size_t index, size_t capacity) {
    return (index + 1) % capacity;
}

ag_timeseries_t* ag_timeseries_create(size_t capacity) {
    /* Validate capacity */
    if (capacity == 0) {
        return NULL;
    }

    /* Allocate handle */
    ag_timeseries_t* ts = (ag_timeseries_t*)malloc(sizeof(ag_timeseries_t));
    if (ts == NULL) {
        return NULL;
    }

    /* Allocate data arrays */
    ts->timestamps = (int64_t*)malloc(capacity * sizeof(int64_t));
    ts->values = (double*)malloc(capacity * sizeof(double));

    if (ts->timestamps == NULL || ts->values == NULL) {
        /* Cleanup on partial allocation failure */
        free(ts->timestamps);
        free(ts->values);
        free(ts);
        return NULL;
    }

    /* Initialize state */
    ts->capacity = capacity;
    ts->size = 0;
    ts->head = 0;
    ts->tail = 0;

    /* Zero-initialize arrays (defensive, not strictly necessary) */
    memset(ts->timestamps, 0, capacity * sizeof(int64_t));
    memset(ts->values, 0, capacity * sizeof(double));

    return ts;
}

void ag_timeseries_destroy(ag_timeseries_t* ts) {
    if (ts == NULL) {
        return;
    }

    /* Free data arrays */
    free(ts->timestamps);
    free(ts->values);

    /* Free handle */
    free(ts);
}

int ag_timeseries_append(ag_timeseries_t* ts, int64_t timestamp_ms, double value) {
    /* Validate handle */
    if (ts == NULL) {
        return AG_ERR_INVALID_ARG;
    }

    /* Write to current head position */
    ts->timestamps[ts->head] = timestamp_ms;
    ts->values[ts->head] = value;

    /* Advance head */
    ts->head = advance_index(ts->head, ts->capacity);

    /* Update size and tail */
    if (ts->size < ts->capacity) {
        /* Buffer not yet full */
        ts->size++;
    } else {
        /* Buffer full - advance tail (overwrite oldest) */
        ts->tail = advance_index(ts->tail, ts->capacity);
    }

    return AG_OK;
}

size_t ag_timeseries_query_last(
    const ag_timeseries_t* ts,
    size_t max_points,
    int64_t* out_timestamps,
    double* out_values
) {
    /* Validate inputs */
    if (ts == NULL || out_timestamps == NULL || out_values == NULL) {
        return 0;
    }

    if (ts->size == 0 || max_points == 0) {
        return 0;
    }

    /* Determine how many points to return */
    size_t num_points = (max_points < ts->size) ? max_points : ts->size;

    /*
     * Iterate backwards from newest to oldest
     * Newest is at (head - 1 + capacity) % capacity
     */
    size_t current_idx;
    if (ts->head == 0) {
        current_idx = ts->capacity - 1;
    } else {
        current_idx = ts->head - 1;
    }

    for (size_t i = 0; i < num_points; i++) {
        out_timestamps[i] = ts->timestamps[current_idx];
        out_values[i] = ts->values[current_idx];

        /* Move to previous element */
        if (current_idx == 0) {
            current_idx = ts->capacity - 1;
        } else {
            current_idx--;
        }
    }

    return num_points;
}

size_t ag_timeseries_query_range(
    const ag_timeseries_t* ts,
    int64_t start_ms,
    int64_t end_ms,
    size_t max_points,
    int64_t* out_timestamps,
    double* out_values
) {
    /* Validate inputs */
    if (ts == NULL || out_timestamps == NULL || out_values == NULL) {
        return 0;
    }

    if (ts->size == 0 || max_points == 0) {
        return 0;
    }

    /* Validate range */
    if (start_ms > end_ms) {
        return 0;
    }

    /*
     * Iterate from oldest to newest, collecting matching points.
     * Oldest is at tail (when buffer is full) or index 0 (when not full).
     */
    size_t start_idx = (ts->size < ts->capacity) ? 0 : ts->tail;
    size_t count = 0;

    for (size_t i = 0; i < ts->size && count < max_points; i++) {
        size_t idx = (start_idx + i) % ts->capacity;
        int64_t timestamp = ts->timestamps[idx];

        /* Check if timestamp is in range */
        if (timestamp >= start_ms && timestamp <= end_ms) {
            out_timestamps[count] = timestamp;
            out_values[count] = ts->values[idx];
            count++;
        }
    }

    return count;
}

size_t ag_timeseries_size(const ag_timeseries_t* ts) {
    if (ts == NULL) {
        return 0;
    }
    return ts->size;
}

size_t ag_timeseries_capacity(const ag_timeseries_t* ts) {
    if (ts == NULL) {
        return 0;
    }
    return ts->capacity;
}
