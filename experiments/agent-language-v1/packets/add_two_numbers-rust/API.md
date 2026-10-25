Write solution.rs containing `pub fn solve(first: &Option<Box<Node>>, second: &Option<Box<Node>>) -> Option<Box<Node>>` and any private helpers. Standard library only, safe Rust; no main, stdin/stdout, process control, or file access. Rust1.90, edition2024, opt-level=3, overflow-checks=yes, panic=abort, lto=off, forbid unsafe_code, deny warnings. Coordinator supplies a separate rlib entry wrapper.
The following type is PREPENDED by the coordinator at your crate root; use it directly, do not declare it again:
```rust
pub struct Node { pub value: i64, pub next: Option<Box<Node>> }
```
Caller retains inputs and releases the returned result.
