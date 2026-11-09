#!/usr/bin/env python3
"""Verify each eligible scalar using one frozen native state per scratch fixture.

Each change needs two independent stable reads. Before advancing, a capsule load
must restore two complete provider baselines. Raw state and receipts stay local.
Models and dedicated actions remain separate incomplete workflows.
The opt-in digest path checks two fresh native exposed-parameter projections and
full pages around capture, every 32 cases, and release; opaque state is unproved.
"""
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re
import sys

from mcp_client import McpClient
from parameter_digest import DigestProjectionError, HASH_FIELDS, project_snapshot
from verify_all_parameters import (
    AcceptanceFailure, ContextChanged, Journal, RestorationFailure, UnknownMutation, candidate,
    classification, parameter_schema, provider_index, require, requested_value, raw_value,
    utc_now, value_identity,
)
from verify_plugin_states import StateRunner, snapshot_difference, validate_settlement


MAX_FIXTURES = 60
MAX_SCALAR_CORRECTIONS = 32
MAX_REPAIR_PASSES = 3
MAX_DIGEST_CASES_BETWEEN_FULL_CHECKS = 32


def validate_repair_parameters(parameters):
    require(parameters is None or isinstance(parameters, dict), "Repair parameters must map exact plugin names to parameter names")
    result = {}
    for plugin, names in (parameters or {}).items():
        require(isinstance(plugin, str) and plugin.strip() and "\0" not in plugin and len(plugin.encode()) <= 256,
                "Repair plugin names require 1..256 UTF-8 bytes without NUL")
        require(isinstance(names, (set, frozenset, tuple, list)) and names, "Each repair plugin requires explicit parameter names")
        require(all(isinstance(name, str) and name.strip() and "\0" not in name and len(name.encode()) <= 512 for name in names),
                "Repair parameter names require 1..512 UTF-8 bytes without NUL")
        result[plugin] = frozenset(names)
    require(sum(len(names) for names in result.values()) <= MAX_SCALAR_CORRECTIONS,
            "Configure at most 32 exact plugin/parameter repair pairs")
    return result


def parse_repair_parameters(pairs):
    require(len(pairs) <= MAX_SCALAR_CORRECTIONS, "Configure at most 32 repair pairs")
    result = {}
    for pair in pairs:
        parts = pair.split(":", 1)
        require(len(parts) == 2, "Use PLUGIN:PARAM for each repair pair")
        result.setdefault(parts[0], set()).add(parts[1])
    return validate_repair_parameters(result)


