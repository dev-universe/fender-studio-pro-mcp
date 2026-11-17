#!/usr/bin/env python3
"""Offline state-token acceptance behavior. No DAW or native add-on is launched."""
import copy
import hashlib
import json
import math
from pathlib import Path
import struct
import tempfile
import unittest

from test_verify_all_parameters import DeviceClient, INSTANCE, TARGET, number
from verify_all_parameters import (
    AcceptanceFailure, ContextChanged, Journal, PRIVATE_ROOT, RestorationFailure,
    UnknownMutation, encoded, value_identity,
)
from verify_plugin_states import StateRunner, explicit_candidate


OTHER = {"plugin": "Other Effect", "channel": "FSP Verify FX Other", "slot": 0}
OTHER_INSTANCE = {**INSTANCE, "object_uid": "ABCDEFAB-1234-1234-1234-123456789ABC"}


class StateClient(DeviceClient):
    def __init__(self, parameters=None):
        super().__init__(parameters)
        self.targets = {TARGET["channel"]: self.parameters,
                        OTHER["channel"]: [number("gain", 0.75)]}
        self.target_name = TARGET["channel"]
        self.tokens = {}
        self.released = {}
        self.state_calls = []
        self.capture_count = 0
        self.restore_count = 0
        self.volatile_capture = self.ignore_load = self.false_load = False
        self.unknown_load = self.document_changed_by_load = False
        self.schema_drift = self.accept_released = self.accept_wrong_target = False
        self.bad_capture = self.mute_after_write = False
        self.delayed_restore = False
        self.readonly_drift = False
        self.pending_restore = None
        self.pending_reads = 0
        self.post_load_snapshots = 0

    def select_target(self, args):
        channel = args.get("channel")
        if channel in self.targets:
            self.target_name = channel
            self.parameters = self.targets[channel]
            self.instance = dict(INSTANCE if channel == TARGET["channel"] else OTHER_INSTANCE)

    def call(self, name, args):
        if name.startswith("plugin_"):
            self.select_target(args)
        if name == "operation_run":
            operation = args["operation"]
            if operation == "tracks":
                return self.receipt([{"name": channel, "channel": channel, "mediaType": "Audio",
                                      "eventCount": 0, "events": []}
                                     for channel in self.targets if not args["args"].get("name")
                                     or channel == args["args"]["name"]])
            if operation == "channels":
                return self.receipt([{"label": channel, "mute": self.muted,
                                      "recordArmed": False, "monitor": False} for channel in self.targets])
        if name == "plugin_parameters" and not args.get("names") and self.restore_count:
            self.post_load_snapshots += 1
            if self.pending_restore is not None:
                if self.pending_reads:
                    self.pending_reads -= 1
                else:
                    self.parameters = self.pending_restore
                    self.targets[self.target_name] = self.parameters
                    self.pending_restore = None
            if self.readonly_drift:
                meter = self.parameters[1]
                meter.update(number(meter["name"], meter["value"] + 0.01, readonly=True))
        if name == "plugin_parameter_set":
            if "text" in args:
                self.writes.append(copy.deepcopy(args))
                parameter = next(p for p in self.parameters if p["name"] == args["param"])
                before = copy.deepcopy(parameter)
                parameter["value"] = parameter["text"] = args["text"]
                parameter["native_value"]["data"]["value"] = args["text"]
                return self.receipt({"before": before, "after": copy.deepcopy(parameter)}, "applied_verified")
            result = super().call(name, args)
            if self.mute_after_write:
                self.muted = False
            return result
        if name != "plugin_state":
            return super().call(name, args)
        self.calls.append((name, copy.deepcopy(args)))
        self.state_calls.append(copy.deepcopy(args))
        action = args["action"]
        if action == "capture":
            self.capture_count += 1
            token = f"{self.capture_count:064X}"
            self.tokens[token] = {"parameters": copy.deepcopy(self.parameters), "instance": dict(self.instance)}
            payload = encoded(self.parameters)
            if self.volatile_capture:
                payload += str(self.capture_count).encode()
            return self.receipt({"ok": True, "token": "invalid" if self.bad_capture else token,
                                 "byte_count": len(payload), "sha256": hashlib.sha256(payload).hexdigest(),
                                 "serialization": "component_alias_IStorable", "verified": False,
                                 "instance": dict(self.instance)}, "applied_unverified")
        if action == "release":
            token = args["token"]
            self.released[token] = self.tokens.pop(token)
            return self.receipt({"ok": True, "released": True, "instance": dict(self.instance)}, "applied_unverified")
        if action != "restore":
            raise AssertionError(action)
        self.restore_count += 1
        token = args["token"]
        stored = self.tokens.get(token)
        if stored is None and self.accept_released:
            stored = self.released.get(token)
        if stored is None or (stored["instance"] != self.instance and not self.accept_wrong_target):
            return self.receipt({"native_outcome": "not_applied", "errors": ["invalid token or target"]},
                                "not_applied", "invalid token or target")
        if not self.ignore_load:
            if self.delayed_restore:
                self.pending_restore = copy.deepcopy(stored["parameters"])
                self.pending_reads = 1
            else:
                self.parameters = copy.deepcopy(stored["parameters"])
                self.targets[self.target_name] = self.parameters
        if self.schema_drift:
            self.parameters.append(number("unexpected", 0.5))
        if self.document_changed_by_load:
            self.document = "test:2"
        if self.unknown_load:
            raise TimeoutError("state load interrupted after dispatch")
        return self.receipt({"ok": True, "load_accepted": not self.false_load, "verified": False,
                             "instance": dict(self.instance)}, "applied_unverified")


