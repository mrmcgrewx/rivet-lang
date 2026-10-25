#!/usr/bin/env python3
"""Supplementary frozen-C holdouts under ASan/UBSan, portable to Linux CI."""
import argparse
import json
from pathlib import Path
import tempfile
import sys

from evaluate import final_candidates, verify_manifest
from grade import C_FLAGS, HARNESS, check_cases, run
from protocol import BASE, TASKS, digest


def sanitizer_build(task, source, directory, mode):
    directory = Path(directory)
    directory.mkdir(parents=True, exist_ok=True)
    executable = directory / "program"
    command = ["clang", *C_FLAGS, f"-fsanitize={mode}", "-fno-sanitize-recover=all", "-fno-omit-frame-pointer",
               f"-DTASK={TASKS[task]}", "-I", HARNESS, HARNESS / "driver.c", HARNESS / "c_backend.c",
               source, "-o", executable]
    if sys.platform != "darwin":
        command.append("-lm")
    result = run(command)
    built = {"ok": result["exit_code"] == 0, "logs": [result], "executable": str(executable)}
    if built["ok"]:
        built["executable_bytes"] = executable.stat().st_size
    return built


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--mode", choices=["address,undefined", "undefined"], default="address,undefined")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    verify_manifest()
    corpus = json.loads((BASE / "corpora.json").read_text())
    root = Path(tempfile.mkdtemp(prefix="rivet-pilot-sanitizers-"))
    report = {"mode": args.mode, "compiler": run(["clang", "--version"])["stdout"],
              "corpus_sha256": digest(BASE / "corpora.json"), "trials": []}
    for candidate in final_candidates():
        if candidate["language"] != "c":
            continue
        row = {"id": candidate["id"], "source_sha256": digest(candidate["source"])}
        row["build"] = sanitizer_build(candidate["task"], candidate["source"], root / candidate["id"], args.mode)
        if row["build"]["ok"]:
            row["holdout"] = check_cases(candidate["task"], row["build"]["executable"], corpus["holdout"][candidate["task"]])
        report["trials"].append(row)
        print(candidate["id"], row.get("holdout", {}).get("passed", 0), "passed", flush=True)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    if len(report["trials"]) != 3 or not all(r.get("holdout", {}).get("ok") for r in report["trials"]):
        raise SystemExit(1)


if __name__ == "__main__":
    main()
