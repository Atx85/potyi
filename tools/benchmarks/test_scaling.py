"""Runner regressions: partial output, timeout cleanup, and sample accounting."""
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock

import scaling


class ScalingTests(unittest.TestCase):
    def test_group_signal_denied_falls_back_to_our_child(self):
        process = mock.Mock(pid=123)
        with mock.patch.object(scaling.os, "killpg", side_effect=PermissionError):
            scaling.kill_owned_group(process)
        process.kill.assert_called_once_with()

    def test_interrupted_final_write_keeps_prior_events(self):
        text = 'POTYI_SCALING {"type":"phase","name":"editing"}\nPOTYI_SCALING {"type":'
        self.assertEqual(scaling.events(text), [{"type": "phase", "name": "editing"}])
        with self.assertRaises(json.JSONDecodeError):
            scaling.events(text + "\n")

    def test_libtest_prefix_and_partial_records_survive_timeout(self):
        with tempfile.TemporaryDirectory() as folder:
            log = Path(folder) / "probe.log"
            script = "import time; print('test probe ... POTYI_SCALING {\"type\":\"phase\",\"name\":\"navigation\"}', flush=True); time.sleep(30)"
            result = scaling.bounded([sys.executable, "-c", script], os.environ, log, .3, memory=False)
            self.assertEqual(result["status"], "timed_out")
            self.assertEqual(result["last_phase"], "navigation")
            self.assertLess(result["seconds"], 3)
            self.assertEqual(len(result["events"]), 1)

    def test_failed_run_keeps_partial_measurement_without_claiming_completion(self):
        with tempfile.TemporaryDirectory() as folder:
            row = {"type": "measurement", "stage": "insert", "iteration": 0, "milliseconds": 12, "counters": {}}
            script = f"print({(scaling.MARKER + json.dumps(row))!r}, flush=True); raise SystemExit(7)"
            result = scaling.bounded([sys.executable, "-c", script], os.environ, Path(folder) / "probe.log", 2, False)
            self.assertEqual(result["status"], "failed")
            self.assertEqual(result["exit_code"], 7)
            self.assertEqual(result["events"], [row])

    def test_summary_excludes_warmup_and_incomplete_runs_and_separates_first_edit(self):
        def run(status, warmup, first, later):
            return dict(status=status, warmup=warmup, events=[
                dict(type="measurement", stage="insert", iteration=i, milliseconds=ms, counters={"read_bytes": i})
                for i, ms in enumerate((first, later))])
        rows = scaling.summarize([run("complete", True, 900, 900), run("timed_out", False, 800, 800),
                                  run("complete", False, 10, 20), run("complete", False, 30, 40)])
        self.assertEqual([(r["bucket"], r["samples"], r["median_ms"], r["p99_ms"]) for r in rows],
                         [("first", 2, 20, 30), ("later", 2, 30, 40)])
        self.assertEqual(rows[1]["counters"]["read_bytes"]["median"], 1)

    def test_zero_exit_without_completion_is_failure(self):
        with tempfile.TemporaryDirectory() as folder:
            row = scaling.bounded([sys.executable, "-c", "pass"], os.environ,
                                  Path(folder) / "probe.log", 2, False)
            self.assertEqual(row["status"], "failed")


if __name__ == "__main__":
    unittest.main()
