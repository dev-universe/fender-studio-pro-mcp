"""Offline fixed focus-binding safety checks. No DAW is launched."""
import copy
from contextlib import redirect_stderr, redirect_stdout
import io
import json
from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import patch

from test_verify_all_parameter_states import DigestSweepClient
from test_verify_all_parameters import number
from verify_all_parameters import (
    AcceptanceFailure, ContextChanged, Journal, PRIVATE_ROOT, RestorationFailure,
    UnknownMutation, candidate, decode_float_bits,
)
from verify_impact_focus import ALIAS, IMPACT_CLASS, PEERS, FOCUS_ROLES, ImpactFocusRunner, main


TARGET = {"kind": "instrument", "plugin": "Impact", "track": "FSP Verify Inst Impact"}


def cutoff_number(name, value):
    parameter = number(name, float(value), lower=0.0, upper=10000.0)
    normalized = struct.unpack(">f", struct.pack(">f", value / 10000.0))[0]
    parameter.update(default=10000.0, normalized=normalized, text=f"cutoff:{normalized:.8g}",
                     display_parser="NativeFormatter { flags: 1 }",
                     support={"raw": True, "normalized": True, "text": True})
    return parameter


class FocusClient(DigestSweepClient):
    def __init__(self, alias=ALIAS):
        self.alias = alias
        self.peers = tuple(f"pad[{index}].{alias.removeprefix('focus.')}" for index in range(128))
        kind = FOCUS_ROLES[alias]
        initial = 10000.0 if kind == "Float" else {"Integer": 48.0, "Toggle": 0.0, "List": 0.0}[kind]
        parameters = [self.focus_parameter(name, initial) for name in (*self.peers, self.alias)]
        parameters.append(number("unrelated", 0.25))
        super().__init__(parameters)
        self.live_class = IMPACT_CLASS
        self.instance["class_uid"] = IMPACT_CLASS
        self.peer = self.peers[37]
        self.fault = None
        self.restoration_fault = None
        self.main_volume = 0
        self.main_matches = 1
        self.focused_mute = 1
        self.full_reads = 0
        self.after_change_full_reads = 0
        self.changed_page_fault = None

    def find(self, name):
        return next(parameter for parameter in self.parameters if parameter["name"] == name)

    def focus_parameter(self, name, desired):
        kind = FOCUS_ROLES[self.alias]
        if kind == "Float":
            return cutoff_number(name, desired)
        upper = {"focus.wave.transpose": 96.0, "focus.wave.tune": 200.0,
                 "focus.filter.active": 1.0, "focus.filter.type": 8.0, "focus.filter.clip": 1.0}[self.alias]
        result = number(name, float(desired), kind=kind, lower=0.0, upper=upper)
        result.update(default=0.0, normalized=desired/upper,
                      support={"raw": True, "normalized": True, "text": True})
        return result

    def update(self, name, desired):
        parameter = self.find(name)
        parameter.update(self.focus_parameter(name, desired) if name == self.alias or name in self.peers else number(name, desired))

    def call(self, name, args):
        if name == "operation_run":
            self.calls.append((name, copy.deepcopy(args)))
            if args["operation"] == "tracks":
                return self.receipt([{"name": TARGET["track"], "mediaType": "Music", "selected": True,
                                      "eventCount": 0, "events": []}])
            if args["operation"] == "channels":
                return self.receipt([{"label": "메인", "volume": self.main_volume, "output": None}
                                     for _ in range(self.main_matches)])
        if name == "plugin_inspect":
            self.calls.append((name, copy.deepcopy(args)))
            result = {"focusedTrackState": {"label": TARGET["track"], "mute": self.focused_mute,
                                            "recordArmed": 0, "monitor": 0}}
            if args.get("include_context"):
                sample = {"context": {"session": self.session, "document": self.document, "open": True,
                                       "title": "MCP Validation Scratch"},
                          "song": {"title": "MCP Validation Scratch", "transport": {"playing": False, "recording": False}},
                          "target_track": {"exact_match_count": 1, "name": TARGET["track"], "mediaType": "Music",
                                           "eventCount": 0, "events": [], "events_complete": True},
                          "selection": {"complete": True, "tracks": [{"name": TARGET["track"], "mediaType": "Music"}]},
                          "focused_track": result["focusedTrackState"]}
                result.update(native={"ok": True, "source": "dsp", "instance": dict(self.instance), "outcome": "observed"},
                              fixture_context={"version": 1, "atomic": False, "scope": "music_fixture_before_after",
                                               "target": {**TARGET, "slot": None}, "before": sample,
                                               "after": copy.deepcopy(sample)})
            return self.receipt(result)
        if name == "plugin_parameters" and not args.get("names") and args["offset"] == 0:
            self.full_reads += 1
            if len(self.writes) == 1:
                self.after_change_full_reads += 1
                if self.changed_page_fault:
                    self.changed_page_fault(self)
        if name == "plugin_parameter_set":
            self.calls.append((name, copy.deepcopy(args)))
            self.writes.append(copy.deepcopy(args))
            assert args["param"] == self.alias
            assert args["source"] == "dsp" and args["expected_instance"] == self.instance["object_uid"]
            desired = decode_float_bits(args["float_bits"])
            restoring = len(self.writes) == 2
            fault = self.restoration_fault if restoring else self.fault
            if fault == "before_drift":
                self.update(self.alias, 8700.0)
                self.update(self.peer, 8700.0)
            before = self.description(self.find(self.alias))
            if fault not in ("ignore", "refuse"):
                self.update(self.alias, desired)
                if self.fault != "no_peer":
                    if FOCUS_ROLES[self.alias] == "Float":
                        normalized = struct.unpack(">f", struct.pack(">f", desired / 10000.0))[0]
                        peer_value = normalized * 10000.0
                    else:
                        peer_value = desired
                    self.update(self.peer, peer_value)
                if self.fault == "two_peers":
                    self.update(self.peers[38], desired)
                if self.fault == "peer_other_bits" and not restoring:
                    self.update(self.peer, desired + 0.01)
                if self.fault == "unrelated" and not restoring:
                    self.update("unrelated", 0.9)
            if fault == "unknown":
                raise TimeoutError("Interrupted after dispatch")
            if fault == "document":
                self.document = "changed-document"
            if fault == "public_unknown":
                return self.receipt({}, "outcome_unknown")
            if fault == "refuse":
                return self.receipt({"native_outcome": "not_applied"}, "not_applied", {"code": "native_rejected"})
            after = self.description(self.find(self.alias))
            result = {"before": before, "after": after, "source": "dsp", "instance": dict(self.instance)}
            if fault == "missing_before":
                result.pop("before")
            if fault == "bad_before":
                result["before"]["native_value"]["data"]["bits_hex"] = "broken"
            if fault == "wrong_instance":
                result["instance"]["object_uid"] = "other-instance"
            if fault == "contradictory":
                result["native_outcome"] = "not_applied"
            if fault == "schema":
                self.find(self.alias)["tag"] += 1
                result["after"]["tag"] += 1
            if fault == "lie" and restoring:
                self.update(self.alias, 0.123)
                self.update(self.peer, 0.123)
            if fault == "main_nonzero" and not restoring:
                self.main_volume = 0.5
            return self.receipt(result, "applied_verified")
        receipt = super().call(name, args)
        if name == "plugin_parameters" and args.get("names"):
            receipt["structuredContent"]["result"]["plugin"] = "Impact"
        return receipt


