#!/usr/bin/env python3
"""Measure a bounded, actions-disabled Vapor run on Linux (Python 3.9+)."""

import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import signal
import statistics
import subprocess
import sys
import time


def digest(path):
    checksum = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(65536), b""):
            checksum.update(block)
    return checksum.hexdigest()


def bounded_read(path, limit):
    if not path.is_file():
        raise ValueError(f"expected a regular file: {path}")
    with path.open("rb") as source:
        data = source.read(limit + 1)
    if len(data) > limit:
        raise ValueError(f"input exceeds {limit} bytes: {path}")
    return data


def measurement_env():
    # Never inherit a configured webhook or a launch-time exit/config override.
    result = {k: v for k, v in os.environ.items() if not k.startswith("VAPOR_SENTINEL_")}
    result["VAPOR_SENTINEL_ENABLE_ACTIONS"] = "0"
    result["VAPOR_SENTINEL_SOURCE_ID"] = "pilot-measurement"
    return result


def runtime_rss(pid, executable_identity):
    """Read the post-exec image's high-water mark; exclude the Python launcher."""
    try:
        proc = Path("/proc") / str(pid)
        current = (proc / "exe").stat()
        if (current.st_dev, current.st_ino) != executable_identity:
            return None
        for line in (proc / "status").read_text().splitlines():
            if line.startswith("VmHWM:"):
                return int(line.split()[1])  # Linux reports KiB as 'kB'.
    except (OSError, ValueError):
        pass  # The process may exit between the identity and status reads.
    return None


def signal_group(pid, kind):
    try:
        os.killpg(pid, kind)
    except ProcessLookupError:
        pass  # Reap a process that exited at the timeout boundary normally.


def run_measured(command, directory, env, timeout):
    """Reap exactly this child with wait4; never use cumulative child RSS."""
    stop_reason = None
    peak_runtime_rss = None
    rss_samples = 0
    executable = Path(command[0]).stat()
    executable_identity = (executable.st_dev, executable.st_ino)
    with (directory / "evidence.jsonl").open("xb") as stdout, (
        directory / "runtime.stderr.log"
    ).open("xb") as stderr:
        started = time.monotonic()
        child = subprocess.Popen(
            command, stdout=stdout, stderr=stderr, env=env, start_new_session=True
        )
        deadline = started + timeout
        kill_deadline = None
        while True:
            try:
                pid, status, usage = os.wait4(child.pid, os.WNOHANG)
                if pid:
                    child.returncode = os.waitstatus_to_exitcode(status)
                    break
                rss = runtime_rss(child.pid, executable_identity)
                if rss is not None:
                    peak_runtime_rss = max(peak_runtime_rss or 0, rss)
                    rss_samples += 1
                now = time.monotonic()
                if stop_reason is None and now >= deadline:
                    stop_reason = "timeout"
                    signal_group(child.pid, signal.SIGTERM)
                    kill_deadline = now + 2
                elif kill_deadline is not None and now >= kill_deadline:
                    signal_group(child.pid, signal.SIGKILL)
                    kill_deadline = None
                time.sleep(0.02)
            except KeyboardInterrupt:
                stop_reason = "interrupted"
                signal_group(child.pid, signal.SIGTERM)
                kill_deadline = time.monotonic() + 2
        wall = time.monotonic() - started
    cpu = usage.ru_utime + usage.ru_stime
    return {
        "exit_code": child.returncode,
        "stop_reason": stop_reason,
        "wall_seconds": wall,
        "user_cpu_seconds": usage.ru_utime,
        "system_cpu_seconds": usage.ru_stime,
        "average_cpu_percent_one_core": 100 * cpu / wall,
        "child_lifetime_peak_rss_kib": usage.ru_maxrss,
        "runtime_high_water_rss_kib_observed": peak_runtime_rss,
        "runtime_rss_samples": rss_samples,
    }


