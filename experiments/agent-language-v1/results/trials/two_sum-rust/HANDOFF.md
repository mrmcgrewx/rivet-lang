# Two Sum — Rust handoff

## Implementation

`solution.rs` exports `pub fn solve(nums: &[i64], target: i64) -> (usize, usize)` using only safe Rust and the standard library.

The solver scans the input once and stores previously visited values and their indices in a `HashMap`. Before inserting the current value, it looks up `target - value`. A match returns the stored index and the current index. Lookup before insertion guarantees distinct indices and handles equal-valued pairs. The difference fits in `i64` under the supplied input constraints. The final panic is unreachable for the promised valid inputs containing a pair.

Expected time complexity is O(n); auxiliary space is O(n). Hash-map operations provide expected rather than worst-case constant time.

## Formal submission outcome

Exactly one submission was made using the required command:

```sh
python3 /Users/kylemcgrew/code/ai_language/experiments/agent-language-v1/grade.py submit /private/tmp/rivet-authoring-pilot-k2ijvu9_/two_sum-rust
```

The command exited successfully and reported attempt 1 passed, with all 10 development cases passing. No solver edits or further submissions occurred after that result. No separate compilation or testing was performed. Withheld-suite results are unknown.
