/*
 * simple_example.c - Simple demonstration of ag_timeseries library
 *
 * Compile:
 *   gcc -I../include simple_example.c -L../lib -lag_core -o simple_example
 *
 * Run:
 *   ./simple_example
 */

#include "ag_timeseries.h"
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    printf("ag_timeseries Simple Example\n");
    printf("==============================\n\n");

    // Create a buffer for 10 data points
    printf("Creating time-series buffer with capacity 10...\n");
    ag_timeseries_t* ts = ag_timeseries_create(10);
    if (ts == NULL) {
        fprintf(stderr, "Failed to create time-series buffer\n");
        return 1;
    }

    printf("Buffer created. Capacity: %zu, Size: %zu\n\n",
           ag_timeseries_capacity(ts), ag_timeseries_size(ts));

    // Append some data points
    printf("Appending 5 data points...\n");
    int64_t base_time = 1735689600000;  // 2025-01-01 00:00:00 UTC
    for (int i = 0; i < 5; i++) {
        int64_t timestamp = base_time + (i * 1000);  // 1 second intervals
        double value = 10.0 + (i * 2.5);

        int result = ag_timeseries_append(ts, timestamp, value);
        if (result != AG_OK) {
            fprintf(stderr, "Failed to append data point %d\n", i);
            ag_timeseries_destroy(ts);
            return 1;
        }
        printf("  Appended: timestamp=%lld, value=%.2f\n",
               (long long)timestamp, value);
    }

    printf("\nBuffer size after append: %zu\n\n", ag_timeseries_size(ts));

    // Query last 3 points
    printf("Querying last 3 points (newest first):\n");
    int64_t timestamps[3];
    double values[3];
    size_t count = ag_timeseries_query_last(ts, 3, timestamps, values);

    for (size_t i = 0; i < count; i++) {
        printf("  [%zu] timestamp=%lld, value=%.2f\n",
               i, (long long)timestamps[i], values[i]);
    }
    printf("\n");

    // Query range
    printf("Querying range [%lld, %lld]:\n",
           (long long)(base_time + 1000), (long long)(base_time + 3000));
    int64_t range_timestamps[10];
    double range_values[10];
    count = ag_timeseries_query_range(ts, base_time + 1000, base_time + 3000,
                                      10, range_timestamps, range_values);

    for (size_t i = 0; i < count; i++) {
        printf("  [%zu] timestamp=%lld, value=%.2f\n",
               i, (long long)range_timestamps[i], range_values[i]);
    }
    printf("\n");

    // Test ring buffer wraparound
    printf("Testing ring buffer wraparound...\n");
    printf("Appending 15 more points (buffer capacity is 10)...\n");
    for (int i = 5; i < 20; i++) {
        int64_t timestamp = base_time + (i * 1000);
        double value = 10.0 + (i * 2.5);
        ag_timeseries_append(ts, timestamp, value);
    }

    printf("Buffer size after wraparound: %zu (should be 10)\n", ag_timeseries_size(ts));
    printf("Querying last 5 points:\n");
    count = ag_timeseries_query_last(ts, 5, timestamps, values);
    for (size_t i = 0; i < count; i++) {
        printf("  [%zu] timestamp=%lld, value=%.2f\n",
               i, (long long)timestamps[i], values[i]);
    }
    printf("\n");

    // Cleanup
    printf("Destroying buffer...\n");
    ag_timeseries_destroy(ts);
    printf("Done!\n");

    return 0;
}
