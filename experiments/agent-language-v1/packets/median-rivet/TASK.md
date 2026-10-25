# median in rivet

Given two nondecreasing sorted integer arrays, return the median of their combined values. For an even total length, average the two middle values. Require O(log(m+n)) search work (or better); input parsing/preparation is outside this search requirement. Do not merge or sort the arrays.
Constraints: 0 <= m,n <= 1000; 1 <= m+n <= 2000; each value is in [-1000000,1000000].
Input: two count-prefixed arrays. Rivet must read_array twice and return a Float.
Problem: https://leetcode.com/problems/median-of-two-sorted-arrays/description/

## Public examples

- Input values: `{"a": [1, 3], "b": [2]}`; result: `2.0`.
- Input values: `{"a": [1, 2], "b": [3, 4]}`; result: `2.5`.

Write `solution.rvb`. All valid inputs must work; do not hardcode examples. Caller handles input parsing and output for C/Rust. Rivet uses its generic read/return instructions. Domain constraints may be assumed; no extra domain-validation pass is required. Preserve language/runtime safety checks.

Submit using exactly:
```sh
python3 /Users/kylemcgrew/code/ai_language/experiments/agent-language-v1/grade.py submit /private/tmp/rivet-authoring-pilot-k2ijvu9_/median-rivet
```

The development suite has 10 cases and reports compiler diagnostics or the first failure. You cannot inspect its cases in advance. Stop on first pass or after 3 submissions. A separate withheld suite is run after your work is frozen; you cannot repair from its feedback.
