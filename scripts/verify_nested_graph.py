#!/usr/bin/env python3
"""Finite two-Splitter control acceptance in an existing disposable scratch song.

This API observes nodes and parameters, not routing edges. Nested topology must
be established separately in the DAW. Raw evidence stays in repository .local.
"""
import argparse
import copy
from datetime import datetime, timezone
import hashlib
import json
import math
from pathlib import Path
import sys

from mcp_client import McpClient
from verify_all_parameters import (AcceptanceFailure, ContextChanged, Journal,
                                   RestorationFailure, Runner, UnknownMutation,
                                   require, utc_now)


ALL_NAMES = ("numOutputs", "splitMode") + tuple(
    f"{kind}[{index}]" for kind in ("gain", "mute") for index in range(1, 6)
) + tuple(f"splitFreq[{index}]" for index in range(1, 5))


def raw(name, value):
    # Only native binary switches may interchange bool and numeric 0/1.
    if name.startswith("mute[") and type(value) is bool:
        value = int(value)
    require(type(value) in (int, float) and math.isfinite(value),
            "Missing finite raw Splitter value: " + name)
    if name.startswith("mute["):
        require(value in (0, 1), "Unsupported Splitter switch: " + name)
    return value


def active_names(count):
    require(type(count) in (int, float) and count in (2, 3, 4, 5),
            "Unsupported Splitter branch count")
    return {"numOutputs", "splitMode"} | {
        f"{kind}[{index}]" for kind in ("gain", "mute") for index in range(1, int(count) + 1)
    } | {f"splitFreq[{index}]" for index in range(1, int(count))}


def comparable(snapshot):
    value = copy.deepcopy(snapshot)
    for item in value.values():
        for name, parameter in item["params"].items():
            parameter["value"] = raw(name, parameter["value"])
    return value


def same(left, right):
    return comparable(left) == comparable(right)


