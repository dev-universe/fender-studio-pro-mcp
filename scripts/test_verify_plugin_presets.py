#!/usr/bin/env python3
"""Offline preset acceptance behavior. No DAW or native add-on is launched."""
import copy
from contextlib import redirect_stdout
import io
import json
import math
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from test_verify_all_parameters import INSTANCE, TARGET, number
from test_verify_plugin_states import OTHER, StateClient
from verify_all_parameters import (
    AcceptanceFailure, ContextChanged, Journal, PRIVATE_ROOT, RestorationFailure,
    UnknownMutation, value_identity,
)
from verify_plugin_presets import CATALOG_SOURCE, PresetRunner, main


def writable(name, value, **kwargs):
    return {**number(name, value, **kwargs), "access_metadata_complete": True}


class PresetClient(StateClient):
    def __init__(self, parameters=None, count=3):
        super().__init__(parameters or [writable("gain", 0.25), writable("mix", 0.5)])
        for parameters in self.targets.values():
            for parameter in parameters:
                parameter["access_metadata_complete"] = True
        self.entries = [{"index": i, "name": "default" if i == 0 else f"Preset {i}",
                         "class_key": "{" + INSTANCE["class_uid"] + "}", "read_only": True,
                         "availability": "local_read_only", "recallable": True} for i in range(count)]
        self.catalogs, self.released_catalogs, self.preset_calls = {}, {}, []
        self.catalog_count = self.applies = self.changed_reads = 0
        self.active_preset = False
        self.catalog_fault = self.change_mode = None
        self.deny_apply = self.unknown_apply = self.unknown_receipt = False
        self.context_loss = self.stale_uid = self.accept_released_catalog = False
        self.unmute_after_catalog = self.readonly_changed_unstable = False
        self.native_unknown = self.missing_accepted = self.false_rejection = False
        self.expire_before_apply = self.writable_proof_late = False
        self.restore_ulp_drift = False
        self.preset_value = 0.75

    def select_target(self, args):
        super().select_target(args)
        if self.stale_uid and self.active_preset and self.target_name == TARGET["channel"]:
            self.instance["object_uid"] = "DEFDEFDE-1234-1234-1234-123456789ABC"

    def call(self, name, args):
        if name == "plugin_parameters" and not args.get("names") and self.active_preset:
            self.changed_reads += 1
            if self.change_mode == "unstable" and self.changed_reads == 2:
                self.parameters[0].update(writable("gain", math.nextafter(0.75, math.inf)))
            if self.change_mode == "schema_unstable" and self.changed_reads == 2:
                self.parameters[0]["max"] = 3.0
            if self.readonly_changed_unstable:
                meter = self.parameters[-1]
                meter.update(writable(meter["name"], 0.01 * self.changed_reads, readonly=True))
            if self.writable_proof_late:
                self.parameters[0]["access_metadata_complete"] = self.changed_reads > 1
        if name == "plugin_state" and args["action"] == "restore":
            self.active_preset = False
        if name != "plugin_presets":
            result = super().call(name, args)
            if name == "plugin_state" and args["action"] == "restore" and self.restore_ulp_drift:
                self.parameters[0].update(writable("gain", math.nextafter(0.25, math.inf)))
            return result
        self.select_target(args)
        self.calls.append((name, copy.deepcopy(args)))
        self.preset_calls.append(copy.deepcopy(args))
        self.assert_bound(args)
        action = args["action"]
        token = args.get("catalog_token")
        if action == "list":
            if token is None:
                self.catalog_count += 1
                token = f"{0xCA700000 + self.catalog_count:064X}"
                self.catalogs[token] = dict(self.instance)
            elif self.catalogs.get(token) != self.instance:
                return self.refused()
            offset, limit = args["offset"], args["limit"]
            end = min(offset + limit, len(self.entries))
            page = {"ok": True, "source": CATALOG_SOURCE, "instance": dict(self.instance),
                    "catalog_token": token, "catalog_complete": True, "observed_count": len(self.entries),
                    "offset": offset, "entries": copy.deepcopy(self.entries[offset:end]),
                    "next": end if end < len(self.entries) else None, "outcome": "observed",
                    "factory_origin": "not_inferred"}
            fault = self.catalog_fault
            if fault == "incomplete":
                page["catalog_complete"] = False
            elif fault == "too_many":
                page["observed_count"] = 4097
            elif fault == "duplicate":
                page["entries"][-1]["index"] = offset
            elif fault == "bad_next":
                page["next"] = offset
            elif fault == "url":
                page["entries"][0]["url"] = "file:///forbidden/preset"
            elif fault == "instance" and offset:
                page["instance"]["object_uid"] = "another Device"
            elif fault == "token" and offset:
                page["catalog_token"] = "F" * 64
            elif fault == "count" and offset:
                page["observed_count"] -= 1
            elif fault == "empty_page":
                page["entries"] = []
            if self.unmute_after_catalog:
                self.muted = False
            return self.receipt(page, "applied_unverified")
        stored = self.catalogs.get(token)
        if stored is None and self.accept_released_catalog:
            stored = self.released_catalogs.get(token)
        if stored != self.instance:
            return self.refused()
        if action == "release":
            self.released_catalogs[token] = self.catalogs.pop(token)
            return self.receipt({"ok": True, "source": CATALOG_SOURCE, "catalog_token": token,
                                 "instance": dict(self.instance), "released": True,
                                 "outcome": "applied_verified"}, "applied_unverified")
        if action != "apply":
            raise AssertionError(action)
        if self.expire_before_apply:
            return self.refused()
        if self.deny_apply:
            if self.false_rejection:
                self.parameters[0].update(writable("gain", 0.75))
            return self.refused()
        self.applies += 1
        self.changed_reads = 0
        original_instance = dict(self.instance)
        self.active_preset = True
        if self.change_mode not in ("unchanged", "schema_only", "readonly_only"):
            self.parameters[0].update(writable("gain", self.preset_value,
                                              lower=self.parameters[0]["min"], upper=self.parameters[0]["max"]))
        if self.change_mode == "readonly_only":
            self.parameters[-1].update(writable(self.parameters[-1]["name"], 0.9, readonly=True))
        if self.change_mode in ("schema", "schema_only"):
            self.parameters[0]["max"] = 2.0
        if self.context_loss:
            self.document = "test:2"
        if self.unknown_apply:
            raise TimeoutError("preset interrupted after dispatch")
        return self.receipt({"ok": True, "source": CATALOG_SOURCE, "instance": original_instance,
                             "catalog_token": token, "index": args["index"], "accepted": not self.missing_accepted,
                             "verified": False, "outcome": "unknown" if self.native_unknown else "applied_unverified"},
                            "unknown" if self.unknown_receipt else "applied_unverified")

    def assert_bound(self, args):
        if args.get("expected_document") != "test:1" or args.get("expected_instance") != self.instance["object_uid"]:
            raise AssertionError("Preset call was not bound to its document and native Device")

    def refused(self):
        return self.receipt({"native_outcome": "not_applied", "errors": ["invalid catalog or target"]},
                            "not_applied", "invalid catalog or target")

    def close(self):
        pass


