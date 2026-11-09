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
from test_parameter_digest import floating, normalized
from test_verify_all_parameter_states import DigestSweepClient
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


class CatalogClient(PresetClient):
    description = DigestSweepClient.description
    indexed = DigestSweepClient.indexed

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.full_snapshots = 0
        self.page_hook = self.lifecycle_hook = None

    def call(self, name, args):
        if (name == "plugin_presets" and args["action"] == "list"
                and args.get("catalog_token") in self.released_catalogs and self.accept_released_catalog):
            token = args["catalog_token"]
            self.catalogs[token] = self.released_catalogs[token]
        result = super().call(name, args)
        body = result["structuredContent"]["result"]
        if name == "plugin_parameters" and not args.get("names"):
            if args["offset"] == 0:
                self.full_snapshots += 1
            body["params"] = [self.indexed(parameter, parameter["native"]["index"]) for parameter in body["params"]]
            if self.page_hook:
                self.page_hook(self, body)
        if name == "plugin_presets":
            if result["structuredContent"]["outcome"] == "not_applied":
                body.update(source=CATALOG_SOURCE, instance=dict(self.instance), ok=False)
            if args["action"] == "release" and body.get("released") is True:
                body["verified"] = True
            if self.lifecycle_hook:
                self.lifecycle_hook(self, args, result["structuredContent"])
        return result


