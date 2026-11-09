"""Offline disconnected-mode safety checks. No DAW or native add-on is launched."""
import copy
from contextlib import redirect_stderr, redirect_stdout
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from test_verify_all_parameter_states import DigestSweepClient
from test_verify_all_parameters import TARGET, number
from verify_all_parameters import (
    AcceptanceFailure, ContextChanged, Journal, PRIVATE_ROOT, RestorationFailure,
    UnknownMutation, decode_float_bits,
)
from verify_external_modes import CLASSES, ExternalModeRunner, main


class ExternalClient(DigestSweepClient):
    def __init__(self, plugin="External Effect Mono"):
        parameters = [number(name, value, "Toggle", lower=0, upper=1)
                      for name, value in (("pingMode", 0), ("scopeMode", 0), ("showImage", 1))]
        parameters += [number("latencyOffset", 0, "Integer", lower=-32000, upper=32000),
                       number("latencyTotal", 0.25, lower=-32000.0, upper=1000000.0)]
        for parameter in parameters[3:]:
            parameter["normalized"] = (parameter["value"] - parameter["min"]) / (parameter["max"] - parameter["min"])
        for name in ("sendPort", "receivePort"):
            parameter = number(name, -1, "List", lower=0, upper=-1)
            parameter.update(writable=False, normalized=1, support={"raw": False})
            parameters.append(parameter)
        parameters += [number("other" + str(index), 0.25) for index in range(17)]
        super().__init__(parameters)
        self.target = {**TARGET, "plugin": plugin}
        self.live_class = CLASSES[plugin]
        self.faults = {}
        self.choice_fault = None
        self.read_fault = None
        self.delay_total = 0
        self.pending_total = None
        self.active_page_reads = 0
        self.closed = False

    def indexed(self, parameter, index):
        return {**self.description(parameter), "index": index}

    def find(self, name):
        return next(p for p in self.parameters if p["name"] == name)

    def update(self, name, value):
        parameter = self.find(name)
        updated = number(name, value, parameter["native_kind"], lower=parameter["min"], upper=parameter["max"])
        updated["support"] = copy.deepcopy(parameter["support"])
        updated["writable"] = parameter["writable"]
        if parameter["max"] > parameter["min"]:
            updated["normalized"] = (value - parameter["min"]) / (parameter["max"] - parameter["min"])
        parameter.update(updated)

    def call(self, name, args):
        if name == "plugin_parameter_choices":
            self.select_target(args)
            self.calls.append((name, copy.deepcopy(args)))
            result = {"ok": True, "param": args["param"], "instance": dict(self.instance),
                      "source": "controller", "reported_count": 0, "offset": 0,
                      "choices": [], "next_offset": None, "page_complete": True, "all_indices_read": True}
            if self.choice_fault:
                self.choice_fault(self, result)
            return self.receipt(result)
        if name == "plugin_parameter_set":
            self.select_target(args)
            self.calls.append((name, copy.deepcopy(args)))
            self.writes.append(copy.deepcopy(args))
            name, value = args["param"], args.get("value")
            if "float_bits" in args:
                value = decode_float_bits(args["float_bits"])
            before = self.description(self.find(name))
            fault = self.faults.get(len(self.writes))
            if fault != "ignore":
                if name == "pingMode":
                    mode = (value, 1 - value, 0)
                elif name == "scopeMode":
                    mode = (1 - value, value, 0)
                elif name == "showImage":
                    mode = (0, 1 - value, value)
                elif name == "latencyOffset":
                    mode = None
                    self.update(name, value)
                    if self.delay_total:
                        self.pending_total = self.delay_total
                    else:
                        self.update("latencyTotal", 0.25)
                else:
                    raise AssertionError("Forbidden write: " + name)
                if mode is not None:
                    for peer, selected in zip(("pingMode", "scopeMode", "showImage"), mode):
                        self.update(peer, selected)
                    if mode[0]:
                        self.update("latencyOffset", 7)
                        self.update("latencyTotal", 0.75)
            if fault == "extra_raw":
                self.update("other0", 0.5)
            elif fault == "extra_text":
                self.find("other0")["text"] = "different"
            elif fault == "extra_normalized":
                self.find("other0")["normalized"] = 0.5
            elif fault == "wrong_peer_mode":
                self.update("showImage", 1)
            elif fault == "wrong_own":
                self.update(name, 1 - value)
            elif fault == "schema":
                self.find("other0")["default"] = 0.75
            elif fault == "document":
                self.document = "test:2"
            elif fault == "session":
                self.session = "different-session"
            elif fault == "instance":
                self.instance["object_uid"] = "changed-instance"
            elif fault == "timeout":
                raise TimeoutError("Synthetic interrupted dispatch")
            result = {"before": before, "after": self.description(self.find(name)),
                      "instance": dict(self.instance), "source": "controller", "param": name}
            if fault == "nested_unknown":
                result["native_outcome"] = "outcome_unknown"
            elif fault == "nested_partial":
                result["native_outcome"] = "partially_applied"
            elif fault == "nested_malformed":
                result["native_outcome"] = {}
            elif fault == "false_after":
                result["after"] = before
            outcome = "outcome_unknown" if fault == "unknown" else "applied_verified"
            if fault == "missing_outcome":
                outcome = None
            elif fault == "malformed_outcome":
                outcome = {}
            self.active_page_reads = 0
            return self.receipt(result, outcome)
        if name == "plugin_parameters":
            self.active_page_reads += 1
            if self.pending_total is not None:
                if self.pending_total:
                    self.pending_total -= 1
                else:
                    self.update("latencyTotal", 0.25)
                    self.pending_total = None
            result = super().call(name, args)
            body = result["structuredContent"]["result"]
            body["parameter_discovery"]["source"] = "controller"
            if self.read_fault:
                self.read_fault(self, result["structuredContent"])
            return result
        if name in ("plugin_state", "plugin_action", "plugin_parameter_digest"):
            raise AssertionError("External mode verifier must use complete public pages and fixed scalar writes")
        return super().call(name, args)

    def close(self):
        self.closed = True


