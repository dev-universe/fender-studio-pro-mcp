#!/usr/bin/env python3
"""Offline transaction and safety checks. No DAW or native module is launched."""
import copy
import json
import math
from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import patch

from parameter_digest import HASH_FIELDS, project_snapshot

from verify_all_parameters import (AcceptanceFailure, ContextChanged, DependencyFailure,
                                   Journal, PRIVATE_ROOT, RestorationFailure, Runner,
                                   UnknownMutation, candidate, classification, decode_float_bits,
                                   value_identity)


TARGET = {"plugin": "Fixture Effect", "channel": "FSP Verify FX Fixture", "slot": 0}
INSTANCE = {"object_uid": "12345678-1234-1234-1234-123456789ABC",
            "class_uid": "87654321-4321-4321-4321-CBA987654321"}


def number(name, value, kind="Float", readonly=False, enabled=True, lower=0.0, upper=1.0):
    def native(value):
        data = {"kind": "integer", "value": value} if type(value) is int else {
            "kind": "float", "value": value, "bits_hex": struct.pack(">d", value).hex().upper()}
        return {"variant_tag": 1 if type(value) is int else 2, "data": data}

    parameter = {"name": name, "tag": 10, "native_kind": kind, "value": value, "min": lower, "max": upper,
            "normalized": value, "text": str(value), "readOnly": readonly, "enabled": enabled,
            "writable": not readonly and enabled, "support": {"raw": not readonly and enabled},
            "native_value": native(value),
            "display_parser": "PlainNumeric"}
    if kind in ("Integer", "List"):
        parameter.update(native_min=native(lower), native_max=native(upper))
    return parameter


def normalized_float(value, normalized, kind="Float"):
    parameter = number("gain", value, kind=kind, lower=0.0, upper=1.0)
    parameter["normalized"] = normalized
    parameter["support"]["normalized"] = True
    return parameter


class DeviceClient:
    def __init__(self, parameters=None):
        self.parameters = copy.deepcopy(parameters or [number("gain", 0.25)])
        self.document, self.session = "test:1", "test"
        self.instance = dict(INSTANCE)
        self.calls, self.writes = [], []
        self.muted = True
        self.ignore_change = self.lie_about_change = False
        self.fail_restore = self.lie_about_restore = False
        self.unknown_change = self.change_document = False
        self.side_effect = False
        self.replace_on_page = None
        self.pages = 0
        self.read_only_drift = False
        self.read_only_access_after = "unchanged"
        self.post_change_readings = None
        self.post_change_read_count = 0
        self.setter_normalized = None

    def receipt(self, result, outcome="observed", error=None):
        receipt = {"document": self.document, "session": self.session, "result": result, "outcome": outcome}
        if error:
            receipt["error"] = error
        return {"structuredContent": receipt}

    def call(self, name, args):
        self.calls.append((name, copy.deepcopy(args)))
        if name == "host_status":
            return self.receipt({"title": "MCP Validation Scratch", "open": True})
        if name == "operation_run":
            operation = args["operation"]
            if operation == "song":
                return self.receipt({"title": "MCP Validation Scratch", "transport": {"playing": False, "recording": False}})
            if operation == "tracks":
                return self.receipt([{"name": TARGET["channel"], "channel": TARGET["channel"], "mediaType": "Audio",
                                      "eventCount": 0, "events": []}])
            if operation == "channels":
                return self.receipt([{"label": TARGET["channel"], "mute": self.muted, "recordArmed": False, "monitor": False}])
            raise AssertionError(operation)
        if name == "plugin_parameters":
            if args.get("names"):
                if self.post_change_readings and len(self.writes) == 1:
                    reading = self.post_change_readings[min(self.post_change_read_count, len(self.post_change_readings) - 1)]
                    self.parameters[0].update(copy.deepcopy(reading))
                    self.post_change_read_count += 1
                params = [copy.deepcopy(p) for p in self.parameters if p["name"] in args["names"]]
                return self.receipt({"params": params, "missing": [n for n in args["names"] if n not in {p["name"] for p in params}]})
            self.pages += 1
            if self.read_only_drift and self.pages > 1:
                parameter = self.parameters[1]
                parameter.update(number(parameter["name"], 0.99, parameter["native_kind"],
                                        readonly=parameter["readOnly"], enabled=parameter["enabled"],
                                        lower=parameter["min"], upper=parameter["max"]))
                if self.read_only_access_after is None:
                    parameter.pop("access_metadata_complete", None)
                elif self.read_only_access_after != "unchanged":
                    parameter["access_metadata_complete"] = self.read_only_access_after
            if self.replace_on_page == self.pages:
                self.instance["object_uid"] = "ABCDEFAB-1234-1234-1234-123456789ABC"
            offset = args["offset"]
            end = min(offset + args["limit"], len(self.parameters))
            params = [{**copy.deepcopy(p), "native": {"index": i, "tag": p["tag"], "name": p["name"]}}
                      for i, p in enumerate(self.parameters) if offset <= i < end]
            return self.receipt({"params": params, "missing": [], "instance": dict(self.instance), "reported_count": len(self.parameters),
                                 "offset": offset, "next_offset": end if end < len(self.parameters) else None,
                                 "page_complete": True, "failed_indices": [],
                                 "parameter_discovery": {"source": "dsp", "completeness": "current_provider"}})
        if name == "plugin_parameter_set":
            self.writes.append(copy.deepcopy(args))
            self.assert_source(args)
            parameter = next(p for p in self.parameters if p["name"] == args["param"])
            before = copy.deepcopy(parameter)
            requested = decode_float_bits(args["float_bits"]) if "float_bits" in args else args["value"]
            updated = number(parameter["name"], requested, parameter["native_kind"],
                             lower=parameter["min"], upper=parameter["max"])
            updated["support"] = copy.deepcopy(parameter["support"])
            if parameter["max"] > parameter["min"]:
                updated["normalized"] = (requested - parameter["min"]) / (parameter["max"] - parameter["min"])
            restore = len(self.writes) % 2 == 0
            if not restore and self.setter_normalized is not None:
                updated["normalized"] = self.setter_normalized
            ignored = self.fail_restore if restore else self.ignore_change
            if not ignored:
                parameter.update(updated)
            if not restore and self.side_effect:
                self.parameters[1].update(number(self.parameters[1]["name"], 0.99))
            if not restore and self.change_document:
                self.document = "test:2"
            if not restore and self.unknown_change:
                raise TimeoutError("test transport interruption after dispatch")
            reported_after = copy.deepcopy(parameter)
            if (restore and self.lie_about_restore) or (not restore and self.lie_about_change):
                reported_after.update(updated)
            return self.receipt({"before": before, "after": reported_after, "source": "dsp", "instance": dict(self.instance)},
                                "applied_verified")
        raise AssertionError(name)

    def assert_source(self, args):
        if args.get("source") != "dsp" or args.get("expected_instance") != INSTANCE["object_uid"]:
            raise AssertionError("Write did not bind the explicit native source and instance")


class ReceiptClient:
    def __init__(self, receipt):
        self.receipt = receipt

    def call(self, name, args):
        return {"structuredContent": copy.deepcopy(self.receipt)}


