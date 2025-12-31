/*
 * test_timeseries.c - Comprehensive Unit Tests
 *
 * Test Coverage:
 *   - Creation and destruction
 *   - Append operations (normal and boundary cases)
 *   - Ring buffer wraparound
 *   - Query last N points
 *   - Query range
 *   - Size and capacity queries
 *   - NULL pointer handling
 *   - Edge cases (empty buffer, full buffer, etc.)
 *
 * Copyright (c) 2025 AlgorithmicGrid
 */

#include "ag_timeseries.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include <math.h>

/* Test framework macros */
#define TEST(name) static void test_##name(void)
#define RUN_TEST(name) do { \
    printf("Running %s...", #name); \
    test_##name(); \
    printf(" PASSED\n"); \
} while(0)

#define ASSERT(cond) do { \
    if (!(cond)) { \
        fprintf(stderr, "\nAssertion failed: %s\n  File: %s\n  Line: %d\n", \
                #cond, __FILE__, __LINE__); \
        exit(1); \
    } \
} while(0)

#define ASSERT_EQ(a, b) ASSERT((a) == (b))
#define ASSERT_NE(a, b) ASSERT((a) != (b))
#define ASSERT_DOUBLE_EQ(a, b) ASSERT(fabs((a) - (b)) < 1e-9)

/* Test: Create and destroy */
TEST(create_destroy) {
    ag_timeseries_t* ts = ag_timeseries_create(100);
    ASSERT_NE(ts, NULL);
    ASSERT_EQ(ag_timeseries_capacity(ts), 100);
    ASSERT_EQ(ag_timeseries_size(ts), 0);
    ag_timeseries_destroy(ts);
}

/* Test: Create with zero capacity should fail */
TEST(create_zero_capacity) {
    ag_timeseries_t* ts = ag_timeseries_create(0);
    ASSERT_EQ(ts, NULL);
}

/* Test: Destroy NULL is safe */
TEST(destroy_null) {
    ag_timeseries_destroy(NULL);  /* Should not crash */
}

/* Test: Append single point */
TEST(append_single) {
    ag_timeseries_t* ts = ag_timeseries_create(10);
    ASSERT_NE(ts, NULL);

    int result = ag_timeseries_append(ts, 1000, 42.5);
    ASSERT_EQ(result, AG_OK);
    ASSERT_EQ(ag_timeseries_size(ts), 1);

    ag_timeseries_destroy(ts);
}

/* Test: Append to NULL should fail */
TEST(append_null) {
    int result = ag_timeseries_append(NULL, 1000, 42.5);
    ASSERT_EQ(result, AG_ERR_INVALID_ARG);
}

/* Test: Append multiple points */
TEST(append_multiple) {
    ag_timeseries_t* ts = ag_timeseries_create(5);
    ASSERT_NE(ts, NULL);

    for (int i = 0; i < 3; i++) {
        int result = ag_timeseries_append(ts, 1000 + i, 10.0 + i);
        ASSERT_EQ(result, AG_OK);
    }

    ASSERT_EQ(ag_timeseries_size(ts), 3);
    ASSERT_EQ(ag_timeseries_capacity(ts), 5);

    ag_timeseries_destroy(ts);
}

/* Test: Ring buffer wraparound */
TEST(ring_buffer_wraparound) {
    ag_timeseries_t* ts = ag_timeseries_create(3);
    ASSERT_NE(ts, NULL);

    /* Fill buffer */
    ag_timeseries_append(ts, 1000, 1.0);
    ag_timeseries_append(ts, 2000, 2.0);
    ag_timeseries_append(ts, 3000, 3.0);
    ASSERT_EQ(ag_timeseries_size(ts), 3);

    /* Overwrite oldest */
    ag_timeseries_append(ts, 4000, 4.0);
    ASSERT_EQ(ag_timeseries_size(ts), 3);  /* Size stays at capacity */

    ag_timeseries_append(ts, 5000, 5.0);
    ASSERT_EQ(ag_timeseries_size(ts), 3);

    /* Query last 3 - should get newest (5, 4, 3) */
    int64_t timestamps[3];
    double values[3];
    size_t count = ag_timeseries_query_last(ts, 3, timestamps, values);

    ASSERT_EQ(count, 3);
    ASSERT_EQ(timestamps[0], 5000);  /* Newest */
    ASSERT_DOUBLE_EQ(values[0], 5.0);
    ASSERT_EQ(timestamps[1], 4000);
    ASSERT_DOUBLE_EQ(values[1], 4.0);
    ASSERT_EQ(timestamps[2], 3000);  /* Oldest remaining */
    ASSERT_DOUBLE_EQ(values[2], 3.0);

    ag_timeseries_destroy(ts);
}

