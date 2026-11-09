#!/usr/bin/env python3
"""Verify finite issued preset catalogs or recalls in disposable plugin fixtures.

Start the MCP owner before the DAW. Only issued catalog indices are recalled;
state capsules stay native, and independent full-provider reads prove recovery.
Catalog-only mode lists and releases catalogs without recalling presets.
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
from parameter_digest import DigestProjectionError, project_snapshot
from verify_all_parameters import (
    AcceptanceFailure, ContextChanged, Journal, RestorationFailure,
    UnknownMutation, provider_index, require, utc_now, value_identity,
)
from verify_plugin_states import StateRunner, hexadecimal, snapshot_difference, validate_settlement


CATALOG_SOURCE = "native_slot_preset_catalog"
APPLIED = ("applied_verified", "applied_unverified")


@dataclass(frozen=True)
class PresetCatalog:
    token: str
    entries: tuple


def metadata_only(value):
    if isinstance(value, dict):
        require(not set(value).intersection({"bytes", "state_bytes", "blob", "path", "url",
                                             "location", "file_path", "preset_path"}),
                "Preset operations must return metadata without paths, URLs or state bytes")
        for item in value.values():
            metadata_only(item)
    elif isinstance(value, list):
        for item in value:
            metadata_only(item)


def authoritative_writable(parameter):
    return (parameter.get("access_metadata_complete") is True
            and parameter.get("writable") is True and parameter.get("readOnly") is False)


def writable_changes(baseline, current):
    """Compare exact exposed values of the same native indices and names only."""
    original = {(provider_index(p), p["name"]): p for p in baseline.records}
    changed = []
    for after in current.records:
        before = original.get((provider_index(after), after["name"]))
        if before is None or not all(authoritative_writable(p) for p in (before, after)):
            continue
        if not all(p.get("native_value", {}).get("data", {}).get("kind") in ("float", "integer", "text")
                   for p in (before, after)):
            continue
        if value_identity(before) != value_identity(after):
            changed.append(after["name"])
    return changed


class PresetRunner(StateRunner):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.summary.update(mode="preset_recalls", factory_origin="not_inferred",
                            preset_display_verified=False, opaque_contents_verified=False,
                            all_writable_parameters_verified=False)

    def require_main_zero(self):
        try:
            require(isinstance(self.document, str) and bool(self.document)
                    and isinstance(self.session, str) and bool(self.session),
                    "Main output guard requires a bound document and session")
            channels = self.operation("channels")
            require(isinstance(channels, list) and all(isinstance(row, dict) for row in channels),
                    "Main output guard requires a fresh channels observation")
            main = [row for row in channels if row.get("label") == "메인"]
            require(len(main) == 1, "Main output guard requires one unique 메인 channel")
            volume = main[0].get("volume")
            require(type(volume) in (int, float) and math.isfinite(volume) and volume == 0
                    and "output" in main[0] and main[0]["output"] is None,
                    "Main output must have finite numeric volume exactly zero and explicit output null")
        except (UnknownMutation, ContextChanged):
            raise
        except Exception as error:
            raise ContextChanged("Fresh blocked Main output was not established; no follow-on mutation is permitted") from error

    def state_call(self, provider, action, token=None, allow_error=False):
        if action == "restore":
            self.require_main_zero()
        return super().state_call(provider, action, token, allow_error)

    def preset_call(self, provider, action, *, token=None, index=None, offset=None, allow_error=False):
        args = {**provider.target, "action": action,
                "expected_instance": provider.instance["object_uid"]}
        if token is not None:
            args["catalog_token"] = token
        if index is not None:
            args["index"] = index
        if action == "list":
            args.update(offset=offset, limit=64)
        if action == "apply":
            self.require_main_zero()
        return self.call("plugin_presets", args, mutation=True, allow_error=allow_error)

    def preset_result(self, provider, receipt, *, token=None, index=None):
        result = receipt.get("result", {})
        require(isinstance(result, dict), "Preset operation returned no native result")
        metadata_only(result)
        if result.get("outcome") == "unknown" or result.get("native_outcome") == "unknown":
            raise UnknownMutation("Native preset outcome is unknown; no automatic replay or restore is permitted")
        if result.get("instance") != provider.instance:
            raise ContextChanged("Preset operation resolved a different native Device")
        require(result.get("ok") is True and result.get("source") == CATALOG_SOURCE,
                "Preset operation did not use the native slot catalog")
        require(result.get("outcome") in ("observed", *APPLIED), "Native preset outcome is not known")
        if token is not None:
            require(result.get("catalog_token") == token, "Preset operation returned a different issued catalog")
        if index is not None:
            require(type(result.get("index")) is int and result["index"] == index,
                    "Preset operation returned a different issued index")
        return result

    def catalog_call(self, provider, action, *, token=None, offset=None):
        require(action in ("list", "release"), "Catalog-only mode permits only list and release")
        receipt = self.preset_call(provider, action, token=token, offset=offset, allow_error=True)
        outer, native = receipt.get("outcome"), receipt.get("result")
        allowed = ("observed", *APPLIED) if action == "list" else APPLIED
        if not isinstance(outer, str) or outer not in allowed or not isinstance(native, dict):
            raise UnknownMutation("Catalog lifecycle outcome is malformed or contradictory")
        if native.get("instance") != provider.instance:
            raise ContextChanged("Catalog lifecycle resolved a different native Device")
        if native.get("source") != CATALOG_SOURCE:
            raise UnknownMutation("Catalog lifecycle lacks native catalog provenance")
        metadata_only(native)
        expected = ("observed",) if action == "list" else APPLIED
        outcomes = [native[field] for field in ("outcome", "native_outcome") if field in native]
        if (not outcomes or any(not isinstance(value, str) or value not in expected for value in outcomes)
                or len(set(outcomes)) > 1):
            raise UnknownMutation("Catalog lifecycle native outcome is unknown or contradictory")
        if (receipt.get("error") or native.get("ok") is not True
              or any(native[field] is not False for field in ("accepted",) if field in native)
              or (action == "list" and any(native[field] is not False for field in ("released", "verified") if field in native))
              or (action == "release" and (native.get("released") is not True or native.get("verified") is not True))):
            raise UnknownMutation("Catalog success contains malformed or contradictory phase evidence")
        return receipt

    def released_catalog_list_refusal(self, baseline, catalog):
        receipt = self.preset_call(baseline, "list", token=catalog.token, offset=0, allow_error=True)
        expected_error = {"code": "native_rejected",
                          "message": "The native request was rejected before applying a change; inspect the result errors"}
        expected_result = {"plugin": baseline.target["plugin"], "native_outcome": "not_applied",
                           "errors": [{"code": "preset_request_failed",
                                       "message": "catalog token is expired, released or belongs to another service"}]}
        if (receipt.get("outcome") != "not_applied" or receipt.get("error") != expected_error
                or receipt.get("result") != expected_result):
            raise UnknownMutation("Released catalog list did not return the exact bound public expired-token refusal")
        return receipt

    def discover_catalog(self, baseline, transaction, *, catalog_only=False):
        token, count, offset, entries = None, None, 0, []
        while True:
            self.fixture_check(baseline.target)
            self.phase(transaction, "CatalogPageInFlight", offset=offset)
            call = self.catalog_call if catalog_only else self.preset_call
            receipt = call(baseline, "list", token=token, offset=offset)
            require(receipt.get("outcome") in ("observed", *APPLIED), "Catalog page outcome is not known")
            page = self.preset_result(baseline, receipt, token=token)
            issued = page.get("catalog_token")
            require(hexadecimal(issued, 64) and issued == issued.upper(), "Catalog token is malformed")
            if token is None:
                token = issued
            total = page.get("observed_count")
            require(type(total) is int and 0 <= total <= 4096 and page.get("catalog_complete") is True,
                    "A complete bounded native catalog is required")
            if count is not None and total != count:
                raise ContextChanged("Issued catalog count changed between pages")
            count = total
            require(type(page.get("offset")) is int and page["offset"] == offset, "Catalog offset differs from its request")
            batch = page.get("entries")
            end = min(offset + 64, count)
            require(isinstance(batch, list) and len(batch) == end - offset, "Catalog page coverage is incomplete")
            for expected, entry in enumerate(batch, offset):
                require(isinstance(entry, dict) and type(entry.get("index")) is int and entry["index"] == expected,
                        "Catalog indices must be issued, unique and contiguous")
                require(isinstance(entry.get("name"), str) and isinstance(entry.get("class_key"), str)
                        and isinstance(entry.get("availability"), str) and type(entry.get("read_only")) is bool
                        and type(entry.get("recallable")) is bool, "Catalog entry metadata is incomplete")
                entries.append(dict(entry))
            next_offset = page.get("next")
            require((end == count and next_offset is None)
                    or (end < count and type(next_offset) is int and next_offset == end),
                    "Catalog pagination must advance exactly through its issued entries")
            if next_offset is None:
                break
            offset = next_offset
        catalog = PresetCatalog(token, tuple(entries))
        self.journal.append("preset_catalog_complete", transaction=transaction, catalog_token=token,
                            observed_count=count, catalog_complete=True, factory_origin="not_inferred")
        return catalog

    def selected_entries(self, catalog, indices):
        selected = [1] if indices is None else list(indices)
        require(0 < len(selected) <= 64, "Select 1..64 issued preset indices")
        for index in selected:
            require(type(index) is int and 0 <= index < len(catalog.entries),
                    "Selected preset index is outside the complete issued catalog")
            require(catalog.entries[index]["recallable"] is True, "Selected issued preset is not recallable")
        return [catalog.entries[index] for index in selected]

    def confirm_preset_change(self, baseline, transaction):
        observations = []
        changes = None
        for attempt in range(2):
            if attempt and self.settle_interval:
                time.sleep(self.settle_interval)
            self.fixture_check(baseline.target)
            current = self.snapshot(baseline.target)
            if current.instance != baseline.instance or current.source != baseline.source:
                raise ContextChanged("Preset recall replaced the native provider")
            if observations:
                difference = snapshot_difference(observations[0], current, self.read_only_observations)
                require(not difference["schema_changed"] and not difference["changed_parameters"],
                        "Two fresh whole-provider preset observations are unstable")
            else:
                difference = {"schema_changed": False, "changed_parameters": [], "read_only_changed": []}
            observed_changes = writable_changes(baseline, current)
            changes = observed_changes if changes is None else [name for name in changes if name in observed_changes]
            self.journal.append("preset_changed_observation", transaction=transaction, attempt=attempt + 1,
                                changed_writable_parameters=observed_changes, schema_sha256=current.schema,
                                baseline_schema_changed=current.schema != baseline.schema,
                                read_only_changed=difference["read_only_changed"],
                                exact_native_values=True, opaque_contents_verified=False)
            observations.append(current)
        return {"changed_writable_parameters": changes,
                "baseline_schema_changed": observations[-1].schema != baseline.schema,
                "fresh_whole_provider_reads": 2, "opaque_contents_verified": False}

    def release_catalog(self, baseline, catalog, transaction, *, catalog_only=False):
        self.fixture_check(baseline.target)
        self.phase(transaction, "CatalogReleaseInFlight")
        call = self.catalog_call if catalog_only else self.preset_call
        receipt = call(baseline, "release", token=catalog.token)
        require(receipt.get("outcome") in APPLIED, "Catalog release outcome is not known applied")
        result = self.preset_result(baseline, receipt, token=catalog.token)
        require(result.get("released") is True and result.get("outcome") in APPLIED,
                "Catalog release was not confirmed")

    def catalog_projection(self, snapshot):
        try:
            return project_snapshot(snapshot)
        except DigestProjectionError as error:
            raise AcceptanceFailure("Complete typed catalog provider metadata is unavailable: " + str(error)) from error

    def confirm_catalog_baseline(self, baseline, projected, transaction, boundary):
        field = "non_read_only_values_sha256" if self.read_only_observations else "all_values_sha256"
        for attempt in range(2):
            if attempt and self.settle_interval:
                time.sleep(self.settle_interval)
            self.fixture_check(baseline.target)
            current = self.snapshot(baseline.target, baseline)
            observed = self.catalog_projection(current)
            matches = (observed["schema_sha256"] == projected["schema_sha256"]
                       and observed[field] == projected[field])
            self.journal.append("catalog_baseline_observation", transaction=transaction, boundary=boundary,
                                attempt=attempt + 1, matches=matches, projection=observed,
                                value_hash_field=field, expected_value_sha256=projected[field],
                                read_only_values_changed=observed["all_values_sha256"] != projected["all_values_sha256"])
            require(matches, "Catalog lifecycle changed the complete typed provider baseline")

    def run_catalog_case(self, target):
        require(type(self.read_only_observations) is bool,
                "Read-only observations require an explicit boolean mode")
        require(not self.prepare_midi, "Catalog-only mode cannot prepare MIDI fixtures")
        self.summary.update(mode="preset_catalogs", preset_recalls_verified=False,
                            audible_effects_verified=False, catalog_scope="current_complete_issued_catalog")
        self.fixture_check(target, setup=True)
        baseline = self.snapshot(target)
        projected = self.catalog_projection(baseline)
        transaction = f"catalog-{len(self.summary['classes']):03d}"
        reference = self.journal.baseline(transaction, {**baseline.serialized(), "projection": projected})
        self.phase(transaction, "Prepared", plugin=target["plugin"], baseline=reference,
                   source=baseline.source, instance=baseline.instance, schema_sha256=projected["schema_sha256"])
        try:
            catalog = self.discover_catalog(baseline, transaction, catalog_only=True)
            self.confirm_catalog_baseline(baseline, projected, transaction, "catalog_discovered")
            self.release_catalog(baseline, catalog, transaction, catalog_only=True)
            self.confirm_catalog_baseline(baseline, projected, transaction, "catalog_released")
            self.fixture_check(target)
            self.phase(transaction, "ReleasedCatalogCheckInFlight")
            self.released_catalog_list_refusal(baseline, catalog)
            self.confirm_catalog_baseline(baseline, projected, transaction, "released_catalog_refused")
        except Exception:
            self.phase(transaction, "UnknownAfterDispatch")
            raise
        row = {"plugin": target["plugin"], "status": "catalog_case_verified",
               "catalog_count": len(catalog.entries), "catalog_complete": True,
               "catalog_scope": "current_complete_issued_catalog", "factory_origin": "not_inferred",
               "source": baseline.source, "provider_indices_compared": len(baseline.records),
               "schema_sha256": projected["schema_sha256"], "baseline": reference,
               "fresh_whole_provider_reads_per_boundary": 2, "independent_baseline_observations": 6,
               "released_catalog_refused": True, "preset_recalls_verified": False,
               "read_only_values": self.summary["read_only_values"],
               "opaque_contents_verified": False, "preset_display_verified": False,
               "all_writable_parameters_verified": False, "audible_effects_verified": False}
        self.phase(transaction, "StateConfirmed", status=row["status"], released_catalog_refused=True)
        self.summary["classes"].append(row)
        self.counts[row["status"]] += 1
        self.progress()
        return row

    def confirm_refused_unchanged(self, baseline, receipt, transaction):
        native = receipt.get("result", {})
        if (receipt.get("outcome") != "not_applied"
                or native.get("outcome") == "unknown" or native.get("native_outcome") == "unknown"):
            raise UnknownMutation("Invalid catalog recall was not refused before dispatch")
        try:
            self.wait_for_baseline(baseline, transaction)
        except RestorationFailure as error:
            raise UnknownMutation("Refused preset recall contradicted the unchanged provider baseline") from error

    def verify_released_catalog(self, baseline, catalog, index, transaction):
        self.fixture_check(baseline.target)
        self.phase(transaction, "ReleasedCatalogCheckInFlight")
        receipt = self.preset_call(baseline, "apply", token=catalog.token, index=index, allow_error=True)
        self.confirm_refused_unchanged(baseline, receipt, transaction)
        self.journal.append("released_catalog_refused", transaction=transaction,
                            unchanged_provider_independently_confirmed=True)

    def verify_other_catalog_target(self, baseline, catalog, index, target, transaction):
        require(target != baseline.target, "Wrong-target validation requires a different fixture")
        self.fixture_check(target, setup=True)
        if target["plugin"].startswith("External Effect "):
            self.call("plugin_editor_open", target, mutation=True)
        other = self.snapshot(target)
        require(other.instance["object_uid"] != baseline.instance["object_uid"],
                "Wrong-target fixture resolved the original Device")
        self.journal.baseline(transaction + "-wrong-target", other.serialized())
        self.fixture_check(target)
        self.phase(transaction, "WrongTargetCatalogCheckInFlight")
        receipt = self.preset_call(other, "apply", token=catalog.token, index=index, allow_error=True)
        self.confirm_refused_unchanged(other, receipt, transaction)
        self.fixture_check(baseline.target, setup=True)
        self.wait_for_baseline(baseline, transaction)
        self.journal.append("wrong_target_catalog_refused", transaction=transaction,
                            both_providers_independently_unchanged=True)

    def run_preset_case(self, target, indices=None, *, wrong_target=None):
        self.fixture_check(target, setup=True)
        if target["plugin"].startswith("External Effect "):
            self.call("plugin_editor_open", target, mutation=True)
        baseline = self.snapshot(target)
        transaction = f"preset-{len(self.summary['classes']):03d}"
        reference = self.journal.baseline(transaction, baseline.serialized())
        self.phase(transaction, "Prepared", plugin=target["plugin"], baseline=reference,
                   source=baseline.source, instance=baseline.instance, schema_sha256=baseline.schema)
        try:
            capsule = self.capture(baseline)
            self.phase(transaction, "Captured", token=capsule.token, byte_count=capsule.byte_count,
                       sha256=capsule.sha256, opaque_contents_verified=False)
            catalog = self.discover_catalog(baseline, transaction)
            selected = self.selected_entries(catalog, indices)
            self.wait_for_baseline(baseline, transaction)
            if wrong_target is not None:
                self.verify_other_catalog_target(baseline, catalog, selected[0]["index"], wrong_target, transaction)
            results = []
            for selection, entry in enumerate(selected):
                if selection:
                    self.wait_for_baseline(baseline, transaction)
                self.fixture_check(target)
                self.phase(transaction, "ChangeInFlight", index=entry["index"], selection=selection)
                receipt = self.preset_call(baseline, "apply", token=catalog.token,
                                           index=entry["index"], allow_error=True)
                if receipt.get("outcome") == "not_applied":
                    self.confirm_refused_unchanged(baseline, receipt, transaction)
                    result = {"index": entry["index"], "status": "preset_apply_rejected"}
                else:
                    if receipt.get("outcome") not in APPLIED:
                        raise UnknownMutation("Preset recall did not establish a known applied outcome")
                    self.phase(transaction, "Changed", index=entry["index"])
                    failure = None
                    changed = None
                    try:
                        native = self.preset_result(baseline, receipt, token=catalog.token, index=entry["index"])
                        require(not receipt.get("error") and native.get("accepted") is True
                                and native.get("outcome") in APPLIED,
                                "Preset recall was not accepted with a known native applied outcome")
                        changed = self.confirm_preset_change(baseline, transaction)
                        self.phase(transaction, "ChangedReadbackConfirmed", index=entry["index"], **changed)
                    except (UnknownMutation, ContextChanged):
                        raise
                    except Exception as error:
                        failure = str(error)
                    self.restore(baseline, capsule, transaction)
                    if failure is not None:
                        self.phase(transaction, "StateConfirmed", status="change_failed_restored")
                        raise AcceptanceFailure(failure)
                    status = "preset_recall_restored_verified" if changed["changed_writable_parameters"] else "no_state_change_not_verified"
                    result = {"index": entry["index"], "status": status, **changed}
                results.append(result)
                self.counts[result["status"]] += 1
                self.journal.append("preset_result", transaction=transaction, **result)
            self.phase(transaction, "StateReleaseInFlight")
            self.release(baseline, capsule)
            self.release_catalog(baseline, catalog, transaction)
            self.verify_released_catalog(baseline, catalog, selected[0]["index"], transaction)
        except (UnknownMutation, ContextChanged):
            self.phase(transaction, "UnknownAfterDispatch")
            raise
        verified = all(item["status"] == "preset_recall_restored_verified" for item in results)
        status = "preset_case_verified" if verified else "preset_case_not_verified"
        row = {"plugin": target["plugin"], "status": status, "presets": results,
               "catalog_count": len(catalog.entries), "catalog_complete": True,
               "factory_origin": "not_inferred", "source": baseline.source,
               "provider_indices_compared": len(baseline.records), "baseline": reference,
               "released_catalog_refused": True, "state_token_released": True,
               "wrong_target_catalog_refused": wrong_target is not None,
               "read_only_values": self.summary["read_only_values"],
               "opaque_contents_verified": False, "preset_display_verified": False}
        self.phase(transaction, "StateConfirmed", status=status, released_catalog_refused=True)
        self.summary["classes"].append(row)
        self.counts[status] += 1
        self.progress()
        return row


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default="target/release/fender-studio-pro-mcp")
    parser.add_argument("--fixtures", required=True)
    parser.add_argument("--scratch-title", required=True)
    parser.add_argument("--output-dir", default=".local/live/plugin-presets-" + datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ"))
    parser.add_argument("--plugin", action="append", required=True, help="Exact fixture plugin; repeat to select more")
    parser.add_argument("--catalog-only", action="store_true", help="Inventory and release each current issued catalog without recalling presets")
    parser.add_argument("--index", type=int, action="append", help="Issued index0..4095; repeat for finite selections; default only index1")
    parser.add_argument("--wrong-target-plugin", help="Also prove catalog refusal on this different exact fixture")
    parser.add_argument("--pause", action="store_true", help="Start the owner before launching the DAW, then press Enter")
    parser.add_argument("--prepare-midi-fixtures", action="store_true")
    parser.add_argument("--settle-attempts", type=int, default=8)
    parser.add_argument("--settle-interval", type=float, default=0.1)
    parser.add_argument("--read-only-observations", action="store_true",
                        help="Compare every typed schema; report only authoritative nonwritable read-only value drift separately")
    args = parser.parse_args(argv)
    validate_settlement(args.settle_attempts, args.settle_interval)
    require(not args.catalog_only or (args.index is None and args.wrong_target_plugin is None
                                     and not args.prepare_midi_fixtures),
            "Catalog-only mode cannot select indices, wrong targets or MIDI fixture preparation")
    if args.index is not None:
        require(0 < len(args.index) <= 64 and all(0 <= index < 4096 for index in args.index),
                "Select 1..64 issued indices from0..4095")
    fixtures = json.loads(Path(args.fixtures).read_text())
    require(isinstance(fixtures, list) and 0 < len(fixtures) <= 128
            and all(isinstance(item, dict) and isinstance(item.get("plugin"), str) for item in fixtures),
            "Expected 1..128 fixture targets")
    plugins = list(dict.fromkeys(args.plugin))
    require(0 < len(plugins) <= 128, "Select 1..128 exact plugin fixtures")
    require(all(sum(item["plugin"] == plugin for item in fixtures) == 1 for plugin in plugins),
            "Every selected plugin must match one exact fixture")
    targets = [{key: value for key, value in next(item for item in fixtures if item["plugin"] == plugin).items()
                if not key.startswith("test_")} for plugin in plugins]
    other = None
    if args.wrong_target_plugin:
        require(args.wrong_target_plugin not in plugins, "Wrong-target fixture must be outside the selected write scope")
        matches = [item for item in fixtures if item["plugin"] == args.wrong_target_plugin]
        require(len(matches) == 1, "Wrong-target plugin must match one exact fixture")
        other = {key: value for key, value in matches[0].items() if not key.startswith("test_")}
    journal = Journal(args.output_dir)
    client = runner = None
    try:
        binary = Path(args.binary).resolve()
        journal.append("run_started", schema=1, mode="preset_catalogs" if args.catalog_only else "preset_recalls",
                       binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                       scope={"plugins": plugins, "indices": args.index},
                       guarantee=("two fresh complete typed baseline projections after discovery, release and released-token list refusal; no preset recall"
                                  if args.catalog_only else "two independent changed provider reads and two complete restored provider reads; opaque state is not decoded"))
        client = McpClient([str(binary), "serve"], timeout=45)
        print("MCP owner ready", flush=True)
        if args.pause:
            input("Open exactly the named scratch song, then press Enter: ")
        runner = PresetRunner(client, journal, args.scratch_title, args.prepare_midi_fixtures,
                              args.settle_attempts, args.settle_interval, args.read_only_observations)
        runner.initialize()
        for index, target in enumerate(targets, 1):
            print(f"[{index}/{len(targets)}] {target['plugin']}", flush=True)
            if args.catalog_only:
                runner.run_catalog_case(target)
            else:
                runner.run_preset_case(target, args.index, wrong_target=other)
        expected = "catalog_case_verified" if args.catalog_only else "preset_case_verified"
        verified = len(runner.summary["classes"]) == len(targets) and all(row["status"] == expected for row in runner.summary["classes"])
        status = ("catalog_cases_verified" if verified else "catalog_cases_not_verified") if args.catalog_only else (
            "preset_cases_verified" if verified else "preset_cases_not_verified")
        runner.summary.update(status=status, completed_at=utc_now())
        runner.progress()
        journal.append("run_completed", summary=runner.summary)
        print(json.dumps({"status": runner.summary["status"], "counts": dict(runner.counts),
                          "opaque_contents_verified": False, "all_writable_parameters_verified": False}), flush=True)
        return 0 if verified else 2
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
