/*
 * debug_example.c - Debug ring buffer wraparound
 */

#include "ag_timeseries.h"
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    printf("Debug Example - Ring Buffer Wraparound\n");
    printf("=======================================\n\n");

    // Create a small buffer
    ag_timeseries_t* ts = ag_timeseries_create(3);
    if (ts == NULL) {
        fprintf(stderr, "Failed to create buffer\n");
        return 1;
    }

    printf("Buffer created: capacity=%zu, size=%zu\n\n",
           ag_timeseries_capacity(ts), ag_timeseries_size(ts));

    // Append points one by one and query after each
    for (int i = 0; i < 10; i++) {
        int64_t timestamp = 1000 + (i * 100);
        double value = (double)i;

        printf("Appending: timestamp=%lld, value=%.1f\n",
               (long long)timestamp, value);
        ag_timeseries_append(ts, timestamp, value);

        printf("  Size: %zu\n", ag_timeseries_size(ts));

        // Query all points
        int64_t timestamps[3];
        double values[3];
        size_t count = ag_timeseries_query_last(ts, 3, timestamps, values);

        printf("  Query last (newest first):\n");
        for (size_t j = 0; j < count; j++) {
            printf("    [%zu] timestamp=%lld, value=%.1f\n",
                   j, (long long)timestamps[j], values[j]);
        }
        printf("\n");
    }

    ag_timeseries_destroy(ts);
    printf("Done!\n");
    return 0;
}
