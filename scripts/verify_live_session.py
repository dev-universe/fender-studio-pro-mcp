#!/usr/bin/env python3

import argparse
from copy import deepcopy
from datetime import datetime, timezone
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import queue
import statistics
import subprocess
import time

from mcp_client import McpClient


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def provenance(root, binary):
    def git(*args):
        return subprocess.check_output(["git", "-C", str(root), *args])

    source_paths = ["Cargo.toml", "Cargo.lock", "src", "assets", "scripts", "tests"]
    names = sorted(set(git("ls-files", "-z", "--cached", "--others", "--exclude-standard", "--", *source_paths).split(b"\0")) - {b""})
    digest = hashlib.sha256()
    for name in names:
        path = root / os.fsdecode(name)
        digest.update(name + b"\0")
        digest.update(hashlib.sha256(path.read_bytes()).digest() if path.is_file() else b"deleted")
    data_dir = Path(os.environ.get("FSP_MCP_HOME", Path.home() / "Library/Application Support/fender-studio-pro-mcp"))
    manifest_path = data_dir / "installation.json"
    manifest = json.loads(manifest_path.read_text())
    assets = []
    for name, expected in sorted(manifest["files"].items()):
        path = Path(name)
        actual = sha256(path) if path.is_file() else None
        assets.append({"name": path.name, "expected_sha256": expected, "actual_sha256": actual, "matches": actual == expected})
    return {
        "git_head": git("rev-parse", "HEAD").decode().strip(),
        "source_dirty": bool(git("status", "--porcelain", "--", *source_paths).strip()),
        "source_sha256": digest.hexdigest(),
        "source_files": len(names),
        "binary_sha256": sha256(binary),
        "installation_manifest_sha256": sha256(manifest_path),
        "installation_version": manifest.get("version"),
        "installed_assets": assets,
        "installed_assets_match_manifest": bool(assets) and all(a["matches"] for a in assets),
    }


def structured(result):
    if isinstance(result.get("structuredContent"), dict):
        return result["structuredContent"]
    for item in result.get("content", []):
        if item.get("type") == "text":
            value = json.loads(item["text"])
            if isinstance(value, dict):
                return value
    raise RuntimeError("Tool returned no structured receipt")


def canonical_notes(result, track):
    require(result.get("track") == track and result.get("mediaType") == "Music", "Note fixture identity did not match")
    require(result.get("truncated") is False, "Note readback is truncated or completeness is unknown")
    parts = deepcopy(result.get("parts"))
    require(isinstance(parts, list) and 0 < len(parts) <= 128, "Expected between 1 and 128 note parts")
    total = 0
    for part in parts:
        require(all(k in part for k in ("name", "start", "end", "muted", "noteCount", "notes")), "Incomplete part readback")
        for value in (part["start"], part["end"]):
            require(type(value) in (int, float) and math.isfinite(value), "Unknown part timing")
        notes = part["notes"]
        require(isinstance(notes, list) and part["noteCount"] == len(notes), "Incomplete part note readback")
        for note in notes:
            require(all(k in note for k in ("pitch", "velocity", "start", "end", "length", "beat", "muted")), "Incomplete note readback")
            require(type(note["pitch"]) is int and 0 <= note["pitch"] <= 127, "Unknown note pitch")
            for key in ("velocity", "start", "end", "length", "beat"):
                require(type(note[key]) in (int, float) and math.isfinite(note[key]), "Unknown note value: " + key)
        notes.sort(key=lambda n: json.dumps(n, sort_keys=True))
        total += len(notes)
    require(0 < total <= 20000, "Expected between 1 and 20000 complete notes")
    parts.sort(key=lambda p: json.dumps(p, sort_keys=True))
    return {"track": track, "mediaType": "Music", "parts": parts}


def shifted_notes(original):
    expected = deepcopy(original)
    for part in expected["parts"]:
        for note in part["notes"]:
            require(1 <= note["pitch"] <= 126, "Fixture pitches must be within 1..126 before transposing")
            note["pitch"] += 1
        part["notes"].sort(key=lambda n: json.dumps(n, sort_keys=True))
    expected["parts"].sort(key=lambda p: json.dumps(p, sort_keys=True))
    return expected


