#!/usr/bin/env python3
"""Verify one observed Impact focus-cutoff binding in an empty scratch fixture.

Only focus.filter.cutoff is written. Its explicit pad peer is discovered from
two complete changed projections. All exposed values must return exactly to
the immutable original projection. No other focus aliases are established.
"""
import argparse
from copy import deepcopy
from dataclasses import dataclass
from datetime import datetime, timezone
import hashlib
import json
import math
from pathlib import Path
import struct
import sys

from mcp_client import McpClient
from parameter_digest import DigestProjectionError, project_snapshot
from verify_all_parameters import (
    AcceptanceFailure, ContextChanged, Journal, RestorationFailure, Runner,
    UnknownMutation, decode_float_bits, encoded, parameter_schema, require, utc_now,
    value_identity,
)


ALIAS = "focus.filter.cutoff"
PEERS = tuple(f"pad[{index}].filter.cutoff" for index in range(128))
IMPACT_CLASS = "3713E26C-2FCA-4024-9F25-17E9D2BE2B9B"
PROVIDER_COUNT = 4262
MAIN_LABEL = "메인"


def observation(parameter):
    return value_identity(parameter), parameter["native_normalized"], parameter["text"]


def cutoff_candidate(parameter):
    require(parameter.get("name") == ALIAS and parameter.get("native_kind") == "Float"
            and parameter.get("support", {}).get("raw") is True
            and parameter.get("support", {}).get("normalized") is True
            and parameter.get("native_support", {}).get("raw_number") is True
            and parameter.get("native_support", {}).get("normalized") is True,
            "Use the observed raw and normalized Float cutoff profile")

    def exact_float(field):
        native = parameter.get("native_value" if field == "value" else "native_" + field)
        require(isinstance(native, dict) and set(native) == {"variant_tag", "data"}
                and type(native.get("variant_tag")) is int and native["variant_tag"] == 2
                and isinstance(native.get("data"), dict)
                and set(native["data"]) == {"kind", "value", "bits_hex"}
                and native["data"]["kind"] == "float",
                "Cutoff requires its exact native Float carrier for " + field)
        value = decode_float_bits(native["data"]["bits_hex"])
        require(all(type(shadow) in (int, float) and math.isfinite(shadow) and shadow == value
                    for shadow in (native["data"]["value"], parameter.get(field))),
                "Cutoff requires exact finite native and public " + field)
        return value

    original, lower, upper = (exact_float(field) for field in ("value", "min", "max"))
    require(struct.pack(">d", lower).hex() == "0000000000000000" and upper == 10000.0
            and lower <= original <= upper,
            "Cutoff requires the observed finite native raw bounds 0..10000")
    normalized = parameter.get("native_normalized")
    require(isinstance(normalized, dict) and set(normalized) == {"kind", "bits_hex"}
            and normalized.get("kind") == "float32"
            and isinstance(normalized.get("bits_hex"), str) and len(normalized["bits_hex"]) == 8
            and all(character in "0123456789abcdefABCDEF" for character in normalized["bits_hex"]),
            "Cutoff requires exact native Float32 normalized bits")
    normalized_value = struct.unpack(">f", bytes.fromhex(normalized["bits_hex"]))[0]
    require(math.isfinite(normalized_value) and type(parameter.get("normalized")) in (int, float)
            and parameter["normalized"] == normalized_value
            and struct.pack(">f", (original - lower) / (upper - lower)).hex().upper()
            == normalized["bits_hex"].upper(),
            "Cutoff requires its exact linear Float32 normalized shadow")
    original_bits = parameter["native_value"]["data"]["bits_hex"].upper()
    for fraction in (0.5, 0.25, 0.75):
        desired = lower + (upper - lower) * fraction
        bits = struct.pack(">d", desired).hex().upper()
        if bits != original_bits:
            return {"float_bits": bits}, {"float_bits": original_bits}
    raise AcceptanceFailure("No different fixed cutoff binary-fraction candidate exists")


