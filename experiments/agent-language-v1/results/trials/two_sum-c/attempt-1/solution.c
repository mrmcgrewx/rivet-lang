#include "api.h"
#include <stdlib.h>

typedef struct {
    int64_t value;
    size_t index_plus_one;
} Entry;

static size_t hash_value(int64_t value, size_t mask)
{
    uint64_t mixed = (uint64_t)value;
    mixed ^= mixed >> 30;
    mixed *= UINT64_C(0xbf58476d1ce4e5b9);
    mixed ^= mixed >> 27;
    mixed *= UINT64_C(0x94d049bb133111eb);
    mixed ^= mixed >> 31;
    return (size_t)mixed & mask;
}

Pair solve(const int64_t *values, size_t count, int64_t target)
{
    size_t capacity = 1;
    while (capacity < count * 2) {
        capacity *= 2;
    }

    Entry *table = calloc(capacity, sizeof(*table));
    if (table == NULL) {
        return (Pair){SIZE_MAX, SIZE_MAX};
    }
    const size_t mask = capacity - 1;

    for (size_t i = 0; i < count; ++i) {
        const int64_t complement = target - values[i];
        size_t slot = hash_value(complement, mask);
        while (table[slot].index_plus_one != 0) {
            if (table[slot].value == complement) {
                const Pair result = {table[slot].index_plus_one - 1, i};
                free(table);
                return result;
            }
            slot = (slot + 1) & mask;
        }

        slot = hash_value(values[i], mask);
        while (table[slot].index_plus_one != 0 &&
               table[slot].value != values[i]) {
            slot = (slot + 1) & mask;
        }
        table[slot].value = values[i];
        table[slot].index_plus_one = i + 1;
    }

    free(table);
    return (Pair){SIZE_MAX, SIZE_MAX};
}
