#!/usr/bin/env python3
"""Audit native access and validate reversible controls in disposable fixtures.

Raw values and receipts are written only below this repository's .local tree.
Model changes, commands and other coupled actions require a separately proved
state restore workflow and remain explicit incomplete cases in this runner.
The opt-in direct digest mode verifies two fresh complete exposed projections
after every exact original-value restoration, with full-page checkpoints.
"""
import argparse
from collections import Counter
from dataclasses import dataclass
from datetime import datetime, timezone
import hashlib
import json
import math
import os
from pathlib import Path
import re
import struct
import sys
import time
import uuid

from mcp_client import McpClient
from parameter_digest import DigestProjectionError, HASH_FIELDS, project_snapshot
from verify_live_plugins import structured


REPOSITORY = Path(__file__).resolve().parent.parent
PRIVATE_ROOT = REPOSITORY / ".local"
MAX_SAFE_INTEGER = 9_007_199_254_740_991
MAX_OBSERVED_JSON_FLOAT_ULPS = 1
MAX_DIRECT_DIGEST_CASES_BETWEEN_FULL_CHECKS = 32
UNKNOWN_OUTCOMES = frozenset(("outcome_unknown", "unknown"))
NUMERIC_KINDS = {"Toggle", "Integer", "Float", "List", "Scroll"}
EXTERNAL_ACTIONS = {"pingAgain", "pingMode", "zoomOut", "latencyTotal", "ms", "samples", "deviceName"}
EXTERNAL_STATE_ROLES = {"scopeMode", "showImage", "latencyOffset", "mathMode"}
PARAMETER_ACTIONS = {
    "Expander": {"swapfreqs"},
    "Gate": {"swapfreqs"},
    "Compressor": {"studiocomp.swapfreqs", "studiocomp.resetmin"},
    "Limiter": {"resetmin"},
}
MODEL_SELECTORS = {
    "Ampire": {"opt.ampmodel", "opt.cabmodel", *[f"opt.stompmodel{i}" for i in range(1, 9)]},
    "Pedalboard": {f"opt.stompmodel{i}" for i in range(1, 9)},
    "Fat Channel": {"opt.eqmodel", "opt.compmodel"},
    "Voice FX": {"voicefxopt.fxmodel"},
    "Mai Tai": {"character1.presets"},
    "Vocal Tune": {"scale"},
    "Mustang Native": {"global.ampmodel"},
    "Rumble Native": {"global.ampmodel"},
}


class AcceptanceFailure(RuntimeError):
    pass


class ContextChanged(AcceptanceFailure):
    pass


class UnknownMutation(AcceptanceFailure):
    pass


class RestorationFailure(AcceptanceFailure):
    pass


class DependencyFailure(RestorationFailure):
    pass


def require(condition, message):
    if not condition:
        raise AcceptanceFailure(message)


def utc_now():
    return datetime.now(timezone.utc).isoformat()


def encoded(value):
    return json.dumps(value, ensure_ascii=False, allow_nan=False, sort_keys=True, separators=(",", ":")).encode()


def digest(value):
    return hashlib.sha256(encoded(value)).hexdigest()


def private_path(path):
    resolved = Path(path).resolve()
    require(resolved != PRIVATE_ROOT.resolve() and resolved.is_relative_to(PRIVATE_ROOT.resolve()),
            "Raw acceptance files must stay below the repository .local directory")
    return resolved


def exclusive_json(path, value):
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0)
    fd = os.open(path, flags, 0o600)
    with os.fdopen(fd, "wb") as stream:
        stream.write(encoded(value) + b"\n")
        stream.flush()
        os.fsync(stream.fileno())


class Journal:
    """One new private run; a prior run is never replayed automatically."""

    def __init__(self, directory):
        self.directory = private_path(directory)
        self.directory.mkdir(parents=True, exist_ok=True, mode=0o700)
        self.ledger_path = self.directory / "ledger.jsonl"
        if self.ledger_path.exists():
            pending = self.pending(self.ledger_path)
            raise AcceptanceFailure(f"Run ledger already exists; {len(pending)} unresolved transactions. Use a fresh run directory after restoring the scratch baseline")
        self.baselines = self.directory / "baselines"
        self.baselines.mkdir(mode=0o700)
        flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0)
        self.stream = os.fdopen(os.open(self.ledger_path, flags, 0o600), "wb")
        self.sequence = 0

    def append(self, event, **fields):
        self.sequence += 1
        self.stream.write(encoded({"sequence": self.sequence, "at": utc_now(), "event": event, **fields}) + b"\n")
        self.stream.flush()
        os.fsync(self.stream.fileno())

    def baseline(self, key, value):
        path = self.baselines / (key + ".json")
        exclusive_json(path, value)
        return {"file": str(path.relative_to(self.directory)), "sha256": digest(value)}

    def progress(self, value):
        temporary = self.directory / ("progress-" + uuid.uuid4().hex + ".tmp")
        exclusive_json(temporary, value)
        temporary.replace(self.directory / "progress.json")

    def close(self):
        self.stream.close()

    @staticmethod
    def pending(path):
        states = {}
        mutations = {}
        with Path(path).open("rb") as stream:
            for line in stream:
                try:
                    entry = json.loads(line)
                except (json.JSONDecodeError, UnicodeDecodeError):
                    states["truncated_ledger"] = "unknown"
                    break
                if entry.get("event") == "transaction_phase":
                    states[entry["transaction"]] = entry["phase"]
                if entry.get("event") == "rpc_in_flight" and entry.get("mutation") is True:
                    mutations[entry["call"]] = "MutationInFlight"
                elif entry.get("event") == "rpc_returned":
                    if entry["call"] in mutations:
                        receipt = entry.get("receipt")
                        outcome = receipt.get("outcome") if isinstance(receipt, dict) else None
                        result = receipt.get("result") if isinstance(receipt, dict) else None
                        outcomes = [outcome]
                        if isinstance(result, dict):
                            outcomes.extend(result[field] for field in ("outcome", "native_outcome") if field in result)
                        if any(not isinstance(value, str) or value not in (
                                "observed", "applied_verified", "applied_unverified", "not_applied") for value in outcomes):
                            mutations[entry["call"]] = "UnknownAfterDispatch"
                        else:
                            mutations.pop(entry["call"], None)
                elif entry.get("event") == "rpc_context_lost":
                    if entry.get("mutation"):
                        mutations[entry["call"]] = "ContextLostAfterDispatch"
        terminal = {"StateConfirmed", "NoWriteConfirmed", "FailedBeforeDispatch"}
        return {**{key: state for key, state in states.items() if state not in terminal},
                **{f"rpc:{key}": state for key, state in mutations.items()}}


