#include "api.h"

double solve(const int64_t *first, size_t first_count,
             const int64_t *second, size_t second_count)
{
    if (first_count > second_count) {
        const int64_t *array_swap = first;
        first = second;
        second = array_swap;
        size_t count_swap = first_count;
        first_count = second_count;
        second_count = count_swap;
    }

    const size_t total = first_count + second_count;
    const size_t left_count = (total + 1) / 2;
    size_t low = 0;
    size_t high = first_count;

    /* Find the first partition whose right side in the shorter array
       is not smaller than the left side in the longer array. */
    while (low < high) {
        const size_t first_cut = low + (high - low) / 2;
        const size_t second_cut = left_count - first_cut;

        /* first_cut < first_count and 0 < second_cut <= second_count
           follow from searching the shorter array. */
        if (second[second_cut - 1] > first[first_cut]) {
            low = first_cut + 1;
        } else {
            high = first_cut;
        }
    }

    const size_t first_cut = low;
    const size_t second_cut = left_count - first_cut;
    const int64_t first_left = first_cut == 0
        ? INT64_MIN : first[first_cut - 1];
    const int64_t second_left = second_cut == 0
        ? INT64_MIN : second[second_cut - 1];
    const int64_t left_max = first_left > second_left
        ? first_left : second_left;

    if (total % 2 != 0) {
        return (double)left_max;
    }

    const int64_t first_right = first_cut == first_count
        ? INT64_MAX : first[first_cut];
    const int64_t second_right = second_cut == second_count
        ? INT64_MAX : second[second_cut];
    const int64_t right_min = first_right < second_right
        ? first_right : second_right;
    return ((double)left_max + (double)right_min) / 2.0;
}
