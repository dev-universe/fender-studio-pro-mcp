"""Offline finite model-choice acceptance checks. No DAW is launched."""
import copy
from contextlib import redirect_stderr, redirect_stdout
import io
import json
import math
from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import patch

from test_verify_all_parameter_states import DigestSweepClient
from test_verify_all_parameters import TARGET, number
from test_verify_plugin_states import LifecycleReplyMixin
from verify_all_parameters import (
    AcceptanceFailure, ContextChanged, Journal, PRIVATE_ROOT, RestorationFailure,
    UnknownMutation, value_identity,
)
from verify_plugin_models import ModelChoice, ModelRunner, exact_integer, main, selector_range


MODEL = "opt.ampmodel"
MODEL_TARGET = {**TARGET, "plugin": "Ampire"}


def typed(number):
    if type(number) is int:
        return {"variant_tag": 1, "data": {"kind": "integer", "value": number}}
    return {"variant_tag": 2, "data": {"kind": "float", "value": number,
            "bits_hex": struct.pack(">d", number).hex().upper()}}


class ModelClient(DigestSweepClient):
    def __init__(self, lower=0.0, upper=2.0, current=0.0, kind="List"):
        super().__init__([number(MODEL, current, kind=kind, lower=lower, upper=upper),
                          number("meter", 0.1, readonly=True), number("gain", 0.25)])
        self.format_calls = []
        self.format_fault = None
        self.read_fault = None
        self.active_choice = False
        self.model_reads = 0
        self.model_writes = 0
        self.native_unknown = False
        self.restore_default_drift = False
        self.restore_projection_drift = None
        self.missing_set_result = False
        self.closed = False

    def call(self, name, args):
        if name == "plugin_parameter_format":
            self.select_target(args)
            self.calls.append((name, copy.deepcopy(args)))
            self.format_calls.append(copy.deepcopy(args))
            assert args["source"] == "dsp" and args["expected_instance"] == self.instance["object_uid"]
            assert args["expected_document"] == "test:1" and args["param"] == MODEL
            assert 1 <= len(args["values"]) <= 32
            rows = [{"requested_value": value, "requested_native_value": typed(float(value)),
                     "native_value": typed(float(value)), "text": "Model " + str(value)} for value in args["values"]]
            body = {"ok": True, "source": "dsp", "instance": dict(self.instance),
                    "param": MODEL, "current_unchanged": True, "values": rows}
            if self.format_fault == "wrong_order":
                body["values"] = list(reversed(rows))
            elif self.format_fault == "incomplete":
                body["values"] = rows[:-1]
            elif self.format_fault == "native_coercion":
                rows[0]["native_value"] = typed(0.5)
            elif self.format_fault == "instance":
                body["instance"]["object_uid"] = "another Device"
            elif self.format_fault == "source":
                body["source"] = "controller"
            elif self.format_fault == "changed_claim":
                body["current_unchanged"] = False
            elif self.format_fault == "malformed_label":
                rows[0]["text"] = None
            elif self.format_fault == "blank_duplicate_labels":
                for row in rows:
                    row["text"] = ""
            elif self.format_fault == "changes_other_value":
                self.parameters[2].update(number("gain", 0.9))
            return self.receipt(body)
        if name == "plugin_parameter_set":
            self.model_writes += 1
            self.active_choice = True
            self.model_reads = 0
            result = super().call(name, args)
            body = result["structuredContent"]["result"]
            if self.read_fault in ("wrong_requested", "fractional"):
                value = 2.0 if self.read_fault == "wrong_requested" else 0.5
                self.parameters[0].update(number(MODEL, value, kind="List", lower=0.0, upper=2.0))
                body["after"] = self.description(self.parameters[0])
            if self.native_unknown:
                body["native_outcome"] = "unknown"
            if self.missing_set_result:
                result["structuredContent"]["result"] = None
            return result
        if name == "plugin_parameters" and args.get("names") and self.active_choice:
            self.model_reads += 1
            if self.read_fault == "read_error":
                raise RuntimeError("synthetic named read failed")
            if self.read_fault == "context":
                self.document = "test:2"
            elif self.read_fault == "unstable" and self.model_reads == 2:
                self.parameters[0].update(number(MODEL, 2.0, kind="List", lower=0.0, upper=2.0))
            elif self.read_fault == "typed_metadata":
                self.parameters[0]["default"] = 0.75
        if name == "plugin_state" and args["action"] == "restore":
            self.active_choice = False
        result = super().call(name, args)
        if name == "plugin_parameters" and args.get("names") and self.active_choice and self.read_fault == "missing_metadata":
            result["structuredContent"]["result"]["params"][0].pop("native_default")
        if name == "plugin_state" and args["action"] == "restore" and self.restore_default_drift:
            self.parameters[2]["default"] = 0.5
        if name == "plugin_state" and args["action"] == "restore" and self.restore_projection_drift:
            self.parameters[2][self.restore_projection_drift] = 0.75 if self.restore_projection_drift == "normalized" else "different display"
        return result

    def close(self):
        self.closed = True


