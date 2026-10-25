# Two Sum — Rivet

## Deliverables

- `solution.rvb`: RVB2 binary program, with 21 instructions and 13 registers.
- `write_solution.py`: retained byte writer; it only emits instruction records and resolves label PCs. It does not execute or test the algorithm.

## Algorithm

Read one array and one target integer. Scan the array from left to right while maintaining an IntMap from previously visited values to their indices. For each current value, look up `target - value`, using `-1` as the missing sentinel. If found, return an IntArray containing the stored index and current index. Otherwise, insert the current value and index and advance.

The lookup occurs before insertion, so the returned indices are distinct. Every stored index is nonnegative, so the missing sentinel is unambiguous. Replacing an index for a duplicate value preserves a valid earlier occurrence. When the scan reaches the later member of the guaranteed pair, its complement is already present and the program returns a valid pair. An exhaustion branch returns an empty array but is unreachable for inputs satisfying the stated guarantee.

Complement subtraction stays within `[-2000000000, 2000000000]` under the task constraints, well inside signed 64-bit range. All normal Rivet arithmetic, indexing, map, allocation, and fuel checks remain enabled.

## Complexity

Expected O(n) search time using the documented expected constant-time IntMap operations. The runtime does not guarantee constant-time worst-case probing. The algorithm stores at most n distinct entries; this Rivet runtime allocates its fixed 32768-slot map table up front. The result contains two indices, and the input array occupies O(n) storage. The source loop executes at most 10 instructions per unsuccessful element, plus fixed setup and return work, within the supplied fuel limit for n <= 10000.

## Formal submission result

One submission was made using exactly:

```sh
python3 /Users/kylemcgrew/code/ai_language/experiments/agent-language-v1/grade.py submit /private/tmp/rivet-authoring-pilot-k2ijvu9_/two_sum-rivet
```

Reported result: attempt 1 passed all 10 development cases, with `submissions_remaining: 0`. Work on the solver and byte writer stopped at that first development pass; neither was modified afterward. No other compilation, execution of the solver, or testing was performed. No hidden-test outcome is known or claimed.
