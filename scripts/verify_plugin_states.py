#!/usr/bin/env python3
"""Verify issued native state tokens in named disposable plugin fixtures.

The owner must start before the DAW. No song, preset file or state blob is saved.
Raw receipts and baseline parameter values stay below the repository .local tree.
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
from verify_all_parameters import (
    AcceptanceFailure, ContextChanged, Journal, RestorationFailure, Runner,
    UnknownMutation, candidate, classification, decode_float_bits,
    in_observed_float_range, raw_value, require, utc_now, value_identity,
)


@dataclass(frozen=True)
class Capsule:
    token: str
    byte_count: int
    sha256: str


def hexadecimal(value, length):
    return isinstance(value, str) and len(value) == length and all(c in "0123456789abcdefABCDEF" for c in value)


def validate_settlement(attempts, interval):
    require(type(attempts) is int and 2 <= attempts <= 50, "Use 2..50 settle attempts")
    require(type(interval) in (int, float) and math.isfinite(interval) and 0 <= interval <= 1,
            "Use a settle interval from 0 to 1 second")


def snapshot_difference(baseline, current, read_only_observations=False):
    if current.instance != baseline.instance or current.source != baseline.source:
        raise ContextChanged("State restoration resolved a different provider")
    if current.schema != baseline.schema:
        return {"schema_changed": True, "changed_parameters": [], "read_only_changed": []}
    changed, observations = [], []
    for before, after in zip(baseline.records, current.records):
        if value_identity(before) == value_identity(after):
            continue
        explicitly_read_only = all(p.get("access_metadata_complete") is True
                                  and p.get("readOnly") is True and p.get("writable") is False
                                  for p in (before, after))
        if read_only_observations and explicitly_read_only:
            observations.append(before["name"])
        else:
            changed.append(before["name"])
    return {"schema_changed": False, "changed_parameters": changed,
            "read_only_changed": observations}


def explicit_candidate(parameter, desired_float_bits=None, desired_value=None):
    if desired_float_bits is None and desired_value is None:
        return candidate(parameter)[0]
    data_kind = parameter["native_value"]["data"]["kind"]
    if desired_float_bits is not None:
        desired = decode_float_bits(desired_float_bits)
        require(data_kind == "float", "Exact Float bits require a native Float value")
        change = {"float_bits": desired_float_bits.upper()}
    else:
        desired = desired_value
        require(type(desired) in (int, float) and math.isfinite(desired), "Desired value must be finite")
        if data_kind == "float":
            change = {"float_bits": struct.pack(">d", float(desired)).hex().upper()}
        else:
            require(data_kind == "integer" and type(desired) is int and abs(desired) <= 9_007_199_254_740_991,
                    "Native Integer changes require a safe integer")
            change = {"value": desired}
    require(in_observed_float_range(desired, parameter["min"], parameter["max"]),
            "Desired value is outside the live parameter range")
    require(desired != raw_value(parameter), "Desired value must change the selected parameter")
    return change


class StateRunner(Runner):
    def __init__(self, client, journal, scratch_title, prepare_midi=False,
                 settle_attempts=8, settle_interval=0.1, read_only_observations=False):
        super().__init__(client, journal, scratch_title, prepare_midi)
        validate_settlement(settle_attempts, settle_interval)
        self.settle_attempts = settle_attempts
        self.settle_interval = settle_interval
        self.read_only_observations = read_only_observations
        self.summary.update(mode="state_capsules", opaque_contents_verified=False,
                            all_writable_parameters_verified=False,
                            read_only_values="observed_not_restored" if read_only_observations else "exact_comparison")

    def state_call(self, provider, action, token=None, allow_error=False):
        require(action in ("capture", "restore", "release"), "Use a fixed native state lifecycle action")
        args = {**provider.target, "action": action,
                "expected_instance": provider.instance["object_uid"]}
        if token is not None:
            args["token"] = token
        transaction = f"state-call-{self.call_count + 1:06d}"
        self.phase(transaction, "StateLifecycleInFlight", action=action, instance=provider.instance)
        try:
            # Inspect native evidence before propagating an ordinary receipt error.
            receipt = self.call("plugin_state", args, mutation=True, allow_error=True)
            outer = receipt.get("outcome")
            applied = ("applied_verified", "applied_unverified")
            allowed_outer = ("observed", *applied, "not_applied") if action == "capture" else (*applied, "not_applied")
            result = receipt.get("result", {})
            if not isinstance(outer, str) or outer not in allowed_outer or not isinstance(result, dict):
                raise UnknownMutation("Native state lifecycle returned a malformed or contradictory outcome")
            expected = (("not_applied",) if outer == "not_applied" else
                        ("observed",) if action == "capture" else applied)
            native = []
            for field in ("outcome", "native_outcome"):
                if field not in result:
                    continue
                outcome = result[field]
                if not isinstance(outcome, str) or outcome not in expected:
                    raise UnknownMutation("Native state " + action + " has unknown or contradictory " + field)
                native.append(outcome)
            if len(set(native)) > 1:
                raise UnknownMutation("Native state lifecycle outcome fields disagree")
        except (UnknownMutation, ContextChanged):
            self.phase(transaction, "UnknownAfterDispatch", action=action)
            raise
        except Exception as error:
            self.phase(transaction, "UnknownAfterDispatch", action=action)
            raise UnknownMutation("Native state lifecycle evidence is malformed; stop further mutations") from error
        self.phase(transaction, "NoWriteConfirmed" if outer == "not_applied" else "StateConfirmed",
                   action=action, scope="lifecycle_outcome_only", independently_restored=False)
        if receipt.get("error") and not allow_error:
            raise AcceptanceFailure(str(receipt["error"]))
        return receipt

    def capture(self, provider):
        self.fixture_check(provider.target)
        receipt = self.state_call(provider, "capture")
        result = receipt.get("result", {})
        require(receipt.get("outcome") in ("observed", "applied_verified", "applied_unverified")
                and result.get("ok") is True, "Native capture did not return an issued state")
        require(result.get("instance") == provider.instance, "Captured state has a different native target")
        token, size, sha256 = (result.get(key) for key in ("token", "byte_count", "sha256"))
        require(hexadecimal(token, 64) and token == token.upper(), "Native state token is malformed")
        require(type(size) is int and 0 < size <= 16 * 1024 * 1024 and hexadecimal(sha256, 64),
                "Native state summary is malformed")
        require(result.get("serialization") == "component_alias_IStorable", "Unexpected state serialization provider")
        require(not any(key in result for key in ("bytes", "state_bytes", "blob", "path", "url")),
                "Native capture must return metadata without imported or exported state data")
        return Capsule(token, size, sha256.lower())

    def release(self, provider, capsule):
        self.fixture_check(provider.target)
        receipt = self.state_call(provider, "release", capsule.token)
        result = receipt.get("result", {})
        require(receipt.get("outcome") in ("applied_verified", "applied_unverified")
                and result.get("ok") is True and result.get("released") is True
                and result.get("instance") == provider.instance, "Native state release was not confirmed")

    def wait_for_baseline(self, baseline, transaction):
        matching = 0
        last_difference = None
        for attempt in range(self.settle_attempts):
            self.fixture_check(baseline.target)
            current = self.snapshot(baseline.target)
            difference = snapshot_difference(baseline, current, self.read_only_observations)
            match = not difference["schema_changed"] and not difference["changed_parameters"]
            matching = matching + 1 if match else 0
            last_difference = difference
            self.journal.append("state_baseline_observation", transaction=transaction,
                                attempt=attempt + 1, matches=match, consecutive_matches=matching,
                                schema_sha256=current.schema, **difference)
            if matching == 2:
                return current
            if attempt + 1 < self.settle_attempts and self.settle_interval:
                time.sleep(self.settle_interval)
        raise RestorationFailure("Native load did not restore two consecutive complete provider baselines: "
                                 + json.dumps(last_difference, ensure_ascii=False))

    def restore(self, baseline, capsule, transaction):
        self.phase(transaction, "RestoreGuard")
        try:
            self.fixture_check(baseline.target)
            self.phase(transaction, "RestoreInFlight")
            receipt = self.state_call(baseline, "restore", capsule.token, allow_error=True)
            result = receipt.get("result", {})
            require(receipt.get("outcome") in ("applied_unverified", "applied_verified")
                    and not receipt.get("error") and result.get("ok") is True
                    and result.get("load_accepted") is True and result.get("instance") == baseline.instance,
                    "Native state load was not accepted with a known applied outcome")
            self.phase(transaction, "Restored", native_load_accepted=True,
                       native_verified=result.get("verified"))
            self.wait_for_baseline(baseline, transaction)
            self.phase(transaction, "RestoreReadbackConfirmed",
                       compared="all_provider_schema_and_exact_writable_values" if self.read_only_observations
                       else "all_current_provider_indices_schema_and_exact_native_values",
                       consecutive_matches=2, opaque_contents_verified=False)
        except (UnknownMutation, ContextChanged):
            self.phase(transaction, "UnknownAfterDispatch")
            raise
        except Exception as error:
            self.phase(transaction, "RestorationFailed", error_type=type(error).__name__, message=str(error))
            if isinstance(error, RestorationFailure):
                raise
            raise RestorationFailure("State restoration was not independently confirmed; stop further writes") from error

    def verify_released_token(self, baseline, capsule, transaction):
        self.fixture_check(baseline.target)
        self.phase(transaction, "ReleasedTokenCheckInFlight")
        try:
            receipt = self.state_call(baseline, "restore", capsule.token, allow_error=True)
        except (UnknownMutation, ContextChanged):
            self.phase(transaction, "UnknownAfterDispatch")
            raise
        require(receipt.get("outcome") == "not_applied", "Released state token was not refused before dispatch")
        self.wait_for_baseline(baseline, transaction)
        self.journal.append("released_token_refused", transaction=transaction,
                            unchanged_provider_independently_confirmed=True)

    def verify_other_target(self, baseline, capsule, target, transaction):
        require(target != baseline.target, "Wrong-target validation requires a different fixture")
        self.fixture_check(target, setup=True)
        if target["plugin"].startswith("External Effect "):
            self.call("plugin_editor_open", target, mutation=True)
        other = self.snapshot(target)
        require(other.instance["object_uid"] != baseline.instance["object_uid"],
                "Wrong-target fixture resolved the original Device")
        self.journal.baseline(transaction + "-wrong-target", other.serialized())
        self.fixture_check(target)
        self.phase(transaction, "WrongTargetCheckInFlight")
        try:
            receipt = self.state_call(other, "restore", capsule.token, allow_error=True)
        except (UnknownMutation, ContextChanged):
            self.phase(transaction, "UnknownAfterDispatch")
            raise
        require(receipt.get("outcome") == "not_applied", "State token was not refused on a different Device")
        self.wait_for_baseline(other, transaction)
        self.fixture_check(baseline.target, setup=True)
        self.wait_for_baseline(baseline, transaction)
        self.journal.append("wrong_target_token_refused", transaction=transaction,
                            both_providers_independently_unchanged=True)

    def selected_parameter(self, baseline, name, allow_model=False, explicit_desired=False):
        matches = [parameter for parameter in baseline.records if parameter["name"] == name]
        require(len(matches) == 1, "Expected one exact test parameter in the native provider")
        parameter = matches[0]
        category = classification(baseline.target["plugin"], parameter)
        allowed = category in ("numeric_scalar", "literal_string")
        if category == "model_state_restore_required":
            allowed = allow_model and explicit_desired and parameter.get("writable") is True
            allowed = allowed and parameter.get("support", {}).get("raw") is True
        require(allowed and name not in ("bypass", "@bypass"),
                "Use an explicitly proved mutable scalar, or an explicit model and desired value")
        return parameter

    def run_state_case(self, target, parameter_name, *, desired_float_bits=None,
                       desired_value=None, allow_model=False, repeats=1, wrong_target=None):
        require(type(repeats) is int and 1 <= repeats <= 3, "Use 1..3 known state-token repeats")
        self.fixture_check(target, setup=True)
        if target["plugin"].startswith("External Effect "):
            self.call("plugin_editor_open", target, mutation=True)
        baseline = self.snapshot(target)
        parameter = self.selected_parameter(baseline, parameter_name, allow_model,
                                            desired_float_bits is not None or desired_value is not None)
        change = explicit_candidate(parameter, desired_float_bits, desired_value)
        transaction = f"state-{len(self.summary['classes']):03d}"
        reference = self.journal.baseline(transaction, baseline.serialized())
        self.phase(transaction, "Prepared", plugin=target["plugin"], parameter=parameter_name,
                   requested=change, baseline=reference, source=baseline.source,
                   instance=baseline.instance, schema_sha256=baseline.schema)
        capsule = self.capture(baseline)
        self.phase(transaction, "Captured", token=capsule.token,
                   byte_count=capsule.byte_count, sha256=capsule.sha256)
        repeated_capture = self.capture(baseline)
        require(repeated_capture.token != capsule.token, "Native capture reused an existing token")
        capture_stable = capsule.sha256 == repeated_capture.sha256
        self.journal.append("capture_digest_comparison", transaction=transaction,
                            stable=capture_stable, byte_count_equal=capsule.byte_count == repeated_capture.byte_count,
                            diagnostic_only=True)
        self.release(baseline, repeated_capture)
        self.wait_for_baseline(baseline, transaction)
        if wrong_target is not None:
            self.verify_other_target(baseline, capsule, wrong_target, transaction)
        settlements = []
        for repeat in range(repeats):
            current = self.read_parameter(baseline, parameter)
            require(value_identity(current) == value_identity(parameter), "Test parameter drifted from the baseline")
            self.fixture_check(target)
            self.phase(transaction, "ChangeInFlight", repeat=repeat + 1)
            try:
                changed = self.write(baseline, parameter, change)
            except (UnknownMutation, ContextChanged):
                self.phase(transaction, "UnknownAfterDispatch")
                raise
            if changed.get("outcome") == "not_applied":
                self.wait_for_baseline(baseline, transaction)
                self.phase(transaction, "NoWriteConfirmed")
                raise AcceptanceFailure("Scalar/model change was refused; state restoration has not been exercised")
            if changed.get("outcome") not in ("applied_verified", "applied_unverified"):
                self.phase(transaction, "UnknownAfterDispatch")
                raise UnknownMutation("Scalar/model change did not establish a known applied outcome")
            self.phase(transaction, "Changed", repeat=repeat + 1)
            failure = None
            try:
                after = changed.get("result", {}).get("after")
                observed, settlement = self.confirm_changed_readback(
                    baseline, parameter, after, interval=self.settle_interval)
                settlements.append(settlement)
                self.phase(transaction, "ChangedReadbackConfirmed", repeat=repeat + 1,
                           observed=observed, settlement=settlement)
            except (UnknownMutation, ContextChanged):
                self.phase(transaction, "UnknownAfterDispatch")
                raise
            except Exception as error:
                failure = str(error)
            self.restore(baseline, capsule, transaction)
            if failure is not None:
                self.phase(transaction, "StateConfirmed", status="change_failed_restored")
                raise AcceptanceFailure(failure)
            self.counts["changed_load_independently_restored"] += 1
            if settlement["quantized_settlement"]:
                self.counts["quantized_settlements"] += 1
        restored_capture = self.capture(baseline)
        restore_digest_stable = restored_capture.sha256 == capsule.sha256
        self.journal.append("restored_digest_comparison", transaction=transaction,
                            stable=restore_digest_stable,
                            byte_count_equal=restored_capture.byte_count == capsule.byte_count,
                            diagnostic_only=True)
        self.release(baseline, restored_capture)
        self.release(baseline, capsule)
        self.verify_released_token(baseline, capsule, transaction)
        self.phase(transaction, "StateConfirmed", status="state_capsule_case_verified",
                   restored_repeats=repeats, released_token_refused=True)
        row = {"plugin": target["plugin"], "parameter": parameter_name,
               "status": "state_capsule_case_verified", "source": baseline.source,
               "provider_indices_compared": len(baseline.records), "repeats": repeats,
               "capture_digest_stable": capture_stable, "restored_digest_stable": restore_digest_stable,
               "released_token_refused": True, "wrong_target_token_refused": wrong_target is not None,
               "quantized_settlement": any(item["quantized_settlement"] for item in settlements),
               "quantized_repeat_count": sum(item["quantized_settlement"] for item in settlements),
               "baseline": reference, "opaque_contents_verified": False}
        row["read_only_values"] = self.summary["read_only_values"]
        self.summary["classes"].append(row)
        self.counts["state_cases_verified"] += 1
        self.progress()
        return row


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default="target/release/fender-studio-pro-mcp")
    parser.add_argument("--fixtures", required=True)
    parser.add_argument("--scratch-title", required=True)
    parser.add_argument("--output-dir", default=".local/live/plugin-states-" + datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ"))
    parser.add_argument("--plugin", action="append", required=True, help="Select an exact fixture plugin; repeat for more")
    parser.add_argument("--parameter", help="Exact override; otherwise use each fixture's previously tested test_parameter")
    desired = parser.add_mutually_exclusive_group()
    desired.add_argument("--desired-float-bits", help="Explicit finite f64 value as 16 hexadecimal digits")
    desired.add_argument("--desired-value", type=json.loads, help="Explicit finite JSON scalar; Float values use exact bits")
    parser.add_argument("--allow-model", action="store_true", help="Permit an explicitly named model selector with an explicit desired value")
    parser.add_argument("--repeats", type=int, default=1, help="Reuse the same frozen token for 1..3 change/load cycles")
    parser.add_argument("--wrong-target-plugin", help="Also prove rejection on this different exact fixture")
    parser.add_argument("--pause", action="store_true", help="Start the MIDI owner before launching the DAW, then press Enter")
    parser.add_argument("--prepare-midi-fixtures", action="store_true")
    parser.add_argument("--settle-attempts", type=int, default=8)
    parser.add_argument("--settle-interval", type=float, default=0.1)
    parser.add_argument("--read-only-observations", action="store_true",
                        help="Report authoritative native read-only value drift separately; compare every schema and exact writable value")
    args = parser.parse_args(argv)
    require(1 <= args.repeats <= 3, "Use 1..3 repeats")
    validate_settlement(args.settle_attempts, args.settle_interval)
    if args.allow_model:
        require(args.parameter and (args.desired_float_bits is not None or args.desired_value is not None),
                "Model acceptance requires an exact parameter and explicit desired value")
    if args.desired_float_bits is not None:
        decode_float_bits(args.desired_float_bits)
    fixtures = json.loads(Path(args.fixtures).read_text())
    require(isinstance(fixtures, list) and 0 < len(fixtures) <= 128, "Expected 1..128 fixture targets")
    require(all(isinstance(fixture, dict) and isinstance(fixture.get("plugin"), str) for fixture in fixtures),
            "Invalid fixture targets")
    selected = list(dict.fromkeys(args.plugin))
    require(all(sum(fixture["plugin"] == plugin for fixture in fixtures) == 1 for plugin in selected),
            "Every selected plugin must match one exact fixture")
    targets = [next(fixture for fixture in fixtures if fixture["plugin"] == plugin) for plugin in selected]
    other = None
    if args.wrong_target_plugin:
        require(args.wrong_target_plugin not in selected, "Wrong-target fixture must be outside the selected write scope")
        others = [fixture for fixture in fixtures if fixture["plugin"] == args.wrong_target_plugin]
        require(len(others) == 1, "Wrong-target plugin must match one fixture")
        other = {key: value for key, value in others[0].items() if not key.startswith("test_")}
    for fixture in targets:
        require(isinstance(args.parameter or fixture.get("test_parameter"), str),
                "Each selected fixture requires an explicitly named or previously tested scalar")
    journal = Journal(args.output_dir)
    client = runner = None
    try:
        binary = Path(args.binary).resolve()
        journal.append("run_started", schema=1, mode="state_capsules",
                       binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                       scope={"plugins": args.plugin, "parameter": args.parameter, "repeats": args.repeats},
                       guarantee="independent changed value and two complete restored provider reads; opaque state bytes are not decoded")
        client = McpClient([str(binary), "serve"], timeout=45)
        print("MCP owner ready", flush=True)
        if args.pause:
            input("Open exactly the named scratch song, then press Enter: ")
        runner = StateRunner(client, journal, args.scratch_title, args.prepare_midi_fixtures,
                             args.settle_attempts, args.settle_interval, args.read_only_observations)
        runner.initialize()
        for index, fixture in enumerate(targets, 1):
            print(f"[{index}/{len(targets)}] {fixture['plugin']}", flush=True)
            target = {key: value for key, value in fixture.items() if not key.startswith("test_")}
            runner.run_state_case(target, args.parameter or fixture["test_parameter"],
                                  desired_float_bits=args.desired_float_bits, desired_value=args.desired_value,
                                  allow_model=args.allow_model, repeats=args.repeats, wrong_target=other)
        runner.summary.update(status="state_cases_verified", completed_at=utc_now())
        runner.progress()
        journal.append("run_completed", summary=runner.summary)
        print(json.dumps({"status": runner.summary["status"], "counts": dict(runner.counts),
                          "opaque_contents_verified": False, "all_writable_parameters_verified": False}), flush=True)
        return 0
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
