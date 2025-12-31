/*
 * simple2.c - Simpler test
 */

#include "ag_timeseries.h"
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    ag_timeseries_t* ts = ag_timeseries_create(10);
    if (ts == NULL) {
        fprintf(stderr, "Create failed\n");
        return 1;
    }

    printf("Created buffer\n");

    // Fill buffer
    for (int i = 0; i < 10; i++) {
        ag_timeseries_append(ts, 1000 + i, i * 1.0);
    }

    printf("Filled buffer with 10 points\n");

    // Wrap around
    for (int i = 10; i < 20; i++) {
        ag_timeseries_append(ts, 1000 + i, i * 1.0);
    }

    printf("Wrapped around with 10 more points\n");
    printf("Size: %zu\n", ag_timeseries_size(ts));

    // Query
    int64_t timestamps[10];
    double values[10];
    size_t count = ag_timeseries_query_last(ts, 10, timestamps, values);

    printf("Query returned %zu points:\n", count);
    for (size_t i = 0; i < count; i++) {
        printf("  [%zu] %lld -> %.1f\n", i, (long long)timestamps[i], values[i]);
    }

    printf("About to destroy...\n");
    fflush(stdout);

    ag_timeseries_destroy(ts);

    printf("Destroyed successfully\n");
    return 0;
}
