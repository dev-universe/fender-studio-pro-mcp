"""Offline registered-action acceptance checks. No DAW or native add-on is launched."""
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
    UnknownMutation, value_identity,
)
from verify_plugin_actions import ACTION, SPECS, ActionRunner, main, pulse_state


class ActionClient(DigestSweepClient):
    def __init__(self, plugin="Expander"):
        self.spec = SPECS[plugin]
        self.plugin = plugin
        self.target = {**TARGET, "plugin": plugin}
        super().__init__([
            number(self.spec.parameter, 0.0, kind="Toggle"),
            number(self.spec.effects[0], 0.2), number(self.spec.effects[1], 0.8),
            number("threshold", 0.3), number("meter", 0.1, readonly=True),
        ])
        self.live_class = self.spec.class_uid
        self.actions = []
        self.action_reads = {1: 0, 2: 0}
        self.behavior = {}
        self.receipt_fault = None
        self.after_action_page = None
        self.readonly_drift = False
        self.refused_but_changes = False
        self.refusal_evidence = {}
        self.page_hook = self.observe_page

    def update_value(self, index, value):
        parameter = self.parameters[index]
        parameter.update(number(parameter["name"], value, parameter["native_kind"],
                                readonly=parameter["readOnly"], enabled=parameter["enabled"],
                                lower=parameter["min"], upper=parameter["max"]))

    def observe_page(self, client, page):
        if self.actions:
            action = len(self.actions)
            self.action_reads[action] += 1
            if self.after_action_page:
                self.after_action_page(self, action, self.action_reads[action], page)

    def call(self, name, args):
        if name == "plugin_parameters" and self.actions and self.readonly_drift:
            self.update_value(4, self.parameters[4]["value"] + 0.01)
        if name != "plugin_action":
            return super().call(name, args)
        self.select_target(args)
        self.calls.append((name, copy.deepcopy(args)))
        assert args == {**self.target, "action": ACTION, "expected_instance": self.instance["object_uid"],
                        "expected_document": "test:1"}
        self.actions.append(copy.deepcopy(args))
        step = len(self.actions)
        behavior = self.behavior.get(step)
        if behavior == "refuse":
            if self.refused_but_changes:
                self.update_value(3, 0.9)
            return self.receipt({"native_outcome": "not_applied", "errors": ["synthetic refusal"],
                                 **copy.deepcopy(self.refusal_evidence)},
                                "not_applied", {"code": "native_rejected"})
        if behavior != "ignore":
            low, high = self.parameters[1]["value"], self.parameters[2]["value"]
            self.update_value(1, high)
            self.update_value(2, low)
        if behavior == "third_value":
            self.update_value(3, 0.9)
        elif behavior == "third_normalized":
            self.parameters[3]["normalized"] = 0.9
        elif behavior == "third_display":
            self.parameters[3]["text"] = "other display"
        elif behavior == "default":
            self.parameters[3]["default"] = 0.7
        elif behavior == "support":
            self.parameters[3]["support"]["normalized"] = True
        elif behavior == "context":
            self.document = "test:2"
        elif behavior == "timeout":
            raise TimeoutError("synthetic dispatch interruption")
        state = pulse_state(self.indexed(self.parameters[0], 0))
        native = {"ok": True, "outcome": "applied_unverified", "native_outcome": "applied_unverified",
                  "instance": dict(self.instance), "source": "dsp", "action": ACTION,
                  "param": self.spec.parameter, "effect_parameters": list(self.spec.effects),
                  "before": copy.deepcopy(state), "after": copy.deepcopy(state),
                  "upper_dispatched": True, "reset_dispatched": True, "reset_verified": True,
                  "effects_verified": False, "verified": False}
        outcome = "applied_unverified"
        if self.receipt_fault:
            self.receipt_fault(native)
        if behavior == "public_unknown":
            outcome = "outcome_unknown"
        elif behavior == "verified_outer":
            outcome = "applied_verified"
        return self.receipt(native, outcome)

    def close(self):
        pass


def mutate_page(page, index, **fields):
    record = page["params"][index]
    record.update(copy.deepcopy(fields))
    record["native"]["native_description"].update(copy.deepcopy(fields))


