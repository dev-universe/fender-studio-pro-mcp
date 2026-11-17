// studio-one-mcp track edit task ("Track/MCP Track Edit").
//
// A TrackEdit task gets what Studio One's own track scripts use (Add Bus for
// Selected Channels, Pack Folder…): context.functions with moveToFolder and
// colorizeEvent, functions.root.environment (the MixerConsole), and
// functions.root.createFunctions("DeviceEditFunctions") with connectChannel.
// The bridge writes operations to mailbox/track-edit-request.json, runs this
// task's command, and reads mailbox/track-edit-result.json. No dialog.
//
// Every host member is checked before use: a throw, or a call on a missing
// member, pops a modal Scripting Error dialog in Studio One.
include_file("McpEditConfig.js");
include_file("McpTaskGuard.js");
include_file("McpTrackOps.js");

function mteUrl(name) { return Host.Url(McpEditConfig.mailbox + name); }

function mteRead(name) {
	if (!Host.IO.File(mteUrl(name)).exists()) return null;
	var f = Host.IO.openTextFile(mteUrl(name), "utf-8");
	if (!f) return null;
	var text = f.readLine();
	f.close();
	if (typeof text !== "string" || !text) return null;
	var value = null;
	try { value = JSON.parse(text.replace(/^﻿/, "")); } catch (_) { value = null; }
	return value;
}

function mteWrite(name, value) {
	var f = Host.IO.createTextFile(mteUrl(name), "utf-8");
	if (!f) return false;
	f.writeString(JSON.stringify(value) + "\n");
	f.close();
	return true;
}

function mteFn(obj, key) { return !!obj && typeof obj[key] === "function"; }

// Only legacy identity reads may omit outcome. Final observations must report
// observed; every present status/error field must agree with a known read.
function mteNativeReadObserved(value, legacyIdentity) {
	if (!value || typeof value !== "object" || Array.isArray(value) || value.ok !== true) return false;
	if (legacyIdentity !== true && value.outcome !== "observed") return false;
	var has = Object.prototype.hasOwnProperty;
	var outcomes = ["outcome", "native_outcome"];
	for (var i = 0; i < outcomes.length; i++) {
		if (has.call(value, outcomes[i]) && value[outcomes[i]] !== "observed") return false;
	}
	var errors = ["error", "native_error"];
	for (var j = 0; j < errors.length; j++) {
		if (has.call(value, errors[j]) && value[errors[j]] !== null) return false;
	}
	if (has.call(value, "errors") && (!Array.isArray(value.errors) || value.errors.length !== 0)) return false;
	return true;
}

function mteDocumentEnvironmentSignature(value) {
	if (!value || typeof value !== "object" || Array.isArray(value)
		|| value.scope !== "fixed_active_document_controller_environment"
		|| value.mutation_capability !== false || value.atomic_snapshot !== false) return null;
	var keys = Object.keys(value).sort().join(",");
	if (value.status === "observed" && typeof value.ancestor_matches === "boolean"
		&& keys === "ancestor_matches,atomic_snapshot,mutation_capability,scope,status") return JSON.stringify([value.status, value.ancestor_matches]);
	if (value.status === "unavailable" && typeof value.reason === "string" && value.reason.length > 0 && value.reason.length <= 512
		&& keys === "atomic_snapshot,mutation_capability,reason,scope,status") return JSON.stringify([value.status, value.reason]);
	return null;
}

function mteGraphReflection(channel, graphFunctions) {
	try {
		var inserts = mteFn(channel, "find") ? channel.find("Inserts") : null;
		var slot = mteFn(inserts, "find") ? inserts.find("FX01") : null;
		var device = mteFn(slot, "find") ? slot.find("Device") : null;
		var objects = Host.Objects;
		var service = mteFn(objects, "getObjectByName") ? objects.getObjectByName("FenderMcpNative") : null;
		if (!device || !graphFunctions || !service || !mteFn(service, "capabilities") || !mteFn(service, "request")) return null;
		var capabilities = JSON.parse(service.capabilities());
		if (!capabilities.ok || capabilities.api !== "fender-mcp-native-v1") return null;
		var identityText = service.request(device, JSON.stringify({ operation: "inspect" }));
		if (typeof identityText !== "string" || identityText.length > 262144) return { error: "invalid native identity response" };
		var identity = JSON.parse(identityText);
		if (!mteNativeReadObserved(identity, true) || !identity.instance || typeof identity.instance.object_uid !== "string") return { error: "native target identity unavailable" };
		var text = service.request(device, JSON.stringify({ operation: "inspect_aux", expected_instance: identity.instance.object_uid }), graphFunctions);
		if (typeof text !== "string" || text.length > 262144) return { error: "invalid native reflection response" };
		var result = JSON.parse(text);
		if (!mteNativeReadObserved(result) || !result.instance || result.instance.object_uid !== identity.instance.object_uid
			|| result.scope !== "auxiliary_type_observation" || result.mutation_capability !== false) return { error: "native reflection identity or scope unavailable" };
		return result;
	} catch (error) { return { error: String(error).slice(0, 512) }; }
}

