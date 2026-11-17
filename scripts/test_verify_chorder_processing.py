"""Disconnected processing-window contracts. No DAW or native runtime is launched."""
from copy import deepcopy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from test_verify_chorder_learning import ChorderClient
from verify_all_parameters import AcceptanceFailure, ContextChanged, Journal, PRIVATE_ROOT, UnknownMutation
from verify_chorder_learning import ASSIGNED, BIT, MODE, SELECTOR
from verify_chorder_processing import DiagnosticStage, ProcessingRunner, SCRATCH_TITLE, TARGET, TRACK


class ProcessingClient(ChorderClient):
    def __init__(self):
        super().__init__(mode_autoselect=False)
        self.omit_assigned_refresh = True
        self.playing = False
        self.seconds = 0.0
        self.pending = []
        self.processed = set()
        self.transport_log = []
        self.main = [{"label": "메인", "volume": 0.0, "output": None}]
        self.song_fault = self.context_fault = self.inspect_fault = None
        self.tracks_fault = self.metronome_fault = None
        self.mode_autoselect_on_stop = False
        self.positive_model = True
        self.reset_bit = False
        self.immediate_model = False
        self.fail_start = self.fail_stop = False
        self.start_context_change = False
        self.no_progress = False
        self.post_stop_fault = None
        self.transport_fault = self.after_role = None
        self.context_count = 0

    def transport(self):
        return {"playing": self.playing, "recording": False, "loop": False,
                "precount": False, "preroll": False, "position": {"seconds": self.seconds, "display": "position"}}

    def context(self):
        sample = {"context": {"session": self.session, "document": self.document, "open": True, "title": SCRATCH_TITLE},
                  "song": {"title": SCRATCH_TITLE, "transport": {"playing": False, "recording": False}},
                  "target_track": {"exact_match_count": 1, "name": TRACK, "mediaType": "Music",
                                   "eventCount": 0, "events": [], "events_complete": True},
                  "selection": {"complete": True, "tracks": [{"name": TRACK, "mediaType": "Music"}]},
                  "focused_track": {"label": TRACK, "mute": 0, "recordArmed": 0, "monitor": 0}}
        result = {"version": 1, "atomic": False, "scope": "music_fixture_before_after", "target": deepcopy(TARGET),
                  "before": deepcopy(sample), "after": deepcopy(sample)}
        if self.context_fault:
            self.context_fault(result)
        return result

    def call(self, name, args):
        if name == "host_status":
            self.calls.append((name, deepcopy(args)))
            return self.receipt({"title": SCRATCH_TITLE, "open": True,
                                 "document": self.document, "session": self.session})
        if name == "operation_run":
            self.calls.append((name, deepcopy(args)))
            operation = args["operation"]
            if operation == "song":
                if self.playing and not self.no_progress:
                    self.seconds += 0.25
                result = {"title": SCRATCH_TITLE, "trackCount": 1, "selectedTracks": [TRACK], "transport": self.transport()}
                if self.song_fault:
                    self.song_fault(result)
            elif operation == "tracks":
                result = [{"name": TRACK, "mediaType": "Music", "selected": True, "eventCount": 0, "events": []}]
                if self.tracks_fault:
                    self.tracks_fault(result)
            elif operation == "channels":
                result = deepcopy(self.main)
            elif operation == "metronome":
                if args.get("args"):
                    raise AssertionError("The diagnostic must never change metronome settings")
                result = {"click": False, "precount": False, "preroll": False, "precountBars": 1}
                if self.metronome_fault:
                    self.metronome_fault(result)
            else:
                raise AssertionError(operation)
            return self.receipt(result)
        if name == "plugin_inspect":
            self.calls.append((name, deepcopy(args)))
            self.context_count += 1
            result = {"native": {"ok": True, "outcome": "observed", "source": "dsp", "instance": deepcopy(self.instance)},
                      "fixture_context": self.context()}
            if self.inspect_fault:
                self.inspect_fault(result)
            return self.receipt(result)
        if name == "transport_action":
            self.calls.append((name, deepcopy(args)))
            action = args["action"]
            self.transport_log.append(action)
            self.mutation_log.append(action)
            if action == "play":
                self.playing = True
                if self.start_context_change:
                    self.document = "test:changed"
                if self.fail_start:
                    raise TimeoutError("unknown Start after dispatch")
            elif action == "stop":
                self.playing = False
                if self.fail_stop:
                    raise TimeoutError("unknown Stop after dispatch")
                if self.pending == [MODE]:
                    self.processed.add(MODE)
                    if self.mode_autoselect_on_stop:
                        self.update(SELECTOR, 1)
                        self.processed.add(SELECTOR)
                elif self.pending == [SELECTOR]:
                    self.processed.add(SELECTOR)
                elif self.pending == [BIT] and {MODE, SELECTOR} <= self.processed:
                    if self.positive_model:
                        self.update(ASSIGNED, 1)
                    if self.reset_bit:
                        self.update(BIT, 0)
                self.pending.clear()
                if self.post_stop_fault:
                    self.post_stop_fault(self)
            else:
                raise AssertionError(action)
            result = {"action": action, "executed": True, "transport": self.transport()}
            if self.transport_fault:
                self.transport_fault(action, result)
            return self.receipt(result, "applied_verified")
        if name == "plugin_parameter_set":
            self.pending.append(args["param"])
            result = super().call(name, args)
            if args["param"] == BIT and self.immediate_model:
                self.update(ASSIGNED, 1)
            if self.after_role:
                self.after_role(self, args["param"])
            return result
        if name in ("plugin_state", "insert_chain_state", "plugin_presets", "channel_set", "transport_set"):
            raise AssertionError("A forbidden diagnostic mutation was dispatched")
        return super().call(name, args)


