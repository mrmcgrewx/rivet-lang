# Project brief

**Canonical source requirement:** The canonical authored program must be an actual `.rvb` binary file. JSON-source completion is superseded. See `BINARY-PLAN.md` for the current encoding, interfaces, ownership, and acceptance. Native backend/runtime remain foundations.

## Objective

Create an experimental custom programming language or machine-oriented program representation, its compiler/runtime, and a UI that a software engineer can use. Complete, test, and publish both MVPs to a new public GitHub repository under `mrmcgrewx`.

The implementation language and execution target are engineering choices. Python, unsigned-integer processing, and x86-64 Linux are not requirements. A practical locally runnable target takes priority.

## Required native-artifact correction

Native compilation is part of completion. Both demonstrations must build standalone native executables for Apple Silicon; workbench Run launches those artifacts, and they remain usable after closing the workbench. Established backends/linkers and small native runtime helpers are permitted. Document representation rationale, generated-code/runtime boundary, and a concrete efficiency objective measured against a semantics-equivalent baseline. Preserve the validator/interpreter as a correctness reference. The native backend contract is `NATIVE-PLAN.md`; binary source is specified in `BINARY-PLAN.md` and `BINARY-FORMAT.md`.

## Language and runtime

The algorithm exercises are [Two Sum](https://leetcode.com/problems/two-sum/description/), [Add Two Numbers](https://leetcode.com/problems/add-two-numbers/description/), and [Median of Two Sorted Arrays](https://leetcode.com/problems/median-of-two-sorted-arrays/description/). `TWO-SUM-PLAN.md` defines RVB2 runtime input, mutable registers, arrays/maps and control flow; `ADD-TWO-NUMBERS-PLAN.md` adds real linked nodes, copying, checked division/remainder and assertions. `MEDIAN-PLAN.md` adds fractional results, logarithmic partition search and reproducible runtime/memory comparisons with C and Python. Algorithms remain binary source compiled natively. Preserve existing files/semantics, distinguish source from runtime data in the GUI, and reuse a build across inputs.

The user wants these tasks to develop a language AI agents can use in place of established languages with comparable or better performance. `ROADMAP.md` makes that direction durable: prioritize reusable capabilities and independent tests, then functions/modules/libraries and practical agent tooling. Runtime-speed and agent-authoring benefits require representative comparative measurements; binary source and small examples alone are not sufficient evidence.

- Author actual `.rvb` binary source files containing numeric opcodes, types, register references and payloads. JSON, textual assembly, and renamed/embedded text are not the canonical source.
- Provide a GUI decoder and byte-preserving editor for human inspection; the compiler consumes binary source directly.
- Define small, precise, extensible semantics sufficient to test program behavior and compiler correctness.
- Use Rust, C++, or another suitable implementation language; justify backend/library choices.
- Both demonstrations must execute custom-language programs. Ordinary runtime output, networking, and OS support are permitted.
- Internal readability is optional; the UI and history must be understandable to an engineer.
- Treat efficiency as an engineering objective. Define the target and measurements before making performance claims. Complete both MVPs before broad optimization work.

## MVP 1: Hello world

The UI must support opening or creating the example, inspecting and modifying the program or relevant inputs, building and running it, and displaying diagnostics, output, and execution status.

Acceptance:

1. A custom-language program produces `Hello, world!` through the UI.
2. Editing the program changes its executed output.
3. Invalid programs fail with useful diagnostics.
4. Automated tests cover the essential compiler/runtime behavior, and actual UI interaction verifies the path.

## MVP 2: Local HTTP server

The custom-language program must define a local HTTP service, including its endpoint and response. A documented endpoint such as `GET http://127.0.0.1:<port>/hello` returns status 200, an appropriate JSON content type, and a payload such as `{"message":"hello world"}`.

Acceptance:

1. The UI builds, starts, and stops the server, showing its address and useful diagnostics.
2. The UI sends an example request and displays its response.
3. Program changes affect the response.
4. Automated tests verify repeated requests and clean shutdown.
5. The server binds locally by default.
6. Actual UI interaction verifies the full path.

## Reproducibility and completion

- Record dependency versions and exact build, test, and launch commands.
- Verify instructions from a clean checkout or equivalent clean environment.
- Keep requirements, decisions, examples, implementation, meaningful experiments, rejected approaches that affect decisions, validation results, limitations, milestone status, and continuation context in the repository and its history.
- Publish small, understandable commits and push meaningful checkpoints throughout the work.
- Use commits for tracking; no GitHub Projects board or elaborate management system.
- Keep only project materials in the public repository. Never commit credentials or private global instructions.
- When useful, delegate with explicit ownership and committed interfaces; persist handoffs before agents finish and verify the integrated result.
- Work autonomously through completion, pausing only for an unresolved access, permission, or environment blocker.

Final delivery must include the public repository URL, final and milestone commit references, exact local setup and launch instructions, evidence for both MVPs, the implemented architecture, and material limitations or remaining work.
