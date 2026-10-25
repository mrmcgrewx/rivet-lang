# Rivet capability and performance direction

**Historical roadmap — active development concluded October 25, 2026.** The user chose to close the experiment and study machine execution before considering further research. The ideas below are retained as context, not scheduled work or instructions to continue. See the [retrospective and closing decision](POSTMORTEM.md).

The original goal was a practical language that AI agents could use in place of conventional human-oriented languages, with comparable or better performance on relevant workloads. The authored source remains an actual binary file with a human inspection GUI. That goal guided feature choices; the prototype did not establish that it could replace Python, Java or JavaScript.

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

The first native optimization makes existing fuel checks visible to LLVM without changing source or limits. The unchanged median comparison measured a 5.37× balanced-family improvement and a remaining 3.02× C advantage; all eight families improved. [Before/after evidence](NATIVE-OPTIMIZATION-RESULTS.md) also records process latency, peak RSS and compilation/file-size costs. Checked array helpers are a measured next candidate. Preserve their failure semantics and measure generic improvements before claiming broader competitiveness.

The original optimization experiment targeted executable size with `-Oz` versus `-O0`; its historical measurements do not establish execution-speed competitiveness. The median milestone begins runtime/peak-RSS comparisons with C and Python under an explicit method; it is one workload, not broad competitiveness. The longer-term standard includes runtime throughput/latency and peak memory as well as compilation time and artifact size. Use equivalent algorithms, input distributions, correctness criteria, integer semantics and workload sizes. Separate startup/IO from steady-state computation, report distributions and environment, and identify warmup/JIT/GC or allocation effects. Preserve slower results as well as wins.

Do not generalize from Hello World or a short LeetCode input to whole-language superiority. Native/reference differential tests establish correctness, not performance. New optimizations should have a concrete target, a straightforward baseline and a repeatable measurement, followed by semantic regression checks. Agent-generation benefits need their own measured authoring experiment; they are separate from runtime speed.

The first [same-model authoring pilot](AGENT-EXPERIMENT-RESULTS.md) is complete: nine fresh agents, three problems and three languages. All passed first submission and 878 holdouts per language; Rivet showed no correctness/retry advantage and used more output tokens than C/Rust. C led two large workloads, Rust led balanced median, and Rivet beat Rust on large Two Sum. Fixed map setup and immutable-list temporary allocation are concrete costs to investigate. This is one authoring sample per cell; future studies should add unfamiliar variants and repeated agents, separating practical agent choices from a matched-algorithm compiler/runtime comparison. Frozen sources and raw evidence must remain unchanged when testing improvements.