class ActionAcceptanceTests(unittest.TestCase):
    def setUp(self):
        PRIVATE_ROOT.mkdir(exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(prefix="action-runner-unit-", dir=PRIVATE_ROOT)
        self.journal = Journal(Path(self.temporary.name) / "run")

    def tearDown(self):
        self.journal.close()
        self.temporary.cleanup()

    def runner(self, client, read_only_observations=False):
        runner = ActionRunner(client, self.journal, "MCP Validation Scratch",
                              read_only_observations=read_only_observations, settle_interval=0)
        runner.initialize()
        return runner

    def events(self, event=None):
        rows = [json.loads(line) for line in self.journal.ledger_path.read_text().splitlines()]
        return [row for row in rows if event is None or row["event"] == event]

    def test_expander_swap_and_single_inverse_restore_complete_exact_provider(self):
        client = ActionClient()
        baseline = copy.deepcopy(client.parameters)
        runner = self.runner(client)
        row = runner.run_action_case(client.target)
        self.assertEqual(row["status"], "action_changed_readback_restored")
        self.assertTrue(row["complete_provider_projection_restored"])
        self.assertEqual(len(client.actions), 2)
        self.assertEqual(client.action_reads, {1: 2, 2: 2})
        self.assertEqual([value_identity(p) for p in client.parameters], [value_identity(p) for p in baseline])
        self.assertEqual(len(self.events("action_permutation_observation")), 2)
        self.assertEqual(len([e for e in self.events("action_baseline_observation") if e["stage"] == "restored"]), 2)
        self.assertEqual(runner.finish(), 0)
        self.assertFalse(runner.summary["all_writable_parameters_verified"])
        self.assertFalse(row["native_effects_verified"])
        self.assertFalse(row["audible_effects_verified"])
        self.assertFalse(row["opaque_contents_verified"])
        self.assertEqual(Journal.pending(self.journal.ledger_path), {})

    def test_all_three_fixed_classes_use_their_exact_canonical_effects(self):
        for index, plugin in enumerate(SPECS):
            with self.subTest(plugin=plugin):
                self.journal.close()
                self.journal = Journal(Path(self.temporary.name) / str(index))
                client = ActionClient(plugin)
                row = self.runner(client).run_action_case(client.target)
                self.assertEqual(row["status"], "action_changed_readback_restored")
                self.assertEqual(len(client.actions), 2)

    def test_button_reset_without_derived_swap_cannot_enable_inverse(self):
        client = ActionClient()
        client.behavior[1] = "ignore"
        runner = self.runner(client)
        with self.assertRaisesRegex(AcceptanceFailure, "permutation"):
            runner.run_action_case(client.target)
        self.assertEqual(len(client.actions), 1)
        self.assertEqual(runner.counts["inverse_dispatch_attempts"], 0)
        self.assertFalse(runner.summary["classes"][0]["inverse_attempted"])

    def test_unexpected_non_effect_raw_normalized_or_display_change_stops_before_inverse(self):
        for index, behavior in enumerate(("third_value", "third_normalized", "third_display")):
            with self.subTest(behavior=behavior):
                self.journal.close()
                self.journal = Journal(Path(self.temporary.name) / str(index))
                client = ActionClient()
                client.behavior[1] = behavior
                with self.assertRaisesRegex(AcceptanceFailure, "non-effect"):
                    self.runner(client).run_action_case(client.target)
                self.assertEqual(len(client.actions), 1)

    def test_selected_effect_display_and_normalized_values_must_be_stable_on_second_read(self):
        for index, field in enumerate(("text", "normalized")):
            with self.subTest(field=field):
                self.journal.close()
                self.journal = Journal(Path(self.temporary.name) / str(index))
                client = ActionClient()

                def drift(client, action, reading, page):
                    if action == 1 and reading == 2:
                        if field == "text":
                            mutate_page(page, 1, text="late change")
                        else:
                            description = client.description(number("sample", 0.9))
                            mutate_page(page, 1, normalized=description["normalized"],
                                        native_normalized=description["native_normalized"])

                client.after_action_page = drift
                with self.assertRaisesRegex(AcceptanceFailure, "unstable"):
                    self.runner(client).run_action_case(client.target)
                self.assertEqual(len(client.actions), 1)

    def test_second_complete_read_must_still_show_both_swapped_values(self):
        client = ActionClient()

        def revert(client, action, reading, page):
            if action == 1 and reading == 2:
                replacement = client.indexed(number(client.spec.effects[0], 0.2), 1)
                page["params"][1] = replacement

        client.after_action_page = revert
        with self.assertRaises(AcceptanceFailure):
            self.runner(client).run_action_case(client.target)
        self.assertEqual(len(client.actions), 1)

    def test_typed_default_or_support_drift_stops_before_inverse(self):
        for index, behavior in enumerate(("default", "support")):
            self.journal.close()
            self.journal = Journal(Path(self.temporary.name) / str(index))
            client = ActionClient()
            client.behavior[1] = behavior
            with self.assertRaises(ContextChanged):
                self.runner(client).run_action_case(client.target)
            self.assertEqual(len(client.actions), 1)

    def test_readonly_drift_is_refused_by_default(self):
        client = ActionClient()
        client.readonly_drift = True
        with self.assertRaisesRegex(AcceptanceFailure, "non-effect"):
            self.runner(client).run_action_case(client.target)
        self.assertEqual(len(client.actions), 1)

    def test_explicit_authoritative_readonly_observations_allow_drift_without_restore_claim(self):
        client = ActionClient()
        client.readonly_drift = True
        runner = self.runner(client, read_only_observations=True)
        row = runner.run_action_case(client.target)
        self.assertEqual(row["status"], "action_changed_readback_restored")
        self.assertEqual(row["read_only_values"], "observed_not_restored")
        self.assertEqual(len(client.actions), 2)
        self.assertGreater(client.parameters[4]["value"], 0.1)

    def test_missing_readonly_access_proof_cannot_be_excluded(self):
        client = ActionClient()

        def missing_proof(client, action, reading, page):
            if action == 1:
                page["params"][4].pop("access_metadata_complete")

        client.after_action_page = missing_proof
        with self.assertRaisesRegex(AcceptanceFailure, "access proof"):
            self.runner(client, read_only_observations=True).run_action_case(client.target)
        self.assertEqual(len(client.actions), 1)

    def test_schema_or_instance_change_on_second_read_cannot_enable_inverse(self):
        for index, fault in enumerate(("schema", "instance", "source", "count")):
            self.journal.close()
            self.journal = Journal(Path(self.temporary.name) / str(index))
            client = ActionClient()

            def change(client, action, reading, page):
                if action == 1 and reading == 2:
                    if fault == "schema":
                        mutate_page(page, 1, default=0.4,
                                    native_default=client.description(number("sample", 0.4))["native_value"])
                    elif fault == "instance":
                        page["instance"]["object_uid"] = "different"
                    elif fault == "source":
                        page["parameter_discovery"]["source"] = "controller"
                    else:
                        page["reported_count"] += 1

            client.after_action_page = change
            with self.assertRaises(AcceptanceFailure):
                self.runner(client).run_action_case(client.target)
            self.assertEqual(len(client.actions), 1)

    def test_unknown_dispatch_and_context_loss_never_dispatch_inverse(self):
        for index, behavior in enumerate(("timeout", "public_unknown", "context")):
            self.journal.close()
            self.journal = Journal(Path(self.temporary.name) / str(index))
            client = ActionClient()
            client.behavior[1] = behavior
            runner = self.runner(client)
            with self.assertRaises((UnknownMutation, ContextChanged)):
                runner.run_action_case(client.target)
            self.assertEqual(len(client.actions), 1)
            self.assertEqual(runner.counts["inverse_dispatch_attempts"], 0)
            self.assertTrue(Journal.pending(self.journal.ledger_path))

    def test_native_unknown_inside_refused_or_applied_receipts_is_not_safe_to_retry(self):
        client = ActionClient()
        client.receipt_fault = lambda result: result.update(native_outcome="unknown")
        with self.assertRaises(UnknownMutation):
            self.runner(client).run_action_case(client.target)
        self.assertEqual(len(client.actions), 1)

    def test_malformed_or_overclaimed_phase_receipt_never_enables_inverse(self):
        faults = {"source": "controller", "action": "other", "param": "other", "effect_parameters": ["x", "y"],
                  "ok": False, "reset_verified": False, "upper_dispatched": False, "reset_dispatched": False,
                  "verified": True, "effects_verified": True, "native_outcome": None, "before": {}, "after": {}}
        for index, (key, value) in enumerate(faults.items()):
            self.journal.close()
            self.journal = Journal(Path(self.temporary.name) / str(index))
            client = ActionClient()
            client.receipt_fault = lambda result, key=key, value=value: result.update({key: value})
            with self.assertRaises(AcceptanceFailure):
                self.runner(client).run_action_case(client.target)
            self.assertEqual(len(client.actions), 1)

    def test_native_receipt_instance_change_is_context_loss(self):
        client = ActionClient()
        client.receipt_fault = lambda result: result["instance"].update(object_uid="other")
        with self.assertRaises(ContextChanged):
            self.runner(client).run_action_case(client.target)
        self.assertEqual(len(client.actions), 1)

    def test_outer_verified_claim_does_not_certify_an_unverified_button_effect(self):
        client = ActionClient()
        client.behavior[1] = "verified_outer"
        with self.assertRaises(AcceptanceFailure):
            self.runner(client).run_action_case(client.target)
        self.assertEqual(len(client.actions), 1)

    def test_known_refusal_requires_two_complete_unchanged_reads(self):
        client = ActionClient()
        client.behavior[1] = "refuse"
        runner = self.runner(client)
        row = runner.run_action_case(client.target)
        self.assertEqual(row["status"], "action_refused_unchanged")
        self.assertFalse(row["inverse_attempted"])
        self.assertEqual(len(client.actions), 1)
        self.assertEqual(client.action_reads[1], 2)
        self.assertEqual(runner.finish(), 2)
        self.assertEqual(Journal.pending(self.journal.ledger_path), {})

    def test_false_refusal_with_changed_state_cannot_complete(self):
        client = ActionClient()
        client.behavior[1] = "refuse"
        client.refused_but_changes = True
        with self.assertRaises(AcceptanceFailure):
            self.runner(client).run_action_case(client.target)
        self.assertEqual(len(client.actions), 1)

    def test_true_or_malformed_first_refusal_phase_keeps_pending_without_inverse(self):
        flags = ("upper_dispatched", "reset_dispatched", "reset_verified", "effects_verified", "verified")
        for index, (field, value) in enumerate((field, value) for field in flags
                                              for value in (True, 0, 1, None, "false", [], {})):
            with self.subTest(field=field, value=value):
                self.journal.close()
                self.journal = Journal(Path(self.temporary.name) / str(index))
                client = ActionClient()
                client.behavior[1] = "refuse"
                client.refusal_evidence = {field: value}
                runner = self.runner(client)
                with self.assertRaises(UnknownMutation):
                    runner.run_action_case(client.target)
                self.assertEqual(len(client.actions), 1)
                self.assertEqual(client.action_reads[1], 0)
                self.assertEqual(runner.counts["inverse_dispatch_attempts"], 0)
                self.assertEqual(runner.summary["classes"][0]["status"], "unknown_or_context_changed")
                self.assertEqual(Journal.pending(self.journal.ledger_path).get("action-0"), "UnknownAfterDispatch")
                self.assertFalse(any(row.get("phase") == "NoWriteConfirmed" for row in self.events()))

    def test_true_or_malformed_inverse_refusal_phase_stops_without_restore_reads_or_retry(self):
        flags = ("upper_dispatched", "reset_dispatched", "reset_verified", "effects_verified", "verified")
        for index, (field, value) in enumerate((field, value) for field in flags for value in (True, 0, None)):
            with self.subTest(field=field, value=value):
                self.journal.close()
                self.journal = Journal(Path(self.temporary.name) / str(index))
                client = ActionClient()
                client.behavior[2] = "refuse"
                client.refusal_evidence = {field: value}
                runner = self.runner(client)
                with self.assertRaises(UnknownMutation):
                    runner.run_action_case(client.target)
                self.assertEqual(len(client.actions), 2)
                self.assertEqual(client.action_reads[2], 0)
                self.assertEqual(runner.counts["inverse_dispatch_attempts"], 1)
                self.assertEqual(len(self.events("action_permutation_observation")), 2)
                self.assertEqual(Journal.pending(self.journal.ledger_path).get("action-0"), "UnknownAfterDispatch")
                self.assertFalse(any(row.get("phase") == "StateConfirmed" for row in self.events()))

    def test_explicit_false_phase_evidence_retains_two_unchanged_refusal_reads(self):
        client = ActionClient()
        client.behavior[1] = "refuse"
        client.refusal_evidence = dict.fromkeys(("upper_dispatched", "reset_dispatched", "reset_verified",
                                               "effects_verified", "verified"), False)
        row = self.runner(client).run_action_case(client.target)
        self.assertEqual(row["status"], "action_refused_unchanged")
        self.assertEqual(len(client.actions), 1)
        self.assertEqual(client.action_reads[1], 2)
        self.assertEqual(Journal.pending(self.journal.ledger_path), {})

    def test_applied_outcome_inside_refused_receipt_remains_unknown(self):
        for index, (field, value) in enumerate((field, value) for field in ("outcome", "native_outcome")
                                              for value in ("applied_verified", "applied_unverified")):
            self.journal.close()
            self.journal = Journal(Path(self.temporary.name) / str(index))
            client = ActionClient()
            client.behavior[1] = "refuse"
            client.refusal_evidence = {field: value}
            with self.assertRaises(UnknownMutation):
                self.runner(client).run_action_case(client.target)
            self.assertEqual(len(client.actions), 1)
            self.assertEqual(client.action_reads[1], 0)
            self.assertTrue(Journal.pending(self.journal.ledger_path))

    def test_inverse_is_never_retried_after_ignored_refused_or_unknown_dispatch(self):
        for index, behavior in enumerate(("ignore", "refuse", "timeout", "public_unknown", "context")):
            self.journal.close()
            self.journal = Journal(Path(self.temporary.name) / str(index))
            client = ActionClient()
            client.behavior[2] = behavior
            with self.assertRaises((RestorationFailure, UnknownMutation, ContextChanged)):
                self.runner(client).run_action_case(client.target)
            self.assertEqual(len(client.actions), 2)

    def test_restoration_requires_second_complete_baseline_observation(self):
        client = ActionClient()

        def late_drift(client, action, reading, page):
            if action == 2 and reading == 2:
                mutate_page(page, 3, text="restoration drift")

        client.after_action_page = late_drift
        with self.assertRaises(RestorationFailure):
            self.runner(client).run_action_case(client.target)
        self.assertEqual(len(client.actions), 2)

    def test_equal_filters_wrong_class_and_unsupported_target_refuse_before_dispatch(self):
        for index, fault in enumerate(("equal", "class", "plugin", "active_button", "readonly_effect")):
            self.journal.close()
            self.journal = Journal(Path(self.temporary.name) / str(index))
            client = ActionClient()
            target = client.target
            if fault == "equal":
                client.update_value(1, client.parameters[2]["value"])
            elif fault == "class":
                client.live_class = "wrong class"
            elif fault == "plugin":
                target = {**target, "plugin": "Other"}
            elif fault == "active_button":
                client.update_value(0, 1.0)
            else:
                client.parameters[1].update(number(client.spec.effects[0], 0.2, readonly=True))
            with self.assertRaises(AcceptanceFailure):
                self.runner(client).run_action_case(target)
            self.assertEqual(client.actions, [])

    def test_malformed_numeric_carrier_idle_bits_and_cross_ranges_refuse_before_dispatch(self):
        for index, fault in enumerate(("carrier", "idle_bits", "cross_range", "button_max")):
            self.journal.close()
            self.journal = Journal(Path(self.temporary.name) / str(index))
            client = ActionClient()
            if fault == "carrier":
                client.parameters[1]["native_value"]["variant_tag"] = 1
            elif fault == "idle_bits":
                client.parameters[0].update(number(client.spec.parameter, -0.0, kind="Toggle"))
            elif fault == "cross_range":
                client.parameters[1]["max"] = 0.5
            else:
                client.parameters[0]["max"] = 2.0
            with self.assertRaises(AcceptanceFailure):
                self.runner(client).run_action_case(client.target)
            self.assertEqual(client.actions, [])

    def test_numeric_boolean_coercion_in_pulse_state_cannot_enable_inverse(self):
        for index, fault in enumerate(("enabled", "readOnly", "native_value", "native_min")):
            self.journal.close()
            self.journal = Journal(Path(self.temporary.name) / str(index))
            client = ActionClient()

            def malformed(result):
                state = result["after"]
                if fault in ("enabled", "readOnly"):
                    state[fault] = int(state[fault])
                else:
                    state[fault]["data"]["value"] = False

            client.receipt_fault = malformed
            with self.assertRaises(AcceptanceFailure):
                self.runner(client).run_action_case(client.target)
            self.assertEqual(len(client.actions), 1)

    def test_unmuted_fixture_before_inverse_cannot_dispatch_it(self):
        client = ActionClient()

        def unmute(client, action, reading, page):
            if action == 1 and reading == 2:
                client.muted = False

        client.after_action_page = unmute
        runner = self.runner(client)
        with self.assertRaises(RestorationFailure):
            runner.run_action_case(client.target)
        self.assertEqual(len(client.actions), 1)
        self.assertEqual(runner.counts["inverse_dispatch_attempts"], 0)
        self.assertFalse(runner.summary["classes"][0]["inverse_attempted"])

    def test_fixture_failure_before_first_dispatch_is_not_an_unknown_mutation(self):
        client = ActionClient()
        original_hook = client.page_hook

        def unmute_before_dispatch(client, page):
            original_hook(client, page)
            if not client.actions and client.pages == 2:
                client.muted = False

        client.page_hook = unmute_before_dispatch
        runner = self.runner(client)
        with self.assertRaises(AcceptanceFailure):
            runner.run_action_case(client.target)
        self.assertEqual(client.actions, [])
        self.assertEqual(runner.summary["classes"][0]["status"], "rejected_before_dispatch")
        self.assertEqual(Journal.pending(self.journal.ledger_path), {})

    def test_duplicate_case_cannot_accidentally_repeat_action(self):
        client = ActionClient()
        runner = self.runner(client)
        runner.run_action_case(client.target)
        with self.assertRaisesRegex(AcceptanceFailure, "at most once"):
            runner.run_action_case(client.target)
        self.assertEqual(len(client.actions), 2)

    def test_finish_with_no_cases_is_not_verified(self):
        runner = self.runner(ActionClient())
        self.assertEqual(runner.finish(), 2)

    def test_full_provider_pagination_is_observed_in_each_phase(self):
        client = ActionClient()
        client.parameters.extend(number("padding" + str(index), 0.1) for index in range(70))
        row = self.runner(client).run_action_case(client.target)
        self.assertEqual(row["provider_indices_compared"], 75)
        self.assertEqual(client.action_reads, {1: 4, 2: 4})

    def test_cli_selects_exact_fixture_and_closes_client(self):
        client = ActionClient()
        fixtures = Path(self.temporary.name) / "fixtures.json"
        fixtures.write_text(json.dumps([{**client.target, "test_parameter": "ignored"}]))
        directory = Path(self.temporary.name) / "cli"
        with patch("verify_plugin_actions.McpClient", return_value=client), redirect_stdout(io.StringIO()):
            result = main(["--binary", __file__, "--fixtures", str(fixtures), "--scratch-title", "MCP Validation Scratch",
                           "--plugin", "Expander", "--run-actions", "--settle-interval", "0", "--output-dir", str(directory)])
        self.assertEqual(result, 0)
        self.assertEqual(len(client.actions), 2)

    def test_cli_invalid_scope_does_not_start_owner(self):
        fixtures = Path(self.temporary.name) / "fixtures.json"
        fixtures.write_text(json.dumps([TARGET]))
        with patch("verify_plugin_actions.McpClient") as client, self.assertRaises(AcceptanceFailure):
            main(["--fixtures", str(fixtures), "--scratch-title", "MCP Validation Scratch",
                  "--plugin", "Expander", "--run-actions"])
        client.assert_not_called()

    def test_cli_unknown_outcome_records_failure_without_inverse(self):
        client = ActionClient()
        client.behavior[1] = "public_unknown"
        fixtures = Path(self.temporary.name) / "fixtures.json"
        fixtures.write_text(json.dumps([client.target]))
        directory = Path(self.temporary.name) / "cli-failure"
        with patch("verify_plugin_actions.McpClient", return_value=client), redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
            result = main(["--binary", __file__, "--fixtures", str(fixtures), "--scratch-title", "MCP Validation Scratch",
                           "--plugin", "Expander", "--run-actions", "--output-dir", str(directory)])
        self.assertEqual(result, 1)
        self.assertEqual(len(client.actions), 1)
        self.assertTrue(Journal.pending(directory / "ledger.jsonl"))

    def test_cli_contradictory_refusal_stops_before_the_next_plugin(self):
        client = ActionClient()
        client.behavior[1] = "refuse"
        client.refusal_evidence = {"upper_dispatched": True, "reset_dispatched": False}
        fixtures = Path(self.temporary.name) / "fixtures.json"
        fixtures.write_text(json.dumps([client.target, {**client.target, "plugin": "Gate", "slot": 1}]))
        directory = Path(self.temporary.name) / "cli-contradictory-refusal"
        with patch("verify_plugin_actions.McpClient", return_value=client), redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
            result = main(["--binary", __file__, "--fixtures", str(fixtures), "--scratch-title", "MCP Validation Scratch",
                           "--plugin", "Expander", "--plugin", "Gate", "--run-actions", "--output-dir", str(directory)])
        self.assertEqual(result, 1)
        self.assertEqual(len(client.actions), 1)
        self.assertFalse(any(args.get("plugin") == "Gate" for _, args in client.calls))
        self.assertEqual(Journal.pending(directory / "ledger.jsonl").get("action-0"), "UnknownAfterDispatch")


if __name__ == "__main__":
    unittest.main()