def decode_float_bits(bits):
    require(isinstance(bits, str) and len(bits) == 16 and all(c in "0123456789abcdefABCDEF" for c in bits),
            "Finite float bits are unavailable")
    number = struct.unpack(">d", bytes.fromhex(bits))[0]
    require(math.isfinite(number), "Native float bits must be finite")
    return number


def float_ulp_distance(left, right):
    if left == right:
        return 0

    def ordered(number):
        bits = struct.unpack(">Q", struct.pack(">d", number))[0]
        return (~bits & ((1 << 64) - 1)) if bits >> 63 else bits | (1 << 63)

    return abs(ordered(left) - ordered(right))


def in_observed_float_range(value, lower, upper):
    return ((lower <= value or float_ulp_distance(value, lower) <= MAX_OBSERVED_JSON_FLOAT_ULPS)
            and (value <= upper or float_ulp_distance(value, upper) <= MAX_OBSERVED_JSON_FLOAT_ULPS))


def value_identity(parameter):
    native = parameter.get("native_value")
    require(isinstance(native, dict) and isinstance(native.get("data"), dict), "Exact native value metadata is unavailable")
    data = native["data"]
    kind = data.get("kind")
    if kind == "float":
        bits = data.get("bits_hex")
        expected = decode_float_bits(bits)
        number = data.get("value")
        require(type(number) in (int, float) and math.isfinite(number), "Native float value is unavailable")
        require(float_ulp_distance(expected, number) <= MAX_OBSERVED_JSON_FLOAT_ULPS,
                "Native float bits and JSON value disagree")
        require(type(parameter.get("value")) in (int, float) and parameter["value"] == number,
                "Native and scalar float values disagree")
        return (native.get("variant_tag"), kind, bits.upper())
    if kind == "integer":
        number = data.get("value")
        exact = data.get("exact_decimal")
        require(type(number) is int or isinstance(exact, str), "Exact native integer is unavailable")
        if type(number) is int:
            require(type(parameter.get("value")) is int and parameter["value"] == number,
                    "Native and scalar integer values disagree")
        return (native.get("variant_tag"), kind, str(number) if type(number) is int else exact)
    if kind == "text":
        require(isinstance(data.get("value"), str), "Exact native string is unavailable")
        require(parameter.get("value") == data["value"], "Native and scalar string values disagree")
        return (native.get("variant_tag"), kind, data["value"])
    return (native.get("variant_tag"), kind, digest(data))


def raw_value(parameter):
    identity = value_identity(parameter)
    return decode_float_bits(identity[2]) if identity[1] == "float" else parameter.get("value")


def requested_value(arguments):
    return decode_float_bits(arguments["float_bits"]) if "float_bits" in arguments else next(iter(arguments.values()))


def parameter_schema(parameter):
    return {field: parameter.get(field) for field in
            ("name", "tag", "native_kind", "enabled", "readOnly", "writable", "min", "max", "support", "display_parser")}


def provider_index(parameter):
    native = parameter.get("native", parameter)
    return native.get("index") if isinstance(native, dict) else None


@dataclass
class ProviderSnapshot:
    target: dict
    source: str
    instance: dict
    records: list

    @property
    def schema(self):
        return digest([{"index": provider_index(p), **parameter_schema(p)} for p in self.records])

    def serialized(self):
        return {"target": self.target, "source": self.source, "instance": self.instance,
                "schema_sha256": self.schema, "records": self.records}


@dataclass(frozen=True)
class DirectDigestBaseline:
    provider: ProviderSnapshot
    projection: dict


def native_digest_matches(expected, native, read_only_observations=False):
    require(isinstance(native, dict) and native.get("ok") is True
            and native.get("outcome") == "observed" and "error" not in native,
            "Native parameter digest is not a successful observed result")
    for key in HASH_FIELDS:
        require(isinstance(native.get(key), str) and re.fullmatch(r"[0-9a-f]{64}", native[key]) is not None,
                "Native parameter digest lacks an exact " + key)
    for key, value in expected.items():
        if key in HASH_FIELDS[1:]:
            continue
        actual = native.get(key)
        equal = type(actual) is type(value) and actual == value
        if not equal and key in ("instance", "source", "count", "schema_sha256"):
            raise ContextChanged("Native parameter digest changed the bound provider " + key)
        require(equal, "Native parameter digest contract differs: " + key)
    fields = ("non_read_only_values_sha256",) if read_only_observations else HASH_FIELDS[1:]
    return all(native[key] == expected[key] for key in fields)


