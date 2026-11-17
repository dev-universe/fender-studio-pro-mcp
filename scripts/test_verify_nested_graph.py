#!/usr/bin/env python3
"""Offline finite-graph acceptance checks; no native code or DAW is launched."""
import contextlib
import copy
import io
import json
from pathlib import Path
import tempfile
import unittest

from verify_all_parameters import (AcceptanceFailure, ContextChanged, Journal,
                                   PRIVATE_ROOT, RestorationFailure, UnknownMutation)
from verify_nested_graph import (ALL_NAMES, NestedGraphRunner, active_names,
                                 candidate, same)


CHANNEL = "FSP Verify FX Splitter"
TITLE = "MCP Nested Validation"
LABELS = ["Splitter 1", "Splitter 2"]


class SplitterClient:
    def __init__(self):
        self.document, self.session = "test:1", "test"
        self.calls, self.writes = [], []
        self.truncated, self.diagnostics = False, []
        self.labels = list(LABELS)
        self.values = {}
        for node in ("Node01", "Node02"):
            self.values[node] = {"numOutputs": 2, "splitMode": 0,
                                 **{f"gain[{i}]": 1 for i in range(1, 6)},
                                 **{f"mute[{i}]": False for i in range(1, 6)},
                                 **{f"splitFreq[{i}]": value for i, value in enumerate((250, 1000, 4000, 10000), 1)}}
        self.unknown = self.change_document = self.ignore = self.sibling_drift = False
        self.fail_restore = False

    def parameter(self, node, name):
        value = self.values[node][name]
        lower, upper = ((2, 5) if name == "numOutputs" else (0, 2) if name == "splitMode"
                        else (20, 20000) if name.startswith("splitFreq[") else (0, 1))
        return {"name": name, "value": value, "min": lower, "max": upper,
                "normalized": (value - lower) / (upper - lower), "text": str(int(value)) if isinstance(value, bool) else str(value)}

    def inventory(self):
        return {"channel": CHANNEL, "scope": "top_level", "truncated": self.truncated,
                "diagnostics": copy.deepcopy(self.diagnostics), "splitters": [
                    {"channel": CHANNEL, "node": node, "label": self.labels[index], "type": "Splitter", "missing": [],
                     "params": [self.parameter(node, name) for name in ALL_NAMES
                                if name in active_names(self.values[node]["numOutputs"])]}
                    for index, node in enumerate(self.values)]}

    def receipt(self, result, outcome="observed"):
        return {"structuredContent": {"document": self.document, "session": self.session,
                                      "outcome": outcome, "result": result}}

    def call(self, name, args):
        self.calls.append((name, copy.deepcopy(args)))
        if name == "host_status":
            return self.receipt({"open": True, "title": TITLE})
        if name != "operation_run":
            raise AssertionError(name)
        operation, values = args["operation"], args.get("args", {})
        if operation == "song":
            return self.receipt({"title": TITLE, "transport": {"playing": False, "recording": False}})
        if operation == "tracks":
            return self.receipt([{"name": CHANNEL, "channel": CHANNEL, "mediaType": "Audio", "eventCount": 0, "events": []}])
        if operation == "channels":
            return self.receipt([{"label": CHANNEL, "mute": True, "recordArmed": False, "monitor": False}])
        if operation == "splitters":
            return self.receipt(self.inventory())
        if operation != "setSplitter":
            raise AssertionError(operation)
        self.writes.append(copy.deepcopy(args))
        if args.get("expected_document") != "test:1":
            raise AssertionError("Write did not bind the document")
        node, param, wanted = values["node"], values["param"], values["value"]
        if values["label"] != self.labels[list(self.values).index(node)] or values["channel"] != CHANNEL:
            raise AssertionError("Write did not bind exact node/label/channel")
        if param not in active_names(self.values[node]["numOutputs"]):
            raise AssertionError("Inactive control was dispatched")
        before = self.parameter(node, param)
        if not self.ignore and not (self.fail_restore and param.startswith("gain[") and wanted == 1):
            self.values[node][param] = wanted
        if self.sibling_drift:
            sibling = "Node02" if node == "Node01" else "Node01"
            self.values[sibling]["gain[1]"] = 0.25
        if self.change_document:
            self.document = "test:2"
        if self.unknown:
            raise TimeoutError("Transport interruption after dispatch")
        return self.receipt({"channel": CHANNEL, "node": node, "label": values["label"], "type": "Splitter",
                             "requested": wanted, "before": before, "after": self.parameter(node, param), "verified": True},
                            "applied_verified")


