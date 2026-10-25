# Add Two Numbers — Rivet handoff

## Delivered files

- `solution.rvb`: submitted RVB2 binary program.
- `write_solution.py`: retained source authoring tool; constructs numeric binary records and resolves instruction labels only.

## Algorithm and representation

Read two actual singly linked lists with exactly two `read_list` instructions. Traverse their nodes together, treating an exhausted input as contributing zero. Each iteration adds the available head digits and the previous carry, computes the output digit with remainder by 10, and computes the next carry with division by 10. Continue until both inputs and the carry are exhausted.

Because list nodes are immutable and construction prepends, first accumulate digits in a reversed list. Traverse that temporary list to construct a second list in the required least-significant-digit-first order, then return it with the list-return instruction. Both temporary and returned nodes are newly constructed. The program does not modify input nodes, use array substitutes, or convert either whole number into a scalar or string. All arithmetic, list safety, fuel, and allocation checks remain enabled.

For input lengths m and n and result length k <= max(m, n) + 1, time is O(m+n). Additional node storage is O(k), consisting of k temporary nodes and k returned nodes, with O(1) scalar/register state. The runtime retains allocations until cleanup.

## Formal submission outcome

Only one formal submission was made:

```sh
python3 /Users/kylemcgrew/code/ai_language/experiments/agent-language-v1/grade.py submit /private/tmp/rivet-authoring-pilot-k2ijvu9_/add_two_numbers-rivet
```

The command reported attempt 1 passed with all 10 development cases passing. No solver or byte-writer changes were made afterward. No compilation, algorithm execution, or testing was performed outside the formal submission command; Python was used only to construct the binary source. Withheld tests were not accessed or run by this worker, and their outcome is unknown.
