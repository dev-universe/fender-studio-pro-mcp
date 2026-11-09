#!/usr/bin/env python3
"""Verify six fixed External Effect mode transitions in disconnected scratch fixtures.

This checks exposed state, not physical ping emission or hardware calibration.
Each case restores the original complete typed projection before the next case.
Raw receipts remain below the repository .local directory.
"""
import argparse
from dataclasses import dataclass
from datetime import datetime, timezone
from enum import Enum
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
    AcceptanceFailure, ContextChanged, Journal, RestorationFailure, Runner,
    UNKNOWN_OUTCOMES, UnknownMutation, parameter_schema, raw_value, require,
    utc_now, value_identity,
)


CLASSES = {
    "External Effect Mono": "873281D9-58DE-4B2D-B3BD-528F549B1AD9",
    "External Effect Stereo": "889DD9F8-14F0-44A5-BCEE-0F2B157FC5D4",
}
MODE_PARAMETERS = ("pingMode", "scopeMode", "showImage")
LATENCY_PARAMETERS = frozenset(("latencyOffset", "latencyTotal"))
COUPLED_PARAMETERS = frozenset(MODE_PARAMETERS) | LATENCY_PARAMETERS
SETTLE_ATTEMPTS = 6


class Mode(Enum):
    IMAGE = (0, 0, 1)
    SETUP = (1, 0, 0)
    SCOPE = (0, 1, 0)


@dataclass(frozen=True)
class ModeCase:
    parameter: str
    desired: int
    expected: Mode
    staging: tuple[str, int, Mode] | None = None


CASES = (
    ModeCase("pingMode", 1, Mode.SETUP),
    ModeCase("pingMode", 0, Mode.SCOPE, ("pingMode", 1, Mode.SETUP)),
    ModeCase("scopeMode", 1, Mode.SCOPE),
    ModeCase("scopeMode", 0, Mode.SETUP, ("scopeMode", 1, Mode.SCOPE)),
    ModeCase("showImage", 0, Mode.SCOPE),
    ModeCase("showImage", 1, Mode.IMAGE, ("scopeMode", 1, Mode.SCOPE)),
)


def observation(parameter):
    return value_identity(parameter), parameter["native_normalized"], parameter["text"]


def numeric_write(parameter, desired):
    tag, kind, _ = value_identity(parameter)
    require(type(tag) is int and ((kind == "integer" and tag & 15 == 1)
                                or (kind == "float" and tag & 15 == 2)),
            "External controls require an exact numeric carrier")
    if kind == "integer":
        require(type(desired) is int, "Captured integer restoration requires an integer")
        return {"value": desired}, (tag, kind, str(desired))
    bits = struct.pack(">d", float(desired)).hex().upper()
    return {"float_bits": bits}, (tag, kind, bits)