class ExternalModeTests(unittest.TestCase):
    def setUp(self):
        PRIVATE_ROOT.mkdir(exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(prefix="external-modes-unit-", dir=PRIVATE_ROOT)
        self.journal = Journal(Path(self.temporary.name) / "run")

    def tearDown(self):
        self.journal.close()
        self.temporary.cleanup()

    def runner(self, client):
        runner = ExternalModeRunner(client, self.journal, "MCP Validation Scratch", settle_interval=0)
        runner.initialize()
        return runner

    def events(self, event):
        return [row for line in self.journal.ledger_path.read_text().splitlines()
                if (row := json.loads(line))["event"] == event]

    def test_six_fixed_transitions_restore_one_immutable_full_baseline(self):
        client = ExternalClient()
        runner = self.runner(client)
        row = runner.run_fixture(client.target)
        self.assertEqual(row["status"], "six_disconnected_mode_transitions_verified")
        self.assertEqual([(r["parameter"], r["requested_mode"]) for r in row["cases"]],
                         [("pingMode", "setup"), ("pingMode", "scope"), ("scopeMode", "scope"),
                          ("scopeMode", "setup"), ("showImage", "scope"), ("showImage", "image")])
        self.assertTrue(all(r["transition_verified"] and r["complete_provider_projection_restored"] for r in row["cases"]))
        self.assertEqual([(w["param"], w["value"]) for w in client.writes], [
            ("pingMode", 1), ("showImage", 1), ("latencyOffset", 0),
            ("pingMode", 1), ("pingMode", 0), ("showImage", 1), ("latencyOffset", 0),
            ("scopeMode", 1), ("showImage", 1),
            ("scopeMode", 1), ("scopeMode", 0), ("showImage", 1), ("latencyOffset", 0),
            ("showImage", 0), ("showImage", 1),
            ("scopeMode", 1), ("showImage", 1), ("showImage", 1)])
        self.assertEqual(len(list(self.journal.baselines.glob("*.json"))), 1)
        self.assertEqual(len([e for e in self.events("external_mode_observation") if e["stage"] == "changed"]), 12)
        self.assertEqual(len([e for e in self.events("external_baseline_comparison")
                              if e["stage"] == "restored" and e["full_baseline_equal"]]), 12)
        self.assertEqual(Journal.pending(self.journal.ledger_path), {})
        self.assertEqual(runner.finish(), 0)
        for field in ("all_writable_parameters_verified", "hardware_route_verified", "hardware_calibration_verified",
                      "audio_emission_verified", "opaque_contents_verified", "atomic_snapshot"):
            self.assertFalse(runner.summary[field])

    def test_stereo_uses_exact_other_class(self):
        client = ExternalClient("External Effect Stereo")
        row = self.runner(client).run_fixture(client.target)
        self.assertEqual(row["status"], "six_disconnected_mode_transitions_verified")

    def test_wrong_class_or_provider_rejects_without_writes(self):
        client = ExternalClient()
        client.live_class = "not-the-registered-class"
        with self.assertRaises(AcceptanceFailure):
            self.runner(client).run_fixture(client.target)
        self.assertEqual(client.writes, [])

    def test_nonimage_start_rejects_without_preparation_write(self):
        client = ExternalClient()
        client.update("showImage", 0)
        client.update("scopeMode", 1)
        with self.assertRaises(AcceptanceFailure):
            self.runner(client).run_fixture(client.target)
        self.assertEqual(client.writes, [])

    def test_nonempty_choice_or_selected_route_never_writes(self):
        client = ExternalClient()
        client.choice_fault = lambda _, result: result.update(reported_count=1, choices=[{"index": 0, "text": "route"}])
        with self.assertRaises(AcceptanceFailure):
            self.runner(client).run_fixture(client.target)
        self.assertEqual(client.writes, [])

    def test_selected_route_rejects_without_writes(self):
        client = ExternalClient()
        client.update("sendPort", 0)
        with self.assertRaises(ContextChanged):
            self.runner(client).run_fixture(client.target)
        self.assertEqual(client.writes, [])

    def test_unknown_in_change_staging_or_recovery_stops_with_pending_ledger(self):
        for fault, step in (("unknown", 1), ("timeout", 4), ("unknown", 5),
                            ("nested_unknown", 2), ("unknown", 3),
                            ("missing_outcome", 1), ("malformed_outcome", 1),
                            ("nested_partial", 2), ("nested_malformed", 4)):
            with self.subTest(fault=fault, step=step):
                self.journal.close()
                self.journal = Journal(Path(self.temporary.name) / (fault + str(step)))
                client = ExternalClient()
                client.faults[step] = fault
                with self.assertRaises(UnknownMutation):
                    self.runner(client).run_fixture(client.target)
                self.assertEqual(len(client.writes), step)
                self.assertTrue(Journal.pending(self.journal.ledger_path))
                self.assertEqual(self.events("transaction_phase")[-1]["phase"], "UnknownAfterDispatch")

    def test_unknown_during_failed_readback_recovery_stops_without_offset(self):
        client = ExternalClient()
        client.faults = {1: "ignore", 2: "unknown"}
        with self.assertRaises(UnknownMutation):
            self.runner(client).run_fixture(client.target)
        self.assertEqual(len(client.writes), 2)
        self.assertTrue(Journal.pending(self.journal.ledger_path))

    def test_context_loss_in_change_or_recovery_never_writes_again(self):
        for fault, step in (("document", 1), ("session", 2), ("instance", 3)):
            with self.subTest(fault=fault):
                self.journal.close()
                self.journal = Journal(Path(self.temporary.name) / fault)
                client = ExternalClient()
                client.faults[step] = fault
                with self.assertRaises(ContextChanged):
                    self.runner(client).run_fixture(client.target)
                self.assertEqual(len(client.writes), step)
                self.assertTrue(Journal.pending(self.journal.ledger_path))

    def test_known_failed_own_readback_has_one_finite_recovery_then_stops(self):
        client = ExternalClient()
        client.faults[1] = "ignore"
        runner = self.runner(client)
        with self.assertRaises(AcceptanceFailure):
            runner.run_fixture(client.target)
        self.assertEqual([(w["param"], w["value"]) for w in client.writes], [("pingMode", 1), ("showImage", 1)])
        self.assertEqual(runner.summary["fixtures"][0]["cases"][0]["status"], "transition_unconfirmed_restored")
        self.assertEqual(Journal.pending(self.journal.ledger_path), {})
        self.assertEqual(runner.finish(), 2)

    def test_known_bad_peer_mode_is_recovered_but_cannot_pass(self):
        client = ExternalClient()
        client.faults[1] = "wrong_peer_mode"
        runner = self.runner(client)
        with self.assertRaises(AcceptanceFailure):
            runner.run_fixture(client.target)
        self.assertEqual(len(client.writes), 3)
        self.assertEqual(runner.summary["fixtures"][0]["status"], "failed")
        self.assertEqual(Journal.pending(self.journal.ledger_path), {})

    def test_raw_normalized_and_display_mismatches_never_become_new_baseline(self):
        for fault in ("extra_raw", "extra_normalized", "extra_text"):
            with self.subTest(fault=fault):
                self.journal.close()
                self.journal = Journal(Path(self.temporary.name) / fault)
                client = ExternalClient()
                client.faults[1] = fault
                with self.assertRaises(RestorationFailure):
                    self.runner(client).run_fixture(client.target)
                self.assertEqual(len(client.writes), 3)
                self.assertTrue(Journal.pending(self.journal.ledger_path))
                self.assertEqual(len(list(self.journal.baselines.glob("*.json"))), 1)
                self.assertTrue(any(e["full_baseline_equal"] is False for e in self.events("external_baseline_comparison")))

    def test_false_restoration_setter_cannot_replace_full_read_verification(self):
        client = ExternalClient()
        client.faults[2] = "ignore"
        with self.assertRaises(AcceptanceFailure):
            self.runner(client).run_fixture(client.target)
        self.assertEqual(len(client.writes), 2)
        self.assertTrue(Journal.pending(self.journal.ledger_path))

    def test_delayed_latency_total_has_only_six_reads_and_needs_two_consecutive_matches(self):
        client = ExternalClient()
        client.delay_total = 4
        row = self.runner(client).run_fixture(client.target)
        self.assertEqual(row["status"], "six_disconnected_mode_transitions_verified")
        comparisons = [e for e in self.events("external_baseline_comparison")
                       if e["stage"] == "restored" and e["transaction"] == "external-0-0"]
        self.assertEqual([e["full_baseline_equal"] for e in comparisons], [False, False, False, False, True, True])

    def test_unsettled_total_is_not_written_or_excluded(self):
        client = ExternalClient()
        client.delay_total = 100
        with self.assertRaises(RestorationFailure):
            self.runner(client).run_fixture(client.target)
        comparisons = [e for e in self.events("external_baseline_comparison") if e["stage"] == "restored"]
        self.assertEqual(len(comparisons), 6)
        self.assertTrue(all(e["full_baseline_equal"] is False for e in comparisons))
        self.assertEqual(len(client.writes), 3)
        self.assertNotIn("latencyTotal", [w["param"] for w in client.writes])
        self.assertTrue(Journal.pending(self.journal.ledger_path))

    def test_schema_change_during_readback_aborts_without_recovery(self):
        client = ExternalClient()
        client.faults[1] = "schema"
        with self.assertRaises(ContextChanged):
            self.runner(client).run_fixture(client.target)
        self.assertEqual(len(client.writes), 1)

    def test_unknown_read_after_known_change_does_not_start_recovery(self):
        client = ExternalClient()
        def unknown(client, receipt):
            if client.writes:
                receipt["outcome"] = "outcome_unknown"
        client.read_fault = unknown
        with self.assertRaises(UnknownMutation):
            self.runner(client).run_fixture(client.target)
        self.assertEqual(len(client.writes), 1)

    def test_unobserved_pages_after_change_cannot_approve_recovery_or_any_case(self):
        faults = (None, {}, "partially_applied", "unexpected_read_status",
                  "applied_verified", "applied_unverified", [], True, "missing")
        for index, fault in enumerate(faults):
            with self.subTest(fault=fault):
                self.journal.close()
                self.journal = Journal(Path(self.temporary.name) / ("bad-read-" + str(index)))
                client = ExternalClient()
                def corrupt(client, receipt):
                    if client.writes:
                        if fault == "missing":
                            receipt.pop("outcome", None)
                        else:
                            receipt["outcome"] = copy.deepcopy(fault)
                client.read_fault = corrupt
                runner = self.runner(client)
                with self.assertRaises(UnknownMutation):
                    runner.run_fixture(client.target)
                self.assertEqual(len(client.writes), 1)
                self.assertEqual(runner.counts["mode_changed_readback_restored"], 0)
                self.assertFalse(runner.summary["fixtures"][0]["cases"][0].get("transition_verified", False))
                self.assertEqual(runner.finish(), 2)
                self.assertTrue(Journal.pending(self.journal.ledger_path))

    def test_scratch_mute_loss_after_known_change_does_not_start_recovery(self):
        client = ExternalClient()
        def unmute(client, _):
            if client.writes:
                client.muted = False
        client.read_fault = unmute
        with self.assertRaises(ContextChanged):
            self.runner(client).run_fixture(client.target)
        self.assertEqual(len(client.writes), 1)
        self.assertTrue(Journal.pending(self.journal.ledger_path))

    def test_duplicate_fixture_is_not_replayed(self):
        client = ExternalClient()
        runner = self.runner(client)
        runner.run_fixture(client.target)
        count = len(client.writes)
        with self.assertRaises(AcceptanceFailure):
            runner.run_fixture(client.target)
        self.assertEqual(len(client.writes), count)

    def test_cli_runs_only_selected_finite_registered_fixture(self):
        client = ExternalClient()
        fixtures = Path(self.temporary.name) / "fixtures.json"
        fixtures.write_text(json.dumps([client.target]))
        binary = Path(self.temporary.name) / "binary"
        binary.write_bytes(b"synthetic")
        output = Path(self.temporary.name) / "cli-run"
        with patch("verify_external_modes.McpClient", return_value=client), patch("verify_external_modes.time.sleep"), \
                redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
            self.assertEqual(main(["--fixtures", str(fixtures), "--scratch-title", "MCP Validation Scratch",
                                   "--binary", str(binary), "--output-dir", str(output),
                                   "--plugin", "External Effect Mono", "--run-modes"]), 0)
        self.assertTrue(client.closed)
        self.assertEqual(len(client.writes), 18)
        self.assertFalse(json.loads((output / "progress.json").read_text())["all_writable_parameters_verified"])


if __name__ == "__main__":
    unittest.main()