class ExhaustiveStateRunner(StateRunner):
    def __init__(self, client, journal, scratch_title, prepare_midi=False,
                 settle_attempts=8, settle_interval=0.1, read_only_observations=False,
                 verify_released=False, repair_captured_scalars=False, repair_parameters=None,
                 use_parameter_digest=False):
        require(type(use_parameter_digest) is bool, "Parameter digest verification requires an explicit boolean mode")
        repair_parameters = validate_repair_parameters(repair_parameters)
        require(not repair_captured_scalars or bool(repair_parameters), "Captured scalar repair requires a nonempty explicit repair allowlist")
        require(not repair_parameters or repair_captured_scalars, "A repair allowlist requires the explicit captured scalar repair mode")
        super().__init__(client, journal, scratch_title, prepare_midi,
                         settle_attempts, settle_interval, read_only_observations)
        self.verify_released = verify_released
        self.repair_captured_scalars = repair_captured_scalars
        self.repair_parameters = repair_parameters
        self.used_repair_parameters = {}
        self.use_parameter_digest = use_parameter_digest
        self._digest_baseline = self._digest_projection = None
        self._force_full_baseline_checks = 0
        self.summary.update(mode="all_parameter_state_capsules",
                            restoration="capsule_plus_scalar_correction" if repair_captured_scalars
                            else "same_frozen_capsule_after_each_applied_change",
                            before_next_parameter="two_complete_provider_baselines",
                            all_writable_parameters_verified=False,
                            repair_parameters={plugin: sorted(names) for plugin, names in sorted(repair_parameters.items())},
                            repair_parameters_used={})
        if use_parameter_digest:
            self.summary.update(parameter_verification="native_parameter_digest_with_full_page_checkpoints",
                                before_next_parameter="two_fresh_native_parameter_digest_matches",
                                full_page_checkpoint_max_cases=MAX_DIGEST_CASES_BETWEEN_FULL_CHECKS,
                                atomic_snapshot=False)

    def parameter_projection(self, provider):
        try:
            return project_snapshot(provider)
        except DigestProjectionError as error:
            raise AcceptanceFailure("Complete exact parameter digest projection is unavailable: " + str(error)) from error

    def finish(self, write_roundtrips):
        if self.use_parameter_digest and write_roundtrips:
            require(all(not row.get("selected_for_write")
                        or row.get("status") == "eligible_scalar_state_roundtrips_completed"
                        for row in self.summary["classes"]),
                    "Digest fixture end, release or negative-token verification is incomplete")
        return super().finish(write_roundtrips)

    def validate_digest_contract(self, expected, native):
        require(isinstance(native, dict) and native.get("ok") is True
                and native.get("outcome") == "observed" and "error" not in native,
                "Native parameter digest did not return a successful observed result")
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
        compared = ("non_read_only_values_sha256",) if self.read_only_observations else HASH_FIELDS[1:]
        return all(native[key] == expected[key] for key in compared)

    def digest_pair(self, baseline, expected, transaction, checkpoint):
        """Two fresh reads; any mismatch stops without another mutation or load."""
        for attempt in (1, 2):
            self.fixture_check(baseline.target)
            receipt = self.call("plugin_parameter_digest", {**baseline.target, "source": baseline.source,
                                "expected_instance": baseline.instance["object_uid"]})
            require(receipt.get("outcome") == "observed" and "error" not in receipt,
                    "Parameter digest receipt is not an observed read")
            native = receipt.get("result")
            matches = self.validate_digest_contract(expected, native)
            self.journal.append("parameter_digest_observation", transaction=transaction, checkpoint=checkpoint,
                                attempt=attempt, fresh_rpc=self.call_count, matches=matches,
                                consecutive_matches=attempt if matches else 0,
                                all_values_equal=native["all_values_sha256"] == expected["all_values_sha256"],
                                read_only_values=self.summary["read_only_values"],
                                **{key: native[key] for key in HASH_FIELDS},
                                atomic_snapshot=False, opaque_contents_verified=False)
            if not matches:
                current = self.snapshot(baseline.target, baseline)
                projection = self.parameter_projection(current)
                self.journal.append("parameter_digest_mismatch_diagnosis", transaction=transaction,
                                    checkpoint=checkpoint, diagnostic_only=True,
                                    **snapshot_difference(baseline, current, self.read_only_observations),
                                    **{key: projection[key] for key in HASH_FIELDS})
                raise RestorationFailure("Fresh parameter digest did not match the frozen exposed provider projection")
        self.journal.append("parameter_digest_baseline_confirmed", transaction=transaction, checkpoint=checkpoint,
                            consecutive_fresh_digest_matches=2, atomic_snapshot=False, opaque_contents_verified=False)
        return native

    def initialize_parameter_digest(self, baseline, transaction):
        projection = self.parameter_projection(baseline)
        self._digest_baseline, self._digest_projection = baseline, projection
        self.digest_pair(baseline, projection, transaction, "initial_full_baseline")

    def full_digest_checkpoint(self, baseline, transaction, checkpoint, *, require_baseline=True):
        require(self._digest_baseline is baseline, "Parameter digest baseline was not initialized for this fixture")
        current = self.snapshot(baseline.target, baseline)
        projection = self.parameter_projection(current)
        matches = self.validate_digest_contract(self._digest_projection, {**projection, "ok": True, "outcome": "observed"})
        require(not require_baseline or matches, "Full parameter checkpoint differs from the frozen baseline")
        self.digest_pair(baseline, projection, transaction, checkpoint)
        self.journal.append("parameter_digest_full_checkpoint", transaction=transaction, checkpoint=checkpoint,
                            count=len(current.records), full_pages_verified=True,
                            baseline_values_required=require_baseline, consecutive_fresh_digest_matches=2,
                            **{key: projection[key] for key in HASH_FIELDS},
                            atomic_snapshot=False, opaque_contents_verified=False)
        return current

    def wait_for_baseline(self, baseline, transaction):
        if not self.use_parameter_digest:
            return super().wait_for_baseline(baseline, transaction)
        if self._force_full_baseline_checks:
            current = super().wait_for_baseline(baseline, transaction)
            expected, projection = self.parameter_projection(baseline), self.parameter_projection(current)
            require(self.validate_digest_contract(expected, {**projection, "ok": True, "outcome": "observed"}),
                    "Negative token check changed the complete exact provider projection")
            self.digest_pair(baseline, projection, transaction, "negative_token_full_page_confirmation")
            return current
        require(self._digest_baseline is baseline, "Parameter digest baseline was not initialized for this fixture")
        self.digest_pair(baseline, self._digest_projection, transaction, "restored_baseline")
        return baseline

    def verify_released_token(self, baseline, capsule, transaction):
        self._force_full_baseline_checks += 1
        try:
            return super().verify_released_token(baseline, capsule, transaction)
        finally:
            self._force_full_baseline_checks -= 1

    def verify_other_target(self, baseline, capsule, target, transaction):
        self._force_full_baseline_checks += 1
        try:
            return super().verify_other_target(baseline, capsule, target, transaction)
        finally:
            self._force_full_baseline_checks -= 1

    def repairable_parameter(self, baseline, original, current):
        plugin = baseline.target["plugin"]
        require(original["name"] in self.repair_parameters.get(plugin, ()),
                "Captured scalar difference is outside the explicit plugin/parameter repair allowlist")
        for parameter in (original, current):
            category = classification(plugin, parameter)
            require((category == "numeric_scalar" and parameter.get("native_kind") in ("Float", "Integer"))
                    or (category == "literal_string" and parameter.get("name") == "deviceNotes"),
                    "Captured scalar repair excludes models, actions, read-only, disabled and unsupported fields")
        require(parameter_schema(original) == parameter_schema(current), "Captured scalar repair requires the original schema")

    def baseline_correction(self, parameter):
        value_identity(parameter)
        kind = parameter["native_value"]["data"]["kind"]
        if kind == "float":
            return {"float_bits": parameter["native_value"]["data"]["bits_hex"].upper()}
        if kind == "integer":
            return {"value": raw_value(parameter)}
        require(kind == "text" and parameter.get("name") == "deviceNotes", "Unsupported captured scalar correction")
        text = raw_value(parameter)
        require(isinstance(text, str) and "\0" not in text and len(text.encode("utf-16-le")) // 2 <= 4096,
                "Original literal notes exceed the native text bounds")
        return {"text": text}

    def restore(self, baseline, capsule, transaction):
        if not self.repair_captured_scalars:
            return super().restore(baseline, capsule, transaction)
        self.phase(transaction, "RestoreGuard")
        corrections = 0
        try:
            self.fixture_check(baseline.target)
            self.phase(transaction, "RestoreInFlight", restoration="capsule_plus_scalar_correction")
            receipt = self.state_call(baseline, "restore", capsule.token, allow_error=True)
            result = receipt.get("result", {})
            require(receipt.get("outcome") in ("applied_unverified", "applied_verified")
                    and not receipt.get("error") and result.get("ok") is True
                    and result.get("load_accepted") is True and result.get("instance") == baseline.instance,
                    "Native state load was not accepted with a known applied outcome")
            self.phase(transaction, "Restored", native_load_accepted=True,
                       native_verified=result.get("verified"), restoration="capsule_plus_scalar_correction")
            originals = {parameter["name"]: parameter for parameter in baseline.records}
            for repair_pass in range(MAX_REPAIR_PASSES + 1):
                self.fixture_check(baseline.target)
                current = (self.full_digest_checkpoint(baseline, transaction, "hybrid_repair_prevalidation",
                                                      require_baseline=False) if self.use_parameter_digest
                           else self.snapshot(baseline.target, baseline))
                difference = snapshot_difference(baseline, current, self.read_only_observations)
                self.journal.append("captured_scalar_repair_observation", transaction=transaction,
                                    completed_passes=repair_pass, corrections=corrections, **difference)
                if not difference["changed_parameters"]:
                    self.wait_for_baseline(baseline, transaction)
                    if self.use_parameter_digest:
                        self.full_digest_checkpoint(baseline, transaction, "hybrid_repair_complete")
                    self.phase(transaction, "RestoreReadbackConfirmed", restoration="capsule_plus_scalar_correction",
                               corrections=corrections, completed_repair_passes=repair_pass,
                               compared="all_provider_schema_and_exact_writable_values" if self.read_only_observations
                               else "all_current_provider_indices_schema_and_exact_native_values",
                               consecutive_matches=2, opaque_contents_verified=False)
                    if corrections:
                        self.counts["capsule_restorations_with_scalar_correction"] += 1
                    return {"corrections": corrections, "repair_passes": repair_pass}
                changed = difference["changed_parameters"]
                self.phase(transaction, "RestorePartialKnown", restoration="capsule_plus_scalar_correction",
                           next_repair_pass=repair_pass + 1, corrections=corrections,
                           changed_parameters=changed, read_only_changed=difference["read_only_changed"])
                require(repair_pass < MAX_REPAIR_PASSES, "Captured scalar repair exceeded 3 passes")
                require(corrections + len(changed) <= MAX_SCALAR_CORRECTIONS, "Captured scalar repair exceeded 32 corrections")
                present = {parameter["name"]: parameter for parameter in current.records}
                for name in changed:
                    self.repairable_parameter(baseline, originals[name], present[name])
                    self.baseline_correction(originals[name])
                for name in changed:
                    original = originals[name]
                    self.fixture_check(baseline.target)
                    fresh = self.read_parameter(baseline, original)
                    self.repairable_parameter(baseline, original, fresh)
                    if value_identity(fresh) == value_identity(original):
                        continue
                    correction = self.baseline_correction(original)
                    corrections += 1
                    self.counts["captured_scalar_correction_attempts"] += 1
                    self.journal.append("captured_scalar_correction_in_flight", transaction=transaction,
                                        repair_pass=repair_pass + 1, correction=corrections,
                                        plugin=baseline.target["plugin"], parameter=name, requested=correction)
                    written = self.write(baseline, original, correction)
                    require(written.get("outcome") == "applied_verified", "Captured scalar correction did not verify")
                    after = written.get("result", {}).get("after")
                    require(isinstance(after, dict) and parameter_schema(after) == parameter_schema(original)
                            and value_identity(after) == value_identity(original),
                            "Captured scalar correction receipt differs from the exact original")
                    self.fixture_check(baseline.target)
                    readback = self.read_parameter(baseline, original)
                    require(value_identity(readback) == value_identity(original),
                            "Captured scalar correction independent readback differs from the exact original")
                    self.counts["captured_scalar_corrections"] += 1
                    self.used_repair_parameters.setdefault(baseline.target["plugin"], set()).add(name)
                    self.summary["repair_parameters_used"] = {plugin: sorted(names) for plugin, names in sorted(self.used_repair_parameters.items())}
                    self.journal.append("captured_scalar_correction_confirmed", transaction=transaction,
                                        repair_pass=repair_pass + 1, correction=corrections,
                                        plugin=baseline.target["plugin"], parameter=name, exact_original_independently_confirmed=True)
        except (UnknownMutation, ContextChanged):
            self.phase(transaction, "UnknownAfterDispatch")
            raise
        except Exception as error:
            self.phase(transaction, "RestorationFailed", error_type=type(error).__name__, message=str(error))
            if isinstance(error, RestorationFailure):
                raise
            raise RestorationFailure("Bounded captured scalar correction did not restore the original provider baseline") from error

    def state_roundtrip(self, baseline, capsule, reference, parameter, transaction):
        self.fixture_check(baseline.target)
        original = self.read_parameter(baseline, parameter)
        require(value_identity(original) == value_identity(parameter),
                "Selected parameter drifted from the frozen provider baseline")
        change, _ = candidate(original)
        self.phase(transaction, "Prepared", plugin=baseline.target["plugin"],
                   parameter=parameter["name"], index=provider_index(parameter),
                   requested=change, baseline=reference, source=baseline.source,
                   instance=baseline.instance, schema_sha256=baseline.schema)
        self.phase(transaction, "ChangeInFlight")
        try:
            changed = self.write(baseline, original, change)
        except (UnknownMutation, ContextChanged):
            self.phase(transaction, "UnknownAfterDispatch")
            raise
        except Exception as error:
            self.phase(transaction, "UnknownAfterDispatch", error_type=type(error).__name__, message=str(error))
            raise UnknownMutation("Scalar dispatch did not establish a known outcome") from error
        if changed.get("outcome") == "not_applied":
            try:
                self.wait_for_baseline(baseline, transaction)
            except (UnknownMutation, ContextChanged):
                self.phase(transaction, "UnknownAfterDispatch")
                raise
            except Exception as error:
                self.phase(transaction, "UnknownAfterDispatch", reason="rejected write did not preserve the full baseline")
                raise UnknownMutation("Rejected scalar write did not independently preserve the full baseline") from error
            self.phase(transaction, "NoWriteConfirmed", full_provider_unchanged=True, consecutive_matches=2)
            return {"transaction": transaction, "parameter": parameter["name"],
                    "status": "write_rejected", "baseline": reference,
                    "full_provider_unchanged": True, "capsule_load_attempted": False}
        if changed.get("outcome") not in ("applied_verified", "applied_unverified"):
            self.phase(transaction, "UnknownAfterDispatch")
            raise UnknownMutation("Scalar dispatch did not establish a known applied outcome")
        self.phase(transaction, "Changed", reported_outcome=changed["outcome"])
        failure = None
        observed = settlement = None
        try:
            after = changed.get("result", {}).get("after")
            if isinstance(after, dict) and parameter_schema(after) != parameter_schema(original):
                raise ContextChanged("Scalar setter changed the provider parameter schema")
            observed, settlement = self.confirm_changed_readback(
                baseline, original, after,
                interval=self.settle_interval)
            self.phase(transaction, "ChangedReadbackConfirmed", observed=observed, settlement=settlement)
        except (UnknownMutation, ContextChanged):
            self.phase(transaction, "UnknownAfterDispatch")
            raise
        except Exception as error:
            failure = str(error)
        restoration = self.restore(baseline, capsule, transaction)
        status = "changed_readback_restored" if failure is None else "change_failed_restored"
        self.phase(transaction, "StateConfirmed", status=status, same_frozen_capsule=True,
                   restoration=self.summary["restoration"],
                   consecutive_complete_provider_matches=2,
                   parameter_verification="native_parameter_digest" if self.use_parameter_digest else "full_pages",
                   read_only_values=self.summary["read_only_values"], opaque_contents_verified=False)
        return {"transaction": transaction, "parameter": parameter["name"], "status": status,
                "reason": failure, "baseline": reference, "capsule_load_attempted": True,
                "restoration": self.summary["restoration"],
                "scalar_corrections": restoration["corrections"] if restoration else 0,
                "settlement": settlement,
                "quantized": observed is not None and raw_value(observed) != requested_value(change)}

    def run_fixture(self, target, parameters=None):
        self.track_parameter_filter(parameters)
        require(len(self.summary["classes"]) < MAX_FIXTURES, "Use at most60 fixture summaries")
        self.fixture_check(target, setup=True)
        if target["plugin"] == "Splitter":
            self.counts["separate_graph_workflow_required"] += 1
            row = {"plugin": target["plugin"], "status": "separate_graph_workflow_required", "selected_for_write": 0}
            self.summary["classes"].append(row)
            self.journal.append("fixture_result", **row)
            self.progress()
            return row
        if target["plugin"].startswith("External Effect "):
            self.call("plugin_editor_open", target, mutation=True)
        baseline = self.snapshot(target)
        key = f"parameter-state-{len(self.summary['classes']):03d}"
        if self.use_parameter_digest:
            self.initialize_parameter_digest(baseline, key)
        self.track_parameter_filter(parameters, baseline.records)
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
            self.journal.append("parameter_classification", plugin=target["plugin"],
                                index=provider_index(parameter), name=parameter["name"], category=category,
                                advertised_writable=parameter.get("writable"))
            if category in ("numeric_scalar", "literal_string") and (not parameters or parameter["name"] in parameters):
                eligible.append(parameter)
        row = {"plugin": target["plugin"], "status": "audited", "source": baseline.source,
               "count": len(baseline.records), "categories": dict(categories), "selected_for_write": len(eligible),
               "schema_sha256": baseline.schema, "baseline": reference, "results": {},
               "capsule_captured": False, "capsule_released": False,
               "restoration": self.summary["restoration"],
               "read_only_values": self.summary["read_only_values"], "opaque_contents_verified": False}
        if self.use_parameter_digest:
            row.update(parameter_verification="native_parameter_digest_with_full_page_checkpoints",
                       consecutive_fresh_digest_matches_required=2, atomic_snapshot=False,
                       full_page_checkpoint_max_cases=MAX_DIGEST_CASES_BETWEEN_FULL_CHECKS)
        self.summary["classes"].append(row)
        self.progress()
        if not eligible:
            if self.use_parameter_digest:
                self.full_digest_checkpoint(baseline, key, "fixture_end_no_eligible_scalars")
            row["status"] = "no_selected_eligible_scalars"
            self.journal.append("fixture_result", **row)
            self.progress()
            return row
        fixture_transaction = key + "-capsule"
        self.phase(fixture_transaction, "Prepared", plugin=target["plugin"], baseline=reference,
                   source=baseline.source, instance=baseline.instance, schema_sha256=baseline.schema)
        if self.use_parameter_digest:
            self.full_digest_checkpoint(baseline, fixture_transaction, "before_capture")
        capsule = self.capture(baseline)
        self.phase(fixture_transaction, "Captured", token=capsule.token,
                   byte_count=capsule.byte_count, sha256=capsule.sha256)
        row.update(status="running", capsule_captured=True,
                   capsule_byte_count=capsule.byte_count, capsule_sha256=capsule.sha256)
        if self.use_parameter_digest:
            self.full_digest_checkpoint(baseline, fixture_transaction, "immediately_after_capture")
        self.wait_for_baseline(baseline, fixture_transaction)
        self.journal.append("capture_baseline_confirmed", transaction=fixture_transaction,
                            consecutive_complete_provider_matches=2,
                            parameter_verification="native_parameter_digest" if self.use_parameter_digest else "full_pages")
        self.progress()
        results = Counter()
        for case_index, parameter in enumerate(eligible, 1):
            transaction = f"{key}-{provider_index(parameter):06d}"
            result = self.state_roundtrip(baseline, capsule, reference, parameter, transaction)
            results[result["status"]] += 1
            self.counts[result["status"]] += 1
            if result["status"] == "changed_readback_restored":
                self.counts["changed_load_independently_restored"] += 1
                if result["settlement"]["quantized_settlement"]:
                    self.counts["quantized_settlements"] += 1
            self.journal.append("parameter_state_result", **result)
            row["results"] = dict(results)
            self.progress()
            if self.use_parameter_digest and case_index % MAX_DIGEST_CASES_BETWEEN_FULL_CHECKS == 0:
                self.full_digest_checkpoint(baseline, fixture_transaction, "completed_cases_" + str(case_index))
        if self.use_parameter_digest:
            self.full_digest_checkpoint(baseline, fixture_transaction, "fixture_end_before_release")
        self.release(baseline, capsule)
        row["capsule_released"] = True
        if self.use_parameter_digest:
            self.full_digest_checkpoint(baseline, fixture_transaction, "after_release")
        if self.verify_released:
            self.verify_released_token(baseline, capsule, fixture_transaction)
        row.update(status="eligible_scalar_state_roundtrips_completed", released_token_refused=self.verify_released)
        self.phase(fixture_transaction, "StateConfirmed", status=row["status"], capsule_released=True,
                   released_token_refused=self.verify_released, parameters_processed=len(eligible))
        self.counts["fixtures_state_workflow_completed"] += 1
        self.journal.append("fixture_result", **row)
        self.progress()
        return row


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default="target/release/fender-studio-pro-mcp")
    parser.add_argument("--fixtures", required=True)
    parser.add_argument("--scratch-title", required=True)
    parser.add_argument("--output-dir", default=".local/live/all-parameter-states-" + datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ"))
    parser.add_argument("--write-roundtrips", action="store_true", required=True,
                        help="Explicitly change eligible scalars and reload the same frozen state after each applied change")
    parser.add_argument("--plugin", action="append", help="Select an exact fixture plugin; repeat for more")
    parser.add_argument("--parameter", action="append", help="Select an exact eligible parameter; repeat for more while still auditing every index")
    parser.add_argument("--pause", action="store_true", help="Start the MIDI owner before launching the DAW, then press Enter")
    parser.add_argument("--prepare-midi-fixtures", action="store_true")
    parser.add_argument("--read-only-observations", action="store_true")
    parser.add_argument("--use-parameter-digest", action="store_true",
                        help="Require two fresh exact native digest observations and full-page checkpoints at capture, every 32 cases and release")
    parser.add_argument("--settle-attempts", type=int, default=8)
    parser.add_argument("--settle-interval", type=float, default=0.1)
    parser.add_argument("--verify-released-token", action="store_true", help="Once per fixture prove that the released token is refused without changing its provider")
    parser.add_argument("--repair-captured-scalars", action="store_true",
                        help="After one accepted capsule load, permit at most 32 exact Float/Integer/literal notes corrections across 3 passes; never rebase the original baseline")
    parser.add_argument("--repair-parameter", action="append", default=[], metavar="PLUGIN:PARAM",
                        help="Explicitly allow this exact plugin/parameter pair for repair; repeat for at most 32 pairs")
    args = parser.parse_args(argv)
    validate_settlement(args.settle_attempts, args.settle_interval)
    repair_parameters = parse_repair_parameters(args.repair_parameter)
    require(not args.repair_captured_scalars or bool(repair_parameters), "Captured scalar repair requires explicit repair pairs")
    require(not repair_parameters or args.repair_captured_scalars, "Repair pairs require --repair-captured-scalars")
    fixtures = json.loads(Path(args.fixtures).read_text())
    require(isinstance(fixtures, list) and 0 < len(fixtures) <= MAX_FIXTURES, "Expected1..60 fixture targets")
    require(all(isinstance(fixture, dict) and isinstance(fixture.get("plugin"), str) for fixture in fixtures),
            "Invalid fixture targets")
    selected = list(dict.fromkeys(args.plugin or [fixture["plugin"] for fixture in fixtures]))
    require(all(sum(fixture["plugin"] == plugin for fixture in fixtures) == 1 for plugin in selected),
            "Every selected plugin must match one exact fixture")
    targets = [{key: value for key, value in next(fixture for fixture in fixtures if fixture["plugin"] == plugin).items()
                if key not in ("test_parameter", "test_bypass")} for plugin in selected]
    journal = Journal(args.output_dir)
    runner = client = None
    try:
        binary = Path(args.binary).resolve()
        journal.append("run_started", schema=1, mode="all_parameter_state_capsules",
                       binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                       scope={"plugins": args.plugin, "parameters": args.parameter},
                       restoration="capsule_plus_scalar_correction" if args.repair_captured_scalars
                       else "same_frozen_capsule_after_each_applied_change",
                       repair_parameters={plugin: sorted(names) for plugin, names in sorted(repair_parameters.items())},
                       before_next_parameter="two_fresh_native_parameter_digest_matches" if args.use_parameter_digest
                       else "two_complete_provider_baselines",
                       parameter_verification="native_parameter_digest_with_full_page_checkpoints" if args.use_parameter_digest else "full_pages",
                       atomic_snapshot=False, opaque_contents_verified=False,
                       read_only_values="observed_not_restored" if args.read_only_observations else "exact_comparison")
        client = McpClient([str(binary), "serve"], timeout=45)
        print("MCP owner ready", flush=True)
        if args.pause:
            input("Open exactly the named scratch song, then press Enter: ")
        runner = ExhaustiveStateRunner(client, journal, args.scratch_title, args.prepare_midi_fixtures,
                                       args.settle_attempts, args.settle_interval, args.read_only_observations,
                                       args.verify_released_token, args.repair_captured_scalars, repair_parameters,
                                       args.use_parameter_digest)
        runner.summary["scope"] = {"plugins": args.plugin, "parameters": args.parameter}
        runner.initialize()
        for index, target in enumerate(targets, 1):
            print(f"[{index}/{len(targets)}] {target['plugin']}", flush=True)
            runner.run_fixture(target, set(args.parameter or []))
        exit_code = runner.finish(True)
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
