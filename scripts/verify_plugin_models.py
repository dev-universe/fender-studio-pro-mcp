#!/usr/bin/env python3
"""Verify bounded exposed model choices with one immutable native state capsule.

The owner starts before the DAW. Only named, stopped, muted scratch fixtures
are eligible. Raw baselines, labels, plans and receipts stay below .local.
Current exposed selector ranges never establish a complete model catalog.
"""
import argparse
from dataclasses import dataclass
from datetime import datetime, timezone
import hashlib
import json
import math
from pathlib import Path
import sys
import time

from mcp_client import McpClient
from parameter_digest import DigestProjectionError, MAX_SAFE_INTEGER, project_snapshot
from verify_all_parameters import (
    AcceptanceFailure, ContextChanged, Journal, ProviderSnapshot, RestorationFailure, UNKNOWN_OUTCOMES,
    UnknownMutation, classification, decode_float_bits, require, utc_now, value_identity,
)
from verify_plugin_states import StateRunner, explicit_candidate, snapshot_difference, validate_settlement


MAX_CHOICES = 256
MAX_CASES = 128
FORMAT_CHUNK = 32
SELECTOR_SCHEMA_FIELDS = ("name", "tag", "native_kind", "enabled", "readOnly", "writable",
                          "native_min", "native_max", "native_default", "native_support",
                          "support", "display_parser")


@dataclass
class ModelSnapshot(ProviderSnapshot):
    typed_schema: str
    projection: dict

    @property
    def schema(self):
        return self.typed_schema


@dataclass(frozen=True)
class ModelChoice:
    value: int
    label: str


@dataclass(frozen=True)
class ModelPlan:
    lower: int
    upper: int
    baseline_value: int
    choices: tuple[ModelChoice, ...]


def source_hashes():
    root = Path(__file__).resolve().parent
    return {name: hashlib.sha256((root / name).read_bytes()).hexdigest()
            for name in ("verify_plugin_models.py", "verify_plugin_states.py",
                         "verify_all_parameters.py", "parameter_digest.py")}


def exact_integer(native, shadow=None):
    require(isinstance(native, dict) and isinstance(native.get("data"), dict),
            "Model choices require typed native numeric values")
    tag, data = native.get("variant_tag"), native["data"]
    kind = data.get("kind")
    require(type(tag) is int and -(1 << 15) <= tag < (1 << 15)
            and ((kind == "integer" and tag & 15 == 1) or (kind == "float" and tag & 15 == 2)),
            "Model choices require a matching Int or Float carrier")
    identity = value_identity({"native_value": native, "value": data.get("value") if shadow is None else shadow})
    if kind == "integer":
        require(type(data.get("value")) is int, "Model choices require safe native integers")
        number = data["value"]
    else:
        number = decode_float_bits(identity[2])
        require(number.is_integer(), "Model choices require an integral native value")
    require(abs(number) <= MAX_SAFE_INTEGER, "Model choices exceed the formatter's safe integer range")
    return int(number)


def selector_schema(parameter):
    require(all(field in parameter for field in SELECTOR_SCHEMA_FIELDS), "Model selector metadata is incomplete")
    return {field: parameter[field] for field in SELECTOR_SCHEMA_FIELDS}


def selector_range(plugin, parameter):
    require(classification(plugin, parameter) == "model_state_restore_required",
            "Use an exact existing model_state_restore_required selector")
    require(parameter.get("native_kind") in ("Integer", "List")
            and parameter.get("enabled") is True and parameter.get("readOnly") is False
            and parameter.get("writable") is True and parameter.get("support", {}).get("raw") is True,
            "A model selector must be an enabled writable raw Integer or List")
    selector_schema(parameter)
    lower = exact_integer(parameter["native_min"], parameter.get("min"))
    upper = exact_integer(parameter["native_max"], parameter.get("max"))
    current = exact_integer(parameter["native_value"], parameter.get("value"))
    require(lower <= current <= upper and 1 <= upper - lower + 1 <= MAX_CHOICES,
            "Model selector range must contain its current value and at most 256 integral choices")
    return lower, upper, current


