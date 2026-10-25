#include "partition.h"
#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    int64_t a[1000], b[1000], m, n;
    if (scanf("%" SCNd64, &m) != 1 || m < 0 || m > 1000) return 1;
    for (size_t i = 0; i < (size_t)m; ++i) if (scanf("%" SCNd64, &a[i]) != 1) return 1;
    if (scanf("%" SCNd64, &n) != 1 || n < 0 || n > 1000) return 1;
    for (size_t i = 0; i < (size_t)n; ++i) if (scanf("%" SCNd64, &b[i]) != 1) return 1;
    char extra;
    if (scanf(" %c", &extra) == 1 || m + n == 0) return 1;
    if (printf("%.16e\n", median_partition(a, (size_t)m, b, (size_t)n)) < 0) return 1;
    return fflush(stdout) == 0 ? 0 : 1;
}
