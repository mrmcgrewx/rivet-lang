#!/usr/bin/env python3
"""Create fresh, independent worker packets. Run once before measured trials."""
import json
from pathlib import Path
import shutil
import tempfile

from grade import PRIVATE
from protocol import BASE, PROJECT, LANGUAGES, PUBLIC, TASKS, digest, expected

TASK_TEXT = {
    "two_sum": """Given an integer array nums and integer target, return the two distinct indices whose values sum to target. Exactly one unordered pair exists. Either order is accepted. Do not reuse an element. Require a subquadratic search algorithm (expected O(n) or O(n log n), not a quadratic scan).
Constraints: 2 <= nums.length <= 10000; each value and target are in [-1000000000,1000000000].
Input: one count-prefixed array followed by target. Rivet must read_array once then read_int once, and return an IntArray with two indices.
Problem: https://leetcode.com/problems/two-sum/description/
""",
    "add_two_numbers": """Two nonempty singly linked lists represent nonnegative integers with decimal digits in reverse order. Add them and return the result as a singly linked list in the same order. Use actual list traversal and result nodes; do not convert entire numbers to scalar/string/big-integer representations or solve on array substitutes. Require O(m+n) work.
Constraints: each list has 1..100 nodes; values are 0..9; neither number has a leading zero except the number zero (the final node is nonzero when length > 1).
Input: two count-prefixed lists. Rivet must read_list twice and return an IntList. C must return individually malloc-allocated nodes owned by the caller, without input aliases. Rust uses the supplied Box<Node> list type.
Problem: https://leetcode.com/problems/add-two-numbers/description/
""",
    "median": """Given two nondecreasing sorted integer arrays, return the median of their combined values. For an even total length, average the two middle values. Require O(log(m+n)) search work (or better); input parsing/preparation is outside this search requirement. Do not merge or sort the arrays.
Constraints: 0 <= m,n <= 1000; 1 <= m+n <= 2000; each value is in [-1000000,1000000].
Input: two count-prefixed arrays. Rivet must read_array twice and return a Float.
Problem: https://leetcode.com/problems/median-of-two-sorted-arrays/description/
""",
}
RUST_SIGNATURES = {
    "two_sum": "pub fn solve(nums: &[i64], target: i64) -> (usize, usize)",
    "add_two_numbers": "pub fn solve(first: &Option<Box<Node>>, second: &Option<Box<Node>>) -> Option<Box<Node>>",
    "median": "pub fn solve(first: &[i64], second: &[i64]) -> f64",
}
AGENT_RULES = """# Controlled experiment worker
Read TASK.md and CONTRACT.md in this workspace. Read API.md and any provided language reference/header as needed. These files are your complete project context; the repository explicitly exempts experiment workers from reading its broader project docs.
Own only solution.c/solution.rs/solution.rvb (whichever TASK.md assigns), optional Rivet Python byte writers, and HANDOFF.md here. Do not change these instructions or other supplied packet files.
Use only this workspace, the supplied packet, and the exact submission command. Do not inspect the grader, compiler implementation, repository, prior solutions/history, hidden corpora, other workspaces, session logs, or unrelated files. No web/network, package installs, external dependencies, other agents, cross-worker messages, or Git.
All compilation, execution, inspection/validation of Rivet code, and testing must use the formal submission command. Do not manually run compilers, interpreters, debuggers, static analyzers, or tests. You may execute Python solely to construct Rivet bytes from numeric records and resolve labels, not to execute/test the algorithm. File editing and reasoning are allowed.
At most three formal submissions (initial plus two repairs). Stop immediately after the first development pass or third submission. Do not edit the submitted solver after stopping. Explain algorithm/complexity and submission outcome in HANDOFF.md. Do not claim hidden-test correctness or estimate token counts.
"""


