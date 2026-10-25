# Rivet: project retrospective

**Concluded: October 25, 2026.** Rivet is preserved as a completed learning experiment. Further language development and comparative research are out of scope unless explicitly reopened. The final implementation and experiment report were published at `9591b34`; this retrospective records the decision to stop.

## The question we started with

Could an AI author programs more effectively in a representation designed for machines, without the conventions that make source code readable to humans? Could those programs also execute as efficiently as programs written in established languages?

The concrete vision was binary source that an agent could construct, a GUI that could decode it for a person, and real executable programs. It was a useful question to investigate. We initially blurred several different goals: compact source, convenient agent authorship, reliable code generation, and fast execution. The project became more informative as we measured those goals separately.

## What we built

Rivet ended with an actual binary source format, a decoder and typed validator, a reference interpreter, a native compiler path, and a browser workbench. The workbench can open the binary, show decoded instructions or hex bytes, build a native executable, and launch it. The resulting program can run independently of the workbench.

The language grew from Hello World and a local HTTP server to loops, arrays, maps, immutable linked lists, checked integer arithmetic and finite floating-point values. Two Sum, Add Two Numbers and Median of Two Sorted Arrays are programs expressed in Rivet operations. Their algorithms are not hidden in problem-specific host-language helpers.

The native path is:

```text
RVB source bytes
  → decoding and validation
  → typed program representation
  → generated LLVM instructions
  → Clang optimization and native linking
  → executable loaded by the operating system
  → CPU instructions, with calls to runtime and system support
```

Rust implements the compiler and workbench; generic C helpers supply storage, input/output and other runtime services. LLVM/Clang supplies mature native code generation. Reusing those components made the experiment practical. It did not give the new source format an automatic execution advantage.

## Where we missed the mark

The early implementation treated a JSON representation, interpreter and GUI as sufficient progress toward the requested language. Native compilation was initially left as future work. Even after adding native compilation, JSON remained the authored source until the binary-source requirement was corrected explicitly.

Those were implementation and acceptance-definition failures. The agent should have resolved the distinction between source encoding, program semantics and execution artifacts before declaring the project complete. The user had to restate the intended outcome several times; better prompting should not have been the only mechanism for catching the mismatch.

A small acceptance example would have helped immediately: an agent writes a binary source file; the GUI decodes it; Build produces a standalone executable; that executable still runs after the GUI closes. We should then have separated that demonstrable capability from the hypothesis that it would outperform existing approaches.

The interpreter and validator remained useful after the correction. They became a reference for checking native results. The recovery worked because we retained useful components while changing the definition of success.

## What the evidence says

| Question | Observed result | What it supports |
| --- | --- | --- |
| Can an independent agent author valid Rivet binary programs? | All three Rivet workers passed on their first submission and passed 878 withheld cases in total. C and Rust did the same. | Feasibility for these three small problems; no measured correctness or retry advantage. |
| Did the binary workflow reduce authoring output? | Across the three tasks, Rivet used 9,688 output tokens, C 5,810 and Rust 4,086. | No output-token advantage in this workflow. Rivet's specification reading, byte writers and handoffs were part of the work. |
| Did the new language consistently execute faster? | C led large Two Sum and list addition; Rust led balanced median. Rivet beat Rust on large Two Sum. | Performance depends on the submitted algorithm, data structures, runtime and compiler decisions; no universal winner. |
| Could we improve Rivet without weakening its checks? | In a separate unchanged median benchmark, exposing the fuel check to LLVM reduced time from 559.45 to 104.14 ns/query, a 5.37× improvement. | A concrete compiler/runtime change mattered far more than source appearance for that workload. |

The [authoring pilot](AGENT-EXPERIMENT-RESULTS.md) used nine fresh agents with the same model and reasoning setting, one per language/problem pair. All nine passed the first development submission; no repair attempts were needed. Seven timing samples per workload do not turn one authored solution into seven independent authoring trials.

The [native optimization comparison](NATIVE-OPTIMIZATION-RESULTS.md) kept its program, inputs, controls and safety semantics unchanged. It is a different experiment from the independent-agent comparison. Their timing ratios must not be combined into one progression.

