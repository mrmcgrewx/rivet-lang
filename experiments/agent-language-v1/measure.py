#!/usr/bin/env python3
"""Measure frozen, holdout-passing submissions. Run with authoring/tests stopped."""
import argparse
import json
import math
import os
from pathlib import Path
import platform
import random
import signal
import statistics
import subprocess
import sys
import tempfile
import time

from evaluate import verify_manifest
from grade import run
from preloaded import build_preloaded
from protocol import BASE, MASK, check_output, digest, encode, pool_checksum, pool_text


def run_child(command, data=b"", timeout=30):
    """Collect this child's wait4 usage, not cumulative RUSAGE_CHILDREN."""
    def timed_out(_signal, _frame):
        raise TimeoutError(f"child exceeded {timeout}s: {command[0]}")
    with tempfile.TemporaryFile() as incoming, tempfile.TemporaryFile() as outgoing, tempfile.TemporaryFile() as errors:
        incoming.write(data)
        incoming.seek(0)
        previous_handler = signal.signal(signal.SIGALRM, timed_out)
        child = None
        try:
            signal.setitimer(signal.ITIMER_REAL, timeout)
            started = time.perf_counter_ns()
            child = subprocess.Popen(list(map(str, command)), stdin=incoming, stdout=outgoing,
                                     stderr=errors, close_fds=False)
            _, status, usage = os.wait4(child.pid, 0)
            elapsed = time.perf_counter_ns() - started
            child.returncode = os.waitstatus_to_exitcode(status)
        except BaseException:
            if child is not None and child.returncode is None:
                child.kill()
                _, status, _ = os.wait4(child.pid, 0)
                child.returncode = os.waitstatus_to_exitcode(status)
            raise
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
            signal.signal(signal.SIGALRM, previous_handler)
        outgoing.seek(0)
        stdout = outgoing.read().decode()
        errors.seek(0)
        stderr = errors.read().decode()
    if child.returncode != 0 or stderr:
        raise RuntimeError(f"child failed: {command}\nexit={child.returncode}\n{stderr}")
    return {"elapsed_ns": elapsed, "peak_rss_bytes": int(usage.ru_maxrss if sys.platform == "darwin" else usage.ru_maxrss * 1024),
            "peak_rss_raw": usage.ru_maxrss, "user_cpu_seconds": usage.ru_utime,
            "system_cpu_seconds": usage.ru_stime, "stdout": stdout}


def distribution(values):
    return {"median": statistics.median(values), "min": min(values), "max": max(values), "n": len(values)}