class ExternalModeRunner(Runner):
    def __init__(self, client, journal, scratch_title, settle_interval=0.15):
        require(type(settle_interval) in (int, float) and math.isfinite(settle_interval)
                and 0 <= settle_interval <= 1, "Use a finite settle interval from 0 to 1 second")
        super().__init__(client, journal, scratch_title)
        self.settle_interval = settle_interval
        self.summary.update(mode="disconnected_external_modes", fixtures=[],
                            parameter_verification="two_fresh_complete_typed_public_pages",
                            restoration="captured_image_then_conditional_captured_offset",
                            all_writable_parameters_verified=False,
                            hardware_route_verified=False, hardware_calibration_verified=False,
                            audio_emission_verified=False, opaque_contents_verified=False,
                            atomic_snapshot=False, settle_attempts=SETTLE_ATTEMPTS)

    def call(self, name, arguments=None, mutation=False, allow_error=False):
        try:
            receipt = super().call(name, arguments, mutation, allow_error=True)
        except (UnknownMutation, ContextChanged):
            raise
        except Exception as error:
            raise UnknownMutation("External request transport or receipt is unknown; no further write is permitted") from error
        result = receipt.get("result")
        nested = result if isinstance(result, dict) else {}
        error = receipt.get("error")
        code = error.get("code") if isinstance(error, dict) else error
        unknown_fields = (receipt.get("outcome"), code, nested.get("outcome"), nested.get("native_outcome"))
        if any(isinstance(value, str) and value in UNKNOWN_OUTCOMES for value in unknown_fields):
            raise UnknownMutation("External request outcome is unknown; no further write is permitted")
        if mutation and receipt.get("outcome") not in ("applied_verified", "applied_unverified", "not_applied"):
            raise UnknownMutation("External mutation has no known dispatch outcome; no further write is permitted")
        if mutation and any(key in nested and nested[key] not in ("applied_verified", "applied_unverified", "not_applied")
                            for key in ("outcome", "native_outcome")):
            raise UnknownMutation("External mutation has an incomplete native outcome; no further write is permitted")
        if not mutation and receipt.get("outcome") != "observed" and not allow_error:
            raise AcceptanceFailure("External read did not return observed data")
        if error and not allow_error:
            raise AcceptanceFailure(str(error))
        return receipt

    def fixture_check(self, target, setup=False):
        try:
            return super().fixture_check(target, setup)
        except (UnknownMutation, ContextChanged):
            raise
        except AcceptanceFailure as error:
            raise ContextChanged("External scratch fixture or stopped transport changed") from error

    def projection(self, provider, baseline=None, expected=None):
        try:
            result = project_snapshot(provider)
        except DigestProjectionError as error:
            raise AcceptanceFailure("Complete exact External projection is unavailable: " + str(error)) from error
        if baseline is not None and (provider.instance != baseline.instance or provider.source != baseline.source
                                     or result["count"] != expected["count"]
                                     or result["schema_sha256"] != expected["schema_sha256"]):
            raise ContextChanged("External native provider or complete typed schema changed")
        return result

    def disconnected(self, provider):
        parameters = {p["name"]: p for p in provider.records}
        for name in ("sendPort", "receivePort"):
            p = parameters.get(name)
            if not (p and p.get("native_kind") == "List" and type(raw_value(p)) is int and raw_value(p) == -1
                    and type(p.get("min")) is int and p["min"] == 0
                    and type(p.get("max")) is int and p["max"] == -1):
                raise ContextChanged("External fixture must keep both exact empty hardware port selections")
        return parameters

    def empty_choices(self, baseline):
        for name in ("sendPort", "receivePort"):
            result = self.call("plugin_parameter_choices", {
                **baseline.target, "source": "controller", "param": name,
                "offset": 0, "limit": 1, "expected_instance": baseline.instance["object_uid"],
            })["result"]
            require(isinstance(result, dict), "External hardware choices page is malformed")
            if result.get("instance") != baseline.instance or result.get("source") != "controller":
                raise ContextChanged("External hardware choices resolved a different native provider")
            require(result.get("ok") is True and result.get("param") == name
                    and type(result.get("reported_count")) is int and result["reported_count"] == 0
                    and type(result.get("offset")) is int and result["offset"] == 0
                    and result.get("choices") == [] and "next_offset" in result and result["next_offset"] is None
                    and result.get("page_complete") is True and result.get("all_indices_read") is True,
                    "External mode verification requires complete empty hardware choices")

    def mode(self, parameters):
        triple = tuple(raw_value(parameters[name]) for name in MODE_PARAMETERS)
        require(all(type(value) in (int, float) and value in (0, 1) for value in triple),
                "External mode values must be binary")
        try:
            return Mode(triple)
        except ValueError as error:
            raise AcceptanceFailure("External mode triple is not Image, Setup or Scope") from error

    def observe(self, baseline, expected, transaction, stage, attempt):
        self.fixture_check(baseline.target)
        current = self.snapshot(baseline.target, baseline)
        projected = self.projection(current, baseline, expected)
        self.disconnected(current)
        self.journal.append("external_mode_observation", transaction=transaction,
                            stage=stage, attempt=attempt, projection=projected)
        return current, projected

    def dispatch(self, baseline, expected, transaction, name, desired, phase):
        current, _ = self.observe(baseline, expected, transaction, "guard-" + phase, 1)
        self.empty_choices(baseline)
        parameter = next(p for p in current.records if p["name"] == name)
        require(name in (*MODE_PARAMETERS, "latencyOffset") and parameter.get("writable") is True
                and parameter.get("enabled") is True and parameter.get("readOnly") is False
                and parameter.get("support", {}).get("raw") is True,
                "Fixed External control lost enabled raw write support")
        if phase in ("Staging", "Change"):
            require(raw_value(parameter) != desired, "External case must change its own exact original value")
        values, identity = numeric_write(parameter, desired)
        self.phase(transaction, phase + "InFlight", parameter=name)
        self.counts["mutation_attempts"] += 1
        receipt = self.write(baseline, parameter, values)
        result = receipt.get("result")
        require(isinstance(result, dict), "External setter lacks its native receipt")
        if result.get("instance") != baseline.instance or result.get("source") != baseline.source:
            raise ContextChanged("External setter resolved a different native provider")
        require(receipt.get("outcome") in ("applied_verified", "applied_unverified")
                and not receipt.get("error"), "External fixed mutation was not accepted")
        after = result.get("after")
        require(isinstance(after, dict) and parameter_schema(after) == parameter_schema(parameter)
                and value_identity(after) == identity, "External setter did not establish the requested exact own value")
        self.phase(transaction, phase + "Accepted", parameter=name)
        return identity

    def confirm_mode(self, baseline, expected, transaction, case, identity, stage):
        originals = {p["name"]: p for p in baseline.records}
        for attempt in (1, 2):
            if attempt == 2 and self.settle_interval:
                time.sleep(self.settle_interval)
            current, _ = self.observe(baseline, expected, transaction, stage, attempt)
            parameters = {p["name"]: p for p in current.records}
            require(self.mode(parameters) is case.expected
                    and value_identity(parameters[case.parameter]) == identity,
                    "Independent complete External readback did not establish the requested mode and own value")
            require(all(observation(p) == observation(originals[p["name"]]) for p in current.records
                        if p["name"] not in COUPLED_PARAMETERS),
                    "External transition changed an unrelated exposed value")

    def confirm_baseline(self, baseline, expected, transaction, stage):
        consecutive = 0
        originals = {p["name"]: p for p in baseline.records}
        for attempt in range(1, SETTLE_ATTEMPTS + 1):
            if attempt > 1 and self.settle_interval:
                time.sleep(self.settle_interval)
            current, projected = self.observe(baseline, expected, transaction, stage, attempt)
            equal = projected["all_values_sha256"] == expected["all_values_sha256"]
            consecutive = consecutive + 1 if equal else 0
            self.journal.append("external_baseline_comparison", transaction=transaction, stage=stage,
                                attempt=attempt, full_baseline_equal=equal, consecutive_matches=consecutive)
            if consecutive == 2:
                return
            changed = {p["name"] for p in current.records if observation(p) != observation(originals[p["name"]])}
            if not changed <= LATENCY_PARAMETERS:
                raise RestorationFailure("External complete baseline has a non-latency mismatch")
        raise RestorationFailure("External complete baseline did not match twice within the bounded latency settlement")

    def recover(self, baseline, expected, transaction):
        originals = {p["name"]: p for p in baseline.records}
        self.dispatch(baseline, expected, transaction, "showImage", raw_value(originals["showImage"]), "RestoreImage")
        current, _ = self.observe(baseline, expected, transaction, "after-image-restore", 1)
        offset = next(p for p in current.records if p["name"] == "latencyOffset")
        if value_identity(offset) != value_identity(originals["latencyOffset"]):
            self.dispatch(baseline, expected, transaction, "latencyOffset", raw_value(originals["latencyOffset"]), "RestoreOffset")
        self.confirm_baseline(baseline, expected, transaction, "restored")

    def run_fixture(self, target):
        require(isinstance(target, dict) and set(target) <= {"plugin", "channel", "slot", "track", "kind"}
                and target.get("plugin") in CLASSES and target.get("kind", "effect") == "effect"
                and isinstance(target.get("channel"), str) and type(target.get("slot")) is int
                and 0 <= target["slot"] <= 4095,
                "Use one exact registered External Effect scratch insert target")
        require(len(self.summary["fixtures"]) < len(CLASSES)
                and all(row["plugin"] != target["plugin"] for row in self.summary["fixtures"]),
                "Each registered External fixture may run only once")
        target = dict(target)
        self.fixture_check(target)
        baseline = self.snapshot(target)
        expected = self.projection(baseline)
        require(baseline.source == "controller" and baseline.instance["class_uid"] == CLASSES[target["plugin"]]
                and len(baseline.records) == 24, "Use the exact validated 24-record External controller class")
        originals = self.disconnected(baseline)
        require(self.mode(originals) is Mode.IMAGE, "External mode tests require an original valid Image mode")
        for name in (*MODE_PARAMETERS, "latencyOffset"):
            parameter = originals[name]
            require(parameter.get("native_kind") == ("Integer" if name == "latencyOffset" else "Toggle")
                    and parameter.get("writable") is True and parameter.get("enabled") is True
                    and parameter.get("readOnly") is False and parameter.get("support", {}).get("raw") is True,
                    "External captured controls require enabled raw writable native types")
            require(name == "latencyOffset" or (parameter.get("min") == 0 and parameter.get("max") == 1),
                    "External mode toggles require the exact binary range")
            numeric_write(parameter, raw_value(parameter))
        self.empty_choices(baseline)
        fixture_index = len(self.summary["fixtures"])
        reference = self.journal.baseline("external-" + str(fixture_index), baseline.serialized())
        row = {"plugin": target["plugin"], "baseline": reference, "cases": [], "status": "prepared",
               "provider_indices_compared": len(baseline.records), "hardware_calibration_verified": False,
               "all_writable_parameters_verified": False, "opaque_contents_verified": False}
        self.summary["fixtures"].append(row)
        for index, case in enumerate(CASES):
            transaction = f"external-{fixture_index}-{index}"
            result = {"parameter": case.parameter, "requested_mode": case.expected.name.lower(),
                      "status": "prepared", "complete_provider_projection_restored": False}
            row["cases"].append(result)
            self.phase(transaction, "Prepared", baseline=reference, case=index)
            mutation_count = self.counts["mutation_attempts"]
            recovering = False
            try:
                self.confirm_baseline(baseline, expected, transaction, "before-case")
                if case.staging:
                    name, desired, mode = case.staging
                    identity = self.dispatch(baseline, expected, transaction, name, desired, "Staging")
                    self.confirm_mode(baseline, expected, transaction, ModeCase(name, desired, mode), identity, "staged")
                    self.phase(transaction, "StagingConfirmed")
                identity = self.dispatch(baseline, expected, transaction, case.parameter, case.desired, "Change")
                self.confirm_mode(baseline, expected, transaction, case, identity, "changed")
                result["transition_verified"] = True
                self.phase(transaction, "ChangedReadbackConfirmed", consecutive_full_reads=2)
                recovering = True
                self.recover(baseline, expected, transaction)
            except (UnknownMutation, ContextChanged):
                result["status"] = "unknown_or_context_changed"
                self.phase(transaction, "UnknownAfterDispatch" if self.counts["mutation_attempts"] > mutation_count else "FailedBeforeDispatch")
                row["status"] = "failed"
                self.progress()
                raise
            except Exception as error:
                wrote = self.counts["mutation_attempts"] > mutation_count
                result["status"] = "restoration_failed" if recovering else "transition_unconfirmed"
                if wrote and not recovering:
                    try:
                        recovering = True
                        self.recover(baseline, expected, transaction)
                    except (UnknownMutation, ContextChanged):
                        self.phase(transaction, "UnknownAfterDispatch")
                        row["status"] = "failed"
                        self.progress()
                        raise
                    except Exception as recovery_error:
                        self.phase(transaction, "RestorationFailed", error_type=type(recovery_error).__name__)
                        row["status"] = "failed"
                        self.progress()
                        raise RestorationFailure("Finite captured External recovery failed; no retry is permitted") from recovery_error
                    result.update(status="transition_unconfirmed_restored", complete_provider_projection_restored=True)
                    self.phase(transaction, "StateConfirmed", transition_verified=False, complete_provider_projection_restored=True)
                else:
                    self.phase(transaction, "RestorationFailed" if wrote else "FailedBeforeDispatch")
                row["status"] = "failed"
                self.progress()
                raise error
            result.update(status="mode_changed_readback_restored", complete_provider_projection_restored=True,
                          consecutive_changed_reads=2, consecutive_restored_reads=2)
            self.phase(transaction, "StateConfirmed", transition_verified=True, complete_provider_projection_restored=True)
            self.counts["mode_changed_readback_restored"] += 1
            self.journal.append("external_mode_case_result", **result)
            self.progress()
        row["status"] = "six_disconnected_mode_transitions_verified"
        self.progress()
        return row

    def finish(self):
        verified = bool(self.summary["fixtures"]) and all(row["status"] == "six_disconnected_mode_transitions_verified"
                                                       for row in self.summary["fixtures"])
        self.summary.update(status="disconnected_external_modes_verified" if verified else "external_modes_not_verified",
                            completed_at=utc_now())
        self.progress()
        self.journal.append("run_completed", summary=self.summary)
        return 0 if verified else 2


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default="target/release/fender-studio-pro-mcp")
    parser.add_argument("--fixtures", required=True)
    parser.add_argument("--scratch-title", required=True)
    parser.add_argument("--plugin", action="append", required=True, choices=tuple(CLASSES))
    parser.add_argument("--run-modes", action="store_true", required=True)
    parser.add_argument("--output-dir", default=".local/live/external-modes-" + datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ"))
    parser.add_argument("--pause", action="store_true")
    args = parser.parse_args(argv)
    fixtures = json.loads(Path(args.fixtures).read_text())
    plugins = list(dict.fromkeys(args.plugin))
    require(isinstance(fixtures, list) and 0 < len(fixtures) <= 128
            and all(isinstance(item, dict) and isinstance(item.get("plugin"), str) for item in fixtures)
            and all(sum(item["plugin"] == plugin for item in fixtures) == 1 for plugin in plugins),
            "Every selected External plugin must match one finite fixture")
    targets = [{key: value for key, value in next(item for item in fixtures if item["plugin"] == plugin).items()
                if not key.startswith("test_")} for plugin in plugins]
    journal = Journal(args.output_dir)
    client = runner = None
    try:
        binary = Path(args.binary).resolve()
        journal.append("run_started", schema=1, mode="disconnected_external_modes",
                       binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(), plugins=plugins)
        client = McpClient([str(binary), "serve"], timeout=45)
        print("MCP owner ready", flush=True)
        if args.pause:
            input("Open exactly the named disconnected scratch fixture, then press Enter: ")
        runner = ExternalModeRunner(client, journal, args.scratch_title)
        runner.initialize()
        for target in targets:
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
