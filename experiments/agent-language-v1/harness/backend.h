#ifndef AGENT_BENCH_BACKEND_H
#define AGENT_BENCH_BACKEND_H
#include <stddef.h>
#include <stdint.h>
#define ANSWER_CAPACITY 10002
typedef struct {
    size_t na, nb;
    int64_t *a, *b;
    int64_t target;
    size_t expected_n;
    int64_t *expected;
    void *state;
} Case;
typedef struct {
    size_t n;
    int64_t values[ANSWER_CAPACITY];
    double scalar;
    void *owned;
    uint64_t ticks;
} Answer;
void backend_prepare(Case *cases, size_t count);
void backend_solve(Case *input, Answer *output);
void backend_release(Answer *output);
void backend_finish(Case *cases, size_t count);
#endif
