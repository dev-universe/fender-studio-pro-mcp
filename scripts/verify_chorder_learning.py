#!/usr/bin/env python3
"""Run a fixed Chorder learning pilot in a guarded disposable Music fixture.

The default is a read-only selector preflight. One pilot uses separately observed
binding evidence. An explicit candidate experiment proves its binding from
changed model status before loading the original state. No MIDI events are sent.
Raw labels, values, receipts and capsule metadata remain below .local.
"""
import argparse
from copy import deepcopy
from dataclasses import dataclass
from datetime import datetime, timezone
from enum import Enum
import hashlib
import json
from pathlib import Path
import sys

from mcp_client import McpClient
from parameter_digest import project_snapshot
from verify_all_parameters import (
    AcceptanceFailure, ContextChanged, Journal, RestorationFailure, UnknownMutation,
    encoded, private_path, raw_value, require, utc_now, value_identity,
)
from verify_plugin_models import ModelRunner, exact_integer
from verify_plugin_states import explicit_candidate, hexadecimal


CLASS_UID = "D2D9B002-12FC-4F1D-AFDA-70336D1DC108"
ARTIFACT_SHA256 = "fc3c18837b74ebc01880af6976f323107b9c5aaf3f62576e0827eed2376f6c14"
MODE = "chorder.learnMode"
SELECTOR = "chorder.learnInputPitch"
BIT = "chorder.pitch1.learnChord"
ASSIGNED = "chorder.pitch1.chordAssigned"
LEARN = frozenset(f"chorder.pitch{n}.learnChord" for n in range(1, 129))
HELD = frozenset(f"chorder.pitch{n}.{role}" for n in range(1, 129)
                 for role in ("guiInput", "playingLive", "playingScheduled",
                              "outputPlayingLive", "outputPlayingScheduled"))
ACTIONS = frozenset("chorder." + name for name in
                    ("playLearnChord", "clear", "clearAll", "copy", "paste"))
GLOBALS = ("transpose", "loKey", "hiKey", "autofill", "filterOutsideRange",
           "learnMode", "learnInputPitch", "playLearnChord", "clear", "clearAll", "copy", "paste")
ALL_NAMES = frozenset("chorder." + name for name in GLOBALS) | frozenset(
    f"chorder.pitch{n}.{role}" for n in range(1, 129)
    for role in ("guiInput", "learnChord", "playingLive", "playingScheduled",
                 "outputPlayingLive", "outputPlayingScheduled", "chordAssigned"))
DESCRIPTION_FIELDS = frozenset((
    "default", "display_parser", "enabled", "max", "min", "name", "native_default",
    "native_kind", "native_max", "native_min", "native_normalized", "native_support",
    "native_value", "normalized", "readOnly", "support", "tag", "text", "title", "value", "writable",
))
INDEX_FIELDS = frozenset(("native", "index", "access_metadata_complete", "readable"))


@dataclass(frozen=True)
class SelectorMap:
    rows: tuple
    sha256: str


@dataclass(frozen=True)
class LearningBinding:
    value: int
    label: str
    selector_map_sha256: str
    document: str
    session: str
    object_uid: str

    @classmethod
    def parse(cls, value):
        fields = {"value", "label", "selector_map_sha256", "document", "session", "object_uid",
                  "core_pitch_index", "artifact_sha256", "evidence"}
        require(isinstance(value, dict) and set(value) == fields,
                "Selector binding requires the exact fixed evidence contract")
        require(type(value["value"]) is int and 1 <= value["value"] <= 128
                and type(value["core_pitch_index"]) is int and value["core_pitch_index"] == 0
                and value["artifact_sha256"] == ARTIFACT_SHA256
                and value["evidence"] == "observed_gui_and_shipped_pitch_index",
                "The fixed pitch1 pilot requires independently observed core-index-zero binding")
        require(all(isinstance(value[key], str) and value[key] for key in
                    ("label", "selector_map_sha256", "document", "session", "object_uid")),
                "Selector binding identity is incomplete")
        require(hexadecimal(value["selector_map_sha256"], 64), "Selector map hash is malformed")
        return cls(*(value[key] for key in
                     ("value", "label", "selector_map_sha256", "document", "session", "object_uid")))


