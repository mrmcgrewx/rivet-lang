# Rivet median handoff

## Artifacts

- `solution.rvb`: submitted binary RVB2 computation source.
- `solution.py`: retained byte writer; constructs numeric instruction records and resolves branch labels only. It does not execute or test the algorithm.

## Algorithm

Read the two arrays with two `read_array` instructions. Let `half = (m+n+1)/2`. Binary-search the first array's partition index `i` within `[max(0, half-n), min(m, half)]`, with the second array's partition index `j = half-i`.

At each partition, obtain the neighboring left and right values. Missing sides use `-1000001` or `1000001`, strictly outside the supplied input domain. If the first left value exceeds the second right value, decrease the upper search bound. If the second left value exceeds the first right value, increase the lower bound. Otherwise the partition is valid. Return the largest left value for odd total length, or the average of the largest left and smallest right values for even total length, using a Float result.

The search bounds ensure both partition indices stay within their array lengths. Conditional branches guard boundary accesses. Arithmetic and array operations retain RVB2 runtime checks. Inputs are not merged or sorted.

## Complexity

Search performs O(log(min(m,n)+2)) iterations with O(1) work per iteration and O(1) auxiliary storage. Input parsing and array storage require O(m+n) time and storage, outside the requested search-work bound.

## Submission outcome

Ran the exact formal submission command once. Attempt 1 passed all 10 development cases. The submission tool reported `passed: true` and `submissions_remaining: 0`.

Stopped after that first pass; neither the binary source nor byte writer was changed afterward. No independent compilation, execution, or tests were run. No withheld tests were inspected or run by this worker; their outcome is not claimed here.
