# Median of Two Sorted Arrays in binary Rivet

[`median-two-sorted-arrays.rvb`](../examples/median-two-sorted-arrays.rvb) solves [LeetCode problem 4](https://leetcode.com/problems/median-of-two-sorted-arrays/description/) using 803 source bytes, 81 instructions and 32 typed registers. The source contains binary partition search, comparisons, guarded array loads and floating-point mean calculation. Generic runtime support supplies array storage, input/output and resource checks; it has no median solver.

```sh
cargo build --release --locked
./target/release/rivet build examples/median-two-sorted-arrays.rvb -o ./median --profile speed
printf '2 1 3 1 2\n' | ./median
# 2.0000000000000000e+00
printf '2 1 2 2 3 4\n' | ./median
# 2.5000000000000000e+00
printf '0 2 -2 1\n' | ./median
# -5.0000000000000000e-01
```

Each array is its count followed by its elements. Runtime input changes do not change the source or require a rebuild. In the workbench, choose **Median of Two Sorted Arrays**, Build, then edit Runtime input and Run. Both decimal integers and half-integers are exact binary64 values in the problem's range. Scientific notation preserves 17 significant digits; `2.5000000000000000e+00` means 2.5.

## Search

1. Read both arrays and enforce length at most 1000 each and at least one element overall.
2. Swap handles if needed so the first array is shorter; no elements are copied.
3. Binary-search a cut `i` in the first array. Set the other cut to `j = (m+n+1)/2 - i`, so the left partition contains the extra element when the total is odd.
4. Read the elements adjacent to both cuts. Guard boundary loads; signed i64 minimum/maximum stand in for absent lower/upper edges.
5. If the first left edge exceeds the second right edge, move the cut left. If the second left edge exceeds the first right edge, move it right. Otherwise all left-partition elements are at most all right-partition elements.
6. For odd total length, return the maximum left edge. For even length, return the mean of the maximum left and minimum right edges, converting both to Float before adding.

The arrays must already be sorted and values must obey the problem's +/-1000000 bound. These are caller preconditions; the program does not scan to validate them. Both-empty, oversized lengths, malformed tokens, missing values and extra input fail. Some invalid unsorted inputs may fail partition search, but rejection of all unsorted inputs is not promised.

The search takes **O(log(min(m,n)+1)) time and O(1) auxiliary storage**; it handles an empty shorter array in one partition. The standalone program must read and store its input, adding O(m+n) parsing time and storage. Neither the mathematical search bound nor peak process RSS should be confused with those end-to-end costs.

## Reusable additions and measurements

RVB2 now has finite IEEE-754 binary64 values, bit-preserving literals/copies, explicit integer conversion, checked float addition/division and scalar output. Native arithmetic uses direct LLVM operations without fast-math; NaN/infinity literals, nonfinite results and either signed-zero divisor are rejected. Subnormals and signed zero are preserved. The [format specification](COMPUTE-FORMAT.md) defines exact bytes and semantics.

The `speed` build profile uses Clang -O3 and linker dead stripping, preserving runtime checks. Default `size` remains -Oz; `baseline` remains -O0. The [benchmark method](MEDIAN-BENCHMARK-METHOD.md) compares this source with the same search in C and Python. Separate process and preloaded measurements distinguish startup/input costs from repeated search. Results are specific to this implementation, workload and host, not whole-language superiority. The broader [roadmap](ROADMAP.md) still includes functions, modules, practical IO and measured agent-authoring tools.
