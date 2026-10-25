/* Benchmark-only boundaries. No program operation or safety check is replaced. */
#include "backend.h"
#include "runtime.c"

typedef struct { void *a, *b; size_t charge; } InputState;
static rv_value *retained_inputs;
static Case *active_case;
static Answer *active_answer;
static unsigned reads, returns, cleanups;
extern int rv_agent_solve(void);

static _Noreturn void bad_boundary(void) {
    rv_error("invalid experiment boundary sequence");
}

void backend_prepare(Case *cases, size_t count) {
    if (rv_allocations != NULL || retained_inputs != NULL) bad_boundary();
    for (size_t i = 0; i < count; ++i) {
        InputState *state = calloc(1, sizeof(*state));
        if (state == NULL) rv_error("cannot allocate experiment input state");
        rv2_allocation_charge = 0;
#if TASK == 2
        for (size_t j = cases[i].na; j > 0; --j) state->a = rv2_list_cons(cases[i].a[j - 1], state->a);
        for (size_t j = cases[i].nb; j > 0; --j) state->b = rv2_list_cons(cases[i].b[j - 1], state->b);
#else
        state->a = rv2_array_make(cases[i].a, cases[i].na);
#if TASK == 3
        state->b = rv2_array_make(cases[i].b, cases[i].nb);
#endif
#endif
        state->charge = rv2_allocation_charge;
        cases[i].state = state;
    }
    /* Retain only immutable input values; query allocations use the real list. */
    retained_inputs = rv_allocations;
    rv_allocations = NULL;
    rv_cleanup();
}

void *bench_read_array(void) {
#if TASK == 1 || TASK == 3
    InputState *state = active_case->state;
    if (reads == 0) { ++reads; return state->a; }
#if TASK == 3
    if (reads == 1) { ++reads; return state->b; }
#endif
#endif
    bad_boundary();
}
void *bench_read_list(void) {
#if TASK == 2
    InputState *state = active_case->state;
    if (reads == 0) { ++reads; return state->a; }
    if (reads == 1) { ++reads; return state->b; }
#endif
    bad_boundary();
}
int64_t bench_read_int(void) {
#if TASK == 1
    if (reads == 1) { ++reads; return active_case->target; }
#endif
    bad_boundary();
}
static void returning(void) {
    if (reads != 2 || returns != 0 || cleanups != 0) bad_boundary();
    ++returns;
    active_answer->ticks = *rv2_fuel_counter();
}
void bench_return_array(void *array) {
    if (TASK != 1) bad_boundary();
    returning();
    int64_t length = rv2_array_len(array);
    if (length < 0 || length > ANSWER_CAPACITY) bad_boundary();
    active_answer->n = (size_t)length;
    for (size_t i = 0; i < active_answer->n; ++i) {
        active_answer->values[i] = rv2_array_get(array, (int64_t)i);
    }
}
void bench_return_list(void *list) {
    if (TASK != 2) bad_boundary();
    returning();
    active_answer->n = 0;
    while (list != NULL) {
        if (active_answer->n == ANSWER_CAPACITY) bad_boundary();
        active_answer->values[active_answer->n++] = rv2_list_value(list);
        list = rv2_list_next(list);
    }
}
void bench_return_float(double value) {
    if (TASK != 3) bad_boundary();
    returning();
    if (!isfinite(value)) bad_boundary();
    active_answer->scalar = value;
}
void bench_cleanup(void) {
    if (returns != 1 || cleanups != 0) bad_boundary();
    ++cleanups;
    rv_cleanup();
}
void backend_solve(Case *input, Answer *output) {
    if (rv_allocations != NULL || *rv2_fuel_counter() != 0) bad_boundary();
    active_case = input;
    active_answer = output;
    output->n = 0;
    output->scalar = 0;
    output->owned = NULL;
    output->ticks = 0;
    reads = returns = cleanups = 0;
    rv2_allocation_charge = ((InputState *)input->state)->charge;
    if (rv_agent_solve() != 0 || reads != 2 || returns != 1 || cleanups != 1) bad_boundary();
    if (rv_allocations != NULL || rv2_allocation_charge != 0 || *rv2_fuel_counter() != 0) bad_boundary();
}
void backend_release(Answer *output) { (void)output; }
void backend_finish(Case *cases, size_t count) {
    if (rv_allocations != NULL) bad_boundary();
    rv_allocations = retained_inputs;
    retained_inputs = NULL;
    rv_cleanup();
    for (size_t i = 0; i < count; ++i) { free(cases[i].state); cases[i].state = NULL; }
}