class ModelRunner(StateRunner):
    def __init__(self, client, journal, scratch_title, prepare_midi=False,
                 settle_attempts=8, settle_interval=0.1, read_only_observations=False):
        require(type(read_only_observations) is bool, "Read-only observations require an explicit boolean mode")
        super().__init__(client, journal, scratch_title, prepare_midi,
                         settle_attempts, settle_interval, read_only_observations)
        self.summary.update(mode="finite_exposed_model_ranges", source_hashes=source_hashes(),
                            choice_range_scope="current_complete_typed_exposed_selector_descriptor",
                            complete_model_domain_verified=False, audible_effects_verified=False,
                            restoration="same_frozen_capsule_after_each_nonbaseline_choice")

    def snapshot(self, target, previous=None):
        legacy = (ProviderSnapshot(previous.target, previous.source, previous.instance, previous.records)
                  if previous is not None else None)
        observed = super().snapshot(target, legacy)
        try:
            projected = project_snapshot(observed)
        except DigestProjectionError as error:
            raise AcceptanceFailure("Complete typed model baseline is unavailable: " + str(error)) from error
        result = ModelSnapshot(observed.target, observed.source, observed.instance, observed.records,
                               projected["schema_sha256"], projected)
        if previous is not None and result.schema != previous.schema:
            raise ContextChanged("Complete typed model provider schema changed")
        return result

    def wait_for_baseline(self, baseline, transaction):
        field = "non_read_only_values_sha256" if self.read_only_observations else "all_values_sha256"
        matching = 0
        last_difference = None
        for attempt in range(self.settle_attempts):
            self.fixture_check(baseline.target)
            current = self.snapshot(baseline.target)
            difference = snapshot_difference(baseline, current, self.read_only_observations)
            projected_values_equal = current.projection[field] == baseline.projection[field]
            match = (not difference["schema_changed"] and not difference["changed_parameters"]
                     and projected_values_equal)
            matching = matching + 1 if match else 0
            last_difference = {**difference, "projected_values_equal": projected_values_equal,
                               "value_hash_field": field, "expected_value_sha256": baseline.projection[field],
                               "observed_value_sha256": current.projection[field]}
            self.journal.append("state_baseline_observation", transaction=transaction,
                                attempt=attempt + 1, matches=match, consecutive_matches=matching,
                                schema_sha256=current.schema, **last_difference)
            if matching == 2:
                return current
            if attempt + 1 < self.settle_attempts and self.settle_interval:
                time.sleep(self.settle_interval)
        raise RestorationFailure("Two consecutive complete typed model baselines were not restored: "
                                 + json.dumps(last_difference, ensure_ascii=False))

    def read_parameter(self, provider, original):
        fresh = super().read_parameter(provider, original)
        try:
            metadata_equal = selector_schema(fresh) == selector_schema(original)
        except AcceptanceFailure as error:
            raise ContextChanged("Typed model selector metadata became incomplete") from error
        if ("access_metadata_complete" in fresh and fresh["access_metadata_complete"] is not True
                or not metadata_equal):
            raise ContextChanged("Typed model selector metadata changed")
        return fresh

    def build_plan(self, baseline, parameter):
        require(parameter.get("access_metadata_complete") is True,
                "The complete model selector requires authoritative native access metadata")
        lower, upper, current = selector_range(baseline.target["plugin"], parameter)
        choices = []
        values = list(range(lower, upper + 1))
        for offset in range(0, len(values), FORMAT_CHUNK):
            requested = values[offset:offset + FORMAT_CHUNK]
            self.fixture_check(baseline.target)
            receipt = self.call("plugin_parameter_format", {**baseline.target, "param": parameter["name"],
                                "source": baseline.source, "expected_instance": baseline.instance["object_uid"],
                                "values": requested})
            result = receipt.get("result", {})
            require(isinstance(result, dict) and receipt.get("outcome") == "observed" and not receipt.get("error")
                    and result.get("ok") is True and result.get("instance") == baseline.instance
                    and result.get("source") == baseline.source and result.get("param") == parameter["name"]
                    and result.get("current_unchanged") is True,
                    "Native model formatting did not preserve the bound observed selector")
            rows = result.get("values")
            require(isinstance(rows, list) and len(rows) == len(requested), "Native model formatting coverage is incomplete")
            for desired, row in zip(requested, rows):
                require(isinstance(row, dict) and type(row.get("requested_value")) in (int, float)
                        and math.isfinite(row["requested_value"]) and row["requested_value"] == desired
                        and isinstance(row.get("text"), str)
                        and len(row["text"].encode("utf-16-le")) // 2 <= 4096,
                        "Native model formatting order or label is malformed")
                require(exact_integer(row.get("requested_native_value")) == desired
                        and exact_integer(row.get("native_value")) == desired,
                        "Native model formatting coerced a requested integral choice")
                choices.append(ModelChoice(desired, row["text"]))
        return ModelPlan(lower, upper, current, tuple(choices))

    def change_model_choice(self, baseline, parameter, choice, capsule, transaction):
        baseline_value = exact_integer(parameter["native_value"], parameter["value"])
        require(type(choice.value) is int and choice.value != baseline_value,
                "The observed baseline-selected model must never be dispatched as a mutation")
        desired = explicit_candidate(parameter, desired_value=choice.value)
        self.fixture_check(baseline.target)
        fresh = self.read_parameter(baseline, parameter)
        require(value_identity(fresh) == value_identity(parameter), "Model selector drifted from the immutable baseline")
        self.phase(transaction, "Prepared", plugin=baseline.target["plugin"], parameter=parameter["name"],
                   choice=choice.value, requested=desired, source=baseline.source,
                   instance=baseline.instance, schema_sha256=baseline.schema)
        self.phase(transaction, "ChangeInFlight")
        try:
            changed = self.write(baseline, parameter, desired)
        except (UnknownMutation, ContextChanged):
            self.phase(transaction, "UnknownAfterDispatch")
            raise
        native = changed.get("result")
        native = native if isinstance(native, dict) else {}
        if (changed.get("outcome") in UNKNOWN_OUTCOMES
                or native.get("native_outcome") in UNKNOWN_OUTCOMES or native.get("outcome") in UNKNOWN_OUTCOMES):
            self.phase(transaction, "UnknownAfterDispatch")
            raise UnknownMutation("Model dispatch outcome is unknown")
        if changed.get("outcome") == "not_applied":
            self.wait_for_baseline(baseline, transaction)
            self.phase(transaction, "NoWriteConfirmed")
            raise AcceptanceFailure("Model choice was refused; no change/load acceptance was established")
        if changed.get("outcome") not in ("applied_verified", "applied_unverified"):
            self.phase(transaction, "UnknownAfterDispatch")
            raise UnknownMutation("Model choice did not establish a known applied outcome")
        self.phase(transaction, "Changed", reported_outcome=changed["outcome"])
        failure = None
        try:
            observed, facts = self.confirm_changed_readback(baseline, parameter, native.get("after"),
                                                          interval=self.settle_interval)
            expected_identity = (parameter["native_value"]["variant_tag"],
                                 parameter["native_value"]["data"]["kind"],
                                 desired["float_bits"] if "float_bits" in desired else str(choice.value))
            require(value_identity(observed) == expected_identity
                    and exact_integer(observed["native_value"], observed["value"]) == choice.value,
                    "Two fresh model reads did not equal the exact requested native choice")
            self.phase(transaction, "ChangedReadbackConfirmed", choice=choice.value,
                       exact_requested_value=True, fresh_reads=facts["fresh_reads"], observed=observed)
        except (UnknownMutation, ContextChanged):
            self.phase(transaction, "UnknownAfterDispatch")
            raise
        except Exception as error:
            failure = str(error)
        self.restore(baseline, capsule, transaction)
        if failure is not None:
            self.phase(transaction, "StateConfirmed", status="model_change_failed_restored")
            raise AcceptanceFailure(failure)
        self.phase(transaction, "StateConfirmed", status="model_choice_changed_restored_verified", choice=choice.value,
                   same_frozen_capsule=True, consecutive_complete_provider_matches=2,
                   read_only_values=self.summary["read_only_values"], opaque_contents_verified=False)
        self.counts["model_choices_changed_restored_verified"] += 1
        self.journal.append("model_choice_result", transaction=transaction, value=choice.value,
                            status="model_choice_changed_restored_verified")
        self.progress()

    def run_model_case(self, target, parameter_name):
        require(len(self.summary["classes"]) < MAX_CASES, "Use at most 128 finite model cases")
        require(isinstance(parameter_name, str) and parameter_name.strip() and "\0" not in parameter_name
                and len(parameter_name.encode()) <= 512, "Use one exact bounded model parameter name")
        self.fixture_check(target, setup=True)
        baseline = self.snapshot(target)
        parameter = self.selected_parameter(baseline, parameter_name, allow_model=True, explicit_desired=True)
        require(classification(target["plugin"], parameter) == "model_state_restore_required",
                "Only existing classified model selectors belong in this workflow")
        transaction = f"model-{len(self.summary['classes']):03d}"
        reference = self.journal.baseline(transaction, baseline.serialized())
        plan = self.build_plan(baseline, parameter)
        plan_reference = self.journal.baseline(transaction + "-choices", {
            "target": baseline.target, "instance": baseline.instance, "source": baseline.source,
            "parameter": parameter_name, "lower": plan.lower, "upper": plan.upper,
            "count_from_typed_range": len(plan.choices), "observed_baseline_choice": plan.baseline_value,
            "complete_model_domain_verified": False,
            "choices": [{"value": choice.value, "label": choice.label,
                         "baseline_observation_only": choice.value == plan.baseline_value} for choice in plan.choices]})
        selected = tuple(choice for choice in plan.choices if choice.value != plan.baseline_value)
        row = {"plugin": target["plugin"], "parameter": parameter_name, "source": baseline.source,
               "status": "model_case_running", "baseline": reference, "choice_plan": plan_reference,
               "provider_indices_compared": len(baseline.records), "observed_choices": len(plan.choices),
               "baseline_choice_observed": True, "selected_nonbaseline_choices": len(selected),
               "verified_nonbaseline_choices": 0, "complete_model_domain_verified": False,
               "opaque_contents_verified": False, "read_only_values": self.summary["read_only_values"],
               "state_token_released": False, "released_token_refused": False}
        self.summary["classes"].append(row)
        self.counts["baseline_model_choices_observed"] += 1
        self.journal.append("model_baseline_choice_observed", transaction=transaction,
                            value=plan.baseline_value, mutation_dispatched=False, choice_plan=plan_reference)
        self.wait_for_baseline(baseline, transaction)
        if not selected:
            row["status"] = "model_case_observed_facade_only"
            self.counts["model_cases_observed_facade_only"] += 1
            self.progress()
            return row
        capsule = self.capture(baseline)
        self.phase(transaction, "Captured", token=capsule.token, byte_count=capsule.byte_count,
                   sha256=capsule.sha256, baseline=reference, choice_plan=plan_reference)
        self.wait_for_baseline(baseline, transaction)
        for choice in selected:
            self.change_model_choice(baseline, parameter, choice, capsule, transaction + "-choice-" + str(choice.value))
            row["verified_nonbaseline_choices"] += 1
        self.wait_for_baseline(baseline, transaction)
        self.release(baseline, capsule)
        row["state_token_released"] = True
        self.verify_released_token(baseline, capsule, transaction)
        row.update(status="model_case_verified", released_token_refused=True)
        self.phase(transaction, "StateConfirmed", status="model_case_verified", baseline_choice_mutated=False,
                   verified_nonbaseline_choices=len(selected), state_token_released=True,
                   released_token_refused=True, opaque_contents_verified=False)
        self.counts["model_cases_verified"] += 1
        self.progress()
        return row

    def finish_models(self):
        verified = bool(self.summary["classes"]) and all(row["status"] == "model_case_verified"
                                                       for row in self.summary["classes"])
        self.summary.update(status="model_cases_verified" if verified else "model_cases_partial",
                            completed_at=utc_now(), all_writable_parameters_verified=False)
        self.progress()
        self.journal.append("run_completed", summary=self.summary)
        return 0 if verified else 2


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default="target/release/fender-studio-pro-mcp")
    parser.add_argument("--fixtures", required=True)
    parser.add_argument("--scratch-title", required=True)
    parser.add_argument("--output-dir", default=".local/live/plugin-models-" + datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ"))
    parser.add_argument("--plugin", action="append", required=True, help="Exact fixture plugin; at most 16 distinct plugins")
    parser.add_argument("--parameter", required=True, help="Exact existing classified model selector")
    parser.add_argument("--pause", action="store_true")
    parser.add_argument("--prepare-midi-fixtures", action="store_true")
    parser.add_argument("--settle-attempts", type=int, default=8)
    parser.add_argument("--settle-interval", type=float, default=0.1)
    parser.add_argument("--read-only-observations", action="store_true")
    args = parser.parse_args(argv)
    validate_settlement(args.settle_attempts, args.settle_interval)
    selected = list(dict.fromkeys(args.plugin))
    require(0 < len(selected) <= 16, "Use 1..16 distinct model fixtures")
    fixtures = json.loads(Path(args.fixtures).read_text())
    require(isinstance(fixtures, list) and 0 < len(fixtures) <= 128
            and all(isinstance(item, dict) and isinstance(item.get("plugin"), str) for item in fixtures),
            "Expected 1..128 finite fixture targets")
    require(all(sum(item["plugin"] == plugin for item in fixtures) == 1 for plugin in selected),
            "Every model plugin must match one exact fixture")
    targets = [{key: value for key, value in next(item for item in fixtures if item["plugin"] == plugin).items()
                if not key.startswith("test_")} for plugin in selected]
    journal = Journal(args.output_dir)
    client = runner = None
    try:
        binary = Path(args.binary).resolve()
        journal.append("run_started", mode="finite_exposed_model_ranges", source_hashes=source_hashes(),
                       binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                       scope={"plugins": selected, "parameter": args.parameter},
                       complete_model_domain_verified=False, opaque_contents_verified=False)
        client = McpClient([str(binary), "serve"], timeout=45)
        print("MCP owner ready", flush=True)
        if args.pause:
            input("Open exactly the named disposable scratch song, then press Enter: ")
        runner = ModelRunner(client, journal, args.scratch_title, args.prepare_midi_fixtures,
                             args.settle_attempts, args.settle_interval, args.read_only_observations)
        runner.initialize()
        for index, target in enumerate(targets, 1):
            print(f"[{index}/{len(targets)}] {target['plugin']}", flush=True)
            runner.run_model_case(target, args.parameter)
        code = runner.finish_models()
        print(json.dumps({"status": runner.summary["status"], "counts": dict(runner.counts),
                          "all_writable_parameters_verified": False, "complete_model_domain_verified": False}), flush=True)
        return code
    except Exception as error:
        journal.append("run_failed", error_type=type(error).__name__, message=str(error))
        if runner is not None:
            runner.summary.update(status="failed", error_type=type(error).__name__, error=str(error), completed_at=utc_now())
            runner.progress()
        print(f"Model acceptance stopped: {type(error).__name__}: {error}", file=sys.stderr, flush=True)
        return 1
    finally:
        if client is not None:
            client.close()
        journal.close()


if __name__ == "__main__":
    sys.exit(main())