class TransientChangedClient(SplitterClient):
    def __init__(self):
        super().__init__()
        self.pending_revert = None

    def call(self, name, args):
        result = super().call(name, args)
        values = args.get("args", {})
        if name == "operation_run" and args["operation"] == "setSplitter" and values.get("param") == "gain[1]" and values.get("value") == 0.5:
            self.pending_revert = (values["node"], result["structuredContent"]["result"]["before"]["value"])
        return result

    def inventory(self):
        result = super().inventory()
        if self.pending_revert is not None:
            node, original = self.pending_revert
            self.values[node]["gain[1]"] = original
            self.pending_revert = None
        return result


class DelayedRestoreDriftClient(SplitterClient):
    def __init__(self, trigger=("Node01", "gain[1]", 1), drift_on_read=4):
        super().__init__()
        self.trigger, self.drift_on_read = trigger, drift_on_read
        self.restore_reads = None

    def call(self, name, args):
        result = super().call(name, args)
        values = args.get("args", {})
        if name == "operation_run" and args["operation"] == "setSplitter" and (values.get("node"), values.get("param"), values.get("value")) == self.trigger:
            self.restore_reads = 0
        return result

    def inventory(self):
        if self.restore_reads is not None:
            self.restore_reads += 1
            if self.restore_reads == self.drift_on_read:
                self.values[self.trigger[0]]["gain[2]"] = 0.25
        return super().inventory()