class AcceptanceTests(unittest.TestCase):
    def setUp(self):
        PRIVATE_ROOT.mkdir(exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(prefix="runner-unit-", dir=PRIVATE_ROOT)
        self.directory = Path(self.temporary.name) / "run"
        self.journal = Journal(self.directory)

    def tearDown(self):
        if not self.journal.stream.closed:
            self.journal.close()
        self.temporary.cleanup()

    def runner(self, client, read_only_observations=False):
        runner = Runner(client, self.journal, "MCP Validation Scratch", block_size=1,
                        read_only_observations=read_only_observations)
        runner.initialize()
        return runner

    def events(self):
        return [json.loads(line) for line in self.journal.ledger_path.read_text().splitlines()]

    def changed_attempts(self):
        return [event["attempt"] for event in self.events() if event["event"] == "changed_readback_attempt"]

    def test_changed_value_and_independent_restore_are_required_for_a_pass(self):
        client = DeviceClient()
        runner = self.runner(client)
        runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual(client.parameters[0]["value"], 0.25)
        self.assertEqual(runner.counts["changed_readback_restored"], 1)
        self.assertEqual(len(client.writes), 2)
        self.assertEqual(client.writes[1]["float_bits"], "3FD0000000000000")
        self.assertNotIn("value", client.writes[1])
        self.assertEqual(Journal.pending(self.journal.ledger_path), {})
        phases = [event["phase"] for event in self.events() if event["event"] == "transaction_phase"]
        self.assertIn("ChangedReadbackConfirmed", phases)
        self.assertIn("RestoreReadbackConfirmed", phases)
        self.assertEqual(phases[-1], "StateConfirmed")

    def test_ignored_write_with_a_claimed_success_is_not_a_changed_pass(self):
        client = DeviceClient()
        client.ignore_change = True
        runner = self.runner(client)
        runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual(runner.counts["changed_readback_restored"], 0)
        self.assertEqual(runner.counts["change_failed_restored"], 1)
        self.assertEqual(client.parameters[0]["value"], 0.25)

    def test_setter_self_report_does_not_replace_independent_changed_readback(self):
        client = DeviceClient()
        client.ignore_change = client.lie_about_change = True
        runner = self.runner(client)
        runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual(runner.counts["changed_readback_restored"], 0)
        self.assertEqual(runner.counts["change_failed_restored"], 1)

    def test_restore_failure_stops_before_another_parameter_can_be_changed(self):
        client = DeviceClient([number("gain", 0.25), number("mix", 0.5)])
        client.fail_restore = True
        runner = self.runner(client)
        with self.assertRaises(RestorationFailure):
            runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual([write["param"] for write in client.writes], ["gain", "gain"])
        self.assertEqual(runner.counts["changed_readback_restored"], 0)
        self.assertIn("RestorationFailed", Journal.pending(self.journal.ledger_path).values())

    def test_restore_self_report_does_not_replace_independent_restore_readback(self):
        client = DeviceClient()
        client.fail_restore = client.lie_about_restore = True
        runner = self.runner(client)
        with self.assertRaises(RestorationFailure):
            runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertNotEqual(client.parameters[0]["value"], 0.25)
        self.assertEqual(runner.counts["changed_readback_restored"], 0)

    def test_timeout_after_dispatch_never_triggers_a_blind_restore(self):
        client = DeviceClient()
        client.unknown_change = True
        runner = self.runner(client)
        with self.assertRaises(UnknownMutation):
            runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual(len(client.writes), 1)
        self.assertIn("UnknownAfterDispatch", Journal.pending(self.journal.ledger_path).values())

    def test_document_change_after_dispatch_never_restores_into_new_context(self):
        client = DeviceClient()
        client.change_document = True
        runner = self.runner(client)
        with self.assertRaises(ContextChanged):
            runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual(len(client.writes), 1)

    def test_provider_replacement_between_pages_fails_before_any_write(self):
        client = DeviceClient([number(f"gain{i}", 0.25) for i in range(65)])
        client.replace_on_page = 2
        runner = self.runner(client)
        with self.assertRaises(ContextChanged):
            runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual(client.writes, [])

    def test_other_parameter_drift_invalidates_block_before_further_writes(self):
        client = DeviceClient([number("gain", 0.25), number("mix", 0.5)])
        client.side_effect = True
        runner = self.runner(client)
        with self.assertRaises(DependencyFailure):
            runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual([write["param"] for write in client.writes], ["gain", "gain"])
        self.assertEqual(runner.counts["changed_readback_restored"], 0)

    def test_default_comparison_remains_strict_for_proven_read_only_drift(self):
        meter = {**number("unusual observation name", 0.0, readonly=True), "access_metadata_complete": True}
        client = DeviceClient([number("gain", 0.25), meter])
        client.read_only_drift = True
        runner = self.runner(client)
        with self.assertRaises(DependencyFailure):
            runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual(runner.counts["changed_readback_restored"], 0)
        self.assertEqual(runner.summary["read_only_values"], "exact_comparison")
        comparison = next(event for event in self.events() if event["event"] == "block_comparison")
        self.assertEqual(comparison["changed_parameters"], ["unusual observation name"])
        self.assertEqual(comparison["read_only_changed"], [])

    def test_opt_in_reports_proven_read_only_drift_without_claiming_its_restoration(self):
        meter = {**number("unusual observation name", 0.0, readonly=True), "access_metadata_complete": True}
        client = DeviceClient([number("gain", 0.25), meter])
        client.read_only_drift = True
        runner = self.runner(client, read_only_observations=True)
        runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual(runner.counts["changed_readback_restored"], 1)
        self.assertEqual(runner.counts["read_only_value_change_observations"], 1)
        self.assertEqual(runner.counts["category_numeric_scalar"], 1)
        self.assertEqual(runner.counts["category_read_only"], 1)
        self.assertEqual(runner.summary["read_only_values"], "observed_not_restored")
        self.assertFalse(runner.summary["all_writable_parameters_verified"])
        comparison = next(event for event in self.events() if event["event"] == "block_comparison")
        self.assertEqual(comparison["changed_parameters"], [])
        self.assertEqual(comparison["read_only_changed"], ["unusual observation name"])
        self.assertEqual(comparison["read_only_values"], "observed_not_restored")
        self.assertTrue(all(write["param"] == "gain" for write in client.writes))

    def test_read_only_observations_require_complete_authoritative_access_on_both_reads(self):
        for before, after in ((False, True), (True, False), (None, True), (True, None)):
            with self.subTest(before=before, after=after):
                directory = Path(self.temporary.name) / ("incomplete-" + str(before) + "-" + str(after))
                journal = Journal(directory)
                try:
                    meter = number("observation", 0.0, readonly=True)
                    if before is not None:
                        meter["access_metadata_complete"] = before
                    client = DeviceClient([number("gain", 0.25), meter])
                    client.read_only_drift = True
                    client.read_only_access_after = after
                    runner = Runner(client, journal, "MCP Validation Scratch", block_size=1,
                                    read_only_observations=True)
                    runner.initialize()
                    with self.assertRaises(DependencyFailure):
                        runner.run_fixture(TARGET, write_roundtrips=True)
                    self.assertEqual(runner.counts["changed_readback_restored"], 0)
                    self.assertEqual(runner.counts["read_only_value_change_observations"], 0)
                finally:
                    journal.close()

    def test_read_only_observation_option_never_excludes_a_writable_dependency(self):
        client = DeviceClient([number("gain", 0.25), {**number("meter", 0.5), "access_metadata_complete": True}])
        client.side_effect = True
        runner = self.runner(client, read_only_observations=True)
        with self.assertRaises(DependencyFailure):
            runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual([write["param"] for write in client.writes], ["gain", "gain"])
        self.assertEqual(runner.counts["changed_readback_restored"], 0)
        self.assertEqual(runner.counts["read_only_value_change_observations"], 0)

    def test_two_exact_stable_float_reads_can_confirm_normalized_quantized_settlement(self):
        normalized = 0.37
        client = DeviceClient([normalized_float(0.75, 0.75)])
        client.setter_normalized = normalized
        settled = normalized_float(math.nextafter(0.37, math.inf), normalized)
        client.post_change_readings = [settled, settled]
        runner = self.runner(client)
        runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual(runner.counts["changed_readback_restored"], 1)
        self.assertEqual(self.changed_attempts(), [1, 2])
        self.assertEqual(client.parameters[0]["value"], 0.75)
        result = next(event for event in self.events() if event["event"] == "parameter_result")
        self.assertTrue(result["quantized"])
        self.assertEqual(result["settlement"], {"fresh_reads": 2, "exact_raw_stable": True,
                                               "quantized_settlement": True, "evidence": "exact_native_normalized",
                                               "exact_normalized_matches_setter": True})

    def test_normalized_settlement_mismatch_fails_on_the_first_fresh_read(self):
        client = DeviceClient([normalized_float(0.75, 0.75)])
        client.setter_normalized = 0.37
        client.post_change_readings = [normalized_float(math.nextafter(0.37, math.inf), 0.38)]
        runner = self.runner(client)
        runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual(runner.counts["changed_readback_restored"], 0)
        self.assertEqual(runner.counts["change_failed_restored"], 1)
        self.assertEqual(self.changed_attempts(), [1])
        self.assertEqual(client.parameters[0]["value"], 0.75)

    def test_two_unstable_float_raw_reads_are_not_retried_until_they_look_successful(self):
        normalized = 0.37
        client = DeviceClient([normalized_float(0.75, 0.75)])
        client.setter_normalized = normalized
        client.post_change_readings = [normalized_float(math.nextafter(0.37, math.inf), normalized),
                                       normalized_float(math.nextafter(math.nextafter(0.37, math.inf), math.inf), normalized)]
        runner = self.runner(client)
        runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual(runner.counts["changed_readback_restored"], 0)
        self.assertEqual(runner.counts["change_failed_restored"], 1)
        self.assertEqual(self.changed_attempts(), [1, 2])

    def test_second_normalized_read_must_remain_exact_even_when_raw_is_stable(self):
        normalized = 0.37
        client = DeviceClient([normalized_float(0.75, 0.75)])
        client.setter_normalized = normalized
        client.post_change_readings = [normalized_float(math.nextafter(0.37, math.inf), normalized),
                                       normalized_float(math.nextafter(0.37, math.inf), math.nextafter(normalized, math.inf))]
        runner = self.runner(client)
        runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual(runner.counts["changed_readback_restored"], 0)
        self.assertEqual(runner.counts["change_failed_restored"], 1)
        self.assertEqual(self.changed_attempts(), [1, 2])

    def test_settlement_never_turns_ignored_semantic_baseline_drift_into_a_changed_control(self):
        client = DeviceClient([normalized_float(0.75, 0.75)])
        client.ignore_change = True
        client.post_change_readings = [normalized_float(math.nextafter(0.75, math.inf), 0.75)]
        runner = self.runner(client)
        runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual(runner.counts["changed_readback_restored"], 0)
        self.assertEqual(runner.counts["change_failed_restored"], 1)
        self.assertEqual(self.changed_attempts(), [1])
        self.assertEqual(client.parameters[0]["value"], 0.75)

    def test_raw_float_list_cannot_use_the_float_normalized_settlement_exception(self):
        normalized = 0.37
        client = DeviceClient([normalized_float(1.0, 1.0, kind="List")])
        client.setter_normalized = normalized
        client.post_change_readings = [normalized_float(math.nextafter(0.37, math.inf), normalized, kind="List")]
        runner = self.runner(client)
        runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual(runner.counts["changed_readback_restored"], 0)
        self.assertEqual(runner.counts["change_failed_restored"], 1)
        self.assertEqual(self.changed_attempts(), [1])

    def test_all_indices_are_audited_and_ineligible_controls_are_not_written(self):
        client = DeviceClient([number("meter", 0.0, readonly=True),
                               number("disabled", 0.25, enabled=False),
                               number("fixed", 1.0, lower=1.0, upper=1.0)])
        runner = self.runner(client)
        runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual(runner.counts["records_audited"], 3)
        self.assertEqual(runner.counts["category_read_only"], 1)
        self.assertEqual(runner.counts["category_disabled_dependency"], 1)
        self.assertEqual(runner.counts["category_constant_range"], 1)
        self.assertEqual(client.writes, [])
        self.assertFalse(runner.summary["all_writable_parameters_verified"])

    def test_model_selector_requires_enclosing_state_restore_even_when_numeric(self):
        parameter = number("opt.eqmodel", 0, kind="List", lower=0, upper=2)
        self.assertEqual(classification("Fat Channel", parameter), "model_state_restore_required")

    def test_vocal_tune_scale_requires_capsule_restoration_without_classifying_unrelated_scales(self):
        scale = number("scale", 15.0, kind="List", lower=0.0, upper=15.0)
        self.assertEqual(classification("Vocal Tune", scale), "model_state_restore_required")
        self.assertEqual(classification("Fixture Effect", scale), "numeric_scalar")
        self.assertEqual(classification("Vocal Tune", {**scale, "name": "root"}), "numeric_scalar")
        self.assertEqual(classification("Vocal Tune", {**scale, "readOnly": True}), "read_only")

    def test_mustang_and_rumble_amp_model_float_facades_are_not_generic_scalars(self):
        model = number("global.ampmodel", 0.0, kind="Float", lower=0.0, upper=1.0)
        for plugin in ("Mustang Native", "Rumble Native"):
            self.assertEqual(classification(plugin, model), "model_state_restore_required")
            self.assertEqual(classification(plugin, {**model, "name": "global.gain"}), "numeric_scalar")
            self.assertEqual(classification(plugin, {**model, "name": "global.ampmodelOther"}), "numeric_scalar")
            self.assertEqual(classification(plugin, {**model, "readOnly": True}), "read_only")
            self.assertEqual(classification(plugin, {**model, "enabled": False}), "disabled_dependency")
        self.assertEqual(classification("Fixture Effect", model), "numeric_scalar")
        self.assertEqual(model["native_kind"], "Float")

    def test_source_backed_pulse_actions_are_not_scalar_roundtrips(self):
        actions = {"Expander": ["swapfreqs"], "Gate": ["swapfreqs"],
                   "Compressor": ["studiocomp.swapfreqs", "studiocomp.resetmin"],
                   "Limiter": ["resetmin"]}
        for plugin, names in actions.items():
            for name in names:
                parameter = number(name, 0, kind="Toggle", lower=0, upper=1)
                self.assertEqual(classification(plugin, parameter), "dedicated_action_required")
                self.assertEqual(classification("Fixture Effect", parameter), "numeric_scalar")

    def test_exact_external_coordinated_roles_require_state_restoration(self):
        for plugin in ("External Effect Mono", "External Effect Stereo"):
            for name in ("scopeMode", "showImage", "latencyOffset", "mathMode"):
                kind = "Integer" if name == "latencyOffset" else "Toggle"
                parameter = number(name, 0, kind=kind, lower=0, upper=2)
                self.assertEqual(classification(plugin, parameter), "state_restore_required")
                self.assertEqual(classification(plugin, {**parameter, "readOnly": True}), "read_only")
                self.assertEqual(classification(plugin, {**parameter, "enabled": False}), "disabled_dependency")
                self.assertEqual(classification(plugin, {**parameter, "name": name + "Other"}), "numeric_scalar")
                self.assertEqual(classification("Fixture Effect", parameter), "numeric_scalar")
                self.assertEqual(classification("External Effect Custom", parameter), "numeric_scalar")

    def test_external_coordinated_roles_remain_inventoried_without_generic_writes(self):
        class ExternalInventoryClient(DeviceClient):
            def call(self, name, args):
                if name == "plugin_editor_open":
                    self.calls.append((name, copy.deepcopy(args)))
                    return self.receipt({"opened": True}, "applied_unverified")
                return super().call(name, args)

        client = ExternalInventoryClient([number("scopeMode", 0, kind="Toggle", lower=0, upper=1),
                               number("showImage", 0, kind="Toggle", lower=0, upper=1),
                               number("latencyOffset", 0, kind="Integer", lower=0, upper=1024),
                               number("mathMode", 0, kind="Toggle", lower=0, upper=1)])
        runner = self.runner(client)
        runner.run_fixture({**TARGET, "plugin": "External Effect Mono"}, write_roundtrips=True)
        self.assertEqual(runner.counts["records_audited"], 4)
        self.assertEqual(runner.counts["category_state_restore_required"], 4)
        self.assertEqual(client.writes, [])
        self.assertEqual(runner.finish(write_roundtrips=True), 2)
        self.assertEqual(runner.summary["dedicated_or_unresolved_records"], 4)
        self.assertFalse(runner.summary["all_writable_parameters_verified"])

    def test_external_existing_dedicated_actions_and_ordinary_numeric_roles_are_unchanged(self):
        for plugin in ("External Effect Mono", "External Effect Stereo"):
            for name in ("pingMode", "zoomOut", "pingAgain", "latencyTotal", "ms", "samples", "deviceName"):
                parameter = number(name, 0, kind="Toggle", lower=0, upper=1)
                self.assertEqual(classification(plugin, parameter), "dedicated_action_required")
            for name in ("inputGain", "outputGain", "wetdry", "bypass", "phase", "sensitivity", "colorSelect", "showDeviceNotes"):
                parameter = number(name, 0, kind="Integer" if name == "colorSelect" else "Float", lower=0, upper=1)
                self.assertEqual(classification(plugin, parameter), "numeric_scalar")
            notes = {"name": "deviceNotes", "native_kind": "String", "value": "", "enabled": True,
                     "readOnly": False, "writable": True, "support": {"text": True}, "display_parser": "LiteralString",
                     "native_value": {"variant_tag": 19, "data": {"kind": "text", "value": ""}}}
            self.assertEqual(classification(plugin, notes), "literal_string")

    def test_float_exact_metadata_and_negative_zero_do_not_get_silent_tolerance_passes(self):
        parameter = number("gain", -0.0)
        self.assertEqual(classification("Fixture Effect", parameter), "numeric_scalar")
        del parameter["native_value"]["data"]["bits_hex"]
        self.assertEqual(classification("Fixture Effect", parameter), "unresolved_exact_value")

    def test_synthetic_one_ulp_json_drift_restores_the_original_native_bits(self):
        parameter = number("gain", math.nextafter(0.1, math.inf), lower=0.0, upper=1.0)
        parameter["native_value"]["data"]["bits_hex"] = "3FB999999999999A"
        self.assertEqual(classification("Fixture Effect", parameter), "numeric_scalar")
        client = DeviceClient([parameter])
        runner = self.runner(client)
        runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual(runner.counts["changed_readback_restored"], 1)
        self.assertEqual(client.writes[1]["float_bits"], "3FB999999999999A")
        self.assertEqual(value_identity(client.parameters[0])[2], "3FB999999999999A")
        self.assertNotEqual(client.parameters[0]["value"], math.nextafter(0.1, math.inf))
        result = next(event for event in self.events() if event["event"] == "parameter_result")
        self.assertFalse(result["quantized"])

    def test_negative_zero_is_changed_and_restored_using_bits_without_json_numeric_writes(self):
        client = DeviceClient([number("gain", -0.0)])
        runner = self.runner(client)
        runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual(runner.counts["changed_readback_restored"], 1)
        self.assertEqual(client.writes[1]["float_bits"], "8000000000000000")
        self.assertEqual(value_identity(client.parameters[0])[2], "8000000000000000")
        self.assertTrue(all("value" not in write for write in client.writes))

    def test_float_metadata_disagreement_beyond_observed_ulp_is_refused(self):
        parameter = number("gain", 0.25)
        for wrong in (0.5, math.nextafter(math.nextafter(0.25, math.inf), math.inf)):
            with self.subTest(wrong=wrong):
                parameter["value"] = wrong
                parameter["native_value"]["data"]["value"] = wrong
                with self.assertRaises(AcceptanceFailure):
                    value_identity(parameter)
                self.assertEqual(classification("Fixture Effect", parameter), "unresolved_exact_value")

    def test_scalar_and_native_json_values_must_still_agree(self):
        parameter = number("gain", 0.25)
        parameter["value"] = math.nextafter(0.25, math.inf)
        with self.assertRaises(AcceptanceFailure):
            value_identity(parameter)

    def test_float_candidates_are_finite_in_range_and_different_even_for_narrow_or_large_ranges(self):
        pairs = ((1.0, math.nextafter(1.0, math.inf)), (1e308, 1.5e308), (-1.5e308, 1.5e308))
        for lower, upper in pairs:
            for original in (lower, upper):
                with self.subTest(lower=lower, upper=upper, original=original):
                    parameter = number("gain", original, lower=lower, upper=upper)
                    change, restore = candidate(parameter)
                    desired = decode_float_bits(change["float_bits"])
                    self.assertTrue(math.isfinite(desired))
                    self.assertLessEqual(lower, desired)
                    self.assertLessEqual(desired, upper)
                    self.assertNotEqual(desired, original)
                    self.assertEqual(restore["float_bits"], parameter["native_value"]["data"]["bits_hex"])

    def test_integer_values_keep_the_native_integer_write_route(self):
        client = DeviceClient([number("gain", 0, kind="Integer", lower=0, upper=2)])
        runner = self.runner(client)
        runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual(runner.counts["changed_readback_restored"], 1)
        self.assertEqual([write["value"] for write in client.writes], [1, 0])
        self.assertTrue(all("float_bits" not in write for write in client.writes))

    def test_float_carrier_list_uses_integral_choice_and_exact_original_restore(self):
        parameter = number("scale", 0.0, kind="List", lower=0.0, upper=24.0)
        change, restore = candidate(parameter)
        self.assertEqual(change, {"float_bits": "3FF0000000000000"})
        self.assertEqual(restore, {"float_bits": "0000000000000000"})
        client = DeviceClient([parameter])
        runner = self.runner(client)
        runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual([write["float_bits"] for write in client.writes],
                         ["3FF0000000000000", "0000000000000000"])
        self.assertEqual(runner.counts["changed_readback_restored"], 1)
        self.assertEqual(self.changed_attempts(), [1, 2])

    def test_recorded_vocal_tune_custom_scale_plans_an_adjacent_integer(self):
        parameter = number("scale", 15.0, kind="List", lower=0.0, upper=15.0)
        change, restore = candidate(parameter)
        self.assertEqual(change, {"float_bits": "402C000000000000"})
        self.assertEqual(restore, {"float_bits": "402E000000000000"})

    def test_integral_kinds_preserve_carriers_endpoints_and_negative_zero(self):
        for kind in ("Integer", "List"):
            for current, lower, upper, expected in ((0.0, -2.0, 2.0, 1.0),
                                                   (2.0, -2.0, 2.0, 1.0),
                                                   (-2, -2, 2, -1),
                                                   (-0.0, 0.0, 1.0, 1.0),
                                                   (float(9_007_199_254_740_991), float(9_007_199_254_740_990),
                                                    float(9_007_199_254_740_991), float(9_007_199_254_740_990)),
                                                   (9_007_199_254_740_991, 9_007_199_254_740_990,
                                                    9_007_199_254_740_991, 9_007_199_254_740_990)):
                with self.subTest(kind=kind, current=current):
                    parameter = number("mode", current, kind=kind, lower=lower, upper=upper)
                    change, restore = candidate(parameter)
                    if type(current) is int:
                        self.assertEqual(change, {"value": expected})
                        self.assertEqual(restore, {"value": current})
                    else:
                        self.assertEqual(decode_float_bits(change["float_bits"]), expected)
                        self.assertEqual(restore["float_bits"], parameter["native_value"]["data"]["bits_hex"])

    def test_integral_candidates_reject_malformed_nonintegral_or_unsafe_metadata(self):
        invalid = [number("mode", 0.5, kind="List", lower=0.0, upper=2.0),
                   number("mode", 1.0, kind="Integer", lower=0.5, upper=2.0),
                   number("mode", 0.0, kind="List", lower=0.0, upper=2.5),
                   number("mode", 0.0, kind="List", lower=0.0, upper=float(1 << 53)),
                   number("mode", 1 << 53, kind="Integer", lower=0, upper=1 << 53)]
        for field in ("native_min", "native_max", "native_value"):
            for fault in (None, {"variant_tag": True, "data": {"kind": "integer", "value": 0}},
                          {"variant_tag": 1, "data": {"kind": "float", "value": 0.0, "bits_hex": "0000000000000000"}}):
                parameter = number("mode", 0.0, kind="List", lower=0.0, upper=2.0)
                parameter[field] = fault
                invalid.append(parameter)
        rounded = number("mode", 0.0, kind="List", lower=0.0, upper=2.0)
        rounded["native_max"]["data"]["bits_hex"] = struct.pack(">d", math.nextafter(2.0, -math.inf)).hex().upper()
        invalid.append(rounded)
        for field in ("value", "min", "max"):
            for value in (True, float("nan"), float("inf"), 10 ** 400):
                parameter = number("mode", 0.0, kind="List", lower=0.0, upper=2.0)
                parameter[field] = value
                invalid.append(parameter)
        for parameter in invalid:
            with self.subTest(parameter=parameter):
                with self.assertRaises(AcceptanceFailure):
                    candidate(parameter)

    def test_fractional_list_bounds_stop_the_runner_before_any_setter(self):
        client = DeviceClient([number("mode", 0.0, kind="List", lower=0.0, upper=2.5)])
        runner = self.runner(client)
        with self.assertRaises(AcceptanceFailure):
            runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual(client.writes, [])
        self.assertEqual(runner.counts["changed_readback_restored"], 0)

    def test_unmuted_fixture_is_refused_before_any_write(self):
        client = DeviceClient()
        client.muted = False
        runner = self.runner(client)
        with self.assertRaises(AcceptanceFailure):
            runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual(client.writes, [])

    def test_unresolved_in_flight_journal_cannot_be_replayed_or_overwritten(self):
        self.journal.append("transaction_phase", transaction="interrupted", phase="ChangeInFlight")
        self.journal.close()
        before = self.journal.ledger_path.read_bytes()
        with self.assertRaises(AcceptanceFailure):
            Journal(self.directory)
        self.assertEqual(self.journal.ledger_path.read_bytes(), before)
        self.assertEqual(Journal.pending(self.journal.ledger_path), {"interrupted": "ChangeInFlight"})

    def test_real_public_unknown_receipt_before_any_transaction_stays_unresolved(self):
        class UnknownCommandClient(DeviceClient):
            def call(self, name, args):
                if name == "operation_run" and args["operation"] == "command":
                    return self.receipt({}, "outcome_unknown", {"code": "outcome_unknown", "message": "ambiguous mutation"})
                return super().call(name, args)

        runner = self.runner(UnknownCommandClient())
        with self.assertRaises(UnknownMutation):
            runner.operation("command", {"category": "Track", "name": "Mute"}, mutation=True)
        self.assertFalse(any(event["event"] == "transaction_phase" for event in self.events()))
        self.assertEqual(Journal.pending(self.journal.ledger_path), {"rpc:2": "UnknownAfterDispatch"})
        self.journal.close()
        with self.assertRaises(AcceptanceFailure):
            Journal(self.directory)

    def test_pending_mutations_with_malformed_outcomes_remain_unknown(self):
        receipts = [{"outcome": {}}, {"outcome": []}, {"outcome": None}, {},
                    {"outcome": True}, {"outcome": 1}, None, [],
                    {"outcome": "partially_applied"}, {"outcome": "unexpected_status"}]
        for call, receipt in enumerate(receipts, 100):
            self.journal.append("rpc_in_flight", call=call, tool="plugin_parameter_set", mutation=True, arguments={})
            self.journal.append("rpc_returned", call=call, receipt=receipt)
        self.assertEqual(Journal.pending(self.journal.ledger_path),
                         {f"rpc:{call}": "UnknownAfterDispatch" for call in range(100, 100 + len(receipts))})

    def test_pending_known_mutation_receipts_clear_and_read_receipts_do_not_create_mutations(self):
        for call, outcome in enumerate(("applied_verified", "applied_unverified", "not_applied", "observed"), 100):
            self.journal.append("rpc_in_flight", call=call, tool="operation_run", mutation=True, arguments={})
            self.journal.append("rpc_returned", call=call, receipt={"outcome": outcome})
        self.journal.append("rpc_in_flight", call=200, tool="plugin_parameters", mutation=False, arguments={})
        self.journal.append("rpc_returned", call=200, receipt={"outcome": {}})
        self.assertEqual(Journal.pending(self.journal.ledger_path), {})

    def test_pending_bare_mutation_or_capture_retains_present_uncertain_native_outcomes(self):
        faults = ("outcome_unknown", "unknown", "partially_applied", {}, [], None, True, "unexpected_status")
        call = 100
        for field in ("outcome", "native_outcome"):
            for fault in faults:
                call += 1
                self.journal.append("rpc_in_flight", call=call, tool="plugin_state", mutation=True,
                                    arguments={"action": "capture"})
                self.journal.append("rpc_returned", call=call, receipt={"outcome": "applied_unverified",
                                    "result": {field: fault}})
        self.assertEqual(Journal.pending(self.journal.ledger_path),
                         {f"rpc:{number}": "UnknownAfterDispatch" for number in range(101, call + 1)})

    def test_pending_known_native_capture_and_missing_optional_evidence_remain_compatible(self):
        for call, body in enumerate(({}, {"outcome": "observed"}, {"native_outcome": "observed"},
                                     {"outcome": "observed", "native_outcome": "observed"}), 100):
            self.journal.append("rpc_in_flight", call=call, tool="plugin_state", mutation=True,
                                arguments={"action": "capture"})
            self.journal.append("rpc_returned", call=call,
                                receipt={"outcome": "applied_unverified", "result": body})
        self.assertEqual(Journal.pending(self.journal.ledger_path), {})

    def test_common_read_boundary_requires_an_observed_outer_receipt(self):
        faults = (None, {}, [], True, "unknown", "outcome_unknown", "partially_applied",
                  "unexpected_read_status", "applied_verified", "applied_unverified", "missing")
        runner = self.runner(DeviceClient())
        for fault in faults:
            with self.subTest(fault=fault):
                receipt = {"document": "test:1", "session": "test", "outcome": fault,
                           "result": {"open": True, "title": "MCP Validation Scratch"}}
                if fault == "missing":
                    receipt.pop("outcome")
                runner.client = ReceiptClient(receipt)
                with self.assertRaises(UnknownMutation):
                    runner.call("host_status", allow_error=True)

    def test_common_read_and_mutation_reject_present_uncertain_native_outcomes(self):
        runner = self.runner(DeviceClient())
        for mutation in (False, True):
            for field in ("outcome", "native_outcome"):
                for fault in ("unknown", "outcome_unknown", "partially_applied", {}, [], None, True):
                    with self.subTest(mutation=mutation, field=field, fault=fault):
                        runner.client = ReceiptClient({"document": "test:1", "session": "test",
                                                       "outcome": "applied_unverified" if mutation else "observed",
                                                       "result": {field: fault}})
                        with self.assertRaises(UnknownMutation):
                            runner.call("plugin_state" if mutation else "plugin_parameters", mutation=mutation,
                                        allow_error=True)

    def test_common_observed_reads_reject_present_applied_native_evidence(self):
        runner = self.runner(DeviceClient())
        for field in ("outcome", "native_outcome"):
            for outcome in ("applied_verified", "applied_unverified", "not_applied"):
                with self.subTest(field=field, outcome=outcome):
                    runner.client = ReceiptClient({"document": "test:1", "session": "test", "outcome": "observed",
                                                   "result": {field: outcome}})
                    with self.assertRaises(UnknownMutation):
                        runner.call("plugin_parameters", allow_error=True)

    def test_common_mutation_outer_unknown_or_malformed_stops_with_pending_rpc(self):
        runner = self.runner(DeviceClient())
        for outcome in (None, {}, [], True, "partially_applied", "unexpected_status", "outcome_unknown"):
            with self.subTest(outcome=outcome):
                runner.client = ReceiptClient({"document": "test:1", "session": "test", "outcome": outcome,
                                               "result": {"after": {"value": 1}}})
                with self.assertRaises(UnknownMutation):
                    runner.call("plugin_parameter_set", mutation=True, allow_error=True)
        pending = Journal.pending(self.journal.ledger_path)
        self.assertEqual(len(pending), 7)
        self.assertTrue(all(value == "UnknownAfterDispatch" for value in pending.values()))

    def test_common_known_read_errors_keep_allow_error_without_becoming_observed_data(self):
        runner = self.runner(DeviceClient())
        receipt = {"document": "test:1", "session": "test", "outcome": "not_applied",
                   "error": {"code": "native_rejected"}, "result": {"native_outcome": "not_applied"}}
        runner.client = ReceiptClient(receipt)
        with self.assertRaises(AcceptanceFailure):
            runner.call("plugin_parameters")
        self.assertEqual(runner.call("plugin_parameters", allow_error=True), receipt)
        receipt.pop("error")
        with self.assertRaises(AcceptanceFailure):
            runner.call("plugin_parameters")
        self.assertEqual(runner.call("plugin_parameters", allow_error=True)["outcome"], "not_applied")
        receipt.update(outcome="observed", error={"code": "read_failed"}, result={})
        with self.assertRaises(AcceptanceFailure) as observed_error:
            runner.call("plugin_parameters")
        self.assertNotIsInstance(observed_error.exception, UnknownMutation)
        self.assertEqual(runner.call("plugin_parameters", allow_error=True), receipt)

    def test_common_observed_reads_and_known_capture_native_evidence_are_compatible(self):
        runner = self.runner(DeviceClient())
        for mutation, outer, body in ((False, "observed", {}),
                                       (False, "observed", {"outcome": "observed"}),
                                       (True, "applied_unverified", {"native_outcome": "observed"}),
                                       (True, "not_applied", {})):
            receipt = {"document": "test:1", "session": "test", "outcome": outer, "result": body}
            runner.client = ReceiptClient(receipt)
            self.assertEqual(runner.call("plugin_state" if mutation else "plugin_parameters", mutation=mutation,
                                         allow_error=True), receipt)

    def test_unmatched_parameter_filter_does_not_verify_a_zero_write_run(self):
        client = DeviceClient()
        runner = self.runner(client)
        runner.run_fixture(TARGET, write_roundtrips=True, parameters={"missing.exact.name"})
        self.assertEqual(runner.finish(True), 2)
        self.assertEqual(client.writes, [])
        self.assertFalse(runner.summary["eligible_roundtrips_verified"])
        self.assertEqual(runner.summary["status"], "no_eligible_write_cases")
        self.assertEqual(runner.summary["unmatched_parameters"], ["missing.exact.name"])

    def test_unmatched_filter_beside_a_successful_parameter_remains_incomplete(self):
        client = DeviceClient()
        runner = self.runner(client)
        runner.run_fixture(TARGET, write_roundtrips=True, parameters={"gain", "missing.exact.name"})
        self.assertEqual(runner.counts["changed_readback_restored"], 1)
        self.assertEqual(runner.finish(True), 2)
        self.assertFalse(runner.summary["eligible_roundtrips_verified"])
        self.assertEqual(runner.summary["unmatched_parameters"], ["missing.exact.name"])

    def test_read_only_provider_has_no_positive_zero_case_write_verification(self):
        client = DeviceClient([number("meter", 0.0, readonly=True)])
        runner = self.runner(client)
        runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual(runner.finish(True), 2)
        self.assertFalse(runner.summary["eligible_roundtrips_verified"])
        self.assertEqual(runner.summary["status"], "no_eligible_write_cases")

    def test_partial_final_journal_line_is_an_unresolved_run(self):
        self.journal.close()
        with self.journal.ledger_path.open("ab") as stream:
            stream.write(b'{"event":"transaction_phase"')
        self.assertIn("truncated_ledger", Journal.pending(self.journal.ledger_path))

    def test_in_flight_fixture_mutation_is_preserved_without_parameter_transaction(self):
        self.journal.append("rpc_in_flight", call=1, tool="operation_run", mutation=True,
                            arguments={"operation": "command", "args": {"category": "Track", "name": "Mute"}})
        self.journal.close()
        self.assertEqual(Journal.pending(self.journal.ledger_path), {"rpc:1": "MutationInFlight"})

    def test_completed_scalar_run_and_incomplete_categories_have_distinct_exit_statuses(self):
        client = DeviceClient()
        runner = self.runner(client)
        runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual(runner.finish(True), 0)
        self.assertTrue(runner.summary["eligible_roundtrips_verified"])
        self.assertFalse(runner.summary["all_writable_parameters_verified"])
        runner.counts["category_model_state_restore_required"] += 1
        self.assertEqual(runner.finish(True), 2)
        self.assertEqual(runner.summary["status"], "roundtrips_partial")

    def test_raw_output_outside_local_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaises(AcceptanceFailure):
                Journal(Path(directory) / "run")


class DigestDeviceClient(DeviceClient):
    def __init__(self, parameters=None):
        super().__init__(parameters)
        self.digest_calls = 0
        self.digest_hook = self.page_hook = None
        self.receipt_hook = None
        self.digest_readonly_drift = False

    def description(self, parameter):
        value = copy.deepcopy(parameter)
        value.pop("native", None)

        def exact(scalar):
            if type(scalar) is int:
                return {"variant_tag": 1, "data": {"kind": "integer", "value": scalar}}
            return {"variant_tag": 2, "data": {"kind": "float", "value": scalar,
                    "bits_hex": struct.pack(">d", scalar).hex().upper()}}

        value.setdefault("title", value["name"])
        value.setdefault("default", 0.0)
        for field in ("min", "max", "default"):
            value.setdefault("native_" + field, exact(value[field]))
        value.setdefault("native_support", {"raw_number": value["support"].get("raw", False),
                         "normalized": value["support"].get("normalized", False),
                         "display_text": value["support"].get("text", False)})
        value["support"] = {"raw": value["native_support"]["raw_number"],
                            "normalized": value["native_support"]["normalized"],
                            "text": value["native_support"]["display_text"]}
        bits = struct.pack(">f", value["normalized"])
        value["normalized"] = struct.unpack(">f", bits)[0]
        value.setdefault("native_normalized", {"kind": "float32", "bits_hex": bits.hex().upper()})
        return value

    def indexed(self, parameter, index):
        description = self.description(parameter)
        return {**copy.deepcopy(description), "access_metadata_complete": True,
                "native": {"index": index, "name": description["name"], "tag": index + 100,
                           "title": description["name"], "folder": 0, "raw_flags": 0,
                           "native_description": description}}

    def call(self, name, args):
        if name == "plugin_parameter_digest":
            self.calls.append((name, copy.deepcopy(args)))
            self.assert_source(args)
            self.digest_calls += 1
            if self.digest_readonly_drift and len(self.writes) >= 2:
                self.parameters[1].update(number("meter", 0.99, readonly=True))
            records = [self.indexed(parameter, index) for index, parameter in enumerate(self.parameters)]
            body = {**project_snapshot({"source": "dsp", "instance": dict(self.instance), "records": records}),
                    "ok": True, "outcome": "observed"}
            if self.digest_hook:
                self.digest_hook(self, body)
            result = self.receipt(body)
            if self.receipt_hook:
                self.receipt_hook(self, result["structuredContent"])
            return result
        result = super().call(name, args)
        body = result["structuredContent"]["result"]
        if name == "plugin_parameters":
            if args.get("names"):
                body["params"] = [self.description(parameter) for parameter in body["params"]]
            else:
                body["params"] = [self.indexed(parameter, parameter["native"]["index"]) for parameter in body["params"]]
                if self.page_hook:
                    self.page_hook(self, body)
        elif name == "plugin_parameter_set":
            for field in ("before", "after"):
                body[field] = self.description(body[field])
        return result


class DirectDigestTests(unittest.TestCase):
    def setUp(self):
        PRIVATE_ROOT.mkdir(exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(prefix="direct-digest-unit-", dir=PRIVATE_ROOT)
        self.journal = Journal(Path(self.temporary.name) / "run")
        self.no_sleep = patch("verify_all_parameters.time.sleep")
        self.no_sleep.start()

    def tearDown(self):
        self.no_sleep.stop()
        self.journal.close()
        self.temporary.cleanup()

    def reset_journal(self, name):
        self.journal.close()
        self.journal = Journal(Path(self.temporary.name) / name)

    def runner(self, client, read_only_observations=False):
        runner = Runner(client, self.journal, "MCP Validation Scratch", block_size=1,
                        read_only_observations=read_only_observations, use_parameter_digest=True)
        runner.initialize()
        return runner

    def events(self):
        return [json.loads(line) for line in self.journal.ledger_path.read_text().splitlines()]

    def test_each_direct_restore_needs_two_fresh_digests_and_a_complete_fixture_end(self):
        client = DigestDeviceClient([number("gain", 0.25), number("mix", 0.5)])
        runner = self.runner(client)
        runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual(runner.finish(True), 0)
        row = runner.summary["classes"][0]
        self.assertTrue(row["digest_fixture_complete"])
        self.assertEqual(runner.counts["changed_readback_restored"], 2)
        observations = [event for event in self.events()
                        if event["event"] == "direct_parameter_digest_observation" and event["checkpoint"] == "direct_restore"]
        self.assertEqual([event["attempt"] for event in observations], [1, 2, 1, 2])
        self.assertEqual(len({event["fresh_rpc"] for event in observations}), 4)
        checkpoints = [event["checkpoint"] for event in self.events()
                       if event["event"] == "direct_parameter_digest_full_checkpoint"]
        self.assertEqual(checkpoints, ["before_first_write", "fixture_end"])
        self.assertEqual(len(client.writes), 4)
        self.assertFalse(any(name == "plugin_state" for name, _ in client.calls))
        self.assertFalse(runner.summary["all_writable_parameters_verified"])

    def test_explicit_mode_never_allows_parameter_batches_or_implicit_truthy_flags(self):
        for block in (True, 0, 2, 16, 64, 1.0):
            with self.assertRaises(AcceptanceFailure):
                Runner(None, self.journal, "Scratch", block_size=block, use_parameter_digest=True)
        for flag in (None, 0, 1, "true"):
            with self.assertRaises(AcceptanceFailure):
                Runner(None, self.journal, "Scratch", block_size=1, use_parameter_digest=flag)

    def test_a_second_digest_disagreement_stops_after_direct_restore_without_more_writes(self):
        client = DigestDeviceClient([number("gain", 0.25), number("mix", 0.5)])
        client.digest_hook = lambda client, body: body.update(non_read_only_values_sha256="0" * 64) if client.digest_calls == 6 else None
        runner = self.runner(client)
        runner.block_size = 64
        with self.assertRaises(DependencyFailure):
            runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual(len(client.writes), 2)
        self.assertEqual(runner.counts["changed_readback_restored"], 0)
        diagnosis = next(event for event in self.events() if event["event"] == "direct_parameter_digest_mismatch_diagnosis")
        self.assertTrue(diagnosis["diagnostic_only"])
        self.assertTrue(diagnosis["native_parity_matches"])
        self.assertEqual(diagnosis["native_parity_reference"], "current_diagnostic_projection")
        self.assertTrue(diagnosis["diagnostic_projection_matches_frozen_baseline"])
        self.assertTrue(Journal.pending(self.journal.ledger_path))
        with self.assertRaises(AcceptanceFailure):
            runner.finish(True)

    def test_coupled_state_changes_are_not_repaired_or_rebased(self):
        client = DigestDeviceClient([number("gain", 0.25), number("mix", 0.5)])
        client.side_effect = True
        runner = self.runner(client)
        with self.assertRaises(DependencyFailure):
            runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual([write["param"] for write in client.writes], ["gain", "gain"])
        self.assertEqual(client.parameters[1]["value"], 0.99)
        self.assertFalse(any(name == "plugin_state" for name, _ in client.calls))
        diagnosis = next(event for event in self.events() if event["event"] == "direct_parameter_digest_mismatch_diagnosis")
        self.assertTrue(diagnosis["native_parity_matches"])
        self.assertFalse(diagnosis["diagnostic_projection_matches_frozen_baseline"])

    def test_readonly_drift_requires_the_explicit_mode_and_preserves_exact_schema(self):
        for allowed in (False, True):
            self.reset_journal("readonly-" + str(allowed))
            client = DigestDeviceClient([number("gain", 0.25), number("meter", 0.1, readonly=True)])
            client.digest_readonly_drift = True
            runner = self.runner(client, read_only_observations=allowed)
            if allowed:
                runner.run_fixture(TARGET, write_roundtrips=True)
                self.assertEqual(runner.finish(True), 0)
                self.assertEqual(runner.summary["read_only_values"], "observed_not_restored")
                self.assertEqual(runner.counts["read_only_digest_difference_observations"], 2)
            else:
                with self.assertRaises(DependencyFailure):
                    runner.run_fixture(TARGET, write_roundtrips=True)
            self.assertEqual(len(client.writes), 2)

    def test_readonly_access_and_typed_default_support_drift_are_not_excluded(self):
        for field in ("read_only", "default", "support"):
            self.reset_journal("schema-" + field)
            client = DigestDeviceClient([number("gain", 0.25), number("meter", 0.1, readonly=True)])

            def drift(client, body):
                if len(client.writes) >= 2:
                    if field == "read_only":
                        client.parameters[1]["readOnly"] = False
                    elif field == "default":
                        client.parameters[1]["default"] = 0.5
                    else:
                        client.parameters[1]["support"]["raw"] = True
                        client.parameters[1]["writable"] = True
                    records = [client.indexed(parameter, index) for index, parameter in enumerate(client.parameters)]
                    body.update(project_snapshot({"source": "dsp", "instance": dict(client.instance), "records": records}))

            client.digest_hook = drift
            runner = self.runner(client, read_only_observations=True)
            with self.assertRaises(AcceptanceFailure):
                runner.run_fixture(TARGET, write_roundtrips=True)
            self.assertEqual(len(client.writes), 2)

    def test_every_native_contract_field_and_hash_encoding_is_required_before_writes(self):
        exemplar = DigestDeviceClient()
        value = exemplar.call("plugin_parameter_digest", {"source": "dsp", "expected_instance": INSTANCE["object_uid"]})["structuredContent"]["result"]
        for field in value:
            self.reset_journal("missing-" + field)
            client = DigestDeviceClient()
            client.digest_hook = lambda _, body, field=field: body.pop(field)
            runner = self.runner(client)
            with self.assertRaises(AcceptanceFailure, msg=field):
                runner.run_fixture(TARGET, write_roundtrips=True)
            self.assertEqual(client.writes, [])
        for field in HASH_FIELDS:
            self.reset_journal("malformed-" + field)
            client = DigestDeviceClient()
            client.digest_hook = lambda _, body, field=field: body.update({field: "F" * 64})
            runner = self.runner(client)
            with self.assertRaises(AcceptanceFailure):
                runner.run_fixture(TARGET, write_roundtrips=True)
            self.assertEqual(client.writes, [])

    def test_wrong_native_identity_provider_count_and_schema_cannot_begin_writes(self):
        replacements = {"instance": {**INSTANCE, "object_uid": "ABCDEFAB-1234-1234-1234-123456789ABC"},
                        "source": "controller", "count": 2, "schema_sha256": "0" * 64,
                        "access_complete": False, "atomic_snapshot": True,
                        "coverage": "cached", "opaque_contents_verified": True}
        for field, value in replacements.items():
            self.reset_journal("wrong-" + field)
            client = DigestDeviceClient()
            client.digest_hook = lambda _, body, field=field, value=value: body.update({field: value})
            runner = self.runner(client)
            with self.assertRaises(AcceptanceFailure):
                runner.run_fixture(TARGET, write_roundtrips=True)
            self.assertEqual(client.writes, [])

    def test_error_or_unknown_native_read_cannot_reuse_supplied_hashes(self):
        for change in ({"ok": False}, {"outcome": "unknown"}, {"error": {"message": "read failed"}}):
            self.reset_journal("read-" + next(iter(change)))
            client = DigestDeviceClient()
            client.digest_hook = lambda _, body: body.update(change)
            runner = self.runner(client)
            with self.assertRaises(AcceptanceFailure):
                runner.run_fixture(TARGET, write_roundtrips=True)
            self.assertEqual(client.writes, [])

    def test_complete_public_metadata_is_required_for_independent_initial_projection(self):
        client = DigestDeviceClient()

        def missing(_, body):
            body["params"][0].pop("native_min")
            body["params"][0]["native"]["native_description"].pop("native_min")

        client.page_hook = missing
        runner = self.runner(client)
        with self.assertRaises(AcceptanceFailure):
            runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual(client.digest_calls, 0)
        self.assertEqual(client.writes, [])

    def test_nonselected_display_and_normalized_changes_are_detected_after_raw_restoration(self):
        for field, value in (("text", "changed display"), ("normalized", 0.75)):
            self.reset_journal("changed-" + field)
            client = DigestDeviceClient([number("gain", 0.25), number("mix", 0.5)])

            def drift(client, body):
                if len(client.writes) >= 2:
                    client.parameters[1][field] = value
                    records = [client.indexed(parameter, index) for index, parameter in enumerate(client.parameters)]
                    body.update(project_snapshot({"source": "dsp", "instance": dict(client.instance), "records": records}))

            client.digest_hook = drift
            runner = self.runner(client)
            with self.assertRaises(DependencyFailure):
                runner.run_fixture(TARGET, write_roundtrips=True)
            self.assertEqual(len(client.writes), 2)

    def test_checkpoint_at_case_32_stops_before_the_next_parameter(self):
        client = DigestDeviceClient([number("gain" + str(index), 0.25) for index in range(33)])
        client.page_hook = lambda client, body: body["params"][0]["native"].update(raw_flags=1) if client.pages == 3 else None
        runner = self.runner(client)
        with self.assertRaises(ContextChanged):
            runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual(len(client.writes), 64)
        self.assertEqual(runner.counts["changed_readback_restored"], 32)
        with self.assertRaises(AcceptanceFailure):
            runner.finish(True)

    def test_successful_33_case_run_includes_the_periodic_full_page_checkpoint(self):
        client = DigestDeviceClient([number("gain" + str(index), 0.25) for index in range(33)])
        runner = self.runner(client)
        runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual(runner.finish(True), 0)
        checkpoints = [event["checkpoint"] for event in self.events()
                       if event["event"] == "direct_parameter_digest_full_checkpoint"]
        self.assertEqual(checkpoints, ["before_first_write", "completed_cases_32", "fixture_end"])
        self.assertEqual(len(client.writes), 66)

    def test_missing_fixture_end_cannot_pass_even_after_every_selected_scalar_was_confirmed(self):
        client = DigestDeviceClient()
        client.page_hook = lambda client, body: body["params"][0]["native"].update(raw_flags=1) if client.pages == 3 else None
        runner = self.runner(client)
        with self.assertRaises(ContextChanged):
            runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual(runner.counts["changed_readback_restored"], 1)
        with self.assertRaises(AcceptanceFailure):
            runner.finish(True)

    def test_unknown_change_or_changed_document_never_gets_a_restore_or_next_write(self):
        for field, error in (("unknown_change", UnknownMutation), ("change_document", ContextChanged)):
            self.reset_journal("unknown-" + field)
            client = DigestDeviceClient([number("gain", 0.25), number("mix", 0.5)])
            setattr(client, field, True)
            runner = self.runner(client)
            with self.assertRaises(error):
                runner.run_fixture(TARGET, write_roundtrips=True)
            self.assertEqual(len(client.writes), 1)
            self.assertEqual(client.digest_calls, 4)

    def test_a_lied_about_direct_restore_stops_before_digest_confirmation(self):
        client = DigestDeviceClient([number("gain", 0.25), number("mix", 0.5)])
        client.fail_restore = client.lie_about_restore = True
        runner = self.runner(client)
        with self.assertRaises(RestorationFailure):
            runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual(len(client.writes), 2)
        self.assertEqual(client.digest_calls, 4)

    def test_outer_digest_context_error_or_unknown_receipt_stops_with_no_later_write(self):
        for field, value, error in (("document", "other:1", ContextChanged),
                                    ("outcome", "outcome_unknown", AcceptanceFailure),
                                    ("error", {"message": "read failed"}, AcceptanceFailure)):
            self.reset_journal("outer-" + field)
            client = DigestDeviceClient([number("gain", 0.25), number("mix", 0.5)])

            def change(client, receipt):
                if client.digest_calls == 6:
                    receipt[field] = value

            client.receipt_hook = change
            runner = self.runner(client)
            with self.assertRaises(error):
                runner.run_fixture(TARGET, write_roundtrips=True)
            self.assertEqual(len(client.writes), 2)
            self.assertEqual(runner.counts["changed_readback_restored"], 0)

    def test_exact_negative_zero_is_restored_without_capsule_calls(self):
        client = DigestDeviceClient([number("gain", -0.0)])
        runner = self.runner(client)
        runner.run_fixture(TARGET, write_roundtrips=True)
        self.assertEqual(runner.finish(True), 0)
        self.assertEqual(client.writes[1]["float_bits"], "8000000000000000")
        self.assertEqual(value_identity(client.parameters[0])[2], "8000000000000000")

    def test_unmatched_filters_and_zero_selected_controls_never_gain_a_digest_pass(self):
        client = DigestDeviceClient([number("meter", 0.1, readonly=True)])
        runner = self.runner(client, read_only_observations=True)
        runner.run_fixture(TARGET, write_roundtrips=True, parameters={"missing"})
        self.assertEqual(runner.finish(True), 2)
        self.assertFalse(runner.summary["eligible_roundtrips_verified"])
        self.assertEqual(runner.summary["unmatched_parameters"], ["missing"])
        self.assertEqual(client.writes, [])


if __name__ == "__main__":
    unittest.main()
