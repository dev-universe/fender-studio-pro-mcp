#!/usr/bin/env python3
"""Observe one Chorder learning candidate across separated normal transport windows.

The owner prepares a new, globally empty named song. This diagnostic creates no
tracks, changes no settings and never captures or loads native state. A positive
assigned-chord observation is not a serialization or audible-processing proof.
"""
from enum import Enum
import hashlib
import math
from pathlib import Path
import time

from verify_all_parameters import (
    AcceptanceFailure, ContextChanged, UnknownMutation, encoded, raw_value, require, utc_now,
)
from verify_chorder_learning import (
    ASSIGNED, BIT, LEARN, MODE, SELECTOR, LearningRunner, changed_names, parameters,
    quiet,
)


SCRATCH_TITLE = "MCP Chorder Processing Diagnostic"
TRACK = "FSP Verify NoteFX Chorder Diagnostic"
TARGET = {"plugin": "Chorder", "kind": "note_effect", "slot": 0, "track": TRACK}
ASSIGNED_NAMES = frozenset(f"chorder.pitch{n}.chordAssigned" for n in range(1, 129))


class DiagnosticStage(Enum):
    PREPARATION = "preparation"
    MODE = "mode"
    SELECTOR = "selector"
    BIT = "bit"
    COMPLETE = "complete"
    STOPPED = "stopped"


ROLE_STAGES = {MODE: DiagnosticStage.MODE, SELECTOR: DiagnosticStage.SELECTOR, BIT: DiagnosticStage.BIT}


def position(transport):
    point = transport.get("position") if isinstance(transport, dict) else None
    value = point.get("seconds") if isinstance(point, dict) else None
    require(type(value) in (int, float) and math.isfinite(value) and value >= 0,
            "Transport requires a finite nonnegative position observation")
    return value