class LifecycleReplyMixin:
    """Add actual native phase outcomes, with faults after a simulated dispatch."""
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.lifecycle_fault_phase = None
        self.lifecycle_fault_fields = {"native_outcome": "unknown"}
        self.lifecycle_fault_outer = {}
        self.lifecycle_fault_call_count = None

    def call(self, name, args):
        phase = None
        if name == "plugin_state":
            phase = ("released_restore" if args["action"] == "restore" and args["token"] in self.released
                     else args["action"])
        response = super().call(name, args)
        if phase is not None:
            receipt = response["structuredContent"]
            native = receipt["result"]
            native.setdefault("outcome", "not_applied" if receipt["outcome"] == "not_applied" else
                              "observed" if phase == "capture" else
                              "applied_verified" if phase == "release" else "applied_unverified")
            if phase == self.lifecycle_fault_phase:
                native.update(copy.deepcopy(self.lifecycle_fault_fields))
                receipt.update(copy.deepcopy(self.lifecycle_fault_outer))
                self.lifecycle_fault_call_count = len(self.calls)
        return response


class LifecycleStateClient(LifecycleReplyMixin, StateClient):
    pass


class SettlingStateClient(StateClient):
    def __init__(self, mode="stable"):
        parameter = number("cab.micmixA", 0.75, lower=0.0, upper=1.0)
        parameter["normalized"] = 0.75
        parameter["support"]["normalized"] = True
        parameter["access_metadata_complete"] = True
        super().__init__([parameter])
        self.settlement_mode = mode
        self.awaiting_settlement = False
        self.settled_reads = 0

    def call(self, name, args):
        if name == "plugin_parameter_set":
            receipt = super().call(name, args)
            self.awaiting_settlement = True
            self.settled_reads = 0
            for parameter in (self.parameters[0], receipt["structuredContent"]["result"]["after"]):
                parameter["normalized"] = 0.75 if self.settlement_mode == "normalized_unchanged" else 0.37
                parameter["support"]["normalized"] = True
                parameter["access_metadata_complete"] = True
            return receipt
        if name == "plugin_parameters" and args.get("names") and self.awaiting_settlement:
            self.settled_reads += 1
            bits = "3FD7AE147AE147AF"
            if self.settlement_mode == "baseline":
                bits = "3FE8000000000000"
            elif self.settlement_mode == "unstable" and self.settled_reads == 2:
                bits = "3FD7AE147AE147B0"
            value = struct.unpack(">d", bytes.fromhex(bits))[0]
            self.parameters[0].update(number("cab.micmixA", value, lower=0.0, upper=1.0))
            normalized = 0.37
            if self.settlement_mode == "normalized_mismatch":
                normalized = 0.38
            elif self.settlement_mode == "normalized_unchanged":
                normalized = 0.75
            elif self.settlement_mode == "normalized_unstable" and self.settled_reads == 2:
                normalized = math.nextafter(normalized, math.inf)
            self.parameters[0]["normalized"] = normalized
            self.parameters[0]["support"]["normalized"] = True
            self.parameters[0]["access_metadata_complete"] = True
            if self.settlement_mode == "schema_mismatch":
                self.parameters[0]["tag"] += 1
            if self.settlement_mode == "unsupported_normalized":
                self.parameters[0]["support"]["normalized"] = False
        if name == "plugin_state" and args["action"] == "restore":
            self.awaiting_settlement = False
        return super().call(name, args)