def measure(args):
    verify_manifest()
    correctness = json.loads(args.correctness.read_text())
    corpus = json.loads((BASE / "corpora.json").read_text())
    work = args.work or Path(tempfile.mkdtemp(prefix="rivet-pilot-measure-"))
    work.mkdir(parents=True, exist_ok=True)
    trials = [row for row in correctness["trials"] if row["correctness_pass"]]
    report = {"format": 1, "started_unix_ns": time.time_ns(), "correctness_sha256": digest(args.correctness),
              "manifest_sha256": digest(BASE / "manifest.json"), "measurement_script_sha256": digest(Path(__file__)),
              "environment": {"platform": platform.platform(), "machine": platform.machine(),
                              "python": sys.version, "processor": run(["sysctl", "-n", "machdep.cpu.brand_string"])["stdout"].strip(),
                              "clang": run(["clang", "--version"])["stdout"].strip(),
                              "rustc": run(["rustc", "--version"])["stdout"].strip()},
              "settings": {"samples": args.samples, "target_ms": args.target_ms, "warmup_rounds": args.warmup_rounds,
                           "max_calls": args.max_calls, "seed": args.seed, "scope": "solve/capture/normalize/hash/cleanup",
                           "rss_scope": "per-child whole-process peak including setup, preflight and warmup; not auxiliary storage"},
              "builds": {}, "verification": [], "orders": [], "raw_samples": [], "summary": {}}
    standalone, loaded, pools = {}, {}, {}
    for trial in trials:
        trial_id, task, language = trial["id"], trial["task"], trial["language"]
        source = BASE.parents[1] / trial["source"]
        assert digest(source) == trial["source_sha256"]
        standalone[trial_id] = Path(trial["build"]["executable"])
        if language == "rivet":
            build = build_preloaded(task, standalone[trial_id], work / trial_id)
            loaded[trial_id] = Path(build["executable"])
            report["builds"][trial_id] = build
        else:
            loaded[trial_id] = standalone[trial_id]
        report["builds"].setdefault(trial_id, {})["standalone_sha256"] = digest(standalone[trial_id])
        report["builds"][trial_id]["preloaded_sha256"] = digest(loaded[trial_id])
        for family, cases in corpus["performance"][task].items():
            pool = work / f"{task}-{family}.txt"
            pool.write_text(pool_text(task, cases))
            pools[task, family] = pool
            for index, case in enumerate(cases):
                result = run_child([standalone[trial_id]], encode(task, case))
                assert check_output(task, case, result["stdout"])["ok"]
                report["verification"].append({"trial": trial_id, "family": family, "case": index, "stdout": result["stdout"]})
    rng = random.Random(args.seed)
    jobs = [(row["id"], row["task"], row["language"], family)
            for row in trials for family in corpus["performance"][row["task"]]]

    def loaded_call(trial, task, family, rounds, mode):
        result = run_child([loaded[trial], "--bench", pools[task, family], rounds, args.warmup_rounds, mode])
        payload = json.loads(result.pop("stdout"))
        expected = pool_checksum(task, corpus["performance"][task][family])
        count = len(corpus["performance"][task][family])
        assert payload["checksum"] == (expected * rounds) & MASK
        assert payload["expected_checksum_per_round"] == expected
        assert payload["calls"] == rounds * count and payload["warmup_calls"] == args.warmup_rounds * count
        assert payload["mode"] == mode and payload["rounds"] == rounds
        result["process_elapsed_ns"] = result.pop("elapsed_ns")
        result.update(payload)
        result["ns_per_call"] = payload["elapsed_ns"] / payload["calls"]
        return result

    # Always preflight both modes for every frozen pool, including smoke reproductions.
    for trial, task, language, family in jobs:
        for mode in ("evaluate", "control"):
            result = loaded_call(trial, task, family, 1, mode)
            report["verification"].append({"trial": trial, "family": family, "preloaded_mode": mode, **result})
    print(f"Verified {len(trials)} frozen submissions across {len(jobs)} language/family combinations.", flush=True)
    if args.verify_only:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, indent=2) + "\n")
        return report

    for sample in range(-1, args.samples):
        ordered = jobs.copy()
        rng.shuffle(ordered)
        report["orders"].append({"scope": "process", "sample": sample, "jobs": ordered})
        for trial, task, language, family in ordered:
            case = corpus["performance"][task][family][0]
            result = run_child([standalone[trial]], encode(task, case))
            assert check_output(task, case, result["stdout"])["ok"]
            report["raw_samples"].append({"scope": "process", "phase": "warmup" if sample < 0 else "measure",
                                           "sample": sample, "trial": trial, "task": task, "language": language,
                                           "family": family, **result})
    rounds = {}
    ordered = jobs.copy()
    rng.shuffle(ordered)
    report["orders"].append({"scope": "calibration", "jobs": ordered})
    for trial, task, language, family in ordered:
        result = loaded_call(trial, task, family, 128, "evaluate")
        wanted = math.ceil(args.target_ms * 1_000_000 / max(result["elapsed_ns"], 1) * 128)
        rounds[trial, family] = max(1, min(wanted, args.max_calls // len(corpus["performance"][task][family])))
        report["raw_samples"].append({"scope": "preloaded", "phase": "calibration", "trial": trial,
                                       "task": task, "language": language, "family": family, **result})
    report["calibrated_rounds"] = {trial: {family: n for (t, family), n in rounds.items() if t == trial} for trial in standalone}
    print("Process samples complete; preloaded calibration complete.", flush=True)
    for sample in range(args.samples):
        ordered = [(*job, mode) for job in jobs for mode in ("evaluate", "control")]
        rng.shuffle(ordered)
        report["orders"].append({"scope": "preloaded", "sample": sample, "jobs": ordered})
        for trial, task, language, family, mode in ordered:
            result = loaded_call(trial, task, family, rounds[trial, family], mode)
            report["raw_samples"].append({"scope": "preloaded", "phase": "measure", "sample": sample,
                                           "trial": trial, "task": task, "language": language, "family": family, **result})
        print(f"Randomized preloaded sample {sample + 1}/{args.samples} complete.", flush=True)
    for scope in ("process", "preloaded"):
        report["summary"][scope] = {}
        for trial, task, language, family in jobs:
            selected = [s for s in report["raw_samples"] if s["scope"] == scope and s["phase"] == "measure"
                        and s["trial"] == trial and s["family"] == family and s.get("mode", "evaluate") == "evaluate"]
            timing = "elapsed_ns" if scope == "process" else "ns_per_call"
            summary = {timing: distribution([s[timing] for s in selected]),
                       "peak_rss_bytes": distribution([s["peak_rss_bytes"] for s in selected])}
            if scope == "preloaded":
                controls = [s["ns_per_call"] for s in report["raw_samples"] if s["scope"] == scope and s["phase"] == "measure"
                            and s["trial"] == trial and s["family"] == family and s.get("mode") == "control"]
                summary["control_ns_per_call"] = distribution(controls)
            report["summary"][scope].setdefault(task, {}).setdefault(family, {})[language] = summary
    report["finished_unix_ns"] = time.time_ns()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    print(f"Saved {len(report['raw_samples'])} observations to {args.output}", flush=True)
    return report


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--correctness", type=Path, default=BASE / "results/correctness.json")
    parser.add_argument("--output", type=Path, default=BASE / "results/performance.json")
    parser.add_argument("--work", type=Path)
    parser.add_argument("--samples", type=int, default=7)
    parser.add_argument("--target-ms", type=int, default=50)
    parser.add_argument("--warmup-rounds", type=int, default=32)
    parser.add_argument("--max-calls", type=int, default=2_000_000)
    parser.add_argument("--seed", type=int, default=2026102603)
    parser.add_argument("--verify-only", action="store_true")
    args = parser.parse_args()
    if args.samples < 1 or args.target_ms < 1 or not 0 <= args.warmup_rounds <= 10000 or not 8 <= args.max_calls <= 100_000_000:
        parser.error("measurement settings outside bounds")
    measure(args)
