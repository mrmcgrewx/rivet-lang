/* Generic immutable linked-node ABI tests; no application arithmetic. */
#include "../../native/runtime.c"
#include <assert.h>

static void check_lists(void) {
    assert(rv2_list_len(NULL) == 0);
    assert(rv2_allocation_charge == 0);
    void *tail = rv2_list_cons(INT64_MIN, NULL);
    void *left = rv2_list_cons(-12, tail);
    void *right = rv2_list_cons(INT64_MAX, tail);
    assert(left != right);
    assert(rv2_list_len(left) == 2 && rv2_list_len(right) == 2);
    assert(rv2_list_next(left) == tail && rv2_list_next(right) == tail);
    assert(rv2_list_value(left) == -12);
    assert(rv2_list_value(right) == INT64_MAX);
    assert(rv2_list_value(tail) == INT64_MIN);
    assert(rv2_list_next(tail) == NULL && rv2_list_len(tail) == 1);
    assert(rv2_allocation_charge == 3 * 24);
    assert(((rv_value *)tail)->kind == RV_INT_LIST);

    void *deep = NULL;
    for (int64_t index = 0; index < 10000; ++index) {
        deep = rv2_list_cons(index, deep);
        assert(rv2_list_len(deep) == index + 1);
    }
    for (int64_t remaining = 10000; remaining != 0; --remaining) {
        assert(rv2_list_len(deep) == remaining);
        assert(rv2_list_value(deep) == remaining - 1);
        deep = rv2_list_next(deep);
    }
    assert(deep == NULL);
    assert(rv2_allocation_charge == 10003 * 24);
    /* Ownership is the iterative allocation chain, never recursive tail freeing. */
    rv_cleanup();
    rv_cleanup();
    assert(rv_allocations == NULL && rv2_allocation_charge == 0);
}

static void fill_quota(void) {
    for (size_t index = 0; index < 85; ++index) (void)rv2_map_new();
    int64_t values[2] = {0};
    (void)rv2_array_make(values, 2);
    for (size_t index = 0; index < 10922; ++index) (void)rv2_list_cons(0, NULL);
    assert(rv2_allocation_charge == RV2_MAX_ALLOCATION);
}

int main(int argc, char **argv) {
    if (argc == 1) { check_lists(); return 0; }
    if (strcmp(argv[1], "read") == 0) {
        void *list = rv2_read_list();
        assert(rv2_allocation_charge == (size_t)rv2_list_len(list) * 24);
        rv2_return_list(list);
    } else if (strcmp(argv[1], "two-lists") == 0) {
        void *left = rv2_read_list();
        void *right = rv2_read_list();
        int64_t values[] = {rv2_list_len(left), rv2_list_len(right),
                            rv2_list_value(left), rv2_list_value(right)};
        rv2_return(rv2_array_make(values, 4));
    } else if (strcmp(argv[1], "empty") == 0) {
        rv2_return_list(NULL);
    } else if (strcmp(argv[1], "shared-output") == 0) {
        void *tail = rv2_list_cons(2, rv2_list_cons(3, NULL));
        void *first = rv2_list_cons(1, tail);
        void *second = rv2_list_cons(4, tail);
        assert(rv2_list_next(first) == rv2_list_next(second));
        assert(rv2_list_value(first) == 1 && rv2_list_value(tail) == 2);
        rv2_return_list(second);
    } else if (strcmp(argv[1], "nil-value") == 0) {
        (void)rv2_list_value(NULL);
    } else if (strcmp(argv[1], "nil-next") == 0) {
        (void)rv2_list_next(NULL);
    } else if (strcmp(argv[1], "wrong-type") == 0) {
        (void)rv2_list_len(rv2_array_make(NULL, 0));
    } else if (strcmp(argv[1], "length-limit") == 0 || strcmp(argv[1], "length-before-budget") == 0) {
        void *list = NULL;
        for (size_t index = 0; index < 10000; ++index) list = rv2_list_cons(0, list);
        if (strcmp(argv[1], "length-before-budget") == 0) {
            rv2_charge(RV2_MAX_ALLOCATION - rv2_allocation_charge);
        }
        (void)rv2_list_cons(0, list);
    } else if (strcmp(argv[1], "quota-ok") == 0 || strcmp(argv[1], "quota-exceeded") == 0) {
        fill_quota();
        if (strcmp(argv[1], "quota-exceeded") == 0) (void)rv2_list_cons(0, NULL);
    } else if (strcmp(argv[1], "read-exhausted-budget") == 0) {
        /* Establish exhaustion without allocating needless test memory. */
        rv2_charge(RV2_MAX_ALLOCATION);
        rv2_return_list(rv2_read_list());
    } else {
        return 2;
    }
    rv_cleanup();
    return 0;
}
