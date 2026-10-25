#ifndef AGENT_SOLVER_API_H
#define AGENT_SOLVER_API_H
#include <stddef.h>
#include <stdint.h>
typedef struct { size_t first, second; } Pair;
typedef struct Node { int64_t value; struct Node *next; } Node;
#if TASK == 1
Pair solve(const int64_t *values, size_t count, int64_t target);
#elif TASK == 2
Node *solve(const Node *first, const Node *second);
#elif TASK == 3
double solve(const int64_t *first, size_t first_count, const int64_t *second, size_t second_count);
#else
#error Invalid TASK
#endif
#endif
