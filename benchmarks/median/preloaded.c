/* Benchmark-only boundaries. No changes to the production runtime or source. */
#define _POSIX_C_SOURCE 200809L
#ifdef BENCH_RIVET
#include "../../native/runtime.c"
#endif
#include "partition.h"
#include <errno.h>
#include <inttypes.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

typedef struct {
    int64_t *a, *b;
    size_t m, n;
    double expected;
#ifdef BENCH_RIVET
    void *rivet_a, *rivet_b;
    uint64_t ticks;
#endif
} bench_case;

static void require(int condition, const char *message) {
    if (!condition) { fprintf(stderr, "median benchmark: %s\n", message); exit(1); }
}

#ifdef BENCH_RIVET
int rivet_bench_eval(void);
static void *bench_inputs[2];
static size_t bench_reads, bench_returns, bench_cleanups;
static size_t bench_allocation_charge;
static double bench_result;

void *bench_read_array(void) {
    require(bench_reads < 2, "generated program read more than two arrays");
    return bench_inputs[bench_reads++];
}
void bench_return_float(double value) {
    require(isfinite(value) && bench_returns == 0, "invalid generated return");
    bench_result = value;
    ++bench_returns;
}
void bench_cleanup(void) { ++bench_cleanups; }

static double evaluate(bench_case *test) {
    bench_inputs[0] = test->rivet_a;
    bench_inputs[1] = test->rivet_b;
    bench_reads = bench_returns = bench_cleanups = 0;
    rv2_steps = 0; /* New independent invocation; keep every real fuel call. */
    int status = rivet_bench_eval();
    require(status == 0 && bench_reads == 2 && bench_returns == 1 && bench_cleanups == 1,
            "generated invocation did not match boundary contract");
    require(rv2_allocation_charge == bench_allocation_charge,
            "generated search unexpectedly allocated; harness cannot defer those cleanups");
    test->ticks = rv2_steps;
    return bench_result;
}
#else
static double evaluate(bench_case *test) {
    return median_partition(test->a, test->m, test->b, test->n);
}
#endif

static double control(bench_case *test) {
    /* A volatile read keeps the control loop observable. This is only a harness
       floor; it does not estimate/remove Rivet's boundary or safety overhead. */
    return *(volatile double *)&test->expected;
}

static uint64_t nanoseconds(void) {
    struct timespec value;
    require(clock_gettime(CLOCK_MONOTONIC, &value) == 0, "clock_gettime failed");
    return (uint64_t)value.tv_sec * UINT64_C(1000000000) + (uint64_t)value.tv_nsec;
}

static uint64_t count_argument(const char *text) {
    char *end = NULL;
    errno = 0;
    unsigned long long value = strtoull(text, &end, 10);
    require(errno == 0 && text != end && *end == '\0' && text[0] != '-', "invalid repetition count");
    return (uint64_t)value;
}

static int64_t *read_array(FILE *file, size_t *length) {
    int64_t count;
    require(fscanf(file, "%" SCNd64, &count) == 1 && count >= 0 && count <= 1000, "invalid array length");
    *length = (size_t)count;
    int64_t *values = malloc((*length + 1) * sizeof(*values));
    require(values != NULL, "cannot allocate case");
    for (size_t i = 0; i < *length; ++i) {
        require(fscanf(file, "%" SCNd64, &values[i]) == 1, "missing element");
        require(values[i] >= -1000000 && values[i] <= 1000000, "element outside benchmark bounds");
        require(i == 0 || values[i - 1] <= values[i], "unsorted benchmark case");
    }
    return values;
}

int main(int argc, char **argv) {
    require(argc == 5, "expected DATASET ROUNDS WARMUP_ROUNDS evaluate|control");
    uint64_t rounds = count_argument(argv[2]), warmups = count_argument(argv[3]);
    int evaluating = strcmp(argv[4], "evaluate") == 0;
    require(evaluating || strcmp(argv[4], "control") == 0, "invalid mode");
    FILE *file = fopen(argv[1], "r");
    require(file != NULL, "cannot open dataset");
    size_t count;
    require(fscanf(file, "%zu", &count) == 1 && count > 0 && count <= 256, "invalid case count");
    require(rounds > 0 && rounds <= UINT64_C(100000000) / count && warmups <= UINT64_C(100000000) / count,
            "too many repetitions");
    bench_case *cases = calloc(count, sizeof(*cases));
    require(cases != NULL, "cannot allocate cases");
    for (size_t i = 0; i < count; ++i) {
        cases[i].a = read_array(file, &cases[i].m);
        cases[i].b = read_array(file, &cases[i].n);
        require(cases[i].m + cases[i].n != 0 && fscanf(file, "%lf", &cases[i].expected) == 1 &&
                isfinite(cases[i].expected), "invalid expected median");
#ifdef BENCH_RIVET
        cases[i].rivet_a = rv2_array_make(cases[i].a, cases[i].m);
        cases[i].rivet_b = rv2_array_make(cases[i].b, cases[i].n);
        free(cases[i].a); free(cases[i].b);
        cases[i].a = cases[i].b = NULL;
#endif
    }
    char extra;
    require(fscanf(file, " %c", &extra) != 1, "extra dataset input");
    require(fclose(file) == 0, "cannot close dataset");
#ifdef BENCH_RIVET
    bench_allocation_charge = rv2_allocation_charge;
#endif
    double expected_checksum = 0.0;
    for (size_t i = 0; i < count; ++i) {
        require(evaluate(&cases[i]) == cases[i].expected, "oracle mismatch");
        expected_checksum += cases[i].expected * (double)(1 + i % 17);
    }
    double (*invoke)(bench_case *) = evaluating ? evaluate : control;
    for (uint64_t round = 0; round < warmups; ++round) {
        for (size_t i = 0; i < count; ++i) (void)invoke(&cases[i]);
    }
    double checksum = 0.0;
    uint64_t started = nanoseconds();
    for (uint64_t round = 0; round < rounds; ++round) {
        for (size_t i = 0; i < count; ++i) checksum += invoke(&cases[i]) * (double)(1 + i % 17);
    }
    uint64_t elapsed = nanoseconds() - started;
    require(checksum == expected_checksum * (double)rounds, "checksum mismatch");
#ifdef BENCH_RIVET
    const char *implementation = "rivet";
#else
    const char *implementation = "c";
#endif
    printf("{\"implementation\":\"%s\",\"mode\":\"%s\",\"cases\":%zu,\"rounds\":%" PRIu64
           ",\"calls\":%" PRIu64 ",\"warmup_calls\":%" PRIu64 ",\"elapsed_ns\":%" PRIu64
           ",\"checksum\":%.17g,\"expected_checksum_per_round\":%.17g",
           implementation, argv[4], count, rounds, count * rounds, count * warmups, elapsed,
           checksum, expected_checksum);
#ifdef BENCH_RIVET
    printf(",\"ticks_per_case\":[");
    for (size_t i = 0; i < count; ++i) printf("%s%" PRIu64, i == 0 ? "" : ",", cases[i].ticks);
    printf("]");
#endif
    printf("}\n");
    for (size_t i = 0; i < count; ++i) { free(cases[i].a); free(cases[i].b); }
    free(cases);
#ifdef BENCH_RIVET
    rv_cleanup(); /* The real cleanup runs once, outside the timed region. */
#endif
    return fflush(stdout) == 0 ? 0 : 1;
}
