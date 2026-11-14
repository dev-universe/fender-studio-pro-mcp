#!/usr/bin/env python3
"""Offline full-capsule scalar sweep checks. No DAW is launched."""
import copy
import json
import math
from pathlib import Path
import struct
import tempfile
import unittest

from test_verify_all_parameters import INSTANCE, TARGET, number
from test_verify_plugin_states import StateClient, SettlingStateClient
from verify_all_parameters import (AcceptanceFailure, ContextChanged, Journal, PRIVATE_ROOT,
                                   RestorationFailure, UnknownMutation,
                                   decode_float_bits, value_identity)
from verify_all_parameter_states import ExhaustiveStateRunner, parse_repair_parameters
from parameter_digest import project_snapshot


EXPANDER_TARGET = {**TARGET, "plugin": "Expander"}
PRO_EQ_TARGET = {**TARGET, "plugin": "Pro EQ"}
PRO_EQ_CLASS = "D9AE9ACD-69B4-4B43-B8D5-983E39C559A5"


class SweepClient(StateClient):
    def __init__(self, parameters=None):
        super().__init__(parameters)
        self.before_writes = []
        self.behaviors = {}
        self.couplings = {}
        self.capture_changes_schema = False
        self.capture_changes_class = False
        self.after_load_changes_class = False
        self.schema_changed_by_write = False
        self.live_class = None
        self.events = []

    def select_target(self, args):
        super().select_target(args)
        if self.live_class:
            self.instance["class_uid"] = self.live_class

    def call(self, name, args):
        if name == "plugin_editor_open":
            return self.receipt({"requested": True, "verified": False}, "applied_unverified")
        if name == "plugin_parameter_set":
            self.select_target(args)
            self.calls.append((name, copy.deepcopy(args)))
            self.writes.append(copy.deepcopy(args))
            self.events.append(("write", args["param"]))
            self.before_writes.append([value_identity(parameter) for parameter in self.parameters])
            parameter = next(item for item in self.parameters if item["name"] == args["param"])
            before = copy.deepcopy(parameter)
            behavior = self.behaviors.get(args["param"])
            if behavior in ("reject", "reject_changed"):
                if behavior == "reject_changed":
                    parameter.update(number(parameter["name"], 0.99))
                return self.receipt({"native_outcome": "not_applied", "errors": ["test refusal"]},
                                    "not_applied", "test refusal")
            if behavior != "ignore":
                if "text" in args:
                    parameter["value"] = parameter["text"] = args["text"]
                    parameter["native_value"]["data"]["value"] = args["text"]
                else:
                    desired = decode_float_bits(args["float_bits"]) if "float_bits" in args else args["value"]
                    updated = number(parameter["name"], desired, parameter["native_kind"],
                                     lower=parameter["min"], upper=parameter["max"])
                    updated["support"] = copy.deepcopy(parameter["support"])
                    parameter.update(updated)
            coupled = self.couplings.get(args["param"])
            if coupled:
                other = next(item for item in self.parameters if item["name"] == coupled)
                other.update(number(coupled, 0.99))
            if self.schema_changed_by_write:
                parameter["tag"] += 1
            if behavior == "unknown":
                raise TimeoutError("test change interrupted after dispatch")
            if behavior == "public_unknown":
                return self.receipt({}, "outcome_unknown", {"code": "outcome_unknown", "message": "ambiguous mutation"})
            if behavior == "document_changed":
                self.document = "test:2"
            return self.receipt({"before": before, "after": copy.deepcopy(parameter),
                                 "instance": dict(self.instance), "source": "dsp"}, "applied_verified")
        if name == "plugin_state":
            self.events.append((args["action"], args.get("token")))
            result = super().call(name, args)
            if args["action"] == "capture":
                if self.capture_changes_schema:
                    self.parameters[0]["tag"] += 1
                if self.capture_changes_class:
                    self.live_class = "ABCDEFAB-4321-4321-4321-CBA987654321"
            elif args["action"] == "restore" and self.after_load_changes_class:
                self.live_class = "ABCDEFAB-4321-4321-4321-CBA987654321"
            return result
        return super().call(name, args)


class LossyStateClient(SweepClient):
    def __init__(self, parameters=None):
        super().__init__(parameters)
        self.lossy_overrides = {}
        self.lie_about_correction = False

    def call(self, name, args):
        if name == "plugin_parameter_set" and self.lie_about_correction and args["param"] == "attack":
            self.behaviors["attack"] = "ignore"
            result = super().call(name, args)
            stored = next(iter(self.tokens.values()))
            original = next(parameter for parameter in stored["parameters"] if parameter["name"] == "attack")
            result["structuredContent"]["result"]["after"] = copy.deepcopy(original)
            return result
        result = super().call(name, args)
        if name == "plugin_state" and args["action"] == "restore":
            receipt = result["structuredContent"]
            if receipt["outcome"] in ("applied_verified", "applied_unverified") and receipt["result"].get("load_accepted"):
                for parameter in self.parameters:
                    if parameter["name"] in self.lossy_overrides:
                        parameter.update(copy.deepcopy(self.lossy_overrides[parameter["name"]]))
        return result


class DigestSweepClient(LossyStateClient):
    """Complete synthetic 8.1.1-shaped pages and fresh independently bound reads."""
    def __init__(self, parameters=None):
        super().__init__(parameters)
        self.digest_calls = 0
        self.digest_hook = self.page_hook = None
        self.digest_readonly_drift = False

    def description(self, parameter):
        value = copy.deepcopy(parameter)
        value.pop("native", None)
        value.pop("index", None)

        def exact(scalar):
            if scalar is None:
                return None
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
        if value["normalized"] is None:
            value.setdefault("native_normalized", {"kind": "not_applicable"})
        else:
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
            self.select_target(args)
            self.calls.append((name, copy.deepcopy(args)))
            self.digest_calls += 1
            self.events.append(("digest", len(self.writes)))
            if self.digest_readonly_drift and self.restore_count:
                self.parameters[1].update(number("meter", self.parameters[1]["value"] + 0.01, readonly=True))
            records = [self.indexed(parameter, index) for index, parameter in enumerate(self.parameters)]
            result = {**project_snapshot({"source": "dsp", "instance": dict(self.instance), "records": records}),
                      "ok": True, "outcome": "observed"}
            if self.digest_hook:
                self.digest_hook(self, result)
            return self.receipt(result)
        result = super().call(name, args)
        body = result["structuredContent"]["result"]
        if name == "plugin_parameters":
            if args.get("names"):
                body["params"] = [self.description(parameter) for parameter in body["params"]]
            else:
                self.events.append(("full_page", len(self.writes)))
                body["params"] = [self.indexed(parameter, parameter["native"]["index"]) for parameter in body["params"]]
                if self.page_hook:
                    self.page_hook(self, body)
        elif name == "plugin_parameter_set":
            for field in ("before", "after"):
                if isinstance(body.get(field), dict):
                    body[field] = self.description(body[field])
        return result


def expander_attack(value, bits):
    parameter = number("attack", value, lower=0.0, upper=1.0)
    parameter["native_value"]["data"]["bits_hex"] = bits
    return parameter