def distribution(values):
    values = sorted(values)
    return None if not values else {
        "samples": len(values), "p50_ms": statistics.median(values),
        "p95_ms": values[math.ceil(len(values) * 0.95) - 1],
        "min_ms": values[0], "max_ms": values[-1],
    }


class SessionValidation:
    def __init__(self, client, report, output, title):
        self.client, self.report, self.output, self.title = client, report, output, title
        self.document = self.session = None

    def checkpoint(self):
        temporary = self.output.with_suffix(".tmp")
        temporary.write_text(json.dumps(self.report, ensure_ascii=False, indent=2, allow_nan=False))
        temporary.chmod(0o600)
        temporary.replace(self.output)

    def operation(self, name, arguments=None, expected_document=None):
        request = {"operation": name, "args": arguments or {}}
        if expected_document is not None:
            request["expected_document"] = expected_document
        entry = {"tool": "operation_run", "arguments": request}
        self.report["receipts"].append(entry)
        started = time.monotonic_ns()
        try:
            raw = self.client.call("operation_run", request)
            entry["wall_ms"] = (time.monotonic_ns() - started) / 1000000
            entry["tool_result"] = raw
            entry["receipt"] = structured(raw)
            error = entry["receipt"].get("error") or {}
            entry["timeout"] = "timeout" in str(error.get("code", "")).lower()
            return entry["receipt"]
        except Exception as error:
            entry.setdefault("wall_ms", (time.monotonic_ns() - started) / 1000000)
            entry["exception"] = str(error)
            entry["timeout"] = isinstance(error, (TimeoutError, queue.Empty))
            raise
        finally:
            self.checkpoint()

    def observed(self, name, arguments=None):
        receipt = self.operation(name, arguments)
        require(not receipt.get("error") and receipt.get("outcome") == "observed", "Read operation failed: " + name)
        if self.document is not None:
            require(receipt.get("document") == self.document and receipt.get("session") == self.session, "Session or document changed")
        return receipt["result"]

    def guard(self):
        context = self.observed("context")
        require(context.get("open") is True and context.get("title") == self.title, "Open exactly the named scratch session")
        if self.document is None:
            require(context.get("document") and context.get("session"), "Missing session or document identity")
            self.document, self.session = context["document"], context["session"]
        require(context.get("document") == self.document and context.get("session") == self.session, "Scratch session identity changed")
        transport = self.observed("song").get("transport", {})
        require(transport.get("playing") is False and transport.get("recording") is False, "Scratch session must be stopped and not recording")

    def second_owner(self, binary):
        self.guard()
        row = self.report["checks"]["second_owner"] = {"performed": True, "status": "running"}
        self.checkpoint()
        try:
            result = subprocess.run([str(binary), "call", "context"], capture_output=True, text=True, timeout=30)
            row.update({"exit_code": result.returncode, "stdout": result.stdout, "stderr": result.stderr})
            require(result.returncode != 0 and "controller_busy" in result.stdout + result.stderr, "Second owner was not rejected with controller_busy")
            row["status"] = "passed"
        finally:
            self.checkpoint()

    def stale_document(self, preferred_channel):
        self.guard()
        tracks = self.observed("tracks", {"events": False})
        channels = self.observed("channels")
        fixtures = []
        for track in tracks:
            channel = track.get("channel")
            if track.get("mediaType") != "Audio" or not channel:
                continue
            if preferred_channel:
                if channel != preferred_channel:
                    continue
            elif not track.get("name", "").startswith(("MCP ", "FSP Verify ")):
                continue
            if sum(t.get("channel") == channel for t in tracks) != 1:
                continue
            matching = [c for c in channels if c.get("label") == channel]
            if len(matching) == 1 and matching[0].get("mute") in (0, 1, False, True):
                fixtures.append(matching[0])
        require(fixtures, "No unique existing audio fixture channel; supply --audio-channel")
        channel = fixtures[0]
        row = self.report["checks"]["stale_document"] = {"performed": True, "status": "running", "channel": channel["label"]}
        self.checkpoint()
        receipt = self.operation("setChannel", {"channel": channel["label"], "field": "mute", "value": not bool(channel["mute"])}, self.document + ":stale-validation")
        row["actual_outcome"] = receipt.get("outcome")
        require(receipt.get("outcome") == "not_applied" and (receipt.get("error") or {}).get("code") == "document_changed", "Stale-document write was not explicitly refused; do not retry")
        require(receipt.get("document") == self.document and receipt.get("session") == self.session, "Document changed during stale-token check")
        after = [c for c in self.observed("channels") if c.get("label") == channel["label"]]
        require(len(after) == 1 and after[0].get("mute") == channel["mute"], "Stale-document check changed the channel")
        row["status"] = "passed"
        self.checkpoint()

    def measure(self, operation, count, arguments=None):
        row = self.report["performance"][operation] = {"requested_samples": count, "arguments": arguments or {}, "samples": []}
        for index in range(count):
            sample = {"index": index + 1, "success": False}
            row["samples"].append(sample)
            try:
                result = self.observed(operation, arguments)
                if operation == "tracks":
                    require(isinstance(result, list), "Track snapshot did not return a list")
                    sample["actual_track_count"] = len(result)
                else:
                    require(isinstance(result, dict) and result.get("pong") is True, "Ping did not return pong")
                sample["success"] = True
            except Exception as error:
                sample["error"] = str(error)
            entry = self.report["receipts"][-1]
            sample.update({"wall_ms": entry["wall_ms"], "receipt_elapsed_ms": entry.get("receipt", {}).get("elapsed_ms"), "timeout": entry.get("timeout", False)})
            if entry.get("receipt", {}).get("document") not in (None, self.document) or entry.get("receipt", {}).get("session") not in (None, self.session):
                self.checkpoint()
                raise RuntimeError("Document or bridge session changed during measurement")
            self.checkpoint()
            if len(row["samples"]) >= 3 and not any(s["success"] for s in row["samples"][-3:]):
                break
        successful = [s for s in row["samples"] if s["success"]]
        row.update({
            "completed_samples": len(row["samples"]), "successful_samples": len(successful),
            "failed_samples": len(row["samples"]) - len(successful), "not_run_samples": count - len(row["samples"]),
            "timeouts": sum(s["timeout"] for s in row["samples"]),
            "wall_all": distribution([s["wall_ms"] for s in row["samples"]]),
            "wall_successful": distribution([s["wall_ms"] for s in successful]),
            "receipt_successful": distribution([s["receipt_elapsed_ms"] for s in successful if type(s["receipt_elapsed_ms"]) in (int, float)]),
        })
        if operation == "tracks":
            row["actual_track_counts"] = sorted({s["actual_track_count"] for s in successful})
        self.checkpoint()
        require(row["completed_samples"] == count, "Measurement stopped after three consecutive failures")

    def notes(self, track):
        return canonical_notes(self.observed("notes", {"track": track, "maxNotes": 20000}), track)

    def note_roundtrip(self, track):
        row = self.report["checks"]["notes"] = {"requested": True, "performed": True, "status": "running", "track": track, "change_readback_verified": False, "restore_readback_verified": False}
        self.checkpoint()
        self.guard()
        original = self.notes(track)
        expected = shifted_notes(original)
        row.update({
            "parts": len(original["parts"]), "notes": sum(p["noteCount"] for p in original["parts"]),
        })
        self.checkpoint()
        try:
            self.guard()
            require(self.notes(track) == original, "Notes changed before transpose")
            try:
                change = self.operation("editNotes", {"track": track, "ops": [{"op": "transpose", "semitones": 1}]}, self.document)
                row["change_receipt_outcome"] = change.get("outcome")
            except Exception as error:
                row["change_exception"] = str(error)
            self.guard()
            changed = self.notes(track)
            row["change_readback_verified"] = changed == expected
            row["original_still_present"] = changed == original
            require(changed == expected, "Transpose readback is not the complete +1 state; no automatic inverse will run")
        finally:
            try:
                if row["change_readback_verified"]:
                    self.guard()
                    require(self.notes(track) == expected, "Notes changed before restoration; no inverse dispatched")
                    row["restore_dispatched"] = True
                    self.checkpoint()
                    try:
                        restored = self.operation("editNotes", {"track": track, "ops": [{"op": "transpose", "semitones": -1}]}, self.document)
                        row["restore_receipt_outcome"] = restored.get("outcome")
                    except Exception as error:
                        row["restore_exception"] = str(error)
                    self.guard()
                    row["restore_readback_verified"] = self.notes(track) == original
                    require(row["restore_readback_verified"], "Restoration readback differs from original; do not repeat transpose")
                    row["status"] = "passed"
            finally:
                if row["status"] == "running":
                    row["status"] = "failed_restore_unverified" if row["change_readback_verified"] else "failed_no_automatic_restore"
                self.checkpoint()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", default="target/debug/fender-studio-pro-mcp")
    parser.add_argument("--scratch-title", required=True)
    parser.add_argument("--audio-channel", help="Existing unique audio fixture channel; otherwise detect an MCP/FSP Verify fixture")
    parser.add_argument("--note-track", help="Opt in to a +1/-1 transpose roundtrip on this unique instrument track")
    parser.add_argument("--output", default=".local/live/session-validation.json")
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    binary = Path(args.binary).resolve()
    output = Path(args.output).resolve()
    require(output.is_relative_to(root / ".local/live"), "Output must remain in the private .local/live directory")
    os.umask(0o077)
    output.parent.mkdir(parents=True, exist_ok=True)
    report = {
        "started_at": datetime.now(timezone.utc).isoformat(), "scratch_title": args.scratch_title,
        "acceptance": {"scratch": "exact title, open, stopped, same document/session", "second_owner": "nonzero exit and controller_busy", "stale_document": "not_applied/document_changed with unchanged channel", "notes": "full +1 readback then exact original after -1, when opted in", "performance": "100 warm ping and 20 actual-size track snapshots; report errors and timeouts"},
        "environment": {"system": platform.system(), "release": platform.release(), "machine": platform.machine(), "cpu_count": os.cpu_count(), "load_average_before": os.getloadavg()},
        "checks": {"second_owner": {"performed": False}, "stale_document": {"performed": False}, "notes": {"requested": bool(args.note_track), "performed": False, "status": "not_run" if args.note_track else "not_requested"}},
        "performance": {"baseline": "absent", "comparison": "none", "run_count": 1, "percentiles": "p50 median; p95 nearest-rank", "wall_clock": "monotonic; complete MCP tools/call until parsed JSON-RPC response, excluding report writes", "receipt_clock": "server elapsed_ms", "limitations": ["single warm run", "actual session size, not a synthetic 64-track fixture", "no before/after speed claim", "limiter not profiled"]},
        "not_implemented": ["capture_job", "managed_recording_job", "export_job"], "receipts": [],
        "status": "running",
    }
    client = None
    validation = SessionValidation(client, report, output, args.scratch_title)
    validation.checkpoint()
    try:
        report["provenance_before"] = provenance(root, binary)
        require(report["provenance_before"]["installed_assets_match_manifest"], "Installed assets do not match installation manifest")
        validation.checkpoint()
        client = McpClient([str(binary), "serve"], timeout=30)
        validation.client = client
        report["mcp_initialization"] = client.initialize
        validation.guard()
        validation.second_owner(binary)
        validation.stale_document(args.audio_channel)
        validation.guard()
        validation.measure("ping", 100)
        validation.measure("tracks", 20, {"events": True, "maxEvents": 50})
        validation.guard()
        if args.note_track:
            validation.note_roundtrip(args.note_track)
        report["provenance_after"] = provenance(root, binary)
        require(report["provenance_before"] == report["provenance_after"], "Binary, sources or installed assets changed during validation")
        require(all(report["performance"][name]["failed_samples"] == 0 for name in ("ping", "tracks")), "Performance run included failed samples")
        report["status"] = "passed"
    except (Exception, KeyboardInterrupt) as error:
        report["status"] = "failed"
        report["error"] = str(error) or type(error).__name__
        for check in report["checks"].values():
            if check.get("status") == "running":
                check["status"] = "failed"
    finally:
        if client is not None:
            try:
                client.close()
            except Exception as error:
                report["close_error"] = str(error)
                report["status"] = "failed"
        report["completed_at"] = datetime.now(timezone.utc).isoformat()
        report["environment"]["load_average_after"] = os.getloadavg()
        validation.checkpoint()
    print(json.dumps({"status": report["status"], "checks": {name: {k: v for k, v in check.items() if k in ("performed", "status", "change_readback_verified", "restore_readback_verified")} for name, check in report["checks"].items()}}, ensure_ascii=False))
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
