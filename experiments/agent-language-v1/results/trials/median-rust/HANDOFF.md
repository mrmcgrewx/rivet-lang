# Median in Rust

Implemented `pub fn solve(first: &[i64], second: &[i64]) -> f64` in `solution.rs` using safe Rust and the standard library.

The algorithm binary-searches a partition in the shorter array. The complementary partition in the longer array makes the combined left side contain `ceil((m + n) / 2)` elements. Comparing the values bordering both partitions determines which direction to search. When every left value is at most every right value, an odd-length median is the maximum left value; an even-length median averages that value and the minimum right value. Empty partitions are handled explicitly, with normal Rust bounds and overflow checks preserved. The algorithm assumes the supplied sorted-input and nonempty-total constraints, and neither merges nor sorts inputs.

Search time is `O(log(min(m, n) + 2))`, including the constant-time empty-array case, and auxiliary space is `O(1)`. Inputs are borrowed without copying or modification.

## Submission outcome

Exactly one formal submission was made using the command in `TASK.md`. Attempt 1 passed all 10 development cases. No separate compilation, execution, or tests were run. Work stopped at that first development pass, and the solver was not edited afterward. Withheld-suite results are not known or claimed.
