# add_two_numbers in c

Two nonempty singly linked lists represent nonnegative integers with decimal digits in reverse order. Add them and return the result as a singly linked list in the same order. Use actual list traversal and result nodes; do not convert entire numbers to scalar/string/big-integer representations or solve on array substitutes. Require O(m+n) work.
Constraints: each list has 1..100 nodes; values are 0..9; neither number has a leading zero except the number zero (the final node is nonzero when length > 1).
Input: two count-prefixed lists. Rivet must read_list twice and return an IntList. C must return individually malloc-allocated nodes owned by the caller, without input aliases. Rust uses the supplied Box<Node> list type.
Problem: https://leetcode.com/problems/add-two-numbers/description/

## Public examples

- Input values: `{"a": [2, 4, 3], "b": [5, 6, 4]}`; result: `[7, 0, 8]`.
- Input values: `{"a": [0], "b": [0]}`; result: `[0]`.
- Input values: `{"a": [9, 9, 9, 9, 9, 9, 9], "b": [9, 9, 9, 9]}`; result: `[8, 9, 9, 9, 0, 0, 0, 1]`.

Write `solution.c`. All valid inputs must work; do not hardcode examples. Caller handles input parsing and output for C/Rust. Rivet uses its generic read/return instructions. Domain constraints may be assumed; no extra domain-validation pass is required. Preserve language/runtime safety checks.

Submit using exactly:
```sh
python3 /Users/kylemcgrew/code/ai_language/experiments/agent-language-v1/grade.py submit /private/tmp/rivet-authoring-pilot-k2ijvu9_/add_two_numbers-c
```

The development suite has 10 cases and reports compiler diagnostics or the first failure. You cannot inspect its cases in advance. Stop on first pass or after 3 submissions. A separate withheld suite is run after your work is frozen; you cannot repair from its feedback.