class StateAcceptanceTests(unittest.TestCase):
    def setUp(self):
        PRIVATE_ROOT.mkdir(exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(prefix="state-runner-unit-", dir=PRIVATE_ROOT)
        self.journal = Journal(Path(self.temporary.name) / "run")

    def tearDown(self):
        self.journal.close()
        self.temporary.cleanup()

    def runner(self, client):
        runner = StateRunner(client, self.journal, "MCP Validation Scratch",
                             settle_attempts=4, settle_interval=0)
        runner.initialize()
        return runner

    def events(self):
        return [json.loads(line) for line in self.journal.ledger_path.read_text().splitlines()]

    def reset_journal(self, name):
        self.journal.close()
        self.journal = Journal(Path(self.temporary.name) / name)

    def test_actual_phase_outcomes_keep_capture_observed_and_optional_native_outcome_compatible(self):
        client = LifecycleStateClient()
        runner = self.runner(client)
        runner.run_state_case(TARGET, "gain")
        self.assertEqual(runner.counts["state_cases_verified"], 1)
        captures = [event["receipt"] for event in self.events() if event.get("event") == "rpc_returned"
                    and isinstance(event["receipt"].get("result"), dict) and "serialization" in event["receipt"]["result"]]
        self.assertTrue(captures)
        self.assertTrue(all(receipt["outcome"] == "applied_unverified"
                            and receipt["result"]["outcome"] == "observed"
                            and "native_outcome" not in receipt["result"] for receipt in captures))
        self.assertEqual(Journal.pending(self.journal.ledger_path), {})

    def test_nested_unknown_lifecycle_phases_stop_without_follow_on_requests(self):
        for phase, writes, restores in (("capture", 0, 0), ("restore", 1, 1),
                                        ("release", 0, 0), ("released_restore", 1, 2)):
            self.reset_journal("unknown-" + phase)
            client = LifecycleStateClient()
            client.lifecycle_fault_phase = phase
            runner = self.runner(client)
            with self.assertRaises(UnknownMutation):
                runner.run_state_case(TARGET, "gain")
            self.assertEqual(len(client.writes), writes)
            self.assertEqual(client.restore_count, restores)
            self.assertEqual(len(client.calls), client.lifecycle_fault_call_count)
            self.assertEqual(runner.counts["state_cases_verified"], 0)
            pending = Journal.pending(self.journal.ledger_path)
            self.assertTrue(any(key.startswith("state-call-") and value == "UnknownAfterDispatch"
                                for key, value in pending.items()))

    def test_capture_has_a_durable_inflight_marker_even_without_an_enclosing_case(self):
        client = LifecycleStateClient()
        client.lifecycle_fault_phase = "capture"
        runner = self.runner(client)
        baseline = runner.snapshot(TARGET)
        with self.assertRaises(UnknownMutation):
            runner.capture(baseline)
        events = self.events()
        inflight = next(event for event in events if event.get("phase") == "StateLifecycleInFlight")
        dispatch = next(event for event in events if event.get("event") == "rpc_in_flight" and event.get("tool") == "plugin_state")
        self.assertLess(inflight["sequence"], dispatch["sequence"])
        self.assertEqual(Journal.pending(self.journal.ledger_path), {
            inflight["transaction"]: "UnknownAfterDispatch",
            f"rpc:{dispatch['call']}": "UnknownAfterDispatch",
        })
        self.assertEqual(client.writes, [])
        self.assertEqual(client.restore_count, 0)

    def test_malformed_or_unknown_nested_outcomes_never_certify_a_state_case(self):
        for field in ("outcome", "native_outcome"):
            for index, value in enumerate(("unknown", "outcome_unknown", "unexpected", None, True, [], {})):
                self.reset_journal(f"malformed-{field}-{index}")
                client = LifecycleStateClient()
                client.lifecycle_fault_phase = "capture"
                client.lifecycle_fault_fields = {field: value}
                runner = self.runner(client)
                with self.assertRaises(UnknownMutation):
                    runner.run_state_case(TARGET, "gain")
                self.assertEqual([call["action"] for call in client.state_calls], ["capture"])
                self.assertEqual(client.writes, [])
                self.assertTrue(Journal.pending(self.journal.ledger_path))

    def test_phase_and_outer_native_outcome_contradictions_stop_at_the_boundary(self):
        cases = (("capture", {"outcome": "not_applied"}, {}),
                 ("capture", {"outcome": "applied_unverified"}, {}),
                 ("restore", {"outcome": "not_applied"}, {}),
                 ("restore", {"outcome": "observed"}, {}),
                 ("restore", {"native_outcome": "applied_verified"}, {}),
                 ("release", {"outcome": "not_applied"}, {}),
                 ("release", {}, {"outcome": "not_applied"}),
                 ("released_restore", {"native_outcome": "applied_unverified"}, {}),
                 ("released_restore", {}, {"outcome": "applied_unverified"}))
        for index, (phase, native, outer) in enumerate(cases):
            self.reset_journal("contradiction-" + str(index))
            client = LifecycleStateClient()
            client.lifecycle_fault_phase, client.lifecycle_fault_fields, client.lifecycle_fault_outer = phase, native, outer
            runner = self.runner(client)
            with self.assertRaises(UnknownMutation):
                runner.run_state_case(TARGET, "gain")
            self.assertEqual(len(client.calls), client.lifecycle_fault_call_count)
            self.assertEqual(runner.counts["state_cases_verified"], 0)
            self.assertIn("UnknownAfterDispatch", Journal.pending(self.journal.ledger_path).values())

    def test_malformed_outer_outcomes_and_native_result_stop_without_incidental_type_errors(self):
        for index, outer in enumerate(({"outcome": []}, {"outcome": {}}, {"outcome": None}, {"result": []}, {"result": None})):
            self.reset_journal("outer-malformed-" + str(index))
            client = LifecycleStateClient()
            client.lifecycle_fault_phase = "capture"
            client.lifecycle_fault_fields = {}
            client.lifecycle_fault_outer = outer
            runner = self.runner(client)
            with self.assertRaises(UnknownMutation):
                runner.run_state_case(TARGET, "gain")
            self.assertEqual(client.writes, [])
            self.assertTrue(Journal.pending(self.journal.ledger_path))

    def test_frozen_state_restores_changed_scalar_and_coupled_parameter(self):
        client = StateClient([number("gain", 0.25), number("mix", 0.5)])
        client.side_effect = True
        runner = self.runner(client)
        row = runner.run_state_case(TARGET, "gain")
        self.assertEqual([p["value"] for p in client.parameters], [0.25, 0.5])
        self.assertEqual(len(client.writes), 1)
        self.assertEqual(row["provider_indices_compared"], 2)
        self.assertEqual(runner.counts["changed_load_independently_restored"], 1)
        self.assertEqual(runner.counts["state_cases_verified"], 1)
        self.assertFalse(row["opaque_contents_verified"])
        self.assertEqual(Journal.pending(self.journal.ledger_path), {})

    def test_accepted_load_without_restoration_does_not_pass(self):
        client = StateClient()
        client.ignore_load = True
        runner = self.runner(client)
        with self.assertRaises(RestorationFailure):
            runner.run_state_case(TARGET, "gain")
        self.assertEqual(len(client.writes), 1)
        self.assertEqual(client.restore_count, 1)
        self.assertEqual(runner.counts["state_cases_verified"], 0)
        self.assertIn("RestorationFailed", Journal.pending(self.journal.ledger_path).values())

    def test_false_load_result_stops_without_retry(self):
        client = StateClient()
        client.false_load = True
        runner = self.runner(client)
        with self.assertRaises(RestorationFailure):
            runner.run_state_case(TARGET, "gain")
        self.assertEqual(client.restore_count, 1)

    def test_restoring_only_selected_value_does_not_hide_schema_drift(self):
        client = StateClient()
        client.schema_drift = True
        runner = self.runner(client)
        with self.assertRaises(RestorationFailure):
            runner.run_state_case(TARGET, "gain")
        self.assertEqual(runner.counts["state_cases_verified"], 0)

    def test_readonly_meter_drift_is_not_silently_excluded(self):
        client = StateClient([number("gain", 0.25), number("meter", 0.0, readonly=True)])
        client.readonly_drift = True
        runner = self.runner(client)
        with self.assertRaises(RestorationFailure):
            runner.run_state_case(TARGET, "gain")
        self.assertEqual(runner.counts["state_cases_verified"], 0)
        changed = [event["changed_parameters"] for event in self.events()
                   if event["event"] == "state_baseline_observation" and not event["matches"]]
        self.assertTrue(changed and all("meter" in names for names in changed))

    def test_explicit_native_readonly_observations_do_not_claim_value_restoration(self):
        meter = {**number("meter", 0.0, readonly=True), "access_metadata_complete": True}
        client = StateClient([number("gain", 0.25), meter])
        client.readonly_drift = True
        runner = StateRunner(client, self.journal, "MCP Validation Scratch",
                             settle_attempts=4, settle_interval=0, read_only_observations=True)
        runner.initialize()
        row = runner.run_state_case(TARGET, "gain")
        self.assertEqual(row["read_only_values"], "observed_not_restored")
        self.assertEqual(runner.counts["state_cases_verified"], 1)
        observations = [e["read_only_changed"] for e in self.events()
                        if e["event"] == "state_baseline_observation"]
        self.assertTrue(any("meter" in names for names in observations))
        self.assertFalse(row["opaque_contents_verified"])

    def test_readonly_observations_require_complete_authoritative_access_flags(self):
        client = StateClient([number("gain", 0.25), number("meter", 0.0, readonly=True)])
        client.readonly_drift = True
        runner = StateRunner(client, self.journal, "MCP Validation Scratch",
                             settle_attempts=4, settle_interval=0, read_only_observations=True)
        runner.initialize()
        with self.assertRaises(RestorationFailure):
            runner.run_state_case(TARGET, "gain")
        self.assertEqual(runner.counts["state_cases_verified"], 0)

    def test_ignored_change_is_not_a_pass_even_after_successful_load(self):
        client = StateClient()
        client.ignore_change = True
        runner = self.runner(client)
        with self.assertRaises(AcceptanceFailure):
            runner.run_state_case(TARGET, "gain")
        self.assertEqual(client.parameters[0]["value"], 0.25)
        self.assertEqual(client.restore_count, 1)
        self.assertEqual(runner.counts["changed_load_independently_restored"], 0)

    def test_setter_self_report_cannot_replace_independent_changed_read(self):
        client = StateClient()
        client.ignore_change = client.lie_about_change = True
        runner = self.runner(client)
        with self.assertRaises(AcceptanceFailure):
            runner.run_state_case(TARGET, "gain")
        self.assertEqual(runner.counts["state_cases_verified"], 0)

    def test_synthetic_float_quantization_requires_two_stable_fresh_reads_and_exact_restore(self):
        client = SettlingStateClient()
        runner = self.runner(client)
        row = runner.run_state_case(TARGET, "cab.micmixA", desired_float_bits="3FD7AE147AE147AE")
        self.assertEqual(client.settled_reads, 2)
        self.assertTrue(row["quantized_settlement"])
        self.assertEqual(row["quantized_repeat_count"], 1)
        self.assertEqual(runner.counts["quantized_settlements"], 1)
        self.assertEqual(value_identity(client.parameters[0])[2], "3FE8000000000000")
        changed = [event for event in self.events() if event.get("phase") == "ChangedReadbackConfirmed"]
        self.assertTrue(changed[0]["settlement"]["quantized_settlement"])
        self.assertEqual(changed[0]["observed"]["normalized"], 0.37)
        self.assertEqual(changed[0]["observed"]["native_value"]["data"]["bits_hex"], "3FD7AE147AE147AF")

    def test_float_settlement_with_different_normalized_value_is_refused(self):
        client = SettlingStateClient("normalized_mismatch")
        runner = self.runner(client)
        with self.assertRaises(AcceptanceFailure):
            runner.run_state_case(TARGET, "cab.micmixA", desired_float_bits="3FD7AE147AE147AE")
        self.assertEqual(client.settled_reads, 1)
        self.assertEqual(client.restore_count, 1)
        self.assertEqual(runner.counts["state_cases_verified"], 0)
        self.assertEqual(runner.counts["quantized_settlements"], 0)
        self.assertEqual(value_identity(client.parameters[0])[2], "3FE8000000000000")

    def test_unstable_two_float_readbacks_are_refused_even_with_the_same_normalized_value(self):
        client = SettlingStateClient("unstable")
        runner = self.runner(client)
        with self.assertRaises(AcceptanceFailure):
            runner.run_state_case(TARGET, "cab.micmixA", desired_float_bits="3FD7AE147AE147AE")
        self.assertEqual(client.settled_reads, 2)
        self.assertEqual(client.restore_count, 1)
        self.assertEqual(runner.counts["state_cases_verified"], 0)
        self.assertEqual(value_identity(client.parameters[0])[2], "3FE8000000000000")

    def test_exact_raw_stability_does_not_allow_normalized_readback_drift(self):
        client = SettlingStateClient("normalized_unstable")
        runner = self.runner(client)
        with self.assertRaises(AcceptanceFailure):
            runner.run_state_case(TARGET, "cab.micmixA", desired_float_bits="3FD7AE147AE147AE")
        self.assertEqual(client.settled_reads, 2)
        self.assertEqual(client.restore_count, 1)
        self.assertEqual(runner.counts["state_cases_verified"], 0)

    def test_raw_settlement_drift_with_unchanged_semantic_normalization_is_not_a_change(self):
        client = SettlingStateClient("normalized_unchanged")
        runner = self.runner(client)
        with self.assertRaises(AcceptanceFailure):
            runner.run_state_case(TARGET, "cab.micmixA", desired_float_bits="3FD7AE147AE147AE")
        self.assertEqual(runner.counts["changed_load_independently_restored"], 0)
        self.assertEqual(client.restore_count, 1)
        self.assertEqual(value_identity(client.parameters[0])[2], "3FE8000000000000")

    def test_normalized_settlement_does_not_turn_an_unchanged_baseline_into_a_pass(self):
        client = SettlingStateClient("baseline")
        runner = self.runner(client)
        with self.assertRaises(AcceptanceFailure):
            runner.run_state_case(TARGET, "cab.micmixA", desired_float_bits="3FD7AE147AE147AE")
        self.assertEqual(runner.counts["changed_load_independently_restored"], 0)
        self.assertEqual(client.restore_count, 1)

    def test_settlement_schema_change_preserves_the_context_failure_guard(self):
        client = SettlingStateClient("schema_mismatch")
        runner = self.runner(client)
        with self.assertRaises(ContextChanged):
            runner.run_state_case(TARGET, "cab.micmixA", desired_float_bits="3FD7AE147AE147AE")
        self.assertEqual(client.restore_count, 0)
        self.assertEqual(runner.counts["state_cases_verified"], 0)

    def test_unknown_scalar_change_never_triggers_blind_load(self):
        client = StateClient()
        client.unknown_change = True
        runner = self.runner(client)
        with self.assertRaises(UnknownMutation):
            runner.run_state_case(TARGET, "gain")
        self.assertEqual(client.restore_count, 0)
        self.assertIn("UnknownAfterDispatch", Journal.pending(self.journal.ledger_path).values())

    def test_document_change_after_scalar_write_never_loads_into_new_document(self):
        client = StateClient()
        client.change_document = True
        runner = self.runner(client)
        with self.assertRaises(ContextChanged):
            runner.run_state_case(TARGET, "gain")
        self.assertEqual(client.restore_count, 0)

    def test_unknown_load_is_not_retried(self):
        client = StateClient()
        client.unknown_load = True
        runner = self.runner(client)
        with self.assertRaises(UnknownMutation):
            runner.run_state_case(TARGET, "gain")
        self.assertEqual(client.restore_count, 1)

    def test_document_change_during_load_does_not_publish_restoration_pass(self):
        client = StateClient()
        client.document_changed_by_load = True
        runner = self.runner(client)
        with self.assertRaises(ContextChanged):
            runner.run_state_case(TARGET, "gain")
        self.assertEqual(runner.counts["state_cases_verified"], 0)

    def test_fixture_mute_loss_blocks_state_load(self):
        client = StateClient()
        client.mute_after_write = True
        runner = self.runner(client)
        with self.assertRaises(RestorationFailure):
            runner.run_state_case(TARGET, "gain")
        self.assertEqual(client.restore_count, 0)

    def test_delayed_native_settlement_needs_two_fresh_matching_snapshots(self):
        client = StateClient()
        client.delayed_restore = True
        runner = self.runner(client)
        runner.run_state_case(TARGET, "gain")
        observations = [e for e in self.events() if e["event"] == "state_baseline_observation"]
        self.assertTrue(any(not e["matches"] for e in observations))
        self.assertTrue(any(e["consecutive_matches"] == 2 for e in observations))
        self.assertEqual(runner.counts["state_cases_verified"], 1)

    def test_nondeterministic_capture_digests_are_diagnostics_only(self):
        client = StateClient()
        client.volatile_capture = True
        runner = self.runner(client)
        row = runner.run_state_case(TARGET, "gain")
        self.assertFalse(row["capture_digest_stable"])
        self.assertFalse(row["restored_digest_stable"])
        self.assertEqual(runner.counts["state_cases_verified"], 1)

    def test_same_frozen_token_is_reused_for_each_explicit_repeat(self):
        client = StateClient()
        runner = self.runner(client)
        row = runner.run_state_case(TARGET, "gain", repeats=3)
        restores = [call["token"] for call in client.state_calls if call["action"] == "restore"]
        self.assertEqual(restores, [restores[0]] * 4)
        self.assertEqual(len(client.writes), 3)
        self.assertEqual(row["repeats"], 3)
        self.assertEqual(runner.counts["changed_load_independently_restored"], 3)

    def test_released_token_must_be_refused_before_dispatch(self):
        client = StateClient()
        client.accept_released = True
        runner = self.runner(client)
        with self.assertRaises(AcceptanceFailure):
            runner.run_state_case(TARGET, "gain")
        self.assertEqual(runner.counts["state_cases_verified"], 0)

    def test_wrong_target_checks_both_providers_and_returns_to_original(self):
        client = StateClient()
        runner = self.runner(client)
        row = runner.run_state_case(TARGET, "gain", wrong_target=OTHER)
        wrong = [call for call in client.state_calls if call["action"] == "restore" and call["channel"] == OTHER["channel"]]
        self.assertEqual(len(wrong), 1)
        self.assertEqual(wrong[0]["expected_instance"], OTHER_INSTANCE["object_uid"])
        self.assertEqual(client.targets[OTHER["channel"]][0]["value"], 0.75)
        self.assertEqual(client.targets[TARGET["channel"]][0]["value"], 0.25)
        self.assertTrue(row["wrong_target_token_refused"])

    def test_wrong_target_false_acceptance_stops_before_scalar_mutation(self):
        client = StateClient()
        client.accept_wrong_target = True
        runner = self.runner(client)
        with self.assertRaises(AcceptanceFailure):
            runner.run_state_case(TARGET, "gain", wrong_target=OTHER)
        self.assertEqual(client.writes, [])

    def test_models_require_explicit_selector_and_desired_value(self):
        target = {**TARGET, "plugin": "Fat Channel"}
        client = StateClient([number("opt.eqmodel", 0.0, kind="List", lower=0.0, upper=2.0)])
        runner = self.runner(client)
        for options in ({}, {"allow_model": True}):
            with self.subTest(options=options), self.assertRaises(AcceptanceFailure):
                runner.run_state_case(target, "opt.eqmodel", **options)
        self.assertEqual(client.writes, [])
        self.assertEqual(client.capture_count, 0)

    def test_explicit_model_selection_is_changed_and_full_state_restored(self):
        target = {**TARGET, "plugin": "Fat Channel"}
        client = StateClient([number("opt.eqmodel", 0.0, kind="List", lower=0.0, upper=2.0)])
        runner = self.runner(client)
        row = runner.run_state_case(target, "opt.eqmodel", allow_model=True,
                                    desired_float_bits="3FF0000000000000")
        self.assertEqual(client.writes[0]["float_bits"], "3FF0000000000000")
        self.assertEqual(client.parameters[0]["value"], 0.0)
        self.assertEqual(row["parameter"], "opt.eqmodel")

    def test_host_bypass_is_not_used_as_a_plugin_serialization_test(self):
        client = StateClient([number("bypass", 0, kind="Toggle", lower=0, upper=1)])
        runner = self.runner(client)
        with self.assertRaises(AcceptanceFailure):
            runner.run_state_case(TARGET, "bypass")
        self.assertEqual(client.capture_count, 0)

    def test_exact_native_float_bits_survive_observed_json_rounding(self):
        parameter = number("gain", math.nextafter(0.1, math.inf), lower=0.0, upper=1.0)
        parameter["native_value"]["data"]["bits_hex"] = "3FB999999999999A"
        client = StateClient([parameter])
        runner = self.runner(client)
        runner.run_state_case(TARGET, "gain")
        self.assertEqual(value_identity(client.parameters[0])[2], "3FB999999999999A")
        self.assertTrue(all("float_bits" in write and "value" not in write for write in client.writes))

    def test_negative_zero_survives_capsule_restore(self):
        client = StateClient([number("gain", -0.0)])
        runner = self.runner(client)
        runner.run_state_case(TARGET, "gain")
        self.assertEqual(value_identity(client.parameters[0])[2], "8000000000000000")

    def test_malformed_native_state_summary_fails_before_scalar_write(self):
        client = StateClient()
        client.bad_capture = True
        runner = self.runner(client)
        with self.assertRaises(AcceptanceFailure):
            runner.run_state_case(TARGET, "gain")
        self.assertEqual(client.writes, [])

    def test_explicit_candidates_reject_nonfinite_or_unchanged_values(self):
        parameter = number("gain", 0.25)
        for bits in ("7FF0000000000000", "7FF8000000000000", "3FD0000000000000"):
            with self.subTest(bits=bits), self.assertRaises(AcceptanceFailure):
                explicit_candidate(parameter, bits)
        with self.assertRaises(AcceptanceFailure):
            explicit_candidate(parameter, desired_value=True)

    def test_unicode_literal_notes_are_restored_by_enclosing_state(self):
        parameter = {"name": "deviceNotes", "tag": 1, "native_kind": "String", "value": "원래 메모",
                     "text": "원래 메모", "min": None, "max": None, "enabled": True, "readOnly": False,
                     "writable": True, "support": {"text": True}, "display_parser": "LiteralString",
                     "native_value": {"variant_tag": 3, "data": {"kind": "text", "value": "원래 메모"}}}
        client = StateClient([parameter])
        runner = self.runner(client)
        runner.run_state_case(TARGET, "deviceNotes")
        self.assertEqual(client.parameters[0]["value"], "원래 메모")
        self.assertIn("한글", client.writes[0]["text"])


if __name__ == "__main__":
    unittest.main()