def candidate(parameter, parameters):
    name = parameter["name"]
    present = raw(name, parameter["value"])
    low, high = parameter["min"], parameter["max"]
    if name.startswith("mute["):
        wanted = 1 - present
    elif name.startswith("gain["):
        # Binary fractions avoid the observed native Float32 rounding of 0.7.
        wanted = next((number for number in (0.5, 1, 0, 0.25, 0.75)
                       if low <= number <= high and number != present), None)
        require(wanted is not None, "No exact binary gain candidate is available")
    else:
        index = int(name[len("splitFreq["):-1])
        lower = max(low, raw(name, parameters[f"splitFreq[{index - 1}]"]["value"])) if index > 1 else low
        upper = min(high, raw(name, parameters[f"splitFreq[{index + 1}]"]["value"])) if index < 4 else high
        require(lower <= present <= upper and lower < upper,
                "Crossover neighbors have no safe interval")
        first, last = math.floor(lower) + 1, math.ceil(upper) - 1
        wanted = next((number for number in ((first + last) // 2, first, last)
                       if lower < number < upper and number != present and abs(number) <= 2 ** 24), None)
        require(wanted is not None, "No exactly representable integer-Hz crossover candidate is available")
    require(low <= wanted <= high and wanted != present, "Native test target would be unchanged")
    return wanted


class NestedGraphRunner(Runner):
    def __init__(self, client, journal, scratch_title, channel, splitter_labels):
        require(isinstance(channel, str) and channel.startswith("FSP Verify "),
                "Only a named FSP Verify fixture is allowed")
        require(len(splitter_labels) == 2 and len(set(splitter_labels)) == 2
                and all(isinstance(label, str) and label.strip() for label in splitter_labels),
                "Supply exactly two distinct nonempty Splitter labels")
        super().__init__(client, journal, scratch_title)
        self.channel, self.labels = channel, tuple(splitter_labels)
        self.target = {"plugin": "Splitter", "kind": "effect", "channel": channel}
        self.identities = None
        self.initial = self.baseline = None
        self.rows = []
        self.summary.update(mode="two_splitter_controls", scope={"channel": channel, "splitter_labels": list(self.labels)},
                            topology_verified_separately=False, routing_edges_observed=False,
                            expected_case_count=42, cases=self.rows,
                            hidden_inactive_values="not observed; only original active values are restored",
                            comparison="exact finite raw values and complete active descriptors; bool/0/1 equivalence only for mute switches",
                            original_state_restored=False, passed=False)

    def guard(self):
        self.fixture_check(self.target)

    def capture(self):
        receipt = self.call("operation_run", {"operation": "splitters", "args": {"channel": self.channel}})
        require(receipt.get("outcome") == "observed", "Splitter inventory was not an observed read")
        result = receipt.get("result")
        require(isinstance(result, dict) and result.get("channel") == self.channel,
                "Splitter inventory channel is unavailable")
        # The existing provider calls this scope top_level even when its flat
        # node enumeration contains nested Splitters. No edge is inferred here.
        require(result.get("truncated") is False and result.get("diagnostics") == [],
                "A complete bounded Splitter inventory without diagnostics is required")
        items = result.get("splitters")
        require(isinstance(items, list) and len(items) == 2, "Exactly two Splitters must be observable")
        snapshot, identities, found_labels = {}, {}, set()
        for item in items:
            require(isinstance(item, dict) and not item.get("error") and item.get("missing") == [],
                    "A Splitter has unavailable controls")
            node, label = item.get("node"), item.get("label")
            require(item.get("channel") == self.channel and item.get("type") == "Splitter"
                    and isinstance(node, str) and node and isinstance(label, str) and label,
                    "Native Splitter identity is unavailable")
            require(node not in snapshot and label not in found_labels, "Duplicate Splitter node or label")
            found_labels.add(label)
            params = item.get("params")
            require(isinstance(params, list), "Native Splitter parameter list is unavailable")
            mapped = {}
            for parameter in params:
                require(isinstance(parameter, dict) and isinstance(parameter.get("name"), str),
                        "Native Splitter parameter identity is unavailable")
                name = parameter["name"]
                require(name not in mapped, "Duplicate Splitter parameter: " + name)
                value = raw(name, parameter.get("value"))
                low, high = parameter.get("min"), parameter.get("max")
                require(type(low) in (int, float) and type(high) in (int, float)
                        and math.isfinite(low) and math.isfinite(high) and low < high and low <= value <= high,
                        "Missing finite Splitter bounds: " + name)
                mapped[name] = copy.deepcopy(parameter)
            count = raw("numOutputs", mapped.get("numOutputs", {}).get("value"))
            mode = raw("splitMode", mapped.get("splitMode", {}).get("value"))
            require(mode in (0, 1, 2), "Unsupported Splitter mode")
            require(set(mapped) == active_names(count), "Incomplete or unexpected active Splitter parameters")
            identities[node] = {"channel": self.channel, "node": node, "label": label}
            snapshot[node] = {**identities[node], "type": "Splitter", "params": mapped}
        if self.identities is not None and identities != self.identities:
            raise ContextChanged("Native Splitter node or label changed")
        require(found_labels == set(self.labels), "Observed Splitter labels differ from the explicit fixture labels")
        if self.identities is None:
            self.identities = identities
            self.journal.append("splitter_identities_bound", identities=identities,
                                identity_limit="node and label only; provider exposes no instance UID or routing edges")
        return snapshot

    def phase(self, transaction, phase, **fields):
        self.journal.append("transaction_phase", transaction=transaction, phase=phase, **fields)

    @staticmethod
    def siblings(snapshot, node):
        return {key: value for key, value in snapshot.items() if key != node}

    def confirm_baseline(self, expected, purpose):
        observed = None
        for attempt in (1, 2):
            self.journal.append("splitter_baseline_readback_attempt", purpose=purpose, attempt=attempt)
            self.guard()
            observed = self.capture()
            require(same(observed, expected), "Complete two-Splitter baseline differs on independent read " + str(attempt))
            self.journal.append("splitter_baseline_readback", purpose=purpose, attempt=attempt,
                                complete_exact_baseline=True, required_fresh_reads=2)
        return observed

    def write_checked(self, node, param, wanted, before, purpose, allow_unchanged=False):
        self.guard()
        fresh_before = self.capture()
        if not same(fresh_before, before):
            raise ContextChanged("Full Splitter state drifted before dispatch; no automatic restore is permitted")
        parameters = before[node]["params"]
        require(param in parameters, "Splitter write target is inactive: " + param)
        present = raw(param, parameters[param]["value"])
        wanted = raw(param, wanted)
        require(parameters[param]["min"] <= wanted <= parameters[param]["max"], "Requested Splitter value is out of bounds")
        if wanted == present:
            require(allow_unchanged, "Unchanged writes cannot count as accepted control cases")
            return fresh_before
        identity = self.identities[node]
        self.journal.append("splitter_write_intent", purpose=purpose, identity=identity, parameter=param,
                            before=present, requested=wanted)
        receipt = self.call("operation_run", {"operation": "setSplitter", "args": {
            **identity, "param": param, "value": wanted}}, mutation=True, allow_error=True)
        outcome = receipt.get("outcome")
        if outcome == "not_applied":
            current = self.capture()
            if not same(current, before):
                raise UnknownMutation("Rejected Splitter write changed the observable state")
            raise AcceptanceFailure("Splitter setter rejected the requested change")
        if outcome not in ("applied_verified", "applied_unverified"):
            raise UnknownMutation("Splitter setter did not establish a known applied outcome")
        result = receipt.get("result", {})
        errors = " ".join(str(value) for value in (receipt.get("error"), result.get("error") if isinstance(result, dict) else None)).lower()
        if "outcome unknown" in errors or "outcome_unknown" in errors:
            raise UnknownMutation("Native Splitter setter reported an unknown outcome")
        if not isinstance(result, dict) or any(result.get(key) != value for key, value in identity.items()) or result.get("type") != "Splitter":
            raise ContextChanged("Splitter setter identity changed")
        after = result.get("after", {})
        reported_before = result.get("before", {})
        require(not receipt.get("error") and not result.get("error") and after.get("name") == param,
                "Known-applied Splitter setter returned an error or missing parameter")
        require(reported_before.get("name") == param and raw(param, reported_before.get("value")) == present
                and raw(param, result.get("requested")) == wanted,
                "Splitter setter receipt differs from the bound pre-dispatch value or request")
        setter_value = raw(param, after.get("value"))
        # Candidates are exact integers/binary fractions, so no Float32 or broad
        # numeric tolerance is needed, including for restoration.
        require(setter_value == wanted and setter_value != present,
                "Setter did not report the exact requested changed raw value")
        first = observed = None
        for attempt in (1, 2):
            self.journal.append("splitter_changed_readback_attempt", purpose=purpose, identity=identity,
                                parameter=param, attempt=attempt)
            self.guard()
            observed = self.capture()
            require(param in observed[node]["params"]
                    and raw(param, observed[node]["params"][param]["value"]) == setter_value,
                    "Independent Splitter read differs from the setter's exact raw value")
            require(same(self.siblings(observed, node), self.siblings(before, node)),
                    "A sibling Splitter changed during the setter")
            if first is not None:
                require(same(observed, first), "Two independent complete changed Splitter snapshots are unstable")
            else:
                first = observed
            self.journal.append("splitter_changed_readback", purpose=purpose, identity=identity,
                                parameter=param, requested=wanted, setter=setter_value,
                                independent=raw(param, observed[node]["params"][param]["value"]),
                                attempt=attempt, required_fresh_reads=2, changed=True,
                                siblings_unchanged=True, full_snapshot_stable=attempt == 2)
        return observed

    def restore_node(self, node, desired, expected_siblings, purpose):
        self.guard()
        current = self.capture()

        def restore(name):
            nonlocal current
            if not same(self.siblings(current, node), expected_siblings):
                raise RestorationFailure("Sibling Splitter state drifted; no further restoration writes are permitted")
            wanted = raw(name, desired["params"][name]["value"])
            require(name in current[node]["params"], "Restore target is no longer active: " + name)
            if raw(name, current[node]["params"][name]["value"]) == wanted:
                return
            current = self.write_checked(node, name, wanted, current, purpose, allow_unchanged=True)

        restore("numOutputs")
        restore("splitMode")
        ordinary = [name for name in desired["params"] if name.startswith(("gain[", "mute["))]
        for name in ordinary:
            restore(name)
        frequencies = [name for name in desired["params"] if name.startswith("splitFreq[")]
        decreasing = [name for name in frequencies if raw(name, desired["params"][name]["value"])
                      <= raw(name, current[node]["params"][name]["value"])]
        increasing = [name for name in frequencies if name not in decreasing]
        for name in sorted(decreasing) + sorted(increasing, reverse=True):
            restore(name)
        return self.confirm_baseline({**expected_siblings, node: desired}, purpose)

    def restore_initial(self):
        self.phase("initial-state", "RestoreInFlight")
        try:
            current = self.capture()
            for label in self.labels:
                node = next(key for key, item in self.initial.items() if item["label"] == label)
                current = self.restore_node(node, self.initial[node], self.siblings(current, node), "initial_restore")
            self.confirm_baseline(self.initial, "final_initial_restore")
        except (UnknownMutation, ContextChanged):
            self.phase("initial-state", "UnknownAfterDispatch")
            raise
        except Exception as error:
            self.phase("initial-state", "RestorationFailed", message=str(error))
            raise RestorationFailure("Original active Splitter states were not independently restored") from error
        self.summary["original_state_restored"] = True
        self.summary["initial_restore_fresh_reads"] = 2
        self.phase("initial-state", "StateConfirmed", original_active_values_restored=True, complete_exact_fresh_reads=2)

    def run_case(self, node, param, wanted, alternate=None):
        transaction = f"case-{len(self.rows) + 1:02d}"
        row = {"transaction": transaction, **self.identities[node], "parameter": param,
               "requested": wanted, "status": "running"}
        self.rows.append(row)
        self.progress()
        self.guard()
        current = self.capture()
        if not same(current, self.baseline):
            raise ContextChanged("Complete two-Splitter baseline drifted before a case")
        self.phase(transaction, "Prepared", node=node, parameter=param, requested=wanted)
        failure = None
        try:
            self.phase(transaction, "ChangeInFlight")
            if alternate is not None:
                current = self.write_checked(node, param, alternate, current, "case_prepare")
                row["preparation"] = alternate
            current = self.write_checked(node, param, wanted, current, "case_change")
            row["changed_readback_confirmed"] = True
            row["changed_readback_fresh_reads"] = 2
            self.phase(transaction, "ChangedReadbackConfirmed", complete_stable_fresh_reads=2)
        except (UnknownMutation, ContextChanged) as error:
            row.update(status="stopped_unknown_or_context", error=str(error))
            self.phase(transaction, "UnknownAfterDispatch", message=str(error))
            self.progress()
            raise
        except Exception as error:
            failure = error
            row["change_error"] = str(error)
        try:
            self.phase(transaction, "RestoreInFlight")
            final = self.restore_node(node, self.baseline[node], self.siblings(self.baseline, node), "case_restore")
            require(same(final, self.baseline), "Complete two-Splitter baseline did not restore")
        except (UnknownMutation, ContextChanged) as error:
            row.update(status="stopped_unknown_or_context", restore_error=str(error))
            self.phase(transaction, "UnknownAfterDispatch", message=str(error))
            self.progress()
            raise
        except Exception as error:
            row.update(status="restoration_failed", restore_error=str(error))
            self.phase(transaction, "RestorationFailed", message=str(error))
            self.progress()
            raise RestorationFailure("Full Splitter baseline restore failed; no further writes are permitted") from error
        row["all_sibling_descriptors_unchanged"] = True
        row["full_baseline_restored"] = True
        row["restore_baseline_fresh_reads"] = 2
        row["status"] = "change_failed_restored" if failure else "changed_readback_restored"
        self.phase(transaction, "StateConfirmed", case_status=row["status"])
        if failure is None:
            self.counts["changed_readback_restored"] += 1
        self.journal.append("parameter_acceptance", **row)
        self.progress()
        print(f"{transaction} {self.identities[node]['label']} {param}: {row['status']}", flush=True)
        if failure is not None:
            raise AcceptanceFailure("A Splitter control case failed despite confirmed baseline restoration") from failure

    def run(self):
        try:
            self.initialize()
            self.guard()
            self.initial = self.capture()
            self.summary["initial_baseline"] = self.journal.baseline("initial-two-splitters", self.initial)
            self.phase("initial-state", "Prepared")
            current = self.initial
            for label in self.labels:
                node = next(key for key, item in current.items() if item["label"] == label)
                current = self.write_checked(node, "numOutputs", 5, current, "five_branch_setup", allow_unchanged=True)
            self.guard()
            self.baseline = self.capture()
            require(all(set(item["params"]) == set(ALL_NAMES) for item in self.baseline.values()),
                    "Both Splitters must expose all 16 five-branch controls")
            self.summary["expanded_baseline"] = self.journal.baseline("five-branch-two-splitters", self.baseline)
            for label in self.labels:
                node = next(key for key, item in self.baseline.items() if item["label"] == label)
                parameters = self.baseline[node]["params"]
                for count in range(2, 6):
                    self.run_case(node, "numOutputs", count, alternate=4 if count == 5 else None)
                original_mode = raw("splitMode", parameters["splitMode"]["value"])
                for mode in range(3):
                    self.run_case(node, "splitMode", mode, alternate=(mode + 1) % 3 if mode == original_mode else None)
                for name in ALL_NAMES[2:]:
                    self.run_case(node, name, candidate(parameters[name], parameters))
            require(len(self.rows) == 42 and self.counts["changed_readback_restored"] == 42,
                    "The finite 42-case acceptance matrix is incomplete")
            self.restore_initial()
        except (UnknownMutation, ContextChanged, RestorationFailure) as error:
            self.summary.update(status="stopped", error_type=type(error).__name__, error=str(error), completed_at=utc_now())
            self.journal.append("run_stopped", no_automatic_restore=True, error_type=type(error).__name__, message=str(error))
            self.progress()
            raise
        except Exception as error:
            self.summary.update(status="failed", error_type=type(error).__name__, error=str(error))
            if self.initial is not None:
                self.restore_initial()
            self.summary["completed_at"] = utc_now()
            self.progress()
            raise
        self.summary.update(status="two_splitter_controls_verified", passed=True, completed_at=utc_now(),
                            verified_case_count=len(self.rows))
        self.progress()
        self.journal.append("run_completed", summary=self.summary)
        return self.summary


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default="target/release/fender-studio-pro-mcp")
    parser.add_argument("--scratch-title", required=True)
    parser.add_argument("--channel", required=True)
    parser.add_argument("--splitter-label", action="append", required=True,
                        help="Exact native label; repeat exactly twice")
    parser.add_argument("--output-dir", default=".local/live/nested-graph-" + datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ"))
    parser.add_argument("--pause", action="store_true", help="Wait after starting the sole MCP owner, before binding the scratch song")
    args = parser.parse_args(argv)
    journal = Journal(args.output_dir)
    client, runner = None, None
    try:
        binary = Path(args.binary).resolve()
        journal.append("run_started", schema=1, binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                       scratch_title=args.scratch_title, channel=args.channel, splitter_labels=args.splitter_label,
                       topology_verified_separately=False, expected_case_count=42)
        client = McpClient([str(binary), "serve"], timeout=45)
        if args.pause:
            input("Open exactly the named disposable scratch song, then press Enter: ")
        runner = NestedGraphRunner(client, journal, args.scratch_title, args.channel, args.splitter_label)
        summary = runner.run()
        print(json.dumps({key: summary[key] for key in ("status", "passed", "verified_case_count", "original_state_restored",
                                                       "topology_verified_separately")}), flush=True)
        return 0
    except Exception as error:
        journal.append("run_failed", error_type=type(error).__name__, message=str(error))
        if runner is not None:
            runner.summary.update(passed=False, error_type=type(error).__name__, error=str(error), completed_at=utc_now())
            runner.progress()
        print(f"Acceptance stopped: {type(error).__name__}: {error}", file=sys.stderr, flush=True)
        return 1
    finally:
        if client is not None:
            client.close()
        journal.close()


if __name__ == "__main__":
    sys.exit(main())