class ExhaustiveStateTests(unittest.TestCase):
    def setUp(self):
        PRIVATE_ROOT.mkdir(exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(prefix="all-state-unit-", dir=PRIVATE_ROOT)
        self.journal = Journal(Path(self.temporary.name) / "run")

    def tearDown(self):
        self.journal.close()
        self.temporary.cleanup()

    def runner(self, client, read_only_observations=False, verify_released=False,
               repair_captured_scalars=False, repair_parameters=None, use_parameter_digest=False):
        runner = ExhaustiveStateRunner(client, self.journal, "MCP Validation Scratch",
                                       settle_attempts=4, settle_interval=0,
                                       read_only_observations=read_only_observations,
                                       verify_released=verify_released,
                                       repair_captured_scalars=repair_captured_scalars,
                                       repair_parameters=repair_parameters,
                                       use_parameter_digest=use_parameter_digest)
        runner.initialize()
        return runner

    def events(self):
        return [json.loads(line) for line in self.journal.ledger_path.read_text().splitlines()]

    def lossy_bypass(self, original=0.0, changed=1.0):
        client = DigestSweepClient([number("bypass", original, kind="Toggle")])
        client.live_class = PRO_EQ_CLASS
        client.lossy_overrides["bypass"] = number("bypass", changed, kind="Toggle")
        return client

    def test_same_frozen_capsule_restores_coupled_filters_before_every_next_parameter(self):
        client = SweepClient([number("filterlow", 0.25), number("filterhigh", 0.5)])
        initial = [value_identity(parameter) for parameter in client.parameters]
        client.couplings = {"filterlow": "filterhigh", "filterhigh": "filterlow"}
        runner = self.runner(client)
        row = runner.run_fixture(TARGET)
        self.assertEqual([write["param"] for write in client.writes], ["filterlow", "filterhigh"])
        self.assertEqual(client.before_writes, [initial, initial])
        self.assertEqual([value_identity(parameter) for parameter in client.parameters], initial)
        self.assertEqual(client.capture_count, 1)
        self.assertEqual(client.restore_count, 2)
        tokens = [call["token"] for call in client.state_calls if call["action"] == "restore"]
        self.assertEqual(len(set(tokens)), 1)
        self.assertEqual(row["results"], {"changed_readback_restored": 2})
        self.assertTrue(row["capsule_released"])
        self.assertEqual(runner.counts["changed_load_independently_restored"], 2)
        self.assertEqual(Journal.pending(self.journal.ledger_path), {})
        states = [event for event in self.events() if event["event"] == "state_baseline_observation"]
        self.assertEqual(len(states), 6)
        self.assertTrue(all(event["matches"] for event in states))
        self.assertEqual([event for event in client.events if event[0] == "release"], [("release", tokens[0])])

    def test_ignored_known_change_restores_frozen_state_then_continues_as_incomplete(self):
        client = SweepClient([number("gain", 0.25), number("mix", 0.5)])
        client.behaviors["gain"] = "ignore"
        runner = self.runner(client)
        row = runner.run_fixture(TARGET)
        self.assertEqual(row["results"], {"change_failed_restored": 1, "changed_readback_restored": 1})
        self.assertEqual(client.restore_count, 2)
        self.assertEqual([parameter["value"] for parameter in client.parameters], [0.25, 0.5])
        self.assertEqual(runner.finish(True), 2)
        self.assertFalse(runner.summary["eligible_roundtrips_verified"])
        self.assertEqual(Journal.pending(self.journal.ledger_path), {})

    def test_rejected_write_needs_two_full_unchanged_baselines_and_no_capsule_load(self):
        client = SweepClient([number("gain", 0.25), number("mix", 0.5)])
        client.behaviors["gain"] = "reject"
        runner = self.runner(client)
        row = runner.run_fixture(TARGET)
        self.assertEqual(row["results"], {"write_rejected": 1, "changed_readback_restored": 1})
        self.assertEqual(client.restore_count, 1)
        rejected = next(event for event in self.events() if event.get("status") == "write_rejected"
                        and event["event"] == "parameter_state_result")
        self.assertTrue(rejected["full_provider_unchanged"])
        self.assertFalse(rejected["capsule_load_attempted"])
        self.assertEqual(runner.finish(True), 2)

    def test_rejected_write_that_changes_state_is_unknown_and_never_loaded_or_advanced(self):
        client = SweepClient([number("gain", 0.25), number("mix", 0.5)])
        client.behaviors["gain"] = "reject_changed"
        runner = self.runner(client)
        with self.assertRaises(UnknownMutation):
            runner.run_fixture(TARGET)
        self.assertEqual(len(client.writes), 1)
        self.assertEqual(client.restore_count, 0)
        self.assertFalse(any(call["action"] == "release" for call in client.state_calls))

    def test_unknown_dispatch_never_loads_the_capsule_or_changes_another_parameter(self):
        client = SweepClient([number("gain", 0.25), number("mix", 0.5)])
        client.behaviors["gain"] = "unknown"
        runner = self.runner(client)
        with self.assertRaises(UnknownMutation):
            runner.run_fixture(TARGET)
        self.assertEqual(len(client.writes), 1)
        self.assertEqual(client.restore_count, 0)
        self.assertFalse(any(call["action"] == "release" for call in client.state_calls))
        self.assertIn("UnknownAfterDispatch", Journal.pending(self.journal.ledger_path).values())

    def test_real_public_unknown_receipt_stops_and_keeps_the_rpc_pending(self):
        client = SweepClient()
        client.behaviors["gain"] = "public_unknown"
        runner = self.runner(client)
        with self.assertRaises(UnknownMutation):
            runner.run_fixture(TARGET)
        self.assertEqual(client.restore_count, 0)
        self.assertFalse(any(call["action"] == "release" for call in client.state_calls))
        pending = Journal.pending(self.journal.ledger_path)
        self.assertTrue(any(key.startswith("rpc:") and value == "UnknownAfterDispatch" for key, value in pending.items()))

    def test_unmatched_filter_is_unverified_without_capturing_or_writing(self):
        client = SweepClient()
        runner = self.runner(client)
        row = runner.run_fixture(TARGET, parameters={"missing.exact.name"})
        self.assertEqual(row["selected_for_write"], 0)
        self.assertEqual(client.capture_count, 0)
        self.assertEqual(client.writes, [])
        self.assertEqual(runner.finish(True), 2)
        self.assertFalse(runner.summary["eligible_roundtrips_verified"])
        self.assertEqual(runner.summary["unmatched_parameters"], ["missing.exact.name"])

    def test_matched_and_unmatched_state_filter_does_not_hide_a_typo(self):
        client = SweepClient()
        runner = self.runner(client)
        runner.run_fixture(TARGET, parameters={"gain", "missing.exact.name"})
        self.assertEqual(runner.counts["changed_readback_restored"], 1)
        self.assertEqual(client.restore_count, 1)
        self.assertEqual(runner.finish(True), 2)
        self.assertFalse(runner.summary["eligible_roundtrips_verified"])
        self.assertEqual(runner.summary["unmatched_parameters"], ["missing.exact.name"])

    def test_changed_document_after_dispatch_stops_without_a_load(self):
        client = SweepClient([number("gain", 0.25), number("mix", 0.5)])
        client.behaviors["gain"] = "document_changed"
        runner = self.runner(client)
        with self.assertRaises(ContextChanged):
            runner.run_fixture(TARGET)
        self.assertEqual(len(client.writes), 1)
        self.assertEqual(client.restore_count, 0)

    def test_accepted_load_that_does_not_restore_stops_before_the_next_write(self):
        client = SweepClient([number("gain", 0.25), number("mix", 0.5)])
        client.ignore_load = True
        runner = self.runner(client)
        with self.assertRaises(RestorationFailure):
            runner.run_fixture(TARGET)
        self.assertEqual(len(client.writes), 1)
        self.assertEqual(client.restore_count, 1)
        self.assertEqual(runner.counts["changed_load_independently_restored"], 0)
        self.assertFalse(any(call["action"] == "release" for call in client.state_calls))

    def test_false_load_result_stops_even_when_values_appear_restored(self):
        client = SweepClient([number("gain", 0.25), number("mix", 0.5)])
        client.false_load = True
        runner = self.runner(client)
        with self.assertRaises(RestorationFailure):
            runner.run_fixture(TARGET)
        self.assertEqual(len(client.writes), 1)
        self.assertEqual(client.restore_count, 1)

    def test_unknown_load_stops_without_another_write_or_release(self):
        client = SweepClient([number("gain", 0.25), number("mix", 0.5)])
        client.unknown_load = True
        runner = self.runner(client)
        with self.assertRaises(UnknownMutation):
            runner.run_fixture(TARGET)
        self.assertEqual(len(client.writes), 1)
        self.assertEqual(client.restore_count, 1)
        self.assertFalse(any(call["action"] == "release" for call in client.state_calls))

    def test_class_uid_change_after_capture_stops_before_the_first_change(self):
        client = SweepClient()
        client.capture_changes_class = True
        runner = self.runner(client)
        with self.assertRaises(ContextChanged):
            runner.run_fixture(TARGET)
        self.assertEqual(client.capture_count, 1)
        self.assertEqual(client.writes, [])
        self.assertEqual(client.restore_count, 0)

    def test_page_instance_change_stops_before_capture(self):
        client = SweepClient([number(f"gain{i}", 0.25) for i in range(65)])
        client.replace_on_page = 2
        runner = self.runner(client)
        with self.assertRaises(ContextChanged):
            runner.run_fixture(TARGET)
        self.assertEqual(client.capture_count, 0)
        self.assertEqual(client.writes, [])

    def test_capture_schema_mutation_does_not_get_a_scalar_write(self):
        client = SweepClient()
        client.capture_changes_schema = True
        runner = self.runner(client)
        with self.assertRaises(RestorationFailure):
            runner.run_fixture(TARGET)
        self.assertEqual(client.writes, [])
        self.assertEqual(client.restore_count, 0)

    def test_schema_change_after_write_is_not_blindly_restored(self):
        client = SweepClient()
        client.schema_changed_by_write = True
        runner = self.runner(client)
        with self.assertRaises(ContextChanged):
            runner.run_fixture(TARGET)
        self.assertEqual(len(client.writes), 1)
        self.assertEqual(client.restore_count, 0)

    def test_class_uid_change_after_load_stops_before_another_parameter(self):
        client = SweepClient([number("gain", 0.25), number("mix", 0.5)])
        client.after_load_changes_class = True
        runner = self.runner(client)
        with self.assertRaises(ContextChanged):
            runner.run_fixture(TARGET)
        self.assertEqual(len(client.writes), 1)
        self.assertEqual(client.restore_count, 1)

    def test_released_token_refusal_runs_once_per_fixture_when_requested(self):
        client = SweepClient([number("gain", 0.25), number("mix", 0.5)])
        runner = self.runner(client, verify_released=True)
        row = runner.run_fixture(TARGET)
        self.assertTrue(row["released_token_refused"])
        self.assertEqual(client.capture_count, 1)
        self.assertEqual(client.restore_count, 3)
        self.assertEqual(len([event for event in self.events() if event["event"] == "released_token_refused"]), 1)
        self.assertEqual(Journal.pending(self.journal.ledger_path), {})

    def test_accepting_a_released_token_is_a_failure_not_a_successful_cleanup(self):
        client = SweepClient()
        client.accept_released = True
        runner = self.runner(client, verify_released=True)
        with self.assertRaises(AcceptanceFailure):
            runner.run_fixture(TARGET)
        self.assertEqual(runner.counts["fixtures_state_workflow_completed"], 0)

    def test_complete_read_only_observations_are_optional_and_never_claimed_restored(self):
        meter = {**number("meter", 0.0, readonly=True), "access_metadata_complete": True}
        client = SweepClient([number("gain", 0.25), meter])
        client.readonly_drift = True
        runner = self.runner(client, read_only_observations=True)
        row = runner.run_fixture(TARGET)
        self.assertEqual(row["results"], {"changed_readback_restored": 1})
        self.assertEqual(row["read_only_values"], "observed_not_restored")
        self.assertFalse(row["opaque_contents_verified"])
        self.assertFalse(runner.summary["all_writable_parameters_verified"])

    def test_strict_read_only_values_cannot_drift_during_restoration(self):
        meter = {**number("meter", 0.0, readonly=True), "access_metadata_complete": True}
        client = SweepClient([number("gain", 0.25), meter])
        client.readonly_drift = True
        runner = self.runner(client)
        with self.assertRaises(RestorationFailure):
            runner.run_fixture(TARGET)

    def test_incomplete_read_only_proof_remains_a_failed_restoration(self):
        client = SweepClient([number("gain", 0.25), number("meter", 0.0, readonly=True)])
        client.readonly_drift = True
        runner = self.runner(client, read_only_observations=True)
        with self.assertRaises(RestorationFailure):
            runner.run_fixture(TARGET)

    def test_stable_native_float_settlement_is_followed_by_full_state_load(self):
        client = SettlingStateClient()
        runner = self.runner(client)
        row = runner.run_fixture(TARGET)
        self.assertEqual(row["results"], {"changed_readback_restored": 1})
        self.assertEqual(client.restore_count, 1)
        self.assertEqual(runner.counts["quantized_settlements"], 1)
        self.assertEqual(value_identity(client.parameters[0])[2], "3FE8000000000000")

    def test_parameter_results_stay_in_the_ledger_with_only_one_class_summary(self):
        client = SweepClient([number(f"gain{i}", 0.25) for i in range(20)])
        runner = self.runner(client)
        row = runner.run_fixture(TARGET)
        self.assertEqual(len(runner.summary["classes"]), 1)
        self.assertEqual(len(row["results"]), 1)
        self.assertNotIn("parameters", row)
        self.assertEqual(len([event for event in self.events() if event["event"] == "parameter_state_result"]), 20)
        self.assertEqual(len(list(self.journal.baselines.glob("*.json"))), 1)

    def test_model_and_dedicated_actions_remain_incomplete_in_the_class_aggregate(self):
        target = {**TARGET, "plugin": "Fat Channel"}
        client = SweepClient([number("opt.eqmodel", 0, kind="List", lower=0, upper=2), number("gain", 0.25)])
        runner = self.runner(client)
        row = runner.run_fixture(target)
        self.assertEqual([write["param"] for write in client.writes], ["gain"])
        self.assertEqual(row["categories"]["model_state_restore_required"], 1)
        self.assertEqual(runner.finish(True), 2)
        self.assertFalse(runner.summary["all_writable_parameters_verified"])

    def test_dedicated_external_action_is_not_used_as_a_generic_state_scalar(self):
        target = {**TARGET, "plugin": "External Effect Mono"}
        client = SweepClient([number("pingAgain", 0, kind="Toggle"), number("gain", 0.25)])
        runner = self.runner(client)
        row = runner.run_fixture(target)
        self.assertEqual([write["param"] for write in client.writes], ["gain"])
        self.assertEqual(row["categories"]["dedicated_action_required"], 1)
        self.assertEqual(runner.finish(True), 2)

    def test_literal_string_is_changed_and_restored_by_the_same_full_capsule(self):
        original = "scratch 한글"
        parameter = {"name": "deviceNotes", "tag": 10, "native_kind": "String", "value": original,
                     "min": None, "max": None, "normalized": None, "text": original,
                     "readOnly": False, "enabled": True, "writable": True,
                     "support": {"raw": False, "text": True}, "display_parser": "LiteralString",
                     "native_value": {"variant_tag": 3, "data": {"kind": "text", "value": original}}}
        client = SweepClient([parameter])
        runner = self.runner(client)
        row = runner.run_fixture(TARGET)
        self.assertEqual(row["results"], {"changed_readback_restored": 1})
        self.assertEqual(len(client.writes), 1)
        self.assertNotEqual(client.writes[0]["text"], original)
        self.assertEqual(client.parameters[0]["value"], original)
        self.assertEqual(client.capture_count, 1)
        self.assertEqual(client.restore_count, 1)

    def lossy_expander(self):
        original = expander_attack(0.75, "3FE8000000000000")
        client = LossyStateClient([number("lookahead", 0, kind="Toggle"), original])
        client.lossy_overrides["attack"] = expander_attack(math.nextafter(0.75, -math.inf), "3FE7FFFFFFFFFFFF")
        return client

    def test_lossy_capsule_stays_a_failure_in_the_default_strict_mode(self):
        client = self.lossy_expander()
        runner = self.runner(client)
        with self.assertRaises(RestorationFailure):
            runner.run_fixture(EXPANDER_TARGET, parameters={"lookahead"})
        self.assertEqual([write["param"] for write in client.writes], ["lookahead"])
        self.assertEqual(client.restore_count, 1)

    def test_explicit_repair_restores_synthetic_attack_bits_without_rebasing_the_baseline(self):
        client = self.lossy_expander()
        runner = self.runner(client, repair_captured_scalars=True, repair_parameters={"Expander": {"attack"}})
        row = runner.run_fixture(EXPANDER_TARGET, parameters={"lookahead"})
        self.assertEqual([write["param"] for write in client.writes], ["lookahead", "attack"])
        self.assertEqual(client.writes[1]["float_bits"], "3FE8000000000000")
        self.assertEqual(client.restore_count, 1)
        self.assertEqual(client.capture_count, 1)
        self.assertEqual(value_identity(client.parameters[1])[2], "3FE8000000000000")
        baseline = json.loads((self.journal.directory / row["baseline"]["file"]).read_text())
        self.assertEqual(value_identity(baseline["records"][1])[2], "3FE8000000000000")
        self.assertEqual(row["restoration"], "capsule_plus_scalar_correction")
        self.assertEqual(runner.summary["repair_parameters"], {"Expander": ["attack"]})
        self.assertEqual(runner.summary["repair_parameters_used"], {"Expander": ["attack"]})
        self.assertEqual(runner.counts["captured_scalar_corrections"], 1)
        self.assertEqual(runner.counts["changed_readback_restored"], 1)
        correction = next(event for event in self.events() if event["event"] == "captured_scalar_correction_confirmed")
        self.assertEqual(correction["parameter"], "attack")
        self.assertTrue(correction["exact_original_independently_confirmed"])
        phases = [event["phase"] for event in self.events() if event["event"] == "transaction_phase"]
        self.assertIn("RestorePartialKnown", phases)
        self.assertNotIn("RestorationFailed", phases)
        self.assertEqual(Journal.pending(self.journal.ledger_path), {})

    def test_hybrid_repair_never_runs_after_an_unknown_load(self):
        client = self.lossy_expander()
        client.unknown_load = True
        runner = self.runner(client, repair_captured_scalars=True, repair_parameters={"Expander": {"attack"}})
        with self.assertRaises(UnknownMutation):
            runner.run_fixture(EXPANDER_TARGET, parameters={"lookahead"})
        self.assertEqual([write["param"] for write in client.writes], ["lookahead"])
        self.assertEqual(client.restore_count, 1)

    def test_unaccepted_load_cannot_enter_captured_scalar_repair(self):
        client = self.lossy_expander()
        client.false_load = True
        runner = self.runner(client, repair_captured_scalars=True, repair_parameters={"Expander": {"attack"}})
        with self.assertRaises(RestorationFailure):
            runner.run_fixture(EXPANDER_TARGET, parameters={"lookahead"})
        self.assertEqual([write["param"] for write in client.writes], ["lookahead"])
        self.assertFalse(any(event.get("phase") == "RestorePartialKnown" for event in self.events()))

    def test_hybrid_correction_unknown_stops_without_another_load_or_write(self):
        client = self.lossy_expander()
        client.behaviors["attack"] = "public_unknown"
        runner = self.runner(client, repair_captured_scalars=True, repair_parameters={"Expander": {"attack"}})
        with self.assertRaises(UnknownMutation):
            runner.run_fixture(EXPANDER_TARGET, parameters={"lookahead"})
        self.assertEqual([write["param"] for write in client.writes], ["lookahead", "attack"])
        self.assertEqual(client.restore_count, 1)
        self.assertFalse(any(call["action"] == "release" for call in client.state_calls))

    def test_hybrid_correction_needs_independent_exact_readback_not_a_setter_claim(self):
        client = self.lossy_expander()
        client.lie_about_correction = True
        runner = self.runner(client, repair_captured_scalars=True, repair_parameters={"Expander": {"attack"}})
        with self.assertRaises(RestorationFailure):
            runner.run_fixture(EXPANDER_TARGET, parameters={"lookahead"})
        self.assertEqual(client.restore_count, 1)
        self.assertEqual(runner.counts["captured_scalar_correction_attempts"], 1)
        self.assertEqual(runner.counts["captured_scalar_corrections"], 0)
        self.assertEqual(runner.counts["changed_readback_restored"], 0)

    def test_toggle_difference_is_refused_before_any_other_scalar_is_corrected(self):
        client = self.lossy_expander()
        client.parameters.append(number("flag", 0, kind="Toggle"))
        client.lossy_overrides["flag"] = number("flag", 1, kind="Toggle")
        runner = self.runner(client, repair_captured_scalars=True, repair_parameters={"Expander": {"attack", "flag"}})
        with self.assertRaises(RestorationFailure):
            runner.run_fixture(EXPANDER_TARGET, parameters={"lookahead"})
        self.assertEqual([write["param"] for write in client.writes], ["lookahead"])
        self.assertEqual(runner.counts["captured_scalar_correction_attempts"], 0)

    def test_proeq_latched_bypass_restores_exact_float_bits_and_integer_in_both_modes(self):
        for mode in (False, True):
            for index, original in enumerate((0.0, -0.0, 1.0, 0, 1)):
                with self.subTest(digest=mode, original=original, carrier=type(original).__name__):
                    self.journal.close()
                    self.journal = Journal(Path(self.temporary.name) / f"latch-{mode}-{index}")
                    changed = (1 if original == 0 else 0) if type(original) is int else (1.0 if original == 0 else 0.0)
                    client = self.lossy_bypass(original, changed)
                    expected = value_identity(client.parameters[0])
                    runner = self.runner(client, repair_captured_scalars=True,
                                         repair_parameters={"Pro EQ": {"bypass"}}, use_parameter_digest=mode)
                    row = runner.run_fixture(PRO_EQ_TARGET)
                    self.assertEqual(row["results"], {"changed_readback_restored": 1})
                    self.assertEqual([write["param"] for write in client.writes], ["bypass", "bypass"])
                    self.assertEqual(value_identity(client.parameters[0]), expected)
                    if type(original) is float:
                        self.assertEqual(client.writes[1]["float_bits"], struct.pack(">d", original).hex().upper())
                    else:
                        self.assertEqual(client.writes[1]["value"], original)
                        self.assertNotIn("float_bits", client.writes[1])
                    self.assertEqual(client.capture_count, 1)
                    self.assertEqual(client.restore_count, 1)
                    self.assertEqual(runner.counts["captured_scalar_corrections"], 1)
                    self.assertEqual(runner.summary["repair_parameters_used"], {"Pro EQ": ["bypass"]})
                    self.assertTrue(row["capsule_released"])
                    self.assertEqual(Journal.pending(self.journal.ledger_path), {})
                    latch_reads = [event for event in self.events() if event["event"] == "captured_latch_baseline_observation"]
                    self.assertEqual([event["consecutive_complete_matches"] for event in latch_reads], [1, 2])
                    self.assertTrue(all(event["exact_typed_metadata"] for event in latch_reads))
                    if mode:
                        checkpoints = [event["checkpoint"] for event in self.events()
                                       if event["event"] == "parameter_digest_full_checkpoint"]
                        self.assertIn("hybrid_repair_complete", checkpoints)
                    else:
                        observations = [event for event in self.events() if event["event"] == "state_baseline_observation"]
                        self.assertTrue(any(event["consecutive_matches"] == 2 for event in observations))

    def test_latched_bypass_does_not_enable_hybrid_or_an_unlisted_pair(self):
        for mode in (False, True):
            for configured in (False, True):
                with self.subTest(digest=mode, configured=configured):
                    self.journal.close()
                    self.journal = Journal(Path(self.temporary.name) / f"no-latch-{mode}-{configured}")
                    client = self.lossy_bypass()
                    options = {"repair_captured_scalars": True, "repair_parameters": {"Pro EQ": {"gain"}}} if configured else {}
                    runner = self.runner(client, use_parameter_digest=mode, **options)
                    with self.assertRaises(RestorationFailure):
                        runner.run_fixture(PRO_EQ_TARGET)
                    self.assertEqual([write["param"] for write in client.writes], ["bypass"])
                    self.assertEqual(client.restore_count, 1)
                    self.assertEqual(runner.counts["captured_scalar_correction_attempts"], 0)

    def test_latched_bypass_refuses_other_plugin_class_name_and_source(self):
        for variant in ("plugin", "class", "name", "source"):
            with self.subTest(variant=variant):
                self.journal.close()
                self.journal = Journal(Path(self.temporary.name) / ("latch-role-" + variant))
                client = self.lossy_bypass()
                target = PRO_EQ_TARGET
                parameter_name = "bypass"
                if variant == "plugin":
                    target = TARGET
                elif variant == "class":
                    client.live_class = INSTANCE["class_uid"]
                elif variant == "name":
                    parameter_name = "otherToggle"
                    client.parameters[0]["name"] = parameter_name
                    client.lossy_overrides = {parameter_name: number(parameter_name, 1.0, kind="Toggle")}
                runner = self.runner(client, repair_captured_scalars=True,
                                     repair_parameters={target["plugin"]: {parameter_name}})
                if variant == "source":
                    provider = runner.snapshot(target)
                    provider.source = "controller"
                    with self.assertRaises(AcceptanceFailure):
                        runner.repairable_parameter(provider, provider.records[0], provider.records[0])
                    self.assertEqual(client.writes, [])
                else:
                    with self.assertRaises(RestorationFailure):
                        runner.run_fixture(target)
                    self.assertEqual(len(client.writes), 1)
                    self.assertEqual(runner.counts["captured_scalar_correction_attempts"], 0)

    def test_latched_bypass_rejects_metadata_or_carrier_drift_before_any_correction(self):
        mutations = {
            "kind": lambda p: p.update(native_kind="Float"),
            "range": lambda p: p.update(max=2.0),
            "default": lambda p: p.update(default=1.0),
            "support": lambda p: p["support"].update(normalized=True),
            "read_only": lambda p: p.update(readOnly=True, writable=False),
            "disabled": lambda p: p.update(enabled=False, writable=False),
            "carrier": lambda p: p.update(native_value={"variant_tag": 1, "data": {"kind": "integer", "value": 1}}, value=1),
            "variant_tag": lambda p: p["native_value"].update(variant_tag=1),
            "nonbinary_value": lambda p: p.update(number("bypass", 0.75, kind="Toggle")),
            "typed_min": lambda p: p.update(native_min={"variant_tag": 2, "data": {
                "kind": "float", "value": 0.0, "bits_hex": "0000000000000001"}}),
        }
        for label, mutate in mutations.items():
            with self.subTest(field=label):
                self.journal.close()
                self.journal = Journal(Path(self.temporary.name) / ("latch-drift-" + label))
                client = self.lossy_bypass()
                mutate(client.lossy_overrides["bypass"])
                runner = self.runner(client, repair_captured_scalars=True, repair_parameters={"Pro EQ": {"bypass"}})
                with self.assertRaises(AcceptanceFailure):
                    runner.run_fixture(PRO_EQ_TARGET)
                self.assertEqual([write["param"] for write in client.writes], ["bypass"])
                self.assertEqual(runner.counts["captured_scalar_correction_attempts"], 0)

    def test_latched_bypass_requires_complete_indexed_access_and_typed_metadata(self):
        for missing in ("access_metadata_complete", "native_default", "native_support"):
            with self.subTest(missing=missing):
                self.journal.close()
                self.journal = Journal(Path(self.temporary.name) / ("latch-missing-" + missing))
                client = self.lossy_bypass()

                def omit(owner, page):
                    for parameter in page["params"]:
                        parameter.pop(missing, None)
                        parameter["native"]["native_description"].pop(missing, None)

                client.page_hook = omit
                runner = self.runner(client, repair_captured_scalars=True, repair_parameters={"Pro EQ": {"bypass"}})
                with self.assertRaises(RestorationFailure):
                    runner.run_fixture(PRO_EQ_TARGET)
                self.assertEqual([write["param"] for write in client.writes], ["bypass"])
                self.assertEqual(runner.counts["captured_scalar_correction_attempts"], 0)

    def test_registered_bypass_with_an_invalid_captured_kind_value_or_carrier_is_not_repaired(self):
        for invalid in ("kind", "value", "tag"):
            with self.subTest(invalid=invalid):
                self.journal.close()
                self.journal = Journal(Path(self.temporary.name) / ("latch-baseline-" + invalid))
                client = self.lossy_bypass()
                if invalid == "kind":
                    client.parameters[0]["native_kind"] = "Float"
                    client.lossy_overrides["bypass"]["native_kind"] = "Float"
                elif invalid == "value":
                    client.parameters[0].update(number("bypass", 0.25, kind="Toggle"))
                    client.lossy_overrides["bypass"].update(number("bypass", 0.75, kind="Toggle"))
                else:
                    client.parameters[0]["native_value"]["variant_tag"] = 1
                    client.lossy_overrides["bypass"]["native_value"]["variant_tag"] = 1
                runner = self.runner(client, repair_captured_scalars=True,
                                     repair_parameters={"Pro EQ": {"bypass"}})
                with self.assertRaises(RestorationFailure):
                    runner.run_fixture(PRO_EQ_TARGET)
                self.assertEqual([write["param"] for write in client.writes], ["bypass"])
                self.assertEqual(runner.counts["captured_scalar_correction_attempts"], 0)

    def test_latched_bypass_fresh_default_drift_stops_before_correction(self):
        client = self.lossy_bypass()
        original_call = client.call

        def drift_named(name, args):
            result = original_call(name, args)
            if name == "plugin_parameters" and args.get("names") and client.restore_count:
                parameter = result["structuredContent"]["result"]["params"][0]
                parameter["default"] = 1.0
                parameter["native_default"] = {"variant_tag": 2, "data": {
                    "kind": "float", "value": 1.0, "bits_hex": "3FF0000000000000"}}
            return result

        client.call = drift_named
        runner = self.runner(client, repair_captured_scalars=True, repair_parameters={"Pro EQ": {"bypass"}})
        with self.assertRaises(RestorationFailure):
            runner.run_fixture(PRO_EQ_TARGET)
        self.assertEqual([write["param"] for write in client.writes], ["bypass"])
        self.assertEqual(runner.counts["captured_scalar_correction_attempts"], 0)

    def test_latched_bypass_correction_receipt_and_independent_readback_keep_exact_metadata(self):
        for field in ("receipt", "readback"):
            with self.subTest(field=field):
                self.journal.close()
                self.journal = Journal(Path(self.temporary.name) / ("latch-after-" + field))
                client = self.lossy_bypass()
                original_call = client.call

                def drift(name, args):
                    result = original_call(name, args)
                    parameter = None
                    if field == "receipt" and name == "plugin_parameter_set" and len(client.writes) == 2:
                        parameter = result["structuredContent"]["result"]["after"]
                    if field == "readback" and name == "plugin_parameters" and args.get("names") and len(client.writes) == 2:
                        parameter = result["structuredContent"]["result"]["params"][0]
                    if parameter is not None:
                        parameter["default"] = 1.0
                        parameter["native_default"] = {"variant_tag": 2, "data": {
                            "kind": "float", "value": 1.0, "bits_hex": "3FF0000000000000"}}
                    return result

                client.call = drift
                runner = self.runner(client, repair_captured_scalars=True, repair_parameters={"Pro EQ": {"bypass"}})
                with self.assertRaises(RestorationFailure):
                    runner.run_fixture(PRO_EQ_TARGET)
                self.assertEqual([write["param"] for write in client.writes], ["bypass", "bypass"])
                self.assertEqual(client.restore_count, 1)
                self.assertEqual(runner.counts["captured_scalar_corrections"], 0)

    def test_unregistered_toggle_or_list_prevents_even_an_allowed_bypass_correction(self):
        for kind in ("Toggle", "List"):
            with self.subTest(kind=kind):
                self.journal.close()
                self.journal = Journal(Path(self.temporary.name) / ("latch-other-" + kind))
                client = self.lossy_bypass()
                client.parameters.append(number("otherAction", 0.0, kind=kind))
                client.lossy_overrides["otherAction"] = number("otherAction", 1.0, kind=kind)
                runner = self.runner(client, repair_captured_scalars=True,
                                     repair_parameters={"Pro EQ": {"bypass", "otherAction"}})
                with self.assertRaises(RestorationFailure):
                    runner.run_fixture(PRO_EQ_TARGET, parameters={"bypass"})
                self.assertEqual([write["param"] for write in client.writes], ["bypass"])
                self.assertEqual(runner.counts["captured_scalar_correction_attempts"], 0)

    def test_final_latch_pages_refuse_late_default_support_or_carrier_drift_in_either_read(self):
        for mode in (False, True):
            for field in ("default", "support", "carrier"):
                for attempt in (1, 2):
                    with self.subTest(digest=mode, field=field, attempt=attempt):
                        self.journal.close()
                        self.journal = Journal(Path(self.temporary.name) / f"latch-final-{mode}-{field}-{attempt}")
                        client = self.lossy_bypass()
                        final_reads = []

                        def late_drift(owner, page):
                            events = self.events()
                            corrections = [event["sequence"] for event in events
                                           if event["event"] == "captured_scalar_correction_confirmed"]
                            if not corrections:
                                return
                            confirmed = any(event["sequence"] > corrections[-1]
                                            and ((event["event"] == "state_baseline_observation"
                                                  and event.get("consecutive_matches") == 2)
                                                 or (event["event"] == "parameter_digest_baseline_confirmed"
                                                     and event.get("checkpoint") == "restored_baseline"))
                                            for event in events)
                            if not confirmed:
                                return
                            final_reads.append(1)
                            if len(final_reads) != attempt:
                                return
                            for parameter in (page["params"][0], page["params"][0]["native"]["native_description"]):
                                if field == "default":
                                    parameter["default"] = 1.0
                                    parameter["native_default"] = {"variant_tag": 2, "data": {
                                        "kind": "float", "value": 1.0, "bits_hex": "3FF0000000000000"}}
                                elif field == "support":
                                    parameter["support"]["normalized"] = True
                                    parameter["native_support"]["normalized"] = True
                                else:
                                    parameter["native_value"] = {"variant_tag": 1, "data": {"kind": "integer", "value": 0}}
                                    parameter["value"] = 0

                        client.page_hook = late_drift
                        runner = self.runner(client, repair_captured_scalars=True,
                                             repair_parameters={"Pro EQ": {"bypass"}}, use_parameter_digest=mode)
                        with self.assertRaises(AcceptanceFailure):
                            runner.run_fixture(PRO_EQ_TARGET)
                        self.assertEqual(len(final_reads), attempt)
                        self.assertEqual([write["param"] for write in client.writes], ["bypass", "bypass"])
                        self.assertEqual(client.restore_count, 1)
                        self.assertEqual(runner.counts["changed_readback_restored"], 0)
                        self.assertFalse(any(call["action"] == "release" for call in client.state_calls))

    def test_latched_bypass_unknown_load_correction_or_changed_context_never_retries(self):
        for failure in ("load", "correction", "context"):
            with self.subTest(failure=failure):
                self.journal.close()
                self.journal = Journal(Path(self.temporary.name) / ("latch-unknown-" + failure))
                client = self.lossy_bypass()
                if failure == "load":
                    client.unknown_load = True
                elif failure == "context":
                    client.document_changed_by_load = True
                else:
                    original_call = client.call

                    def unknown_correction(name, args):
                        if name == "plugin_parameter_set" and client.restore_count:
                            client.behaviors["bypass"] = "public_unknown"
                        return original_call(name, args)

                    client.call = unknown_correction
                runner = self.runner(client, repair_captured_scalars=True,
                                     repair_parameters={"Pro EQ": {"bypass"}}, use_parameter_digest=True)
                with self.assertRaises((UnknownMutation, ContextChanged)):
                    runner.run_fixture(PRO_EQ_TARGET)
                self.assertEqual(len(client.writes), 2 if failure == "correction" else 1)
                self.assertEqual(client.restore_count, 1)
                self.assertFalse(any(call["action"] == "release" for call in client.state_calls))

    def test_hybrid_read_only_drift_is_observed_only_with_complete_opt_in_proof(self):
        meter = {**number("meter", 0.0, readonly=True), "access_metadata_complete": True}
        client = self.lossy_expander()
        client.parameters.append(meter)
        client.lossy_overrides["meter"] = number("meter", 0.75, readonly=True)
        runner = self.runner(client, read_only_observations=True, repair_captured_scalars=True, repair_parameters={"Expander": {"attack"}})
        row = runner.run_fixture(EXPANDER_TARGET, parameters={"lookahead"})
        self.assertEqual(row["results"], {"changed_readback_restored": 1})
        self.assertEqual([write["param"] for write in client.writes], ["lookahead", "attack"])
        observations = [event for event in self.events() if event["event"] == "captured_scalar_repair_observation"]
        self.assertTrue(any("meter" in event["read_only_changed"] for event in observations))
        self.assertEqual(row["read_only_values"], "observed_not_restored")

    def test_hybrid_incomplete_read_only_proof_is_never_repaired_or_silently_excluded(self):
        client = self.lossy_expander()
        client.parameters.append(number("meter", 0.0, readonly=True))
        client.lossy_overrides["meter"] = number("meter", 0.75, readonly=True)
        runner = self.runner(client, read_only_observations=True, repair_captured_scalars=True, repair_parameters={"Expander": {"attack"}})
        with self.assertRaises(RestorationFailure):
            runner.run_fixture(EXPANDER_TARGET, parameters={"lookahead"})
        self.assertEqual([write["param"] for write in client.writes], ["lookahead"])

    def test_hybrid_schema_change_stops_before_scalar_correction(self):
        client = self.lossy_expander()
        client.schema_drift = True
        runner = self.runner(client, repair_captured_scalars=True, repair_parameters={"Expander": {"attack"}})
        with self.assertRaises(ContextChanged):
            runner.run_fixture(EXPANDER_TARGET, parameters={"lookahead"})
        self.assertEqual([write["param"] for write in client.writes], ["lookahead"])
        self.assertEqual(client.restore_count, 1)

    def test_hybrid_three_pass_bound_stops_a_cyclic_scalar_dependency(self):
        client = LossyStateClient([number("lookahead", 0, kind="Toggle"), number("low", 0.25), number("high", 0.5)])
        client.lossy_overrides["low"] = number("low", 0.99)
        client.couplings = {"low": "high", "high": "low"}
        runner = self.runner(client, repair_captured_scalars=True, repair_parameters={"Expander": {"low", "high"}})
        with self.assertRaises(RestorationFailure):
            runner.run_fixture(EXPANDER_TARGET, parameters={"lookahead"})
        self.assertEqual([write["param"] for write in client.writes], ["lookahead", "low", "high", "low"])
        self.assertEqual(client.restore_count, 1)
        self.assertEqual(runner.counts["captured_scalar_corrections"], 3)
        phases = [event["phase"] for event in self.events() if event["event"] == "transaction_phase"]
        self.assertEqual(phases[-1], "RestorationFailed")
        self.assertEqual(phases[-2], "RestorePartialKnown")

    def test_hybrid_thirty_two_correction_bound_is_total_across_passes(self):
        fields = [number(f"gain{i}", 0.25) for i in range(32)]
        client = LossyStateClient([number("lookahead", 0, kind="Toggle"), *fields])
        client.lossy_overrides = {f"gain{i}": number(f"gain{i}", 0.5) for i in range(31)}
        client.couplings = {"gain29": "gain31", "gain30": "gain0"}
        runner = self.runner(client, repair_captured_scalars=True, repair_parameters={"Expander": {f"gain{i}" for i in range(32)}})
        with self.assertRaises(RestorationFailure):
            runner.run_fixture(EXPANDER_TARGET, parameters={"lookahead"})
        self.assertEqual(runner.counts["captured_scalar_correction_attempts"], 31)
        self.assertEqual(len(client.writes), 32)
        self.assertEqual(client.restore_count, 1)
        self.assertEqual(runner.counts["changed_readback_restored"], 0)

    def test_hybrid_requires_a_nonempty_explicit_allowlist_before_any_native_call(self):
        client = self.lossy_expander()
        with self.assertRaises(AcceptanceFailure):
            self.runner(client, repair_captured_scalars=True)
        self.assertEqual(client.calls, [])
        self.assertEqual(client.writes, [])

    def test_allowlist_configuration_does_not_implicitly_enable_hybrid_mode(self):
        client = self.lossy_expander()
        with self.assertRaises(AcceptanceFailure):
            self.runner(client, repair_parameters={"Expander": {"attack"}})
        self.assertEqual(client.calls, [])

    def test_wrong_plugin_allowlist_cannot_correct_a_same_named_parameter(self):
        client = self.lossy_expander()
        runner = self.runner(client, repair_captured_scalars=True, repair_parameters={"Other Effect": {"attack"}})
        with self.assertRaises(RestorationFailure):
            runner.run_fixture(EXPANDER_TARGET, parameters={"lookahead"})
        self.assertEqual([write["param"] for write in client.writes], ["lookahead"])
        self.assertEqual(runner.summary["repair_parameters_used"], {})

    def test_unlisted_scalar_difference_stops_before_an_allowed_scalar_is_corrected(self):
        client = self.lossy_expander()
        client.parameters.append(number("gain", 0.25))
        client.lossy_overrides["gain"] = number("gain", 0.75)
        runner = self.runner(client, repair_captured_scalars=True, repair_parameters={"Expander": {"attack"}})
        with self.assertRaises(RestorationFailure):
            runner.run_fixture(EXPANDER_TARGET, parameters={"lookahead"})
        self.assertEqual([write["param"] for write in client.writes], ["lookahead"])
        self.assertEqual(runner.counts["captured_scalar_correction_attempts"], 0)

    def test_explicitly_allowlisted_model_role_is_still_not_a_scalar_repair(self):
        client = self.lossy_expander()
        client.parameters.append(number("opt.eqmodel", 0.0, lower=0.0, upper=2.0))
        client.lossy_overrides["opt.eqmodel"] = number("opt.eqmodel", 1.0, lower=0.0, upper=2.0)
        runner = self.runner(client, repair_captured_scalars=True,
                             repair_parameters={"Fat Channel": {"attack", "opt.eqmodel"}})
        with self.assertRaises(RestorationFailure):
            runner.run_fixture({**TARGET, "plugin": "Fat Channel"}, parameters={"lookahead"})
        self.assertEqual([write["param"] for write in client.writes], ["lookahead"])

    def test_allowlisted_integer_correction_uses_the_exact_original_integer(self):
        client = LossyStateClient([number("lookahead", 0, kind="Toggle"),
                                  number("counter", 3, kind="Integer", lower=0, upper=10)])
        client.lossy_overrides["counter"] = number("counter", 5, kind="Integer", lower=0, upper=10)
        runner = self.runner(client, repair_captured_scalars=True,
                             repair_parameters={"Expander": {"counter"}})
        row = runner.run_fixture(EXPANDER_TARGET, parameters={"lookahead"})
        self.assertEqual(row["results"], {"changed_readback_restored": 1})
        self.assertEqual(client.writes[1]["value"], 3)
        self.assertNotIn("float_bits", client.writes[1])

    def test_repair_pair_parser_preserves_exact_names_and_splits_only_once(self):
        self.assertEqual(parse_repair_parameters(["Expander:attack", "Fixture:param:with:colons"]),
                         {"Expander": frozenset({"attack"}), "Fixture": frozenset({"param:with:colons"})})
        for pairs in (["Expander"], ["Expander:"], [":attack"], ["Expander:a\0b"],
                      ["x" * 257 + ":attack"], ["Expander:" + "x" * 513],
                      [f"Expander:field{i}" for i in range(33)]):
            with self.subTest(pairs=pairs):
                with self.assertRaises(AcceptanceFailure):
                    parse_repair_parameters(pairs)

    def test_digest_is_opt_in_and_default_keeps_full_provider_reads(self):
        client = DigestSweepClient()
        runner = self.runner(client)
        row = runner.run_fixture(TARGET)
        self.assertEqual(client.digest_calls, 0)
        self.assertEqual(client.pages, 5)
        self.assertNotIn("parameter_verification", row)
        self.assertEqual(row["results"], {"changed_readback_restored": 1})

    def test_digest_mode_has_two_fresh_matches_and_bounded_full_checkpoints(self):
        client = DigestSweepClient([number(f"gain{i}", 0.25) for i in range(33)])
        runner = self.runner(client, use_parameter_digest=True)
        row = runner.run_fixture(TARGET)
        self.assertEqual(row["results"], {"changed_readback_restored": 33})
        self.assertEqual(client.pages, 6)
        self.assertEqual(client.restore_count, 33)
        self.assertEqual(len(client.writes), 33)
        checkpoints = [event for event in self.events() if event["event"] == "parameter_digest_full_checkpoint"]
        self.assertEqual([event["checkpoint"] for event in checkpoints],
                         ["before_capture", "immediately_after_capture", "completed_cases_32",
                          "fixture_end_before_release", "after_release"])
        self.assertTrue(all(event["consecutive_fresh_digest_matches"] == 2
                            and not event["atomic_snapshot"] and not event["opaque_contents_verified"] for event in checkpoints))
        observations = [event for event in self.events() if event["event"] == "parameter_digest_observation"]
        self.assertEqual(len({event["fresh_rpc"] for event in observations}), len(observations))
        self.assertTrue(all(event["matches"] for event in observations))
        self.assertFalse(row["atomic_snapshot"])
        self.assertIn("full_page_checkpoints", row["parameter_verification"])
        self.assertEqual(Journal.pending(self.journal.ledger_path), {})

    def test_changed_second_initial_digest_stops_before_capture(self):
        client = DigestSweepClient()
        client.digest_hook = lambda owner, result: result.update(all_values_sha256="0" * 64) if owner.digest_calls == 2 else None
        runner = self.runner(client, use_parameter_digest=True)
        with self.assertRaises(RestorationFailure):
            runner.run_fixture(TARGET)
        self.assertEqual(client.digest_calls, 2)
        self.assertEqual(client.capture_count, 0)
        self.assertEqual(client.writes, [])
        self.assertEqual(client.pages, 2)

    def test_changed_second_restoration_digest_is_not_hidden_by_matching_full_diagnosis(self):
        client = DigestSweepClient([number("gain", 0.25), number("mix", 0.5)])
        after_load = []

        def changed_second(owner, result):
            if owner.restore_count:
                after_load.append(owner.digest_calls)
                if len(after_load) == 2:
                    result["non_read_only_values_sha256"] = "0" * 64

        client.digest_hook = changed_second
        runner = self.runner(client, use_parameter_digest=True)
        with self.assertRaises(RestorationFailure):
            runner.run_fixture(TARGET)
        self.assertEqual(len(client.writes), 1)
        self.assertEqual(client.restore_count, 1)
        self.assertFalse(any(call["action"] == "release" for call in client.state_calls))
        self.assertEqual(runner.counts["changed_load_independently_restored"], 0)
        diagnosis = [event for event in self.events() if event["event"] == "parameter_digest_mismatch_diagnosis"]
        self.assertEqual(len(diagnosis), 1)
        self.assertEqual(diagnosis[0]["changed_parameters"], [])

    def test_digest_readonly_drift_requires_existing_explicit_observation_mode(self):
        client = DigestSweepClient([number("gain", 0.25), number("meter", 0.0, readonly=True)])
        client.digest_readonly_drift = True
        runner = self.runner(client, read_only_observations=True, use_parameter_digest=True)
        row = runner.run_fixture(TARGET)
        self.assertEqual(row["results"], {"changed_readback_restored": 1})
        self.assertEqual(row["read_only_values"], "observed_not_restored")
        observations = [event for event in self.events() if event["event"] == "parameter_digest_observation"]
        self.assertTrue(any(not event["all_values_equal"] for event in observations))
        self.assertTrue(all(event["matches"] for event in observations))
        self.assertFalse(row["opaque_contents_verified"])

    def test_digest_strict_default_refuses_readonly_value_drift(self):
        client = DigestSweepClient([number("gain", 0.25), number("meter", 0.0, readonly=True)])
        client.digest_readonly_drift = True
        runner = self.runner(client, use_parameter_digest=True)
        with self.assertRaises(RestorationFailure):
            runner.run_fixture(TARGET)
        self.assertEqual(len(client.writes), 1)
        self.assertEqual(client.restore_count, 1)
        self.assertEqual(runner.counts["changed_load_independently_restored"], 0)

    def test_digest_contract_rejects_missing_malformed_or_incomplete_native_results(self):
        mutations = [("missing_hash", lambda result: result.pop("all_values_sha256")),
                     ("bad_hash", lambda result: result.update(all_values_sha256="z" * 64)),
                     ("upper_hash", lambda result: result.update(all_values_sha256=result["all_values_sha256"].upper())),
                     ("access", lambda result: result.update(access_complete=False)),
                     ("missing_meta", lambda result: result.pop("coverage")),
                     ("fields", lambda result: result.update(fields_covered={})),
                     ("version", lambda result: result.update(hash_format_version="future-v2")),
                     ("atomic", lambda result: result.update(atomic_snapshot=True)),
                     ("opaque", lambda result: result.update(opaque_contents_verified=True)),
                     ("accepted", lambda result: result.update(outcome="accepted")),
                     ("native_error", lambda result: result.update(error="incomplete native access")),
                     ("missing_error_hashes", lambda result: (result.clear(), result.update(ok=False, error="owner changed")))]
        for label, mutate in mutations:
            with self.subTest(label=label):
                client = DigestSweepClient()
                client.digest_hook = lambda owner, result: mutate(result)
                runner = self.runner(client, use_parameter_digest=True)
                with self.assertRaises(AcceptanceFailure):
                    runner.run_fixture(TARGET)
                self.assertEqual(client.digest_calls, 1)
                self.assertEqual(client.capture_count, 0)
                self.assertEqual(client.writes, [])
                self.assertEqual(client.pages, 1)

    def test_digest_contract_refuses_foreign_instance_source_count_or_schema(self):
        for field, value in (("instance", {**INSTANCE, "object_uid": "different"}), ("source", "controller"),
                             ("count", 2), ("schema_sha256", "0" * 64)):
            with self.subTest(field=field):
                client = DigestSweepClient()
                client.digest_hook = lambda owner, result: result.update({field: value})
                runner = self.runner(client, use_parameter_digest=True)
                with self.assertRaises(ContextChanged):
                    runner.run_fixture(TARGET)
                self.assertEqual(client.capture_count, 0)
                self.assertEqual(client.writes, [])

    def test_digest_outer_error_cannot_publish_an_otherwise_matching_hash(self):
        client = DigestSweepClient()
        original = client.call

        def errored_receipt(name, args):
            receipt = original(name, args)
            if name == "plugin_parameter_digest":
                receipt["structuredContent"]["error"] = ""
            return receipt

        client.call = errored_receipt
        runner = self.runner(client, use_parameter_digest=True)
        with self.assertRaises(AcceptanceFailure):
            runner.run_fixture(TARGET)
        self.assertEqual(client.capture_count, 0)
        self.assertEqual(client.writes, [])

    def test_digest_constructor_requires_an_explicit_boolean(self):
        client = DigestSweepClient()
        with self.assertRaises(AcceptanceFailure):
            self.runner(client, use_parameter_digest="yes")
        self.assertEqual(client.calls, [])

    def test_digest_mode_refuses_old_public_metadata_before_any_capture(self):
        client = SweepClient()
        runner = self.runner(client, use_parameter_digest=True)
        with self.assertRaises(AcceptanceFailure):
            runner.run_fixture(TARGET)
        self.assertEqual(client.capture_count, 0)
        self.assertEqual(client.writes, [])
        self.assertFalse(any(name == "plugin_parameter_digest" for name, _ in client.calls))

    def test_full_checkpoint_after_capture_catches_default_support_and_value_drift(self):
        def drift_default(parameter):
            parameter["default"] = 0.125
            parameter["native_default"] = {"variant_tag": 2, "data": {
                "kind": "float", "value": 0.125, "bits_hex": "3FC0000000000000"}}

        def drift_support(parameter):
            parameter["native_support"]["normalized"] = True
            parameter["support"]["normalized"] = True

        for label, mutation in (("default", drift_default), ("support", drift_support),
                                ("display", lambda parameter: parameter.update(text="different display"))):
            with self.subTest(label=label):
                self.journal.close()
                self.journal = Journal(Path(self.temporary.name) / label)
                client = DigestSweepClient()

                def page_hook(owner, page):
                    if owner.capture_count:
                        parameter = page["params"][0]
                        mutation(parameter)
                        mutation(parameter["native"]["native_description"])

                client.page_hook = page_hook
                runner = self.runner(client, use_parameter_digest=True)
                with self.assertRaises(AcceptanceFailure):
                    runner.run_fixture(TARGET)
                self.assertEqual(client.capture_count, 1)
                self.assertEqual(client.restore_count, 0)
                self.assertEqual(client.writes, [])

    def test_periodic_full_checkpoint_failure_stops_before_case_thirty_three(self):
        client = DigestSweepClient([number(f"gain{i}", 0.25) for i in range(35)])

        def corrupt_full_page(owner, page):
            if len(owner.writes) >= 32:
                page["params"][0].pop("native_default")
                page["params"][0]["native"]["native_description"].pop("native_default")

        client.page_hook = corrupt_full_page
        runner = self.runner(client, use_parameter_digest=True)
        with self.assertRaises(AcceptanceFailure):
            runner.run_fixture(TARGET)
        self.assertEqual(len(client.writes), 32)
        self.assertEqual(client.restore_count, 32)
        self.assertFalse(any(call["action"] == "release" for call in client.state_calls))
        self.assertEqual(runner.counts["fixtures_state_workflow_completed"], 0)

    def test_final_full_checkpoint_failure_cannot_be_finished_as_verified(self):
        client = DigestSweepClient()

        def corrupt_final_page(owner, page):
            if owner.writes:
                parameter = page["params"][0]
                parameter["text"] = parameter["native"]["native_description"]["text"] = "different final display"

        client.page_hook = corrupt_final_page
        runner = self.runner(client, use_parameter_digest=True)
        with self.assertRaises(AcceptanceFailure):
            runner.run_fixture(TARGET)
        self.assertEqual(runner.counts["changed_readback_restored"], 1)
        self.assertEqual(runner.counts["fixtures_state_workflow_completed"], 0)
        with self.assertRaises(AcceptanceFailure):
            runner.finish(True)
        self.assertFalse(any(call["action"] == "release" for call in client.state_calls))

    def test_digest_hybrid_repair_keeps_full_prevalidation_and_final_checkpoint(self):
        client = DigestSweepClient([number("lookahead", 0, kind="Toggle"), number("attack", 0.75)])
        client.lossy_overrides["attack"] = number("attack", math.nextafter(0.75, -math.inf))
        runner = self.runner(client, use_parameter_digest=True, repair_captured_scalars=True,
                             repair_parameters={"Expander": {"attack"}})
        row = runner.run_fixture(EXPANDER_TARGET, parameters={"lookahead"})
        self.assertEqual(row["results"], {"changed_readback_restored": 1})
        self.assertEqual([write["param"] for write in client.writes], ["lookahead", "attack"])
        self.assertEqual(value_identity(client.parameters[1])[2], "3FE8000000000000")
        checkpoints = [event["checkpoint"] for event in self.events() if event["event"] == "parameter_digest_full_checkpoint"]
        self.assertEqual(checkpoints.count("hybrid_repair_prevalidation"), 2)
        self.assertIn("hybrid_repair_complete", checkpoints)
        self.assertIn("fixture_end_before_release", checkpoints)

    def test_digest_hybrid_prevalidation_default_drift_cannot_trigger_scalar_correction(self):
        client = DigestSweepClient([number("lookahead", 0, kind="Toggle"), number("attack", 0.75)])
        client.lossy_overrides["attack"] = number("attack", 0.5)

        def changed_default(owner, page):
            if owner.restore_count:
                for parameter in (page["params"][0], page["params"][0]["native"]["native_description"]):
                    parameter["native_default"] = None
                    parameter["default"] = None

        client.page_hook = changed_default
        runner = self.runner(client, use_parameter_digest=True, repair_captured_scalars=True,
                             repair_parameters={"Expander": {"attack"}})
        with self.assertRaises(ContextChanged):
            runner.run_fixture(EXPANDER_TARGET, parameters={"lookahead"})
        self.assertEqual([write["param"] for write in client.writes], ["lookahead"])
        self.assertEqual(runner.counts["captured_scalar_correction_attempts"], 0)

    def test_digest_unknown_dispatch_and_load_never_gain_fallback_mutations(self):
        for dispatch in (True, False):
            with self.subTest(dispatch=dispatch):
                self.journal.close()
                self.journal = Journal(Path(self.temporary.name) / ("dispatch" if dispatch else "load"))
                client = DigestSweepClient([number("gain", 0.25), number("mix", 0.5)])
                if dispatch:
                    client.behaviors["gain"] = "public_unknown"
                else:
                    client.unknown_load = True
                runner = self.runner(client, use_parameter_digest=True)
                with self.assertRaises(UnknownMutation):
                    runner.run_fixture(TARGET)
                self.assertEqual(len(client.writes), 1)
                self.assertEqual(client.restore_count, 0 if dispatch else 1)
                self.assertFalse(any(call["action"] == "release" for call in client.state_calls))

    def test_digest_released_token_negative_check_retains_two_full_provider_reads(self):
        client = DigestSweepClient()
        runner = self.runner(client, use_parameter_digest=True, verify_released=True)
        row = runner.run_fixture(TARGET)
        self.assertTrue(row["released_token_refused"])
        last_restore = max(index for index, (name, args) in enumerate(client.calls)
                           if name == "plugin_state" and args["action"] == "restore")
        following = client.calls[last_restore + 1:]
        self.assertEqual(len([args for name, args in following if name == "plugin_parameters" and not args.get("names")]), 2)
        self.assertEqual(len([name for name, _ in following if name == "plugin_parameter_digest"]), 2)


if __name__ == "__main__":
    unittest.main()