class ProcessingRunner(LearningRunner):
    def __init__(self, client, journal):
        super().__init__(client, journal, SCRATCH_TITLE)
        self.stage = DiagnosticStage.PREPARATION
        self.provider = None
        self.role_writes = set()
        self.window_stages = set()
        self.summary["source_hashes"]["verify_chorder_processing.py"] = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
        self.summary.update(mode="separated_chorder_processing_diagnostic", model_status_witness=False,
                            model_roundtrip_verified=False, all_builtin_plugins_verified=False,
                            transport_windows=[], parameter_attempts=[], state_lifecycle_calls=0,
                            processing_queue_acknowledged=False, nominal_window_seconds=0.25,
                            hard_transport_duration_bound=False, manual_stop_required=False,
                            restoration="not_attempted_discard_unsaved_diagnostic")

    def state_call(self, *args, **kwargs):
        raise AcceptanceFailure("This diagnostic has no state lifecycle operation")

    def main_zero(self):
        try:
            channels = self.operation("channels")
        except (ContextChanged, UnknownMutation):
            raise
        except Exception as error:
            raise ContextChanged("Fresh Main observation is unavailable; stop all diagnostic dispatch") from error
        require(isinstance(channels, list) and all(isinstance(row, dict) for row in channels),
                "Main channel observation is unavailable")
        mains = [row for row in channels if row.get("label") == "메인"]
        require(len(mains) == 1 and type(mains[0].get("volume")) in (int, float)
                and math.isfinite(mains[0]["volume"]) and mains[0]["volume"] == 0
                and "output" in mains[0] and mains[0]["output"] is None,
                "Require one numeric-zero Main with an explicit null output observation")
        self.journal.append("diagnostic_main_zero_observed", volume=0, output=None,
                            physical_silence_verified=False)

    def fixture_check(self, target, setup=False):
        require(target == TARGET and type(target.get("slot")) is int and not setup,
                "Root must prepare the exact fresh diagnostic fixture")
        song = self.operation("song")
        require(isinstance(song, dict) and song.get("title") == SCRATCH_TITLE
                and type(song.get("trackCount")) is int and song["trackCount"] == 1
                and song.get("selectedTracks") == [TRACK],
                "The diagnostic requires one total, exclusively selected track")
        transport = song.get("transport")
        require(isinstance(transport, dict) and transport.get("playing") is False
                and transport.get("recording") is False
                and all(transport.get(name) is False for name in ("loop", "precount", "preroll")),
                "The diagnostic requires stopped transport with loop, precount and preroll off")
        position(transport)
        tracks = self.operation("tracks", {"events": True, "maxEvents": 1})
        require(isinstance(tracks, list) and len(tracks) == 1 and isinstance(tracks[0], dict)
                and tracks[0].get("name") == TRACK and tracks[0].get("mediaType") == "Music"
                and tracks[0].get("selected") is True
                and type(tracks[0].get("eventCount")) is int and tracks[0]["eventCount"] == 0
                and tracks[0].get("events") == [], "The complete track inventory must be one empty Music track")
        metronome = self.operation("metronome")
        require(isinstance(metronome, dict)
                and all(metronome.get(name) is False for name in ("click", "precount"))
                and "preroll" in metronome and (metronome["preroll"] is None or metronome["preroll"] is False)
                and all(metronome.get(name) is None for name in ("error", "native_error"))
                and ("errors" not in metronome or metronome["errors"] == []),
                "Metronome click and precount must be off; its preroll must be explicitly unsupported or off")
        if self.provider is None:
            return song
        result = self.call("plugin_inspect", {**target, "source": "dsp", "include_context": True,
                           "expected_instance": self.provider.instance["object_uid"]})["result"]
        require(isinstance(result, dict) and result.get("error") is None and result.get("native_error") is None,
                "Strict diagnostic native inspection is unavailable")
        native = result.get("native")
        require(isinstance(native, dict) and native.get("ok") is True and native.get("error") is None
                and native.get("source") == "dsp" and native.get("instance") == self.provider.instance
                and all(native[key] == "observed" for key in ("outcome", "native_outcome") if key in native),
                "The diagnostic provider changed")
        context = result.get("fixture_context")
        require(isinstance(context, dict)
                and set(context) == {"version", "atomic", "scope", "target", "before", "after"}
                and type(context.get("version")) is int and context["version"] == 1
                and context.get("atomic") is False and context.get("scope") == "music_fixture_before_after"
                and context.get("target") == TARGET and type(context["target"].get("slot")) is int
                and encoded(context.get("before")) == encoded(context.get("after")),
                "The strict bracketed diagnostic context is incomplete or changed")
        for sample in (context["before"], context["after"]):
            require(isinstance(sample, dict)
                    and set(sample) == {"context", "song", "target_track", "selection", "focused_track"}
                    and sample["context"] == {"session": self.session, "document": self.document,
                                              "open": True, "title": SCRATCH_TITLE}
                    and sample["context"].get("open") is True
                    and sample["song"] == {"title": SCRATCH_TITLE,
                                           "transport": {"playing": False, "recording": False}}
                    and all(sample["song"]["transport"].get(key) is False for key in ("playing", "recording")),
                    "The diagnostic document or stopped context changed")
            track = sample["target_track"]
            require(isinstance(track, dict)
                    and set(track) == {"exact_match_count", "name", "mediaType", "eventCount", "events", "events_complete"}
                    and type(track.get("exact_match_count")) is int and track["exact_match_count"] == 1
                    and track.get("name") == TRACK and track.get("mediaType") == "Music"
                    and type(track.get("eventCount")) is int and track["eventCount"] == 0
                    and track.get("events") == [] and track.get("events_complete") is True,
                    "The sole diagnostic track needs a strict empty iterator witness")
            require(sample["selection"] == {"complete": True, "tracks": [{"name": TRACK, "mediaType": "Music"}]}
                    and sample["selection"].get("complete") is True,
                    "The diagnostic selection is incomplete")
            focused = sample["focused_track"]
            require(isinstance(focused, dict) and set(focused) == {"label", "mute", "recordArmed", "monitor"}
                    and focused.get("label") == TRACK
                    and all(type(focused.get(key)) in (bool, int, float) and focused[key] == 0
                            for key in ("mute", "recordArmed", "monitor")),
                    "The diagnostic track must be unmuted, unarmed and unmonitored")
        self.journal.append("diagnostic_empty_context_observed", instance=self.provider.instance,
                            atomic=False, total_tracks=1, iterator_empty=True)
        return song

    def guard(self, expected):
        current = self.observed(expected)
        require(current.projection["all_values_sha256"] == expected.projection["all_values_sha256"],
                "Diagnostic values changed before dispatch")
        self.fixture_check(expected.target)
        self.main_zero()
        return current

    def set_role(self, expected, name, desired):
        require(name in ROLE_STAGES and self.stage is ROLE_STAGES[name]
                and name not in self.role_writes and desired == 1,
                "Each fixed diagnostic role can be written only once to one")
        self.role_writes.add(name)
        self.summary["parameter_attempts"].append(name)
        return super().set_role(expected, name, desired)

    def transport_result(self, receipt, action):
        result = receipt.get("result")
        if (receipt.get("outcome") not in ("applied_verified", "applied_unverified")
                or receipt.get("error") is not None or not isinstance(result, dict)
                or result.get("action") != action or result.get("executed") is not True
                or any(result.get(key) is not None for key in ("error", "native_error"))
                or ("errors" in result and result["errors"] != [])
                or ("ok" in result and result["ok"] is not True)
                or any(result[key] not in ("applied_verified", "applied_unverified")
                       for key in ("outcome", "native_outcome") if key in result)
                or ("outcome" in result and "native_outcome" in result
                    and result["outcome"] != result["native_outcome"])):
            raise UnknownMutation("Transport action lacks a known matching execution receipt")
        transport = result.get("transport")
        require(isinstance(transport, dict) and transport.get("playing") is (action == "play")
                and transport.get("recording") is False,
                "Transport action did not establish its expected nonrecording state")
        position(transport)
        return transport

    def safety_stop(self):
        try:
            receipt = self.call("host_status")
            result = receipt.get("result")
            require(isinstance(result, dict) and result.get("open") is True
                    and result.get("title") == SCRATCH_TITLE and result.get("document") == self.document
                    and result.get("session") == self.session,
                    "Fresh diagnostic Stop document lease is unavailable")
            stopped = self.call("transport_action", {"action": "stop"}, mutation=True, allow_error=True)
            self.transport_result(stopped, "stop")
            self.journal.append("diagnostic_safety_stop_confirmed", no_plugin_followup=True)
        except Exception as error:
            self.summary["manual_stop_required"] = True
            self.journal.append("diagnostic_manual_stop_required", error_type=type(error).__name__,
                                no_plugin_followup=True)

    def process_window(self, expected):
        require(self.stage in (DiagnosticStage.MODE, DiagnosticStage.SELECTOR, DiagnosticStage.BIT)
                and self.stage not in self.window_stages and len(self.window_stages) < 3,
                "Only one normal transport window is allowed per fixed role stage")
        self.guard(expected)
        song = self.fixture_check(expected.target)
        initial = position(song["transport"])
        self.main_zero()
        self.window_stages.add(self.stage)
        observation = {"stage": self.stage.value, "started_at": utc_now(), "nominal_seconds": 0.25}
        self.summary["transport_windows"].append(observation)
        started = time.monotonic()
        may_be_playing, stop_attempted = True, False
        try:
            receipt = self.call("transport_action", {"action": "play"}, mutation=True, allow_error=True)
            if receipt.get("outcome") == "not_applied":
                result = receipt.get("result")
                transport = result.get("transport") if isinstance(result, dict) else None
                point = transport.get("position") if isinstance(transport, dict) else None
                seconds = point.get("seconds") if isinstance(point, dict) else None
                if (isinstance(result, dict) and result.get("action") == "play"
                        and result.get("executed") is False
                        and ("ok" not in result or result["ok"] is False)
                        and all(result[key] == "not_applied" for key in ("outcome", "native_outcome") if key in result)
                        and isinstance(transport, dict) and transport.get("playing") is False
                        and transport.get("recording") is False
                        and type(seconds) in (int, float) and math.isfinite(seconds) and seconds >= 0):
                    may_be_playing = False
                    raise AcceptanceFailure("Play was explicitly unexecuted and stopped; no retry is permitted")
                raise UnknownMutation("Play refusal lacks consistent unexecuted and stopped evidence")
            self.transport_result(receipt, "play")
            time.sleep(0.25)
            middle = self.operation("song")
            require(isinstance(middle, dict) and middle.get("title") == SCRATCH_TITLE
                    and type(middle.get("trackCount")) is int and middle["trackCount"] == 1
                    and middle.get("selectedTracks") == [TRACK],
                    "The playing diagnostic song or sole selected track changed")
            stop_attempted = True
            stopped = self.call("transport_action", {"action": "stop"}, mutation=True, allow_error=True)
            self.transport_result(stopped, "stop")
            may_be_playing = False
            require(isinstance(middle, dict) and middle.get("title") == SCRATCH_TITLE
                    and middle.get("transport", {}).get("playing") is True
                    and middle["transport"].get("recording") is False
                    and position(middle["transport"]) > initial,
                    "Transport position did not establish an advancing nonrecording lifecycle")
            self.fixture_check(expected.target)
            self.main_zero()
            observation.update(known_start_stop=True, advanced=True, completed_at=utc_now(),
                               elapsed_seconds=time.monotonic() - started,
                               target_processing_acknowledged=False)
            self.journal.append("diagnostic_transport_window_observed", **observation)
        except Exception:
            if may_be_playing and not stop_attempted:
                self.safety_stop()
            elif may_be_playing:
                self.summary["manual_stop_required"] = True
            raise
        return self.settled_pair(expected)

    def settled_pair(self, previous):
        samples = [self.observed(previous) for _ in range(2)]
        require(samples[0].projection["all_values_sha256"] == samples[1].projection["all_values_sha256"],
                "The diagnostic requires two identical complete settled native observations")
        return samples[-1]

    def preparation_values(self, snapshot, mapping, require_selected=False):
        records = quiet(snapshot)
        selector = raw_value(records[SELECTOR])
        require(type(selector) in (int, float) and selector == int(selector) and 0 <= selector <= 128
                and records[SELECTOR]["text"] == mapping.rows[int(selector)][1]
                and raw_value(records[MODE]) == 1
                and all(raw_value(records[name]) == 0 for name in LEARN | ASSIGNED_NAMES)
                and (not require_selected or selector == 1),
                "The settled preparation must retain an empty model and a formatter-backed selector")
        return selector

    def run(self, target=None):
        target = TARGET if target is None else target
        try:
            require(self.stage is DiagnosticStage.PREPARATION,
                    "A completed or stopped diagnostic cannot be resumed or replayed")
            require(target == TARGET and type(target.get("slot")) is int,
                    "Only the fixed fresh diagnostic target is supported")
            self.initialize()
            self.fixture_check(target)
            baseline = self.snapshot(target)
            self.provider = baseline
            original = quiet(baseline)
            require(raw_value(original[MODE]) == raw_value(original[SELECTOR]) == 0
                    and all(raw_value(original[name]) == 0 for name in LEARN | ASSIGNED_NAMES),
                    "The diagnostic starts mode-off, sentinel-selected and with all model statuses empty")
            self.exact_pair(baseline)
            self.journal.baseline("chorder-processing-original", baseline.serialized())
            mapping = self.selector_map(baseline)
            self.stage = DiagnosticStage.MODE
            current = self.set_role(baseline, MODE, 1)
            require(changed_names(baseline, current) <= {MODE, SELECTOR},
                    "Mode preparation changed unrelated native values")
            self.preparation_values(current, mapping)
            current = self.process_window(current)
            require(changed_names(baseline, current) <= {MODE, SELECTOR},
                    "The Mode window changed unrelated native values")
            selected = self.preparation_values(current, mapping)
            if selected != 1:
                self.stage = DiagnosticStage.SELECTOR
                before = current
                current = self.set_role(before, SELECTOR, 1)
                require(changed_names(before, current) == {SELECTOR},
                        "Selector preparation changed unrelated native values")
                self.preparation_values(current, mapping, require_selected=True)
                current = self.process_window(current)
                require(changed_names(before, current) == {SELECTOR},
                        "The Selector window changed unrelated native values")
                self.preparation_values(current, mapping, require_selected=True)
            prepared = current
            self.stage = DiagnosticStage.BIT
            current = self.set_role(prepared, BIT, 1)
            require(changed_names(prepared, current) in ({BIT}, {BIT, ASSIGNED}),
                    "The fixed learning bit changed unrelated native values")
            if raw_value(parameters(current)[ASSIGNED]) == 0:
                current = self.process_window(current)
            changed = changed_names(prepared, current)
            records = quiet(current)
            require(changed <= {BIT, ASSIGNED}
                    and raw_value(records[MODE]) == raw_value(records[SELECTOR]) == 1
                    and all(raw_value(records[name]) == 0 for name in ASSIGNED_NAMES - {ASSIGNED})
                    and raw_value(records[ASSIGNED]) in (0, 1),
                    "The final diagnostic changed an unrelated role or model status")
            positive = changed == {BIT, ASSIGNED} and raw_value(records[BIT]) == raw_value(records[ASSIGNED]) == 1
            self.stage = DiagnosticStage.COMPLETE
            self.summary.update(status="model_status_witness_observed" if positive else "no_model_status_witness",
                                model_status_witness=positive, completed_at=utc_now(),
                                final_changed_names=sorted(changed), selector_map_sha256=mapping.sha256,
                                final_schema_sha256=current.schema,
                                final_all_values_sha256=current.projection["all_values_sha256"])
            self.progress()
            self.journal.append("run_completed", summary=self.summary)
            return self.summary
        except Exception as error:
            self.stage = DiagnosticStage.STOPPED
            self.summary.update(status="failed", model_status_witness=False,
                                error_type=type(error).__name__, completed_at=utc_now())
            self.progress()
            self.journal.append("run_failed", error_type=type(error).__name__, no_followup_mutation=True)
            raise
