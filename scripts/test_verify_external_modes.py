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
    def __init__(self, plugin="External Effect Mono", availability=True, offset=0, math_mode=0, zoom=0):
        parameters = [number(name, value, "Toggle", lower=0, upper=1)
                      for name, value in (("pingMode", 0), ("scopeMode", 0), ("showImage", 1))]
        parameters += [number("latencyOffset", offset, "Integer", lower=-32000, upper=32000),
                       number("latencyTotal", 0.25, lower=-32000.0, upper=1000000.0)]
        parameters[3].update(tag=104, default=0, support={key: True for key in ("raw", "normalized", "text")})
        for parameter in parameters[3:]:
            parameter["normalized"] = (parameter["value"] - parameter["min"]) / (parameter["max"] - parameter["min"])
        for name in ("sendPort", "receivePort"):
            parameter = number(name, -1, "List", lower=0, upper=-1)
            parameter.update(writable=False, normalized=1, support={"raw": False})
            parameters.append(parameter)
        ping = number("pingAgain", 0, "Toggle", enabled=availability, lower=0, upper=1)
        ping.update(tag=112, default=0, support={key: availability for key in ("raw", "normalized", "text")})
        parameters.append(ping)
        for name, tag, value in (("mathMode", 114, math_mode), ("zoomOut", 115, zoom)):
            parameter = number(name, value, "Toggle", lower=0, upper=1)
            parameter.update(tag=tag, default=0, support={key: True for key in ("raw", "normalized", "text")})
            parameters.append(parameter)
        parameters += [number("other" + str(index), 0.25) for index in range(14)]
        super().__init__(parameters)
        self.target = {**TARGET, "plugin": plugin}
        self.live_class = CLASSES[plugin]
        self.faults = {}
        self.choice_fault = None
        self.read_fault = None
        self.delay_total = 0
        self.pending_total = None
        self.pending_total_value = 0.25
        self.original_offset = offset
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
        updated["tag"] = parameter["tag"]
        if parameter["max"] > parameter["min"]:
            updated["normalized"] = (value - parameter["min"]) / (parameter["max"] - parameter["min"])
        parameter.update(updated)

    def availability(self, enabled):
        self.find("pingAgain").update(enabled=enabled, writable=enabled,
                                      support={key: enabled for key in ("raw", "normalized", "text")},
                                      native_support={key: enabled for key in ("raw_number", "normalized", "display_text")})

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
            if fault not in ("ignore", "refuse"):
                if name == "pingMode":
                    mode = (value, 1 - value, 0)
                    self.availability(bool(value))
                elif name == "scopeMode":
                    mode = (1 - value, value, 0)
                elif name == "showImage":
                    mode = (0, 1 - value, value)
                elif name == "latencyOffset":
                    mode = None
                    self.update(name, value)
                    if self.delay_total:
                        self.pending_total = self.delay_total
                        self.pending_total_value = 0.25 if value == self.original_offset else 0.75
                    else:
                        self.update("latencyTotal", 0.25 if value == self.original_offset else 0.75)
                elif name in ("mathMode", "zoomOut"):
                    mode = None
                    self.update(name, value)
                    self.update("latencyOffset", 7)
                    self.update("latencyTotal", 0.75)
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
            elif fault == "wrong_availability":
                self.availability(not bool(value))
            elif fault == "mode_drift":
                self.update("showImage", 0)
                self.update("scopeMode", 1)
                self.update("pingMode", 0)
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
            elif fault == "refuse":
                outcome = "not_applied"
            self.active_page_reads = 0
            return self.receipt(result, outcome, {"code": "native_rejected"} if fault == "refuse" else None)
        if name == "plugin_parameters":
            self.active_page_reads += 1
            if self.pending_total is not None:
                if self.pending_total:
                    self.pending_total -= 1
                else:
                    self.update("latencyTotal", self.pending_total_value)
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
            ("pingMode", 1), ("pingMode", 0), ("pingMode", 1), ("showImage", 1), ("latencyOffset", 0),
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

    def test_coordinated_three_controls_restore_both_classes_and_availability_profiles(self):
        for index, (plugin, enabled) in enumerate((name, enabled) for name in CLASSES for enabled in (True, False)):
            with self.subTest(plugin=plugin, enabled=enabled):
                self.journal.close()
                self.journal = Journal(Path(self.temporary.name) / ("controls-" + str(index)))
                client = ExternalClient(plugin, availability=enabled)
                runner = self.runner(client)
                row = runner.run_controls(client.target)
                self.assertEqual(row["status"], "three_disconnected_controls_verified")
                self.assertEqual([case["parameter"] for case in row["cases"]], ["latencyOffset", "mathMode", "zoomOut"])
                self.assertTrue(all(case["control_verified"] and case["complete_provider_projection_restored"] for case in row["cases"]))
                self.assertEqual([(write["param"], write["value"]) for write in client.writes], [
                    ("latencyOffset", 1), ("latencyOffset", 0), ("showImage", 1),
                    ("mathMode", 1), ("mathMode", 0), ("showImage", 1), ("latencyOffset", 0),
                    ("zoomOut", 1), ("zoomOut", 0), ("showImage", 1), ("latencyOffset", 0)])
                self.assertEqual(len(list(self.journal.baselines.glob("*.json"))), 1)
                self.assertEqual(len([event for event in self.events("external_mode_observation")
                                      if event["stage"] == "control-changed"]), 6)
                self.assertEqual(runner.finish(), 0)
                self.assertFalse(runner.summary["hardware_calibration_verified"])
                self.assertFalse(runner.summary["audio_emission_verified"])
                self.assertFalse(runner.summary["all_writable_parameters_verified"])
                self.assertEqual(Journal.pending(self.journal.ledger_path), {})

    def test_controls_choose_adjacent_integer_at_offset_bounds_and_invert_captured_on_toggles(self):
        for offset, desired in ((-32000, -31999), (32000, 31999)):
            with self.subTest(offset=offset):
                self.journal.close()
                self.journal = Journal(Path(self.temporary.name) / ("bound-" + str(offset)))
                client = ExternalClient(offset=offset, math_mode=1, zoom=1)
                row = self.runner(client).run_controls(client.target)
                self.assertEqual(row["status"], "three_disconnected_controls_verified")
                self.assertEqual(client.writes[0]["value"], desired)
                self.assertEqual(client.writes[1]["value"], offset)
                self.assertEqual(client.writes[3]["param"], "mathMode")
                self.assertEqual(client.writes[3]["value"], 0)
                self.assertEqual(client.writes[7]["param"], "zoomOut")
                self.assertEqual(client.writes[7]["value"], 0)
                self.assertTrue(all(type(write["value"]) is int and "float_bits" not in write for write in client.writes))

    def test_controls_reject_fractional_float_carriers_or_wrong_metadata_before_any_write(self):
        faults = (("latencyOffset", "value", 0.5), ("mathMode", "value", 0.0),
                  ("zoomOut", "tag", 114), ("latencyOffset", "max", 32001),
                  ("mathMode", "default", 1), ("zoomOut", "enabled", False),
                  ("latencyOffset", "native_min", {"variant_tag": 2, "data": {"kind": "float", "value": -32000.0, "bits_hex": "C0DF400000000000"}}))
        for index, (name, field, value) in enumerate(faults):
            with self.subTest(name=name, field=field):
                self.journal.close()
                self.journal = Journal(Path(self.temporary.name) / ("control-metadata-" + str(index)))
                client = ExternalClient()
                if field == "value":
                    client.update(name, value)
                else:
                    client.find(name)[field] = value
                with self.assertRaises(AcceptanceFailure):
                    self.runner(client).run_controls(client.target)
                self.assertEqual(client.writes, [])

    def test_control_unknown_dispatch_own_restore_or_coordinated_restore_never_writes_again(self):
        for index, (step, fault) in enumerate(((1, "unknown"), (2, "timeout"), (3, "nested_unknown"),
                                               (4, "unknown"), (5, "unknown"), (6, "timeout"),
                                               (7, "nested_unknown"), (8, "timeout"), (9, "unknown"),
                                               (10, "unknown"), (11, "timeout"))):
            with self.subTest(step=step, fault=fault):
                self.journal.close()
                self.journal = Journal(Path(self.temporary.name) / ("control-unknown-" + str(index)))
                client = ExternalClient()
                client.faults[step] = fault
                with self.assertRaises(UnknownMutation):
                    self.runner(client).run_controls(client.target)
                self.assertEqual(len(client.writes), step)
                self.assertTrue(Journal.pending(self.journal.ledger_path))

    def test_control_unknown_read_after_change_never_starts_own_recovery(self):
        client = ExternalClient()
        def unknown(client, receipt):
            if client.writes:
                receipt["outcome"] = "partially_applied"
        client.read_fault = unknown
        with self.assertRaises(UnknownMutation):
            self.runner(client).run_controls(client.target)
        self.assertEqual(len(client.writes), 1)
        self.assertTrue(Journal.pending(self.journal.ledger_path))

    def test_controls_require_empty_choices_before_the_own_restore_setter(self):
        client = ExternalClient()
        def connected(client, page):
            if len(client.writes) == 1:
                page.update(reported_count=1, choices=[{"index": 0, "text": "route"}])
        client.choice_fault = connected
        with self.assertRaises(AcceptanceFailure):
            self.runner(client).run_controls(client.target)
        self.assertEqual(len(client.writes), 1)

    def test_control_metadata_or_context_change_after_dispatch_does_not_get_recovery(self):
        for index, (step, fault) in enumerate(((1, "schema"), (4, "document"), (5, "session"), (7, "instance"))):
            with self.subTest(step=step, fault=fault):
                self.journal.close()
                self.journal = Journal(Path(self.temporary.name) / ("control-context-" + str(index)))
                client = ExternalClient()
                client.faults[step] = fault
                with self.assertRaises(ContextChanged):
                    self.runner(client).run_controls(client.target)
                self.assertEqual(len(client.writes), step)

    def test_controls_cannot_accept_availability_schema_drift_as_an_allowed_latency_peer(self):
        client = ExternalClient()
        def availability_drift(client, receipt):
            if len(client.writes) == 1:
                parameter = next(p for p in receipt["result"]["params"] if p["name"] == "pingAgain")
                parameter.update(enabled=False, writable=False,
                                 support={key: False for key in ("raw", "normalized", "text")},
                                 native_support={key: False for key in ("raw_number", "normalized", "display_text")})
        client.read_fault = availability_drift
        with self.assertRaises(ContextChanged):
            self.runner(client).run_controls(client.target)
        self.assertEqual(len(client.writes), 1)

    def test_known_ignored_control_change_gets_one_finite_recovery_then_stops(self):
        client = ExternalClient()
        client.faults[1] = "ignore"
        runner = self.runner(client)
        with self.assertRaises(AcceptanceFailure):
            runner.run_controls(client.target)
        self.assertEqual([(w["param"], w["value"]) for w in client.writes], [("latencyOffset", 1), ("showImage", 1)])
        self.assertEqual(runner.summary["fixtures"][0]["cases"][0]["status"], "control_unconfirmed_restored")
        self.assertEqual(runner.counts["control_changed_readback_restored"], 0)
        self.assertEqual(Journal.pending(self.journal.ledger_path), {})
        self.assertEqual(runner.finish(), 2)

    def test_control_wrong_mode_peer_is_restored_once_but_cannot_be_a_verified_case(self):
        client = ExternalClient()
        client.faults[4] = "mode_drift"
        runner = self.runner(client)
        with self.assertRaises(AcceptanceFailure):
            runner.run_controls(client.target)
        self.assertEqual(len(client.writes), 7)
        self.assertEqual(runner.counts["control_changed_readback_restored"], 1)
        self.assertEqual(runner.summary["fixtures"][0]["cases"][1]["status"], "control_unconfirmed_restored")
        self.assertEqual(Journal.pending(self.journal.ledger_path), {})

    def test_control_unrelated_raw_normalized_or_display_drift_remains_unrestored(self):
        for index, fault in enumerate(("extra_raw", "extra_normalized", "extra_text")):
            with self.subTest(fault=fault):
                self.journal.close()
                self.journal = Journal(Path(self.temporary.name) / ("control-extra-" + str(index)))
                client = ExternalClient()
                client.faults[4] = fault
                runner = self.runner(client)
                with self.assertRaises(RestorationFailure):
                    runner.run_controls(client.target)
                self.assertEqual(len(client.writes), 7)
                self.assertEqual(runner.counts["control_changed_readback_restored"], 1)
                self.assertTrue(Journal.pending(self.journal.ledger_path))
                self.assertEqual(len(list(self.journal.baselines.glob("*.json"))), 1)

    def test_control_own_normalized_and_display_need_two_stable_fresh_readbacks(self):
        for index, field in enumerate(("normalized", "display")):
            with self.subTest(field=field):
                self.journal.close()
                self.journal = Journal(Path(self.temporary.name) / ("own-readback-" + str(index)))
                client = ExternalClient()
                def unstable(client, receipt):
                    if len(client.writes) == 1 and client.active_page_reads == 2:
                        parameter = next(p for p in receipt["result"]["params"] if p["name"] == "latencyOffset")
                        if field == "normalized":
                            parameter.update(normalized=0.75, native_normalized={"kind": "float32", "bits_hex": "3F400000"})
                        else:
                            parameter["text"] = "different"
                client.read_fault = unstable
                runner = self.runner(client)
                with self.assertRaises(AcceptanceFailure):
                    runner.run_controls(client.target)
                self.assertEqual(len(client.writes), 3)
                self.assertEqual(runner.counts["control_changed_readback_restored"], 0)
                self.assertEqual(Journal.pending(self.journal.ledger_path), {})

    def test_control_recovery_failure_never_retries_or_advances(self):
        for index, (step, fault, prerequisite) in enumerate(((2, "ignore", None), (5, "refuse", None),
                                                            (2, "mode_drift", None),
                                                            (6, "ignore", "mode_drift"), (7, "ignore", None))):
            with self.subTest(step=step, fault=fault):
                self.journal.close()
                self.journal = Journal(Path(self.temporary.name) / ("control-recovery-" + str(index)))
                client = ExternalClient()
                if prerequisite:
                    client.faults[4] = prerequisite
                client.faults[step] = fault
                with self.assertRaises(AcceptanceFailure):
                    self.runner(client).run_controls(client.target)
                self.assertEqual(len(client.writes), step)
                self.assertTrue(Journal.pending(self.journal.ledger_path))

    def test_failed_control_fixture_cannot_advance_to_another_valid_class_after_catching_error(self):
        client = ExternalClient()
        client.faults[1] = "unknown"
        runner = self.runner(client)
        with self.assertRaises(UnknownMutation):
            runner.run_controls(client.target)
        other = {**client.target, "plugin": "External Effect Stereo", "channel": "FSP Verify FX Other"}
        client.live_class = CLASSES["External Effect Stereo"]
        client.targets[other["channel"]] = ExternalClient("External Effect Stereo").parameters
        calls = len(client.calls)
        with self.assertRaises(AcceptanceFailure):
            runner.run_controls(other)
        self.assertEqual(len(client.writes), 1)
        self.assertEqual(len(client.calls), calls)

    def test_controls_cannot_mix_with_a_mode_workflow_or_replay_a_fixture(self):
        client = ExternalClient()
        runner = self.runner(client)
        runner.run_controls(client.target)
        writes = len(client.writes)
        for call in (runner.run_controls, runner.run_fixture):
            with self.assertRaises(AcceptanceFailure):
                call(client.target)
        self.assertEqual(len(client.writes), writes)

    def test_stereo_uses_exact_other_class(self):
        client = ExternalClient("External Effect Stereo")
        row = self.runner(client).run_fixture(client.target)
        self.assertEqual(row["status"], "six_disconnected_mode_transitions_verified")

    def test_both_captured_availability_profiles_restore_for_both_exact_classes(self):
        for index, (plugin, enabled) in enumerate((name, enabled) for name in CLASSES for enabled in (True, False)):
            with self.subTest(plugin=plugin, enabled=enabled):
                self.journal.close()
                self.journal = Journal(Path(self.temporary.name) / ("profile-" + str(index)))
                client = ExternalClient(plugin, availability=enabled)
                runner = self.runner(client)
                row = runner.run_fixture(client.target)
                self.assertEqual(row["status"], "six_disconnected_mode_transitions_verified")
                self.assertEqual(len(client.writes), 19)
                repair = [event for event in self.events("transaction_phase") if event["phase"] == "AvailabilityRestored"]
                self.assertEqual(len(repair), 1)
                self.assertIs(repair[0]["captured_enabled"], enabled)
                self.assertEqual(repair[0]["transaction"], "external-0-1" if enabled else "external-0-0")
                self.assertIs(client.find("pingAgain")["enabled"], enabled)
                self.assertTrue(all(w["param"] in ("pingMode", "scopeMode", "showImage", "latencyOffset") for w in client.writes))
                self.assertEqual(Journal.pending(self.journal.ledger_path), {})

    def test_actual_second_case_all_off_profile_is_observed_then_restored_exactly(self):
        client = ExternalClient()
        self.runner(client).run_fixture(client.target)
        observations = [e for e in self.events("external_mode_observation") if e["transaction"] == "external-0-1"]
        original_schema = next(e["projection"]["schema_sha256"] for e in observations if e["stage"] == "before-case")
        changed = [e for e in observations if e["stage"] == "changed"]
        self.assertEqual(len(changed), 2)
        self.assertTrue(all(e["projection"]["schema_sha256"] != original_schema for e in changed))
        restored = [e for e in observations if e["stage"] == "restored"]
        self.assertEqual(len(restored), 2)
        self.assertTrue(all(e["projection"]["schema_sha256"] == original_schema for e in restored))
        self.assertEqual([(w["param"], w["value"]) for w in client.writes[3:8]],
                         [("pingMode", 1), ("pingMode", 0), ("pingMode", 1), ("showImage", 1), ("latencyOffset", 0)])

    def test_availability_repair_unknown_refusal_or_ignored_callback_stops_without_image_write(self):
        for index, (enabled, step, fault) in enumerate((enabled, 6 if enabled else 2, fault)
                                                     for enabled in (True, False)
                                                     for fault in ("unknown", "timeout", "nested_unknown", "refuse", "ignore", "wrong_availability")):
            with self.subTest(enabled=enabled, fault=fault):
                self.journal.close()
                self.journal = Journal(Path(self.temporary.name) / ("repair-" + str(index)))
                client = ExternalClient(availability=enabled)
                client.faults[step] = fault
                with self.assertRaises(AcceptanceFailure):
                    self.runner(client).run_fixture(client.target)
                self.assertEqual(len(client.writes), step)
                self.assertEqual(client.writes[-1]["param"], "pingMode")
                self.assertEqual(client.writes[-1]["value"], int(enabled))
                self.assertTrue(Journal.pending(self.journal.ledger_path))

    def test_contradictory_profile_cannot_retrigger_an_unchanged_setup_value(self):
        for enabled, step in ((True, 1), (False, 6)):
            with self.subTest(enabled=enabled):
                self.journal.close()
                self.journal = Journal(Path(self.temporary.name) / ("contradiction-" + str(step)))
                client = ExternalClient(availability=enabled)
                client.faults[step] = "wrong_availability"
                with self.assertRaises(RestorationFailure):
                    self.runner(client).run_fixture(client.target)
                self.assertEqual(len(client.writes), step)
                self.assertEqual(client.find("pingMode")["value"], int(enabled))
                self.assertIs(client.find("pingAgain")["enabled"], not enabled)
                self.assertTrue(Journal.pending(self.journal.ledger_path))

    def test_repair_requires_complete_empty_hardware_choices(self):
        client = ExternalClient()
        def connected(client, page):
            if len(client.writes) == 5:
                page.update(reported_count=1, choices=[{"index": 0, "text": "route"}])
        client.choice_fault = connected
        with self.assertRaises(AcceptanceFailure):
            self.runner(client).run_fixture(client.target)
        self.assertEqual(len(client.writes), 5)
        self.assertTrue(Journal.pending(self.journal.ledger_path))

    def test_profile_lost_after_image_is_not_repaired_again_or_excluded(self):
        client = ExternalClient()
        def lost(client, _):
            if len(client.writes) == 7:
                client.availability(False)
        client.read_fault = lost
        runner = self.runner(client)
        with self.assertRaises(AcceptanceFailure):
            runner.run_fixture(client.target)
        self.assertEqual(len(client.writes), 7)
        self.assertEqual(runner.counts["mode_changed_readback_restored"], 1)
        self.assertEqual(len([e for e in self.events("transaction_phase") if e["phase"] == "AvailabilityRestored"]), 1)
        self.assertTrue(Journal.pending(self.journal.ledger_path))

    def test_only_the_consistent_authoritative_ping_availability_group_can_change(self):
        faults = (
            ("pingAgain", {"tag": 113}), ("pingAgain", {"native_kind": "Integer"}),
            ("pingAgain", {"min": -1}), ("pingAgain", {"max": 2}),
            ("pingAgain", {"default": 1}), ("pingAgain", {"readOnly": True}),
            ("pingAgain", {"enabled": True}), ("pingAgain", {"writable": True}),
            ("pingAgain", {"support": {"raw": False, "normalized": True, "text": False}}),
            ("pingAgain", {"native_support": {"raw_number": False, "normalized": False, "display_text": True}}),
            ("pingAgain", {"native_default": {"variant_tag": 2, "data": {"kind": "float", "value": 0, "bits_hex": "0000000000000000"}}}),
            ("pingAgain", {"title": "different"}),
            ("other0", {"enabled": False}), ("other0", {"default": 1}),
        )
        for index, (name, fields) in enumerate(faults):
            with self.subTest(name=name, fields=fields):
                self.journal.close()
                self.journal = Journal(Path(self.temporary.name) / ("metadata-" + str(index)))
                client = ExternalClient()
                def corrupt(client, receipt):
                    if len(client.writes) == 5:
                        next(p for p in receipt["result"]["params"] if p["name"] == name).update(copy.deepcopy(fields))
                client.read_fault = corrupt
                with self.assertRaises(ContextChanged):
                    self.runner(client).run_fixture(client.target)
                self.assertEqual(len(client.writes), 5)
                self.assertTrue(Journal.pending(self.journal.ledger_path))

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
        self.assertEqual(len(client.writes), 19)
        self.assertFalse(json.loads((output / "progress.json").read_text())["all_writable_parameters_verified"])

    def test_cli_controls_are_explicit_and_keep_the_mode_workflow_separate(self):
        client = ExternalClient()
        fixtures = Path(self.temporary.name) / "fixtures-controls.json"
        fixtures.write_text(json.dumps([client.target]))
        binary = Path(self.temporary.name) / "binary-controls"
        binary.write_bytes(b"synthetic")
        output = Path(self.temporary.name) / "cli-controls"
        with patch("verify_external_modes.McpClient", return_value=client), patch("verify_external_modes.time.sleep"), \
                redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
            self.assertEqual(main(["--fixtures", str(fixtures), "--scratch-title", "MCP Validation Scratch",
                                   "--binary", str(binary), "--output-dir", str(output),
                                   "--plugin", "External Effect Mono", "--run-controls"]), 0)
        self.assertTrue(client.closed)
        self.assertEqual(len(client.writes), 11)
        summary = json.loads((output / "progress.json").read_text())
        self.assertEqual(summary["mode"], "disconnected_external_controls")
        self.assertFalse(summary["all_writable_parameters_verified"])
        self.assertFalse(summary["audio_emission_verified"])


if __name__ == "__main__":
    unittest.main()