def prepare():
    PRIVATE.mkdir(parents=True, exist_ok=True)
    config_path = PRIVATE / "config.json"
    if config_path.exists():
        raise SystemExit("Refusing to replace existing trial registration")
    root = Path(tempfile.mkdtemp(prefix="rivet-authoring-pilot-", dir="/private/tmp"))
    frozen_compiler = PRIVATE / "rivet-frozen"
    shutil.copy2(PROJECT / "target/release/rivet", frozen_compiler)
    contract = (PROJECT / "docs/AGENT-EXPERIMENT-PLAN.md").read_text()
    reference = (PROJECT / "docs/COMPUTE-FORMAT.md").read_text()
    reference = "# Rivet RVB2 binary source reference\n\n## Header and register bank" + reference.split("## Header and register bank", 1)[1].split("## Native compilation and inspection", 1)[0]
    start = reference.index("For the committed Two Sum program")
    end = reference.index("| Resource/condition", start)
    reference = reference[:start] + reference[end:]
    trials, packets = [], {}
    for task in TASKS:
        for language in LANGUAGES:
            trial_id = f"{task}-{language}"
            workspace = root / trial_id
            workspace.mkdir()
            (workspace / "AGENTS.md").write_text(AGENT_RULES)
            (workspace / "CONTRACT.md").write_text(contract)
            command = f"python3 {BASE}/grade.py submit {workspace}"
            examples = "\n".join(f"- Input values: `{json.dumps(case)}`; result: `{json.dumps(expected(task, case))}`." for case in PUBLIC[task])
            extension = {"rivet": "rvb", "c": "c", "rust": "rs"}[language]
            (workspace / "TASK.md").write_text(f"# {task} in {language}\n\n{TASK_TEXT[task]}\n## Public examples\n\n{examples}\n\nWrite `solution.{extension}`. All valid inputs must work; do not hardcode examples. Caller handles input parsing and output for C/Rust. Rivet uses its generic read/return instructions. Domain constraints may be assumed; no extra domain-validation pass is required. Preserve language/runtime safety checks.\n\nSubmit using exactly:\n```sh\n{command}\n```\n\nThe development suite has 10 cases and reports compiler diagnostics or the first failure. You cannot inspect its cases in advance. Stop on first pass or after 3 submissions. A separate withheld suite is run after your work is frozen; you cannot repair from its feedback.\n")
            if language == "c":
                shutil.copyfile(BASE / "harness/api.h", workspace / "api.h")
                api = "Write solution.c with #include \"api.h\" and the solve function declared for your TASK (1=two_sum, 2=add_two_numbers, 3=median). Header types are supplied; do not redefine them. Standard C11 library only. No main, stdin/stdout, process control, or file access. Compiler: Apple Clang17, -std=c11 -O3 -Wall -Wextra -Werror -pedantic, separate solver/driver translation units, no LTO or fast-math. Final solution also tested with ASan/UBSan.\n"
            elif language == "rust":
                api = f"Write solution.rs containing `{RUST_SIGNATURES[task]}` and any private helpers. Standard library only, safe Rust; no main, stdin/stdout, process control, or file access. Rust1.90, edition2024, opt-level=3, overflow-checks=yes, panic=abort, lto=off, forbid unsafe_code, deny warnings. Coordinator supplies a separate rlib entry wrapper.\n"
                if task == "add_two_numbers":
                    api += "The following type is PREPENDED by the coordinator at your crate root; use it directly, do not declare it again:\n```rust\npub struct Node { pub value: i64, pub next: Option<Box<Node>> }\n```\nCaller retains inputs and releases the returned result.\n"
            else:
                api = "Write actual binary RVB2 bytes in solution.rvb using RVB2.md. No textual or JSON source is accepted. A Python byte writer may encode numeric instruction records and resolve label PCs; retain it as a .py file. It cannot solve the input problem or run an interpreter/self-test. No existing assembler/solution files are supplied. The language's checked arithmetic, array/list/map checks, 1,000,000-instruction fuel, and allocation limits stay enabled under the speed compilation profile. Exactly the read/return sequence in TASK.md is required. Do not add stdin reads.\n"
                (workspace / "RVB2.md").write_text(reference)
            (workspace / "API.md").write_text(api)
            packet = {p.name: digest(p) for p in sorted(workspace.iterdir())}
            packets[trial_id] = packet
            trials.append({"id": trial_id, "task": task, "language": language, "workspace": str(workspace),
                           "agent_path": "/root/pilot_" + trial_id.replace("-", "_")})
            destination = BASE / "packets" / trial_id
            shutil.copytree(workspace, destination)
    config = {"format": 1, "trials": trials, "corpus": str(BASE / "corpora.json"),
              "results": str(BASE / "results/trials"), "builds": str(PRIVATE / "builds"),
              "rivet": str(frozen_compiler), "parent_thread_id": "01a173c9-16f5-72a1-a5dc-2c4013e0f5b9"}
    config_path.write_text(json.dumps(config, indent=2) + "\n")
    frozen_paths = [BASE / "corpora.json", BASE / "protocol.py", BASE / "grade.py", BASE / "prepare.py", BASE / "preloaded.py", BASE / "test_harness.py",
                    *sorted((BASE / "harness").glob("*")), PROJECT / "native/runtime.c", PROJECT / "native/runtime.h"]
    manifest = {"format": 1, "expected_model": "gpt-6-astra", "expected_effort": "xhigh",
                "compiler_sha256": digest(frozen_compiler),
                "files": {str(p.relative_to(PROJECT)): digest(p) for p in frozen_paths}, "packets": packets}
    (BASE / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps({"workspaces": str(root), "trials": len(trials), "config": str(config_path)}))


if __name__ == "__main__":
    prepare()