/* Test: Query last from empty buffer */
TEST(query_last_empty) {
    ag_timeseries_t* ts = ag_timeseries_create(10);
    ASSERT_NE(ts, NULL);

    int64_t timestamps[5];
    double values[5];
    size_t count = ag_timeseries_query_last(ts, 5, timestamps, values);

    ASSERT_EQ(count, 0);

    ag_timeseries_destroy(ts);
}

/* Test: Query last with NULL handle */
TEST(query_last_null) {
    int64_t timestamps[5];
    double values[5];
    size_t count = ag_timeseries_query_last(NULL, 5, timestamps, values);
    ASSERT_EQ(count, 0);
}

/* Test: Query last with NULL output arrays */
TEST(query_last_null_output) {
    ag_timeseries_t* ts = ag_timeseries_create(10);
    ASSERT_NE(ts, NULL);

    ag_timeseries_append(ts, 1000, 1.0);

    size_t count = ag_timeseries_query_last(ts, 5, NULL, NULL);
    ASSERT_EQ(count, 0);

    ag_timeseries_destroy(ts);
}

/* Test: Query last fewer points than requested */
TEST(query_last_fewer_than_requested) {
    ag_timeseries_t* ts = ag_timeseries_create(10);
    ASSERT_NE(ts, NULL);

    ag_timeseries_append(ts, 1000, 1.0);
    ag_timeseries_append(ts, 2000, 2.0);

    int64_t timestamps[5];
    double values[5];
    size_t count = ag_timeseries_query_last(ts, 5, timestamps, values);

    ASSERT_EQ(count, 2);
    ASSERT_EQ(timestamps[0], 2000);  /* Newest first */
    ASSERT_DOUBLE_EQ(values[0], 2.0);
    ASSERT_EQ(timestamps[1], 1000);
    ASSERT_DOUBLE_EQ(values[1], 1.0);

    ag_timeseries_destroy(ts);
}

/* Test: Query last exactly capacity */
TEST(query_last_exact_capacity) {
    ag_timeseries_t* ts = ag_timeseries_create(3);
    ASSERT_NE(ts, NULL);

    ag_timeseries_append(ts, 1000, 1.0);
    ag_timeseries_append(ts, 2000, 2.0);
    ag_timeseries_append(ts, 3000, 3.0);

    int64_t timestamps[3];
    double values[3];
    size_t count = ag_timeseries_query_last(ts, 3, timestamps, values);

    ASSERT_EQ(count, 3);
    ASSERT_EQ(timestamps[0], 3000);
    ASSERT_EQ(timestamps[1], 2000);
    ASSERT_EQ(timestamps[2], 1000);

    ag_timeseries_destroy(ts);
}

/* Test: Query range basic */
TEST(query_range_basic) {
    ag_timeseries_t* ts = ag_timeseries_create(10);
    ASSERT_NE(ts, NULL);

    ag_timeseries_append(ts, 1000, 1.0);
    ag_timeseries_append(ts, 2000, 2.0);
    ag_timeseries_append(ts, 3000, 3.0);
    ag_timeseries_append(ts, 4000, 4.0);
    ag_timeseries_append(ts, 5000, 5.0);

    int64_t timestamps[10];
    double values[10];
    size_t count = ag_timeseries_query_range(ts, 2000, 4000, 10, timestamps, values);

    ASSERT_EQ(count, 3);
    ASSERT_EQ(timestamps[0], 2000);  /* Oldest first in range */
    ASSERT_DOUBLE_EQ(values[0], 2.0);
    ASSERT_EQ(timestamps[1], 3000);
    ASSERT_DOUBLE_EQ(values[1], 3.0);
    ASSERT_EQ(timestamps[2], 4000);
    ASSERT_DOUBLE_EQ(values[2], 4.0);

    ag_timeseries_destroy(ts);
}

/* Test: Query range empty buffer */
TEST(query_range_empty) {
    ag_timeseries_t* ts = ag_timeseries_create(10);
    ASSERT_NE(ts, NULL);

    int64_t timestamps[10];
    double values[10];
    size_t count = ag_timeseries_query_range(ts, 1000, 5000, 10, timestamps, values);

    ASSERT_EQ(count, 0);

    ag_timeseries_destroy(ts);
}

