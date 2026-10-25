#include "api.h"

#include <stdlib.h>

Node *solve(const Node *first, const Node *second)
{
    Node *result = NULL;
    Node **tail = &result;
    int64_t carry = 0;

    while (first != NULL || second != NULL || carry != 0) {
        int64_t sum = carry;
        if (first != NULL) {
            sum += first->value;
            first = first->next;
        }
        if (second != NULL) {
            sum += second->value;
            second = second->next;
        }

        Node *node = malloc(sizeof *node);
        if (node == NULL) {
            while (result != NULL) {
                Node *next = result->next;
                free(result);
                result = next;
            }
            return NULL;
        }

        node->value = sum % 10;
        node->next = NULL;
        *tail = node;
        tail = &node->next;
        carry = sum / 10;
    }

    return result;
}