Memory was measured as each child process's peak resident set, including setup and runtime overhead, not just algorithm storage. Input-token counters included cached input, and cache residency differed across workers. The published reports preserve those distinctions, raw results, unsuccessful infrastructure probes and limitations. These experiments do not establish a general language ranking, a billing comparison or a reliable success rate on unfamiliar tasks.

## The central technical lesson

**Binary source is a representation choice, not a shortcut around language implementation or machine execution.**

RVB bytes are not ARM64 machine instructions. Their meaning still has to be defined, validated and lowered into operations the target can execute. A decoder can make that meaning readable to a human, so the format is not intrinsically exclusive to AI. Nor does a smaller source file imply fewer model tokens: our agents used textual instructions and encoding scripts to construct the binary files.

Similar execution paths do not make all languages equivalent. Semantics, type information, ownership rules, data layout and runtime design can change both the work a program performs and the optimizations a compiler can safely make. The practical lesson is that removing human-readable syntax alone does not supply those benefits. Established compilers and runtimes also embody substantial work on optimization, platforms, libraries, debugging and compatibility.

Rivet provided concrete examples. Its fixed-size map incurred substantial setup cost on tiny Two Sum inputs. Its immutable prepend-only lists led the addition program to construct a temporary reversed chain and then a second result chain. Its original per-instruction fuel calls impeded optimization until we exposed the same checks more effectively to the backend. These are specific design and implementation costs, rather than consequences of whether the source looks like text or bytes.

## What remains plausible, but unproven

A better programming interface for agents does not necessarily require discovering new algorithms, inventing a CPU architecture or replacing established languages. Typed construction tools, edits to program structure, precise diagnostics and reliable validation could be layered over an existing language or compiler. They might improve edit reliability or reduce repair work while retaining existing libraries and machine-code generation.

That is a separate hypothesis. Rivet did not compare those interfaces, retrain a model for its representation, or demonstrate that a custom binary format is the best way to provide them. A meaningful advantage would require a defined workload, a specific measure of success and evidence against a strong existing baseline.

## What was worth keeping

The most useful outcome is a small system whose execution path can be inspected end to end. Source bytes, decoded operations, generated LLVM, native assembly and runtime support are all available. Independent implementations and measured results helped replace intuitions with testable explanations.

The engineering evidence is also reusable: reference/native comparisons, generic feature tests, immutable experiment inputs, actual token counters, independent review and clean-checkout reproduction. At the final experiment milestone, all 100 Rust tests and 18 browser scenarios passed. The [validation record](AGENT-EXPERIMENT-VALIDATION.md) links those checks, the separate C sanitizer evidence and the workbench proof.

For studying execution, this repository can serve as a small worked example:

| Layer to inspect | Starting point | Question to follow |
| --- | --- | --- |
| Source encoding | [RVB2 specification](COMPUTE-FORMAT.md) | How do bytes describe types, operands and branches? |
| Validation | [Computation decoder and validator](../src/language/compute.rs) | What can be checked before a program runs? |
| Lowering | [LLVM generation](../src/language/compute_codegen.rs) | Which source operations become arithmetic, branches or helper calls? |
| Native build | [Compiler driver](../src/native.rs), generated `.ll` and `.s` artifacts | What does the backend receive, and what instructions does it produce? |
| Runtime support | [Runtime implementation](../native/runtime.c) | Where do allocation, bounds checks, input/output and cleanup happen? |
| Measurement | [Optimization evidence](NATIVE-OPTIMIZATION-RESULTS.md) | Which costs were observed, and what changed when we optimized them? |

This is a reading path through completed work, not a new development plan.

## Closing decision

We are ending active development here. The prototype, examples, workbench, reports and history remain available for study and sharing. The earlier roadmap is retained as historical context; its unimplemented items are not current commitments. No further optimization, language expansion or agent study is planned.

The next step is learning more about compilers, runtimes and machine execution at the user's own pace. Rivet gave us a concrete reason to study those layers: we built enough of the system to see where representation ends and execution cost begins. The experiment succeeded as a way to test an idea and refine our understanding, while leaving its hoped-for AI-specific advantages unproven.