class BindingEvidence(Enum):
    EXTERNAL_GUI = "separately_observed_gui_binding"
    CANDIDATE = "unverified_lowest_formatted_candidate"


@dataclass(frozen=True)
class LearningIntent:
    value: int
    label: str
    evidence: BindingEvidence


def value_key(parameter):
    return (value_identity(parameter), encoded(parameter["native_normalized"]), parameter["text"])


def typed_description(snapshot, original, description):
    try:
        require(isinstance(description, dict) and DESCRIPTION_FIELDS <= description.keys()
                and description.keys() <= DESCRIPTION_FIELDS | INDEX_FIELDS,
                "A fresh native description requires every plain producer field")
        require(description["name"] == original["name"] and isinstance(description["title"], str)
                and len(description["title"].encode("utf-16-le")) // 2 <= 4096
                and description["title"] == original["title"],
                "The fresh native description changed its bound name or title")
        plain = {field: deepcopy(description[field]) for field in DESCRIPTION_FIELDS}
        native = deepcopy(original["native"])
        native["native_description"] = plain
        index = native["index"]
        require(type(index) is int and 0 <= index < len(snapshot.records)
                and snapshot.records[index]["name"] == original["name"],
                "The fresh native description has no bound frozen provider index")
        expected_index_fields = {"native": native, "index": index, "access_metadata_complete": True,
                                 "readable": True}
        for field in INDEX_FIELDS & description.keys():
            require(encoded(description[field]) == encoded(expected_index_fields[field]),
                    "Fresh optional indexed metadata contradicts the frozen provider")
        # Plain native replies omit the indexed DSP wrapper. Only its frozen
        # indexing facts are reused; every value and descriptor field is fresh.
        current = {**plain, "native": native, "access_metadata_complete": True}
        records = list(snapshot.records)
        records[index] = current
        projected = project_snapshot({"source": snapshot.source, "instance": snapshot.instance, "records": records})
        require(projected["schema_sha256"] == snapshot.projection["schema_sha256"],
                "The complete typed fresh description changed its frozen schema")
        require(encoded(plain["native_value"]["variant_tag"]) == encoded(original["native_value"]["variant_tag"])
                and plain["native_value"]["data"]["kind"] == original["native_value"]["data"]["kind"]
                and plain["native_normalized"]["kind"] == original["native_normalized"]["kind"],
                "The fresh native value or normalized carrier changed")
        value_identity(current)
        return current
    except (ContextChanged, UnknownMutation):
        raise
    except Exception as error:
        raise ContextChanged("The fresh native description is incomplete or contradictory; stop all dispatch") from error


def parameters(snapshot):
    return {parameter["name"]: parameter for parameter in snapshot.records}


def validate_provider(snapshot):
    require(snapshot.target.get("plugin") == "Chorder" and snapshot.target.get("kind") == "note_effect"
            and snapshot.source == "dsp" and snapshot.instance.get("class_uid", "").upper() == CLASS_UID
            and len(snapshot.records) == 908 and set(parameters(snapshot)) == ALL_NAMES,
            "The pilot requires the exact complete Chorder DSP provider")
    records = parameters(snapshot)
    for name in {MODE, SELECTOR, *LEARN}:
        parameter = records[name]
        require(parameter.get("enabled") is True and parameter.get("writable") is True
                and parameter.get("readOnly") is False and parameter.get("support", {}).get("raw") is True
                and parameter.get("access_metadata_complete") is True,
                "A fixed learning role lacks authoritative enabled raw write access")
        require(parameter["native_kind"] == ("Integer" if name == SELECTOR else "Toggle")
                and raw_value(parameter) == int(raw_value(parameter))
                and parameter["min"] == 0 and parameter["max"] == (128 if name == SELECTOR else 1),
                "A fixed learning role has an unexpected numeric descriptor")
    for n in range(1, 129):
        status = records[f"chorder.pitch{n}.chordAssigned"]
        require(status.get("readOnly") is True and status.get("writable") is False
                and status.get("native_kind") == "Integer" and raw_value(status) in (0, 1, 2),
                "The independent assigned-chord model observation is unavailable")
    require(snapshot.projection["read_only_count"] == 640
            and snapshot.projection["excluded_opaque_count"] == 0,
            "The pilot requires all 640 read-only records and no opaque exclusions")
    return records