class NestedGraphTests(unittest.TestCase):
    def setUp(self):
        PRIVATE_ROOT.mkdir(exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(prefix="nested-runner-unit-", dir=PRIVATE_ROOT)
        self.journal = Journal(Path(self.temporary.name) / "run")

    def tearDown(self):
        self.journal.close()
        self.temporary.cleanup()

    def runner(self, client, prepare=False):
        runner = NestedGraphRunner(client, self.journal, TITLE, CHANNEL, LABELS)
        if prepare:
            runner.initialize()
            runner.guard()
            runner.initial = runner.capture()
            current = runner.initial
            for node in current:
                current = runner.write_checked(node, "numOutputs", 5, current, "test_setup")
            runner.baseline = runner.capture()
            client.writes.clear()
        return runner

    def events(self):
        return [json.loads(line) for line in self.journal.ledger_path.read_text().splitlines()]

    def test_all_42_cases_change_independently_restore_every_sibling_and_restore_initial(self):
        client = SplitterClient()
        initial = copy.deepcopy(client.values)
        runner = self.runner(client)
        with contextlib.redirect_stdout(io.StringIO()):
            summary = runner.run()
        self.assertTrue(summary["passed"])
        self.assertEqual(summary["verified_case_count"], 42)
        self.assertFalse(summary["topology_verified_separately"])
        self.assertFalse(summary["routing_edges_observed"])
        self.assertTrue(summary["original_state_restored"])
        self.assertEqual(summary["initial_restore_fresh_reads"], 2)
        self.assertEqual(client.values, initial)
        self.assertEqual(len(summary["cases"]), 42)
        self.assertTrue(all(row["changed_readback_confirmed"] and row["full_baseline_restored"]
                            and row["all_sibling_descriptors_unchanged"] for row in summary["cases"]))
        self.assertTrue(all(row["changed_readback_fresh_reads"] == 2 and row["restore_baseline_fresh_reads"] == 2
                            for row in summary["cases"]))
        self.assertEqual(len([entry for entry in self.events() if entry["event"] == "parameter_acceptance"]), 42)
        self.assertEqual(Journal.pending(self.journal.ledger_path), {})
        for row in summary["cases"]:
            if row["parameter"] == "numOutputs" and row["requested"] == 5:
                self.assertEqual(row["preparation"], 4)
            if row["parameter"] == "splitMode" and row["requested"] == 0:
                self.assertEqual(row["preparation"], 1)

    def test_one_changed_read_followed_by_reversion_cannot_pass_the_42_case_run(self):
        client = TransientChangedClient()
        runner = self.runner(client)
        with contextlib.redirect_stdout(io.StringIO()), self.assertRaises(AcceptanceFailure):
            runner.run()
        self.assertFalse(runner.summary["passed"])
        self.assertNotEqual(runner.counts["changed_readback_restored"], 42)
        self.assertEqual(runner.rows[-1]["status"], "change_failed_restored")
        self.assertNotIn("changed_readback_confirmed", runner.rows[-1])
        self.assertTrue(runner.summary["original_state_restored"])
        observations = [entry for entry in self.events() if entry["event"] == "splitter_changed_readback_attempt"
                        and entry["purpose"] == "case_change" and entry["parameter"] == "gain[1]"]
        self.assertEqual([entry["attempt"] for entry in observations], [1, 2])

    def test_second_changed_snapshot_must_match_the_entire_target_descriptor(self):
        client = SplitterClient()
        runner = self.runner(client, prepare=True)
        inventory = client.inventory
        reads = 0

        def changing_descriptor():
            nonlocal reads
            result = inventory()
            if client.values["Node01"]["gain[1]"] == 0.5:
                reads += 1
                if reads == 2:
                    target = next(item for item in result["splitters"] if item["node"] == "Node01")
                    next(item for item in target["params"] if item["name"] == "gain[2]")["text"] = "transient descriptor"
            return result

        client.inventory = changing_descriptor
        with contextlib.redirect_stdout(io.StringIO()), self.assertRaises(AcceptanceFailure):
            runner.run_case("Node01", "gain[1]", 0.5)
        self.assertEqual(runner.rows[-1]["status"], "change_failed_restored")
        self.assertIn("snapshots are unstable", runner.rows[-1]["change_error"])
        self.assertEqual(runner.counts["changed_readback_restored"], 0)

    def test_second_restored_baseline_drift_stops_the_full_run_without_next_case(self):
        # Two post-setter reads and the first complete restoration read match;
        # only the second complete restoration read exposes this drift.
        client = DelayedRestoreDriftClient()
        runner = self.runner(client)
        with contextlib.redirect_stdout(io.StringIO()), self.assertRaises(RestorationFailure):
            runner.run()
        self.assertFalse(runner.summary["passed"])
        self.assertFalse(runner.summary["original_state_restored"])
        self.assertEqual(runner.rows[-1]["status"], "restoration_failed")
        self.assertEqual(len(runner.rows), 8)
        self.assertEqual(runner.counts["changed_readback_restored"], 7)
        self.assertNotIn("full_baseline_restored", runner.rows[-1])
        self.assertEqual(client.restore_reads, 4)
        self.assertEqual(client.writes[-1]["args"]["param"], "gain[1]")
        self.assertEqual(client.writes[-1]["args"]["value"], 1)
        phases = [entry["phase"] for entry in self.events() if entry["event"] == "transaction_phase"
                  and entry["transaction"] == runner.rows[-1]["transaction"]]
        self.assertEqual(phases[-1], "RestorationFailed")
        self.assertNotIn("StateConfirmed", phases)

    def test_second_final_initial_baseline_drift_cannot_confirm_original_state(self):
        client = DelayedRestoreDriftClient(trigger=("Node02", "numOutputs", 2), drift_on_read=6)
        runner = self.runner(client, prepare=True)
        with self.assertRaises(RestorationFailure):
            runner.restore_initial()
        self.assertFalse(runner.summary["original_state_restored"])
        self.assertEqual(client.restore_reads, 6)
        self.assertEqual(len(client.writes), 2)
        observations = [entry for entry in self.events() if entry["event"] == "splitter_baseline_readback_attempt"
                        and entry["purpose"] == "final_initial_restore"]
        self.assertEqual([entry["attempt"] for entry in observations], [1, 2])
        phases = [entry["phase"] for entry in self.events() if entry["event"] == "transaction_phase"
                  and entry["transaction"] == "initial-state"]
        self.assertEqual(phases[-1], "RestorationFailed")
        self.assertNotIn("StateConfirmed", phases)

    def test_wrong_explicit_label_fails_before_writes(self):
        client = SplitterClient()
        client.labels[1] = "Unexpected Splitter"
        with self.assertRaisesRegex(AcceptanceFailure, "labels differ"):
            self.runner(client).run()
        self.assertEqual(client.writes, [])

    def test_duplicate_labels_fail_before_writes(self):
        client = SplitterClient()
        client.labels[1] = client.labels[0]
        with self.assertRaisesRegex(AcceptanceFailure, "Duplicate"):
            self.runner(client).run()
        self.assertEqual(client.writes, [])

    def test_truncated_inventory_fails_before_writes(self):
        client = SplitterClient()
        client.truncated = True
        with self.assertRaisesRegex(AcceptanceFailure, "complete bounded"):
            self.runner(client).run()
        self.assertEqual(client.writes, [])

    def test_diagnostics_fail_before_writes(self):
        client = SplitterClient()
        client.diagnostics = ["bounded search failed"]
        with self.assertRaisesRegex(AcceptanceFailure, "complete bounded"):
            self.runner(client).run()
        self.assertEqual(client.writes, [])

    def test_node_or_label_replacement_is_context_loss(self):
        client = SplitterClient()
        runner = self.runner(client, prepare=True)
        client.labels[1] = "Replaced Splitter"
        with self.assertRaises(ContextChanged):
            runner.capture()
        self.assertEqual(client.writes, [])

    def test_stale_document_after_dispatch_stops_without_restore(self):
        client = SplitterClient()
        runner = self.runner(client, prepare=True)
        client.change_document = True
        with self.assertRaises(ContextChanged):
            runner.run_case("Node01", "gain[1]", 0.5)
        self.assertEqual(len(client.writes), 1)
        self.assertEqual(client.values["Node01"]["gain[1]"], 0.5)

    def test_baseline_drift_before_dispatch_stops_without_overwriting_it(self):
        client = SplitterClient()
        runner = self.runner(client, prepare=True)
        client.values["Node01"]["gain[1]"] = 0.25
        with self.assertRaises(ContextChanged):
            runner.run_case("Node01", "gain[1]", 0.5)
        self.assertEqual(client.writes, [])
        self.assertEqual(client.values["Node01"]["gain[1]"], 0.25)

    def test_unknown_transport_after_dispatch_has_no_retry_or_blind_restore(self):
        client = SplitterClient()
        runner = self.runner(client, prepare=True)
        client.unknown = True
        with self.assertRaises(UnknownMutation):
            runner.run_case("Node01", "gain[1]", 0.5)
        self.assertEqual(len(client.writes), 1)
        self.assertEqual(runner.counts["changed_readback_restored"], 0)
        self.assertTrue(Journal.pending(self.journal.ledger_path))

    def test_unknown_receipt_after_dispatch_has_no_retry_or_restore(self):
        client = SplitterClient()
        runner = self.runner(client, prepare=True)
        original = client.receipt

        def unknown(result, outcome="observed"):
            return original(result, "outcome_unknown" if outcome == "applied_verified" else outcome)

        client.receipt = unknown
        with self.assertRaises(UnknownMutation):
            runner.run_case("Node01", "gain[1]", 0.5)
        self.assertEqual(len(client.writes), 1)

    def test_native_unknown_error_never_causes_blind_restore(self):
        client = SplitterClient()
        runner = self.runner(client, prepare=True)
        original = client.receipt

        def native_error(result, outcome="observed"):
            if outcome == "applied_verified":
                result = {"error": "native Splitter readback failed; outcome unknown"}
                outcome = "applied_unverified"
            return original(result, outcome)

        client.receipt = native_error
        with self.assertRaises(UnknownMutation):
            runner.run_case("Node01", "gain[1]", 0.5)
        self.assertEqual(len(client.writes), 1)

    def test_sibling_drift_stops_before_another_write_and_cannot_pass(self):
        client = SplitterClient()
        runner = self.runner(client, prepare=True)
        client.sibling_drift = True
        with self.assertRaises(RestorationFailure):
            runner.run_case("Node01", "gain[1]", 0.5)
        self.assertEqual(len(client.writes), 1)
        self.assertEqual(runner.counts["changed_readback_restored"], 0)
        self.assertEqual(runner.rows[-1]["status"], "restoration_failed")

    def test_inactive_branch_control_is_rejected_before_dispatch(self):
        client = SplitterClient()
        runner = self.runner(client)
        runner.initialize()
        before = runner.capture()
        with self.assertRaisesRegex(AcceptanceFailure, "inactive"):
            runner.write_checked("Node01", "gain[5]", 0.5, before, "inactive_test")
        self.assertEqual(client.writes, [])

    def test_restore_failure_stops_immediately_without_retry(self):
        client = SplitterClient()
        runner = self.runner(client, prepare=True)
        client.fail_restore = True
        with self.assertRaises(RestorationFailure):
            runner.run_case("Node01", "gain[1]", 0.5)
        self.assertEqual(len(client.writes), 2)
        self.assertEqual(client.values["Node01"]["gain[1]"], 0.5)
        self.assertEqual(runner.counts["changed_readback_restored"], 0)

    def test_ignored_write_cannot_count_as_changed_acceptance(self):
        client = SplitterClient()
        runner = self.runner(client, prepare=True)
        client.ignore = True
        with contextlib.redirect_stdout(io.StringIO()), self.assertRaises(AcceptanceFailure):
            runner.run_case("Node01", "gain[1]", 0.5)
        self.assertEqual(len(client.writes), 1)
        self.assertEqual(runner.counts["changed_readback_restored"], 0)
        self.assertEqual(runner.rows[-1]["status"], "change_failed_restored")

    def test_unchanged_request_cannot_pass_without_preparation(self):
        client = SplitterClient()
        runner = self.runner(client, prepare=True)
        with contextlib.redirect_stdout(io.StringIO()), self.assertRaises(AcceptanceFailure):
            runner.run_case("Node01", "numOutputs", 5)
        self.assertEqual(client.writes, [])
        self.assertEqual(runner.counts["changed_readback_restored"], 0)

    def test_exact_comparison_only_normalizes_native_mute_switches(self):
        client = SplitterClient()
        runner = self.runner(client, prepare=True)
        before = runner.capture()
        modified = copy.deepcopy(before)
        modified["Node01"]["params"]["mute[1]"]["value"] = 0
        self.assertTrue(same(before, modified))
        modified["Node01"]["params"]["gain[1]"]["value"] = 1.0000000000000002
        self.assertFalse(same(before, modified))
        modified["Node01"]["params"]["gain[1]"]["value"] = True
        with self.assertRaises(AcceptanceFailure):
            same(before, modified)

    def test_candidates_use_exact_binary_gain_and_integer_crossovers_between_neighbors(self):
        client = SplitterClient()
        runner = self.runner(client, prepare=True)
        params = runner.capture()["Node01"]["params"]
        self.assertEqual(candidate(params["gain[1]"], params), 0.5)
        for index in range(1, 5):
            name = f"splitFreq[{index}]"
            value = candidate(params[name], params)
            self.assertIs(type(value), int)
            self.assertNotEqual(value, params[name]["value"])
            self.assertLess(params[f"splitFreq[{index - 1}]"]["value"] if index > 1 else 20, value)
            self.assertLess(value, params[f"splitFreq[{index + 1}]"]["value"] if index < 4 else 20000)


if __name__ == "__main__":
    unittest.main()
