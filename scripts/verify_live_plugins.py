#!/usr/bin/env python3

import argparse
from datetime import datetime, timezone
import json
import math
from pathlib import Path
import re
import subprocess
import time
import xml.etree.ElementTree as ET

from mcp_client import McpClient


def normalize(value):
    return "".join(c.lower() for c in value if c.isalnum())


def structured(result):
    if "structuredContent" in result:
        return result["structuredContent"]
    for item in result.get("content", []):
        if item.get("type") == "text":
            return json.loads(item["text"])
    raise RuntimeError("MCP tool returned no structured receipt")


def equal_number(a, b):
    return type(a) in (int, float) and type(b) in (int, float) and math.isfinite(a) and math.isfinite(b) and abs(a - b) <= 1e-6 * (1 + abs(a))


def parameter_readback(receipt, plugin, param):
    result = receipt.get("result", {})
    after = result.get("after", {})
    if receipt.get("outcome") not in ("applied_verified", "applied_unverified") or receipt.get("error"):
        raise RuntimeError("Parameter write did not return an applied receipt")
    if result.get("plugin") != plugin or result.get("param") != param:
        raise RuntimeError("Parameter readback identity did not match the request")
    value, normalized = after.get("value"), after.get("normalized")
    if type(value) not in (int, float) or not math.isfinite(value) or type(normalized) not in (int, float) or not math.isfinite(normalized) or not 0 <= normalized <= 1:
        raise RuntimeError("Parameter change has no finite numeric readback")
    return after


