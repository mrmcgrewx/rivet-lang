# Two Sum in C

Implemented `solution.c` using a dynamically allocated, open-addressed hash table with linear probing. Each iteration searches for `target - values[i]` among previously inserted values, then inserts the current value if no pair was found. Checking before insertion ensures distinct indices and handles equal-valued pairs. Entries retain an original index, with zero occupancy encoded by an index-plus-one field. The table is freed before every return after successful allocation.

The power-of-two table has capacity at least twice the input length. With ordinary hash distribution, lookup and insertion take expected constant time, yielding expected O(n) total time and O(n) auxiliary space. Pathological collisions can produce O(n²) time. The hash uses defined unsigned arithmetic; complement subtraction fits signed 64-bit arithmetic under the supplied constraints. Allocation failure returns the invalid sentinel pair `{SIZE_MAX, SIZE_MAX}` without dereferencing a null pointer.

Formal submission 1 passed all 10 development cases. No repairs or additional submissions were made. The solver was frozen immediately after that pass. All compilation and execution used the exact submission command from `TASK.md`. No withheld-test results are known or claimed.
