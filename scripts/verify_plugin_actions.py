#!/usr/bin/env python3
"""Verify registered filter-swap pulses and their conditional inverse in scratch fixtures.

Each pulse remains natively unverified. Two complete independent reads must
prove the exact filter permutation before one inverse pulse is allowed. No
automatic inverse follows unknown dispatch or an unconfirmed effect.
"""
import argparse
from dataclasses import dataclass
from datetime import datetime, timezone
import hashlib
import json
import math
from pathlib import Path
import struct
import sys
import time

from mcp_client import McpClient
from parameter_digest import DigestProjectionError, project_snapshot
from verify_all_parameters import (
    AcceptanceFailure, ContextChanged, Journal, RestorationFailure, UnknownMutation,
    raw_value, require, utc_now, value_identity,
)
from verify_plugin_states import StateRunner, validate_settlement


ACTION = "swap_filter_frequencies"


@dataclass(frozen=True)
class ActionSpec:
    class_uid: str
    parameter: str
    effects: tuple[str, str]


SPECS = {
    "Expander": ActionSpec("831A0046-BD71-45BF-A419-6FC90D6E9F2F", "swapfreqs", ("filterlow", "filterhigh")),
    "Gate": ActionSpec("132C0E1F-06FD-4C96-BD65-678C04FE1C92", "swapfreqs", ("filterlow", "filterhigh")),
    "Compressor": ActionSpec("36F3F4D1-CBB4-4BF7-A7E3-EBCCED53718B", "studiocomp.swapfreqs",
                             ("studiocomp.sidechainfreqlow", "studiocomp.sidechainfreqhigh")),
}

LEVEL_RESET_SPECS = {
    "reset_meter_hold": ("resethold", 2, 1383298120),
    "reset_meter_loudness": ("resetloudness", 4, 1383298124),
}


def require_effect_witness(action, plugins):
    if action in LEVEL_RESET_SPECS:
        require(plugins == ["Level Meter"], "Meter resets require only the exact Level Meter fixture")
        raise AcceptanceFailure(
            "Meter reset verification is blocked before dispatch: an independent non-neutral "
            "hold or integrated-loudness accumulator witness is unavailable. Toggle reset, ACK "
            "and unchanged DSP settings cannot prove the effect or restore reset history.")
    require(action == ACTION and all(plugin in SPECS for plugin in plugins),
            "Filter swaps require only the registered Expander, Gate and Compressor fixtures")


def explicitly_read_only(*parameters):
    return all(p.get("access_metadata_complete") is True and p.get("readOnly") is True
               and p.get("writable") is False for p in parameters)


def observation_identity(parameter):
    normalized = parameter["native_normalized"]
    normalized = (normalized["kind"], normalized.get("bits_hex", "").upper())
    return value_identity(parameter), normalized, parameter["text"]


def pulse_state(parameter):
    descriptor = parameter["native"]["native_description"]
    return {key: descriptor[key] for key in ("name", "tag", "native_kind", "enabled", "readOnly",
                                            "native_value", "native_min", "native_max")}


def numeric_value(parameter):
    tag, kind, _ = value_identity(parameter)
    require(type(tag) is int and -(1 << 15) <= tag < (1 << 15)
            and ((kind == "integer" and tag & 15 == 1) or (kind == "float" and tag & 15 == 2)),
            "Action controls require a matching exact numeric carrier")
    value = raw_value(parameter)
    require(type(value) in (int, float) and math.isfinite(value), "Action controls require finite numeric values")
    return value


def numeric_bound(parameter, field):
    return numeric_value({"native_value": parameter["native_" + field], "value": parameter[field]})


def validate_pulse_state(observed, expected):
    require(isinstance(observed, dict) and set(observed) == set(expected)
            and type(observed.get("tag")) is int
            and all(type(observed.get(flag)) is bool for flag in ("enabled", "readOnly")),
            "Native button state metadata is malformed")
    for field in ("native_value", "native_min", "native_max"):
        native = observed.get(field)
        require(isinstance(native, dict) and isinstance(native.get("data"), dict),
                "Native button state lacks an exact numeric value")
        numeric_value({"native_value": native, "value": native["data"].get("value")})
    require(observed == expected, "Native button pulse did not preserve the exact idle parameter state")


