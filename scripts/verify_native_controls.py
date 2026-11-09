#!/usr/bin/env python3
"""Live acceptance for existing, empty, muted scratch fixtures; never creates them."""

import argparse
from datetime import datetime, timezone
import hashlib
import json
import math
from pathlib import Path

from mcp_client import McpClient
from verify_live_plugins import equal_number, structured


PANNER_NAMES = (
    "editMode", "azimuth", "altitude", "spread", "size", "balanceAzimuth",
    "balanceAmount", "lfeLevel", "centerLevel", "disableCenter", "muteLfe",
    "sendLfe", "snap", "sizeLock", "soloLfe",
)
PANNER_SWITCHES = {"editMode", "disableCenter", "muteLfe", "sendLfe", "snap", "sizeLock", "soloLfe"}
SPLITTER_NAMES = ("numOutputs", "splitMode") + tuple(
    f"{kind}[{i}]" for kind in ("gain", "mute") for i in range(1, 6)
) + tuple(f"splitFreq[{i}]" for i in range(1, 5))


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def number(value):
    # Native switches may read back as booleans; all writes use numeric raw values.
    if type(value) is bool:
        return int(value)
    require(type(value) in (int, float) and math.isfinite(value), "Missing finite native numeric value")
    return value


def same(left, right):
    return equal_number(number(left), number(right))


def parameter_map(result):
    params = result.get("params")
    require(isinstance(params, list), "Native parameter list is missing")
    mapped = {}
    for parameter in params:
        require(isinstance(parameter, dict) and isinstance(parameter.get("name"), str), "Parameter identity is missing")
        name = parameter["name"]
        require(name not in mapped, "Ambiguous native parameter: " + name)
        mapped[name] = parameter
    return mapped


def bounds(parameter):
    low, high = number(parameter.get("min")), number(parameter.get("max"))
    require(low < high, "Native parameter has no nontrivial writable range: " + parameter["name"])
    value = number(parameter.get("value"))
    require(low <= value <= high, "Native value is outside its reported range: " + parameter["name"])
    return low, high, value