def inspect_recording(path, expected_samples, policy_hash):
    states, deliveries = Counter(), Counter()
    timestamps = []
    event_ids, delivery_ids, scheduled_ids = set(), set(), set()
    run_id = None
    with path.open(encoding="utf-8") as source:
        for number, line in enumerate(source, 1):
            try:
                row = json.loads(line)
                if row["schema_version"] != 1:
                    raise ValueError("unsupported schema")
                if not isinstance(row["run_id"], str) or not row["run_id"]:
                    raise ValueError("missing run identity")
                if run_id is None:
                    run_id = row["run_id"]
                if row["run_id"] != run_id:
                    raise ValueError("mixed runs")
                if row["record_type"] == "evaluation":
                    if row["replay"] is not False or row["policy_sha256"] != policy_hash:
                        raise ValueError("expected live observations under the copied policy")
                    if row["event_id"] in event_ids:
                        raise ValueError("duplicate evaluation")
                    event_ids.add(row["event_id"])
                    if row["evidence"]["sequence"] != len(event_ids):
                        raise ValueError("nonconsecutive observations")
                    stamp = row["observed_at_ms"]
                    if type(stamp) is not int or stamp < 0:
                        raise ValueError("invalid observation timestamp")
                    timestamps.append(stamp)
                    states[row["evidence"]["state"]] += 1
                    if row["scheduled_plan"]["kind"] != "no_action":
                        scheduled_ids.add(row["event_id"])
                elif row["record_type"] == "delivery":
                    result = row["delivery"]
                    if result["event_id"] in delivery_ids:
                        raise ValueError("duplicate delivery")
                    delivery_ids.add(result["event_id"])
                    outcome = result["outcome"]
                    deliveries[outcome["status"]] += 1
                    if outcome["status"] != "skipped":
                        raise ValueError("measurement must not deliver notifications")
                else:
                    raise ValueError("unexpected record type")
            except (KeyError, TypeError, ValueError) as exc:
                raise ValueError(f"invalid evidence at line {number}: {exc}") from exc
    if len(timestamps) != expected_samples:
        raise ValueError(f"expected {expected_samples} evaluations; found {len(timestamps)}")
    if delivery_ids != scheduled_ids:
        raise ValueError("scheduled notifications and recorded outcomes differ")
    gaps = [b - a for a, b in zip(timestamps, timestamps[1:])]
    return {
        "evaluations": len(timestamps),
        "states": dict(states),
        "delivery_status_counts": dict(deliveries),
        "evidence_bytes": path.stat().st_size,
        # These are wall-clock differences, not monotonic scheduling measurements.
        "wall_clock_sample_gap_ms": {
            "minimum": min(gaps),
            "median": statistics.median(gaps),
            "maximum": max(gaps),
            "nonpositive_count": sum(gap <= 0 for gap in gaps),
        },
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path("target/release/vapor_project"))
    parser.add_argument("--config", type=Path, default=Path("vapor-sentinel.json"))
    parser.add_argument("--samples", type=int, default=30)
    parser.add_argument("--interval-secs", type=int, default=1)
    parser.add_argument("--timeout-secs", type=float)
    parser.add_argument("--output", type=Path, required=True, help="new directory; never overwritten")
    parser.add_argument("--label", default="unspecified workload")
    parser.add_argument("--source-revision", help="operator-supplied source label, not build attestation")
    args = parser.parse_args()
    if sys.platform != "linux" or not hasattr(os, "wait4"):
        parser.error("resource measurement currently supports Linux only")
    if not 2 <= args.samples <= 1000 or not 1 <= args.interval_secs <= 86400:
        parser.error("samples must be 2..1000 and interval-secs must be 1..86400")
    timeout = args.timeout_secs
    if timeout is None:
        timeout = (args.samples - 1) * args.interval_secs + 30
    if not 0 < timeout < float("inf"):
        parser.error("timeout-secs must be a positive finite number")

    binary, original_config = args.binary.resolve(strict=True), args.config.resolve(strict=True)
    if not binary.is_file() or not os.access(binary, os.X_OK):
        parser.error("binary must be an executable regular file")
    config_bytes = bounded_read(original_config, 16384)
    config = json.loads(config_bytes)
    if not isinstance(config, dict) or not isinstance(config.get("dsl_path"), str):
        parser.error("config must contain a string dsl_path")
    policy = Path(config["dsl_path"])
    if not policy.is_absolute():
        policy = original_config.parent / policy
    policy_bytes = bounded_read(policy, 65536)
    config["dsl_path"] = "policy.vapor"
    config["poll_interval_secs"] = args.interval_secs
    directory = args.output.resolve()
    directory.mkdir(mode=0o700, parents=True, exist_ok=False)
    (directory / "policy.vapor").write_bytes(policy_bytes)
    config_path = directory / "config.json"
    config_path.write_text(json.dumps(config, indent=2) + "\n", encoding="utf-8")
    policy_hash = hashlib.sha256(policy_bytes).hexdigest()
    report = {
        "schema_version": 1,
        "started_at_utc": datetime.now(timezone.utc).isoformat(),
        "workload_label": args.label,
        "source_revision_label": args.source_revision,
        "binary_sha256": digest(binary),
        "original_config_sha256": hashlib.sha256(config_bytes).hexdigest(),
        "effective_config_sha256": digest(config_path),
        "policy_sha256": policy_hash,
        "platform": {"os": sys.platform, "kernel": platform.release(), "architecture": platform.machine()},
        "logical_cpus_visible": os.cpu_count(),
        "cpu_affinity_count": len(os.sched_getaffinity(0)),
        "samples_requested": args.samples,
        "poll_interval_secs": args.interval_secs,
        "actions_enabled": False,
        "valid_measurement": False,
    }
    env = measurement_env()
    try:
        # Check parsing/configuration outside the timed observation process.
        checked = subprocess.run(
            [str(binary), "--config", str(config_path), "--check"],
            env=env, capture_output=True, timeout=10, check=False,
        )
        (directory / "check.stderr.log").write_bytes(checked.stderr)
        if checked.returncode != 0 or json.loads(checked.stdout).get("valid") is not True:
            raise ValueError("configuration check failed; inspect check.stderr.log")
        report["resources"] = run_measured(
            [str(binary), "--config", str(config_path), "--samples", str(args.samples)],
            directory, env, timeout,
        )
        resources = report["resources"]
        if resources["exit_code"] != 0 or resources["stop_reason"] is not None:
            raise ValueError("runtime did not complete normally; inspect resources and runtime.stderr.log")
        report["recording"] = inspect_recording(directory / "evidence.jsonl", args.samples, policy_hash)
        if resources["runtime_rss_samples"] == 0:
            raise ValueError("post-exec RSS was unavailable; verify Linux /proc access and use the runtime binary directly")
        report["valid_measurement"] = True
    except (OSError, ValueError, subprocess.SubprocessError) as exc:
        report["error"] = str(exc)
    (directory / "report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(directory / "report.json")
    if report.get("resources", {}).get("stop_reason") == "interrupted":
        return 130
    return 0 if report["valid_measurement"] else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError) as exc:
        print(f"measurement failed: {exc}", file=sys.stderr)
        sys.exit(1)
