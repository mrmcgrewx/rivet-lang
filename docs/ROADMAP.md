# Rivet capability and performance direction

The user's goal is a practical language that AI agents can use in place of conventional human-oriented languages, with comparable or better performance on relevant workloads. The authored source remains an actual binary file with a human inspection GUI. That goal guides feature choices; it is not a claim that the current prototype already replaces Python, Java or JavaScript.

## How exercises grow the language

Each exercise should introduce reusable primitives, then test those primitives independently. An example algorithm belongs in binary source and generated native operations; adding a specialized host-language solver does not advance the language. Preserve version compatibility, exact source roundtrips, source/data separation, reference/native agreement and standalone execution as capabilities grow.

| Stage | Capability | Evidence / next gate |
| --- | --- | --- |
| Native binary foundation | Actual binary source, typed validation, inspector, native executable, console/HTTP support | RVB1 validation |
| Runtime computation | Mutable registers, branches/loops, arrays/maps, stdin and bounded execution | Two Sum plus generic compute tests |
| Linked data | Immutable linked nodes, nullable handles, copying, guarded division/remainder, assertions | Add Two Numbers plus generic linked-data tests |
| Numeric search and comparison | Finite binary64, scalar output, speed profile, partition search | Median plus generic Float tests and scoped C/Python runtime/RSS measurements |
| Program composition | Typed functions, calls/returns, modules, reusable libraries and stable interfaces | Multiple programs sharing one library, recursion/lifetime/error design, separate compilation where justified |
| Practical applications | Broader data types, structured errors and controlled file/network capabilities | Representative agent tasks beyond interview exercises; capability and lifecycle tests |
| Agent authoring tools | Schema-aware binary construction/patching, stable diagnostics, instruction-aware diffs and roundtrip editing | Measure task correctness, edit reliability, repair rate and authoring cost against textual-language baselines |
| Performance engineering | Representative throughput/latency/memory/size/build-time benchmark suite | Reproducible comparisons with equivalent algorithms and established compilers/runtimes |

Future stages are direction, not scheduled or completed work. Functions and modules are a particularly important next step toward reuse; a collection of standalone algorithm examples is not yet a practical application ecosystem.

## Performance standard

Binary source may improve compactness and mechanical editing; it does not inherently speed up the resulting program. Execution cost comes from emitted native code, data structures, allocation, runtime checks and algorithms. LLVM/Clang is the current machine-code backend. Keep safety checks and observable semantics explicit when optimizing.

The original optimization experiment targeted executable size with `-Oz` versus `-O0`; its historical measurements do not establish execution-speed competitiveness. The median milestone begins runtime/peak-RSS comparisons with C and Python under an explicit method; it is one workload, not broad competitiveness. The longer-term standard includes runtime throughput/latency and peak memory as well as compilation time and artifact size. Use equivalent algorithms, input distributions, correctness criteria, integer semantics and workload sizes. Separate startup/IO from steady-state computation, report distributions and environment, and identify warmup/JIT/GC or allocation effects. Preserve slower results as well as wins.

Do not generalize from Hello World or a short LeetCode input to whole-language superiority. Native/reference differential tests establish correctness, not performance. New optimizations should have a concrete target, a straightforward baseline and a repeatable measurement, followed by semantic regression checks. Agent-generation benefits need their own measured authoring experiment; they are separate from runtime speed.
