/* Unmeasured setup scaffolding. No task solver or oracle lives in this driver. */
#define _POSIX_C_SOURCE 200809L
#include "backend.h"
#include <errno.h>
#include <inttypes.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#if TASK < 1 || TASK > 3
#error Invalid TASK
#endif

typedef struct { FILE *file; size_t bytes; } Reader;
static void require(int condition, const char *message) {
    if (!condition) { fprintf(stderr, "experiment harness: %s\n", message); exit(1); }
}
static int space(int character) {
    return character == ' ' || (character >= '\t' && character <= '\r');
}
static int read_byte(Reader *reader) {
    int character = fgetc(reader->file);
    require(!ferror(reader->file), "cannot read input");
    if (character != EOF) {
        ++reader->bytes;
        require(reader->bytes <= (size_t)128 * 1024 * 1024, "input exceeds 128 MiB harness limit");
    }
    return character;
}
static int64_t integer(Reader *reader) {
    int character;
    do { character = read_byte(reader); } while (space(character));
    require(character != EOF, "missing integer");
    int negative = character == '-';
    if (negative) character = read_byte(reader);
    require(character >= '0' && character <= '9', "invalid signed integer");
    uint64_t magnitude = 0;
    uint64_t limit = negative ? UINT64_C(9223372036854775808) : UINT64_C(9223372036854775807);
    while (character >= '0' && character <= '9') {
        uint64_t digit = (uint64_t)(character - '0');
        require(magnitude <= (limit - digit) / 10, "integer outside signed i64 range");
        magnitude = magnitude * 10 + digit;
        character = read_byte(reader);
    }
    require(character == EOF || space(character), "invalid integer suffix");
    if (negative && magnitude == UINT64_C(9223372036854775808)) return INT64_MIN;
    return negative ? -(int64_t)magnitude : (int64_t)magnitude;
}
static void finish_input(Reader *reader) {
    int character;
    do { character = read_byte(reader); } while (space(character));
    require(character == EOF, "unconsumed input");
}
static int64_t *array(Reader *reader, size_t *length, size_t maximum) {
    int64_t count = integer(reader);
    require(count >= 0 && (uint64_t)count <= maximum, "array length outside harness bounds");
    *length = (size_t)count;
    int64_t *values = malloc((*length + 1) * sizeof(*values));
    require(values != NULL, "cannot allocate input array");
    for (size_t i = 0; i < *length; ++i) values[i] = integer(reader);
    return values;
}
static void read_case(Reader *reader, Case *input, int expected) {
    input->a = array(reader, &input->na, 10000);
#if TASK == 1
    input->target = integer(reader);
#else
    input->b = array(reader, &input->nb, 10000);
#endif
    if (expected) {
        input->expected = array(reader, &input->expected_n, ANSWER_CAPACITY);
#if TASK == 1
        require(input->expected_n == 2, "expected pair must contain two indices");
        if (input->expected[0] > input->expected[1]) {
            int64_t temporary = input->expected[0];
            input->expected[0] = input->expected[1];
            input->expected[1] = temporary;
        }
#elif TASK == 3
        require(input->expected_n == 1, "expected twice-median must contain one integer");
#endif
    }
}
static void normalize(Answer *answer) {
    require(answer->n <= ANSWER_CAPACITY, "answer exceeds capture capacity");
#if TASK == 1
    require(answer->n == 2, "solver must return two indices");
    if (answer->values[0] > answer->values[1]) {
        int64_t temporary = answer->values[0];
        answer->values[0] = answer->values[1];
        answer->values[1] = temporary;
    }
#elif TASK == 3
    double doubled = answer->scalar * 2.0;
    require(isfinite(answer->scalar) && isfinite(doubled) &&
            doubled >= -9223372036854775808.0 && doubled < 9223372036854775808.0,
            "median cannot be represented as finite twice-i64");
    int64_t value = (int64_t)doubled;
    require((double)value == doubled, "median is not an integer or half integer");
    answer->n = 1;
    answer->values[0] = value;
#endif
}
static uint64_t hash(const int64_t *values, size_t count) {
    uint64_t result = UINT64_C(1469598103934665603);
    result = (result ^ (uint64_t)count) * UINT64_C(1099511628211);
    for (size_t i = 0; i < count; ++i) result = (result ^ (uint64_t)values[i]) * UINT64_C(1099511628211);
    return result;
}
static uint64_t evaluate(Case *input, Answer *answer) {
    backend_solve(input, answer);
    normalize(answer);
    uint64_t result = hash(answer->values, answer->n);
    backend_release(answer);
    return result;
}
static uint64_t clock_ns(void) {
    struct timespec value;
    require(clock_gettime(CLOCK_MONOTONIC, &value) == 0, "monotonic clock failed");
    return (uint64_t)value.tv_sec * UINT64_C(1000000000) + (uint64_t)value.tv_nsec;
}
static uint64_t argument(const char *text) {
    require(text[0] >= '0' && text[0] <= '9', "invalid benchmark count");
    char *end = NULL;
    errno = 0;
    unsigned long long value = strtoull(text, &end, 10);
    require(errno == 0 && *end == '\0', "invalid benchmark count");
    return (uint64_t)value;
}
static void free_cases(Case *cases, size_t count) {
    backend_finish(cases, count);
    for (size_t i = 0; i < count; ++i) { free(cases[i].a); free(cases[i].b); free(cases[i].expected); }
    free(cases);
}
int main(int argc, char **argv) {
    int benchmark = argc != 1;
    require(!benchmark || (argc == 6 && strcmp(argv[1], "--bench") == 0),
            "usage: program [--bench POOL ROUNDS WARMUPS evaluate|control]");
    uint64_t rounds = 1, warmups = 0;
    int measuring = 1;
    FILE *file = stdin;
    if (benchmark) {
        rounds = argument(argv[3]);
        warmups = argument(argv[4]);
        measuring = strcmp(argv[5], "evaluate") == 0;
        require(measuring || strcmp(argv[5], "control") == 0, "invalid benchmark mode");
        require(rounds >= 1 && warmups <= 10000, "benchmark counts outside bounds");
        file = fopen(argv[2], "r");
        require(file != NULL, "cannot open benchmark pool");
    }
    Reader reader = {file, 0};
    int64_t declared = benchmark ? integer(&reader) : 1;
    require(declared >= 1 && declared <= 256, "pool must contain 1 to 256 cases");
    size_t count = (size_t)declared;
    require(rounds <= UINT64_C(100000000) / count, "benchmark exceeds 100000000 calls");
    Case *cases = calloc(count, sizeof(*cases));
    require(cases != NULL, "cannot allocate cases");
    for (size_t i = 0; i < count; ++i) read_case(&reader, &cases[i], benchmark);
    finish_input(&reader);
    if (benchmark) require(fclose(file) == 0, "cannot close benchmark pool");
    backend_prepare(cases, count);
    Answer answer;
    if (!benchmark) {
        backend_solve(&cases[0], &answer);
        normalize(&answer);
#if TASK == 3
        require(printf("%.16e\n", answer.scalar) >= 0, "cannot print median");
#else
        require(fputc('[', stdout) != EOF, "cannot print answer");
        for (size_t i = 0; i < answer.n; ++i) {
            require(printf("%s%" PRId64, i == 0 ? "" : ",", answer.values[i]) >= 0, "cannot print answer");
        }
        require(fputs("]\n", stdout) != EOF, "cannot print answer");
#endif
        backend_release(&answer);
        free_cases(cases, count);
        return fflush(stdout) == 0 ? 0 : 1;
    }
    uint64_t expected_hashes[256], ticks[256];
    uint64_t expected_sum = 0;
    for (size_t i = 0; i < count; ++i) {
        backend_solve(&cases[i], &answer);
        normalize(&answer);
        int matches = answer.n == cases[i].expected_n;
        if (matches) {
            for (size_t j = 0; j < answer.n; ++j) {
                if (answer.values[j] != cases[i].expected[j]) matches = 0;
            }
        }
        ticks[i] = answer.ticks;
        backend_release(&answer);
        if (!matches) {
            fprintf(stderr, "experiment harness: preflight answer mismatch at case %zu\n", i);
            free_cases(cases, count);
            return 1;
        }
        expected_hashes[i] = hash(cases[i].expected, cases[i].expected_n);
        expected_sum += expected_hashes[i] * (uint64_t)(1 + i % 17);
    }
    for (uint64_t round = 0; round < warmups; ++round) {
        for (size_t i = 0; i < count; ++i) {
            if (measuring) (void)evaluate(&cases[i], &answer);
            else (void)*(volatile uint64_t *)&expected_hashes[i];
        }
    }
    uint64_t checksum = 0;
    uint64_t started = clock_ns();
    for (uint64_t round = 0; round < rounds; ++round) {
        for (size_t i = 0; i < count; ++i) {
            uint64_t value = measuring ? evaluate(&cases[i], &answer) : *(volatile uint64_t *)&expected_hashes[i];
            checksum += value * (uint64_t)(1 + i % 17);
        }
    }
    uint64_t elapsed = clock_ns() - started;
    int valid = checksum == expected_sum * rounds;
    free_cases(cases, count);
    require(valid, "timed checksum mismatch");
    printf("{\"mode\":\"%s\",\"cases\":%zu,\"rounds\":%" PRIu64 ",\"calls\":%" PRIu64
           ",\"warmup_calls\":%" PRIu64 ",\"elapsed_ns\":%" PRIu64 ",\"checksum\":%" PRIu64
           ",\"expected_checksum_per_round\":%" PRIu64 ",\"ticks_per_case\":[",
           argv[5], count, rounds, (uint64_t)count * rounds, (uint64_t)count * warmups, elapsed, checksum, expected_sum);
    for (size_t i = 0; i < count; ++i) printf("%s%" PRIu64, i == 0 ? "" : ",", ticks[i]);
    printf("]}\n");
    return fflush(stdout) == 0 ? 0 : 1;
}