// Fixed parent-chain observation of this channel's actual Inserts folder.
// The existing auxiliary metadata helper keeps its original one-node profile.
function mteInsertFolderAncestry(channel) {
	try {
		var inserts = mteFn(channel, "find") ? channel.find("Inserts") : null;
		var slot = mteFn(inserts, "find") ? inserts.find("FX01") : null;
		var device = mteFn(slot, "find") ? slot.find("Device") : null;
		var objects = Host.Objects;
		var service = mteFn(objects, "getObjectByName") ? objects.getObjectByName("FenderMcpNative") : null;
		if (!device || !inserts || !service || !mteFn(service, "capabilities") || !mteFn(service, "request")) return null;
		var capabilities = JSON.parse(service.capabilities());
		if (!capabilities.ok || capabilities.api !== "fender-mcp-native-v1") return null;
		var identityText = service.request(device, JSON.stringify({ operation: "inspect" }));
		if (typeof identityText !== "string" || identityText.length > 262144) return { error: "invalid native identity response" };
		var identity = JSON.parse(identityText);
		if (!mteNativeReadObserved(identity, true) || !identity.instance || typeof identity.instance.object_uid !== "string") return { error: "native target identity unavailable" };
		var text = service.request(device, JSON.stringify({ operation: "inspect_insert_folder_ancestry", expected_instance: identity.instance.object_uid }), inserts);
		if (typeof text !== "string" || text.length > 262144) return { error: "invalid native ancestry response" };
		var result = JSON.parse(text);
		if (!mteNativeReadObserved(result) || !result.instance || result.instance.object_uid !== identity.instance.object_uid
			|| result.scope !== "insert_folder_ancestry_observation" || result.mutation_capability !== false || result.atomic_snapshot !== false
			|| typeof result.ancestor_matches_active_model !== "boolean" || !result.insert_folder_ancestry_report
			|| result.insert_folder_ancestry_report.traversal !== "parent_chain_only"
			|| result.insert_folder_ancestry_report.ancestor_matches_active_model !== result.ancestor_matches_active_model) return { error: "native ancestry identity or scope unavailable" };
		if (Object.prototype.hasOwnProperty.call(result, "document_environment_observation")
			|| Object.prototype.hasOwnProperty.call(result.insert_folder_ancestry_report, "document_environment_observation")) {
			var environment = mteDocumentEnvironmentSignature(result.document_environment_observation);
			if (environment === null || environment !== mteDocumentEnvironmentSignature(result.insert_folder_ancestry_report.document_environment_observation))
				return { error: "native document Environment observation unavailable" };
		}
		return result;
	} catch (error) { return { error: String(error).slice(0, 512) }; }
}

function mteCanonicalUid(value) {
	return typeof value === "string" && /^[0-9A-F]{8}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{12}$/.test(value)
		&& value !== "00000000-0000-0000-0000-000000000000";
}

