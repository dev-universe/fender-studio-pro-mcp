"""Disconnected Chorder learning transactions. No DAW/native runtime is launched."""
import copy
import hashlib
import json
from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import patch

from test_verify_all_parameter_states import DigestSweepClient
from test_verify_all_parameters import number
from verify_all_parameters import AcceptanceFailure, ContextChanged, Journal, PRIVATE_ROOT, UnknownMutation, encoded, decode_float_bits
from verify_chorder_learning import (
    ACTIONS, ALL_NAMES, ARTIFACT_SHA256, ASSIGNED, BIT, CLASS_UID, HELD, LEARN, MODE, SELECTOR,
    LearningBinding, LearningRunner, SelectorMap,
)


TARGET = {"plugin": "Chorder", "kind": "note_effect", "slot": 0, "track": "FSP Verify NoteFX Chorder"}


def float_number(name, value=0, kind="Toggle", readonly=False, maximum=1):
    parameter = number(name, float(value), kind, readonly=readonly, lower=0.0, upper=float(maximum))
    parameter.update(access_metadata_complete=True, default=0.0)
    parameter["normalized"] = float(value) / maximum if maximum else 0.0
    return parameter


class ChorderClient(DigestSweepClient):
    def __init__(self):
        records = []
        global_names = ["transpose", "loKey", "hiKey", "autofill", "filterOutsideRange", "learnMode",
                        "learnInputPitch", "playLearnChord", "clear", "clearAll", "copy", "paste"]
        for name in global_names:
            kind = "Integer" if name in ("transpose", "loKey", "hiKey", "learnInputPitch") else "Toggle"
            records.append(float_number("chorder." + name, kind=kind, maximum=128 if name == "learnInputPitch" else 1))
        for n in range(1, 129):
            for role in ("guiInput", "learnChord", "playingLive", "playingScheduled", "outputPlayingLive", "outputPlayingScheduled", "chordAssigned"):
                readonly = role not in ("guiInput", "learnChord")
                records.append(float_number(f"chorder.pitch{n}.{role}", kind="Integer" if role == "chordAssigned" else
                                            "Float" if role == "guiInput" else "Toggle", readonly=readonly,
                                            maximum=2 if role == "chordAssigned" else 1))
        super().__init__(records)
        self.live_class = CLASS_UID
        self.instance.update(class_uid=CLASS_UID)
        self.parameters[6]["text"] = " - "
        self.model = False
        self.serialize_model = True
        self.serialize_gui = False
        self.restore_gui = False
        self.serial_changes = True
        self.main_volume = 0
        self.auto_key = 65
        self.omit_assigned_refresh = False
        self.couple_bit = False
        self.format_fault = None
        self.named_fault = None
        self.unknown_at = None
        self.fault_after_dispatch = None
        self.snapshot_fault_after_load = None
        self.lifecycle_count = 0
        self.capture_count = 0
        self.state_log = []
        self.lifecycle_fault = None
        self.stale_after_bit = False
        self.stale_after_bit_used = False
        self.mutation_log = []
        self.typed_load_read_count = 0
        self.post_load_assigned_reads = []

    def find(self, name):
        return next(parameter for parameter in self.parameters if parameter["name"] == name)

    def description(self, parameter):
        result = super().description(parameter)
        result.pop("access_metadata_complete", None)
        return result

    def update(self, name, value):
        parameter = self.find(name)
        parameter.update(float_number(name, value, parameter["native_kind"], parameter["readOnly"], parameter["max"]))
        if name == SELECTOR:
            parameter["text"] = self.label(int(value))

    def label(self, value):
        return " - " if value == 0 else f"Observed key {value - 1}"

    def native(self, value):
        return {"variant_tag": 2, "data": {"kind": "float", "value": float(value),
                "bits_hex": struct.pack(">d", float(value)).hex().upper()}}

    def call(self, name, args):
        if name == "operation_run":
            self.calls.append((name, copy.deepcopy(args)))
            operation = args["operation"]
            if operation == "tracks":
                return self.receipt([{"name": TARGET["track"], "mediaType": "Music", "selected": True,
                                      "eventCount": 0, "events": []}])
            if operation == "channels":
                return self.receipt([{"label": "메인", "volume": self.main_volume, "output": None}])
        if name == "plugin_inspect":
            self.calls.append((name, copy.deepcopy(args)))
            return self.receipt({"focusedTrackState": {"label": TARGET["track"], "mute": 1, "recordArmed": 0, "monitor": 0}})
        if name == "plugin_parameter_format":
            self.calls.append((name, copy.deepcopy(args)))
            result = {"ok": True, "outcome": "observed", "source": "dsp", "param": SELECTOR,
                      "instance": copy.deepcopy(self.instance), "current_unchanged": True,
                      "before": self.description(self.find(SELECTOR)), "after": self.description(self.find(SELECTOR)),
                      "values": [{"requested_value": value, "requested_native_value": self.native(value),
                                  "native_value": self.native(value), "text": self.label(value)} for value in args["values"]]}
            if self.format_fault:
                self.format_fault(result)
            return self.receipt(result)
        if name == "plugin_parameter_set":
            self.calls.append((name, copy.deepcopy(args)))
            self.writes.append(copy.deepcopy(args))
            role = args["param"]
            self.mutation_log.append(role)
            value = decode_float_bits(args["float_bits"]) if "float_bits" in args else args["value"]
            before = self.description(self.find(role))
            self.update(role, value)
            if role == MODE and value:
                self.update(SELECTOR, self.auto_key)
            if role == MODE and not value:
                for learn in LEARN:
                    self.update(learn, 0)
            if role == BIT:
                self.model = True
                if not self.omit_assigned_refresh:
                    self.update(ASSIGNED, 1)
                if self.couple_bit:
                    self.update("chorder.transpose", 1)
            if self.unknown_at == role:
                raise TimeoutError("test unknown after dispatch")
            result = {"source": "dsp", "param": role, "instance": copy.deepcopy(self.instance), "outcome": "applied_verified",
                      "before": before, "after": self.description(self.find(role))}
            if self.fault_after_dispatch:
                self.fault_after_dispatch(self, role, result)
            return self.receipt(result, "applied_verified")
        if name == "plugin_state":
            self.calls.append((name, copy.deepcopy(args)))
            action = args["action"]
            self.state_log.append(action)
            self.mutation_log.append(action)
            self.lifecycle_count += 1
            result = {"ok": True, "outcome": "observed" if action == "capture" else "applied_unverified",
                      "instance": copy.deepcopy(self.instance)}
            if action == "capture":
                self.capture_count += 1
                token = f"{self.capture_count:064X}"
                payload = {}
                if self.serialize_model:
                    payload["model"] = self.model if self.serial_changes else False
                if self.serialize_gui:
                    payload["gui"] = {key: self.find(key)["value"] for key in {MODE, SELECTOR, *LEARN}}
                data = encoded(payload)
                self.tokens[token] = {"model": self.model, "records": copy.deepcopy(self.parameters)}
                result.update(token=token, byte_count=len(data), sha256=hashlib.sha256(data).hexdigest(),
                              serialization="component_alias_IStorable")
            elif action == "release":
                self.tokens.pop(args["token"])
                result["released"] = True
            elif action == "restore":
                self.restore_count += 1
                stored = self.tokens[args["token"]]
                if self.serialize_model:
                    self.model = stored["model"]
                if self.restore_gui:
                    self.parameters = copy.deepcopy(stored["records"])
                self.update(ASSIGNED, int(self.model))
                result["load_accepted"] = True
                if self.unknown_at == "restore":
                    raise TimeoutError("test unknown restore after dispatch")
                if self.snapshot_fault_after_load:
                    self.snapshot_fault_after_load(self)
            else:
                raise AssertionError(action)
            if self.lifecycle_fault:
                self.lifecycle_fault(self, action, result)
            return self.receipt(result, "applied_unverified")
        if name == "plugin_parameters" and self.stale_after_bit and self.model and not self.restore_count:
            if args.get("offset") == 0 and not self.stale_after_bit_used:
                self.stale_after_bit_used = True
                self.update(BIT, 0)
        if name == "plugin_parameters" and self.restore_count:
            if args.get("offset") == 0:
                self.post_load_assigned_reads.append((self.find(ASSIGNED)["value"], len(self.writes)))
        result = super().call(name, args)
        if name == "plugin_parameters" and args.get("names") and self.named_fault:
            self.named_fault(result["structuredContent"]["result"])
        return result


