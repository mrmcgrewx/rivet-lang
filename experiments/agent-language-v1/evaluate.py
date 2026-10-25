#!/usr/bin/env python3
"""Post-freeze holdout evaluation, also usable from a clean checkout."""
import argparse
import json
from pathlib import Path
import shutil
import tempfile

from grade import PRIVATE, build, check_cases
from protocol import BASE, PROJECT, digest


def verify_manifest():
    manifest = json.loads((BASE / "manifest.json").read_text())
    for filename, wanted in manifest["files"].items():
        if digest(PROJECT / filename) != wanted:
            raise ValueError(f"Frozen experiment file changed: {filename}")
    for trial, files in manifest["packets"].items():
        for filename, wanted in files.items():
            if digest(BASE / "packets" / trial / filename) != wanted:
                raise ValueError(f"Frozen packet changed: {trial}/{filename}")
    return manifest


def final_candidates():
    candidates = []
    for directory in sorted((BASE / "results/trials").iterdir()):
        attempts = sorted(directory.glob("attempt-*/result.json"))
        if not attempts:
            continue
        results = [json.loads(path.read_text()) for path in attempts]
        task, language = directory.name.rsplit("-", 1)
        extension = {"rivet": "rvb", "c": "c", "rust": "rs"}[language]
        candidates.append({"id": directory.name, "task": task, "language": language,
                           "attempts": results, "source": attempts[-1].parent / f"solution.{extension}"})
    return candidates


def evaluate(output, build_root, copy_handoffs=False, sanitizer_mode="address,undefined"):
    manifest = verify_manifest()
    corpus = json.loads((BASE / "corpora.json").read_text())
    report = {"format": 1, "protocol_commit": "7461e96", "manifest_sha256": digest(BASE / "manifest.json"),
              "corpus_sha256": digest(BASE / "corpora.json"), "local_c_sanitizer_mode": sanitizer_mode, "trials": []}
    config = json.loads((PRIVATE / "config.json").read_text()) if copy_handoffs else None
    compiler = Path(config["rivet"]) if config else PROJECT / "target/release/rivet"
    if config:
        assert digest(compiler) == manifest["compiler_sha256"]
    for candidate in final_candidates():
        task, language, trial = candidate["task"], candidate["language"], candidate["id"]
        attempts = candidate["attempts"]
        row = {"id": trial, "task": task, "language": language,
               "submissions": len(attempts), "retries": len(attempts) - 1,
               "first_development_pass": attempts[0]["passed"], "final_development_pass": attempts[-1]["passed"],
               "source": str(candidate["source"].relative_to(PROJECT)),
               "source_sha256": digest(candidate["source"]), "source_bytes": candidate["source"].stat().st_size}
        if config:
            workspace = Path(next(t["workspace"] for t in config["trials"] if t["id"] == trial))
            for filename, wanted in attempts[-1]["files"].items():
                assert digest(workspace / filename) == wanted, f"post-submission source change: {trial}/{filename}"
            for filename, wanted in manifest["packets"][trial].items():
                assert digest(workspace / filename) == wanted, f"worker changed packet: {trial}/{filename}"
            handoff = workspace / "HANDOFF.md"
            if handoff.exists():
                shutil.copyfile(handoff, BASE / "results/trials" / trial / "HANDOFF.md")
        built = build(language, task, candidate["source"], build_root / trial, rivet=compiler)
        row["build"] = built
        if built["ok"]:
            row["holdout"] = check_cases(task, built["executable"], corpus["holdout"][task])
            all_performance = [case for cases in corpus["performance"][task].values() for case in cases]
            row["performance_cases"] = check_cases(task, built["executable"], all_performance)
            if language == "c":
                from sanitizer_check import sanitizer_build
                sanitized = sanitizer_build(task, candidate["source"], build_root / trial / "sanitized", sanitizer_mode)
                row["sanitizer_build"] = sanitized
                if sanitized["ok"]:
                    row["sanitizer_holdout"] = check_cases(task, sanitized["executable"], corpus["holdout"][task])
            elif language == "rivet":
                row["reference_holdout"] = check_cases(task, candidate["source"], corpus["holdout"][task], interpreter=True)
            row["correctness_pass"] = row["holdout"]["ok"] and row["performance_cases"]["ok"]
            if language == "c":
                row["correctness_pass"] &= row.get("sanitizer_holdout", {}).get("ok", False)
            if language == "rivet":
                row["correctness_pass"] &= row["reference_holdout"]["ok"]
        else:
            row["correctness_pass"] = False
        report["trials"].append(row)
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps(report, indent=2) + "\n")
        print(f"{trial}: {row.get('holdout', {}).get('passed', 0)}/{len(corpus['holdout'][task])} holdouts; correctness_pass={row['correctness_pass']}", flush=True)
    return report


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=BASE / "results/correctness.json")
    parser.add_argument("--build-root", type=Path)
    parser.add_argument("--collect-handoffs", action="store_true")
    parser.add_argument("--sanitizer-mode", choices=["address,undefined", "undefined"], default="address,undefined")
    args = parser.parse_args()
    root = args.build_root or Path(tempfile.mkdtemp(prefix="rivet-pilot-evaluate-"))
    evaluate(args.output, root, args.collect_handoffs, args.sanitizer_mode)
