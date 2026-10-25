# Initial architecture

These are bootstrap decisions, not claims about an existing implementation. Commit the concrete language and module/API contracts before delegating implementation.

## Target

Start on macOS arm64 with Rust 1.90.0 and a browser UI served by the local executable. This target runs on the development machine without a VM, cross compiler, or native app distribution. Keep the core portable where practical; other platforms are unverified until tested.

## Proposed compiler and execution path

Use a small, versioned, typed program representation with an explicit validation stage and a distinct executable intermediate form. Rust owns parsing, validation/lowering, and execution. Define output and HTTP response operations as language/runtime capabilities. Include enough value composition to show computation and verify that program edits propagate through compilation to execution.

The final syntax, types, operations, and artifact format remain to be specified before implementation. Prefer explicit semantics and diagnostic locations over a broad syntax. Initially use a bytecode/intermediate-form interpreter; a native backend would add toolchain complexity before either required demonstration is complete.

## Workbench

Use a local browser workbench for program editing, diagnostics, build results, execution output, and HTTP lifecycle/request inspection. Serve its static assets from the Rust executable. Avoid a frontend build pipeline unless a concrete need justifies it. Keep the frontend independent of example behavior: all executed output and HTTP responses come from compiled programs.

The workbench service and program HTTP service should have separate lifecycles. Bind to loopback, report port conflicts clearly, and stop/join program-server workers deterministically. Document resource and protocol limits. Validate workbench requests and prevent cross-origin mutation of the local runtime.

## Dependencies and performance

Pin the Rust toolchain and commit dependency lockfiles. Choose minimal maintained parsing/serialization/networking dependencies during interface design and document their purpose. No dependencies have been selected yet.

For efficiency evaluation, first define a repeatable release-build benchmark on the documented target: compile a fixed program and execute its non-network computation repeatedly; separately measure local HTTP request latency. Record workload, iteration count, timing scope, and build mode. Initial measurements establish a baseline only, with no comparison to mature languages implied.