@dataclass(frozen=True)
class FocusBinding:
    alias: str
    peer: str
    pad_index: int


class ImpactFocusRunner(Runner):
    def __init__(self, client, journal, scratch_title, use_fixture_context=False):
        super().__init__(client, journal, scratch_title, use_fixture_context=use_fixture_context)
        self.started = False
        self.summary.update(mode="one_impact_focus_cutoff_binding", fixtures=[],
                            parameter_verification="two_fresh_complete_typed_public_projections",
                            restoration="one_exact_alias_original_write", atomic_snapshot=False,
                            all_writable_parameters_verified=False, all_focus_aliases_verified=False,
                            all_focus_selections_verified=False, opaque_contents_verified=False,
                            description_validation="fresh_typed_values_with_frozen_indexed_metadata",
                            output_guard="fresh_exact_scratch_main_zero_before_every_setter")

    def call(self, name, arguments=None, mutation=False, allow_error=False):
        require(not mutation or (name == "plugin_parameter_set" and isinstance(arguments, dict)
                                and arguments.get("param") == ALIAS),
                "The fixed pilot permits only its cutoff-alias parameter setter")
        try:
            receipt = super().call(name, arguments, mutation, allow_error=True)
        except (UnknownMutation, ContextChanged):
            raise
        except Exception as error:
            raise UnknownMutation("Focus request evidence is unavailable; no follow-on write is permitted") from error
        result = receipt.get("result")
        nested = result if isinstance(result, dict) else {}
        if receipt.get("error") is not None or any(nested.get(key) is not None for key in ("error", "native_error")):
            if not (mutation and allow_error and receipt["outcome"] == "not_applied"):
                raise UnknownMutation("Focus request has error evidence; no follow-on write is permitted")
        if mutation:
            if receipt["outcome"] not in ("applied_verified", "applied_unverified", "not_applied"):
                raise UnknownMutation("Focus setter has no known mutation outcome")
            if any(nested[key] not in ("applied_verified", "applied_unverified", "not_applied")
                   for key in ("outcome", "native_outcome") if key in nested):
                raise UnknownMutation("Focus setter has conflicting native outcome evidence")
            if any((nested[key] == "not_applied") != (receipt["outcome"] == "not_applied")
                   for key in ("outcome", "native_outcome") if key in nested):
                raise UnknownMutation("Focus setter has conflicting applied and refused outcomes")
        elif receipt["outcome"] != "observed":
            raise UnknownMutation("Focus read did not establish fresh observed evidence")
        return receipt

    def fixture_check(self, target, setup=False):
        context_mode = self.use_fixture_context
        try:
            if setup:
                self.use_fixture_context = False
            return super().fixture_check(target, setup=False)
        except (UnknownMutation, ContextChanged):
            raise
        except Exception as error:
            raise ContextChanged("Focus scratch fixture or stopped transport changed") from error
        finally:
            self.use_fixture_context = context_mode

    def projection(self, provider, expected=None):
        try:
            projected = project_snapshot(provider)
        except DigestProjectionError as error:
            raise ContextChanged("Complete exact Impact metadata is unavailable") from error
        if not (provider.source == "dsp" and provider.instance["class_uid"] == IMPACT_CLASS
                and len(provider.records) == PROVIDER_COUNT and projected["read_only_count"] == 0
                and projected["excluded_opaque_count"] == 0):
            raise ContextChanged("Use the exact complete writable Impact DSP provider")
        if not all(parameter.get("access_metadata_complete") is True
                   and parameter.get("enabled") is True and parameter.get("writable") is True
                   and parameter.get("readOnly") is False and parameter.get("support", {}).get("raw") is True
                   for parameter in provider.records):
            raise ContextChanged("Every Impact record requires complete enabled raw access")
        if expected is not None and (projected["schema_sha256"] != expected["schema_sha256"]
                                     or projected["instance"] != expected["instance"]
                                     or projected["source"] != expected["source"]):
            raise ContextChanged("Impact source, identity or complete schema changed")
        return projected

    def snapshot(self, target, previous=None):
        try:
            return super().snapshot(target, previous)
        except (UnknownMutation, ContextChanged):
            raise
        except Exception as error:
            raise ContextChanged("Complete Impact provider identity and indexed pages were not established") from error

    def read_parameter(self, provider, original):
        try:
            result = self.call("plugin_parameters", {**provider.target, "source": provider.source,
                               "names": [original["name"]], "expected_instance": provider.instance["object_uid"]})["result"]
            require(isinstance(result, dict) and set(result) <= {
                "channel", "slot", "plugin", "params", "missing", "source", "instance",
                "ok", "outcome", "native_outcome", "error", "native_error", "errors"},
                "Named cutoff read has an unsupported envelope")
            require(result.get("plugin") == provider.target["plugin"]
                    and all(result[key] == provider.target.get(key) for key in ("channel", "slot") if key in result)
                    and ("source" not in result or result["source"] == provider.source)
                    and ("instance" not in result or encoded(result["instance"]) == encoded(provider.instance))
                    and ("ok" not in result or result["ok"] is True)
                    and all(result[key] == "observed" for key in ("outcome", "native_outcome") if key in result)
                    and all(result.get(key) is None for key in ("error", "native_error"))
                    and ("errors" not in result or type(result["errors"]) is list and result["errors"] == []),
                    "Named cutoff read contradicts the bound observed provider")
            require(result.get("missing") == [] and isinstance(result.get("params"), list)
                    and len(result["params"]) == 1, "Named cutoff read did not return one complete parameter")
            return self.validated_description(provider, original, result["params"][0])
        except (UnknownMutation, ContextChanged):
            raise
        except Exception as error:
            raise ContextChanged("Impact cutoff parameter identity and exact typed observation were not established") from error

    def validated_description(self, provider, original, fresh):
        try:
            index = original["native"]["index"]
            fields = set(original["native"]["native_description"])
            require(isinstance(fresh, dict) and fields <= set(fresh)
                    and set(fresh) <= fields | {"native", "access_metadata_complete", "index"},
                    "Cutoff observation is missing a complete native description")
            require(type(index) is int and 0 <= index < len(provider.records)
                    and provider.records[index]["name"] == original["name"]
                    and parameter_schema(fresh) == parameter_schema(original),
                    "Cutoff observation does not match its frozen parameter schema")
            require(("native" not in fresh or encoded(fresh["native"]) == encoded(original["native"]))
                    and ("access_metadata_complete" not in fresh or fresh["access_metadata_complete"] is True)
                    and ("index" not in fresh or type(fresh["index"]) is int and fresh["index"] == index),
                    "Cutoff observation contradicts its frozen indexed metadata")
            description = {key: deepcopy(fresh[key]) for key in fields}
            require(type(description["native_value"].get("variant_tag")) is int
                    and value_identity(description)[:2] == value_identity(original)[:2]
                    and description["native_normalized"].get("kind") == original["native_normalized"].get("kind"),
                    "Cutoff observation changed its native value carrier")
            indexed = deepcopy(original)
            indexed.update(description)
            indexed["native"]["native_description"] = deepcopy(description)
            records = list(provider.records)
            records[index] = indexed
            projected = project_snapshot({"source": provider.source, "instance": provider.instance, "records": records})
            require(projected["schema_sha256"] == self.projection(provider)["schema_sha256"],
                    "Cutoff observation changed exact native schema")
            return description
        except (UnknownMutation, ContextChanged):
            raise
        except Exception as error:
            raise ContextChanged("Cutoff observation has malformed or contradictory exact typed evidence") from error

    def full_read(self, baseline, expected, transaction, checkpoint, attempt):
        self.fixture_check(baseline.target)
        current = self.snapshot(baseline.target, baseline)
        projected = self.projection(current, expected)
        self.fixture_check(baseline.target)
        self.journal.append("focus_complete_projection", transaction=transaction, checkpoint=checkpoint,
                            attempt=attempt, count=PROVIDER_COUNT, schema_sha256=projected["schema_sha256"],
                            all_values_sha256=projected["all_values_sha256"],
                            atomic_snapshot=False, opaque_contents_verified=False)
        return current, projected

    def confirm_original(self, baseline, expected, transaction, checkpoint):
        for attempt in (1, 2):
            _, current = self.full_read(baseline, expected, transaction, checkpoint, attempt)
            require(current == expected, "Complete Impact typed projection differs from its immutable original")

    def write(self, provider, parameter, values):
        require(parameter["name"] == ALIAS, "The fixed focus pilot may write only the cutoff alias")
        self.fixture_check(provider.target)
        channels = self.operation("channels")
        if not isinstance(channels, list):
            raise ContextChanged("Scratch Main channels are unavailable")
        main = [row for row in channels if isinstance(row, dict) and row.get("label") == MAIN_LABEL]
        if not (len(main) == 1 and type(main[0].get("volume")) in (int, float)
                and main[0]["volume"] == 0 and "output" in main[0] and main[0]["output"] is None):
            raise ContextChanged("Exact scratch Main must remain at zero before every setter")
        self.journal.append("zero_main_output_observed", volume=0, output="scratch_main")
        self.fixture_check(provider.target)
        return super().write(provider, parameter, values)

    def applied_after(self, receipt, baseline, original, *, restoring=False, expected_before=None):
        result = receipt.get("result")
        if not (isinstance(result, dict) and result.get("source") == baseline.source
                and result.get("instance") == baseline.instance
                and ("ok" not in result or result["ok"] is True)
                and ("param" not in result or result["param"] == original["name"])
                and ("errors" not in result or type(result["errors"]) is list and result["errors"] == [])):
            raise ContextChanged("Focus setter did not preserve the exact native provider")
        require(receipt["outcome"] in ("applied_verified", "applied_unverified"), "Focus change was refused")
        if restoring:
            require(receipt["outcome"] == "applied_verified", "Focus original setter did not verify")
        before = self.validated_description(baseline, original, result.get("before"))
        try:
            same_before = observation(before) == observation(original if expected_before is None else expected_before)
        except Exception as error:
            raise ContextChanged("Focus setter before-value evidence is incomplete") from error
        if not same_before:
            raise ContextChanged("Focus alias drifted between its frozen observation and setter dispatch")
        return self.validated_description(baseline, original, result.get("after"))

    def find_binding(self, baseline, current, changed_alias):
        before = {parameter["name"]: parameter for parameter in baseline.records}
        after = {parameter["name"]: parameter for parameter in current.records}
        changed = {name for name in before if observation(before[name]) != observation(after[name])}
        peers = changed.intersection(PEERS)
        require(len(peers) == 1 and changed == {ALIAS, *peers},
                "Changed Impact state must contain only the alias and exactly one explicit cutoff peer")
        peer = next(iter(peers))
        identity = value_identity(changed_alias)
        require(value_identity(after[ALIAS]) == identity == value_identity(after[peer]),
                "Alias and discovered peer do not share the exact changed native carrier and bits")
        require(identity != value_identity(before[ALIAS]) and identity != value_identity(before[peer]),
                "Alias and discovered peer must each change from their own original bits")
        require(value_identity(before[ALIAS]) == value_identity(before[peer]),
                "Observed alias peer has a different immutable own-original native value")
        return FocusBinding(ALIAS, peer, PEERS.index(peer))

    def restore_original(self, baseline, expected, original, arguments, transaction):
        current, _ = self.full_read(baseline, expected, transaction, "before_restore", 1)
        before = next(parameter for parameter in current.records if parameter["name"] == ALIAS)
        self.phase(transaction, "RestoreInFlight")
        restored = self.write(baseline, original, arguments)
        after = self.applied_after(restored, baseline, original, restoring=True, expected_before=before)
        require(value_identity(after) == value_identity(original), "Focus original setter returned different exact bits")
        self.phase(transaction, "Restored")
        self.confirm_original(baseline, expected, transaction, "restored")
        self.phase(transaction, "StateConfirmed", complete_provider_projection_restored=True,
                   consecutive_complete_restored_reads=2, atomic_snapshot=False)

    def run_fixture(self, target):
        require(not self.started, "The fixed focus pilot can run only once per runner")
        self.started = True
        require(isinstance(target, dict) and set(target) == {"kind", "plugin", "track"}
                and target.get("kind") == "instrument" and target.get("plugin") == "Impact"
                and isinstance(target.get("track"), str) and target["track"].startswith("FSP Verify "),
                "Use one exact named empty Impact instrument fixture")
        self.fixture_context_provider = None
        self.fixture_check(target, setup=True)
        baseline = self.snapshot(target)
        expected = self.projection(baseline)
        self.fixture_context_provider = baseline
        originals = {parameter["name"]: parameter for parameter in baseline.records}
        require(ALIAS in originals and all(name in originals for name in PEERS),
                "Impact requires the fixed cutoff alias and all128 explicit cutoff peers")
        original = originals[ALIAS]
        identity = value_identity(original)
        require(original["native_kind"] == "Float" and identity[1] == "float" and identity[0] & 15 == 2,
                "The cutoff alias requires its exact native Float carrier")
        change_arguments, restore_arguments = cutoff_candidate(original)
        reference = self.journal.baseline("impact-focus-original", baseline.serialized())
        transaction = "impact-focus-cutoff"
        row = {"plugin": "Impact", "parameter": ALIAS, "baseline": reference, "status": "prepared",
               "complete_provider_projection_restored": False, "binding_verified": False}
        self.summary["fixtures"].append(row)
        self.phase(transaction, "Prepared", baseline=reference, requested=change_arguments)
        self.progress()
        self.confirm_original(baseline, expected, transaction, "original")
        self.phase(transaction, "ChangeInFlight")
        restoring = False
        try:
            receipt = self.write(baseline, original, change_arguments)
            if receipt["outcome"] == "not_applied":
                row["status"] = "write_refused"
                try:
                    self.confirm_original(baseline, expected, transaction, "refused")
                except (UnknownMutation, ContextChanged):
                    raise
                except Exception as error:
                    raise UnknownMutation("Refused focus setter contradicted its unchanged full projection") from error
                self.phase(transaction, "NoWriteConfirmed")
                self.progress()
                raise AcceptanceFailure("Focus setter was refused; no binding was established")
            self.phase(transaction, "Changed", reported_outcome=receipt["outcome"])
            after = self.applied_after(receipt, baseline, original)
            changed_alias, settlement = self.confirm_changed_readback(baseline, original, after)
            projections, bindings = [], []
            for attempt in (1, 2):
                current, projected = self.full_read(baseline, expected, transaction, "changed", attempt)
                bindings.append(self.find_binding(baseline, current, changed_alias))
                projections.append(projected)
            require(projections[0] == projections[1] and bindings[0] == bindings[1],
                    "Impact alias binding or full changed typed values are unstable")
            binding = bindings[0]
            row.update(binding_verified=True, peer=binding.peer, pad_index=binding.pad_index,
                       settlement=settlement, consecutive_complete_changed_reads=2)
            self.phase(transaction, "ChangedReadbackConfirmed", alias=ALIAS, peer=binding.peer,
                       pad_index=binding.pad_index, consecutive_complete_changed_reads=2)
            restoring = True
            self.restore_original(baseline, expected, original, restore_arguments, transaction)
        except (UnknownMutation, ContextChanged):
            row["status"] = "unknown_or_context_changed"
            self.phase(transaction, "UnknownAfterDispatch")
            self.progress()
            raise
        except Exception as error:
            if row["status"] == "write_refused":
                raise
            if not restoring:
                restoring = True
                try:
                    self.restore_original(baseline, expected, original, restore_arguments, transaction)
                except (UnknownMutation, ContextChanged):
                    row["status"] = "unknown_or_context_changed"
                    self.phase(transaction, "UnknownAfterDispatch")
                    self.progress()
                    raise
                except Exception as restore_error:
                    row["status"] = "restoration_failed"
                    self.phase(transaction, "RestorationFailed", error_type=type(restore_error).__name__)
                    self.progress()
                    raise RestorationFailure("Single focus original restoration failed; no retry is permitted") from restore_error
                row.update(status="binding_unconfirmed_restored", complete_provider_projection_restored=True)
                self.progress()
                raise error
            row["status"] = "restoration_failed"
            self.phase(transaction, "RestorationFailed", error_type=type(error).__name__)
            self.progress()
            raise RestorationFailure("Focus restoration failed; no retry is permitted") from error
        row.update(status="one_focus_binding_changed_readback_restored", complete_provider_projection_restored=True,
                   consecutive_complete_restored_reads=2)
        self.counts["focus_binding_changed_readback_restored"] += 1
        self.journal.append("focus_binding_result", **row)
        self.progress()
        return row

    def finish(self):
        verified = len(self.summary["fixtures"]) == 1 and self.summary["fixtures"][0]["status"] == "one_focus_binding_changed_readback_restored"
        self.summary.update(status="one_focus_binding_verified" if verified else "focus_binding_not_verified",
                            completed_at=utc_now())
        self.progress()
        self.journal.append("run_completed", summary=self.summary)
        return 0 if verified else 2


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default="target/release/fender-studio-pro-mcp")
    parser.add_argument("--fixtures", required=True)
    parser.add_argument("--scratch-title", required=True)
    parser.add_argument("--run-pilot", action="store_true", required=True)
    parser.add_argument("--use-fixture-context", action="store_true")
    parser.add_argument("--output-dir", default=".local/live/impact-focus-" + datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ"))
    parser.add_argument("--pause", action="store_true")
    args = parser.parse_args(argv)
    fixtures = json.loads(Path(args.fixtures).read_text())
    require(isinstance(fixtures, list) and 0 < len(fixtures) <= 128
            and all(isinstance(item, dict) for item in fixtures), "Use a finite fixture list")
    selected = [item for item in fixtures if item.get("plugin") == "Impact"]
    require(len(selected) == 1, "Use exactly one Impact fixture")
    target = {key: value for key, value in selected[0].items() if not key.startswith("test_")}
    journal = Journal(args.output_dir)
    client = runner = None
    try:
        binary = Path(args.binary).resolve()
        journal.append("run_started", schema=1, mode="one_impact_focus_cutoff_binding",
                       binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest())
        client = McpClient([str(binary), "serve"], timeout=45)
        print("MCP owner ready", flush=True)
        if args.pause:
            input("Open the named empty muted Impact scratch fixture with Main at zero, then press Enter: ")
        runner = ImpactFocusRunner(client, journal, args.scratch_title, args.use_fixture_context)
        runner.initialize()
        runner.run_fixture(target)
        return runner.finish()
    except Exception as error:
        journal.append("run_failed", error_type=type(error).__name__, message=str(error))
        if runner is not None:
            runner.summary.update(status="failed", error_type=type(error).__name__, completed_at=utc_now())
            runner.progress()
        print("Acceptance stopped: " + type(error).__name__ + ": " + str(error), file=sys.stderr, flush=True)
        return 1
    finally:
        if client is not None:
            client.close()
        journal.close()


if __name__ == "__main__":
    sys.exit(main())