/* Test: Query range with NULL handle */
TEST(query_range_null) {
    int64_t timestamps[10];
    double values[10];
    size_t count = ag_timeseries_query_range(NULL, 1000, 5000, 10, timestamps, values);
    ASSERT_EQ(count, 0);
}

/* Test: Query range invalid (start > end) */
TEST(query_range_invalid) {
    ag_timeseries_t* ts = ag_timeseries_create(10);
    ASSERT_NE(ts, NULL);

    ag_timeseries_append(ts, 1000, 1.0);

    int64_t timestamps[10];
    double values[10];
    size_t count = ag_timeseries_query_range(ts, 5000, 1000, 10, timestamps, values);

    ASSERT_EQ(count, 0);

    ag_timeseries_destroy(ts);
}

/* Test: Query range no matches */
TEST(query_range_no_matches) {
    ag_timeseries_t* ts = ag_timeseries_create(10);
    ASSERT_NE(ts, NULL);

    ag_timeseries_append(ts, 1000, 1.0);
    ag_timeseries_append(ts, 2000, 2.0);

    int64_t timestamps[10];
    double values[10];
    size_t count = ag_timeseries_query_range(ts, 5000, 6000, 10, timestamps, values);

    ASSERT_EQ(count, 0);

    ag_timeseries_destroy(ts);
}

/* Test: Query range with wraparound */
TEST(query_range_wraparound) {
    ag_timeseries_t* ts = ag_timeseries_create(3);
    ASSERT_NE(ts, NULL);

    /* Fill and wrap */
    ag_timeseries_append(ts, 1000, 1.0);
    ag_timeseries_append(ts, 2000, 2.0);
    ag_timeseries_append(ts, 3000, 3.0);
    ag_timeseries_append(ts, 4000, 4.0);  /* Overwrites 1000 */
    ag_timeseries_append(ts, 5000, 5.0);  /* Overwrites 2000 */

    /* Buffer now has: 3000, 4000, 5000 */
    int64_t timestamps[10];
    double values[10];
    size_t count = ag_timeseries_query_range(ts, 3500, 5000, 10, timestamps, values);

    ASSERT_EQ(count, 2);
    ASSERT_EQ(timestamps[0], 4000);
    ASSERT_DOUBLE_EQ(values[0], 4.0);
    ASSERT_EQ(timestamps[1], 5000);
    ASSERT_DOUBLE_EQ(values[1], 5.0);

    ag_timeseries_destroy(ts);
}

/* Test: Query range with max_points limit */
TEST(query_range_max_points_limit) {
    ag_timeseries_t* ts = ag_timeseries_create(10);
    ASSERT_NE(ts, NULL);

    for (int i = 0; i < 10; i++) {
        ag_timeseries_append(ts, 1000 + i * 100, i * 1.0);
    }

    int64_t timestamps[3];
    double values[3];
    size_t count = ag_timeseries_query_range(ts, 1000, 2000, 3, timestamps, values);

    /* Should match 11 points (1000-2000 inclusive), but limit to 3 */
    ASSERT_EQ(count, 3);
    ASSERT_EQ(timestamps[0], 1000);
    ASSERT_EQ(timestamps[1], 1100);
    ASSERT_EQ(timestamps[2], 1200);

    ag_timeseries_destroy(ts);
}

/* Test: Size and capacity queries with NULL */
TEST(size_capacity_null) {
    ASSERT_EQ(ag_timeseries_size(NULL), 0);
    ASSERT_EQ(ag_timeseries_capacity(NULL), 0);
}

/* Test: Large capacity */
TEST(large_capacity) {
    ag_timeseries_t* ts = ag_timeseries_create(10000);
    ASSERT_NE(ts, NULL);
    ASSERT_EQ(ag_timeseries_capacity(ts), 10000);
    ASSERT_EQ(ag_timeseries_size(ts), 0);

    /* Append many points */
    for (int i = 0; i < 5000; i++) {
        ag_timeseries_append(ts, i, i * 0.5);
    }
    ASSERT_EQ(ag_timeseries_size(ts), 5000);

    ag_timeseries_destroy(ts);
}