def error_message(error):
    return error.get("message", str(error)) if isinstance(error, dict) else str(error)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", default="target/debug/fender-studio-pro-mcp")
    parser.add_argument("--scratch-title", required=True)
    parser.add_argument("--output", default=".local/live/plugin-roundtrips.json")
    parser.add_argument("--filter", default="")
    parser.add_argument("--limit", type=int, default=0)
    parser.add_argument("--kinds", default="effect,instrument")
    args = parser.parse_args()
    categories = {"effect": "AudioEffect", "instrument": "AudioSynth", "note_effect": "MusicEffect"}
    kinds = list(dict.fromkeys(kind.strip() for kind in args.kinds.split(",")))
    if not kinds or any(kind not in categories for kind in kinds):
        parser.error("--kinds must contain effect, instrument, or note_effect")
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    catalog = json.loads(subprocess.check_output([args.binary, "local-catalog"], text=True))
    metadata_names = {normalize(name) for p in catalog["plugins"] for name in [p["name"], *p["aliases"]]}
    metadata_ids = {cid.lower() for p in catalog["plugins"] for cid in p["class_ids"]}
    cache_path = Path.home() / "Library/Application Support/Fender/Studio Pro 8/Arm64"
    native_ids = set()
    for cache in sorted(cache_path.glob("Plugins-*.settings")):
        tree = ET.fromstring(cache.read_text(encoding="utf-8-sig"))
        native_ids.update(c.get("classID", "").lower() for c in tree.iter("ClassDescription") if "(Native)" in c.get("subCategory", ""))
    utility_names = {"Splitter", "External Effect Mono", "External Effect Stereo"}
    report = {
        "started_at": datetime.now(timezone.utc).isoformat(),
        "scratch_title": args.scratch_title,
        "parameter_completeness": "partial_metadata_candidates",
        "acceptance": "one changed parameter read back and restored per reachable plugin, not every plugin action",
        "plugins": [], "receipts": [],
    }

    def checkpoint():
        temporary = output.with_suffix(".tmp")
        temporary.write_text(json.dumps(report, ensure_ascii=False, indent=2))
        temporary.replace(output)

    client = McpClient([args.binary, "serve"], timeout=30)
    report["mcp_initialization"] = client.initialize
    tools = client.request("tools/list")
    report["mcp_tools"] = [tool["name"] for tool in tools.get("tools", [])]
    document = None

    def call(name, arguments=None):
        result = structured(client.call(name, arguments or {}))
        report["receipts"].append({"tool": name, "arguments": arguments or {}, "receipt": result})
        checkpoint()
        return result

    def operation(name, arguments=None, mutation=False, allow_error=False):
        request = {"operation": name, "args": arguments or {}}
        if mutation:
            request["expected_document"] = document
        result = call("operation_run", request)
        if result.get("error") and not allow_error:
            raise RuntimeError(error_message(result["error"]))
        if result.get("document") != document:
            raise RuntimeError("Document changed during plugin verification")
        return result

    def read_tracks():
        return operation("tracks", {"events": False})["result"]

    def unique_audio_track(name):
        existing = [t for t in read_tracks() if t["name"] == name]
        if existing:
            if len(existing) != 1 or existing[0]["mediaType"] != "Audio":
                raise RuntimeError("Ambiguous existing scratch fixture")
            return existing[0]["channel"]
        before = {t["name"] for t in read_tracks()}
        operation("addTrack", {"type": "audioStereo"}, True)
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            added = [t for t in read_tracks() if t["name"] not in before and t["mediaType"] == "Audio"]
            if len(added) == 1:
                channel = added[0]["channel"]
                operation("setChannelLabel", {"channel": channel, "name": name}, True)
                return name
            time.sleep(0.05)
        raise RuntimeError("Added audio track did not become visible")

    def stable_note_effect(identity_target, allow_absent=False):
        deadline = time.monotonic() + 3
        stable_reads = 0
        previous_state = None
        last_error = "Note FX focus did not stabilize"
        while True:
            selected = [t for t in read_tracks() if t.get("selected")]
            state = None
            inspected = None
            if len(selected) != 1 or selected[0]["name"] != identity_target["track"] or selected[0]["mediaType"] != "Music":
                last_error = "Note FX scratch fixture selection was not verified"
            else:
                instrument = operation("pluginInspect", {"kind": "instrument", "track": identity_target["track"], "plugin": "Mojito"}, allow_error=True)
                if instrument.get("error"):
                    last_error = error_message(instrument["error"])
                elif normalize(instrument["result"].get("plugin", "")) != normalize("Mojito"):
                    last_error = "Note FX scratch fixture does not contain Mojito"
                else:
                    inspected = operation("pluginInspect", identity_target, allow_error=True)
                    if not inspected.get("error"):
                        state = "present"
                    else:
                        last_error = error_message(inspected["error"])
                        if allow_absent and last_error == "the Note FX slot is not connected":
                            state = "absent"
            if state is None:
                stable_reads = 0
            elif state == previous_state:
                stable_reads += 1
            else:
                stable_reads = 1
            previous_state = state
            if stable_reads >= 2:
                return inspected["result"] if state == "present" else None
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise RuntimeError("Note FX focus did not stabilize within 3 seconds: " + last_error)
            time.sleep(min(0.1, remaining))

    def note_effect_target(plugin):
        track = "FSP Verify NoteFX " + plugin["name"]
        existing = [t for t in read_tracks() if t["name"] == track]
        if existing and (len(existing) != 1 or existing[0]["mediaType"] != "Music"):
            raise RuntimeError("Ambiguous existing Note FX scratch fixture")
        if not existing:
            added = operation("trackTask", {"ops": [{"op": "addInstrumentTrack", "name": track, "instrument": "Mojito"}]}, True)
            results = added["result"].get("results", [])
            if len(results) != 1 or results[0].get("error") or results[0].get("connected") is not True:
                raise RuntimeError("Mojito scratch track creation was not verified: " + str(added["result"]))
        fixtures = operation("tracks", {"name": track, "events": True, "maxEvents": 1})["result"]
        fixtures = [t for t in fixtures if t["name"] == track]
        if len(fixtures) != 1 or fixtures[0]["mediaType"] != "Music" or fixtures[0].get("eventCount") != 0:
            raise RuntimeError("Note FX scratch fixture must be one unique empty instrument track")
        operation("selectTrack", {"name": track, "mediaType": "Music", "exclusive": True}, True)
        target = {"kind": "note_effect", "track": track, "slot": 0, "plugin": plugin["name"]}
        identity_target = {**target, "plugin": plugin["classId"]}
        if stable_note_effect(identity_target, allow_absent=True) is None:
            inserted = operation("command", {
                "category": "Track", "name": "Add Note FX to Selected Tracks", "args": {"cid": plugin["classId"]},
            }, True)
            if inserted["result"].get("executed") is not True:
                raise RuntimeError("Note FX insertion command was not accepted")
            stable_note_effect(identity_target)
        return target

    try:
        context = call("host_status")
        if context.get("error") or context["result"].get("title") != args.scratch_title:
            raise RuntimeError("Open exactly the named disposable scratch session before running")
        document = context["document"]
        session = operation("song")["result"]
        if session["transport"]["playing"] or session["transport"]["recording"]:
            raise RuntimeError("Stop playback and recording in the named scratch session before verification")
        inventory = []
        note_effect_names = {"arpeggiator", "chorder", "inputfilter", "repeater"}
        for kind in kinds:
            category = categories[kind]
            live = operation("pluginCatalog", {"category": category})["result"]
            for plugin in live["plugins"]:
                native = plugin["classId"].lower() in native_ids or plugin["classId"].lower() in metadata_ids or normalize(plugin["name"]) in metadata_names or plugin["name"] in utility_names
                if kind == "note_effect":
                    native = normalize(plugin["name"]) in note_effect_names
                if native and (not args.filter or args.filter.lower() in plugin["name"].lower()):
                    inventory.append({**plugin, "kind": kind})
        if args.limit:
            inventory = inventory[:args.limit]
        report["inventory"] = inventory
        checkpoint()
        for index, plugin in enumerate(inventory, 1):
            row = {**plugin, "status": "running", "parameters": 0, "missing_candidates": 0}
            report["plugins"].append(row)
            checkpoint()
            print(f"[{index}/{len(inventory)}] {plugin['kind']} {plugin['name']}", flush=True)
            try:
                if plugin["kind"] == "effect" and plugin["name"] == "Splitter":
                    row["status"] = "unsupported_graph_control"
                    row["error"] = "Splitter is a graph node; its native edit methods have not been verified. No additional graph node was inserted."
                    continue
                if plugin["kind"] == "effect":
                    channel = unique_audio_track("FSP Verify FX " + re.sub(r"[^A-Za-z0-9]+", " ", plugin["name"]).strip())
                    operation("setChannel", {"channel": channel, "field": "mute", "value": True}, True)
                    inserts = operation("inserts", {"channel": channel})["result"]
                    slots = inserts[0]["inserts"] if inserts else []
                    if not slots:
                        added = operation("trackTask", {"ops": [{"op": "addPlugin", "channel": channel, "plugin": plugin["name"]}]}, True)
                        if any(r.get("error") for r in added["result"].get("results", [])):
                            raise RuntimeError(added["result"])
                    target = {"channel": channel, "slot": 0, "plugin": plugin["name"]}
                elif plugin["kind"] == "instrument":
                    track = "FSP Verify Inst " + plugin["name"]
                    existing = [t for t in read_tracks() if t["name"] == track]
                    if not existing:
                        added = operation("trackTask", {"ops": [{"op": "addInstrumentTrack", "name": track, "instrument": plugin["name"]}]}, True)
                        if any(r.get("error") for r in added["result"].get("results", [])):
                            raise RuntimeError(added["result"])
                    operation("selectTrack", {"name": track, "mediaType": "Music"}, True)
                    # An instrument track's MIDI channel is not its audio output.
                    target = {"kind": "instrument", "track": track, "plugin": plugin["name"]}
                else:
                    target = note_effect_target(plugin)
                row["target"] = target
                deadline = time.monotonic() + 6
                while True:
                    params = operation("pluginParams", target)["result"]
                    if not params.get("error") or time.monotonic() >= deadline:
                        break
                    time.sleep(0.1)
                if params.get("error"):
                    raise RuntimeError(params["error"])
                row["parameters"] = len(params.get("params", []))
                row["missing_candidates"] = len(params.get("missing", []))
                row["readable_parameter_names"] = [p["name"] for p in params.get("params", [])]
                candidates = []
                for parameter in params.get("params", []):
                    name = parameter.get("name", "").lower()
                    value = parameter.get("value")
                    normalized = parameter.get("normalized")
                    if isinstance(value, bool) or not isinstance(value, (int, float)) or not isinstance(normalized, (int, float)):
                        continue
                    if not math.isfinite(value) or not 0 <= normalized <= 1 or parameter.get("max", 0) <= parameter.get("min", 0):
                        continue
                    if re.search(r"meter|peak|rms|reset|load|save|trigger|sample|record|bypass|mute|solo|focus|selection", name):
                        continue
                    priority = 0 if re.search(r"gain|level|mix|amount|frequency|freq|time|threshold|cutoff|drive|rate|width|spread|decay|attack|release", name) else 1
                    candidates.append((priority, name, parameter))
                if not candidates:
                    row["status"] = "readable_no_safe_write_candidate" if row["parameters"] else "no_readable_candidates"
                    if plugin["kind"] == "effect":
                        rack = operation("inserts", {"channel": target["channel"]})["result"]
                        slots = rack[0]["inserts"] if len(rack) == 1 else []
                        matching = [s for s in slots if s["slot"] == target["slot"]]
                        if len(matching) != 1 or type(matching[0].get("bypassed")) is not bool:
                            raise RuntimeError("The original bypass state is unknown")
                        original = matching[0]["bypassed"]
                        bypass_args = {"channel": target["channel"], "slot": target["slot"], "plugin": plugin["name"]}
                        try:
                            changed = operation("setInsertBypass", {**bypass_args, "bypassed": not original}, True)
                            if changed["result"].get("before") is not original or changed["result"].get("after") is not (not original):
                                raise RuntimeError("Bypass change was not observed")
                        finally:
                            restored = operation("setInsertBypass", {**bypass_args, "bypassed": original}, True)
                            if restored["result"].get("after") is not original:
                                raise RuntimeError("Bypass restoration did not match the original state")
                        row.update({"status": "bypass_changed_readback_restored", "before_bypassed": original, "bypass_change": changed["result"], "bypass_restore": restored["result"]})
                    continue
                for _, _, parameter in sorted(candidates, key=lambda item: (item[0], item[1]))[:6]:
                    changed = None
                    restored = None
                    try:
                        if parameter.get("min") == 0 and parameter.get("max") == 1 and parameter["value"] in (0, 1):
                            desired = 1 if parameter["value"] == 0 else 0
                        else:
                            desired = 0.37 if abs(parameter["normalized"] - 0.37) > 0.08 else 0.63
                        changed = operation("setPluginParam", {**target, "param": parameter["name"], "normalized": desired}, True)
                        after = parameter_readback(changed, plugin["name"], parameter["name"])
                        if equal_number(parameter["value"], after["value"]):
                            continue
                    finally:
                        restored = operation("setPluginParam", {**target, "param": parameter["name"], "value": parameter["value"]}, True)
                        restored_after = parameter_readback(restored, plugin["name"], parameter["name"])
                        if not equal_number(parameter["value"], restored_after["value"]):
                            raise RuntimeError("Parameter restoration did not match the original value")
                    row.update({"status": "changed_readback_restored", "tested_parameter": parameter["name"], "before": parameter, "change": changed["result"], "restore": restored["result"]})
                    break
                if row["status"] == "running":
                    row["status"] = "no_parameter_changed"
            except Exception as error:
                row["status"] = "failed"
                row["error"] = str(error)
            finally:
                checkpoint()
                print("  " + row["status"] + f"; readable={row['parameters']}; missing={row['missing_candidates']}", flush=True)
        report["completed_at"] = datetime.now(timezone.utc).isoformat()
        report["summary"] = {status: sum(p["status"] == status for p in report["plugins"]) for status in sorted({p["status"] for p in report["plugins"]})}
        checkpoint()
        print(json.dumps(report["summary"], ensure_ascii=False), flush=True)
    finally:
        client.close()


if __name__ == "__main__":
    main()
