/* Generic computation helpers, independently exercised without a Rivet program. */
#include "../../native/runtime.c"
#include <assert.h>

static void check_compute(void) {
    uint64_t *counter = rv2_fuel_counter();
    assert(counter == &rv2_steps && *counter == 0);
    rv2_tick();
    assert(*counter == 1);
    *counter = 39;
    for (size_t index = 0; index < 16; ++index) {
        assert(rv2_fuel_counter() == counter && *counter == 39);
    }
    rv2_tick();
    assert(*counter == 40);
    rv_cleanup();
    assert(rv2_fuel_counter() == counter && *counter == 0);
    int64_t input[] = {INT64_MIN, 0, INT64_MAX};
    void *array = rv2_array_make(input, 3);
    input[1] = 123;
    assert(rv2_array_len(array) == 3);
    assert(rv2_array_get(array, 0) == INT64_MIN);
    assert(rv2_array_get(array, 1) == 0);
    assert(rv2_array_get(array, 2) == INT64_MAX);
    assert(rv2_array_len(rv2_array_make(NULL, 0)) == 0);
    void *map = rv2_map_new();
    assert(rv2_map_get(map, 0, INT64_MIN) == INT64_MIN);
    rv2_map_put(map, 0, INT64_MAX);
    rv2_map_put(map, INT64_MIN, -1);
    rv2_map_put(map, INT64_MAX, INT64_MIN);
    assert(rv2_map_get(map, 0, 12) == INT64_MAX);
    assert(rv2_map_get(map, INT64_MIN, 12) == -1);
    assert(rv2_map_get(map, INT64_MAX, 12) == INT64_MIN);
    rv2_map_put(map, 0, -77);
    assert(rv2_map_get(map, 0, 12) == -77);

    /* Deliberately force a probe chain; find same-bucket keys independently of
       map insertion so collisions and overwrite traversal are both exercised. */
    int64_t collisions[32];
    size_t found = 0;
    for (int64_t key = 1; found < 32; ++key) {
        if (rv2_hash(key) == 0) collisions[found++] = key;
    }
    void *collision_map = rv2_map_new();
    for (size_t index = 0; index < 32; ++index) {
        rv2_map_put(collision_map, collisions[index], (int64_t)index);
    }
    for (size_t index = 0; index < 32; ++index) {
        assert(rv2_map_get(collision_map, collisions[index], -1) == (int64_t)index);
        rv2_map_put(collision_map, collisions[index], -(int64_t)index);
        assert(rv2_map_get(collision_map, collisions[index], -100) == -(int64_t)index);
    }
    void *full = rv2_map_new();
    for (int64_t key = -5000; key < 5000; ++key) rv2_map_put(full, key, key * 2);
    for (int64_t key = -5000; key < 5000; ++key) {
        assert(rv2_map_get(full, key, INT64_MAX) == key * 2);
        rv2_map_put(full, key, -key);
        assert(rv2_map_get(full, key, INT64_MAX) == -key);
    }
    assert(rv2_map_get(full, 5000, -99) == -99);
    for (uint64_t step = 0; step < RV2_MAX_STEPS; ++step) rv2_tick();
    assert(rv2_steps == RV2_MAX_STEPS && *counter == RV2_MAX_STEPS);
    assert(rv2_fuel_counter() == counter && *counter == RV2_MAX_STEPS);
    rv_cleanup();
    rv_cleanup();
    assert(rv2_steps == 0 && rv2_allocation_charge == 0 && rv2_input == NULL);
    assert(rv2_fuel_counter() == counter && *counter == 0);
    rv2_tick();
    assert(rv2_steps == 1);
    rv_cleanup();
}

int main(int argc, char **argv) {
    if (argc == 1) { check_compute(); return 0; }
    if (strcmp(argv[1], "read-int") == 0) {
        int64_t value = rv2_read_int();
        rv2_return(rv2_array_make(&value, 1));
    } else if (strcmp(argv[1], "read-array") == 0) {
        rv2_return(rv2_read_array());
    } else if (strcmp(argv[1], "array-then-int") == 0) {
        void *array = rv2_read_array();
        int64_t output[] = {rv2_array_len(array), rv2_read_int()};
        rv2_return(rv2_array_make(output, 2));
    } else if (strcmp(argv[1], "empty") == 0) {
        rv2_return(rv2_array_make(NULL, 0));
    } else if (strcmp(argv[1], "array-limit") == 0) {
        int64_t value = 0;
        (void)rv2_array_make(&value, 10001);
    } else if (strcmp(argv[1], "index-negative") == 0 || strcmp(argv[1], "index-upper") == 0) {
        int64_t value = 1;
        int64_t index = strcmp(argv[1], "index-negative") == 0 ? -1 : 1;
        (void)rv2_array_get(rv2_array_make(&value, 1), index);
    } else if (strcmp(argv[1], "map-limit") == 0) {
        void *map = rv2_map_new();
        for (int64_t key = 0; key <= 10000; ++key) rv2_map_put(map, key, 0);
    } else if (strcmp(argv[1], "quota-map") == 0) {
        for (size_t index = 0; index < 86; ++index) (void)rv2_map_new();
    } else if (strcmp(argv[1], "quota-array") == 0 || strcmp(argv[1], "quota-array-ok") == 0) {
        int64_t values[10000] = {0};
        for (size_t index = 0; index < 838; ++index) (void)rv2_array_make(values, 10000);
        (void)rv2_array_make(values, 8608);
        assert(rv2_allocation_charge == RV2_MAX_ALLOCATION);
        if (strcmp(argv[1], "quota-array") == 0) (void)rv2_array_make(values, 1);
    } else if (strcmp(argv[1], "fuel") == 0) {
        for (uint64_t step = 0; step <= RV2_MAX_STEPS; ++step) rv2_tick();
    } else if (strcmp(argv[1], "fuel-getter-last") == 0 || strcmp(argv[1], "fuel-getter") == 0) {
        uint64_t *counter = rv2_fuel_counter();
        *counter = RV2_MAX_STEPS - 1;
        rv2_tick();
        assert(*counter == RV2_MAX_STEPS);
        assert(rv2_fuel_counter() == counter && *counter == RV2_MAX_STEPS);
        if (strcmp(argv[1], "fuel-getter") == 0) rv2_tick();
    } else if (strcmp(argv[1], "wrong-type") == 0) {
        (void)rv2_array_len(rv2_map_new());
    } else {
        return 2;
    }
    rv_cleanup();
    return 0;
}
