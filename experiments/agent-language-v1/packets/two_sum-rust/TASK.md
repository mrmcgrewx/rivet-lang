# two_sum in rust

Given an integer array nums and integer target, return the two distinct indices whose values sum to target. Exactly one unordered pair exists. Either order is accepted. Do not reuse an element. Require a subquadratic search algorithm (expected O(n) or O(n log n), not a quadratic scan).
Constraints: 2 <= nums.length <= 10000; each value and target are in [-1000000000,1000000000].
Input: one count-prefixed array followed by target. Rivet must read_array once then read_int once, and return an IntArray with two indices.
Problem: https://leetcode.com/problems/two-sum/description/

## Public examples

- Input values: `{"a": [2, 7, 11, 15], "target": 9}`; result: `[0, 1]`.
- Input values: `{"a": [3, 2, 4], "target": 6}`; result: `[1, 2]`.
- Input values: `{"a": [3, 3], "target": 6}`; result: `[0, 1]`.

Write `solution.rs`. All valid inputs must work; do not hardcode examples. Caller handles input parsing and output for C/Rust. Rivet uses its generic read/return instructions. Domain constraints may be assumed; no extra domain-validation pass is required. Preserve language/runtime safety checks.

Submit using exactly:
```sh
python3 /Users/kylemcgrew/code/ai_language/experiments/agent-language-v1/grade.py submit /private/tmp/rivet-authoring-pilot-k2ijvu9_/two_sum-rust
```

The development suite has 10 cases and reports compiler diagnostics or the first failure. You cannot inspect its cases in advance. Stop on first pass or after 3 submissions. A separate withheld suite is run after your work is frozen; you cannot repair from its feedback.