function mteInsertFolderPresetPreflight(channel) {
	try {
		var inserts = mteFn(channel, "find") ? channel.find("Inserts") : null;
		var slot = mteFn(inserts, "find") ? inserts.find("FX01") : null;
		var device = mteFn(slot, "find") ? slot.find("Device") : null;
		var objects = Host.Objects;
		var service = mteFn(objects, "getObjectByName") ? objects.getObjectByName("FenderMcpNative") : null;
		if (!device || !inserts || !service || !mteFn(service, "capabilities") || !mteFn(service, "request"))
			return { status: "unavailable", reason: "fixed native folder preset preflight unavailable" };
		var capabilities = JSON.parse(service.capabilities());
		if (!mteNativeReadObserved(capabilities, true) || capabilities.api !== "fender-mcp-native-v1")
			return { error: "native preset preflight capability response unavailable" };
		if (capabilities.insert_folder_preset_preflight_observation !== true)
			return { status: "unavailable", reason: "native folder preset preflight capability unavailable" };
		var identityText = service.request(device, JSON.stringify({ operation: "inspect" }));
		if (typeof identityText !== "string" || identityText.length > 262144) return { error: "invalid native identity response" };
		var identity = JSON.parse(identityText);
		if (!mteNativeReadObserved(identity, true) || !identity.instance || !mteCanonicalUid(identity.instance.object_uid)
			|| !mteCanonicalUid(identity.instance.class_uid) || typeof identity.instance.native_class !== "string" || !identity.instance.native_class
			|| typeof identity.instance.object_id !== "string" || !identity.instance.object_id) return { error: "native target identity unavailable" };
		var text = service.request(device, JSON.stringify({ operation: "inspect_insert_folder_preset_preflight", expected_instance: identity.instance.object_uid }), inserts);
		if (typeof text !== "string" || text.length > 262144) return { error: "invalid native preset preflight response" };
		var result = JSON.parse(text);
		if (!mteNativeReadObserved(result) || !result.instance || result.instance.object_uid !== identity.instance.object_uid
			|| result.instance.class_uid !== identity.instance.class_uid || result.instance.native_class !== identity.instance.native_class
			|| result.instance.object_id !== identity.instance.object_id || result.scope !== "insert_folder_preset_preflight_observation"
			|| result.mutation_capability !== false || result.atomic_snapshot !== false) return { error: "native preset preflight identity or scope unavailable" };
		var allowed = ["ok", "outcome", "native_outcome", "error", "native_error", "errors", "instance", "scope", "mutation_capability", "atomic_snapshot", "status"];
		if (result.status === "unavailable") {
			allowed.push("reason");
			if (typeof result.reason !== "string" || !result.reason || result.reason.length > 512) return { error: "invalid native preset unavailability" };
		} else if (result.status === "observed") {
			allowed = allowed.concat(["anchor_slot_bound", "folder", "document_environment_observation", "target_flags", "metadata"]);
			var folder = result.folder, metadata = result.metadata;
			if (result.anchor_slot_bound !== true || !folder || Object.keys(folder).sort().join(",") !== "class_uid,id,native_class,object_uid"
				|| folder.id !== "Inserts" || folder.native_class !== "AudioInsertFolder" || !mteCanonicalUid(folder.object_uid)
				|| (folder.class_uid !== null && !mteCanonicalUid(folder.class_uid))
				|| mteDocumentEnvironmentSignature(result.document_environment_observation) !== JSON.stringify(["observed", true])
				|| typeof result.target_flags !== "number" || result.target_flags % 1 !== 0 || result.target_flags < 3 || result.target_flags > 2147483647
				|| (result.target_flags & 3) !== 3 || !metadata
				|| Object.keys(metadata).sort().join(",") !== "attribute_count,class_category,class_name,class_vendor"
				|| typeof metadata.attribute_count !== "number" || metadata.attribute_count % 1 !== 0 || metadata.attribute_count < 0 || metadata.attribute_count > 64)
				return { error: "native preset preflight binding or metadata unavailable" };
			var fields = ["class_name", "class_vendor", "class_category"];
			for (var i = 0; i < fields.length; i++) {
				var value = metadata[fields[i]];
				if (value !== null && (typeof value !== "string" || value.length > 256 || value.indexOf("\u0000") >= 0))
					return { error: "native preset preflight text unavailable" };
			}
		} else return { error: "native preset preflight status unavailable" };
		var keys = Object.keys(result);
		for (var k = 0; k < keys.length; k++) if (allowed.indexOf(keys[k]) < 0) return { error: "unexpected native preset preflight field" };
		return result;
	} catch (error) { return { error: String(error).slice(0, 512) }; }
}

// Which of `names` exist on obj, and as what (for the probe).
function mteShape(obj, names) {
	var out = {};
	if (!obj) return null;
	for (var i = 0; i < names.length; i++) out[names[i]] = typeof obj[names[i]];
	return out;
}

