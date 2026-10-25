/* Generic solver adapter; all task algorithms belong to the worker's solve. */
#include "backend.h"
#include "api.h"
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>

#if TASK != 3
static void require(int condition, const char *message) {
    if (!condition) {
        fprintf(stderr, "experiment C backend: %s\n", message);
        exit(1);
    }
}
#endif

#if TASK == 2
typedef struct { Node *first, *second; size_t na, nb; } ListInputs;

static Node *input_nodes(const int64_t *values, size_t count) {
    if (count == 0) return NULL;
    Node *nodes = malloc(count * sizeof(*nodes));
    require(nodes != NULL, "cannot allocate input nodes");
    for (size_t i = 0; i < count; ++i) {
        nodes[i].value = values[i];
        nodes[i].next = i + 1 == count ? NULL : &nodes[i + 1];
    }
    return nodes;
}

static int within_input(const Node *node, const Node *base, size_t count) {
    if (base == NULL) return 0;
    /* Integer addresses avoid undefined ordering comparisons across allocations. */
    uintptr_t address = (uintptr_t)node, start = (uintptr_t)base;
    return address >= start && address - start < count * sizeof(*base);
}
#endif

void backend_prepare(Case *cases, size_t count) {
#if TASK == 2
    for (size_t i = 0; i < count; ++i) {
        ListInputs *state = malloc(sizeof(*state));
        require(state != NULL, "cannot allocate input state");
        state->first = input_nodes(cases[i].a, cases[i].na);
        state->second = input_nodes(cases[i].b, cases[i].nb);
        state->na = cases[i].na;
        state->nb = cases[i].nb;
        cases[i].state = state;
    }
#else
    (void)cases;
    (void)count;
#endif
}

void backend_solve(Case *input, Answer *output) {
    /* The fixed capture buffer is reused; do not clear 80 KiB per tiny result. */
    output->n = 0;
    output->scalar = 0.0;
    output->owned = NULL;
    output->ticks = 0;
#if TASK == 1
    Pair result = solve(input->a, input->na, input->target);
    require(result.first <= (size_t)INT64_MAX && result.second <= (size_t)INT64_MAX,
            "returned index is outside signed i64 range");
    output->n = 2;
    output->values[0] = (int64_t)result.first;
    output->values[1] = (int64_t)result.second;
#elif TASK == 2
    ListInputs *state = input->state;
    Node *head = solve(state->first, state->second);
    output->owned = head;
    for (Node *node = head; node != NULL; node = node->next) {
        require(output->n < ANSWER_CAPACITY, "returned list exceeds capture limit or contains a cycle");
        require(!within_input(node, state->first, state->na) && !within_input(node, state->second, state->nb),
                "returned list aliases input nodes");
        output->values[output->n++] = node->value;
    }
#elif TASK == 3
    output->scalar = solve(input->a, input->na, input->b, input->nb);
#endif
}

void backend_release(Answer *output) {
#if TASK == 2
    Node *node = output->owned;
    /* Capture established a bounded, nil-terminated chain before any free. */
    for (size_t i = 0; i < output->n; ++i) {
        require(node != NULL, "returned list changed before release");
        Node *next = node->next;
        free(node);
        node = next;
    }
    require(node == NULL, "returned list changed before release");
#endif
    output->owned = NULL;
}

void backend_finish(Case *cases, size_t count) {
#if TASK == 2
    for (size_t i = 0; i < count; ++i) {
        ListInputs *state = cases[i].state;
        free(state->first);
        free(state->second);
        free(state);
        cases[i].state = NULL;
    }
#else
    (void)cases;
    (void)count;
#endif
}