class PresetAcceptanceTests(unittest.TestCase):
    def setUp(self):
        PRIVATE_ROOT.mkdir(exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(prefix="preset-runner-unit-", dir=PRIVATE_ROOT)
        self.journal = Journal(Path(self.temporary.name) / "run")

    def tearDown(self):
        self.journal.close()
        self.temporary.cleanup()

    def runner(self, client, read_only_observations=False):
        runner = PresetRunner(client, self.journal, "MCP Validation Scratch", settle_attempts=4,
                              settle_interval=0, read_only_observations=read_only_observations)
        runner.initialize()
        return runner

    def events(self, event=None):
        events = [json.loads(line) for line in self.journal.ledger_path.read_text().splitlines()]
        return [item for item in events if item["event"] == event] if event else events

    def test_recall_proves_two_changed_snapshots_and_exact_whole_restoration(self):
        client = PresetClient()
        before = [value_identity(p) for p in client.parameters]
        row = self.runner(client).run_preset_case(TARGET)
        self.assertEqual(row["status"], "preset_case_verified")
        self.assertEqual(row["presets"][0]["changed_writable_parameters"], ["gain"])
        self.assertEqual(row["presets"][0]["fresh_whole_provider_reads"], 2)
        self.assertEqual(len(self.events("preset_changed_observation")), 2)
        self.assertEqual([value_identity(p) for p in client.parameters], before)
        self.assertEqual([call["action"] for call in client.state_calls], ["capture", "restore", "release"])
        self.assertEqual([call["action"] for call in client.preset_calls], ["list", "apply", "release", "apply"])
        self.assertFalse(client.catalogs)
        self.assertFalse(client.tokens)
        self.assertEqual(client.applies, 1)
        self.assertFalse(row["opaque_contents_verified"])
        self.assertFalse(row["preset_display_verified"])
        self.assertEqual(Journal.pending(self.journal.ledger_path), {})
        observations = self.events("state_baseline_observation")
        self.assertGreaterEqual(sum(item["matches"] for item in observations), 6)

    def test_catalog_paginates_complete_issued_entries_with_same_bound_token(self):
        client = PresetClient(count=130)
        row = self.runner(client).run_preset_case(TARGET, [129])
        pages = [call for call in client.preset_calls if call["action"] == "list"]
        self.assertEqual([call["offset"] for call in pages], [0, 64, 128])
        self.assertEqual([call["limit"] for call in pages], [64, 64, 64])
        self.assertNotIn("catalog_token", pages[0])
        self.assertEqual(pages[1]["catalog_token"], pages[2]["catalog_token"])
        self.assertEqual(row["catalog_count"], 130)
        self.assertEqual(row["presets"][0]["index"], 129)
        self.assertTrue(all(call["expected_instance"] == INSTANCE["object_uid"]
                            and call["expected_document"] == "test:1" for call in client.preset_calls))
        self.assertEqual(row["factory_origin"], "not_inferred")

    def test_explicit_indices_include_zero_and_repeat_only_after_full_recovery(self):
        client = PresetClient()
        row = self.runner(client).run_preset_case(TARGET, [0, 2, 2])
        self.assertEqual([entry["index"] for entry in row["presets"]], [0, 2, 2])
        self.assertEqual(client.restore_count, 3)
        self.assertEqual(client.capture_count, 1)
        self.assertEqual(client.catalog_count, 1)
        self.assertEqual(row["status"], "preset_case_verified")

    def test_default_does_not_fall_back_to_index_zero(self):
        client = PresetClient(count=1)
        with self.assertRaisesRegex(AcceptanceFailure, "outside"):
            self.runner(client).run_preset_case(TARGET)
        self.assertEqual(client.applies, 0)

    def test_nonrecallable_entry_is_refused_before_apply(self):
        client = PresetClient()
        client.entries[1]["recallable"] = False
        with self.assertRaisesRegex(AcceptanceFailure, "not recallable"):
            self.runner(client).run_preset_case(TARGET)
        self.assertEqual(client.applies, 0)

    def test_incomplete_catalog_never_dispatches_recall(self):
        client = PresetClient()
        client.catalog_fault = "incomplete"
        with self.assertRaisesRegex(AcceptanceFailure, "complete bounded"):
            self.runner(client).run_preset_case(TARGET)
        self.assertEqual(client.applies, 0)

    def test_catalog_over_bound_never_dispatches_recall(self):
        client = PresetClient()
        client.catalog_fault = "too_many"
        with self.assertRaises(AcceptanceFailure):
            self.runner(client).run_preset_case(TARGET)
        self.assertEqual(client.applies, 0)

    def test_catalog_duplicate_indices_never_dispatch_recall(self):
        client = PresetClient()
        client.catalog_fault = "duplicate"
        with self.assertRaisesRegex(AcceptanceFailure, "contiguous"):
            self.runner(client).run_preset_case(TARGET)
        self.assertEqual(client.applies, 0)

    def test_catalog_pagination_must_advance(self):
        client = PresetClient(count=65)
        client.catalog_fault = "bad_next"
        with self.assertRaisesRegex(AcceptanceFailure, "pagination"):
            self.runner(client).run_preset_case(TARGET)
        self.assertEqual(client.applies, 0)

    def test_catalog_page_must_cover_every_issued_index(self):
        client = PresetClient()
        client.catalog_fault = "empty_page"
        with self.assertRaisesRegex(AcceptanceFailure, "coverage"):
            self.runner(client).run_preset_case(TARGET)
        self.assertEqual(client.applies, 0)

    def test_catalog_rejects_location_exports(self):
        client = PresetClient()
        client.catalog_fault = "url"
        with self.assertRaisesRegex(AcceptanceFailure, "paths, URLs"):
            self.runner(client).run_preset_case(TARGET)
        self.assertEqual(client.applies, 0)

    def test_catalog_page_uid_replacement_stops_without_further_mutations(self):
        client = PresetClient(count=65)
        client.catalog_fault = "instance"
        with self.assertRaises(ContextChanged):
            self.runner(client).run_preset_case(TARGET)
        self.assertEqual(client.applies, 0)
        self.assertEqual([call["action"] for call in client.state_calls], ["capture"])

    def test_catalog_token_cannot_change_between_pages(self):
        client = PresetClient(count=65)
        client.catalog_fault = "token"
        with self.assertRaisesRegex(AcceptanceFailure, "different issued catalog"):
            self.runner(client).run_preset_case(TARGET)
        self.assertEqual(client.applies, 0)

    def test_catalog_count_cannot_change_between_pages(self):
        client = PresetClient(count=65)
        client.catalog_fault = "count"
        with self.assertRaises(ContextChanged):
            self.runner(client).run_preset_case(TARGET)
        self.assertEqual(client.applies, 0)

    def test_accepted_but_unchanged_is_not_verified_and_tokens_are_released(self):
        client = PresetClient()
        client.change_mode = "unchanged"
        runner = self.runner(client)
        row = runner.run_preset_case(TARGET)
        self.assertEqual(row["status"], "preset_case_not_verified")
        self.assertEqual(row["presets"][0]["status"], "no_state_change_not_verified")
        self.assertEqual(runner.counts["preset_case_verified"], 0)
        self.assertEqual(client.restore_count, 1)
        self.assertFalse(client.catalogs)
        self.assertFalse(client.tokens)

    def test_read_only_change_is_not_writable_change_proof(self):
        client = PresetClient([writable("gain", 0.25), writable("meter", 0.1, readonly=True)])
        client.change_mode = "readonly_only"
        row = self.runner(client).run_preset_case(TARGET)
        self.assertEqual(row["presets"][0]["status"], "no_state_change_not_verified")

    def test_incomplete_writable_metadata_cannot_prove_change(self):
        client = PresetClient()
        client.parameters[0]["access_metadata_complete"] = False
        row = self.runner(client).run_preset_case(TARGET)
        self.assertEqual(row["presets"][0]["status"], "no_state_change_not_verified")

    def test_writability_must_be_authoritative_on_both_changed_reads(self):
        client = PresetClient()
        client.writable_proof_late = True
        row = self.runner(client).run_preset_case(TARGET)
        self.assertEqual(row["presets"][0]["status"], "no_state_change_not_verified")

    def test_stable_actual_float_facade_bits_prove_change_without_tolerance(self):
        client = PresetClient([writable("gain", 7957.139611244202, upper=10000.0)])
        client.preset_value = 3700.000047683716
        row = self.runner(client).run_preset_case(TARGET)
        self.assertEqual(row["status"], "preset_case_verified")
        self.assertEqual(client.parameters[0]["native_value"]["data"]["bits_hex"], "40BF1523BD900000")

    def test_known_denied_recall_is_unchanged_and_not_verified(self):
        client = PresetClient()
        client.deny_apply = True
        row = self.runner(client).run_preset_case(TARGET)
        self.assertEqual(row["presets"][0]["status"], "preset_apply_rejected")
        self.assertEqual(client.restore_count, 0)
        self.assertFalse(client.tokens)
        self.assertFalse(client.catalogs)

    def test_expired_issued_catalog_cannot_count_as_verified(self):
        client = PresetClient()
        client.expire_before_apply = True
        row = self.runner(client).run_preset_case(TARGET)
        self.assertEqual(row["presets"][0]["status"], "preset_apply_rejected")
        self.assertEqual(client.applies, 0)

    def test_false_not_applied_with_changed_state_stops_without_blind_restore(self):
        client = PresetClient()
        client.deny_apply = client.false_rejection = True
        with self.assertRaises(UnknownMutation):
            self.runner(client).run_preset_case(TARGET)
        self.assertEqual(client.restore_count, 0)
        self.assertEqual([call["action"] for call in client.preset_calls], ["list", "apply"])

    def test_applied_without_native_acceptance_is_restored_then_rejected(self):
        client = PresetClient()
        client.missing_accepted = True
        with self.assertRaisesRegex(AcceptanceFailure, "not accepted"):
            self.runner(client).run_preset_case(TARGET)
        self.assertEqual(client.restore_count, 1)
        self.assertEqual(client.parameters[0]["value"], 0.25)

    def test_changed_metadata_is_allowed_until_exact_capsule_restoration(self):
        client = PresetClient()
        client.change_mode = "schema"
        row = self.runner(client).run_preset_case(TARGET)
        self.assertTrue(row["presets"][0]["baseline_schema_changed"])
        self.assertEqual(row["status"], "preset_case_verified")
        self.assertEqual(client.parameters[0]["max"], 1.0)

    def test_schema_change_without_changed_values_is_not_proof(self):
        client = PresetClient()
        client.change_mode = "schema_only"
        row = self.runner(client).run_preset_case(TARGET)
        self.assertTrue(row["presets"][0]["baseline_schema_changed"])
        self.assertEqual(row["presets"][0]["status"], "no_state_change_not_verified")

    def test_one_ulp_changed_value_instability_is_restored_then_fails(self):
        client = PresetClient()
        client.change_mode = "unstable"
        with self.assertRaisesRegex(AcceptanceFailure, "unstable"):
            self.runner(client).run_preset_case(TARGET)
        self.assertEqual(client.parameters[0]["value"], 0.25)
        self.assertEqual(client.restore_count, 1)
        self.assertEqual(client.applies, 1)

    def test_schema_instability_is_restored_then_fails(self):
        client = PresetClient()
        client.change_mode = "schema_unstable"
        with self.assertRaisesRegex(AcceptanceFailure, "unstable"):
            self.runner(client).run_preset_case(TARGET)
        self.assertEqual(client.parameters[0]["max"], 1.0)
        self.assertEqual(client.restore_count, 1)

    def test_readonly_drift_in_changed_phase_is_strict_by_default(self):
        client = PresetClient([writable("gain", 0.25), writable("meter", 0.1, readonly=True)])
        client.readonly_changed_unstable = True
        with self.assertRaisesRegex(AcceptanceFailure, "unstable"):
            self.runner(client).run_preset_case(TARGET)
        self.assertEqual(client.restore_count, 1)

    def test_explicit_authoritative_readonly_observation_policy_is_retained(self):
        client = PresetClient([writable("gain", 0.25), writable("meter", 0.1, readonly=True)])
        client.readonly_changed_unstable = True
        row = self.runner(client, True).run_preset_case(TARGET)
        self.assertEqual(row["status"], "preset_case_verified")
        self.assertEqual(row["read_only_values"], "observed_not_restored")
        self.assertEqual(self.events("preset_changed_observation")[-1]["read_only_changed"], ["meter"])

    def test_restore_false_acceptance_stops_before_release_or_next_selection(self):
        client = PresetClient()
        client.ignore_load = True
        with self.assertRaises(RestorationFailure):
            self.runner(client).run_preset_case(TARGET, [1, 2])
        self.assertEqual(client.applies, 1)
        self.assertEqual(client.restore_count, 1)
        self.assertNotIn("release", [call["action"] for call in client.preset_calls])

    def test_schema_must_be_exact_again_after_restoration(self):
        client = PresetClient()
        client.schema_drift = True
        with self.assertRaises(RestorationFailure):
            self.runner(client).run_preset_case(TARGET)
        self.assertEqual(client.applies, 1)

    def test_one_ulp_baseline_restore_mismatch_remains_a_failure(self):
        client = PresetClient()
        client.restore_ulp_drift = True
        with self.assertRaises(RestorationFailure):
            self.runner(client).run_preset_case(TARGET)
        self.assertEqual(client.restore_count, 1)
        self.assertNotIn("release", [call["action"] for call in client.preset_calls])

    def test_unknown_apply_is_never_retried_restored_or_released(self):
        client = PresetClient()
        client.unknown_apply = True
        with self.assertRaises(UnknownMutation):
            self.runner(client).run_preset_case(TARGET, [1, 2])
        self.assertEqual(client.applies, 1)
        self.assertEqual([call["action"] for call in client.state_calls], ["capture"])
        self.assertEqual([call["action"] for call in client.preset_calls], ["list", "apply"])
        self.assertTrue(Journal.pending(self.journal.ledger_path))

    def test_unknown_receipt_is_never_treated_as_acceptance(self):
        client = PresetClient()
        client.unknown_receipt = True
        with self.assertRaises(UnknownMutation):
            self.runner(client).run_preset_case(TARGET)
        self.assertEqual(client.restore_count, 0)

    def test_embedded_native_unknown_stops_even_if_generic_receipt_says_applied(self):
        client = PresetClient()
        client.native_unknown = True
        with self.assertRaises(UnknownMutation):
            self.runner(client).run_preset_case(TARGET)
        self.assertEqual(client.restore_count, 0)
        self.assertEqual([call["action"] for call in client.preset_calls], ["list", "apply"])

    def test_document_loss_after_apply_stops_without_blind_restore(self):
        client = PresetClient()
        client.context_loss = True
        with self.assertRaises(ContextChanged):
            self.runner(client).run_preset_case(TARGET)
        self.assertEqual(client.restore_count, 0)
        self.assertEqual(client.applies, 1)

    def test_changed_native_uid_stops_without_blind_restore(self):
        client = PresetClient()
        client.stale_uid = True
        with self.assertRaises(ContextChanged):
            self.runner(client).run_preset_case(TARGET)
        self.assertEqual(client.restore_count, 0)

    def test_unknown_restore_stops_without_catalog_cleanup_mutations(self):
        client = PresetClient()
        client.unknown_load = True
        with self.assertRaises(UnknownMutation):
            self.runner(client).run_preset_case(TARGET)
        self.assertEqual(client.restore_count, 1)
        self.assertEqual([call["action"] for call in client.preset_calls], ["list", "apply"])

    def test_released_catalog_cannot_be_reported_verified_when_accepted_again(self):
        client = PresetClient()
        client.accept_released_catalog = True
        runner = self.runner(client)
        with self.assertRaises(UnknownMutation):
            runner.run_preset_case(TARGET)
        self.assertEqual(client.applies, 2)
        self.assertEqual(client.restore_count, 1)
        self.assertEqual(runner.counts["preset_case_verified"], 0)

    def test_optional_wrong_target_check_proves_both_providers_unchanged(self):
        client = PresetClient()
        row = self.runner(client).run_preset_case(TARGET, wrong_target=OTHER)
        self.assertTrue(row["wrong_target_catalog_refused"])
        foreign = [call for call in client.preset_calls if call["channel"] == OTHER["channel"]]
        self.assertEqual([call["action"] for call in foreign], ["apply"])
        self.assertNotEqual(foreign[0]["expected_instance"], INSTANCE["object_uid"])
        self.assertEqual(client.applies, 1)

    def test_fixture_muted_guard_is_rechecked_after_catalog_before_apply(self):
        client = PresetClient()
        client.unmute_after_catalog = True
        with self.assertRaises(AcceptanceFailure):
            self.runner(client).run_preset_case(TARGET)
        self.assertEqual(client.applies, 0)

    def test_ledger_records_in_flight_before_each_preset_mutation(self):
        client = PresetClient()
        self.runner(client).run_preset_case(TARGET)
        events = self.events()
        in_flight = [item for item in events if item["event"] == "rpc_in_flight" and item["tool"] == "plugin_presets"]
        self.assertEqual(len(in_flight), 4)
        self.assertTrue(all(item["mutation"] is True for item in in_flight))
        phases = [item["phase"] for item in events if item["event"] == "transaction_phase"]
        self.assertIn("ChangeInFlight", phases)
        self.assertIn("ReleasedCatalogCheckInFlight", phases)
        self.assertEqual(phases[-1], "StateConfirmed")

    def test_cli_returns_nonzero_for_accepted_but_unchanged(self):
        client = PresetClient()
        client.change_mode = "unchanged"
        fixture = Path(self.temporary.name) / "fixtures.json"
        fixture.write_text(json.dumps([TARGET]))
        output = io.StringIO()
        with patch("verify_plugin_presets.McpClient", return_value=client), redirect_stdout(output):
            result = main(["--binary", __file__, "--fixtures", str(fixture),
                           "--scratch-title", "MCP Validation Scratch", "--plugin", TARGET["plugin"],
                           "--output-dir", str(Path(self.temporary.name) / "cli"), "--settle-interval", "0"])
        self.assertEqual(result, 2)
        self.assertIn('"status": "preset_cases_not_verified"', output.getvalue())


if __name__ == "__main__":
    unittest.main()
