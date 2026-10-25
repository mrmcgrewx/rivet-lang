# Project instructions

Read `docs/BRIEF.md`, `docs/ARCHITECTURE.md`, `docs/NATIVE-PLAN.md`, `docs/BINARY-PLAN.md`, `docs/TWO-SUM-PLAN.md`, and `docs/STATE.md` before starting work. Follow the machine's applicable GitHub identity instructions; keep credentials and private machine instructions outside this repository.

- Git commits are the work ledger. Do not create a Projects board or additional tracking system.
- Commit the brief and initial state and push them before substantive implementation.
- Use coherent commits with bodies explaining what changed, why, and how it was checked. Push meaningful checkpoints.
- Update `docs/STATE.md` with milestone status, validation results, remaining work, and limitations at meaningful handoffs.
- Both demonstrations must execute programs in the custom representation. Never hardcode demonstration behavior in the UI.
- Before delegation, commit task ownership, interfaces, and assumptions. Delegates must read this file and the committed state, stay within their assigned files or worktrees, and persist findings and validation before ending. The coordinator integrates and verifies the combined result.
- Keep networking local by default. Prefer a small, precise design over premature optimization.
- Do not report validation that was not actually performed. Require a clean build and actual browser UI verification before marking the milestones complete.

Current binary-source ownership and interfaces are specified in `docs/BINARY-PLAN.md`; `docs/CONTRACT.md` retains historical APIs. Read the current validation and review records before changing behavior.

RVB2 computation ownership/interfaces are specified in `docs/TWO-SUM-PLAN.md`, with exact encoding in `docs/COMPUTE-FORMAT.md`. Keep algorithms in binary source and generated native operations; runtime helpers must remain general-purpose.

Read `docs/ADD-TWO-NUMBERS-PLAN.md` for the additive linked-list/copy/arithmetic contract and `docs/ROADMAP.md` for the user's language-capability and performance direction. Extend general capabilities, test them outside individual examples, and distinguish measured performance from aspirations.

Read `docs/MEDIAN-PLAN.md` for the floating-point, median, speed-profile and comparison contract. Benchmark-only boundary substitutions must remain outside production code; preserve runtime checks and report timing/memory scopes precisely.

Read `docs/NATIVE-OPTIMIZATION-PLAN.md` before optimizing native computation. Preserve exact source fuel/failure behavior and compare unchanged benchmark inputs and baselines; record new optimization interfaces before delegation.
