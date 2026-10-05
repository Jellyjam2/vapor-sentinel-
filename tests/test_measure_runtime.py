"""Exercise the measurement CLI against the real release binary and failing children."""

import importlib.util
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "measure_runtime.py"
spec = importlib.util.spec_from_file_location("measure_runtime", SCRIPT)
measure = importlib.util.module_from_spec(spec)
spec.loader.exec_module(measure)


@unittest.skipUnless(sys.platform == "linux", "Linux resource measurement")
class MeasurementTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.directory = Path(self.temp.name)
        self.policy = self.directory / "original.vapor"
        self.policy.write_text('vapor s(){send("PILOT_TEST");}\n', encoding="utf-8")
        self.config = self.directory / "original.json"
        self.config.write_text(json.dumps({"dsl_path": "original.vapor", "poll_interval_secs": 4}))
        self.output = self.directory / "measurement"

    def run_cli(self, binary=None, extra=()):
        env = dict(os.environ)
        env.update({
            "VAPOR_SENTINEL_ENABLE_ACTIONS": "1",
            "VAPOR_SENTINEL_WEBHOOK_URL": "deliberately-invalid-must-be-removed",
            "VAPOR_SENTINEL_CONFIG": "nonexistent",
            "VAPOR_SENTINEL_SOURCE_ID": "invalid\nidentity",
        })
        return subprocess.run([
            sys.executable, str(SCRIPT), "--binary", str(binary or ROOT / "target/release/vapor_project"),
            "--config", str(self.config), "--samples", "2", "--output", str(self.output), *extra,
        ], env=env, capture_output=True, text=True, timeout=15)

    def fake_binary(self, body):
        binary = self.directory / "controlled-child"
        binary.write_text(
            "#!/usr/bin/env python3\nimport json, os, signal, sys, time\n"
            "if '--check' in sys.argv:\n    print(json.dumps({'valid': True}))\n    sys.exit(0)\n" + body,
            encoding="utf-8",
        )
        binary.chmod(0o755)
        return binary

    def report(self):
        return json.loads((self.output / "report.json").read_text())

    def test_real_binary_disables_inherited_webhook_and_preserves_inputs(self):
        before = (self.policy.read_bytes(), self.config.read_bytes())
        completed = self.run_cli()
        report = self.report()
        restricted_proc = report.get("error", "").startswith("post-exec RSS was unavailable")
        self.assertEqual(completed.returncode, 1 if restricted_proc else 0, completed.stderr)
        self.assertEqual(report["valid_measurement"], not restricted_proc)
        self.assertEqual(report["recording"]["evaluations"], 2)
        self.assertEqual(report["recording"]["states"], {"Unknown": 1, "Anomalous": 1})
        self.assertEqual(report["recording"]["delivery_status_counts"], {"skipped": 1})
        self.assertEqual(before, (self.policy.read_bytes(), self.config.read_bytes()))
        self.assertEqual(report["poll_interval_secs"], 1)
        rows = [json.loads(line) for line in (self.output / "evidence.jsonl").read_text().splitlines()]
        self.assertEqual(rows[0]["source_id"], "pilot-measurement")
        self.assertFalse(rows[0]["replay"])
        # A missing final delivery must invalidate the measurement rather than imply success.
        incomplete = self.directory / "incomplete.jsonl"
        incomplete.write_text("\n".join(json.dumps(row) for row in rows if row["record_type"] != "delivery") + "\n")
        with self.assertRaisesRegex(ValueError, "recorded outcomes differ"):
            measure.inspect_recording(incomplete, 2, report["policy_sha256"])
        if restricted_proc:
            self.skipTest("post-exec /proc inspection is restricted; CI smoke must verify RSS")
        self.assertGreater(report["resources"]["runtime_high_water_rss_kib_observed"], 0)
        self.assertGreater(report["resources"]["runtime_rss_samples"], 0)

    def test_nonzero_exit_is_recorded_and_fails(self):
        binary = self.fake_binary("print('controlled failure', file=sys.stderr)\nsys.exit(7)\n")
        completed = self.run_cli(binary)
        self.assertEqual(completed.returncode, 1)
        self.assertFalse(self.report()["valid_measurement"])
        self.assertEqual(self.report()["resources"]["exit_code"], 7)
        self.assertIn("controlled failure", (self.output / "runtime.stderr.log").read_text())

    def test_empty_successful_process_is_not_a_valid_measurement(self):
        completed = self.run_cli(self.fake_binary("sys.exit(0)\n"))
        self.assertEqual(completed.returncode, 1)
        self.assertFalse(self.report()["valid_measurement"])
        self.assertIn("expected 2 evaluations; found 0", self.report()["error"])

    def test_timeout_terminates_and_reaps_an_unresponsive_child(self):
        binary = self.fake_binary("signal.signal(signal.SIGTERM, signal.SIG_IGN)\ntime.sleep(30)\n")
        completed = self.run_cli(binary, ("--timeout-secs", "0.5"))
        self.assertEqual(completed.returncode, 1)
        resources = self.report()["resources"]
        self.assertEqual(resources["stop_reason"], "timeout")
        self.assertEqual(resources["exit_code"], -signal.SIGKILL)

    def test_existing_output_is_never_overwritten(self):
        self.output.mkdir()
        original = self.output / "report.json"
        original.write_text("keep this recording")
        completed = self.run_cli()
        self.assertNotEqual(completed.returncode, 0)
        self.assertEqual(original.read_text(), "keep this recording")
        self.assertEqual(list(self.output.iterdir()), [original])

    def test_invalid_timeout_is_rejected_before_creating_files(self):
        for value in ["nan", "inf", "0", "-1"]:
            completed = self.run_cli(extra=("--timeout-secs", value))
            self.assertNotEqual(completed.returncode, 0)
            self.assertFalse(self.output.exists())


if __name__ == "__main__":
    unittest.main()
