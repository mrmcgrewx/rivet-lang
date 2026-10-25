# Add Two Numbers — C

## Implementation

`solution.c` traverses both supplied linked lists directly, adding one digit from each available list and the previous carry. It appends the sum modulo 10 to a newly allocated result node and carries the quotient by 10 to the next iteration. Traversal continues while either input has a node or a carry remains, including a final carry node when needed.

Every returned node is individually allocated with `malloc`. The inputs are neither modified nor aliased by the result. A pointer to the next output link permits constant-time append. Allocation failure frees the partial result and returns `NULL`.

For input lengths m and n, time is O(max(m, n)), hence O(m+n). Auxiliary space excluding the returned list is O(1); the output has at most max(m, n)+1 nodes. Digit sums remain between 0 and 19 under the supplied input constraints.

## Submission outcome

Used the exact formal submission command once. Attempt 1 reported `passed: true` and `development_cases_passed: 10`. No other compilation or execution was performed. Stopped after that first development pass, with no subsequent changes to `solution.c`.

The withheld suite was not accessed or run by this worker; no hidden-test result is claimed.