def quiet(snapshot):
    records = validate_provider(snapshot)
    require(all(raw_value(records[name]) == 0 for name in HELD | ACTIONS),
            "All GUI note inputs, held/output flags and action controls must be idle")
    return records


def changed_names(before, after):
    require(before.instance == after.instance and before.source == after.source,
            "The learning provider identity changed")
    if before.schema != after.schema:
        raise ContextChanged("The complete typed learning schema changed")
    return {a["name"] for a, b in zip(before.records, after.records) if value_key(a) != value_key(b)}


class LearningRunner(ModelRunner):
    def __init__(self, client, journal, scratch_title):
        super().__init__(client, journal, scratch_title, prepare_midi=False,
                         settle_attempts=2, settle_interval=0, read_only_observations=False)
        self.live_capsules = {}
        self.summary["source_hashes"]["verify_chorder_learning.py"] = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
        self.summary.update(mode="chorder_fixed_learning_pilot", all_learning_controls_verified=False,
                            model_scope="one_originally_empty_input_chord_and_one_output_bit",
                            opaque_contents_verified=False, atomic_snapshot=False)

    def observed(self, baseline):
        self.fixture_check(baseline.target)
        current = self.snapshot(baseline.target, baseline)
        quiet(current)
        return current

    def observed_result(self, receipt, baseline):
        result = receipt.get("result")
        if (receipt.get("outcome") != "observed" or receipt.get("error") is not None
                or not isinstance(result, dict)
                or ("ok" in result and result["ok"] is not True)
                or any(result[key] != "observed" for key in ("outcome", "native_outcome") if key in result)
                or any(result.get(key) is not None for key in ("error", "native_error"))
                or ("errors" in result and result["errors"] != [])
                or ("source" in result and result["source"] != baseline.source)
                or ("instance" in result and encoded(result["instance"]) != encoded(baseline.instance))):
            raise UnknownMutation("The native read has contradictory or unavailable observation evidence")
        return result

    def read_parameter(self, provider, original):
        try:
            receipt = self.call("plugin_parameters", {**provider.target, "source": provider.source,
                                "names": [original["name"]], "expected_instance": provider.instance["object_uid"]})
            result = self.observed_result(receipt, provider)
            require(result.keys() <= {"channel", "slot", "plugin", "params", "missing", "source", "instance",
                                      "ok", "outcome", "native_outcome", "error", "native_error", "errors"}
                    and result.get("missing") == [] and isinstance(result.get("params"), list)
                    and len(result["params"]) == 1
                    and ("plugin" not in result or result["plugin"] == provider.target["plugin"])
                    and all(encoded(result[field]) == encoded(provider.target.get(field))
                            for field in ("channel", "slot") if field in result),
                    "The named native read does not match its exact bound target")
            return typed_description(provider, original, result["params"][0])
        except (ContextChanged, UnknownMutation):
            raise
        except Exception as error:
            raise ContextChanged("The named native read was not established; stop all dispatch") from error

    def exact_pair(self, baseline):
        for _ in range(2):
            current = self.observed(baseline)
            require(current.projection["all_values_sha256"] == baseline.projection["all_values_sha256"],
                    "Two fresh complete typed provider matches are required")
        return current

    def guard(self, expected):
        current = self.observed(expected)
        require(current.projection["all_values_sha256"] == expected.projection["all_values_sha256"],
                "Provider values changed before the next dispatch")
        channels = self.operation("channels")
        mains = [channel for channel in channels if channel.get("label") == "메인"]
        require(len(mains) == 1 and type(mains[0].get("volume")) in (int, float)
                and mains[0]["volume"] == 0 and mains[0].get("output") is None,
                "The actual uniquely resolved Main output volume must be zero")
        return current

    def state_call(self, provider, action, token=None, allow_error=False):
        self.guard(provider)
        receipt = super().state_call(provider, action, token, allow_error)
        result = receipt.get("result")
        allowed = ("observed",) if action == "capture" else ("applied_verified", "applied_unverified")
        outer_allowed = ("observed", "applied_verified", "applied_unverified") if action == "capture" else allowed
        if (receipt.get("outcome") not in outer_allowed or receipt.get("error") is not None
                or not isinstance(result, dict) or result.get("ok") is not True
                or result.get("instance") != provider.instance or result.get("outcome") not in allowed
                or any(result[key] not in allowed for key in ("outcome", "native_outcome") if key in result)
                or any(result.get(key) is not None for key in ("error", "native_error"))
                or ("errors" in result and result["errors"] != [])):
            self.phase("chorder-fixed-pilot", "UnknownAfterDispatch", action=action)
            raise UnknownMutation("State lifecycle evidence is not a known successful bound outcome")
        return receipt

    def capture(self, provider):
        require(len(self.live_capsules) < 3, "The learning pilot permits at most three live capsules")
        capsule = super().capture(provider)
        require(capsule.token not in self.live_capsules, "A fresh capture reused an issued token")
        self.live_capsules[capsule.token] = capsule
        self.phase("chorder-fixed-pilot", "CapsuleMetadataConfirmed", live_capsules=len(self.live_capsules))
        return capsule

    def release(self, provider, capsule):
        require(capsule.token in self.live_capsules, "Release requires one live issued capsule")
        super().release(provider, capsule)
        del self.live_capsules[capsule.token]

    def capsule_pair(self, provider, retained=None, different_from=None):
        first = self.capture(provider)
        second = self.capture(provider)
        require(first.token != second.token and (first.byte_count, first.sha256) ==
                (second.byte_count, second.sha256), "Two independent serialized captures disagree")
        if retained is not None:
            require((first.byte_count, first.sha256) == (retained.byte_count, retained.sha256),
                    "Fresh serialization did not match the immutable original capsule")
        if different_from is not None:
            require((first.byte_count, first.sha256) != (different_from.byte_count, different_from.sha256),
                    "The candidate model edit did not change its prepared serialization")
        self.exact_pair(provider)
        self.release(provider, second)
        if retained is not None:
            self.release(provider, first)
        return first

    def selector_map(self, baseline):
        parameter = parameters(baseline)[SELECTOR]
        rows = []
        for offset in range(0, 129, 32):
            self.exact_pair(baseline)
            requested = list(range(offset, min(offset + 32, 129)))
            receipt = self.call("plugin_parameter_format", {**baseline.target, "param": SELECTOR,
                                "source": "dsp", "expected_instance": baseline.instance["object_uid"],
                                "values": requested})
            result = self.observed_result(receipt, baseline)
            require(isinstance(result, dict) and result.get("ok") is True
                    and result.get("instance") == baseline.instance and result.get("source") == "dsp"
                    and result.get("param") == SELECTOR and result.get("current_unchanged") is True
                    and result.get("error") is None and result.get("native_error") is None
                    and ("errors" not in result or result["errors"] == []),
                    "Native selector formatting did not preserve the observed parameter")
            for field in ("before", "after"):
                description = typed_description(baseline, parameter, result.get(field))
                if value_key(description) != value_key(parameter):
                    raise ContextChanged("Formatter before/after values differ from the complete observed selector")
            values = result.get("values")
            require(isinstance(values, list) and len(values) == len(requested),
                    "The finite selector formatter page is incomplete")
            for desired, row in zip(requested, values):
                require(isinstance(row, dict) and type(row.get("requested_value")) in (int, float)
                        and row["requested_value"] == desired
                        and exact_integer(row.get("requested_native_value")) == desired
                        and exact_integer(row.get("native_value")) == desired
                        and all(row[field].get("variant_tag") == parameter["native_value"]["variant_tag"]
                                for field in ("requested_native_value", "native_value"))
                        and isinstance(row.get("text"), str) and row["text"]
                        and len(row["text"].encode("utf-16-le")) // 2 <= 4096,
                        "Selector formatting changed a requested exact carrier or label")
                rows.append((desired, row["text"]))
            self.exact_pair(baseline)
        require(len({label for _, label in rows}) == 129
                and rows[0][1] == parameter["text"], "Selector sentinel or labels are ambiguous")
        result = SelectorMap(tuple(rows), hashlib.sha256(encoded(rows)).hexdigest())
        self.journal.append("selector_map_observed", rows=rows, sha256=result.sha256,
                            automatic_core_pitch_binding=False)
        return result

    def preflight(self, target):
        require(set(target) == {"plugin", "kind", "slot", "track"} and target["plugin"] == "Chorder"
                and target["kind"] == "note_effect" and type(target["slot"]) is int and target["slot"] >= 0
                and isinstance(target["track"], str) and target["track"].startswith("FSP Verify "),
                "Use one exact named Chorder NoteFX fixture")
        self.fixture_check(target)
        baseline = self.snapshot(target)
        records = quiet(baseline)
        require(raw_value(records[MODE]) == 0 and raw_value(records[SELECTOR]) == 0
                and all(raw_value(records[name]) == 0 for name in LEARN)
                and raw_value(records[ASSIGNED]) == 0,
                "The fixed pilot starts mode-off, sentinel-selected, with an empty pitch1 model chord")
        self.journal.baseline("chorder-original", baseline.serialized())
        self.exact_pair(baseline)
        return baseline, self.selector_map(baseline)

    def set_role(self, expected, name, desired):
        require(name in (MODE, SELECTOR, BIT), "Only three fixed learning roles can be dispatched")
        self.guard(expected)
        before = parameters(expected)[name]
        self.phase("chorder-fixed-pilot", "ParameterInFlight", parameter=name)
        receipt = self.write(expected, before, explicit_candidate(before, desired_value=desired))
        result = receipt.get("result")
        if (receipt.get("outcome") not in ("applied_verified", "applied_unverified")
                or receipt.get("error") is not None or not isinstance(result, dict)
                or result.get("instance") != expected.instance or result.get("source") != "dsp"
                or result.get("param") != name
                or any(result[key] not in ("applied_verified", "applied_unverified")
                       for key in ("outcome", "native_outcome") if key in result)
                or ("outcome" in result and "native_outcome" in result
                    and result["outcome"] != result["native_outcome"])
                or ("ok" in result and result["ok"] is not True)
                or ("plugin" in result and result["plugin"] != expected.target["plugin"])
                or any(result.get(key) is not None for key in ("error", "native_error"))
                or ("errors" in result and result["errors"] != [])):
            raise UnknownMutation("Learning setter did not establish a known applied bound result")
        setter_before = typed_description(expected, before, result.get("before"))
        after = typed_description(expected, before, result.get("after"))
        if value_key(setter_before) != value_key(before):
            raise ContextChanged("Learning setter before-value disagrees with its immediately guarded provider")
        require(raw_value(after) == desired, "Learning setter readback did not match the exact requested role")
        fresh = [self.observed(expected) for _ in range(2)]
        require(fresh[0].projection["all_values_sha256"] == fresh[1].projection["all_values_sha256"]
                and all(value_key(parameters(item)[name]) == value_key(after) for item in fresh),
                "Learning change lacks two fresh complete stable typed observations")
        return fresh[-1]

    def pilot(self, baseline, selector_map, binding):
        require(isinstance(binding, LearningBinding) and binding.selector_map_sha256 == selector_map.sha256
                and (binding.document, binding.session, binding.object_uid) ==
                (self.document, self.session, baseline.instance["object_uid"])
                and selector_map.rows[1] == (binding.value, binding.label),
                "The separately observed lowest-key binding does not match this exact live context")
        return self._pilot(baseline, selector_map,
                           LearningIntent(binding.value, binding.label, BindingEvidence.EXTERNAL_GUI))

    def candidate_pilot(self, baseline, selector_map):
        require(isinstance(selector_map, SelectorMap) and len(selector_map.rows) == 129
                and tuple(row[0] for row in selector_map.rows) == tuple(range(129))
                and hashlib.sha256(encoded(selector_map.rows)).hexdigest() == selector_map.sha256,
                "The candidate requires the complete observed finite selector map")
        records = quiet(baseline)
        require(raw_value(records[MODE]) == 0 and raw_value(records[SELECTOR]) == 0
                and raw_value(records[ASSIGNED]) == 0
                and all(raw_value(records[name]) == 0 for name in LEARN),
                "The candidate starts from the original empty pitch1 chord and quiet sentinel view")
        value, label = selector_map.rows[1]
        self.summary.update(mode="chorder_candidate_binding_experiment", candidate_binding_verified=False,
                            candidate_core_pitch_index=0, external_gui_binding_observed=False,
                            binding_proof="pending_two_complete_changed_model_status_observations")
        self.phase("chorder-fixed-pilot", "CandidateBindingPending", selector_value=value,
                   selector_label=label, expected_core_pitch_index=0, selector_map_sha256=selector_map.sha256)
        return self._pilot(baseline, selector_map, LearningIntent(value, label, BindingEvidence.CANDIDATE))

    def _pilot(self, baseline, selector_map, intent):
        candidate = intent.evidence is BindingEvidence.CANDIDATE
        self.phase("chorder-fixed-pilot", "OriginalProtection")
        original = self.capsule_pair(baseline)
        prepared = self.set_role(baseline, MODE, 1)
        require(changed_names(baseline, prepared) <= {MODE, SELECTOR, *LEARN},
                "Learning setup changed an unrelated value or model status")
        require(raw_value(parameters(prepared)[SELECTOR]) in range(129)
                and parameters(prepared)[SELECTOR]["text"] == selector_map.rows[
                    int(raw_value(parameters(prepared)[SELECTOR]))][1], "Prepared selector is not formatter-bound")
        if raw_value(parameters(prepared)[SELECTOR]) != intent.value:
            selected = self.set_role(prepared, SELECTOR, intent.value)
            require(changed_names(prepared, selected) <= {SELECTOR, *LEARN},
                    "Key selection changed an unrelated value or model status")
            prepared = selected
        records = parameters(prepared)
        require(records[SELECTOR]["text"] == intent.label and raw_value(records[ASSIGNED]) == 0
                and all(raw_value(records[name]) == 0 for name in LEARN),
                "The selected originally empty chord did not yield an empty refreshed learning view")
        prep_capsule = self.capsule_pair(prepared)
        self.release(prepared, prep_capsule)
        self.phase("chorder-fixed-pilot", "PrimaryBitChange")
        changed = self.set_role(prepared, BIT, 1)
        require(changed_names(prepared, changed) == {BIT, ASSIGNED}
                and raw_value(parameters(changed)[ASSIGNED]) == 1,
                "The fixed learning bit did not change exactly its independent explicit-chord model status")
        if candidate:
            self.summary.update(candidate_binding_verified=True,
                                binding_proof="two_complete_changed_model_status_observations")
            self.phase("chorder-fixed-pilot", "CandidateBindingConfirmed", observations=2,
                       selector_value=intent.value, selector_label=intent.label, core_pitch_index=0,
                       independent_model_status=ASSIGNED, other_model_statuses_unchanged=True)
        changed_capsule = self.capsule_pair(changed, different_from=prep_capsule if candidate else None)
        require((changed_capsule.byte_count, changed_capsule.sha256) !=
                (prep_capsule.byte_count, prep_capsule.sha256),
                "The learning edit did not change the prepared protected serialization")
        self.release(changed, changed_capsule)
        self.phase("chorder-fixed-pilot", "OriginalLoadInFlight")
        receipt = self.state_call(changed, "restore", original.token)
        if receipt["result"].get("load_accepted") is not True:
            raise RestorationFailure("The sole immutable original capsule load was not accepted")
        model_restored = [self.observed(baseline) for _ in range(2)]
        require(model_restored[0].projection["all_values_sha256"] == model_restored[1].projection["all_values_sha256"],
                "Post-load model observations are unstable")
        for current in model_restored:
            require(raw_value(parameters(current)[ASSIGNED]) == 0,
                    "Original load did not restore the empty model chord before GUI cleanup")
            require(changed_names(baseline, current) <= {MODE, SELECTOR, *LEARN},
                    "Original load left an unrelated typed or model-status difference")
        restored = model_restored[-1]
        self.phase("chorder-fixed-pilot", "ModelRestoredBeforeGuiCleanup", observations=2)
        if raw_value(parameters(restored)[MODE]) != 0:
            off = self.set_role(restored, MODE, 0)
            require(changed_names(restored, off) <= {MODE, *LEARN}
                    and all(raw_value(parameters(off)[name]) == 0 for name in LEARN),
                    "Mode-off GUI cleanup changed an unrelated value")
            restored = off
        require(raw_value(parameters(restored)[MODE]) == 0
                and all(raw_value(parameters(restored)[name]) == 0 for name in LEARN),
                "Quiet mode-off state was not established before restoring the sentinel")
        if raw_value(parameters(restored)[SELECTOR]) != 0:
            sentinel = self.set_role(restored, SELECTOR, 0)
            require(changed_names(restored, sentinel) <= {SELECTOR},
                    "Sentinel GUI cleanup changed an unrelated value")
        self.exact_pair(baseline)
        self.capsule_pair(baseline, retained=original)
        self.release(baseline, original)
        self.exact_pair(baseline)
        require(not self.live_capsules, "The successful pilot still owns live state tokens")
        self.phase("chorder-fixed-pilot", "StateConfirmed", complete_typed_reads=2,
                   fresh_serialized_matches=2, model_status_restored_before_gui_cleanup=True)
        self.summary.update(status=("fixed_candidate_binding_and_learning_roundtrip_verified" if candidate
                                    else "fixed_learning_bit_roundtrip_verified"), completed_at=utc_now(),
                            learning_bits_verified=1, full_learning_sweep_verified=False,
                            serialized_model_claim="one_empty_input_chord_changed_and_restored_with_independent_status")
        self.progress()
        return self.summary


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default="target/release/fender-studio-pro-mcp")
    parser.add_argument("--fixture", required=True, help="Private JSON with one exact Chorder target")
    parser.add_argument("--scratch-title", required=True)
    parser.add_argument("--output-dir", default=".local/live/chorder-learning-" + datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ"))
    pilot = parser.add_mutually_exclusive_group()
    pilot.add_argument("--binding-file", help="Private separately observed binding evidence; enables ONE fixed bit pilot")
    pilot.add_argument("--candidate-pilot", action="store_true",
                       help="Experiment on one fixed lowest-key candidate; model status must prove its binding before load")
    parser.add_argument("--pause", action="store_true", help="Start the owner before opening the scratch song")
    args = parser.parse_args(argv)
    target = json.loads(private_path(args.fixture).read_text())
    binding = LearningBinding.parse(json.loads(private_path(args.binding_file).read_text())) if args.binding_file else None
    journal = Journal(args.output_dir)
    client = runner = None
    try:
        binary = Path(args.binary).resolve()
        journal.append("run_started", mode="chorder_fixed_learning_pilot", schema=1,
                       binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                       pilot_requested=binding is not None or args.candidate_pilot,
                       candidate_pilot_requested=args.candidate_pilot,
                       source_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
        client = McpClient([str(binary), "serve"], timeout=45)
        if args.pause:
            input("Open exactly the named disposable scratch song, then press Enter: ")
        runner = LearningRunner(client, journal, args.scratch_title)
        runner.initialize()
        baseline, mapping = runner.preflight(target)
        if args.candidate_pilot:
            runner.candidate_pilot(baseline, mapping)
        elif binding is None:
            runner.summary.update(status="selector_preflight_only", selector_map_sha256=mapping.sha256,
                                  learning_bits_verified=0, completed_at=utc_now())
            runner.progress()
        else:
            runner.pilot(baseline, mapping, binding)
        journal.append("run_completed", summary=runner.summary)
        print(json.dumps({"status": runner.summary["status"], "all_learning_controls_verified": False}), flush=True)
        return 0
    except Exception as error:
        journal.append("run_failed", error_type=type(error).__name__, message=str(error),
                       automatic_cleanup_attempted=False)
        if runner is not None:
            runner.summary.update(status="failed", error_type=type(error).__name__, error=str(error),
                                  completed_at=utc_now(), pending_known_capsules=len(runner.live_capsules),
                                  unresolved_run_requires_scratch_discard=True)
            runner.progress()
        print(f"Acceptance stopped: {type(error).__name__}: {error}", file=sys.stderr, flush=True)
        return 1
    finally:
        if client is not None:
            client.close()
        journal.close()


if __name__ == "__main__":
    sys.exit(main())
