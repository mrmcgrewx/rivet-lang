# Median C handoff

Implemented `solution.c` against the supplied `api.h` interface.

The solver first makes the shorter array the binary-search array. It fixes
the number of elements on the combined left side to `(m + n + 1) / 2`,
then searches for the smallest first-array cut whose next element is at
least the other array's last left-side element. Sorted inputs and this
minimal cut ensure that neither left boundary exceeds the opposite right
boundary. The maximum left boundary is the median for an odd total; for an
even total, the solver averages it with the minimum right boundary.

Empty partition sides use integer extrema as sentinels. Conditional
expressions avoid indexing those sides, including when an entire input
array is empty. The binary-search midpoint remains strictly below the
shorter array's length, and its complementary cut is within the longer
array and positive, so both search accesses are in bounds. Inputs are not
modified, merged, or sorted.

Search time is O(log(min(m, n) + 2)), including constant work for an empty
shorter array; auxiliary space is O(1). Converting the middle values to
double before adding avoids integer addition overflow. Under the supplied
value constraints, both integral and half-integral results are exact.

## Submission result

Ran exactly one formal submission:

```sh
python3 /Users/kylemcgrew/code/ai_language/experiments/agent-language-v1/grade.py submit /private/tmp/rivet-authoring-pilot-k2ijvu9_/median-c
```

The command reported attempt 1, `passed: true`, and 10 development cases
passed. No compilation or execution was performed outside that command.
The solver was frozen immediately after the first development pass and
has not been edited afterward. No withheld-suite result is available to
this worker.