class LearningPilotTests(unittest.TestCase):
    def setUp(self):
        PRIVATE_ROOT.mkdir(exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(prefix="chorder-learning-", dir=PRIVATE_ROOT)
        self.journal = Journal(Path(self.temp.name) / "run")

    def tearDown(self):
        self.journal.close()
        self.temp.cleanup()

    def preflight(self, client=None):
        client = client or ChorderClient()
        runner = LearningRunner(client, self.journal, "MCP Validation Scratch")
        runner.initialize()
        baseline, mapping = runner.preflight(TARGET)
        binding = LearningBinding(1, client.label(1), mapping.sha256,
                                  runner.document, runner.session, baseline.instance["object_uid"])
        return client, runner, baseline, mapping, binding

    def pilot(self, client=None):
        client, runner, baseline, mapping, binding = self.preflight(client)
        result = runner.pilot(baseline, mapping, binding)
        return client, runner, result

    def test_default_preflight_is_read_only_and_formats_all_129_in_chunks(self):
        client, runner, baseline, mapping, binding = self.preflight()
        self.assertEqual(len(baseline.records), 908)
        self.assertEqual(len(mapping.rows), 129)
        self.assertEqual([len(args["values"]) for name, args in client.calls if name == "plugin_parameter_format"], [32, 32, 32, 32, 1])
        self.assertEqual(client.mutation_log, [])

    def test_one_bit_restores_model_before_only_mode_then_selector_cleanup(self):
        client, runner, result = self.pilot()
        self.assertEqual(result["status"], "fixed_learning_bit_roundtrip_verified")
        self.assertFalse(result["full_learning_sweep_verified"])
        self.assertEqual([write["param"] for write in client.writes], [MODE, SELECTOR, BIT, MODE, SELECTOR])
        self.assertEqual(client.restore_count, 1)
        self.assertFalse(client.model)
        self.assertEqual(client.tokens, {})
        self.assertGreaterEqual(len(client.post_load_assigned_reads), 2)
        self.assertEqual(client.post_load_assigned_reads[:2], [(0.0, 3), (0.0, 3)])

    def test_load_that_restores_gui_needs_no_gui_setters(self):
        client = ChorderClient()
        client.restore_gui = True
        client, runner, result = self.pilot(client)
        self.assertEqual([write["param"] for write in client.writes], [MODE, SELECTOR, BIT])
        self.assertEqual(client.restore_count, 1)

    def test_gui_only_serializer_cannot_fake_model_restore_with_changed_hash(self):
        client = ChorderClient()
        client.serialize_model = False
        client.serialize_gui = True
        client.restore_gui = True
        client, runner, baseline, mapping, binding = self.preflight(client)
        with self.assertRaises(AcceptanceFailure):
            runner.pilot(baseline, mapping, binding)
        self.assertTrue(client.model)
        self.assertEqual(client.restore_count, 1)
        self.assertEqual([write["param"] for write in client.writes], [MODE, SELECTOR, BIT])
        self.assertEqual(client.state_log[-1], "restore")

    def test_serialization_unchanged_from_prepared_is_not_proof(self):
        client = ChorderClient()
        client.serial_changes = False
        client, runner, baseline, mapping, binding = self.preflight(client)
        with self.assertRaises(AcceptanceFailure):
            runner.pilot(baseline, mapping, binding)
        self.assertEqual(client.restore_count, 0)
        self.assertEqual([write["param"] for write in client.writes], [MODE, SELECTOR, BIT])

    def test_wrong_or_stale_binding_never_dispatches_capture_or_set(self):
        client, runner, baseline, mapping, binding = self.preflight()
        for wrong in (LearningBinding(2, client.label(2), mapping.sha256, runner.document, runner.session, baseline.instance["object_uid"]),
                      LearningBinding(1, "wrong", mapping.sha256, runner.document, runner.session, baseline.instance["object_uid"]),
                      LearningBinding(1, client.label(1), "0" * 64, runner.document, runner.session, baseline.instance["object_uid"]),
                      LearningBinding(1, client.label(1), mapping.sha256, "other", runner.session, baseline.instance["object_uid"])):
            with self.assertRaises(AcceptanceFailure):
                runner.pilot(baseline, mapping, wrong)
        self.assertEqual(client.mutation_log, [])

    def test_no_assigned_refresh_or_unrelated_drift_stops_before_restore(self):
        for field in ("omit_assigned_refresh", "couple_bit"):
            with self.subTest(field=field):
                self.journal.close()
                self.journal = Journal(Path(self.temp.name) / field)
                client = ChorderClient()
                setattr(client, field, True)
                client, runner, baseline, mapping, binding = self.preflight(client)
                with self.assertRaises(AcceptanceFailure):
                    runner.pilot(baseline, mapping, binding)
                self.assertEqual(client.restore_count, 0)
                self.assertEqual([write["param"] for write in client.writes], [MODE, SELECTOR, BIT])

    def test_unknown_restore_forbids_gui_cleanup_capture_or_release(self):
        client = ChorderClient()
        client.unknown_at = "restore"
        client, runner, baseline, mapping, binding = self.preflight(client)
        with self.assertRaises(UnknownMutation):
            runner.pilot(baseline, mapping, binding)
        self.assertEqual(client.state_log[-1], "restore")
        self.assertEqual([write["param"] for write in client.writes], [MODE, SELECTOR, BIT])
        self.assertEqual(client.restore_count, 1)

    def test_unknown_primary_forbids_restore_and_cleanup(self):
        client = ChorderClient()
        client.unknown_at = BIT
        client, runner, baseline, mapping, binding = self.preflight(client)
        with self.assertRaises(UnknownMutation):
            runner.pilot(baseline, mapping, binding)
        self.assertEqual(client.restore_count, 0)
        self.assertEqual(client.mutation_log[-1], BIT)

    def test_main_guard_rechecked_after_setup_and_primary(self):
        for fail_on in (SELECTOR, BIT):
            with self.subTest(fail_on=fail_on):
                self.journal.close()
                self.journal = Journal(Path(self.temp.name) / fail_on.replace(".", "-"))
                client = ChorderClient()
                client.fault_after_dispatch = lambda c, role, result: setattr(c, "main_volume", 0.5) if role == fail_on else None
                client, runner, baseline, mapping, binding = self.preflight(client)
                with self.assertRaises(AcceptanceFailure):
                    runner.pilot(baseline, mapping, binding)
                self.assertEqual(client.mutation_log[-1], fail_on)
                self.assertEqual(client.restore_count, 0)

    def test_main_guard_rechecked_immediately_before_original_load(self):
        client = ChorderClient()
        def block_after_changed_capsule_release(current, phase, result):
            if phase == "release" and current.state_log.count("release") == 5:
                current.main_volume = 0.5
        client.lifecycle_fault = block_after_changed_capsule_release
        client, runner, baseline, mapping, binding = self.preflight(client)
        with self.assertRaises(AcceptanceFailure):
            runner.pilot(baseline, mapping, binding)
        self.assertEqual(client.restore_count, 0)
        self.assertEqual(client.state_log[-1], "release")
        self.assertEqual([write["param"] for write in client.writes], [MODE, SELECTOR, BIT])

    def test_incomplete_duplicate_and_coerced_formatter_rows_are_read_only_failures(self):
        faults = [lambda result: result["values"].pop(),
                  lambda result: result.pop("before"),
                  lambda result: result["after"].update(native_default={"variant_tag": 2, "data": {"kind": "float", "bits_hex": "3FF0000000000000", "value": 1.0}}),
                  lambda result: result["values"][0].update(native_value={"variant_tag": 2, "data": {"kind": "float", "bits_hex": "3FE0000000000000", "value": 0.5}}),
                  lambda result: [row.update(text="duplicate") for row in result["values"]]]
        for index, fault in enumerate(faults):
            with self.subTest(index=index):
                self.journal.close()
                self.journal = Journal(Path(self.temp.name) / f"format-{index}")
                client = ChorderClient()
                client.format_fault = fault
                with self.assertRaises(AcceptanceFailure):
                    self.preflight(client)
                self.assertEqual(client.mutation_log, [])

    def test_idle_gate_rejects_notes_audition_learning_and_nonempty_target(self):
        for index, role in enumerate(("chorder.pitch1.guiInput", "chorder.pitch2.outputPlayingLive", "chorder.playLearnChord", MODE, ASSIGNED)):
            with self.subTest(role=role):
                self.journal.close()
                self.journal = Journal(Path(self.temp.name) / f"idle-{index}")
                client = ChorderClient()
                client.update(role, 1)
                with self.assertRaises(AcceptanceFailure):
                    self.preflight(client)
                self.assertEqual(client.mutation_log, [])


    def test_lifecycle_unknown_or_malformed_tokens_never_trigger_followup_cleanup(self):
        faults = [lambda c, phase, result: result.update(outcome="unknown") if phase == "capture" else None,
                  lambda c, phase, result: result.update(token="bad") if phase == "capture" else None,
                  lambda c, phase, result: result.update(token="1".zfill(64)) if phase == "capture" else None,
                  lambda c, phase, result: result.update(native_error={"code": "failed"}) if phase == "release" else None]
        for index, fault in enumerate(faults):
            with self.subTest(index=index):
                self.journal.close()
                self.journal = Journal(Path(self.temp.name) / f"lifecycle-{index}")
                client = ChorderClient()
                client.lifecycle_fault = fault
                client, runner, baseline, mapping, binding = self.preflight(client)
                with self.assertRaises(AcceptanceFailure):
                    runner.pilot(baseline, mapping, binding)
                self.assertEqual(client.writes, [])
                self.assertEqual(client.restore_count, 0)
                self.assertEqual(client.state_log[-1], "release" if index == 3 else "capture")

    def test_stale_learning_read_and_postload_unrelated_drift_forbid_gui_repair(self):
        for mode in ("stale", "postload"):
            with self.subTest(mode=mode):
                self.journal.close()
                self.journal = Journal(Path(self.temp.name) / mode)
                client = ChorderClient()
                if mode == "stale":
                    client.stale_after_bit = True
                else:
                    client.snapshot_fault_after_load = lambda c: c.update("chorder.transpose", 1)
                client, runner, baseline, mapping, binding = self.preflight(client)
                with self.assertRaises(AcceptanceFailure):
                    runner.pilot(baseline, mapping, binding)
                self.assertEqual([write["param"] for write in client.writes], [MODE, SELECTOR, BIT])
                self.assertEqual(client.restore_count, 1 if mode == "postload" else 0)

    def test_contradictory_setter_outcomes_or_wrong_parameter_stop_after_dispatch(self):
        for index, fields in enumerate(({"native_outcome": "applied_unverified"}, {"param": "other"}, {"native_error": None, "error": {"code": "failed"}})):
            with self.subTest(index=index):
                self.journal.close()
                self.journal = Journal(Path(self.temp.name) / f"setter-fault-{index}")
                client = ChorderClient()
                client.fault_after_dispatch = lambda c, role, result: result.update(fields) if role == MODE else None
                client, runner, baseline, mapping, binding = self.preflight(client)
                with self.assertRaises(UnknownMutation):
                    runner.pilot(baseline, mapping, binding)
                self.assertEqual(client.restore_count, 0)
                self.assertEqual(client.mutation_log[-1], MODE)

    def test_binding_parser_requires_exact_independent_observation_contract(self):
        value = {"value": 1, "label": "key", "selector_map_sha256": "a" * 64,
                 "document": "doc", "session": "session", "object_uid": "uid", "core_pitch_index": 0,
                 "artifact_sha256": ARTIFACT_SHA256, "evidence": "observed_gui_and_shipped_pitch_index"}
        self.assertEqual(LearningBinding.parse(value).value, 1)
        for key, wrong in (("core_pitch_index", 1), ("value", True), ("evidence", "assumed"), ("artifact_sha256", "a" * 64)):
            with self.subTest(key=key), self.assertRaises(AcceptanceFailure):
                LearningBinding.parse({**value, key: wrong})


class LearningDescriptionBoundaryTests(unittest.TestCase):
    def setUp(self):
        PRIVATE_ROOT.mkdir(exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(prefix="chorder-description-", dir=PRIVATE_ROOT)
        self.journals = []

    def tearDown(self):
        for journal in self.journals:
            journal.close()
        self.temp.cleanup()

    def fixture(self):
        client = ChorderClient()
        journal = Journal(Path(self.temp.name) / f"run-{len(self.journals)}")
        self.journals.append(journal)
        runner = LearningRunner(client, journal, "MCP Validation Scratch")
        runner.initialize()
        return client, runner, runner.snapshot(TARGET)

    def wrapper(self, client, role, description):
        index = next(i for i, parameter in enumerate(client.parameters) if parameter["name"] == role)
        value = client.indexed(client.find(role), index)["native"]
        value["native_description"] = copy.deepcopy(description)
        return value

    def test_contradictory_setter_descriptions_stop_after_one_write(self):
        faults = [
            ("after", lambda d: d.update(normalized=999)),
            ("after", lambda d: d.pop("normalized")),
            ("before", lambda d: d.update(normalized=999)),
            ("before", lambda d: d.pop("normalized")),
            ("after", lambda d: d.update(min=-99)),
            ("before", lambda d: d.update(min=-99)),
            ("after", lambda d: d.update(max=999)),
            ("after", lambda d: d.update(default=999)),
            ("after", lambda d: d.update(value=999)),
            ("after", lambda d: d.update(title=None)),
            ("after", lambda d: d["native_normalized"].update(bits_hex="broken")),
            ("after", lambda d: d["native_value"].update(variant_tag="2")),
        ]
        for index, (field, fault) in enumerate(faults):
            with self.subTest(index=index):
                client, runner, baseline = self.fixture()
                client.fault_after_dispatch = lambda c, role, result: fault(result[field])
                with self.assertRaises((ContextChanged, UnknownMutation)):
                    runner.set_role(baseline, MODE, 1)
                self.assertEqual(client.mutation_log, [MODE])
                self.assertEqual(client.state_log, [])

    def test_contradictory_formatter_descriptions_stop_before_next_page(self):
        faults = [
            ("before", lambda d: d.update(normalized=999)),
            ("before", lambda d: d.pop("normalized")),
            ("after", lambda d: d.update(min=-99)),
            ("after", lambda d: d.update(max=999)),
            ("after", lambda d: d.update(default=999)),
            ("after", lambda d: d.update(title=None)),
        ]
        for index, (field, fault) in enumerate(faults):
            with self.subTest(index=index):
                client, runner, baseline = self.fixture()
                client.format_fault = lambda result: fault(result[field])
                with self.assertRaises((ContextChanged, UnknownMutation)):
                    runner.selector_map(baseline)
                self.assertEqual(sum(name == "plugin_parameter_format" for name, _ in client.calls), 1)
                self.assertEqual(client.mutation_log, [])

    def test_full_pilot_normalized_contradiction_does_not_dispatch_a_second_role_or_load(self):
        client, runner, baseline = self.fixture()
        mapping = runner.selector_map(baseline)
        binding = LearningBinding(1, client.label(1), mapping.sha256,
                                  runner.document, runner.session, baseline.instance["object_uid"])
        client.fault_after_dispatch = lambda c, role, result: result["after"].update(normalized=999) if role == MODE else None
        with self.assertRaises((ContextChanged, UnknownMutation)):
            runner.pilot(baseline, mapping, binding)
        self.assertEqual([write["param"] for write in client.writes], [MODE])
        self.assertEqual(client.restore_count, 0)
        self.assertEqual(client.mutation_log[-1], MODE)

    def test_actual_plain_descriptions_allow_absent_index_and_access_fields(self):
        client, runner, baseline = self.fixture()
        original = next(parameter for parameter in baseline.records if parameter["name"] == MODE)
        fresh = runner.read_parameter(baseline, original)
        self.assertEqual(fresh["native"]["index"], original["native"]["index"])
        self.assertEqual(fresh["native"]["native_description"], client.description(client.find(MODE)))
        runner.set_role(baseline, MODE, 1)
        self.assertEqual(client.mutation_log, [MODE])

    def test_missing_each_plain_field_in_named_read_stops_without_dispatch(self):
        plain_fields = ("default", "display_parser", "enabled", "max", "min", "name", "native_default",
                        "native_kind", "native_max", "native_min", "native_normalized", "native_support",
                        "native_value", "normalized", "readOnly", "support", "tag", "text", "title", "value", "writable")
        for field in plain_fields:
            with self.subTest(field=field):
                client, runner, baseline = self.fixture()
                original = next(parameter for parameter in baseline.records if parameter["name"] == MODE)
                client.named_fault = lambda result: result["params"][0].pop(field)
                with self.assertRaises((ContextChanged, UnknownMutation)):
                    runner.read_parameter(baseline, original)
                self.assertEqual(client.mutation_log, [])

    def test_named_envelope_contradictions_stop_without_dispatch(self):
        faults = [lambda r: r.update(source="controller"),
                  lambda r: r.update(instance={"object_uid": "foreign"}),
                  lambda r: r.update(ok=False), lambda r: r.update(ok=1),
                  lambda r: r.update(outcome="applied_verified"),
                  lambda r: r.update(native_outcome="unknown"),
                  lambda r: r.update(error={}), lambda r: r.update(native_error=""),
                  lambda r: r.update(errors=["failed_read"]), lambda r: r.update(errors=None),
                  lambda r: r.update(plugin="foreign"), lambda r: r.update(slot=1),
                  lambda r: r.update(parameter_discovery={"source": "controller"}),
                  lambda r: r.update(native={"outcome": "unknown"}),
                  lambda r: r.update(missing=[MODE]), lambda r: r["params"].append(copy.deepcopy(r["params"][0]))]
        for index, fault in enumerate(faults):
            with self.subTest(index=index):
                client, runner, baseline = self.fixture()
                original = next(parameter for parameter in baseline.records if parameter["name"] == MODE)
                client.named_fault = fault
                with self.assertRaises((ContextChanged, UnknownMutation)):
                    runner.read_parameter(baseline, original)
                self.assertEqual(client.mutation_log, [])

    def test_present_matching_named_identity_and_observed_status_are_allowed(self):
        client, runner, baseline = self.fixture()
        original = next(parameter for parameter in baseline.records if parameter["name"] == MODE)
        client.named_fault = lambda r: r.update(source="dsp", instance=copy.deepcopy(baseline.instance),
                                               ok=True, outcome="observed", native_outcome="observed", errors=[])
        runner.read_parameter(baseline, original)
        self.assertEqual(client.mutation_log, [])

    def test_matching_optional_wrapper_is_allowed_but_changed_frozen_facts_are_not(self):
        client, runner, baseline = self.fixture()
        def matching_wrapper(current, role, result):
            for field in ("before", "after"):
                result[field]["native"] = self.wrapper(current, role, result[field])
        client.fault_after_dispatch = matching_wrapper
        runner.set_role(baseline, MODE, 1)
        self.assertEqual(client.mutation_log, [MODE])
        faults = [lambda d: d["native"].update(raw_flags=12345),
                  lambda d: d["native"].update(index=999),
                  lambda d: d["native"].update(name="foreign"),
                  lambda d: d["native"]["native_description"].update(normalized=999),
                  lambda d: d.update(index=999),
                  lambda d: d.update(access_metadata_complete=False),
                  lambda d: d.update(readable=False)]
        for index, fault in enumerate(faults):
            with self.subTest(index=index):
                client, runner, baseline = self.fixture()
                def corrupt_wrapper(current, role, result):
                    matching_wrapper(current, role, result)
                    fault(result["after"])
                client.fault_after_dispatch = corrupt_wrapper
                with self.assertRaises((ContextChanged, UnknownMutation)):
                    runner.set_role(baseline, MODE, 1)
                self.assertEqual(client.mutation_log, [MODE])

    def test_missing_setter_description_and_wrong_native_carrier_stop_after_one_write(self):
        faults = [lambda r: r.pop("before"), lambda r: r.pop("after"),
                  lambda r: r["after"]["native_value"].update(variant_tag=1),
                  lambda r: r["after"].update(native_normalized={"kind": "not_applicable"}, normalized=None),
                  lambda r: r.update(ok=False), lambda r: r.update(native_outcome={"bad": "status"})]
        for index, fault in enumerate(faults):
            with self.subTest(index=index):
                client, runner, baseline = self.fixture()
                client.fault_after_dispatch = lambda c, role, result: fault(result)
                with self.assertRaises((ContextChanged, UnknownMutation)):
                    runner.set_role(baseline, MODE, 1)
                self.assertEqual(client.mutation_log, [MODE])


class CandidatePilotTests(unittest.TestCase):
    setUp = LearningPilotTests.setUp
    tearDown = LearningPilotTests.tearDown
    preflight = LearningPilotTests.preflight

    def test_candidate_proves_one_binding_and_restores_with_other_assigned_keys_unchanged(self):
        client = ChorderClient()
        client.update("chorder.pitch2.chordAssigned", 1)
        client.update("chorder.pitch3.chordAssigned", 2)
        client, runner, baseline, mapping, _ = self.preflight(client)
        result = runner.candidate_pilot(baseline, mapping)
        self.assertEqual(result["status"], "fixed_candidate_binding_and_learning_roundtrip_verified")
        self.assertTrue(result["candidate_binding_verified"])
        self.assertEqual(result["candidate_core_pitch_index"], 0)
        self.assertEqual(result["binding_proof"], "two_complete_changed_model_status_observations")
        self.assertFalse(result["external_gui_binding_observed"])
        self.assertFalse(result["full_learning_sweep_verified"])
        self.assertEqual(client.find("chorder.pitch2.chordAssigned")["value"], 1)
        self.assertEqual(client.find("chorder.pitch3.chordAssigned")["value"], 2)
        self.assertEqual([write["param"] for write in client.writes], [MODE, SELECTOR, BIT, MODE, SELECTOR])
        self.assertEqual(client.restore_count, 1)
        self.assertEqual(client.tokens, {})
        self.assertEqual(client.post_load_assigned_reads[:2], [(0.0, 3), (0.0, 3)])

    def test_wrong_candidate_mapping_or_missing_assigned_change_stops_without_load_or_cleanup(self):
        for mode in ("wrong_key", "no_assigned"):
            with self.subTest(mode=mode):
                self.journal.close()
                self.journal = Journal(Path(self.temp.name) / mode)
                client = ChorderClient()
                if mode == "no_assigned":
                    client.omit_assigned_refresh = True
                else:
                    def other_key(current, role, result):
                        if role == BIT:
                            current.update(ASSIGNED, 0)
                            current.update("chorder.pitch2.chordAssigned", 1)
                    client.fault_after_dispatch = other_key
                client, runner, baseline, mapping, _ = self.preflight(client)
                with self.assertRaises(AcceptanceFailure):
                    runner.candidate_pilot(baseline, mapping)
                self.assertFalse(runner.summary["candidate_binding_verified"])
                self.assertEqual(client.restore_count, 0)
                self.assertEqual(client.mutation_log[-1], BIT)

    def test_candidate_proof_stays_pending_during_both_changed_full_observations(self):
        class ObservedClient(ChorderClient):
            def call(self, name, args):
                if name == "plugin_parameters" and args.get("offset") == 0 and self.model and not self.restore_count:
                    self.binding_claims.append(self.runner.summary.get("candidate_binding_verified"))
                return super().call(name, args)
        client = ObservedClient()
        client.binding_claims = []
        client, runner, baseline, mapping, _ = self.preflight(client)
        client.runner = runner
        result = runner.candidate_pilot(baseline, mapping)
        self.assertEqual(client.binding_claims[:2], [False, False])
        self.assertTrue(result["candidate_binding_verified"])

    def test_unstable_second_model_observation_never_confirms_candidate_or_loads(self):
        class UnstableClient(ChorderClient):
            changed_reads = 0
            def call(self, name, args):
                if name == "plugin_parameters" and args.get("offset") == 0 and self.model and not self.restore_count:
                    self.changed_reads += 1
                    if self.changed_reads == 2:
                        self.update(ASSIGNED, 0)
                return super().call(name, args)
        client, runner, baseline, mapping, _ = self.preflight(UnstableClient())
        with self.assertRaises(AcceptanceFailure):
            runner.candidate_pilot(baseline, mapping)
        self.assertFalse(runner.summary["candidate_binding_verified"])
        self.assertEqual(client.restore_count, 0)
        self.assertEqual(client.mutation_log[-1], BIT)

    def test_candidate_serializer_must_restore_model_before_gui_cleanup(self):
        client = ChorderClient()
        client.serialize_model = False
        client.serialize_gui = True
        client, runner, baseline, mapping, _ = self.preflight(client)
        with self.assertRaises(AcceptanceFailure):
            runner.candidate_pilot(baseline, mapping)
        self.assertTrue(runner.summary["candidate_binding_verified"])
        self.assertEqual(client.restore_count, 1)
        self.assertEqual([write["param"] for write in client.writes], [MODE, SELECTOR, BIT])
        self.assertEqual(client.mutation_log[-1], "restore")

    def test_unchanged_changed_serialization_stops_before_release_load_or_cleanup(self):
        client = ChorderClient()
        client.serial_changes = False
        client, runner, baseline, mapping, _ = self.preflight(client)
        with self.assertRaises(AcceptanceFailure):
            runner.candidate_pilot(baseline, mapping)
        self.assertEqual(client.restore_count, 0)
        self.assertEqual(client.mutation_log[-2:], ["capture", "capture"])
        self.assertEqual([write["param"] for write in client.writes], [MODE, SELECTOR, BIT])

    def test_candidate_cli_cannot_accept_an_external_binding_file(self):
        from verify_chorder_learning import main
        with patch("verify_chorder_learning.McpClient") as client:
            with self.assertRaises(SystemExit) as error:
                main(["--fixture", "unread.json", "--scratch-title", "MCP Validation Scratch",
                      "--candidate-pilot", "--binding-file", "unread-binding.json"])
            self.assertEqual(error.exception.code, 2)
            client.assert_not_called()

    def test_candidate_cannot_take_a_different_selector_or_unobserved_map(self):
        client, runner, baseline, mapping, _ = self.preflight()
        rows = list(mapping.rows)
        rows[1] = (2, rows[1][1])
        wrong = SelectorMap(tuple(rows), hashlib.sha256(encoded(rows)).hexdigest())
        for value in (wrong, SelectorMap(mapping.rows, "0" * 64)):
            with self.subTest(value=value.sha256), self.assertRaises(AcceptanceFailure):
                runner.candidate_pilot(baseline, value)
        self.assertEqual(client.mutation_log, [])


if __name__ == "__main__":
    unittest.main()