def classification(plugin, parameter):
    """Access facts precede semantic categories; an unknown is never writable."""
    required = ("enabled", "readOnly", "writable", "native_kind", "support", "native_value")
    if any(field not in parameter for field in required):
        return "unresolved_access"
    if not all(type(parameter[field]) is bool for field in ("enabled", "readOnly", "writable")):
        return "unresolved_access"
    if not isinstance(parameter["support"], dict) or not isinstance(parameter["native_kind"], str):
        return "unresolved_access"
    if parameter["readOnly"]:
        return "read_only"
    if not parameter["enabled"]:
        return "disabled_dependency"
    name, kind = parameter.get("name"), parameter["native_kind"]
    if kind not in NUMERIC_KINDS | {"String"}:
        return "dedicated_type_required"
    if name in PARAMETER_ACTIONS.get(plugin, ()) or (plugin.startswith("External Effect ") and name in EXTERNAL_ACTIONS):
        return "dedicated_action_required"
    if plugin in ("External Effect Mono", "External Effect Stereo") and name in EXTERNAL_STATE_ROLES:
        return "state_restore_required"
    if name in MODEL_SELECTORS.get(plugin, set()) or (
            plugin in ("Mustang Native", "Rumble Native") and str(name).startswith("global.effectmodel")):
        return "model_state_restore_required"
    try:
        value_identity(parameter)
    except AcceptanceFailure:
        return "unresolved_exact_value"
    if kind == "String":
        if name != "deviceNotes" or parameter.get("display_parser") != "LiteralString":
            return "dedicated_string_required"
        return "literal_string" if parameter["writable"] and parameter["support"].get("text") is True else "unsupported_write"
    value, lower, upper = raw_value(parameter), parameter.get("min"), parameter.get("max")
    raw_float = parameter["native_value"]["data"].get("kind") == "float"
    if any(type(n) not in (int, float) or not math.isfinite(n) for n in (value, lower, upper)):
        return "unsupported_numeric_representation"
    if not raw_float and any(abs(n) > MAX_SAFE_INTEGER for n in (value, lower, upper)):
        return "unsupported_numeric_representation"
    if lower > upper:
        return "empty_choice_range" if kind == "List" else "invalid_range"
    if lower == upper:
        return "constant_range"
    if not (in_observed_float_range(value, lower, upper) if raw_float else lower <= value <= upper):
        return "state_restore_required" if kind == "List" else "invalid_current_value"
    if parameter["writable"] is not True or parameter["support"].get("raw") is not True:
        return "unsupported_write"
    return "numeric_scalar"


def integral_parameter_value(parameter, field):
    native = parameter.get("native_value" if field == "value" else "native_" + field)
    shadow = parameter.get(field)
    require(isinstance(native, dict) and set(native) == {"variant_tag", "data"}
            and isinstance(native.get("data"), dict), "Integral controls require complete exact native " + field)
    data, tag = native["data"], native["variant_tag"]
    kind = data.get("kind")
    require(type(tag) is int and -(1 << 15) <= tag < (1 << 15)
            and ((kind == "integer" and tag & 15 == 1 and set(data) == {"kind", "value"})
                 or (kind == "float" and tag & 15 == 2 and set(data) == {"kind", "value", "bits_hex"})),
            "Integral controls require a matching native numeric carrier for " + field)
    require(all(type(value) in (int, float) and abs(value) <= MAX_SAFE_INTEGER
                for value in (data.get("value"), shadow)),
            "Integral controls require finite safe native and public values for " + field)
    exact = raw_value({"native_value": native, "value": shadow})
    require(type(exact) in (int, float) and math.isfinite(exact) and abs(exact) <= MAX_SAFE_INTEGER
            and exact == int(exact) and type(shadow) in (int, float) and shadow == exact,
            "Integral controls require a finite exact safe integer " + field)
    return int(exact)


