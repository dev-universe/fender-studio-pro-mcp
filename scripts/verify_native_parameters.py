#!/usr/bin/env python3
"""Validate existing named scratch fixtures through the public MCP tools.

The fixture JSON is an array of plugin target objects. Raw receipts stay local.
This never creates tracks, loads samples, plays audio or changes hardware ports.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import math
from pathlib import Path
import struct
import time

from mcp_client import McpClient
from verify_live_plugins import structured, equal_number


class RestorationFailure(RuntimeError):
    pass


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", default="target/release/fender-studio-pro-mcp")
    parser.add_argument("--fixtures", required=True)
    parser.add_argument("--scratch-title", required=True)
    parser.add_argument("--output", default=".local/live/native-parameters.json")
    parser.add_argument("--pause", action="store_true", help="Start the MIDI owner before opening the DAW; press Enter after opening the scratch song")
    parser.add_argument("--roundtrips", action="store_true", help="Also exercise the fixture's explicitly named previously tested scalar or bypass")
    parser.add_argument("--spatial-controls", action="store_true", help="Rerun the existing Splitter and 5.1 Surround Panner acceptance on this same MCP owner")
    parser.add_argument("--prepare-midi-fixtures", action="store_true", help="For unique empty validation MIDI tracks, observe and set mute on/arm off/monitor off after selection; changes remain in the scratch song")
    parser.add_argument("--external-controls", action="store_true", help="Also verify External Effect list reads, manual latency, phase and Unicode notes with restoration")
    parser.add_argument("--session-checks", action="store_true", help="Rerun owner/stale-document, latency and MCP Keys EditTask acceptance on the same MCP owner")
    args = parser.parse_args()
    fixtures = json.loads(Path(args.fixtures).read_text())
    require(isinstance(fixtures, list) and 0 < len(fixtures) <= 128, "Expected 1..128 fixture targets")
    output = Path(args.output).resolve()
    require(output.is_relative_to((Path(__file__).resolve().parent.parent / ".local").resolve()), "Raw acceptance output must stay in the repository .local directory")
    output.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    report = {"schema": 1, "started_at": datetime.now(timezone.utc).isoformat(),
              "binary_sha256": hashlib.sha256(Path(args.binary).read_bytes()).hexdigest(),
              "plugins": [], "receipts": []}
    client = McpClient([args.binary, "serve"], timeout=30)
    document = None

    def checkpoint():
        temporary = output.with_suffix(".tmp")
        temporary.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
        temporary.chmod(0o600)
        temporary.replace(output)

    def call(name, arguments=None, allow_error=False):
        arguments = dict(arguments or {})
        if document is not None and name in ("plugin_parameters", "plugin_inspect", "plugin_parameter_choices"):
            arguments["expected_document"] = document
        entry = {"tool": name, "arguments": arguments or {}, "status": "in_flight"}
        report["receipts"].append(entry)
        checkpoint()
        receipt = structured(client.call(name, arguments or {}))
        entry.update(receipt=receipt, status="returned")
        checkpoint()
        if document is not None:
            require(receipt.get("document") == document, "Scratch document changed")
        if not allow_error:
            require(not receipt.get("error"), str(receipt.get("error")))
        return receipt

    def operation(name, arguments=None, mutation=False):
        return call("operation_run", {"operation": name, "args": arguments or {},
                    "expected_document": document})["result"]

    def fixture_check(target):
        song = operation("song")
        require(song.get("title") == args.scratch_title and song["transport"].get("playing") is False
                and song["transport"].get("recording") is False, "Scratch transport or song changed")
        name = target.get("track") or target.get("channel")
        require(isinstance(name, str) and name.startswith("FSP Verify "), "Only named validation fixtures are allowed")
        tracks = operation("tracks", {"name": name, "events": True, "maxEvents": 1})
        matches = [t for t in tracks if t.get("name") == name]
        kind = target.get("kind", "effect")
        media = "Music" if kind in ("instrument", "note_effect") else "Audio"
        # Mix FX sits on a scratch bus; it has no media track.
        if kind != "mix_fx":
            require(len(matches) == 1 and matches[0].get("mediaType") == media
                    and matches[0].get("eventCount") == 0 and matches[0].get("events") == [], "Fixture must be one unique empty track: " + name)
        if kind in ("instrument", "note_effect"):
            selected = [t for t in operation("tracks", {"events":False}) if t.get("selected")]
            if len(selected) != 1 or selected[0].get("name") != name or selected[0].get("mediaType") != "Music":
                operation("selectTrack", {"name": name, "mediaType": "Music", "exclusive": True}, True)
            selected = [t for t in operation("tracks", {"events":False}) if t.get("selected")]
            require(len(selected) == 1 and selected[0].get("name") == name and selected[0].get("mediaType") == "Music", "MIDI fixture must be exclusively selected")
            inspected = call("plugin_inspect", target)["result"]
            state = inspected.get("focusedTrackState")
            require(isinstance(state, dict) and state.get("label") == name, "Focused MIDI channel identity is unavailable")
            if args.prepare_midi_fixtures:
                for field, wanted, command in (("mute", 1, "Mute"), ("recordArmed", 0, "Arm"), ("monitor", 0, "Monitor")):
                    require(type(state.get(field)) in (bool, int, float) and state[field] in (0, 1), "MIDI fixture state is unsupported")
                    if state[field] != wanted:
                        operation("command", {"category":"Track", "name":command}, True)
                        inspected = call("plugin_inspect", target)["result"]
                        state = inspected.get("focusedTrackState")
                        require(isinstance(state, dict) and state.get("label") == name and state.get(field) == wanted, "MIDI fixture setup did not read back")
            require(state.get("mute") in (True, 1) and state.get("recordArmed") in (False, 0)
                    and state.get("monitor") in (False, 0), "Focused MIDI fixture must be muted, unarmed and unmonitored")
            return
        channel = matches[0].get("channel") if matches else name
        channels = operation("channels")
        mapped = [c for c in channels if c.get("label") == channel]
        require(len(mapped) == 1 and mapped[0].get("mute") in (True, 1), "Fixture must already be muted: " + name)
        if kind != "mix_fx":
            require(mapped[0].get("recordArmed") in (False, 0) and mapped[0].get("monitor") in (False, 0), "Fixture must not be armed or monitored: " + name)

    def native_page(target, offset=0, source="auto", instance=None):
        query = {**target, "offset": offset, "limit": 64, "source": source}
        if instance:
            query["expected_instance"] = instance
        result = call("plugin_parameters", query)["result"]
        require(not result.get("native_error"), str(result.get("native_error")))
        require(result.get("parameter_discovery", {}).get("completeness") == "current_provider", "Native enumeration unavailable; metadata candidates are not acceptance")
        return result

    def external_roundtrips(target, instance):
        cases = []
        for name, desired in (("inputGain", 0.5), ("outputGain", 0.5), ("wetdry", 0.25)):
            fixture_check(target)
            read = call("plugin_parameters", {**target, "source": "controller", "names": [name], "expected_instance": instance})["result"]
            require(not read.get("missing") and len(read.get("params", [])) == 1, "Native scalar is unavailable")
            original = read["params"][0]
            value = original.get("value")
            require(type(value) in (int, float) and math.isfinite(value) and value != 0, "Bit-exact JSON proof requires a finite nonzero original scalar")
            desired = desired if value != desired else 0.75
            require(original.get("min", math.inf) <= desired <= original.get("max", -math.inf), "Desired scalar is outside the live range")
            change = restore = None
            try:
                change = call("plugin_parameter_set", {**target, "param": name, "value": desired, "expected_instance": instance, "expected_document": document})
                require(change.get("outcome") == "applied_verified", "Native MCP scalar write did not verify")
                require(change["result"]["after"]["value"] == desired and desired != value, "Native scalar did not change")
            finally:
                try:
                    fixture_check(target)
                    restore = call("plugin_parameter_set", {**target, "param": name, "value": value, "expected_instance": instance, "expected_document": document})
                    require(restore.get("outcome") == "applied_verified", "Native scalar restoration did not verify")
                    require(struct.pack(">d", float(restore["result"]["after"]["value"])) == struct.pack(">d", float(value)), "Native scalar restoration was not bit-exact")
                except Exception as error:
                    raise RestorationFailure(str(error)) from error
            cases.append({"parameter": name, "changed_verified": True, "restored_raw_f64_exact": True})
        return cases

    def representative_roundtrip(target, instance, name):
        before = call("plugin_parameters", {**target, "names": [name], "expected_instance": instance})["result"]
        require(not before.get("missing") and len(before.get("params", [])) == 1, "Previously tested scalar unavailable")
        original = before["params"][0]
        value, normalized = original.get("value"), original.get("normalized")
        require(type(value) in (int, float) and math.isfinite(value)
                and type(normalized) in (int, float) and 0 <= normalized <= 1, "Original scalar is not finite")
        desired = 0.37 if abs(normalized - 0.37) > 0.08 else 0.63
        change = {"value": 1 - value} if original.get("min") == 0 and original.get("max") == 1 and value in (0, 1) else {"normalized": desired}
        try:
            fixture_check(target)
            changed = call("plugin_parameter_set", {**target, "param": name, **change, "expected_instance": instance, "expected_document": document})
            require(changed.get("outcome") in ("applied_verified", "applied_unverified"), "Scalar change did not return an applied outcome")
            require(not equal_number(value, changed["result"]["after"].get("value")), "Scalar did not change")
            readback = call("plugin_parameters", {**target, "names": [name], "expected_instance": instance})["result"]["params"][0]
            require(equal_number(changed["result"]["after"]["value"], readback["value"]), "Independent scalar readback did not match")
        finally:
            try:
                fixture_check(target)
                restored = call("plugin_parameter_set", {**target, "param": name, "value": value, "expected_instance": instance, "expected_document": document})
                require(restored.get("outcome") == "applied_verified" and equal_number(value, restored["result"]["after"].get("value")), "Scalar restoration did not match")
            except Exception as error:
                raise RestorationFailure(str(error)) from error
        return {"parameter": name, "changed_readback_restored": True}

    def external_additional_controls(target, instance):
        results = {"choices": [], "roundtrips": []}
        for name in ("sendPort", "receivePort"):
            choices = call("plugin_parameter_choices", {**target, "param": name, "expected_instance":instance})["result"]
            require(choices.get("ok") is True and choices.get("instance", {}).get("object_uid") == instance
                    and choices.get("page_complete") is True, "Native choices did not return a complete page")
            results["choices"].append({"parameter":name, "reported_count":choices["reported_count"], "labels_read":len(choices["choices"]), "scope":"first_native_list_page"})
        for name in ("latencyOffset", "phase", "deviceNotes"):
            fixture_check(target)
            read = call("plugin_parameters", {**target,"source":"controller","names":[name],"expected_instance":instance})["result"]
            require(not read.get("missing") and len(read.get("params", [])) == 1, "Additional native controller unavailable")
            original = read["params"][0]["value"]
            if name == "deviceNotes":
                require(isinstance(original, str), "Original notes text unavailable")
                value = original + "MCP validation — 한글"
                change, restore = {"text":value}, {"text":original}
            else:
                require(type(original) in (int,float) and math.isfinite(original), "Original native scalar unavailable")
                value = 1 if original == 0 else 0
                change, restore = {"value":value}, {"value":original}
            try:
                changed = call("plugin_parameter_set", {**target,"param":name,**change,"expected_instance":instance,"expected_document":document})
                require(changed.get("outcome") == "applied_verified" and changed["result"]["after"].get("value") == value, "Additional native control did not verify")
                fresh = call("plugin_parameters", {**target,"source":"controller","names":[name],"expected_instance":instance})["result"]["params"][0]
                require(fresh.get("value") == value, "Additional control independent readback did not match")
            finally:
                try:
                    fixture_check(target)
                    restored = call("plugin_parameter_set", {**target,"param":name,**restore,"expected_instance":instance,"expected_document":document})
                    require(restored.get("outcome") == "applied_verified" and restored["result"]["after"].get("value") == original, "Additional native control did not restore")
                except Exception as error:
                    raise RestorationFailure(str(error)) from error
            results["roundtrips"].append({"parameter":name,"changed_readback_restored":True})
        return results

    def bypass_roundtrip(target, instance):
        racks = operation("inserts", {"channel": target["channel"]})
        slots = [s for rack in racks for s in rack.get("inserts", []) if s.get("slot") == target["slot"]]
        require(len(slots) == 1 and type(slots[0].get("bypassed")) is bool, "Original bypass unavailable")
        original = slots[0]["bypassed"]
        try:
            fixture_check(target)
            changed = operation("setInsertBypass", {**target, "bypassed": not original, "expected_instance":instance}, True)
            require(changed.get("after") is (not original), "Bypass did not change")
        finally:
            try:
                fixture_check(target)
                restored = operation("setInsertBypass", {**target, "bypassed": original, "expected_instance":instance}, True)
                require(restored.get("after") is original, "Bypass did not restore")
            except Exception as error:
                raise RestorationFailure(str(error)) from error
        return {"bypass_changed_readback_restored": True}

    try:
        print("MCP owner ready", flush=True)
        if args.pause:
            while True:
                line = input()
                if not line.strip():
                    break
                request = json.loads(line)
                receipt = call(request["tool"], request.get("arguments", {}), allow_error=True)
                result = receipt.get("result", {})
                print(json.dumps({"preflight":True, "document":receipt.get("document"), "outcome":receipt.get("outcome"), "error":receipt.get("error"),
                    "native":result.get("native") if isinstance(result, dict) else None,
                    "focusedTrackState":result.get("focusedTrackState") if isinstance(result, dict) else None}), flush=True)
        context = call("host_status")
        require(context.get("result", {}).get("title") == args.scratch_title, "Open exactly the named disposable scratch song")
        document = context["document"]
        song = operation("song")
        require(song["transport"].get("playing") is False and song["transport"].get("recording") is False, "Scratch transport must be stopped")
        report["mcp_tools"] = [t["name"] for t in client.request("tools/list").get("tools", [])]
        for index, fixture in enumerate(fixtures, 1):
            target = dict(fixture)
            test_parameter = target.pop("test_parameter", None)
            test_bypass = target.pop("test_bypass", False)
            row = {"plugin": target["plugin"], "target": target, "status": "running"}
            report["plugins"].append(row)
            print(f"[{index}/{len(fixtures)}] {target['plugin']}", flush=True)
            try:
                fixture_check(target)
                if target.get("plugin") == "Splitter":
                    listed = call("splitters_list", {"channel": target["channel"]})["result"]
                    require(listed.get("truncated") is False and not listed.get("diagnostics") and listed.get("splitters"), "Splitter graph controls unavailable")
                    row.update(status="graph_controls_observed", source="fixed_top_level_graph", nodes=len(listed["splitters"]))
                    continue
                if target["plugin"].startswith("External Effect "):
                    call("plugin_editor_open", {**target, "expected_document": document})
                deadline = time.monotonic() + 3
                while True:
                    try:
                        page = native_page(target)
                        if target["plugin"].startswith("External Effect "):
                            require(page.get("parameter_discovery", {}).get("source") == "controller", "Exact External Effect editor controller is not ready")
                        break
                    except RuntimeError:
                        if time.monotonic() >= deadline:
                            raise
                        time.sleep(0.1)
                source = page["parameter_discovery"]["source"]
                instance = page["instance"]["object_uid"]
                class_uid = page["instance"]["class_uid"]
                count = page["reported_count"]
                records, seen, unreadable, pages = [], set(), [], 0
                while True:
                    require(page["instance"]["object_uid"] == instance and page["instance"]["class_uid"] == class_uid
                            and page["reported_count"] == count and page["parameter_discovery"]["source"] == source, "Provider identity/count changed between pages")
                    require(page.get("page_complete") is True and not page.get("failed_indices"), "Native parameter info page is incomplete")
                    for parameter in page.get("params", []):
                        native = parameter.get("native", parameter)
                        position = native["index"]
                        require(type(position) is int and 0 <= position < count and position not in seen, "Missing, duplicate or invalid native index")
                        seen.add(position)
                        records.append(parameter)
                        if parameter.get("readable") is False:
                            unreadable.append(position)
                    pages += 1
                    next_offset = page.get("next_offset")
                    if next_offset is None:
                        break
                    require(type(next_offset) is int and next_offset > page["offset"] and pages <= 16000, "Non-advancing native pagination")
                    page = native_page(target, next_offset, source, instance)
                require(len(seen) == count and seen == set(range(count)), "Native provider indices are not exhaustive")
                row.update(status="provider_enumerated", source=source, reported_count=count, indices_read=len(seen), pages=pages,
                           unreadable_value_indices=unreadable, parameters=records, instance={"object_uid":instance,"class_uid":class_uid})
                if target["plugin"].startswith("External Effect "):
                    row["roundtrips"] = external_roundtrips(target, instance)
                    if args.external_controls:
                        row["additional_controls"] = external_additional_controls(target, instance)
                elif args.roundtrips and test_parameter:
                    row["representative_roundtrip"] = representative_roundtrip(target, instance, test_parameter)
                elif args.roundtrips and test_bypass:
                    row["representative_roundtrip"] = bypass_roundtrip(target, instance)
                print(f"  {source}: {count} indices, {pages} pages, {len(unreadable)} unavailable values", flush=True)
            except Exception as error:
                row.update(status="failed", error=str(error))
                print("  failed: " + str(error), flush=True)
                if isinstance(error, RestorationFailure):
                    raise
            finally:
                checkpoint()
        if args.spatial_controls:
            from types import SimpleNamespace
            from verify_native_controls import Acceptance
            spatial_output = output.with_name(output.stem + "-spatial.json")
            spatial_args = SimpleNamespace(scratch_title=args.scratch_title, splitter_channel="FSP Verify FX Splitter", panner_channel="FSP Verify Surround Panner")
            spatial_report = {"binary_sha256":report["binary_sha256"], "receipts":[], "cases":[], "splitter":{}, "panner":{}}
            spatial = Acceptance(client, spatial_output, spatial_args, spatial_report)
            spatial.guard()
            spatial.splitter()
            spatial.panner()
            report["spatial_controls"] = {"passed":spatial_report["splitter"].get("passed") is True and spatial_report["panner"].get("passed") is True,
                                          "cases":len(spatial_report["cases"]), "receipts":len(spatial_report["receipts"]), "private_output":str(spatial_output)}
            require(report["spatial_controls"]["passed"], "Spatial regression did not pass")
        if args.session_checks:
            from verify_live_session import SessionValidation
            session_output = output.with_name(output.stem + "-session.json")
            session_report = {"binary_sha256":report["binary_sha256"],"receipts":[],"checks":{},"performance":{}}
            session = SessionValidation(client, session_report, session_output, args.scratch_title)
            session.guard()
            session.second_owner(Path(args.binary).resolve())
            session.stale_document("FSP Verify FX Pro EQ")
            session.measure("ping", 100)
            session.measure("tracks", 20, {"events":True,"maxEvents":50})
            session.note_roundtrip("MCP Keys")
            report["session_checks"] = {"checks":session_report["checks"],"performance":session_report["performance"],"private_output":str(session_output)}
            require(all(row.get("status") == "passed" for row in session_report["checks"].values())
                    and all(session_report["performance"][name]["failed_samples"] == 0 for name in ("ping","tracks")), "Session regression did not pass")
        report["completed_at"] = datetime.now(timezone.utc).isoformat()
        report["summary"] = {status: sum(p["status"] == status for p in report["plugins"]) for status in sorted({p["status"] for p in report["plugins"]})}
        checkpoint()
        print(json.dumps(report["summary"]), flush=True)
        require(not report["summary"].get("failed"), "One or more native acceptance cases failed")
    finally:
        client.close()


if __name__ == "__main__":
    main()