class FocusPilotTests(unittest.TestCase):
    def setUp(self):
        PRIVATE_ROOT.mkdir(exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(prefix="focus-pilot-unit-", dir=PRIVATE_ROOT)
        self.journal = Journal(Path(self.temporary.name) / "run")
        self.count_patch = patch("verify_impact_focus.PROVIDER_COUNT", 130)
        self.count_patch.start()

    def tearDown(self):
        self.count_patch.stop()
        if not self.journal.stream.closed:
            self.journal.close()
        self.temporary.cleanup()

    def runner(self, client, context=False):
        runner = ImpactFocusRunner(client, self.journal, "MCP Validation Scratch", context)
        runner.initialize()
        return runner

    def events(self):
        return [json.loads(line) for line in self.journal.ledger_path.read_text().splitlines()]

    def test_each_of_33_finite_roles_uses_only_own_alias_and_proves_own_peer_restoration(self):
        self.journal.close()
        self.assertEqual(len(FOCUS_ROLES), 33)
        for index, alias in enumerate(FOCUS_ROLES):
            with self.subTest(alias=alias):
                self.journal = Journal(Path(self.temporary.name) / f"role-{index}")
                client = FocusClient(alias)
                original = copy.deepcopy(client.parameters)
                runner = ImpactFocusRunner(client, self.journal, "MCP Validation Scratch", alias=alias)
                runner.initialize()
                result = runner.run_fixture(TARGET)
                self.assertEqual(result["peer"], client.peers[37])
                self.assertEqual([write["param"] for write in client.writes], [alias, alias])
                self.assertEqual(client.parameters, original)
                self.assertTrue(result["binding_verified"])
                self.assertEqual(runner.finish(), 0)
                self.assertFalse(runner.summary["all_focus_aliases_verified"])
                self.assertFalse(runner.summary["all_focus_selections_verified"])
                self.journal.close()

    def test_foreign_role_is_rejected_before_any_rpc(self):
        for alias in (None, False, 1, "pad[0].filter.cutoff", "focus.unknown", "focus.filter.cutoff.extra"):
            with self.subTest(alias=alias):
                client = FocusClient()
                with self.assertRaises(AcceptanceFailure):
                    ImpactFocusRunner(client, self.journal, "MCP Validation Scratch", alias=alias)
                self.assertEqual(client.calls, [])
                self.assertEqual(client.writes, [])

    def test_fixed_role_kind_mismatch_stops_before_setter(self):
        client = FocusClient("focus.filter.drive")
        client.find(client.alias)["native_kind"] = "Toggle"
        runner = ImpactFocusRunner(client, self.journal, "MCP Validation Scratch", alias=client.alias)
        runner.initialize()
        with self.assertRaises(AcceptanceFailure):
            runner.run_fixture(TARGET)
        self.assertEqual(client.writes, [])

    def test_finite_nondefault_role_unknown_does_not_restore_or_try_another_role(self):
        client = FocusClient("focus.filter.drive")
        client.fault = "unknown"
        runner = ImpactFocusRunner(client, self.journal, "MCP Validation Scratch", alias=client.alias)
        runner.initialize()
        with self.assertRaises(UnknownMutation):
            runner.run_fixture(TARGET)
        self.assertEqual([write["param"] for write in client.writes], [client.alias])
        self.assertFalse(runner.summary["fixtures"][0]["binding_verified"])

    def test_binding_is_discovered_from_two_full_changed_reads_and_all_values_restore_twice(self):
        client = FocusClient()
        runner = self.runner(client)
        result = runner.run_fixture(TARGET)
        self.assertTrue(result["binding_verified"])
        self.assertEqual(result["peer"], PEERS[37])
        self.assertEqual(result["pad_index"], 37)
        self.assertEqual([item["param"] for item in client.writes], [ALIAS, ALIAS])
        self.assertEqual(client.writes[0]["float_bits"], "40B3880000000000")
        self.assertEqual(client.writes[1]["float_bits"], "40C3880000000000")
        self.assertEqual(runner.finish(), 0)
        self.assertEqual(Journal.pending(self.journal.ledger_path), {})
        observations = [entry for entry in self.events() if entry["event"] == "focus_complete_projection"]
        self.assertEqual([entry["checkpoint"] for entry in observations],
                         ["original", "original", "changed", "changed", "before_restore", "restored", "restored"])
        self.assertEqual(len([entry for entry in self.events() if entry["event"] == "zero_main_output_observed"]), 2)
        self.assertFalse(runner.summary["all_focus_aliases_verified"])
        self.assertFalse(runner.summary["all_focus_selections_verified"])

    def test_old_decimal_fraction_fails_exact_raw_binding_despite_equal_normalized_and_display(self):
        client = FocusClient()
        changed = []

        def observe(instance):
            changed.append([instance.description(instance.find(name)) for name in (ALIAS, instance.peer)])

        client.changed_page_fault = observe
        with patch("verify_impact_focus.cutoff_candidate", candidate, create=True):
            runner = self.runner(client)
            with self.assertRaisesRegex(AcceptanceFailure, "exact changed native carrier and bits"):
                runner.run_fixture(TARGET)
        alias, peer = changed[0]
        self.assertEqual(alias["value"], 3700.0)
        self.assertEqual(peer["value"], 3700.000047683716)
        self.assertNotEqual(alias["native_value"]["data"]["bits_hex"], peer["native_value"]["data"]["bits_hex"])
        self.assertEqual(alias["native_normalized"], {"kind": "float32", "bits_hex": "3EBD70A4"})
        self.assertEqual(alias["native_normalized"], peer["native_normalized"])
        self.assertEqual(alias["text"], peer["text"])
        self.assertEqual(len(client.writes), 2)
        self.assertEqual(runner.summary["fixtures"][0]["status"], "binding_unconfirmed_restored")
        self.assertFalse(runner.summary["fixtures"][0]["binding_verified"])
        self.assertEqual(Journal.pending(self.journal.ledger_path), {})

    def test_binary_fraction_candidate_uses_half_then_quarter_and_restores_own_original(self):
        for original, desired in ((10000.0, 5000.0), (5000.0, 2500.0), (2500.0, 5000.0)):
            with self.subTest(original=original):
                client = FocusClient()
                for name in (*PEERS, ALIAS):
                    client.update(name, original)
                journal = Journal(Path(self.temporary.name) / f"binary-{original}")
                runner = ImpactFocusRunner(client, journal, "MCP Validation Scratch")
                runner.initialize()
                try:
                    result = runner.run_fixture(TARGET)
                    self.assertTrue(result["binding_verified"])
                    self.assertEqual([decode_float_bits(write["float_bits"]) for write in client.writes],
                                     [desired, original])
                    self.assertEqual(Journal.pending(journal.ledger_path), {})
                finally:
                    journal.close()

    def test_cutoff_profile_rejects_unobserved_bounds_carrier_and_normalized_support_before_setter(self):
        faults = {
            "unit_range": lambda p: p.update(min=0.0, max=1.0),
            "reversed_range": lambda p: p.update(min=10000.0, max=0.0),
            "missing_native_min": lambda p: p.update(native_min=None),
            "nonfinite_native_max": lambda p: p.update(native_max={"variant_tag": 2, "data": {
                "kind": "float", "value": 10000.0, "bits_hex": "7FF0000000000000"}}),
            "wrong_native_min_carrier": lambda p: p.update(native_min={"variant_tag": 1, "data": {
                "kind": "integer", "value": 0}}),
            "wrong_float_carrier": lambda p: p["native_value"].update(variant_tag=18),
            "no_normalized_support": lambda p: p["support"].update(normalized=False),
            "no_native_normalized_support": lambda p: p.update(native_support={
                "raw_number": True, "normalized": False, "display_text": True}),
            "not_applicable_normalized": lambda p: p.update(normalized=None, native_normalized={
                "kind": "not_applicable"}),
        }
        for label, corrupt in faults.items():
            with self.subTest(fault=label):
                client = FocusClient()
                corrupt(client.find(ALIAS))
                journal = Journal(Path(self.temporary.name) / label)
                runner = ImpactFocusRunner(client, journal, "MCP Validation Scratch")
                runner.initialize()
                try:
                    with self.assertRaises((AcceptanceFailure, ContextChanged)):
                        runner.run_fixture(TARGET)
                    self.assertEqual(client.writes, [])
                finally:
                    journal.close()

    def test_fast_context_mode_is_explicit_and_refreshes_the_frozen_provider(self):
        client = FocusClient()
        runner = self.runner(client, context=True)
        runner.run_fixture(TARGET)
        calls = [args for name, args in client.calls if name == "plugin_inspect" and args.get("include_context")]
        self.assertGreater(len(calls), 2)
        self.assertTrue(all(args["source"] == "dsp" and args["expected_instance"] == client.instance["object_uid"] for args in calls))

    def test_real_named_envelope_and_plain_descriptions_need_no_native_wrapper(self):
        client = FocusClient()
        original_call = client.call

        def actual_shape(name, args):
            receipt = original_call(name, args)
            body = receipt["structuredContent"]["result"]
            if name == "plugin_parameters" and args.get("names"):
                body["plugin"] = "Impact"
                self.assertEqual(set(body), {"plugin", "params", "missing"})
                descriptions = body["params"]
            elif name == "plugin_parameter_set":
                descriptions = [body["before"], body["after"]]
            else:
                descriptions = []
            for description in descriptions:
                self.assertFalse({"native", "access_metadata_complete", "index"}.intersection(description))
                self.assertTrue({"native_value", "native_normalized", "native_min", "native_max",
                                 "native_default", "value", "normalized", "text"}.issubset(description))
            return receipt

        client.call = actual_shape
        self.runner(client).run_fixture(TARGET)
        self.assertEqual(len(client.writes), 2)

    def test_present_frozen_index_metadata_and_consistent_native_envelope_are_validated(self):
        client = FocusClient()
        frozen = client.indexed(client.find(ALIAS), len(PEERS))
        original_call = client.call

        def with_metadata(name, args):
            receipt = original_call(name, args)
            body = receipt["structuredContent"]["result"]
            if name == "plugin_parameters" and args.get("names"):
                body.update(source="dsp", instance=dict(client.instance), ok=True, outcome="observed",
                            native_outcome="observed", errors=[], error=None, native_error=None)
                descriptions = body["params"]
            elif name == "plugin_parameter_set":
                descriptions = [body["before"], body["after"]]
            else:
                descriptions = []
            for description in descriptions:
                description.update(native=copy.deepcopy(frozen["native"]), access_metadata_complete=True,
                                   index=frozen["native"]["index"])
            return receipt

        client.call = with_metadata
        self.runner(client).run_fixture(TARGET)
        self.assertEqual(len(client.writes), 2)

    def test_equal_baselines_do_not_guess_the_first_pad(self):
        client = FocusClient()
        client.peer = PEERS[127]
        result = self.runner(client).run_fixture(TARGET)
        self.assertEqual(result["pad_index"], 127)

    def test_no_peer_and_two_peers_are_failures_even_after_bounded_original_restore(self):
        for fault in ("no_peer", "two_peers"):
            with self.subTest(fault=fault):
                client = FocusClient()
                client.fault = fault
                journal = Journal(Path(self.temporary.name) / fault)
                runner = ImpactFocusRunner(client, journal, "MCP Validation Scratch")
                runner.initialize()
                try:
                    with self.assertRaises(AcceptanceFailure):
                        runner.run_fixture(TARGET)
                    self.assertEqual(len(client.writes), 2)
                    self.assertEqual(runner.summary["fixtures"][0]["status"], "binding_unconfirmed_restored")
                    self.assertFalse(runner.summary["fixtures"][0]["binding_verified"])
                    self.assertEqual(Journal.pending(journal.ledger_path), {})
                finally:
                    journal.close()

    def test_peer_must_have_the_same_exact_changed_native_bits(self):
        client = FocusClient()
        client.fault = "peer_other_bits"
        with self.assertRaises(AcceptanceFailure):
            self.runner(client).run_fixture(TARGET)
        self.assertEqual(len(client.writes), 2)

    def test_alias_and_observed_peer_must_share_their_own_original_bits(self):
        client = FocusClient()
        client.update(client.peer, 5000.0)
        with self.assertRaises(RestorationFailure):
            self.runner(client).run_fixture(TARGET)
        self.assertEqual(len(client.writes), 2)
        self.assertEqual(client.find(client.peer)["value"], 10000.0)
        self.assertTrue(Journal.pending(self.journal.ledger_path))

    def test_drift_between_full_changed_reads_is_never_a_pass(self):
        client = FocusClient()
        def drift(instance):
            if instance.after_change_full_reads == 2:
                instance.update(instance.peer, instance.find(instance.peer)["value"] + 0.02)
        client.changed_page_fault = drift
        with self.assertRaises(AcceptanceFailure):
            self.runner(client).run_fixture(TARGET)
        self.assertEqual(len(client.writes), 2)

    def test_unrelated_value_is_not_excluded_and_failed_restore_stops(self):
        client = FocusClient()
        client.fault = "unrelated"
        with self.assertRaises(RestorationFailure):
            self.runner(client).run_fixture(TARGET)
        self.assertEqual(len(client.writes), 2)
        self.assertTrue(Journal.pending(self.journal.ledger_path))

    def test_unknown_change_transport_or_public_status_forbids_any_restore(self):
        for fault in ("unknown", "public_unknown", "document", "wrong_instance", "contradictory", "missing_before", "bad_before"):
            with self.subTest(fault=fault):
                client = FocusClient()
                client.fault = fault
                journal = Journal(Path(self.temporary.name) / fault)
                runner = ImpactFocusRunner(client, journal, "MCP Validation Scratch")
                runner.initialize()
                try:
                    with self.assertRaises((UnknownMutation, ContextChanged)):
                        runner.run_fixture(TARGET)
                    self.assertEqual(len(client.writes), 1)
                    self.assertTrue(Journal.pending(journal.ledger_path))
                finally:
                    journal.close()

    def test_read_transport_loss_after_known_change_forbids_restore(self):
        client = FocusClient()
        def failure(_instance):
            raise TimeoutError("Read interrupted")
        client.changed_page_fault = failure
        with self.assertRaises(UnknownMutation):
            self.runner(client).run_fixture(TARGET)
        self.assertEqual(len(client.writes), 1)

    def boundary_failure(self, stage, corrupt, *, before_write=False):
        client = FocusClient()
        original_call = client.call

        def interrupted(name, args):
            receipt = original_call(name, args)
            body = receipt["structuredContent"]["result"]
            if stage == "named" and name == "plugin_parameters" and args.get("names"):
                corrupt(body)
            elif stage in ("before", "after") and name == "plugin_parameter_set" and len(client.writes) == 1:
                corrupt(body)
            return receipt

        client.call = interrupted
        journal = Journal(Path(self.temporary.name) / ("boundary-" + str(len(list(Path(self.temporary.name).iterdir())))))
        runner = ImpactFocusRunner(client, journal, "MCP Validation Scratch")
        runner.initialize()
        try:
            with self.assertRaises((UnknownMutation, ContextChanged)):
                if before_write:
                    baseline = runner.snapshot(TARGET)
                    runner.read_parameter(baseline, next(record for record in baseline.records if record["name"] == ALIAS))
                else:
                    runner.run_fixture(TARGET)
            self.assertEqual(len(client.writes), 0 if before_write else 1)
            if not before_write:
                self.assertTrue(Journal.pending(journal.ledger_path))
        finally:
            journal.close()

    def test_malformed_named_values_forbid_any_follow_on_setter(self):
        faults = {
            "raw_bits": lambda p: p["native_value"]["data"].update(bits_hex="broken"),
            "normalized_bits": lambda p: p["native_normalized"].update(bits_hex="broken"),
            "display": lambda p: p.update(text=None),
            "variant_string": lambda p: p["native_value"].update(variant_tag="2"),
            "variant_other_kind": lambda p: p["native_value"].update(variant_tag=1),
            "raw_nonfinite": lambda p: p["native_value"]["data"].update(bits_hex="7FF0000000000000"),
            "raw_shadow": lambda p: p.update(value=0.123),
            "normalized_shadow": lambda p: p.update(normalized=0.123),
            "native_min": lambda p: p["native_min"]["data"].update(bits_hex="broken"),
            "normalized_kind": lambda p: p.update(native_normalized={"kind": "not_applicable"}, normalized=None),
            "incomplete_access": lambda p: p.update(access_metadata_complete=False),
            "foreign_index": lambda p: p.update(index=0),
            "foreign_wrapper": lambda p: p.update(native={"index": 0}),
            "missing_text": lambda p: p.pop("text"),
        }
        for label, corrupt in faults.items():
            for before_write in (False, True):
                with self.subTest(fault=label, before_write=before_write):
                    self.boundary_failure("named", lambda body: corrupt(body["params"][0]), before_write=before_write)

    def test_named_envelope_contradictions_forbid_any_follow_on_setter(self):
        faults = ({"instance": {"object_uid": "foreign"}}, {"source": "controller"},
                  {"ok": False, "errors": ["failed_read"]}, {"outcome": "unknown"},
                  {"native_outcome": None}, {"errors": {}}, {"plugin": "Other"})
        for fields in faults:
            with self.subTest(fields=fields):
                self.boundary_failure("named", lambda body: body.update(fields))
        self.boundary_failure("named", lambda body: body.pop("plugin"))

    def test_applied_setter_error_evidence_forbids_any_follow_on_setter(self):
        for fields in ({"ok": False}, {"errors": ["failed_write"]}, {"errors": {}}, {"param": "other"}):
            with self.subTest(fields=fields):
                self.boundary_failure("after", lambda body: body.update(fields))

    def test_malformed_setter_before_and_after_forbid_any_follow_on_setter(self):
        faults = {
            "raw_bits": lambda p: p["native_value"]["data"].update(bits_hex="broken"),
            "normalized_bits": lambda p: p["native_normalized"].update(bits_hex="broken"),
            "display": lambda p: p.update(text=None),
            "variant_string": lambda p: p["native_value"].update(variant_tag="2"),
            "variant_other_kind": lambda p: p["native_value"].update(variant_tag=1),
            "raw_nonfinite": lambda p: p["native_value"]["data"].update(bits_hex="7FF0000000000000"),
            "normalized_shadow": lambda p: p.update(normalized=0.123),
        }
        for stage in ("before", "after"):
            for label, corrupt in faults.items():
                with self.subTest(stage=stage, fault=label):
                    self.boundary_failure(stage, lambda body: corrupt(body[stage]))
            with self.subTest(stage=stage, fault="missing"):
                self.boundary_failure(stage, lambda body: body.pop(stage))

    def test_schema_drift_forbids_restoration(self):
        client = FocusClient()
        client.fault = "schema"
        with self.assertRaises(ContextChanged):
            self.runner(client).run_fixture(TARGET)
        self.assertEqual(len(client.writes), 1)

    def test_alias_drift_during_change_guard_is_not_attributed_to_the_frozen_baseline(self):
        client = FocusClient()
        client.fault = "before_drift"
        with self.assertRaises(ContextChanged):
            self.runner(client).run_fixture(TARGET)
        self.assertEqual(len(client.writes), 1)

    def test_alias_drift_during_restore_guard_never_earns_restoration_proof(self):
        client = FocusClient()
        client.restoration_fault = "before_drift"
        with self.assertRaises(ContextChanged):
            self.runner(client).run_fixture(TARGET)
        self.assertEqual(len(client.writes), 2)
        self.assertTrue(Journal.pending(self.journal.ledger_path))

    def test_dsp_raw_flags_are_schema_and_cannot_be_excluded(self):
        client = FocusClient()
        def drift(instance, page):
            if len(instance.writes) == 1 and page["offset"] == 0:
                page["params"][0]["native"]["raw_flags"] += 1
        client.page_hook = drift
        with self.assertRaises(ContextChanged):
            self.runner(client).run_fixture(TARGET)
        self.assertEqual(len(client.writes), 1)

    def test_incomplete_changed_page_forbids_restoration(self):
        client = FocusClient()
        def incomplete(instance, page):
            if len(instance.writes) == 1:
                page["page_complete"] = False
        client.page_hook = incomplete
        with self.assertRaises(ContextChanged):
            self.runner(client).run_fixture(TARGET)
        self.assertEqual(len(client.writes), 1)

    def test_main_output_must_be_zero_before_change_and_restore(self):
        client = FocusClient()
        client.main_volume = 0.5
        with self.assertRaises(ContextChanged):
            self.runner(client).run_fixture(TARGET)
        self.assertEqual(len(client.writes), 0)

    def test_main_guard_loss_after_change_forbids_restore(self):
        client = FocusClient()
        client.fault = "main_nonzero"
        with self.assertRaises(ContextChanged):
            self.runner(client).run_fixture(TARGET)
        self.assertEqual(len(client.writes), 1)

    def test_main_identity_must_be_unique(self):
        client = FocusClient()
        client.main_matches = 2
        with self.assertRaises(ContextChanged):
            self.runner(client).run_fixture(TARGET)
        self.assertEqual(len(client.writes), 0)

    def test_main_output_shape_must_be_explicit(self):
        client = FocusClient()
        original_call = client.call
        def missing_output(name, args):
            receipt = original_call(name, args)
            if name == "operation_run" and args["operation"] == "channels":
                receipt["structuredContent"]["result"][0].pop("output")
            return receipt
        client.call = missing_output
        with self.assertRaises(ContextChanged):
            self.runner(client).run_fixture(TARGET)
        self.assertEqual(len(client.writes), 0)

    def test_muted_unarmed_music_precondition_cannot_be_repaired_by_the_pilot(self):
        client = FocusClient()
        client.focused_mute = 0
        with self.assertRaises(ContextChanged):
            self.runner(client).run_fixture(TARGET)
        self.assertEqual(len(client.writes), 0)

    def test_ignored_restore_is_a_pending_failure_with_no_third_write(self):
        client = FocusClient()
        client.restoration_fault = "ignore"
        with self.assertRaises(RestorationFailure):
            self.runner(client).run_fixture(TARGET)
        self.assertEqual(len(client.writes), 2)
        self.assertTrue(Journal.pending(self.journal.ledger_path))

    def test_restore_self_report_does_not_replace_full_independent_proof(self):
        client = FocusClient()
        client.restoration_fault = "lie"
        with self.assertRaises(RestorationFailure):
            self.runner(client).run_fixture(TARGET)
        self.assertEqual(len(client.writes), 2)
        self.assertTrue(any(entry["event"] == "focus_complete_projection" and entry["checkpoint"] == "restored"
                            for entry in self.events()))

    def test_unknown_restore_leaves_pending_and_never_retries(self):
        client = FocusClient()
        client.restoration_fault = "unknown"
        with self.assertRaises(UnknownMutation):
            self.runner(client).run_fixture(TARGET)
        self.assertEqual(len(client.writes), 2)
        self.assertTrue(Journal.pending(self.journal.ledger_path))

    def test_refused_change_requires_two_complete_unchanged_reads_and_no_restore(self):
        client = FocusClient()
        client.fault = "refuse"
        with self.assertRaises(AcceptanceFailure):
            self.runner(client).run_fixture(TARGET)
        self.assertEqual(len(client.writes), 1)
        self.assertEqual(Journal.pending(self.journal.ledger_path), {})

    def test_refused_change_that_changed_state_is_unknown_and_never_restored(self):
        client = FocusClient()
        call = client.call
        def contradict(name, args):
            if name == "plugin_parameter_set":
                receipt = call(name, args)
                receipt["structuredContent"].update(outcome="not_applied", error={"code": "native_rejected"})
                return receipt
            return call(name, args)
        client.call = contradict
        with self.assertRaises(UnknownMutation):
            self.runner(client).run_fixture(TARGET)
        self.assertEqual(len(client.writes), 1)

    def test_fixture_count_class_source_and_access_are_not_optional(self):
        client = FocusClient()
        client.parameters.pop()
        with self.assertRaises(ContextChanged):
            self.runner(client).run_fixture(TARGET)
        self.assertEqual(len(client.writes), 0)

    def test_alias_requires_a_float_native_variant_carrier(self):
        client = FocusClient()
        client.find(ALIAS)["native_value"]["variant_tag"] = 1
        with self.assertRaises(AcceptanceFailure):
            self.runner(client).run_fixture(TARGET)
        self.assertEqual(len(client.writes), 0)

    def test_one_pilot_cannot_run_twice_even_after_success(self):
        client = FocusClient()
        runner = self.runner(client)
        runner.run_fixture(TARGET)
        with self.assertRaises(AcceptanceFailure):
            runner.run_fixture(TARGET)
        self.assertEqual(len(client.writes), 2)

    def test_target_has_no_pad_or_parameter_override(self):
        client = FocusClient()
        with self.assertRaises(AcceptanceFailure):
            self.runner(client).run_fixture({**TARGET, "parameter": "focus.sampleEndOffset"})
        self.assertEqual(len(client.writes), 0)

    def test_no_preparation_command_or_other_parameter_can_be_dispatched(self):
        client = FocusClient()
        runner = self.runner(client)
        for name, arguments in (("operation_run", {"operation": "selectTrack", "args": {}}),
                                ("plugin_parameter_set", {"param": PEERS[0], "value": 0.5})):
            with self.subTest(name=name, arguments=arguments):
                before = len(client.calls)
                with self.assertRaises(AcceptanceFailure):
                    runner.call(name, arguments, mutation=True)
                self.assertEqual(len(client.calls), before)

    def test_missing_explicit_cli_opt_in_never_starts_a_client(self):
        with patch("verify_impact_focus.McpClient") as client, redirect_stderr(io.StringIO()):
            with self.assertRaises(SystemExit):
                main(["--fixtures", "unused", "--scratch-title", "MCP Validation Scratch"])
        client.assert_not_called()

    def test_cli_failure_records_private_error_and_closes_client(self):
        client = FocusClient()
        client.fault = "unknown"
        client.closed = False
        client.close = lambda: setattr(client, "closed", True)
        fixture = Path(self.temporary.name) / "fixtures.json"
        fixture.write_text(json.dumps([TARGET]))
        binary = Path(self.temporary.name) / "binary"
        binary.write_bytes(b"offline")
        output = Path(self.temporary.name) / "cli-run"
        with patch("verify_impact_focus.McpClient", return_value=client), redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
            code = main(["--binary", str(binary), "--fixtures", str(fixture), "--scratch-title", "MCP Validation Scratch",
                         "--run-pilot", "--output-dir", str(output)])
        self.assertEqual(code, 1)
        self.assertTrue(client.closed)
        self.assertEqual(json.loads((output / "progress.json").read_text())["status"], "failed")
        self.assertTrue(Journal.pending(output / "ledger.jsonl"))


if __name__ == "__main__":
    unittest.main()
