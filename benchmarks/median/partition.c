/* The same shorter-array binary partition used by the Rivet example. */
#include "partition.h"
#include <stdlib.h>

double median_partition(const int64_t *a, size_t m, const int64_t *b, size_t n) {
    if (m > 1000 || n > 1000 || m + n == 0) abort();
    if (m > n) {
        const int64_t *temporary = a; a = b; b = temporary;
        size_t length = m; m = n; n = length;
    }
    int64_t low = 0, high = (int64_t)m;
    int64_t half = (int64_t)(m + n + 1) / 2;
    while (low <= high) {
        int64_t i = (low + high) / 2, j = half - i;
        int64_t left_a = i == 0 ? INT64_MIN : a[(size_t)i - 1];
        int64_t right_a = i == (int64_t)m ? INT64_MAX : a[(size_t)i];
        int64_t left_b = j == 0 ? INT64_MIN : b[(size_t)j - 1];
        int64_t right_b = j == (int64_t)n ? INT64_MAX : b[(size_t)j];
        if (left_a <= right_b && left_b <= right_a) {
            int64_t left = left_a > left_b ? left_a : left_b;
            if ((m + n) % 2 != 0) return (double)left;
            int64_t right = right_a < right_b ? right_a : right_b;
            return ((double)left + (double)right) / 2.0;
        }
        if (left_a > right_b) high = i - 1;
        else low = i + 1;
    }
    abort(); /* Sorted inputs guarantee a partition. */
}