class ProcessingTests(unittest.TestCase):
    def setUp(self):
        PRIVATE_ROOT.mkdir(exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(prefix="chorder-processing-test-", dir=PRIVATE_ROOT)
        self.journals = []

    def tearDown(self):
        for journal in self.journals:
            journal.close()
        self.temp.cleanup()

    def runner(self, client):
        journal = Journal(Path(self.temp.name) / f"run{len(self.journals)}")
        self.journals.append(journal)
        return ProcessingRunner(client, journal)

    def run_case(self, client=None):
        client = client or ProcessingClient()
        runner = self.runner(client)
        with patch("verify_chorder_processing.time.sleep"):
            result = runner.run()
        return client, runner, result

    def prime(self, client):
        runner = self.runner(client)
        runner.initialize()
        runner.provider = runner.snapshot(TARGET)
        runner.stage = DiagnosticStage.MODE
        return runner, runner.provider

    def test_three_separated_windows_observe_exact_model_status_without_state_calls(self):
        client, runner, result = self.run_case()
        self.assertEqual(client.mutation_log, [MODE, "play", "stop", SELECTOR, "play", "stop", BIT, "play", "stop"])
        self.assertEqual(result["status"], "model_status_witness_observed")
        self.assertTrue(result["model_status_witness"])
        self.assertEqual(result["final_changed_names"], sorted([BIT, ASSIGNED]))
        self.assertEqual(len(result["transport_windows"]), 3)
        self.assertFalse(result["model_roundtrip_verified"])
        self.assertFalse(result["all_builtin_plugins_verified"])
        self.assertFalse(result["processing_queue_acknowledged"])
        self.assertEqual(result["state_lifecycle_calls"], 0)
        self.assertEqual(result["restoration"], "not_attempted_discard_unsaved_diagnostic")
        self.assertEqual(client.state_log, [])
        self.assertTrue(all(window["known_start_stop"] and window["advanced"] for window in result["transport_windows"]))
        entries = [json.loads(line) for line in runner.journal.ledger_path.read_text().splitlines()]
        self.assertEqual(sum(row["event"] == "run_completed" for row in entries), 1)

    def test_explicit_unsupported_metronome_preroll_uses_stopped_transport_guard(self):
        client = ProcessingClient()
        client.metronome_fault = lambda m: m.update(preroll=None)
        client, runner, result = self.run_case(client)
        self.assertTrue(result["model_status_witness"])
        self.assertEqual(client.mutation_log, [MODE, "play", "stop", SELECTOR, "play", "stop", BIT, "play", "stop"])
        self.assertEqual(result["state_lifecycle_calls"], 0)
        self.assertFalse(result["model_roundtrip_verified"])

    def test_metronome_and_transport_preroll_evidence_must_be_explicit(self):
        faults = (("missing_metronome", lambda m: m.pop("preroll"), None),
                  ("enabled_metronome", lambda m: m.update(preroll=True), None),
                  ("integer_metronome", lambda m: m.update(preroll=0), None),
                  ("float_metronome", lambda m: m.update(preroll=0.0), None),
                  ("string_metronome", lambda m: m.update(preroll="false"), None),
                  ("error_metronome", lambda m: m.update(error={"code": "failed"}), None),
                  ("native_error_metronome", lambda m: m.update(native_error={"code": "failed"}), None),
                  ("errors_metronome", lambda m: m.update(errors=[{"code": "failed"}]), None),
                  ("missing_transport", None, lambda s: s["transport"].pop("preroll")),
                  ("enabled_transport", None, lambda s: s["transport"].update(preroll=True)),
                  ("null_transport", None, lambda s: s["transport"].update(preroll=None)))
        for name, metronome_fault, song_fault in faults:
            with self.subTest(name=name):
                client = ProcessingClient()
                def alter_metronome(m, fault=metronome_fault):
                    m["preroll"] = None
                    if fault:
                        fault(m)
                client.metronome_fault = alter_metronome
                client.song_fault = song_fault
                with self.assertRaises(AcceptanceFailure):
                    self.run_case(client)
                self.assertEqual(client.mutation_log, [])
                self.assertEqual(client.writes, [])
                self.assertEqual(client.transport_log, [])

    def test_observed_auto_selection_skips_selector_setter_and_its_window(self):
        client = ProcessingClient()
        client.mode_autoselect_on_stop = True
        client, runner, result = self.run_case(client)
        self.assertEqual(client.mutation_log, [MODE, "play", "stop", BIT, "play", "stop"])
        self.assertTrue(result["model_status_witness"])

    def test_pre_window_model_witness_needs_no_third_play(self):
        client = ProcessingClient()
        client.immediate_model = True
        client, runner, result = self.run_case(client)
        self.assertEqual(client.transport_log, ["play", "stop", "play", "stop"])
        self.assertTrue(result["model_status_witness"])
        self.assertEqual([row["param"] for row in client.writes], [MODE, SELECTOR, BIT])

    def test_advancing_transport_without_assigned_status_remains_unverified(self):
        client = ProcessingClient()
        client.positive_model = False
        client, runner, result = self.run_case(client)
        self.assertEqual(result["status"], "no_model_status_witness")
        self.assertFalse(result["model_status_witness"])
        self.assertFalse(result["model_roundtrip_verified"])
        self.assertEqual(len(client.transport_log), 6)
        self.assertEqual([row["param"] for row in client.writes], [MODE, SELECTOR, BIT])

    def test_refreshed_bit_is_not_written_again(self):
        client = ProcessingClient()
        client.positive_model = False
        client.reset_bit = True
        client, runner, result = self.run_case(client)
        self.assertEqual(result["status"], "no_model_status_witness")
        self.assertFalse(result["model_status_witness"])
        self.assertEqual(sum(row["param"] == BIT for row in client.writes), 1)

    def test_main_failure_blocks_all_role_and_transport_writes(self):
        bad = ([{"label": "메인", "volume": 1, "output": None}],
               [{"label": "메인", "volume": False, "output": None}],
               [{"label": "메인", "volume": float("nan"), "output": None}],
               [{"label": "메인", "volume": 0}],
               [{"label": "메인", "volume": 0, "output": "device"}],
               [{"label": "메인", "volume": 0, "output": None}] * 2)
        for main in bad:
            with self.subTest(main=main):
                client = ProcessingClient()
                runner, baseline = self.prime(client)
                client.main = main
                with self.assertRaises(AcceptanceFailure):
                    runner.set_role(baseline, MODE, 1)
                self.assertEqual(client.writes, [])
                self.assertEqual(client.transport_log, [])

    def test_complete_empty_context_and_selection_are_required(self):
        faults = [lambda c: c["before"]["target_track"].update(events_complete=False),
                  lambda c: c["before"]["target_track"].update(eventCount=1, events=[{}]),
                  lambda c: c["before"]["selection"].update(complete=False),
                  lambda c: c["before"]["selection"].update(tracks=[]),
                  lambda c: c["before"]["focused_track"].update(mute=1),
                  lambda c: c["before"]["focused_track"].update(recordArmed=1),
                  lambda c: c["before"]["focused_track"].update(monitor=1),
                  lambda c: c["before"]["context"].update(document="other"),
                  lambda c: c["after"]["target_track"].update(events_complete=False)]
        for fault in faults:
            with self.subTest(fault=fault):
                client = ProcessingClient()
                runner, baseline = self.prime(client)
                client.context_fault = fault
                with self.assertRaises(AcceptanceFailure):
                    runner.set_role(baseline, MODE, 1)
                self.assertEqual(client.writes, [])

    def test_hidden_extra_tracks_events_and_metronome_prevent_preparation(self):
        for kind in ("count", "tracks", "events", "selected", "click", "loop", "precount", "position"):
            with self.subTest(kind=kind):
                client = ProcessingClient()
                if kind == "count":
                    client.song_fault = lambda s: s.update(trackCount=2)
                elif kind == "tracks":
                    client.tracks_fault = lambda rows: rows.append({"name": "hidden"})
                elif kind == "events":
                    client.tracks_fault = lambda rows: rows[0].update(eventCount=1, events=[{}])
                elif kind == "selected":
                    client.song_fault = lambda s: s.update(selectedTracks=[])
                elif kind == "click":
                    client.metronome_fault = lambda m: m.update(click=True)
                else:
                    client.song_fault = lambda s, k=kind: s["transport"].update({k: True if k != "position" else None})
                with self.assertRaises(AcceptanceFailure):
                    self.run_case(client)
                self.assertEqual(client.mutation_log, [])

    def test_unavailable_native_iterator_context_blocks_before_first_setter(self):
        client = ProcessingClient()
        client.inspect_fault = lambda result: result.pop("fixture_context")
        with self.assertRaises(AcceptanceFailure):
            self.run_case(client)
        self.assertEqual(client.mutation_log, [])

    def test_unknown_start_sends_at_most_one_fresh_document_safety_stop(self):
        client = ProcessingClient()
        client.fail_start = True
        runner = self.runner(client)
        with patch("verify_chorder_processing.time.sleep"), self.assertRaises(UnknownMutation):
            runner.run()
        self.assertEqual(client.mutation_log, [MODE, "play", "stop"])
        self.assertFalse(runner.summary["model_status_witness"])
        self.assertFalse(runner.summary["manual_stop_required"])

    def test_context_loss_during_start_prevents_automatic_stop_or_plugin_followup(self):
        client = ProcessingClient()
        client.start_context_change = True
        runner = self.runner(client)
        with patch("verify_chorder_processing.time.sleep"), self.assertRaises(ContextChanged):
            runner.run()
        self.assertEqual(client.mutation_log, [MODE, "play"])
        self.assertTrue(runner.summary["manual_stop_required"])

    def test_unknown_stop_is_not_retried(self):
        client = ProcessingClient()
        client.fail_stop = True
        runner = self.runner(client)
        with patch("verify_chorder_processing.time.sleep"), self.assertRaises(UnknownMutation):
            runner.run()
        self.assertEqual(client.mutation_log, [MODE, "play", "stop"])
        self.assertTrue(runner.summary["manual_stop_required"])

    def test_nonadvancing_transport_stops_once_and_no_next_role_is_written(self):
        client = ProcessingClient()
        client.no_progress = True
        with self.assertRaises(AcceptanceFailure):
            self.run_case(client)
        self.assertEqual(client.mutation_log, [MODE, "play", "stop"])

    def test_wrong_peer_status_after_processing_stops_before_selector(self):
        client = ProcessingClient()
        client.post_stop_fault = lambda c: c.update("chorder.pitch2.chordAssigned", 1)
        with self.assertRaises(AcceptanceFailure):
            self.run_case(client)
        self.assertEqual(client.mutation_log, [MODE, "play", "stop"])

    def test_schema_change_after_processing_stops_before_next_setter(self):
        client = ProcessingClient()
        client.post_stop_fault = lambda c: c.find("chorder.transpose").update(tag=1000)
        with self.assertRaises(ContextChanged):
            self.run_case(client)
        self.assertEqual(client.mutation_log, [MODE, "play", "stop"])

    def test_setter_unknown_never_starts_transport_or_writes_another_role(self):
        client = ProcessingClient()
        client.unknown_at = MODE
        with self.assertRaises(UnknownMutation):
            self.run_case(client)
        self.assertEqual(client.mutation_log, [MODE])

    def test_fixed_role_descriptor_range_and_initial_model_are_required(self):
        for role, field, value in ((MODE, "max", 2), (SELECTOR, "max", 127), (ASSIGNED, "value", 1)):
            with self.subTest(role=role, field=field):
                client = ProcessingClient()
                if field == "value":
                    client.update(role, value)
                else:
                    client.find(role)[field] = value
                with self.assertRaises(AcceptanceFailure):
                    self.run_case(client)
                self.assertEqual(client.mutation_log, [])

    def test_idle_gui_and_output_roles_are_required_before_any_setter(self):
        for role in ("chorder.pitch2.guiInput", "chorder.pitch3.playingLive",
                     "chorder.pitch4.outputPlayingScheduled", "chorder.clearAll"):
            with self.subTest(role=role):
                client = ProcessingClient()
                client.update(role, 1)
                with self.assertRaises(AcceptanceFailure):
                    self.run_case(client)
                self.assertEqual(client.mutation_log, [])

    def test_fresh_main_guard_after_role_prevents_play_when_output_changes(self):
        client = ProcessingClient()
        client.after_role = lambda c, role: c.main[0].update(volume=1)
        with self.assertRaises(AcceptanceFailure):
            self.run_case(client)
        self.assertEqual(client.mutation_log, [MODE])

    def test_conflicting_start_evidence_sends_one_stop_then_no_plugin_followup(self):
        client = ProcessingClient()
        client.transport_fault = lambda action, result: result.update(outcome="applied_verified", native_outcome="applied_unverified") if action == "play" else None
        with self.assertRaises(UnknownMutation):
            self.run_case(client)
        self.assertEqual(client.mutation_log, [MODE, "play", "stop"])

    def test_missing_start_execution_evidence_sends_one_stop(self):
        client = ProcessingClient()
        client.transport_fault = lambda action, result: result.pop("executed") if action == "play" else None
        with self.assertRaises(UnknownMutation):
            self.run_case(client)
        self.assertEqual(client.mutation_log, [MODE, "play", "stop"])

    def test_failed_diagnostic_cannot_resume_or_dispatch_another_role(self):
        client = ProcessingClient()
        client.unknown_at = MODE
        runner = self.runner(client)
        with self.assertRaises(UnknownMutation):
            runner.run()
        with self.assertRaises(AcceptanceFailure):
            runner.set_role(runner.provider, SELECTOR, 1)
        with self.assertRaises(AcceptanceFailure):
            runner.run()
        self.assertEqual(client.mutation_log, [MODE])
        self.assertFalse(runner.summary["model_status_witness"])

    def test_contradictory_play_refusal_sends_one_fresh_safety_stop(self):
        class RefusalAfterPlay(ProcessingClient):
            def call(self, name, args):
                response = super().call(name, args)
                if name == "transport_action" and args["action"] == "play":
                    response["structuredContent"]["outcome"] = "not_applied"
                return response

        client = RefusalAfterPlay()
        runner, baseline = self.prime(client)
        changed = runner.set_role(baseline, MODE, 1)
        with patch("verify_chorder_processing.time.sleep"), self.assertRaises(UnknownMutation):
            runner.process_window(changed)
        self.assertEqual(client.mutation_log, [MODE, "play", "stop"])
        self.assertFalse(client.playing)
        self.assertFalse(runner.summary["manual_stop_required"])

    def test_explicit_unexecuted_stopped_play_refusal_sends_no_stop(self):
        class UnexecutedPlay(ProcessingClient):
            def call(self, name, args):
                if name == "transport_action" and args["action"] == "play":
                    self.calls.append((name, deepcopy(args)))
                    self.transport_log.append("play")
                    self.mutation_log.append("play")
                    return self.receipt({"action": "play", "executed": False, "transport": self.transport()},
                                        "not_applied", {"code": "refused"})
                return super().call(name, args)

        client = UnexecutedPlay()
        runner, baseline = self.prime(client)
        changed = runner.set_role(baseline, MODE, 1)
        with self.assertRaises(AcceptanceFailure):
            runner.process_window(changed)
        self.assertEqual(client.mutation_log, [MODE, "play"])
        self.assertFalse(client.playing)


if __name__ == "__main__":
    unittest.main()