class PresetAcceptanceTests(unittest.TestCase):
    def setUp(self):
        PRIVATE_ROOT.mkdir(exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(prefix="preset-runner-unit-", dir=PRIVATE_ROOT)
        self.journal = Journal(Path(self.temporary.name) / "run")
        self.runner_count = 0

    def tearDown(self):
        self.journal.close()
        self.temporary.cleanup()

    def runner(self, client, read_only_observations=False):
        if self.runner_count:
            self.journal.close()
            self.journal = Journal(Path(self.temporary.name) / f"run-{self.runner_count}")
        self.runner_count += 1
        runner = PresetRunner(client, self.journal, "MCP Validation Scratch", settle_attempts=4,
                              settle_interval=0, read_only_observations=read_only_observations)
        runner.initialize()
        return runner

    def events(self, event=None):
        events = [json.loads(line) for line in self.journal.ledger_path.read_text().splitlines()]
        return [item for item in events if item["event"] == event] if event else events

    def assert_catalog_only_calls(self, client, actions):
        self.assertEqual([call["action"] for call in client.preset_calls], actions)
        self.assertFalse(client.state_calls)
        self.assertFalse(client.writes)
        self.assertEqual(client.applies, 0)
        self.assertFalse(any(name in ("plugin_editor_open", "plugin_parameter_digest") for name, _ in client.calls))

    def test_catalog_only_inventory_never_captures_applies_or_writes(self):
        client = CatalogClient(count=130)
        row = self.runner(client).run_catalog_case(TARGET)
        self.assertEqual(row["status"], "catalog_case_verified")
        self.assertEqual(row["catalog_count"], 130)
        self.assertTrue(row["catalog_complete"])
        self.assert_catalog_only_calls(client, ["list", "list", "list", "release", "list"])
        pages = [call for call in client.preset_calls if call["action"] == "list"]
        self.assertEqual([call["offset"] for call in pages], [0, 64, 128, 0])
        self.assertTrue(all(call["limit"] == 64 and "index" not in call for call in pages))
        self.assertEqual(len({call["catalog_token"] for call in client.preset_calls[1:]}), 1)
        self.assertTrue(all(call["expected_instance"] == INSTANCE["object_uid"]
                            and call["expected_document"] == "test:1" for call in client.preset_calls))
        self.assertEqual(client.full_snapshots, 7)
        observations = self.events("catalog_baseline_observation")
        self.assertEqual([item["boundary"] for item in observations],
                         ["catalog_discovered"] * 2 + ["catalog_released"] * 2 + ["released_catalog_refused"] * 2)
        self.assertTrue(all(item["matches"] and item["value_hash_field"] == "all_values_sha256" for item in observations))
        self.assertFalse(client.catalogs)
        self.assertFalse(Journal.pending(self.journal.ledger_path))
        self.assertFalse(row["preset_recalls_verified"])
        self.assertFalse(row["all_writable_parameters_verified"])
        self.assertFalse(row["opaque_contents_verified"])

    def test_catalog_only_zero_entries_still_releases_and_refuses_list_without_index(self):
        client = CatalogClient(count=0)
        row = self.runner(client).run_catalog_case(TARGET)
        self.assertEqual(row["catalog_count"], 0)
        self.assertEqual(row["status"], "catalog_case_verified")
        self.assert_catalog_only_calls(client, ["list", "release", "list"])
        self.assertEqual(client.preset_calls[-1]["offset"], 0)
        self.assertNotIn("index", client.preset_calls[-1])
        self.assertEqual(client.full_snapshots, 7)

    def test_catalog_only_public_provider_pages_are_complete_for_every_observation(self):
        client = CatalogClient([writable(f"parameter{i}", 0.25) for i in range(130)])
        self.runner(client).run_catalog_case(TARGET)
        pages = [args for name, args in client.calls if name == "plugin_parameters"]
        self.assertEqual([page["offset"] for page in pages], [0, 64, 128] * 7)
        self.assertTrue(all("names" not in page and page["limit"] == 64 for page in pages))
        self.assertTrue(all(page.get("expected_instance") == INSTANCE["object_uid"] for page in pages[3:]))

    def test_catalog_only_rejects_legacy_metadata_before_issuing_catalog(self):
        client = PresetClient()
        with self.assertRaisesRegex(AcceptanceFailure, "Complete typed"):
            self.runner(client).run_catalog_case(TARGET)
        self.assert_catalog_only_calls(client, [])

    def test_catalog_only_two_fresh_reads_at_each_boundary_catch_typed_and_display_drift(self):
        for snapshot in range(2, 8):
            for field, value in (("native_value", floating(0.1)), ("native_default", floating(0.1)),
                                 ("native_normalized", normalized(0.75)[0]),
                                 ("text", "unexpected display")):
                with self.subTest(snapshot=snapshot, field=field):
                    client = CatalogClient()
                    def drift(client, page):
                        if client.full_snapshots != snapshot:
                            return
                        parameter = page["params"][0]
                        parameter[field] = copy.deepcopy(value)
                        parameter["native"]["native_description"][field] = copy.deepcopy(value)
                        if field == "native_default":
                            parameter["default"] = parameter["native"]["native_description"]["default"] = 0.1
                        if field == "native_value":
                            parameter["value"] = parameter["native"]["native_description"]["value"] = 0.1
                        if field == "native_normalized":
                            parameter["normalized"] = parameter["native"]["native_description"]["normalized"] = normalized(0.75)[1]
                    client.page_hook = drift
                    runner = self.runner(client)
                    with self.assertRaises((AcceptanceFailure, ContextChanged)):
                        runner.run_catalog_case(TARGET)
                    expected = ["list"] if snapshot <= 3 else ["list", "release"] if snapshot <= 5 else ["list", "release", "list"]
                    self.assert_catalog_only_calls(client, expected)
                    self.assertEqual(client.full_snapshots, snapshot)
                    self.assertEqual(runner.counts["catalog_case_verified"], 0)
                    self.assertTrue(Journal.pending(self.journal.ledger_path))

    def test_catalog_only_authoritative_readonly_exclusion_is_explicit_and_schema_stays_strict(self):
        for observe in (False, True):
            with self.subTest(observe=observe):
                client = CatalogClient([writable("gain", 0.25), writable("meter", 0.1, readonly=True)])
                client.read_only_drift = True
                if observe:
                    row = self.runner(client, True).run_catalog_case(TARGET)
                    self.assertEqual(row["status"], "catalog_case_verified")
                    self.assertTrue(all(item["read_only_values_changed"] for item in self.events("catalog_baseline_observation")[-6:]))
                else:
                    with self.assertRaises(AcceptanceFailure):
                        self.runner(client).run_catalog_case(TARGET)
                    self.assert_catalog_only_calls(client, ["list"])

    def test_catalog_only_readonly_metadata_cannot_be_excluded_when_missing_or_changed(self):
        for fault in ("missing_access", "default", "writable"):
            with self.subTest(fault=fault):
                client = CatalogClient([writable("gain", 0.25), writable("meter", 0.1, readonly=True)])
                def corrupt(client, page):
                    if client.full_snapshots < 2:
                        return
                    parameter = page["params"][1]
                    if fault == "missing_access":
                        parameter.pop("access_metadata_complete")
                    elif fault == "default":
                        parameter["default"] = parameter["native"]["native_description"]["default"] = 0.1
                        parameter["native_default"] = parameter["native"]["native_description"]["native_default"] = floating(0.1)
                    else:
                        parameter["writable"] = parameter["native"]["native_description"]["writable"] = True
                client.page_hook = corrupt
                with self.assertRaises((AcceptanceFailure, ContextChanged)):
                    self.runner(client, True).run_catalog_case(TARGET)
                self.assert_catalog_only_calls(client, ["list"])

    def test_catalog_only_readonly_flag_does_not_exclude_a_writable_observation(self):
        parameter = writable("gain", 0.25)
        parameter["readOnly"] = True
        client = CatalogClient([parameter])
        def drift(client, page):
            if client.full_snapshots == 2:
                parameter = page["params"][0]
                parameter["text"] = parameter["native"]["native_description"]["text"] = "writable observation changed"
        client.page_hook = drift
        with self.assertRaises(AcceptanceFailure):
            self.runner(client, True).run_catalog_case(TARGET)
        self.assert_catalog_only_calls(client, ["list"])

    def test_catalog_only_exact_negative_zero_normalized_bits_are_retained(self):
        parameter = writable("gain", 0.25)
        parameter["normalized"] = -0.0
        client = CatalogClient([parameter])
        def drift(client, page):
            if client.full_snapshots == 3:
                parameter = page["params"][0]
                for value in (parameter, parameter["native"]["native_description"]):
                    value["native_normalized"] = normalized(0.0)[0]
                    value["normalized"] = 0.0
        client.page_hook = drift
        with self.assertRaises(AcceptanceFailure):
            self.runner(client).run_catalog_case(TARGET)
        self.assert_catalog_only_calls(client, ["list"])
        self.assertEqual(client.full_snapshots, 3)

    def test_catalog_only_unknown_malformed_and_contradictory_lifecycle_stops_without_replay(self):
        faults = [("outer", value) for value in ("unknown", {}, [], None, True)]
        faults += [(field, value) for field in ("outcome", "native_outcome")
                   for value in ("unknown", {}, [], None, True, "not_applied")]
        faults += [("result", value) for value in ({}, [], None)]
        for phase in (1, 2, 3):
            for field, value in faults:
                with self.subTest(phase=phase, field=field, value=value):
                    client = CatalogClient()
                    def corrupt(client, args, receipt):
                        if len(client.preset_calls) != phase:
                            return
                        if field in ("outer", "result"):
                            receipt["outcome" if field == "outer" else "result"] = copy.deepcopy(value)
                        else:
                            receipt["result"][field] = copy.deepcopy(value)
                    client.lifecycle_hook = corrupt
                    runner = self.runner(client)
                    # A known refusal is the required final outcome, so retain it.
                    if phase == 3 and value == "not_applied":
                        continue
                    with self.assertRaises((UnknownMutation, AcceptanceFailure, ContextChanged)):
                        runner.run_catalog_case(TARGET)
                    self.assert_catalog_only_calls(client, ["list", "release", "list"][:phase])
                    self.assertEqual(runner.counts["catalog_case_verified"], 0)
                    self.assertTrue(Journal.pending(self.journal.ledger_path))

    def test_catalog_only_known_outcome_phase_contradictions_never_trigger_cleanup(self):
        for phase, changes in ((1, {"outcome": "applied_verified"}),
                               (1, {"released": True}), (1, {"accepted": True}),
                               (2, {"outcome": "observed"}), (2, {"released": False}),
                               (2, {"verified": 1}), (2, {"native_outcome": "applied_unverified"}),
                               (3, {"native_outcome": "applied_verified"}),
                               (3, {"ok": True}), (3, {"verified": 0}), (3, {"accepted": True})):
            with self.subTest(phase=phase, changes=changes):
                client = CatalogClient()
                def corrupt(client, args, receipt):
                    if len(client.preset_calls) == phase:
                        receipt["result"].update(changes)
                client.lifecycle_hook = corrupt
                with self.assertRaises(UnknownMutation):
                    self.runner(client).run_catalog_case(TARGET)
                self.assert_catalog_only_calls(client, ["list", "release", "list"][:phase])
                self.assertTrue(Journal.pending(self.journal.ledger_path))

    def test_catalog_only_released_token_refusal_requires_explicit_boolean_false_ok(self):
        cases = [("missing", None), ("all_flags_missing", None)]
        cases += [("value", value) for value in (None, True, 0, 1, 0.0, "false", {}, [], [False])]
        for fault, value in cases:
            with self.subTest(fault=fault, value=value):
                client = CatalogClient()
                def corrupt(client, args, receipt):
                    if len(client.preset_calls) != 3:
                        return
                    native = receipt["result"]
                    if fault == "value":
                        native["ok"] = copy.deepcopy(value)
                    else:
                        for field in ("ok", "accepted", "released", "verified") if fault == "all_flags_missing" else ("ok",):
                            native.pop(field, None)
                client.lifecycle_hook = corrupt
                runner = self.runner(client)
                with self.assertRaises(UnknownMutation):
                    runner.run_catalog_case(TARGET)
                self.assert_catalog_only_calls(client, ["list", "release", "list"])
                self.assertEqual(client.full_snapshots, 5)
                self.assertEqual(len(self.events("catalog_baseline_observation")), 4)
                self.assertFalse(any(item["phase"] == "StateConfirmed" for item in self.events("transaction_phase")))
                self.assertEqual(Journal.pending(self.journal.ledger_path)["catalog-000"], "UnknownAfterDispatch")
                self.assertFalse(runner.summary["classes"])
                self.assertEqual(runner.counts["catalog_case_verified"], 0)

    def test_catalog_only_refusal_accepts_explicit_false_optional_flags(self):
        client = CatalogClient()
        def explicit_false(client, args, receipt):
            if len(client.preset_calls) == 3:
                receipt["result"].update(ok=False, accepted=False, released=False, verified=False)
        client.lifecycle_hook = explicit_false
        row = self.runner(client).run_catalog_case(TARGET)
        self.assertEqual(row["status"], "catalog_case_verified")
        self.assert_catalog_only_calls(client, ["list", "release", "list"])
        self.assertEqual(client.full_snapshots, 7)
        self.assertFalse(Journal.pending(self.journal.ledger_path))

    def test_catalog_only_accepted_released_list_stops_without_second_release(self):
        client = CatalogClient()
        client.accept_released_catalog = True
        with self.assertRaises(UnknownMutation):
            self.runner(client).run_catalog_case(TARGET)
        self.assert_catalog_only_calls(client, ["list", "release", "list"])
        self.assertEqual(client.full_snapshots, 5)

    def test_catalog_only_context_loss_at_each_lifecycle_stops_without_cleanup(self):
        for phase in (1, 2, 3):
            with self.subTest(phase=phase):
                client = CatalogClient()
                def drift(client, args, receipt):
                    if len(client.preset_calls) == phase:
                        receipt["document"] = "different:document"
                client.lifecycle_hook = drift
                with self.assertRaises(ContextChanged):
                    self.runner(client).run_catalog_case(TARGET)
                self.assert_catalog_only_calls(client, ["list", "release", "list"][:phase])

    def test_catalog_only_discovery_keeps_existing_exact_catalog_validation(self):
        for fault in ("incomplete", "too_many", "duplicate", "bad_next", "url", "instance", "token", "count", "empty_page"):
            with self.subTest(fault=fault):
                client = CatalogClient(count=65)
                client.catalog_fault = fault
                with self.assertRaises((AcceptanceFailure, ContextChanged)):
                    self.runner(client).run_catalog_case(TARGET)
                self.assertTrue(all(call["action"] == "list" for call in client.preset_calls))
                self.assertFalse(client.state_calls)
                self.assertFalse(client.writes)

    def test_catalog_only_preparation_and_nonboolean_readonly_policy_fail_before_catalog(self):
        for setting in ("prepare_midi", "read_only_observations"):
            with self.subTest(setting=setting):
                client = CatalogClient()
                runner = self.runner(client)
                setattr(runner, setting, True if setting == "prepare_midi" else 1)
                with self.assertRaises(AcceptanceFailure):
                    runner.run_catalog_case(TARGET)
                self.assert_catalog_only_calls(client, [])

    def test_catalog_only_cli_reports_finite_inventory_success(self):
        client = CatalogClient(count=0)
        fixture = Path(self.temporary.name) / "catalog-fixtures.json"
        fixture.write_text(json.dumps([TARGET]))
        output = io.StringIO()
        with patch("verify_plugin_presets.McpClient", return_value=client), redirect_stdout(output):
            result = main(["--catalog-only", "--binary", __file__, "--fixtures", str(fixture),
                           "--scratch-title", "MCP Validation Scratch", "--plugin", TARGET["plugin"],
                           "--output-dir", str(Path(self.temporary.name) / "catalog-cli"), "--settle-interval", "0"])
        self.assertEqual(result, 0)
        self.assertIn('"status": "catalog_cases_verified"', output.getvalue())
        self.assert_catalog_only_calls(client, ["list", "release", "list"])

    def test_catalog_only_cli_incompatible_mutation_options_fail_before_owner_start(self):
        for extra in (("--index", "0"), ("--wrong-target-plugin", OTHER["plugin"]), ("--prepare-midi-fixtures",)):
            with self.subTest(extra=extra), patch("verify_plugin_presets.McpClient") as owner:
                with self.assertRaisesRegex(AcceptanceFailure, "Catalog-only mode"):
                    main(["--catalog-only", "--fixtures", "unread-fixtures", "--scratch-title", "scratch",
                          "--plugin", TARGET["plugin"], *extra])
                owner.assert_not_called()

    def test_catalog_only_cli_bounds_selected_plugins_before_owner_start(self):
        fixture = Path(self.temporary.name) / "bounded-fixtures.json"
        fixture.write_text(json.dumps([TARGET]))
        plugins = [argument for index in range(129) for argument in ("--plugin", f"plugin{index}")]
        with patch("verify_plugin_presets.McpClient") as owner:
            with self.assertRaisesRegex(AcceptanceFailure, "1..128 exact"):
                main(["--catalog-only", "--fixtures", str(fixture), "--scratch-title", "scratch", *plugins])
            owner.assert_not_called()

    def test_catalog_only_cli_stops_before_next_plugin_after_typed_corruption(self):
        client = CatalogClient()
        def drift(client, page):
            if client.full_snapshots == 3:
                parameter = page["params"][0]
                parameter["text"] = parameter["native"]["native_description"]["text"] = "drift"
        client.page_hook = drift
        fixture = Path(self.temporary.name) / "two-fixtures.json"
        fixture.write_text(json.dumps([TARGET, OTHER]))
        with patch("verify_plugin_presets.McpClient", return_value=client), redirect_stdout(io.StringIO()):
            with patch("sys.stderr", new=io.StringIO()):
                result = main(["--catalog-only", "--binary", __file__, "--fixtures", str(fixture),
                               "--scratch-title", "MCP Validation Scratch", "--plugin", TARGET["plugin"],
                               "--plugin", OTHER["plugin"], "--output-dir", str(Path(self.temporary.name) / "stopped-cli"),
                               "--settle-interval", "0"])
        self.assertEqual(result, 1)
        self.assert_catalog_only_calls(client, ["list"])
        self.assertFalse(any(args.get("channel") == OTHER["channel"] for name, args in client.calls if name.startswith("plugin_")))

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