/* Test: Stress test - fill and wrap multiple times */
TEST(stress_multiple_wraps) {
    ag_timeseries_t* ts = ag_timeseries_create(5);
    ASSERT_NE(ts, NULL);

    /* Append 20 points (4x capacity) */
    for (int i = 0; i < 20; i++) {
        ag_timeseries_append(ts, i * 100, i * 1.0);
    }

    ASSERT_EQ(ag_timeseries_size(ts), 5);

    /* Should have last 5: 15, 16, 17, 18, 19 */
    int64_t timestamps[5];
    double values[5];
    size_t count = ag_timeseries_query_last(ts, 5, timestamps, values);

    ASSERT_EQ(count, 5);
    ASSERT_EQ(timestamps[0], 1900);  /* Newest */
    ASSERT_DOUBLE_EQ(values[0], 19.0);
    ASSERT_EQ(timestamps[4], 1500);  /* Oldest */
    ASSERT_DOUBLE_EQ(values[4], 15.0);

    ag_timeseries_destroy(ts);
}

/* Test: Edge case - capacity 1 */
TEST(capacity_one) {
    ag_timeseries_t* ts = ag_timeseries_create(1);
    ASSERT_NE(ts, NULL);

    ag_timeseries_append(ts, 1000, 1.0);
    ASSERT_EQ(ag_timeseries_size(ts), 1);

    ag_timeseries_append(ts, 2000, 2.0);
    ASSERT_EQ(ag_timeseries_size(ts), 1);

    int64_t timestamps[1];
    double values[1];
    size_t count = ag_timeseries_query_last(ts, 1, timestamps, values);

    ASSERT_EQ(count, 1);
    ASSERT_EQ(timestamps[0], 2000);
    ASSERT_DOUBLE_EQ(values[0], 2.0);

    ag_timeseries_destroy(ts);
}

/* Test: Negative timestamps */
TEST(negative_timestamps) {
    ag_timeseries_t* ts = ag_timeseries_create(5);
    ASSERT_NE(ts, NULL);

    ag_timeseries_append(ts, -1000, 1.0);
    ag_timeseries_append(ts, 0, 2.0);
    ag_timeseries_append(ts, 1000, 3.0);

    int64_t timestamps[3];
    double values[3];
    size_t count = ag_timeseries_query_range(ts, -1000, 0, 3, timestamps, values);

    ASSERT_EQ(count, 2);
    ASSERT_EQ(timestamps[0], -1000);
    ASSERT_EQ(timestamps[1], 0);

    ag_timeseries_destroy(ts);
}

/* Test: Query with zero max_points */
TEST(query_zero_max_points) {
    ag_timeseries_t* ts = ag_timeseries_create(10);
    ASSERT_NE(ts, NULL);

    ag_timeseries_append(ts, 1000, 1.0);

    int64_t timestamps[1];
    double values[1];
    size_t count = ag_timeseries_query_last(ts, 0, timestamps, values);
    ASSERT_EQ(count, 0);

    count = ag_timeseries_query_range(ts, 1000, 2000, 0, timestamps, values);
    ASSERT_EQ(count, 0);

    ag_timeseries_destroy(ts);
}

/* Main test runner */
int main(void) {
    printf("=== ag_timeseries Unit Tests ===\n\n");

    RUN_TEST(create_destroy);
    RUN_TEST(create_zero_capacity);
    RUN_TEST(destroy_null);
    RUN_TEST(append_single);
    RUN_TEST(append_null);
    RUN_TEST(append_multiple);
    RUN_TEST(ring_buffer_wraparound);
    RUN_TEST(query_last_empty);
    RUN_TEST(query_last_null);
    RUN_TEST(query_last_null_output);
    RUN_TEST(query_last_fewer_than_requested);
    RUN_TEST(query_last_exact_capacity);
    RUN_TEST(query_range_basic);
    RUN_TEST(query_range_empty);
    RUN_TEST(query_range_null);
    RUN_TEST(query_range_invalid);
    RUN_TEST(query_range_no_matches);
    RUN_TEST(query_range_wraparound);
    RUN_TEST(query_range_max_points_limit);
    RUN_TEST(size_capacity_null);
    RUN_TEST(large_capacity);
    RUN_TEST(stress_multiple_wraps);
    RUN_TEST(capacity_one);
    RUN_TEST(negative_timestamps);
    RUN_TEST(query_zero_max_points);

    printf("\n=== All tests passed! ===\n");
    return 0;
}