// All mixer channels as [{ label, channel }], trying the list shapes seen in Studio One scripts.
function mteChannels(consoleObj) {
	var out = [];
	if (!mteFn(consoleObj, "getChannelList")) return out;
	// 1 is the only list kind Studio One's own scripts use; others are not guessed at.
	var list = consoleObj.getChannelList(1);
	if (!list) return out;
	var n = typeof list.numChannels === "number" ? list.numChannels : (typeof list.count === "number" ? list.count : 0);
	for (var i = 0; i < n; i++) {
		var ch = mteFn(list, "getChannel") ? list.getChannel(i) : (mteFn(list, "at") ? list.at(i) : null);
		if (!ch) continue;
		var label = typeof ch.label === "string" ? ch.label : (typeof ch.name === "string" ? ch.name : null);
		if (label === null) continue;
		out.push({ label: label, channel: ch });
	}
	// Studio Pro 8 also lists each instrument track's note channel (mediaType
	// "Music") under the same label as its audio channel; keep the audio one.
	var audio = {};
	for (var a = 0; a < out.length; a++) if (out[a].channel.mediaType === "Audio") audio[out[a].label] = true;
	var kept = [];
	for (var b = 0; b < out.length; b++) if (!(out[b].channel.mediaType === "Music" && audio[out[b].label])) kept.push(out[b]);
	return kept;
}

function mteTracks(context) {
	var out = [];
	var tl = context.mainTrackList;
	if (!tl || typeof tl.numTracks !== "number" || !mteFn(tl, "getTrack")) return out;
	for (var i = 0; i < tl.numTracks; i++) {
		var t = tl.getTrack(i);
		if (!t) continue;
		var seen = false;
		for (var k = 0; k < out.length; k++) if (out[k] === t) seen = true;
		if (!seen) out.push(t);
	}
	return out;
}

// JSON-safe view of a host object, for the eval probe (as the bridge's describe).
function mteDescribe(value, depth) {
	if (value === null || value === undefined) return value === undefined ? "<undefined>" : null;
	var t = typeof value;
	if (t === "number" || t === "boolean" || t === "string") return value;
	if (t === "function") return "<function>";
	if (Array.isArray(value)) return depth <= 0 ? "<array " + value.length + ">" : value.slice(0, 200).map(function (v) { return mteDescribe(v, depth - 1); });
	try {
		var json = JSON.stringify(value);
		if (json !== undefined && json !== "{}") return JSON.parse(json);
	} catch (_) {}
	if (depth <= 0) return "<object>";
	var out = {}, keys = [];
	try { keys = Object.getOwnPropertyNames(value); } catch (_) {}
	try { for (var k in value) if (keys.indexOf(k) < 0) keys.push(k); } catch (_) {}
	for (var i = 0; i < keys.length && i < 200; i++) {
		try { out[keys[i]] = mteDescribe(value[keys[i]], depth - 1); } catch (e) { out[keys[i]] = "<error " + e + ">"; }
	}
	if (!keys.length) { try { out["<string>"] = String(value); } catch (_) {} }
	return out;
}