def candidate(parameter):
    if parameter["native_kind"] == "String":
        original = parameter["value"]
        value = original + " MCP 한글"
        require(len(value.encode("utf-16-le")) // 2 <= 4096, "String cannot fit a changed bounded test value")
        return {"text": value}, {"text": original}
    if parameter["native_kind"] in ("Integer", "List"):
        original, lower, upper = (integral_parameter_value(parameter, field) for field in ("value", "min", "max"))
    else:
        original, lower, upper = raw_value(parameter), parameter.get("min"), parameter.get("max")
    data_kind = parameter["native_value"]["data"]["kind"]
    if parameter["native_kind"] == "Toggle":
        desired = 0 if original == 1 else 1
    elif parameter["native_kind"] in ("Integer", "List"):
        require(lower <= original <= upper and lower < upper,
                "Integral controls require an in-range value and a different integral choice")
        desired = original + 1 if original < upper else original - 1
    elif data_kind == "integer":
        desired = original + 1 if original + 1 <= upper else original - 1
    else:
        fraction = 0.37 if original > lower + (upper - lower) * 0.5 else 0.63
        choices = (lower * (1 - fraction) + upper * fraction,
                   lower * 0.5 + upper * 0.5,
                   lower, upper,
                   math.nextafter(original, upper if original < upper else lower))
        desired = next((value for value in choices if math.isfinite(value) and lower <= value <= upper and value != original), None)
    require(type(desired) in (int, float) and math.isfinite(desired) and lower <= desired <= upper and desired != original,
            "No different supported test value exists")
    if data_kind == "float":
        return ({"float_bits": struct.pack(">d", float(desired)).hex().upper()},
                {"float_bits": parameter["native_value"]["data"]["bits_hex"].upper()})
    return {"value": desired}, {"value": original}


class Runner:
    def __init__(self, client, journal, scratch_title, prepare_midi=False, block_size=16,
                 read_only_observations=False, use_parameter_digest=False):
        require(type(use_parameter_digest) is bool, "Parameter digest verification requires an explicit boolean mode")
        require(not use_parameter_digest or (type(block_size) is int and block_size == 1),
                "Direct parameter digest verification requires block_size=1")
        self.client, self.journal, self.scratch_title = client, journal, scratch_title
        self.prepare_midi, self.block_size = prepare_midi, block_size
        self.read_only_observations = read_only_observations
        self.direct_digest_mode = use_parameter_digest
        self.direct_digest_baseline = None
        self.document = self.session = None
        self.call_count = 0
        self.summary = {"schema": 1, "started_at": utc_now(), "status": "running", "classes": [],
                        "counts": {}, "all_writable_parameters_verified": False,
                        "read_only_values": "observed_not_restored" if read_only_observations else "exact_comparison"}
        self.counts = Counter()
        self.requested_parameter_names = set()
        self.matched_parameter_names = set()
        if use_parameter_digest:
            self.summary.update(parameter_verification="direct_restoration_with_native_parameter_digest",
                                before_next_parameter="two_fresh_complete_native_parameter_digest_matches",
                                full_page_checkpoint_max_cases=MAX_DIRECT_DIGEST_CASES_BETWEEN_FULL_CHECKS,
                                restoration="direct_exact_original_value", atomic_snapshot=False,
                                opaque_contents_verified=False)

    def track_parameter_filter(self, parameters, records=()):
        if parameters:
            self.requested_parameter_names.update(parameters)
            self.matched_parameter_names.update(self.requested_parameter_names.intersection(parameter["name"] for parameter in records))

    def progress(self):
        self.summary["counts"] = dict(self.counts)
        self.summary["rpc_count"] = self.call_count
        self.journal.progress(self.summary)

    def finish(self, write_roundtrips):
        if self.direct_digest_mode and write_roundtrips:
            require(all(not row.get("selected_for_write") or row.get("digest_fixture_complete") is True
                        for row in self.summary["classes"]),
                    "Direct digest fixture end or full-page checkpoint is incomplete")
        selected = sum(row.get("selected_for_write", 0) for row in self.summary["classes"])
        unmatched = sorted(self.requested_parameter_names - self.matched_parameter_names)
        self.summary["unmatched_parameters"] = unmatched
        self.summary["eligible_roundtrips_verified"] = (write_roundtrips and selected > 0 and not unmatched
                                                       and selected == self.counts["changed_readback_restored"])
        self.summary["native_access_complete"] = self.counts["category_unresolved_access"] == 0
        required = sum(count for key, count in self.counts.items() if key.startswith("category_")
                       and key not in {"category_numeric_scalar", "category_literal_string", "category_read_only",
                                       "category_empty_choice_range", "category_constant_range"})
        self.summary["dedicated_or_unresolved_records"] = required
        partial = required > 0 or self.counts["separate_graph_workflow_required"] > 0
        if write_roundtrips:
            partial = partial or not self.summary["eligible_roundtrips_verified"]
        self.summary["status"] = ("roundtrips_partial" if partial else "selected_scalar_roundtrips_verified") if write_roundtrips else "audit_completed"
        if write_roundtrips and selected == 0:
            self.summary["status"] = "no_eligible_write_cases"
        self.summary["completed_at"] = utc_now()
        self.progress()
        self.journal.append("run_completed", summary=self.summary)
        return 2 if write_roundtrips and partial else 0

    def call(self, name, arguments=None, mutation=False, allow_error=False):
        args = dict(arguments or {})
        if self.document is not None:
            args["expected_document"] = self.document
        self.call_count += 1
        call_id = self.call_count
        self.journal.append("rpc_in_flight", call=call_id, tool=name, arguments=args, mutation=mutation)
        try:
            receipt = structured(self.client.call(name, args))
        except Exception as error:
            self.journal.append("rpc_exception", call=call_id, error_type=type(error).__name__, message=str(error))
            if mutation:
                raise UnknownMutation("Mutation transport failed; no automatic retry or restore is permitted") from error
            raise AcceptanceFailure("Read request failed") from error
        self.journal.append("rpc_returned", call=call_id, receipt=receipt)
        if not isinstance(receipt, dict):
            raise UnknownMutation("Request returned a malformed receipt; no follow-on mutation is permitted")
        if self.document is not None and (receipt.get("document") != self.document or receipt.get("session") != self.session):
            self.journal.append("rpc_context_lost", call=call_id, mutation=mutation)
            raise ContextChanged("Bridge document or session changed")
        outcome = receipt.get("outcome")
        result = receipt.get("result")
        native = [result[field] for field in ("outcome", "native_outcome") if field in result] if isinstance(result, dict) else []
        if any(not isinstance(value, str) or value not in (
                "observed", "applied_verified", "applied_unverified", "not_applied") for value in (outcome, *native)):
            raise UnknownMutation("Request outcome evidence is unknown or malformed; no follow-on mutation is permitted")
        if not mutation and (outcome not in ("observed", "not_applied")
                             or (outcome == "observed" and any(value != "observed" for value in native))):
            raise UnknownMutation("Read request did not establish observed evidence; no follow-on mutation is permitted")
        if receipt.get("error") and not allow_error:
            raise AcceptanceFailure(str(receipt["error"]))
        if not mutation and outcome == "not_applied" and not allow_error:
            raise AcceptanceFailure("Read request was refused and has no observed data")
        return receipt

    def operation(self, name, args=None, mutation=False):
        return self.call("operation_run", {"operation": name, "args": args or {}}, mutation=mutation)["result"]

    def initialize(self):
        receipt = self.call("host_status")
        result = receipt.get("result", {})
        require(result.get("open") is True and result.get("title") == self.scratch_title, "Open exactly the named disposable scratch song")
        require(isinstance(receipt.get("document"), str) and receipt["document"] and isinstance(receipt.get("session"), str) and receipt["session"],
                "Open bridge context is unavailable")
        self.document, self.session = receipt["document"], receipt["session"]
        self.journal.append("context_bound", document=self.document, session=self.session, title=self.scratch_title)

    def fixture_check(self, target, setup=False):
        song = self.operation("song")
        require(song.get("title") == self.scratch_title and song.get("transport", {}).get("playing") is False
                and song["transport"].get("recording") is False, "Scratch song or stopped transport changed")
        name = target.get("track") or target.get("channel")
        require(isinstance(name, str) and name.startswith("FSP Verify "), "Only named validation fixtures are allowed")
        kind = target.get("kind", "effect")
        media = "Music" if kind in ("instrument", "note_effect") else "Audio"
        tracks = self.operation("tracks", {"name": name, "events": True, "maxEvents": 1})
        matches = [track for track in tracks if track.get("name") == name]
        if kind != "mix_fx":
            require(len(matches) == 1 and matches[0].get("mediaType") == media and matches[0].get("eventCount") == 0
                    and matches[0].get("events") == [], "Fixture must be one unique empty track")
        if media == "Music":
            selected = [track for track in self.operation("tracks", {"events": False}) if track.get("selected")]
            exclusive = len(selected) == 1 and selected[0].get("name") == name and selected[0].get("mediaType") == "Music"
            if not exclusive:
                require(setup, "Music fixture selection changed during acceptance")
                self.operation("selectTrack", {"name": name, "mediaType": "Music", "exclusive": True}, mutation=True)
            selected = [track for track in self.operation("tracks", {"events": False}) if track.get("selected")]
            require(len(selected) == 1 and selected[0].get("name") == name and selected[0].get("mediaType") == "Music",
                    "Music fixture must be exclusively selected")
            state = self.call("plugin_inspect", target)["result"].get("focusedTrackState")
            require(isinstance(state, dict) and state.get("label") == name, "Focused Music fixture identity is unavailable")
            if setup and self.prepare_midi:
                for field, wanted, command in (("mute", 1, "Mute"), ("recordArmed", 0, "Arm"), ("monitor", 0, "Monitor")):
                    require(type(state.get(field)) in (bool, int, float) and state[field] in (0, 1), "Music fixture switch is unsupported")
                    if state[field] != wanted:
                        self.operation("command", {"category": "Track", "name": command}, mutation=True)
                        state = self.call("plugin_inspect", target)["result"].get("focusedTrackState")
                        require(isinstance(state, dict) and state.get("label") == name and state.get(field) == wanted,
                                "Music fixture preparation did not read back")
            require(state.get("mute") in (True, 1) and state.get("recordArmed") in (False, 0)
                    and state.get("monitor") in (False, 0), "Music fixture must be muted, unarmed and unmonitored")
        else:
            channel = matches[0].get("channel") if matches else name
            mapped = [item for item in self.operation("channels") if item.get("label") == channel]
            require(len(mapped) == 1 and mapped[0].get("mute") in (True, 1), "Fixture channel must be uniquely mapped and muted")
            if kind != "mix_fx":
                require(mapped[0].get("recordArmed") in (False, 0) and mapped[0].get("monitor") in (False, 0),
                        "Fixture must not be armed or monitored")

    def snapshot(self, target, previous=None):
        records, seen, offset = [], set(), 0
        instance = previous.instance if previous else None
        source = previous.source if previous else "auto"
        count = None
        while True:
            args = {**target, "source": source, "offset": offset, "limit": 64}
            if instance:
                args["expected_instance"] = instance["object_uid"]
            page = self.call("plugin_parameters", args)["result"]
            discovery = page.get("parameter_discovery", {})
            require(not page.get("native_error") and discovery.get("completeness") == "current_provider",
                    "Native current-provider enumeration is unavailable")
            current_instance, current_source = page.get("instance"), discovery.get("source")
            require(isinstance(current_instance, dict) and current_instance.get("object_uid") and current_instance.get("class_uid"),
                    "Provider identity is unavailable")
            require(current_source in ("dsp", "controller"), "Unsupported provider source")
            if instance is not None and current_instance != instance:
                raise ContextChanged("Provider instance changed between pages")
            if source != "auto" and current_source != source:
                raise ContextChanged("Provider source changed between pages")
            instance, source = current_instance, current_source
            reported = page.get("reported_count")
            require(type(reported) is int and 0 <= reported <= 1_000_000, "Invalid native provider count")
            if count is not None and reported != count:
                raise ContextChanged("Provider count changed between pages")
            count = reported
            require(page.get("offset") == offset and page.get("page_complete") is True and not page.get("failed_indices"),
                    "Native parameter index page is incomplete")
            params = page.get("params")
            require(isinstance(params, list) and len(params) <= 64, "Invalid native record page")
            for parameter in params:
                index = provider_index(parameter)
                require(type(index) is int and offset <= index < min(offset + 64, count) and index not in seen,
                        "Invalid or duplicate provider index")
                require(isinstance(parameter.get("name"), str), "Canonical parameter name is unavailable")
                seen.add(index)
                records.append(parameter)
            next_offset = page.get("next_offset")
            if next_offset is None:
                break
            require(type(next_offset) is int and offset < next_offset <= count, "Native pagination did not advance")
            offset = next_offset
        require(seen == set(range(count)), "Current provider index coverage is incomplete")
        records.sort(key=provider_index)
        require(len({p["name"] for p in records}) == len(records), "Canonical parameter names are ambiguous")
        result = ProviderSnapshot(dict(target), source, instance, records)
        if previous is not None and result.schema != previous.schema:
            raise ContextChanged("Provider schema changed")
        return result

    def read_parameter(self, provider, original):
        result = self.call("plugin_parameters", {**provider.target, "source": provider.source,
                           "names": [original["name"]], "expected_instance": provider.instance["object_uid"]})["result"]
        require(not result.get("missing") and len(result.get("params", [])) == 1, "Native parameter became unavailable")
        fresh = result["params"][0]
        if parameter_schema(fresh) != parameter_schema(original):
            raise ContextChanged("Native parameter schema changed")
        return fresh

    def phase(self, transaction, phase, **fields):
        self.journal.append("transaction_phase", transaction=transaction, phase=phase, **fields)

    def write(self, provider, parameter, values):
        return self.call("plugin_parameter_set", {**provider.target, "source": provider.source,
                         "param": parameter["name"], **values, "expected_instance": provider.instance["object_uid"]},
                         mutation=True, allow_error=True)

    def confirm_changed_readback(self, provider, parameter, setter_after, *, interval=0.1):
        require(type(interval) in (int, float) and math.isfinite(interval) and 0 <= interval <= 1,
                "Changed-value read interval must be from 0 to 1 second")
        require(isinstance(setter_after, dict) and parameter_schema(setter_after) == parameter_schema(parameter),
                "Setter parameter metadata changed")
        baseline_identity = value_identity(parameter)
        setter_identity = value_identity(setter_after)
        fresh = []
        for index in range(2):
            if index and interval:
                time.sleep(interval)
            self.journal.append("changed_readback_attempt", parameter=parameter["name"], attempt=index + 1)
            self.fixture_check(provider.target)
            observed = self.read_parameter(provider, parameter)
            require(value_identity(observed) != baseline_identity, "Write produced no changed value")
            normalized = setter_after.get("normalized")
            float_normalized_proof = all(
                item.get("native_kind") == "Float"
                and item.get("native_value", {}).get("data", {}).get("kind") == "float"
                and item.get("support", {}).get("normalized") is True
                for item in (parameter, setter_after, observed)) and (
                    type(normalized) in (int, float) and math.isfinite(normalized) and 0 <= normalized <= 1)
            if float_normalized_proof:
                actual = observed.get("normalized")
                require(type(actual) in (int, float) and math.isfinite(actual) and actual == normalized,
                        "Independent Float normalized value differs from the setter receipt")
            if value_identity(observed) != setter_identity:
                baseline_normalized = parameter.get("normalized")
                require(float_normalized_proof and type(baseline_normalized) in (int, float)
                        and math.isfinite(baseline_normalized) and 0 <= baseline_normalized <= 1
                        and baseline_normalized != normalized,
                        "Settled Float must change the baseline normalized value")
            require(value_identity(observed) == setter_identity or float_normalized_proof,
                    "Independent changed-value readback disagrees with the setter receipt")
            if fresh:
                require(value_identity(observed) == value_identity(fresh[0]),
                        "Two fresh changed-value raw reads are unstable")
                require(observed.get("normalized") == fresh[0].get("normalized"),
                        "Two fresh changed-value normalized reads are unstable")
            fresh.append(observed)
            self.journal.append("changed_readback_observation", parameter=parameter["name"], attempt=index + 1,
                                raw_matches_setter=value_identity(observed) == setter_identity,
                                exact_normalized_matches_setter=float_normalized_proof,
                                changed_from_baseline=True)
        observed = fresh[-1]
        facts = {"fresh_reads": 2, "exact_raw_stable": True,
                 "quantized_settlement": value_identity(observed) != setter_identity,
                 "evidence": "exact_native_normalized" if value_identity(observed) != setter_identity else "exact_native_raw",
                 "exact_normalized_matches_setter": float_normalized_proof}
        return observed, facts

    def roundtrip(self, provider, parameter, transaction):
        self.fixture_check(provider.target)
        original = self.read_parameter(provider, parameter)
        require(value_identity(original) == value_identity(parameter), "Parameter drifted from the immutable block baseline")
        change_args, restore_args = candidate(original)
        original_identity = value_identity(original)
        self.phase(transaction, "Prepared", parameter=original["name"], index=provider_index(parameter),
                   original=original, requested=change_args, source=provider.source,
                   instance=provider.instance, schema_sha256=provider.schema)
        self.phase(transaction, "ChangeInFlight")
        try:
            changed = self.write(provider, original, change_args)
        except (UnknownMutation, ContextChanged):
            self.phase(transaction, "UnknownAfterDispatch")
            raise
        if changed.get("outcome") == "not_applied":
            unchanged = self.read_parameter(provider, original)
            if value_identity(unchanged) != original_identity:
                self.phase(transaction, "UnknownAfterDispatch", reason="not_applied contradicted by changed readback")
                raise UnknownMutation("Rejected write changed parameter state")
            self.phase(transaction, "NoWriteConfirmed", receipt_error=changed.get("error"), result=changed.get("result"))
            return {"transaction": transaction, "status": "write_rejected", "parameter": original["name"]}
        if changed.get("outcome") not in ("applied_verified", "applied_unverified"):
            self.phase(transaction, "UnknownAfterDispatch")
            raise UnknownMutation("Write did not establish a known applied outcome")
        self.phase(transaction, "Changed", reported_outcome=changed["outcome"])
        failure = None
        observed = None
        settlement = None
        try:
            after = changed.get("result", {}).get("after")
            observed, settlement = self.confirm_changed_readback(provider, original, after)
            self.phase(transaction, "ChangedReadbackConfirmed", observed=observed, settlement=settlement)
        except (UnknownMutation, ContextChanged):
            self.phase(transaction, "UnknownAfterDispatch")
            raise
        except Exception as error:
            failure = str(error)
        try:
            self.fixture_check(provider.target)
            self.read_parameter(provider, original)
            self.phase(transaction, "RestoreInFlight")
            restored = self.write(provider, original, restore_args)
            require(restored.get("outcome") == "applied_verified", "Restoration setter did not verify")
            after = restored.get("result", {}).get("after")
            require(isinstance(after, dict) and value_identity(after) == original_identity, "Restoration receipt does not match the exact original value")
            self.phase(transaction, "Restored")
            self.fixture_check(provider.target)
            fresh = self.read_parameter(provider, original)
            require(value_identity(fresh) == original_identity, "Independent restoration readback differs from the exact original value")
            self.phase(transaction, "RestoreReadbackConfirmed", observed=fresh)
        except Exception as error:
            self.phase(transaction, "RestorationFailed", error_type=type(error).__name__, message=str(error))
            raise RestorationFailure("Restoration was not independently confirmed; no further writes are permitted") from error
        return {"transaction": transaction, "status": "changed_readback_restored" if failure is None else "change_failed_restored",
                "parameter": original["name"], "reason": failure,
                "settlement": settlement,
                "quantized": observed is not None and raw_value(observed) != requested_value(change_args)}

    def confirm_block(self, baseline, transactions):
        if self.direct_digest_mode:
            require(len(transactions) == 1, "Direct digest verification cannot batch parameter transactions")
            require(self.direct_digest_baseline is not None and self.direct_digest_baseline.provider is baseline,
                    "Direct parameter digest baseline was not initialized for this fixture")
            item = transactions[0]
            self.direct_digest_pair(baseline, self.direct_digest_baseline.projection,
                                    item["transaction"], "direct_restore")
            if item["status"] != "write_rejected":
                self.phase(item["transaction"], "StateConfirmed", status=item["status"],
                           whole_provider_comparison="exact_typed_exposed_parameter_projection",
                           consecutive_fresh_digest_matches=2, atomic_snapshot=False,
                           read_only_values=self.summary["read_only_values"],
                           opaque_contents_verified=False)
            self.counts[item["status"]] += 1
            self.journal.append("parameter_result", **item)
            self.progress()
            return
        self.fixture_check(baseline.target)
        current = self.snapshot(baseline.target, baseline)
        changed, observations = [], []
        for before, after in zip(baseline.records, current.records):
            try:
                equal = value_identity(before) == value_identity(after)
            except AcceptanceFailure:
                equal = before.get("native_value") == after.get("native_value") and before.get("value") == after.get("value")
            if not equal:
                explicitly_read_only = all(parameter.get("access_metadata_complete") is True
                                           and parameter.get("readOnly") is True and parameter.get("writable") is False
                                           for parameter in (before, after))
                if self.read_only_observations and explicitly_read_only:
                    observations.append(before["name"])
                else:
                    changed.append(before["name"])
        self.journal.append("block_comparison", schema_changed=False, changed_parameters=changed,
                            read_only_changed=observations,
                            read_only_values="observed_not_restored" if self.read_only_observations else "exact_comparison",
                            transactions=[item["transaction"] for item in transactions])
        self.counts["read_only_value_change_observations"] += len(observations)
        if changed:
            self.journal.append("block_dependency_change", parameters=changed, transactions=[item["transaction"] for item in transactions])
            raise DependencyFailure("Other provider state differs from the baseline after a test block")
        for item in transactions:
            if item["status"] != "write_rejected":
                self.phase(item["transaction"], "StateConfirmed", status=item["status"],
                           whole_provider_comparison="all_provider_schema_and_exact_writable_values_at_block_boundary"
                           if self.read_only_observations else "all_read_native_values_at_block_boundary",
                           read_only_values="observed_not_restored" if self.read_only_observations else "exact_comparison")
            self.counts[item["status"]] += 1
            self.journal.append("parameter_result", **item)
        self.progress()

    def direct_projection(self, provider):
        try:
            return project_snapshot(provider)
        except DigestProjectionError as error:
            raise AcceptanceFailure("Complete exact parameter digest projection is unavailable: " + str(error)) from error

    def direct_digest_read(self, baseline):
        self.fixture_check(baseline.target)
        receipt = self.call("plugin_parameter_digest", {**baseline.target, "source": baseline.source,
                            "expected_instance": baseline.instance["object_uid"]})
        require(receipt.get("outcome") == "observed" and not receipt.get("error"),
                "Native parameter digest receipt is not an observed read")
        return receipt.get("result")

    def direct_digest_pair(self, baseline, expected, transaction, checkpoint):
        for attempt in (1, 2):
            native = self.direct_digest_read(baseline)
            matches = native_digest_matches(expected, native, self.read_only_observations)
            if matches and self.read_only_observations and native["all_values_sha256"] != expected["all_values_sha256"]:
                self.counts["read_only_digest_difference_observations"] += 1
            self.journal.append("direct_parameter_digest_observation", transaction=transaction,
                                checkpoint=checkpoint, attempt=attempt, fresh_rpc=self.call_count,
                                matches=matches, consecutive_matches=attempt if matches else 0,
                                all_values_equal=native["all_values_sha256"] == expected["all_values_sha256"],
                                read_only_values=self.summary["read_only_values"],
                                **{key: native[key] for key in HASH_FIELDS},
                                atomic_snapshot=False, opaque_contents_verified=False)
            if not matches:
                current = self.snapshot(baseline.target, baseline)
                projection = self.direct_projection(current)
                diagnosis = self.direct_digest_read(baseline)
                parity = native_digest_matches(projection, diagnosis)
                frozen = self.direct_digest_baseline.projection
                value_field = "non_read_only_values_sha256" if self.read_only_observations else "all_values_sha256"
                baseline_matches = (projection["schema_sha256"] == frozen["schema_sha256"]
                                    and projection[value_field] == frozen[value_field])
                self.journal.append("direct_parameter_digest_mismatch_diagnosis", transaction=transaction,
                                    checkpoint=checkpoint, diagnostic_only=True, full_pages_read=True,
                                    native_parity_matches=parity,
                                    native_parity_reference="current_diagnostic_projection",
                                    diagnostic_projection_matches_frozen_baseline=baseline_matches,
                                    read_only_values=self.summary["read_only_values"],
                                    **{key: projection[key] for key in HASH_FIELDS},
                                    atomic_snapshot=False, opaque_contents_verified=False)
                raise DependencyFailure("Direct restoration did not preserve the frozen complete exposed provider projection")

    def direct_full_checkpoint(self, baseline, transaction, checkpoint):
        require(self.direct_digest_baseline is not None and self.direct_digest_baseline.provider is baseline,
                "Direct parameter digest baseline was not initialized for this fixture")
        self.fixture_check(baseline.target)
        current = self.snapshot(baseline.target, baseline)
        projection = self.direct_projection(current)
        require(native_digest_matches(self.direct_digest_baseline.projection,
                                      {**projection, "ok": True, "outcome": "observed"},
                                      self.read_only_observations),
                "Full direct-restoration checkpoint differs from the frozen baseline")
        self.direct_digest_pair(baseline, projection, transaction, checkpoint)
        self.journal.append("direct_parameter_digest_full_checkpoint", transaction=transaction,
                            checkpoint=checkpoint, full_pages_verified=True, count=len(current.records),
                            consecutive_fresh_digest_matches=2,
                            read_only_values=self.summary["read_only_values"],
                            **{key: projection[key] for key in HASH_FIELDS},
                            atomic_snapshot=False, opaque_contents_verified=False)

    def run_fixture(self, target, write_roundtrips=False, parameters=None):
        if write_roundtrips:
            self.track_parameter_filter(parameters)
        self.fixture_check(target, setup=True)
        if target["plugin"] == "Splitter":
            self.counts["separate_graph_workflow_required"] += 1
            self.journal.append("fixture_result", plugin=target["plugin"], status="separate_graph_workflow_required")
            self.summary["classes"].append({"plugin": target["plugin"], "status": "separate_graph_workflow_required"})
            self.progress()
            return
        if target["plugin"].startswith("External Effect "):
            self.call("plugin_editor_open", target, mutation=True)
        baseline = self.snapshot(target)
        if write_roundtrips:
            self.track_parameter_filter(parameters, baseline.records)
        key = f"provider-{len(self.summary['classes']):03d}"
        reference = self.journal.baseline(key, baseline.serialized())
        self.journal.append("provider_audit", plugin=target["plugin"], baseline=reference,
                            count=len(baseline.records), source=baseline.source, schema_sha256=baseline.schema)
        categories = Counter()
        eligible = []
        for parameter in baseline.records:
            category = classification(target["plugin"], parameter)
            categories[category] += 1
            self.counts["records_audited"] += 1
            self.counts["category_" + category] += 1
            self.journal.append("parameter_classification", plugin=target["plugin"], index=provider_index(parameter),
                                name=parameter["name"], category=category, advertised_writable=parameter.get("writable"))
            if category in ("numeric_scalar", "literal_string") and (not parameters or parameter["name"] in parameters):
                eligible.append(parameter)
        row = {"plugin": target["plugin"], "count": len(baseline.records), "source": baseline.source,
               "categories": dict(categories), "schema_sha256": baseline.schema, "baseline": reference,
               "status": "audited", "selected_for_write": len(eligible) if write_roundtrips else 0}
        self.summary["classes"].append(row)
        self.progress()
        if not write_roundtrips:
            return
        if self.direct_digest_mode:
            projection = self.direct_projection(baseline)
            self.direct_digest_baseline = DirectDigestBaseline(baseline, projection)
            row.update(parameter_verification="direct_restoration_with_native_parameter_digest",
                       consecutive_fresh_digest_matches_required=2, digest_fixture_complete=False,
                       full_page_checkpoint_max_cases=MAX_DIRECT_DIGEST_CASES_BETWEEN_FULL_CHECKS,
                       atomic_snapshot=False, opaque_contents_verified=False)
            self.direct_digest_pair(baseline, projection, key, "initial_full_baseline")
            self.direct_full_checkpoint(baseline, key, "before_first_write")
        if not eligible:
            if self.direct_digest_mode:
                self.direct_full_checkpoint(baseline, key, "fixture_end_no_eligible_scalars")
                row["digest_fixture_complete"] = True
            row["status"] = "no_selected_eligible_scalars"
            self.progress()
            return
        pending = []
        for index, parameter in enumerate(eligible):
            transaction = f"{key}-{provider_index(parameter):06d}"
            result = self.roundtrip(baseline, parameter, transaction)
            pending.append(result)
            if self.direct_digest_mode or len(pending) >= self.block_size or index == len(eligible) - 1:
                self.confirm_block(baseline, pending)
                pending.clear()
            if (self.direct_digest_mode and (index + 1) % MAX_DIRECT_DIGEST_CASES_BETWEEN_FULL_CHECKS == 0
                    and index + 1 < len(eligible)):
                self.direct_full_checkpoint(baseline, key, "completed_cases_" + str(index + 1))
        if self.direct_digest_mode:
            self.direct_full_checkpoint(baseline, key, "fixture_end")
            row["digest_fixture_complete"] = True
        row["status"] = "eligible_scalar_roundtrips_completed"
        self.progress()


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default="target/release/fender-studio-pro-mcp")
    parser.add_argument("--fixtures", required=True)
    parser.add_argument("--scratch-title", required=True)
    parser.add_argument("--output-dir", default=".local/live/all-parameters-" + datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ"))
    parser.add_argument("--pause", action="store_true", help="Start the MCP MIDI owner before opening the DAW, then press Enter")
    modes = parser.add_mutually_exclusive_group(required=True)
    modes.add_argument("--audit-only", action="store_true", help="Read every native record; Music selection and opt-in fixture preparation can still change scratch track state")
    modes.add_argument("--write-roundtrips", action="store_true", help="Change/read/restore every eligible reversible scalar; dedicated workflows remain explicitly incomplete")
    parser.add_argument("--prepare-midi-fixtures", action="store_true")
    parser.add_argument("--block-size", type=int, default=16, help="Whole-provider state comparison interval, from 1 to 64")
    parser.add_argument("--read-only-observations", action="store_true",
                        help="Observe drifting native read-only values without claiming their restoration; requires complete native access proof before and after")
    parser.add_argument("--use-parameter-digest", action="store_true",
                        help="With --block-size 1, verify each direct restoration by two fresh native digests and periodic complete public pages")
    parser.add_argument("--plugin", action="append", help="Limit to an exact plugin name; repeat to select more")
    parser.add_argument("--parameter", action="append", help="Limit writes to an exact parameter name; all records are still audited")
    args = parser.parse_args(argv)
    if not 1 <= args.block_size <= 64:
        parser.error("--block-size must be from 1 to 64")
    if args.use_parameter_digest and args.block_size != 1:
        parser.error("--use-parameter-digest requires --block-size 1")
    fixtures = json.loads(Path(args.fixtures).read_text())
    require(isinstance(fixtures, list) and 0 < len(fixtures) <= 128, "Expected 1..128 fixture targets")
    targets = []
    for fixture in fixtures:
        require(isinstance(fixture, dict) and isinstance(fixture.get("plugin"), str), "Invalid fixture target")
        target = {key: value for key, value in fixture.items() if key not in ("test_parameter", "test_bypass")}
        if not args.plugin or target["plugin"] in args.plugin:
            targets.append(target)
    require(targets, "No fixture matched the requested scope")
    journal = Journal(args.output_dir)
    client = None
    runner = None
    try:
        binary = Path(args.binary).resolve()
        journal.append("run_started", schema=1, mode="audit" if args.audit_only else "write_roundtrips",
                       binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                       scope={"plugins": args.plugin, "parameters": args.parameter},
                       guarantee="independent parameter roundtrip plus whole-provider block comparison; no coupled-state workflow",
                       parameter_verification="direct_restoration_with_native_parameter_digest" if args.use_parameter_digest else "full_pages",
                       read_only_values="observed_not_restored" if args.read_only_observations else "exact_comparison")
        client = McpClient([str(binary), "serve"], timeout=45)
        print("MCP owner ready", flush=True)
        if args.pause:
            input("Open exactly the named scratch song, then press Enter: ")
        runner = Runner(client, journal, args.scratch_title, args.prepare_midi_fixtures, args.block_size,
                        args.read_only_observations, args.use_parameter_digest)
        runner.summary["scope"] = {"plugins": args.plugin, "parameters": args.parameter}
        runner.summary["mode"] = "audit" if args.audit_only else "write_roundtrips"
        runner.initialize()
        for index, target in enumerate(targets, 1):
            print(f"[{index}/{len(targets)}] {target['plugin']}", flush=True)
            runner.run_fixture(target, args.write_roundtrips, set(args.parameter or []))
        exit_code = runner.finish(args.write_roundtrips)
        print(json.dumps({"status": runner.summary["status"], "counts": dict(runner.counts),
                          "eligible_roundtrips_verified": runner.summary["eligible_roundtrips_verified"],
                          "all_writable_parameters_verified": False}), flush=True)
        return exit_code
    except Exception as error:
        journal.append("run_failed", error_type=type(error).__name__, message=str(error))
        if runner is not None:
            runner.summary.update(status="failed", error_type=type(error).__name__, error=str(error), completed_at=utc_now())
            runner.progress()
        print(f"Acceptance stopped: {type(error).__name__}: {error}", file=sys.stderr, flush=True)
        return 1
    finally:
        if client is not None:
            client.close()
        journal.close()


if __name__ == "__main__":
    sys.exit(main())
