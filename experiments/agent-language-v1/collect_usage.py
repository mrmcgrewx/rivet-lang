#!/usr/bin/env python3
"""Extract numeric usage for exact experiment children; never export transcripts."""
import argparse
import hashlib
import json
from pathlib import Path

from grade import PRIVATE
from protocol import BASE


def collect(session_directory):
    config = json.loads((PRIVATE / "config.json").read_text())
    expected_paths = {t["agent_path"]: t for t in config["trials"]}
    sessions = {}
    for path in Path(session_directory).rglob("*.jsonl"):
        with path.open() as handle:
            try:
                metadata = json.loads(handle.readline())["payload"]
            except (ValueError, KeyError):
                continue
            source = metadata.get("source", {})
            if not isinstance(source, dict):
                continue
            spawn = source.get("subagent", {}).get("thread_spawn", {})
            agent_path = spawn.get("agent_path")
            if agent_path not in expected_paths or spawn.get("parent_thread_id") != config["parent_thread_id"]:
                continue
            if agent_path in sessions:
                raise ValueError(f"Ambiguous measured session for {agent_path}")
            trial = expected_paths[agent_path]
            usage_events, models, calls = [], set(), []
            started, completed = None, None
            for line in handle:
                try:
                    row = json.loads(line)
                except ValueError:
                    continue  # A live writer can have an incomplete final line.
                payload = row.get("payload", {})
                if row["type"] == "turn_context":
                    models.add((payload.get("model"), payload.get("effort")))
                if row["type"] == "event_msg":
                    event = payload.get("type")
                    if event == "task_started":
                        started = started or row.get("timestamp")
                    if event == "task_complete":
                        completed = row.get("timestamp")
                    if event == "token_count" and payload.get("info"):
                        usage_events.append({"timestamp": row.get("timestamp"),
                                             **payload["info"]["total_token_usage"]})
                if row["type"] == "response_item" and payload.get("type") in ("function_call", "custom_tool_call"):
                    calls.append({"timestamp": row.get("timestamp"), "name": payload.get("name"),
                                  "input": payload.get("input", payload.get("arguments"))})
            if not usage_events:
                continue
            usage = dict(usage_events[-1])
            usage.pop("timestamp")
            assert usage["cached_input_tokens"] <= usage["input_tokens"]
            assert usage["reasoning_output_tokens"] <= usage["output_tokens"]
            assert usage["total_tokens"] == usage["input_tokens"] + usage["output_tokens"]
            usage["uncached_input_tokens"] = usage["input_tokens"] - usage["cached_input_tokens"]
            audit_text = json.dumps(calls, indent=2) + "\n"
            audit_path = PRIVATE / "command-audit" / f"{trial['id']}.json"
            audit_path.parent.mkdir(parents=True, exist_ok=True)
            audit_path.write_text(audit_text)
            sessions[agent_path] = {"id": trial["id"], "agent_path": agent_path,
                                    "thread_id": metadata["id"], "models": [dict(model=m, effort=e) for m, e in sorted(models)],
                                    "started": started, "completed": completed, "usage": usage,
                                    "cumulative_usage_observations": usage_events, "tool_call_count": len(calls),
                                    "command_audit_sha256": hashlib.sha256(audit_text.encode()).hexdigest()}
    rows = sorted(sessions.values(), key=lambda r: r["id"])
    report = {"format": 1, "source": "actual child session token_count total_token_usage events",
              "scope": "nine fresh measured workers only; coordinator/harness/review excluded",
              "counting": "cached input is included in input; reasoning output is included in output; cumulative across calls",
              "expected_model": "gpt-6-astra", "expected_effort": "xhigh", "trials": rows}
    for row in rows:
        if row["models"] != [{"model": "gpt-6-astra", "effort": "xhigh"}]:
            raise ValueError(f"Model/effort mismatch: {row['id']}")
    (BASE / "results").mkdir(exist_ok=True)
    (BASE / "results/usage.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"recorded_trials": len(rows), "completed": sum(r["completed"] is not None for r in rows),
                      "model": "gpt-6-astra", "effort": "xhigh"}))
    return report


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--sessions", type=Path, default=Path.home() / ".codex/sessions/2026/10/25")
    args = parser.parse_args()
    collect(args.sessions)