function mteApply(context, ops) {
	var fns = context.functions;
	var root = fns ? fns.root : null;
	var env = root ? root.environment : null;
	var consoleObj = env && mteFn(env, "find") ? env.find("MixerConsole") : null;
	var results = [];
	for (var i = 0; i < ops.length; i++) {
		var op = ops[i] || {};
		var r = { op: op.op };
		if (op.op === "probe") {
			r.context = mteShape(context, ["functions", "mainTrackList", "editor", "iterator"]);
			r.functions = mteShape(fns, ["root", "moveToFolder", "colorizeEvent", "renameEvent", "addTrack", "removeTrack", "executeImmediately"]);
			r.root = mteShape(root, ["environment", "createFunctions"]);
			r.console = mteShape(consoleObj, ["getChannelList", "canConnectToBus", "canConnectToVCA"]);
			var list = consoleObj && mteFn(consoleObj, "getChannelList") ? consoleObj.getChannelList(1) : null;
			r.channelList = mteShape(list, ["numChannels", "count", "getChannel", "at", "numSelectedChannels", "getSelectedChannel"]);
			var chans = mteChannels(consoleObj);
			r.channelCount = chans.length;
			r.firstChannels = [];
			for (var c = 0; c < chans.length && c < 4; c++) r.firstChannels.push(chans[c].label);
			r.channel = chans.length ? mteShape(chans[0].channel, ["label", "name", "input", "getDestinationChannel", "connectTo"]) : null;
			var dev = root && mteFn(root, "createFunctions") ? root.createFunctions("DeviceEditFunctions") : null;
			r.deviceFunctions = mteShape(dev, ["connectChannel", "insertDevice", "removeDevice"]);
			var graphFunctions = root && mteFn(root, "createFunctions") ? root.createFunctions("AudioFXGraphEditFunctions") : null;
			r.graphFunctions = mteShape(graphFunctions, ["graph", "root", "addSplitter", "insertNode", "removeNode", "modifySplitCount", "modifySplitMode", "setSplitCount", "setSplitMode"]);
			r.commandArguments = {};
			var commandNames = ["Select Mix FX", "Select Panner"];
			for (var cmd = 0; cmd < commandNames.length; cmd++) {
				var command = Host.GUI.Commands.findCommand("Console", commandNames[cmd]);
				r.commandArguments[commandNames[cmd]] = command ? mteDescribe(command.arguments, 2) : null;
			}
			if (op.channel) {
				var matches = chans.filter(function (c) { return c.label === op.channel; });
				if (matches.length !== 1) r.error = "probe needs a unique audio channel";
				else {
					var channel = matches[0].channel;
					r.target = { label: op.channel, children: {} };
					r.graphReflection = mteGraphReflection(channel, graphFunctions);
					r.contextEditorReflection = mteGraphReflection(channel, context.editor);
					var insertFolder = mteFn(channel, "find") ? channel.find("Inserts") : null;
					r.insertFolderReflection = mteGraphReflection(channel, insertFolder);
					r.insertFolderAncestry = mteInsertFolderAncestry(channel);
					r.insertFolderPresetPreflight = mteInsertFolderPresetPreflight(channel);
					var panner = mteFn(channel, "find") ? channel.find("Panner") : null;
					r.target.pannerParameters = [];
					var pannerNames = ["pannerType", "panMode", "stereoMode", "panX1", "panX2", "panWidth", "effectClass"];
					for (var pn = 0; pn < pannerNames.length; pn++) {
						var pp = mteFn(panner, "findParameter") ? panner.findParameter(pannerNames[pn]) : null;
						if (pp) r.target.pannerParameters.push({ name: pannerNames[pn], value: pp.value, text: typeof pp.string === "string" ? pp.string : null, min: pp.min, max: pp.max });
					}
					var methods = ["name", "numItems", "count", "find", "findParameter", "getDevice", "insertDeviceClass", "newIterator", "createIterator", "createFunctions"];
					var children = ["Panner", "AudioBusMixFX", "MixFXRack", "MixFX", "MixFx", "MixFxFolder", "MixFXFolder", "MixEffects", "Inserts", "Graph", "FXGraph"];
					for (var z = 0; z < children.length; z++) {
						var child = mteFn(channel, "find") ? channel.find(children[z]) : null;
						var info = { members: mteShape(child, methods), children: {} };
					var nested = ["PanRack", "PanFX", "Graph", "FXGraph", "Combinator", "Device", "FX01"];
						for (var y = 0; y < nested.length; y++) info.children[nested[y]] = mteShape(mteFn(child, "find") ? child.find(nested[y]) : null, methods);
						var combinator = mteFn(child, "find") ? child.find("Combinator") : null;
						info.combinatorGraph = mteShape(mteFn(combinator, "find") ? combinator.find("Graph") : null, methods);
						r.target.children[children[z]] = info;
					}
				}
			}
			var tracks = mteTracks(context);
			r.trackCount = tracks.length;
			r.track = tracks.length ? mteShape(tracks[0], ["name", "isFolder", "parentFolderID", "channel"]) : null;
		} else if (op.op === "route") {
			// { channel, to }: send a channel's output to another channel's input (a bus, the main out).
			var all = mteChannels(consoleObj);
			var src = null, dst = null, nSrc = 0, nDst = 0;
			for (var a = 0; a < all.length; a++) {
				if (all[a].label === op.channel) { src = all[a].channel; nSrc++; }
				if (all[a].label === op.to) { dst = all[a].channel; nDst++; }
			}
			var devf = root && mteFn(root, "createFunctions") ? root.createFunctions("DeviceEditFunctions") : null;
			if (nSrc !== 1) r.error = nSrc ? "channel name is ambiguous: " + op.channel : "no channel named " + op.channel;
			else if (nDst !== 1) r.error = nDst ? "destination name is ambiguous: " + op.to : "no channel named " + op.to;
			else if (!devf || !mteFn(devf, "connectChannel")) r.error = "connectChannel is not available";
			else if (!dst.input) r.error = op.to + " has no input to route to";
			else { devf.connectChannel(src, dst.input); r.done = true; }
		} else if (op.op === "folder") {
			// { tracks: [names], folder: name, create }: move tracks into a folder track
			// (made first when create is set). The folder is expanded before the move,
			// as Studio One's own folder script does: tracks inside a collapsed folder
			// drop out of the track list that scripts see.
			var ts = mteTracks(context);
			var folder = null;
			var folderCount = 0;
			for (var b = 0; b < ts.length; b++) if (ts[b].name === op.folder && ts[b].isFolder) { folder = ts[b]; folderCount++; }
			if (folderCount > 1) { r.error = "folder name is ambiguous: " + op.folder; results.push(r); continue; }
			var names = op.tracks && op.tracks.length ? op.tracks : [];
			var ambiguous = null;
			for (var u = 0; u < names.length; u++) {
				var count = 0;
				for (var v = 0; v < ts.length; v++) if (ts[v].name === names[u] && ts[v] !== folder) count++;
				if (count > 1) { ambiguous = names[u]; break; }
			}
			if (ambiguous !== null) { r.error = "track name is ambiguous: " + ambiguous; results.push(r); continue; }
			if (!folder && op.create && mteFn(fns, "addTrack")) {
				fns.executeImmediately = true;
				var tl = context.mainTrackList;
				var at = tl && mteFn(tl, "getInsertPosition") ? tl.getInsertPosition() : 0;
				folder = fns.addTrack("FolderTrack", at, op.folder);
				r.created = !!folder;
			}
			var folders = context.editor && context.editor.model ? context.editor.model.folders : null;
			if (!folder) r.error = "no folder track named " + op.folder;
			else if (!mteFn(fns, "moveToFolder")) r.error = "moveToFolder is not available";
			else {
				if (folders && mteFn(folders, "isExpanded") && mteFn(folders, "toggleExpand") && !folders.isExpanded(folder)) folders.toggleExpand(folder);
				r.moved = [];
				r.missing = [];
				for (var d = 0; d < names.length; d++) {
					var hit = null;
					for (var e = 0; e < ts.length; e++) if (ts[e].name === names[d] && ts[e] !== folder) hit = ts[e];
					if (hit) { fns.moveToFolder(folder, hit); r.moved.push(names[d]); } else r.missing.push(names[d]);
				}
			}
		} else if (op.op === "renameEvents") {
			// { track, name, numbered }: rename every event on a track, like Studio
			// One's Rename Events ("name(01)", "name(02)"… in time order when numbered).
			var tt = mteTracks(context);
			var tr = null, nTr = 0;
			for (var g = 0; g < tt.length; g++) if (tt[g].name === op.track) { tr = tt[g]; nTr++; }
			if (nTr !== 1) r.error = nTr ? "track name is ambiguous: " + op.track : "no track named " + op.track;
			else if (typeof op.name !== "string" || !op.name) r.error = "renameEvents needs name";
			else if (!mteFn(fns, "renameEvent") || !mteFn(tr, "createIterator")) r.error = "renameEvent is not available";
			else {
				var evs = [];
				var it = tr.createIterator();
				var ev;
				while (it && (ev = it.next())) evs.push(ev);
				evs.sort(function (x, y) { return (x.startTime ? x.startTime.seconds : 0) - (y.startTime ? y.startTime.seconds : 0); });
				r.renamed = [];
				for (var h = 0; h < evs.length; h++) {
					var nm = op.numbered ? op.name + "(" + (h < 9 ? "0" : "") + (h + 1) + ")" : op.name;
					fns.renameEvent(evs[h], nm);
					r.renamed.push(nm);
				}
			}
		} else if (op.op === "eval") {
            r.error = "arbitrary script execution is not supported";
		} else if (typeof mtoOps === "object" && mtoOps.hasOwnProperty(op.op)) {
			var more = mtoOps[op.op](context, op) || {};
			for (var key in more) if (more.hasOwnProperty(key)) r[key] = more[key];
		} else {
			r.error = "unknown op " + op.op;
		}
		results.push(r);
	}
	return results;
}

function McpTrackEditTask() {
	this.interfaces = [Host.Interfaces.IEditTask];
	this.prepareEdit = function (context) { return Host.Results.kResultOk; };
	this.performEdit = function (context) {
		var claim = mcpClaimTask(mteRead, mteWrite, "track-edit-request.json", "track");
		var req = claim.request;
		if (claim.error) {
			mteWrite("track-edit-result.json", { id: req && req.id ? req.id : null, error: claim.error });
			return Host.Results.kResultOk;
		}
		mteWrite("track-edit-result.json", { id: req.id, results: mteApply(context, req.ops) });
		return Host.Results.kResultOk;
	};
}

function createMcpTrackEdit() { return new McpTrackEditTask(); }