class LifecycleModelClient(LifecycleReplyMixin, ModelClient):
    pass


class ModelAcceptanceTests(unittest.TestCase):
    def setUp(self):
        PRIVATE_ROOT.mkdir(exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(prefix="model-runner-unit-", dir=PRIVATE_ROOT)
        self.journal = Journal(Path(self.temporary.name) / "run")
        self.no_sleep = patch("verify_plugin_states.time.sleep")
        self.no_sleep.start()

    def tearDown(self):
        self.no_sleep.stop()
        self.journal.close()
        self.temporary.cleanup()

    def reset_journal(self, name):
        self.journal.close()
        self.journal = Journal(Path(self.temporary.name) / name)

    def runner(self, client, readonly=False):
        runner = ModelRunner(client, self.journal, "MCP Validation Scratch",
                             settle_attempts=2, settle_interval=0, read_only_observations=readonly)
        runner.initialize()
        return runner

    def events(self):
        return [json.loads(line) for line in self.journal.ledger_path.read_text().splitlines()]

    def test_each_nonbaseline_choice_uses_one_frozen_token_and_exact_requested_reads(self):
        client = ModelClient(current=1.0)
        client.couplings[MODEL] = "gain"
        original = copy.deepcopy(client.parameters)
        runner = self.runner(client)
        row = runner.run_model_case(MODEL_TARGET, MODEL)
        self.assertEqual(runner.finish_models(), 0)
        self.assertEqual(row["status"], "model_case_verified")
        self.assertEqual(row["observed_choices"], 3)
        self.assertEqual(row["verified_nonbaseline_choices"], 2)
        self.assertEqual(client.capture_count, 1)
        self.assertEqual([struct.unpack(">d", bytes.fromhex(write["float_bits"]))[0] for write in client.writes], [0.0, 2.0])
        tokens = {call["token"] for call in client.state_calls if "token" in call}
        self.assertEqual(len(tokens), 1)
        self.assertTrue(row["state_token_released"] and row["released_token_refused"])
        self.assertEqual([value_identity(item) for item in client.parameters], [value_identity(item) for item in original])
        confirmations = [event for event in self.events() if event.get("phase") == "ChangedReadbackConfirmed"]
        self.assertEqual([event["fresh_reads"] for event in confirmations], [2, 2])
        self.assertTrue(all(event["exact_requested_value"] for event in confirmations))
        self.assertEqual(runner.counts["baseline_model_choices_observed"], 1)
        self.assertEqual(Journal.pending(self.journal.ledger_path), {})
        self.assertFalse(runner.summary["all_writable_parameters_verified"])
        self.assertFalse(runner.summary["complete_model_domain_verified"])

    def test_plan_and_full_typed_baseline_are_frozen_once(self):
        client = ModelClient()
        runner = self.runner(client)
        row = runner.run_model_case(MODEL_TARGET, MODEL)
        files = list(self.journal.baselines.glob("*.json"))
        self.assertEqual(len(files), 2)
        plan = json.loads((self.journal.directory / row["choice_plan"]["file"]).read_text())
        self.assertEqual(plan["count_from_typed_range"], 3)
        self.assertEqual([item["baseline_observation_only"] for item in plan["choices"]], [True, False, False])
        baseline = json.loads((self.journal.directory / row["baseline"]["file"]).read_text())
        self.assertIn("native_default", baseline["records"][0])
        self.assertEqual(client.capture_count, 1)

    def test_formatter_chunks_are_bounded_and_count_comes_from_the_typed_range(self):
        client = ModelClient(upper=255.0)
        client.format_fault = "blank_duplicate_labels"
        runner = self.runner(client)
        baseline = runner.snapshot(MODEL_TARGET)
        plan = runner.build_plan(baseline, baseline.records[0])
        self.assertEqual(len(plan.choices), 256)
        self.assertEqual([len(call["values"]) for call in client.format_calls], [32] * 8)
        self.assertTrue(all(choice.label == "" for choice in plan.choices))
        self.assertEqual(client.writes, [])
        self.assertEqual(client.capture_count, 0)

    def test_a_placeholder_single_value_facade_is_observed_without_model_domain_acceptance(self):
        client = ModelClient(upper=0.0)
        client.format_fault = "blank_duplicate_labels"
        runner = self.runner(client)
        row = runner.run_model_case(MODEL_TARGET, MODEL)
        self.assertEqual(row["status"], "model_case_observed_facade_only")
        self.assertEqual(runner.finish_models(), 2)
        self.assertEqual(client.capture_count, 0)
        self.assertEqual(client.writes, [])
        self.assertFalse(row["complete_model_domain_verified"])

    def test_the_baseline_selected_value_cannot_be_dispatched_as_a_change(self):
        client = ModelClient()
        runner = self.runner(client)
        baseline = runner.snapshot(MODEL_TARGET)
        with self.assertRaises(AcceptanceFailure):
            runner.change_model_choice(baseline, baseline.records[0], ModelChoice(0, "Observed"), None, "baseline-only")
        self.assertEqual(client.writes, [])

    def test_native_integer_model_carriers_restore_without_float_conversion(self):
        client = ModelClient(lower=0, upper=2, current=1, kind="Integer")
        runner = self.runner(client)
        runner.run_model_case(MODEL_TARGET, MODEL)
        self.assertEqual([write["value"] for write in client.writes], [0, 2])
        self.assertTrue(all("float_bits" not in write for write in client.writes))
        self.assertEqual(runner.finish_models(), 0)

    def test_nonmodel_or_nonintegral_or_oversized_descriptors_never_capture_or_write(self):
        cases = [("ordinary", ModelClient()), ("float_kind", ModelClient(kind="Float")),
                 ("fractional_min", ModelClient(lower=0.5, current=1.0)),
                 ("fractional_max", ModelClient(upper=2.5)), ("outside", ModelClient(current=3.0)),
                 ("too_many", ModelClient(upper=256.0)), ("fractional_current", ModelClient(current=0.5)),
                 ("disabled", ModelClient()), ("readonly", ModelClient())]
        for name, client in cases:
            self.reset_journal(name)
            if name == "disabled":
                client.parameters[0]["enabled"] = False
            elif name == "readonly":
                client.parameters[0].update(number(MODEL, 0.0, kind="List", readonly=True, upper=2.0))
            runner = self.runner(client)
            target = {**MODEL_TARGET, "plugin": "Unclassified"} if name == "ordinary" else MODEL_TARGET
            with self.assertRaises(AcceptanceFailure):
                runner.run_model_case(target, MODEL)
            self.assertEqual(client.capture_count, 0)
            self.assertEqual(client.writes, [])

    def test_model_bounds_use_exact_native_bits_not_rounded_json_shadows(self):
        client = ModelClient(upper=2.0)
        parameter = client.indexed(client.parameters[0], 0)
        rounded = math.nextafter(2.0, -math.inf)
        parameter["native_max"] = typed(rounded)
        with self.assertRaises(AcceptanceFailure):
            selector_range("Ampire", parameter)
        for native in (typed(True), {"variant_tag": 1, "data": {"kind": "integer", "value": 2.5}},
                       {"variant_tag": 2, "data": {"kind": "integer", "value": 1}}, typed(float(1 << 54))):
            with self.assertRaises(AcceptanceFailure):
                exact_integer(native)

    def test_formatting_errors_stop_before_capture_and_never_select_a_model(self):
        for fault in ("wrong_order", "incomplete", "native_coercion", "instance", "source",
                      "changed_claim", "malformed_label", "changes_other_value"):
            self.reset_journal("format-" + fault)
            client = ModelClient()
            client.format_fault = fault
            runner = self.runner(client)
            with self.assertRaises(AcceptanceFailure):
                runner.run_model_case(MODEL_TARGET, MODEL)
            self.assertEqual(client.capture_count, 0)
            self.assertEqual(client.writes, [])

    def test_known_readback_failure_restores_once_then_stops_without_trying_another_choice(self):
        for fault in ("wrong_requested", "fractional", "read_error", "unstable"):
            self.reset_journal("known-" + fault)
            client = ModelClient()
            client.read_fault = fault
            runner = self.runner(client)
            with self.assertRaises(AcceptanceFailure):
                runner.run_model_case(MODEL_TARGET, MODEL)
            self.assertEqual(len(client.writes), 1)
            self.assertEqual(client.restore_count, 1)
            self.assertEqual(exact_integer(client.parameters[0]["native_value"]), 0)
            self.assertEqual(runner.counts["model_choices_changed_restored_verified"], 0)

    def test_unknown_dispatch_or_context_change_never_attempts_restore(self):
        for behavior in ("unknown", "public_unknown", "document_changed", "native_unknown", "read_context"):
            self.reset_journal(behavior)
            client = ModelClient()
            if behavior == "native_unknown":
                client.native_unknown = True
            elif behavior == "read_context":
                client.read_fault = "context"
            else:
                client.behaviors[MODEL] = behavior
            runner = self.runner(client)
            with self.assertRaises((UnknownMutation, ContextChanged)):
                runner.run_model_case(MODEL_TARGET, MODEL)
            self.assertEqual(len(client.writes), 1)
            self.assertEqual(client.restore_count, 0)
            self.assertFalse(any(call["action"] == "release" for call in client.state_calls))

    def test_nested_unknown_capture_restore_release_and_refusal_never_verify_or_continue(self):
        for phase, writes, restores, actions in (
                ("capture", 0, 0, ["capture"]),
                ("restore", 1, 1, ["capture", "restore"]),
                ("release", 2, 2, ["capture", "restore", "restore", "release"]),
                ("released_restore", 2, 3, ["capture", "restore", "restore", "release", "restore"])):
            self.reset_journal("state-unknown-" + phase)
            client = LifecycleModelClient()
            client.lifecycle_fault_phase = phase
            runner = self.runner(client)
            with self.assertRaises(UnknownMutation):
                runner.run_model_case(MODEL_TARGET, MODEL)
            self.assertEqual([call["action"] for call in client.state_calls], actions)
            self.assertEqual(len(client.writes), writes)
            self.assertEqual(client.restore_count, restores)
            self.assertEqual(len(client.calls), client.lifecycle_fault_call_count)
            self.assertEqual(runner.counts["model_cases_verified"], 0)
            self.assertEqual(runner.finish_models(), 2)
            self.assertTrue(any(key.startswith("state-call-") and value == "UnknownAfterDispatch"
                                for key, value in Journal.pending(self.journal.ledger_path).items()))
            self.assertFalse(any(row["released_token_refused"] for row in runner.summary["classes"]))

    def test_malformed_or_contradictory_state_lifecycle_evidence_never_reaches_another_model_choice(self):
        cases = (("capture", {"outcome": "not_applied"}, {}),
                 ("capture", {"native_outcome": []}, {}),
                 ("restore", {"outcome": "not_applied"}, {}),
                 ("restore", {"native_outcome": None}, {}),
                 ("release", {"outcome": "observed"}, {}),
                 ("release", {"native_outcome": {}}, {}),
                 ("released_restore", {"native_outcome": "applied_verified"}, {}),
                 ("released_restore", {"outcome": True}, {}),
                 ("released_restore", {}, {"outcome": "applied_unverified"}))
        for index, (phase, native, outer) in enumerate(cases):
            self.reset_journal("state-invalid-" + str(index))
            client = LifecycleModelClient()
            client.lifecycle_fault_phase, client.lifecycle_fault_fields, client.lifecycle_fault_outer = phase, native, outer
            runner = self.runner(client)
            with self.assertRaises(UnknownMutation):
                runner.run_model_case(MODEL_TARGET, MODEL)
            self.assertEqual(len(client.calls), client.lifecycle_fault_call_count)
            self.assertEqual(runner.counts["model_cases_verified"], 0)
            self.assertEqual(runner.finish_models(), 2)
            self.assertIn("UnknownAfterDispatch", Journal.pending(self.journal.ledger_path).values())

    def test_a_refused_choice_never_gets_a_capsule_load(self):
        client = ModelClient()
        client.behaviors[MODEL] = "reject"
        runner = self.runner(client)
        with self.assertRaises(AcceptanceFailure):
            runner.run_model_case(MODEL_TARGET, MODEL)
        self.assertEqual(client.restore_count, 0)
        self.assertEqual(runner.counts["model_choices_changed_restored_verified"], 0)

    def test_native_load_acknowledgment_does_not_certify_restoration(self):
        for fault in ("ignore_load", "false_load", "schema_drift", "restore_default_drift"):
            self.reset_journal("load-" + fault)
            client = ModelClient()
            setattr(client, fault, True)
            runner = self.runner(client)
            with self.assertRaises(RestorationFailure):
                runner.run_model_case(MODEL_TARGET, MODEL)
            self.assertEqual(len(client.writes), 1)
            self.assertEqual(client.restore_count, 1)
            self.assertEqual(runner.counts["model_choices_changed_restored_verified"], 0)

    def test_exact_writable_restoration_rejects_one_ulp_omission(self):
        client = ModelClient()
        client.lossy_overrides["gain"] = number("gain", math.nextafter(0.25, math.inf))
        runner = self.runner(client)
        with self.assertRaises(RestorationFailure):
            runner.run_model_case(MODEL_TARGET, MODEL)
        self.assertEqual(client.restore_count, 1)
        self.assertEqual(len(client.writes), 1)

    def test_same_raw_value_with_changed_normalized_or_display_projection_is_not_restored(self):
        for field in ("normalized", "text"):
            self.reset_journal("projection-" + field)
            client = ModelClient()
            client.restore_projection_drift = field
            runner = self.runner(client)
            with self.assertRaises(RestorationFailure):
                runner.run_model_case(MODEL_TARGET, MODEL)
            self.assertEqual(value_identity(client.parameters[2]), value_identity(number("gain", 0.25)))
            observations = [event for event in self.events() if event.get("event") == "state_baseline_observation"]
            self.assertTrue(observations)
            self.assertTrue(all(not event["changed_parameters"] for event in observations))
            self.assertFalse(observations[-1]["projected_values_equal"])
            self.assertEqual(client.restore_count, 1)
            self.assertEqual(len(client.writes), 1)
            self.assertEqual(runner.counts["model_choices_changed_restored_verified"], 0)

    def test_a_second_full_projection_mismatch_cannot_reuse_the_first_match(self):
        client = ModelClient()
        restored_pages = 0

        def second_page_drift(owner, page):
            nonlocal restored_pages
            if owner.restore_count != 1:
                return
            restored_pages += 1
            if restored_pages == 2:
                record = page["params"][2]
                record["text"] = record["native"]["native_description"]["text"] = "second fresh display drift"

        client.page_hook = second_page_drift
        runner = self.runner(client)
        with self.assertRaises(RestorationFailure):
            runner.run_model_case(MODEL_TARGET, MODEL)
        observations = [event for event in self.events() if event.get("event") == "state_baseline_observation"]
        self.assertEqual([event["consecutive_matches"] for event in observations[-2:]], [1, 0])
        self.assertEqual(client.restore_count, 1)
        self.assertEqual(len(client.writes), 1)
        self.assertEqual(runner.counts["model_choices_changed_restored_verified"], 0)

    def test_known_applied_but_missing_setter_result_restores_once_and_stops(self):
        client = ModelClient()
        client.missing_set_result = True
        runner = self.runner(client)
        with self.assertRaises(AcceptanceFailure):
            runner.run_model_case(MODEL_TARGET, MODEL)
        self.assertEqual(client.restore_count, 1)
        self.assertEqual(len(client.writes), 1)
        self.assertEqual(runner.counts["model_choices_changed_restored_verified"], 0)

    def test_readonly_drift_is_observed_only_with_authoritative_explicit_opt_in(self):
        for allowed in (False, True):
            self.reset_journal("readonly-" + str(allowed))
            client = ModelClient()
            client.readonly_drift = True
            runner = self.runner(client, readonly=allowed)
            if allowed:
                runner.run_model_case(MODEL_TARGET, MODEL)
                self.assertEqual(runner.finish_models(), 0)
            else:
                with self.assertRaises(RestorationFailure):
                    runner.run_model_case(MODEL_TARGET, MODEL)
            self.assertFalse(runner.summary["all_writable_parameters_verified"])

    def test_capture_side_effects_or_malformed_token_never_reach_a_model_setter(self):
        for fault in ("capture_changes_schema", "capture_changes_class", "bad_capture"):
            self.reset_journal("capture-" + fault)
            client = ModelClient()
            setattr(client, fault, True)
            runner = self.runner(client)
            with self.assertRaises(AcceptanceFailure):
                runner.run_model_case(MODEL_TARGET, MODEL)
            self.assertEqual(client.capture_count, 1)
            self.assertEqual(client.writes, [])

    def test_typed_selector_change_during_named_reads_stops_without_blind_restore(self):
        for fault in ("typed_metadata", "missing_metadata"):
            self.reset_journal(fault)
            client = ModelClient()
            client.read_fault = fault
            runner = self.runner(client)
            with self.assertRaises(ContextChanged):
                runner.run_model_case(MODEL_TARGET, MODEL)
            self.assertEqual(client.restore_count, 0)

    def test_released_token_acceptance_prevents_a_verified_case(self):
        client = ModelClient()
        client.accept_released = True
        runner = self.runner(client)
        with self.assertRaises(AcceptanceFailure):
            runner.run_model_case(MODEL_TARGET, MODEL)
        self.assertEqual(runner.counts["model_cases_verified"], 0)
        self.assertEqual(runner.finish_models(), 2)

    def test_unstopped_or_unmuted_fixture_never_formats_or_captures(self):
        client = ModelClient()
        client.muted = False
        runner = self.runner(client)
        with self.assertRaises(AcceptanceFailure):
            runner.run_model_case(MODEL_TARGET, MODEL)
        self.assertEqual(client.format_calls, [])
        self.assertEqual(client.capture_count, 0)

    def test_cli_uses_finite_fixture_scope_and_closes_the_mock_owner(self):
        client = ModelClient()
        root = Path(self.temporary.name)
        fixtures, binary = root / "fixtures.json", root / "binary"
        fixtures.write_text(json.dumps([MODEL_TARGET]))
        binary.write_text("synthetic executable identity")
        args = ["--binary", str(binary), "--fixtures", str(fixtures), "--scratch-title", "MCP Validation Scratch",
                "--plugin", "Ampire", "--parameter", MODEL, "--settle-attempts", "2", "--settle-interval", "0",
                "--output-dir", str(root / "cli")]
        with patch("verify_plugin_models.McpClient", return_value=client), redirect_stdout(io.StringIO()):
            self.assertEqual(main(args), 0)
        self.assertTrue(client.closed)
        with patch("verify_plugin_models.McpClient") as factory, redirect_stderr(io.StringIO()):
            with self.assertRaises(AcceptanceFailure):
                main([*args, "--plugin", "Missing Fixture"])
        factory.assert_not_called()


if __name__ == "__main__":
    unittest.main()
