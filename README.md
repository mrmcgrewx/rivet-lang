# Rivet

An experimental typed program representation, compiler/runtime, and local browser workbench for software engineers.

**Status:** bootstrap only. Neither MVP is implemented yet. See [project state](docs/STATE.md) for the exact continuation point.

## Goal

Build and execute genuine Rivet programs through a UI, first printing `Hello, world!`, then serving a local JSON HTTP endpoint. Program edits must change runtime behavior; the demonstrations must not be hardcoded in the UI.

## Project documents

- [Brief and acceptance criteria](docs/BRIEF.md)
- [Initial architecture and rationale](docs/ARCHITECTURE.md)
- [Milestones, validation, and continuation state](docs/STATE.md)
- [Contributor instructions](AGENTS.md)

The initial target is macOS on Apple Silicon. Rust 1.90.0 and Cargo 1.90.0 are available on the development machine. Setup and launch instructions will be added and verified when implementation exists.

This is an experiment, not a production language or server. No performance claims have been established.
