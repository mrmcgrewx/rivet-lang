#!/usr/bin/env python3
"""Coordinator-owned build/grading service; workers may invoke, not inspect it."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import time

from protocol import BASE, PROJECT, TASKS, check_output, digest, encode

HARNESS = BASE / "harness"
PRIVATE = PROJECT / ".rivet/agent-language-v1"
C_FLAGS = ["-std=c11", "-O3", "-Wall", "-Wextra", "-Werror", "-pedantic"]
RUST_FLAGS = ["--edition=2024", "-C", "opt-level=3", "-C", "overflow-checks=yes",
              "-C", "panic=abort", "-C", "lto=off", "-F", "unsafe_code", "-D", "warnings"]
NODE = "pub struct Node { pub value: i64, pub next: Option<Box<Node>> }\n"
ENTRIES = {
    "two_sum": "pub fn entry(a: &[i64], target: i64) -> (usize, usize) { solve(a, target) }",
    "add_two_numbers": "pub fn entry(a: &Option<Box<Node>>, b: &Option<Box<Node>>) -> Option<Box<Node>> { solve(a, b) }",
    "median": "pub fn entry(a: &[i64], b: &[i64]) -> f64 { solve(a, b) }",
}


def run(command, *, stdin=b"", timeout=60, cwd=None, env=None):
    started = time.monotonic_ns()
    try:
        done = subprocess.run([str(x) for x in command], input=stdin, capture_output=True,
                              timeout=timeout, cwd=cwd, env=env)
        return {"command": [str(x) for x in command], "exit_code": done.returncode,
                "stdout": done.stdout.decode(errors="replace")[:16000],
                "stderr": done.stderr.decode(errors="replace")[:16000],
                "elapsed_ns": time.monotonic_ns() - started}
    except subprocess.TimeoutExpired:
        return {"command": [str(x) for x in command], "exit_code": None,
                "stdout": "", "stderr": "time limit exceeded",
                "elapsed_ns": time.monotonic_ns() - started}


def build(language, task, source, directory, *, sanitizer=False, rivet=None):
    directory = Path(directory)
    directory.mkdir(parents=True, exist_ok=True)
    executable = directory / "program"
    commands = []
    if language == "c":
        flags = C_FLAGS + (["-fsanitize=address,undefined", "-fno-sanitize-recover=all",
                            "-fno-omit-frame-pointer"] if sanitizer else [])
        commands.append(["clang", *flags, f"-DTASK={TASKS[task]}", "-I", HARNESS,
                         HARNESS / "driver.c", HARNESS / "c_backend.c", source,
                         "-o", executable])
    elif language == "rust":
        combined = directory / "solver.rs"
        combined.write_text((NODE if task == "add_two_numbers" else "") + Path(source).read_text()
                            + "\n#[inline(never)]\n" + ENTRIES[task] + "\n")
        library = directory / "libsolution.rlib"
        commands.append(["rustc", *RUST_FLAGS, "--crate-name", "solution", "--crate-type", "rlib",
                         combined, "-o", library])
        commands.append(["rustc", *RUST_FLAGS, "--cfg", f'task="{task}"', HARNESS / "driver.rs",
                         "--extern", f"solution={library}", "-o", executable])
    else:
        commands.append([rivet or PROJECT / "target/release/rivet", "build", source,
                         "-o", executable, "--profile", "speed"])
    logs = []
    for command in commands:
        result = run(command)
        logs.append(result)
        if result["exit_code"] != 0:
            return {"ok": False, "logs": logs}
    return {"ok": True, "logs": logs, "executable": str(executable),
            "executable_bytes": executable.stat().st_size}


def check_cases(task, executable, cases, *, stop_on_failure=False, interpreter=False):
    outcomes = []
    for index, case in enumerate(cases):
        command = ([PROJECT / "target/release/rivet", "interpret", executable] if interpreter
                   else [executable])
        result = run(command, stdin=encode(task, case), timeout=3)
        checked = check_output(task, case, result["stdout"])
        outcome = {"index": index, "ok": result["exit_code"] == 0 and checked["ok"],
                   "exit_code": result["exit_code"], "elapsed_ns": result["elapsed_ns"]}
        if not outcome["ok"]:
            outcome.update(case=case, check=checked, stderr=result["stderr"])
        outcomes.append(outcome)
        if stop_on_failure and not outcome["ok"]:
            break
    return {"passed": sum(x["ok"] for x in outcomes), "total": len(cases),
            "executed": len(outcomes), "cases": outcomes,
            "ok": len(outcomes) == len(cases) and all(x["ok"] for x in outcomes)}


def submit(workspace, config_path=PRIVATE / "config.json"):
    config = json.loads(Path(config_path).read_text())
    workspace = Path(workspace).resolve()
    trial = next((t for t in config["trials"] if Path(t["workspace"]).resolve() == workspace), None)
    if trial is None:
        raise ValueError("workspace is not registered")
    output = Path(config["results"]) / trial["id"]
    output.mkdir(parents=True, exist_ok=True)
    lock = output / "submitting.lock"
    fd = os.open(lock, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
    os.close(fd)
    try:
        prior = sorted(output.glob("attempt-*/result.json"))
        if len(prior) >= 3 or any(json.loads(p.read_text()).get("passed") for p in prior):
            raise ValueError("trial is finished; no further submissions allowed")
        number = len(prior) + 1
        attempt = output / f"attempt-{number}"
        attempt.mkdir()
        extension = {"c": "c", "rust": "rs", "rivet": "rvb"}[trial["language"]]
        filename = "solution." + extension
        result = {"trial": trial["id"], "attempt": number, "passed": False,
                  "submitted_unix_ns": time.time_ns(), "files": {}}
        try:
            source = workspace / filename
            if not source.is_file() or not 0 < source.stat().st_size <= 65536:
                raise ValueError(f"{filename} must contain 1..65536 bytes")
            names = [source] + (sorted(workspace.glob("*.py")) if trial["language"] == "rivet" else [])
            for path in names:
                if path.is_symlink() or path.stat().st_size > 262144:
                    raise ValueError("source snapshots must be bounded regular files")
                shutil.copyfile(path, attempt / path.name)
                result["files"][path.name] = digest(attempt / path.name)
            built = build(trial["language"], trial["task"], attempt / filename,
                          Path(config["builds"]) / trial["id"] / f"attempt-{number}",
                          rivet=config["rivet"])
            result["build"] = built
            if built["ok"]:
                corpus = json.loads(Path(config["corpus"]).read_text())
                result["development"] = check_cases(trial["task"], built["executable"],
                                                     corpus["development"][trial["task"]],
                                                     stop_on_failure=True)
                result["passed"] = result["development"]["ok"]
        except (ValueError, OSError) as exc:
            result["error"] = str(exc)
        (attempt / "result.json").write_text(json.dumps(result, indent=2) + "\n")
        feedback = {"attempt": number, "passed": result["passed"],
                    "submissions_remaining": 0 if result["passed"] else 3 - number}
        if "error" in result:
            feedback["error"] = result["error"]
        elif not result["build"]["ok"]:
            feedback["compiler_diagnostics"] = result["build"]["logs"][-1]["stderr"]
        elif not result["passed"]:
            failure = next(c for c in result["development"]["cases"] if not c["ok"])
            feedback["first_failed_case"] = failure
        else:
            feedback["development_cases_passed"] = result["development"]["passed"]
        print(json.dumps(feedback, indent=2))
        return result
    finally:
        lock.unlink()


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("action", choices=["submit"])
    parser.add_argument("workspace", type=Path)
    arguments = parser.parse_args()
    submit(arguments.workspace)