class Acceptance:
    def __init__(self, client, output, args, report):
        self.client, self.output, self.args, self.report = client, output, args, report
        self.document = self.session = None
        self.identity = None
        self.format_confirmed = False
        self.current_case = None
        self.blocked_restores = set()

    def checkpoint(self):
        temporary = self.output.with_suffix(".tmp")
        temporary.write_text(json.dumps(self.report, ensure_ascii=False, indent=2, allow_nan=False), encoding="utf-8")
        temporary.chmod(0o600)
        temporary.replace(self.output)

    def call(self, operation, arguments=None, mutation=False):
        if mutation:
            self.guard()
        request = {"operation": operation, "args": arguments or {}}
        if self.document is not None:
            request["expected_document"] = self.document
        entry = {"id": len(self.report["receipts"]), "tool": "operation_run", "arguments": request,
                 "mutation": mutation, "status": "in_flight"}
        self.report["receipts"].append(entry)
        if self.current_case is not None:
            self.current_case["receipt_ids"].append(entry["id"])
        self.checkpoint()  # A killed process still leaves the write intent recorded.
        try:
            receipt = structured(self.client.call("operation_run", request))
            entry["receipt"] = receipt
            if self.document is not None:
                require(receipt.get("document") == self.document and receipt.get("session") == self.session,
                        "Session or document changed; no further writes are permitted")
            require(not receipt.get("error"), "Host error: " + str(receipt.get("error")))
            require(receipt.get("outcome") in (("applied_verified", "applied_unverified") if mutation else ("observed",)),
                    "Unexpected host outcome: " + str(receipt.get("outcome")))
            require(not isinstance(receipt.get("result"), dict) or not receipt["result"].get("error"),
                    "Native control returned an error")
            entry["status"] = "returned"
            return receipt
        except Exception as error:
            entry["status"] = "write_outcome_unknown" if mutation else "read_failed"
            entry["exception"] = str(error)
            raise
        finally:
            self.checkpoint()

    def read(self, operation, arguments=None):
        return self.call(operation, arguments)["result"]

    def guard(self):
        receipt = self.call("context")
        context = receipt["result"]
        require(context.get("open") is True and context.get("title") == self.args.scratch_title,
                "Open exactly the named disposable scratch session")
        if self.document is None:
            require(receipt.get("document") and receipt.get("session"), "Missing document/session identity")
            self.document, self.session = receipt["document"], receipt["session"]
            self.report["document"], self.report["session"] = self.document, self.session
        require(context.get("document") == self.document and context.get("session") == self.session,
                "Scratch document or bridge session changed")
        song = self.read("song")
        require(song.get("title") == self.args.scratch_title, "Scratch song title changed")
        transport = song.get("transport", {})
        require(transport.get("playing") is False and transport.get("recording") is False,
                "Scratch transport must be stopped and not recording")
        tracks = self.read("tracks", {"events": True, "maxEvents": 1})
        channels = self.read("channels")
        require(isinstance(tracks, list) and isinstance(channels, list), "Fixture inventory is missing")
        for name in (self.args.splitter_channel, self.args.panner_channel):
            matches = [t for t in tracks if t.get("name") == name]
            require(len(matches) == 1 and matches[0].get("mediaType") == "Audio"
                    and matches[0].get("eventCount") == 0 and matches[0].get("events") == []
                    and matches[0].get("channel") == name,
                    "Fixture must be one uniquely named empty Audio track/channel: " + name)
            mapped = [c for c in channels if c.get("label") == name]
            require(len(mapped) == 1 and mapped[0].get("mute") in (True, 1)
                    and type(mapped[0].get("mute")) in (bool, int, float),
                    "Fixture channel must already be uniquely named and muted: " + name)
            require(mapped[0].get("recordArmed") in (False, 0) and mapped[0].get("monitor") in (False, 0),
                    "Fixture must not be record-armed or monitored: " + name)
        if not self.format_confirmed:
            probe = self.read("panner", {"channel": self.args.panner_channel})
            speaker = probe.get("speakerFormat")
            require(isinstance(speaker, dict) and speaker.get("text") == "5.1",
                    "Surround fixture must report the native 5.1 channel format")
            self.report["panner"]["format_guard"] = speaker
            self.format_confirmed = True
        self.checkpoint()

    def capture(self, group):
        if group == "splitter":
            result = self.read("splitters", {"channel": self.args.splitter_channel})
            require(result.get("scope") == "top_level" and not result.get("truncated") and not result.get("diagnostics"),
                    "Complete top-level Splitter graph read is required")
            items = result.get("splitters", [])
            require(len(items) == 1 and not items[0].get("error") and not items[0].get("missing"),
                    "Fixture must contain exactly one fully readable top-level Splitter")
            item = items[0]
            require(item.get("channel") == self.args.splitter_channel and item.get("type") == "Splitter"
                    and isinstance(item.get("node"), str) and item["node"]
                    and isinstance(item.get("label"), str) and item["label"], "Missing native Splitter identity")
            identity = {key: item[key] for key in ("channel", "node", "label")}
            if self.identity is None:
                self.identity = identity
            require(identity == self.identity, "Native Splitter node or label changed")
            params = parameter_map(item)
            count = number(params.get("numOutputs", {}).get("value"))
            mode = number(params.get("splitMode", {}).get("value"))
            require(count in (2, 3, 4, 5) and mode in (0, 1, 2), "Unsupported native Splitter count/mode")
            expected = {"numOutputs", "splitMode"} | {
                f"{kind}[{i}]" for kind in ("gain", "mute") for i in range(1, int(count) + 1)
            } | {f"splitFreq[{i}]" for i in range(1, int(count))}
            require(set(params) == expected, "Active Splitter parameter list is incomplete or unexpected")
        else:
            result = self.read("pluginParams", {**self.panner_target(), "names": list(PANNER_NAMES)})
            require(result.get("plugin") == "Surround Panner" and result.get("channel") == self.args.panner_channel,
                    "Expected native Surround Panner identity was not returned")
            params = parameter_map(result)
            require(set(params).issubset(PANNER_NAMES), "Unexpected named panner parameter")
        snapshot = {"params": params, "missing": list(result.get("missing", []))}
        if group == "splitter":
            snapshot.update(self.identity)
        return snapshot

    def panner_target(self):
        return {"channel": self.args.panner_channel, "kind": "panner", "plugin": "Surround Panner"}

    def write_capture(self, group, param, value, normalized=False):
        if group == "splitter":
            receipt = self.call("setSplitter", {**self.identity, "param": param, "value": value}, True)
            result = receipt["result"]
            require(all(result.get(key) == expected for key, expected in self.identity.items()), "Splitter write identity mismatch")
            require(result.get("type") == "Splitter", "Splitter type readback mismatch")
        else:
            field = "normalized" if normalized else "value"
            receipt = self.call("setPluginParam", {**self.panner_target(), "param": param, field: value}, True)
            result = receipt["result"]
            require(result.get("plugin") == "Surround Panner" and result.get("param") == param
                    and result.get("channel") == self.args.panner_channel, "Panner write identity mismatch")
        receipt_id = self.report["receipts"][-1]["id"]
        after = result.get("after", {})
        require(after.get("name") == param, "Write did not identify the expected native parameter")
        require(same(after.get("normalized" if normalized else "value"), value), "Requested value differs from native write readback")
        snapshot = self.capture(group)
        require(param in snapshot["params"] and same(snapshot["params"][param].get("value"), after.get("value")),
                "Fresh native read differs from the setter readback")
        return {"receipt_id": receipt_id, "after": after, "capture": snapshot}

    def restore_raw(self, group, param, original, restoration=True):
        self.guard()
        current = self.capture(group)
        require(param in current["params"], "Restore target is no longer active: " + param)
        if same(current["params"][param].get("value"), original):
            return {"confirmed": True, "already_original": True, "capture": current}
        key = (group, param, number(original))
        require(key not in self.blocked_restores, "An uncertain restore will not be blindly retried: " + param)
        try:
            restored = self.write_capture(group, param, number(original))
        except Exception:
            if restoration:
                self.blocked_restores.add(key)
            raise
        require(same(restored["after"]["value"], original), "Raw restore differs from the original native value")
        return {"confirmed": True, "observed_before_restore": current, **restored}

    def case(self, group, param, desired=None, normalized=False, alternate=None):
        row = {"group": group, "param": param, "desired": desired, "normalized_write": normalized,
               "status": "running", "receipt_ids": []}
        self.report["cases"].append(row)
        self.current_case = row
        original = None
        try:
            before = self.capture(group)
            require(param in before["params"], "Missing native candidate: " + param)
            parameter = before["params"][param]
            low, high, original = bounds(parameter)
            row["before"] = before
            if normalized:
                start = number(parameter.get("normalized"))
                require(0 <= start <= 1, "Missing native normalized value")
                desired = start + 0.06 if start <= 0.94 else start - 0.06
                row["desired"] = desired
            elif desired is None:
                if param.startswith("mute[") or param in PANNER_SWITCHES:
                    require(original in (0, 1) and low <= 0 and high >= 1, "Switch has no verified 0/1 range")
                    desired = 1 - original
                elif param.startswith("splitFreq["):
                    index = int(param[len("splitFreq["):-1])
                    lower = number(before["params"][f"splitFreq[{index - 1}]"]["value"]) if index > 1 else low
                    upper = number(before["params"][f"splitFreq[{index + 1}]"]["value"]) if index < 4 else high
                    lower, upper = max(low, lower), min(high, upper)
                    require(lower <= original <= upper and lower < upper, "Crossover neighbors do not leave a safe interval")
                    desired = original + (upper - original) * 0.07 if upper - original >= original - lower else original - (original - lower) * 0.07
                    require(lower < desired < upper, "Crossover change would reach or cross its neighbor")
                else:
                    step = (high - low) * 0.03
                    desired = original + step if original + step <= high else original - step
                row["desired"] = desired
            if not normalized and same(original, desired):
                require(alternate is not None and low <= alternate <= high and not same(alternate, original), "Test target would be unchanged")
                row["prepare"] = self.write_capture(group, param, alternate)
                require(not same(row["prepare"]["after"]["value"], original), "Preparation did not change the native value")
            require(normalized or low <= desired <= high, "Test value lies outside native bounds")
            comparison = row.get("prepare", {}).get("after", parameter)["value"]
            row["change"] = self.write_capture(group, param, desired, normalized)
            require(not same(comparison, row["change"]["after"]["value"]), "Native parameter did not change")
            row["native_change_confirmed"] = True
            row["status"] = "changed_readback_restored"
        except Exception as error:
            row["status"], row["error"] = "failed", str(error)
        finally:
            if original is not None:
                try:
                    row["restore"] = self.restore_raw(group, param, original)
                except Exception as error:
                    row["status"], row["restore_error"] = "failed", str(error)
            self.current_case = None
            self.checkpoint()
            print(f"{group} {param}: {row['status']}", flush=True)
        require(original is None or row.get("restore", {}).get("confirmed"), "Case restoration is unconfirmed; stopping this group")
        return row

    def restore_snapshot(self, group, snapshot):
        names = list(snapshot["params"])
        # Restore falling crossover targets first in ascending order; rising ones
        # in descending order, preserving the order of valid crossover values.
        frequencies = [name for name in names if name.startswith("splitFreq[")]
        ordinary = [name for name in names if name not in frequencies and name not in ("numOutputs", "splitMode")]
        current = self.capture(group)
        decreasing, increasing = [], []
        for name in frequencies:
            require(name in current["params"], "Missing active crossover during restoration")
            target = number(snapshot["params"][name]["value"])
            present = number(current["params"][name]["value"])
            (decreasing if target <= present else increasing).append(name)
        ordered = ordinary + sorted(decreasing) + sorted(increasing, reverse=True)
        restores = []
        for name in ordered:
            try:
                restores.append({"param": name, **self.restore_raw(group, name, snapshot["params"][name]["value"])})
            except Exception as error:
                restores.append({"param": name, "confirmed": False, "error": str(error)})
        return restores

    def compare_snapshot(self, group, original):
        current = self.capture(group)
        differences = [name for name, parameter in original["params"].items()
                       if name not in current["params"] or not same(parameter.get("value"), current["params"][name].get("value"))]
        require(not differences, "Final native values differ: " + ", ".join(differences))
        return current

    def splitter(self):
        result = self.report["splitter"]
        initial = expanded = None
        try:
            self.guard()
            initial = self.capture("splitter")
            result["initial"] = initial
            result["hidden_branch_state_before_expansion"] = "not_observed; no claim of preserving pre-expansion hidden values"
            for count in (2, 3, 4, 5):
                self.case("splitter", "numOutputs", count, alternate=3 if count == 2 else 2)
            for mode in (0, 1, 2):
                self.case("splitter", "splitMode", mode, alternate=1 if mode == 0 else 0)
            setup = {"group": "splitter", "purpose": "expand_for_16_control_capture", "receipt_ids": []}
            result["expansion_setup"] = setup
            self.current_case = setup
            self.restore_raw("splitter", "numOutputs", 5, restoration=False)
            self.restore_raw("splitter", "splitMode", 2, restoration=False)
            expanded = self.capture("splitter")
            require(set(expanded["params"]) == set(SPLITTER_NAMES), "Five-branch Splitter did not expose all 16 fixed controls")
            result["expanded_baseline"] = expanded
            self.current_case = None
            for kind in ("gain", "mute"):
                for index in range(1, 6):
                    self.case("splitter", f"{kind}[{index}]")
            for index in range(1, 5):
                self.case("splitter", f"splitFreq[{index}]")
        except Exception as error:
            result["error"] = str(error)
        finally:
            cleanup = {"group": "splitter", "receipt_ids": [], "confirmed": False}
            result["cleanup"] = cleanup
            self.current_case = cleanup
            try:
                self.guard()
                if expanded is not None:
                    try:
                        self.restore_raw("splitter", "numOutputs", 5)
                        self.restore_raw("splitter", "splitMode", 2)
                        cleanup["expanded_restores"] = self.restore_snapshot("splitter", expanded)
                        cleanup["expanded_values_confirmed"] = self.compare_snapshot("splitter", expanded)
                    except Exception as error:
                        cleanup["expanded_error"] = str(error)
                if initial is not None:
                    self.restore_raw("splitter", "numOutputs", initial["params"]["numOutputs"]["value"])
                    self.restore_raw("splitter", "splitMode", initial["params"]["splitMode"]["value"])
                    cleanup["initial_restores"] = self.restore_snapshot("splitter", initial)
                    cleanup["final"] = self.compare_snapshot("splitter", initial)
                    cleanup["confirmed"] = True
            except Exception as error:
                cleanup["error"] = str(error)
            self.current_case = None
            cases = [row for row in self.report["cases"] if row["group"] == "splitter"]
            result["passed"] = not result.get("error") and not cleanup.get("expanded_error") and cleanup["confirmed"] and len(cases) == 21 and all(row["status"] == "changed_readback_restored" for row in cases)
            self.checkpoint()

    def panner(self):
        result = self.report["panner"]
        initial = None
        try:
            self.guard()
            initial = self.capture("panner")
            result["initial"] = initial
            for name in PANNER_NAMES:
                self.case("panner", name, normalized=name not in PANNER_SWITCHES)
        except Exception as error:
            result["error"] = str(error)
        finally:
            cleanup = {"group": "panner", "receipt_ids": [], "confirmed": False}
            result["cleanup"] = cleanup
            self.current_case = cleanup
            try:
                self.guard()
                if initial is not None:
                    cleanup["restores"] = self.restore_snapshot("panner", initial)
                    cleanup["final"] = self.compare_snapshot("panner", initial)
                    cleanup["confirmed"] = True
            except Exception as error:
                cleanup["error"] = str(error)
            self.current_case = None
            cases = [row for row in self.report["cases"] if row["group"] == "panner"]
            result["passed"] = bool(initial is not None and not result.get("error") and not initial.get("missing", [])
                                    and cleanup["confirmed"] and len(cases) == 15
                                    and all(row["status"] == "changed_readback_restored" for row in cases))
            self.checkpoint()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch-title", required=True)
    parser.add_argument("--binary", default="target/release/fender-studio-pro-mcp")
    parser.add_argument("--output", default=".local/live/native-controls.json")
    parser.add_argument("--splitter-channel", default="FSP Verify FX Splitter")
    parser.add_argument("--panner-channel", default="FSP Verify Surround Panner")
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    output = Path(args.output).resolve()
    require(output.is_relative_to(root / ".local") and output.suffix == ".json", "Output must be a private repository .local JSON file")
    require(args.scratch_title.strip() and args.splitter_channel != args.panner_channel, "A scratch title and two distinct fixture names are required")
    output.parent.mkdir(parents=True, exist_ok=True)
    report = {
        "started_at": datetime.now(timezone.utc).isoformat(), "scratch_title": args.scratch_title,
        "binary": str(Path(args.binary).resolve()), "scope": {
            "splitter": "one existing top-level Splitter; all 16 fixed controls at five branches; counts 2..5 and modes 0..2",
            "panner": "15 named native candidates on a representative 5.1 Surround Panner, not complete parameter enumeration or other speaker formats",
            "fixtures": "existing unique empty Audio tracks, already muted, stopped, not armed or monitored; no hardware/global settings changes",
        },
        "receipts": [], "cases": [], "splitter": {}, "panner": {}, "passed": False,
    }
    report["binary_sha256"] = hashlib.sha256(Path(args.binary).read_bytes()).hexdigest()
    client = None
    validation = Acceptance(client, output, args, report)
    validation.checkpoint()
    try:
        client = McpClient([args.binary, "serve"], timeout=30)
        validation.client = client
        report["mcp_initialization"] = client.initialize
        validation.guard()
        validation.splitter()
        validation.panner()
        report["passed"] = (report["splitter"].get("passed") is True and report["panner"].get("passed") is True
                            and all(entry["status"] == "returned" for entry in report["receipts"]))
    except Exception as error:
        report["error"] = str(error)
    finally:
        report["completed_at"] = datetime.now(timezone.utc).isoformat()
        validation.checkpoint()
        if client is not None:
            client.close()
    print(json.dumps({"passed": report["passed"], "output": str(output)}, ensure_ascii=False), flush=True)
    raise SystemExit(0 if report["passed"] else 1)


if __name__ == "__main__":
    main()