class ActionRunner(StateRunner):
    def __init__(self, client, journal, scratch_title, prepare_midi=False,
                 read_only_observations=False, settle_interval=0.1):
        require(type(read_only_observations) is bool, "Read-only observations require an explicit boolean mode")
        super().__init__(client, journal, scratch_title, prepare_midi,
                         settle_attempts=2, settle_interval=settle_interval,
                         read_only_observations=read_only_observations)
        self.summary.update(mode="registered_button_actions", action=ACTION,
                            parameter_verification="two_complete_public_page_observations_with_exact_projection",
                            inverse_policy="one_inverse_only_after_two_verified_permutation_reads",
                            native_effects_verified=False, audible_effects_verified=False,
                            atomic_snapshot=False)

    def projection(self, provider, baseline=None, expected=None):
        try:
            projected = project_snapshot(provider)
        except DigestProjectionError as error:
            raise AcceptanceFailure("Complete exact action projection is unavailable: " + str(error)) from error
        if baseline is not None:
            if provider.instance != baseline.instance or provider.source != baseline.source:
                raise ContextChanged("Action observation resolved a different native provider")
            if (projected["count"] != expected["count"]
                    or projected["schema_sha256"] != expected["schema_sha256"]):
                raise ContextChanged("Action changed the complete typed provider schema")
        return projected

    def value_hash(self, projection):
        field = "non_read_only_values_sha256" if self.read_only_observations else "all_values_sha256"
        return projection[field]

    def validate_baseline(self, baseline, spec):
        require(baseline.source == "dsp" and baseline.instance["class_uid"] == spec.class_uid,
                "Action requires the registered exact native class and direct DSP provider")
        parameters = {parameter["name"]: parameter for parameter in baseline.records}
        require(all(name in parameters for name in (spec.parameter, *spec.effects)),
                "Registered action or effect parameters are unavailable")
        for name in (spec.parameter, *spec.effects):
            parameter = parameters[name]
            require(parameter.get("access_metadata_complete") is True and parameter.get("enabled") is True
                    and parameter.get("readOnly") is False and parameter.get("writable") is True
                    and parameter.get("support", {}).get("raw") is True,
                    "Registered action and effects require complete enabled writable access")
            numeric_value(parameter)
        button = parameters[spec.parameter]
        require(button["native_kind"] == "Toggle" and numeric_bound(button, "min") == 0
                and numeric_bound(button, "max") == 1
                and raw_value(button) == 0, "Action must be an idle binary Toggle")
        if button["native_value"]["data"]["kind"] == "float":
            minimum = button["native_min"]["data"]
            reset_bits = (minimum["bits_hex"].upper() if minimum["kind"] == "float"
                          else struct.pack(">d", 0.0).hex().upper())
            require(button["native_value"]["data"]["bits_hex"].upper() == reset_bits,
                    "Idle button value must match its reset representation before dispatch")
        low, high = (parameters[name] for name in spec.effects)
        require(all(p["native_kind"] in ("Float", "Integer") for p in (low, high)),
                "Filter effects require ordinary native numeric parameters")
        require(numeric_value(low) != numeric_value(high), "Equal filter values cannot prove a swap; no preparation write is permitted")
        for destination, source in ((low, high), (high, low)):
            lower, upper = (numeric_bound(destination, field) for field in ("min", "max"))
            require(lower <= numeric_value(source) <= upper,
                    "The filter permutation must fit both captured native ranges")
        return parameters

    def dispatch(self, baseline, spec, transaction, inverse):
        self.fixture_check(baseline.target)
        self.phase(transaction, "InverseInFlight" if inverse else "ChangeInFlight", action=ACTION)
        self.counts["inverse_dispatch_attempts" if inverse else "action_dispatch_attempts"] += 1
        receipt = self.call("plugin_action", {**baseline.target, "action": ACTION,
                            "expected_instance": baseline.instance["object_uid"]}, mutation=True, allow_error=True)
        result = receipt.get("result")
        if isinstance(result, dict) and (result.get("outcome") in ("unknown", "outcome_unknown")
                                        or result.get("native_outcome") in ("unknown", "outcome_unknown")):
            raise UnknownMutation("Native button outcome is unknown; no inverse or retry is permitted")
        if receipt.get("outcome") == "not_applied":
            if isinstance(result, dict):
                applied = any(result.get(field) in ("applied_verified", "applied_unverified")
                              for field in ("outcome", "native_outcome"))
                phase_evidence = any(field in result and result[field] is not False for field in
                                     ("upper_dispatched", "reset_dispatched", "reset_verified", "effects_verified", "verified"))
                if applied or phase_evidence:
                    raise UnknownMutation("Refused button receipt has contradictory or malformed dispatch evidence; no inverse or retry is permitted")
            return False
        require(receipt.get("outcome") == "applied_unverified" and not receipt.get("error")
                and isinstance(result, dict), "Button dispatch did not establish a known unverified applied outcome")
        if result.get("instance") != baseline.instance:
            raise ContextChanged("Button receipt resolved a different native Device")
        require(result.get("ok") is True and result.get("source") == "dsp"
                and result.get("outcome") == "applied_unverified"
                and result.get("native_outcome") == "applied_unverified"
                and result.get("action") == ACTION and result.get("param") == spec.parameter
                and result.get("effect_parameters") == list(spec.effects),
                "Button receipt differs from the fixed native action contract")
        require(all(result.get(flag) is True for flag in ("upper_dispatched", "reset_dispatched", "reset_verified"))
                and result.get("verified") is False and result.get("effects_verified") is False,
                "Button reset evidence cannot certify its musical effect")
        button = next(parameter for parameter in baseline.records if parameter["name"] == spec.parameter)
        validate_pulse_state(result.get("before"), pulse_state(button))
        validate_pulse_state(result.get("after"), pulse_state(button))
        self.phase(transaction, "InverseAccepted" if inverse else "Changed", native_effects_verified=False,
                   native_reset_verified=True, native_outcome="applied_unverified")
        return True

    def observe(self, baseline, expected, transaction, stage):
        self.fixture_check(baseline.target)
        current = self.snapshot(baseline.target, baseline)
        projected = self.projection(current, baseline, expected)
        self.journal.baseline(transaction + "-" + stage, current.serialized())
        return current, projected

    def confirm_baseline(self, baseline, expected, transaction, stage):
        for attempt in (1, 2):
            if attempt == 2 and self.settle_interval:
                time.sleep(self.settle_interval)
            current, projected = self.observe(baseline, expected, transaction, stage + "-" + str(attempt))
            require(self.value_hash(projected) == self.value_hash(expected),
                    "Complete provider projection differs from the immutable action baseline")
            self.journal.append("action_baseline_observation", transaction=transaction, stage=stage,
                                attempt=attempt, matches=True, consecutive_matches=attempt,
                                schema_sha256=projected["schema_sha256"],
                                read_only_values=self.summary["read_only_values"],
                                all_values_equal=projected["all_values_sha256"] == expected["all_values_sha256"],
                                atomic_snapshot=False, opaque_contents_verified=False)
        return current

    def confirm_permutation(self, baseline, expected, parameters, spec, transaction):
        previous = None
        for attempt in (1, 2):
            if attempt == 2 and self.settle_interval:
                time.sleep(self.settle_interval)
            current, projected = self.observe(baseline, expected, transaction, "changed-" + str(attempt))
            if previous is not None:
                require(self.value_hash(projected) == self.value_hash(previous),
                        "Two complete filter-swap observations are unstable")
            present = {parameter["name"]: parameter for parameter in current.records}
            low, high = spec.effects
            require(value_identity(present[low]) == value_identity(parameters[high])
                    and value_identity(present[high]) == value_identity(parameters[low]),
                    "Button own-value reset did not prove the exact filter permutation")
            for name, before in parameters.items():
                after = present[name]
                if name in spec.effects or (self.read_only_observations and explicitly_read_only(before, after)):
                    continue
                require(observation_identity(after) == observation_identity(before),
                        "A non-effect provider value changed during the filter swap: " + name)
            self.journal.append("action_permutation_observation", transaction=transaction, attempt=attempt,
                                exact_filter_permutation=True, other_provider_values_unchanged=True,
                                consecutive_complete_observations=attempt,
                                read_only_values=self.summary["read_only_values"],
                                schema_sha256=projected["schema_sha256"],
                                atomic_snapshot=False, opaque_contents_verified=False)
            previous = projected

    def run_action_case(self, target):
        plugin = target.get("plugin")
        require(plugin in SPECS, "Only the registered Expander, Gate and Compressor fixtures are supported")
        require(len(self.summary["classes"]) < len(SPECS)
                and all(row["plugin"] != plugin for row in self.summary["classes"]),
                "Run each of the three registered plugin fixtures at most once")
        self.fixture_check(target, setup=True)
        baseline = self.snapshot(target)
        spec = SPECS[plugin]
        expected = self.projection(baseline)
        parameters = self.validate_baseline(baseline, spec)
        transaction = "action-" + str(len(self.summary["classes"]))
        reference = self.journal.baseline(transaction, baseline.serialized())
        row = {"plugin": plugin, "action": ACTION, "status": "prepared", "baseline": reference,
               "source": "dsp", "provider_indices_compared": len(baseline.records),
               "action_attempted": False, "inverse_attempted": False, "native_effects_verified": False,
               "audible_effects_verified": False, "opaque_contents_verified": False,
               "read_only_values": self.summary["read_only_values"], "atomic_snapshot": False}
        self.summary["classes"].append(row)
        self.phase(transaction, "Prepared", plugin=plugin, action=ACTION, baseline=reference,
                   instance=baseline.instance, source="dsp", schema_sha256=expected["schema_sha256"])
        self.progress()
        try:
            self.confirm_baseline(baseline, expected, transaction, "before-dispatch")
            attempts_before = self.counts["action_dispatch_attempts"]
            try:
                accepted = self.dispatch(baseline, spec, transaction, inverse=False)
            finally:
                row["action_attempted"] = self.counts["action_dispatch_attempts"] > attempts_before
            if not accepted:
                self.confirm_baseline(baseline, expected, transaction, "refused-unchanged")
                row["status"] = "action_refused_unchanged"
                self.phase(transaction, "NoWriteConfirmed", full_provider_unchanged=True)
            else:
                self.confirm_permutation(baseline, expected, parameters, spec, transaction)
                self.phase(transaction, "ChangedReadbackConfirmed", exact_filter_permutation=True,
                           consecutive_complete_provider_matches=2)
                row["exact_filter_permutation_verified"] = True
                attempts_before = self.counts["inverse_dispatch_attempts"]
                try:
                    accepted = self.dispatch(baseline, spec, transaction, inverse=True)
                finally:
                    row["inverse_attempted"] = self.counts["inverse_dispatch_attempts"] > attempts_before
                require(accepted,
                        "The single conditional inverse was refused; no retry is permitted")
                self.confirm_baseline(baseline, expected, transaction, "restored")
                row.update(status="action_changed_readback_restored", exact_filter_permutation_verified=True,
                           complete_provider_projection_restored=True, consecutive_changed_reads=2,
                           restored_projection_scope="complete_schema_and_non_read_only_values" if self.read_only_observations
                           else "complete_schema_and_all_exposed_values",
                           consecutive_restored_reads=2)
                self.phase(transaction, "StateConfirmed", status=row["status"],
                           complete_provider_projection_restored=True, native_effects_verified=False,
                           audible_effects_verified=False, opaque_contents_verified=False)
        except (UnknownMutation, ContextChanged):
            row["status"] = "unknown_or_context_changed" if row["action_attempted"] else "rejected_before_dispatch"
            self.phase(transaction, "UnknownAfterDispatch" if row["action_attempted"] else "FailedBeforeDispatch",
                       inverse_attempted=row["inverse_attempted"])
            self.progress()
            raise
        except Exception as error:
            restoration = row.get("exact_filter_permutation_verified") is True
            row["status"] = ("restoration_failed" if restoration else "effect_unconfirmed") if row["action_attempted"] else "rejected_before_dispatch"
            phase = ("RestorationFailed" if restoration else "EffectUnconfirmed") if row["action_attempted"] else "FailedBeforeDispatch"
            self.phase(transaction, phase,
                       error_type=type(error).__name__, message=str(error), inverse_attempted=row["inverse_attempted"])
            self.progress()
            if restoration and not isinstance(error, RestorationFailure):
                raise RestorationFailure("Conditional inverse did not independently restore the full provider") from error
            raise
        self.counts[row["status"]] += 1
        self.journal.append("action_case_result", **row)
        self.progress()
        return row

    def finish(self):
        verified = bool(self.summary["classes"]) and all(row["status"] == "action_changed_readback_restored"
                                                       for row in self.summary["classes"])
        self.summary.update(status="action_cases_verified" if verified else "action_cases_not_verified", completed_at=utc_now())
        self.progress()
        self.journal.append("run_completed", summary=self.summary)
        return 0 if verified else 2


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default="target/release/fender-studio-pro-mcp")
    parser.add_argument("--fixtures", required=True)
    parser.add_argument("--scratch-title", required=True)
    parser.add_argument("--output-dir", default=".local/live/plugin-actions-" + datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ"))
    parser.add_argument("--plugin", action="append", required=True, choices=(*SPECS, "Level Meter"))
    parser.add_argument("--action", default=ACTION, choices=(ACTION, *LEVEL_RESET_SPECS))
    parser.add_argument("--run-actions", action="store_true", required=True,
                        help="Run registered filter swaps; meter reset verification remains blocked without an independent effect witness")
    parser.add_argument("--pause", action="store_true", help="Start the MIDI owner before opening the DAW, then press Enter")
    parser.add_argument("--prepare-midi-fixtures", action="store_true")
    parser.add_argument("--settle-interval", type=float, default=0.1)
    parser.add_argument("--read-only-observations", action="store_true")
    args = parser.parse_args(argv)
    validate_settlement(2, args.settle_interval)
    fixtures = json.loads(Path(args.fixtures).read_text())
    require(isinstance(fixtures, list) and 0 < len(fixtures) <= 128
            and all(isinstance(item, dict) and isinstance(item.get("plugin"), str) for item in fixtures),
            "Expected 1..128 finite fixture targets")
    plugins = list(dict.fromkeys(args.plugin))
    require_effect_witness(args.action, plugins)
    require(0 < len(plugins) <= len(SPECS)
            and all(sum(item["plugin"] == plugin for item in fixtures) == 1 for plugin in plugins),
            "Every selected action plugin must match one exact fixture")
    targets = [{key: value for key, value in next(item for item in fixtures if item["plugin"] == plugin).items()
                if not key.startswith("test_")} for plugin in plugins]
    journal = Journal(args.output_dir)
    client = runner = None
    try:
        binary = Path(args.binary).resolve()
        journal.append("run_started", schema=1, mode="registered_button_actions",
                       binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(), scope={"plugins": plugins},
                       guarantee="two full exact filter-permutation observations before a single inverse; two full restoration observations")
        client = McpClient([str(binary), "serve"], timeout=45)
        print("MCP owner ready", flush=True)
        if args.pause:
            input("Open exactly the named scratch song, then press Enter: ")
        runner = ActionRunner(client, journal, args.scratch_title, args.prepare_midi_fixtures,
                              args.read_only_observations, args.settle_interval)
        runner.initialize()
        for target in targets:
            print("ACTION CASE " + target["plugin"], flush=True)
            runner.run_action_case(target)
        return runner.finish()
    except Exception as error:
        journal.append("run_failed", error_type=type(error).__name__, message=str(error))
        if runner is not None:
            runner.summary.update(status="failed", error_type=type(error).__name__, error=str(error), completed_at=utc_now())
            runner.progress()
        print("Acceptance stopped: " + type(error).__name__ + ": " + str(error), file=sys.stderr, flush=True)
        return 1
    finally:
        if client is not None:
            client.close()
        journal.close()


if __name__ == "__main__":
    sys.exit(main())
