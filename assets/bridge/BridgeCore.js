// studio-one-mcp bridge core, shared by the device and component scripts.
//
// Studio One's script engine has no sockets, so the bridge talks to the outside
// world through a mailbox folder (path set in BridgeConfig.js at install time):
//
//   status.json    written by us: {protocol, session, startedAt, heartbeat, component}
//   request.json   written by the client (atomically): {id, op, args}
//   response.json  written by us: {id, session, ok, result | error, ms}
//
// Runs in the component script, the only one of the two with a Host object.
// Studio One 5 has no usable script timer, so the bridge is event-driven: after
// writing request.json the client sends a MIDI CC that the surface maps to the
// component's bridgeTick parameter, and each change calls tick().

const kProtocol = 2;
const kHeartbeatMs = 2000;

function newSession() {
    let s = "";
    for (let i = 0; i < 4; i++)
        s += ("00000000" + Math.floor(Math.random() * 4294967296).toString(16)).slice(-8);
    return s;
}

// JSON-safe view of anything, including host objects that JSON.stringify chokes on.
function describe(value, depth) {
    if (depth === undefined) depth = 2;
    if (value === null || value === undefined) return value === undefined ? "<undefined>" : null;
    const t = typeof value;
    if (t === "number" || t === "boolean" || t === "string") return value;
    if (t === "function") return "<function>";
    if (Array.isArray(value)) return depth <= 0 ? "<array " + value.length + ">" : value.slice(0, 200).map(v => describe(v, depth - 1));
    try {
        const json = JSON.stringify(value);
        if (json !== undefined && json !== "{}") return JSON.parse(json);
    } catch (_) {}
    if (depth <= 0) return "<object>";
    const out = {};
    let keys = [];
    try { keys = Object.getOwnPropertyNames(value); } catch (_) {}
    try { for (const k in value) if (keys.indexOf(k) < 0) keys.push(k); } catch (_) {}
    for (const k of keys.slice(0, 200)) {
        try { out[k] = describe(value[k], depth - 1); } catch (e) { out[k] = "<error " + e + ">"; }
    }
    if (!keys.length) { try { out["<string>"] = String(value); } catch (_) {} }
    return out;
}

class Mailbox {
    constructor(dirUrl) { this.dir = dirUrl.slice(-1) === "/" ? dirUrl : dirUrl + "/"; }
    url(name) { return Host.Url(this.dir + name); }
    read(name) {
        let f = null;
        try {
            if (!Host.IO.File(this.url(name)).exists()) return null;
            f = Host.IO.openTextFile(this.url(name), "utf-8");
            if (!f) return null;
            // The client always writes a single line of JSON.
            const text = f.readLine();
            return typeof text === "string" && text ? JSON.parse(text.replace(/^﻿/, "")) : null;
        } catch (_) {
            return null; // half-written or not JSON yet; try again next tick
        } finally { if (f) f.close(); }
    }
    write(name, value) {
        let f = null;
        try {
            f = Host.IO.createTextFile(this.url(name), "utf-8");
            if (!f) return false;
            f.writeString(JSON.stringify(value) + "\n");
            return true;
        } finally { if (f) f.close(); }
    }
}

class Bridge {
    constructor(config, component) {
        this.config = config;
        this.component = component;
        this.mailbox = new Mailbox(config.mailbox);
        this.session = newSession();
        this.startedAt = Date.now();
        this.lastId = null;
        this.replies = [];
        this.documentObject = undefined;
        this.documentEpoch = 0;
        this.internalTaskDispatch = false;
        this.requestContext = null;
        this.lastBeat = 0;
        this.clocks = {};       // tick counts per clock source, for diagnostics
        this.clockErrors = {};
        this.beat(true);
    }

    close() {
        try { this.mailbox.write("status.json", { protocol: kProtocol, session: this.session, closed: true, heartbeat: Date.now() }); } catch (_) {}
    }

    beat(force) {
        const now = Date.now();
        if (!force && now - this.lastBeat < kHeartbeatMs) return;
        this.lastBeat = now;
        this.mailbox.write("status.json", {
            protocol: kProtocol, session: this.session, startedAt: this.startedAt, heartbeat: now,
            allowEval: false, clocks: this.clocks, clockErrors: this.clockErrors,
        });
    }

    tick(source) {
        if (this.requestContext) return;
        if (source) this.clocks[source] = (this.clocks[source] || 0) + 1;
        const now = Date.now();
        this.beat(false);
        const req = this.mailbox.read("request.json");
        if (!req || typeof req.id !== "string" || req.id.length > 128) return;
        const cached = this.replies.find(r => r.id === req.id);
        if (cached) {
            const saved = this.mailbox.read("response.json");
            if (!saved || saved.id !== req.id) this.mailbox.write("response.json", cached);
            return;
        }
        this.lastId = req.id;
        let result;
        let code = null;
        const context = this.context();
        const reads = ["ping", "context", "song", "tracks", "trackFormats", "channels", "panner", "splitters", "notes", "plugins", "pluginCatalog", "pluginInspect", "hostProbe", "pluginParams", "pluginChoices", "pluginParameterDetails", "pluginParameterFormat", "pluginParameterDigest", "pluginModels", "inserts", "sends", "meters", "listCommands"];
        const readOnly = reads.indexOf(req.op) >= 0 || (req.op === "command" && req.args && req.args.checkOnly === true);
        this.requestContext = context;
        try {
            if (req.protocol !== kProtocol) {
                code = "protocol_mismatch";
                result = fail("bridge protocol 2 is required");
            } else if (req.expectedSession && req.expectedSession !== this.session) {
                code = "session_changed";
                result = fail("the bridge session changed");
            } else if (req.expectedDocument && req.expectedDocument !== context.document) {
                code = "document_changed";
                result = fail("the active document changed");
            } else if (!readOnly && (!req.expectedSession || !req.expectedDocument)) {
                code = "context_required";
                result = fail("writes require expectedSession and expectedDocument from context");
            } else {
                result = this.handle(req.op, req.args || {});
            }
        } catch (e) {
            result = fail(e && e.message || e); // only reachable for plain-JS errors
        }
        this.requestContext = null;
        const afterContext = this.context();
        if (!isFail(result) && req.op === "insertChainState"
            && (context.open !== true || afterContext.open !== true
                || context.document !== afterContext.document || context.session !== afterContext.session)) {
            const outcome = result && result.native_outcome === "not_applied" ? "not_applied" : "unknown";
            result = { ok: false, outcome: outcome, native_outcome: outcome,
                error: { code: "insert_chain_context_changed", message: "The insert-chain document lease changed before publication" } };
        }
        if (!isFail(result) && req.op === "pluginInspect" && req.args && req.args.include_context === true
            && (!result.fixture_context || !result.fixture_context.after
                || !sameFixtureContext(result.fixture_context.after.context, afterContext)))
            result = fail("the fixture document changed before the inspection receipt");
        const reply = isFail(result)
            ? { protocol: kProtocol, id: req.id, session: this.session, document: afterContext.document, ok: false, error: result.bridgeError, code: code || "host_rejected" }
            : { protocol: kProtocol, id: req.id, session: this.session, document: afterContext.document, ok: true, result: result };
        reply.ms = Date.now() - now;
        this.replies.push(reply);
        if (this.replies.length > 32) this.replies.shift();
        this.mailbox.write("response.json", reply);
    }

    context() {
        const dm = appObject("DocumentManager");
        const doc = dm && has(dm, "activeDocument", "object") ? dm.activeDocument : null;
        if (doc !== this.documentObject) {
            this.documentObject = doc;
            this.documentEpoch++;
        }
        return {
            protocol: kProtocol, session: this.session,
            document: this.session + ":" + this.documentEpoch,
            open: !!doc,
            title: doc && has(doc, "title", "string") ? String(doc.title) : null,
            fileUrl: doc && has(doc, "path", "object") && has(doc.path, "url", "string") ? doc.path.url : null,
        };
    }

    // ---- operations -------------------------------------------------------------
    //
    // Nothing in the device scripts uses `throw`: failures are fail() values.
    // Studio One turns exceptions raised while it is calling into a script (and
    // any TypeError on a host object, caught or not) into a modal "Scripting
    // Error" dialog, and while one is open some edits (mute, solo) silently do
    // not apply. So also: check that a host member exists before calling it.

    handle(op, args) {
        if (args.include_context !== undefined && op !== "pluginInspect")
            return fail("context observations apply only to pluginInspect");
        switch (op) {
            case "ping": return { pong: true, context: this.context(), session: this.session, time: Date.now() };
            case "context": return this.context();
            case "channels": return this.fromComponent(c => c.channels());
            case "panner": return this.fromComponent(c => c.panner(args));
            case "setPanner": return this.fromComponent(c => c.setPanner(args));
            case "splitters": return this.fromComponent(c => c.splitters(args));
            case "setSplitter": return this.fromComponent(c => c.setSplitter(args));
            case "setChannel": return this.fromComponent(c => c.setChannel(args));
            case "command": return this.command(args);
            case "editorCommand": return this.editorCommand(args);
            case "listCommands": return this.listCommands(args);
            case "song": return this.song();
            case "tracks": return this.tracks(args);
            case "trackFormats": return this.trackFormats();
            case "selectTrack": return this.selectTrack(args);
            case "notes": return this.notes(args);
            case "editNotes": return this.editNotes(args);
            case "trackTask": return this.trackTask(args);
            case "plugins": return this.plugins(args);
            case "pluginCatalog": return this.pluginCatalog(args);
            case "addPlugin": return this.addPlugin(args);
            case "metronome": return this.metronome(args);
            case "transport": return this.transport(args);
            case "setTransport": return this.setTransport(args);
            case "markers": return this.markers();
            case "addMarker": return this.addMarker(args);
            case "deleteMarker": return this.deleteMarker(args);
            case "selectEvents": return this.selectEvents(args);
            case "setLoop": return this.setLoop(args);
            case "takes": return this.takes(args);
            case "save": return this.run("File", args.newVersion ? "Save New Version" : "Save");
            case "undo": return this.repeat("Edit", "Undo", args.steps);
            case "redo": return this.repeat("Edit", "Redo", args.steps);
            case "trackState": return this.trackState(args);
            case "editEvents": return this.editEvents(args);
            case "addTrack": return this.addTrack(args);
            case "meters": return this.fromComponent(c => c.meters());
            case "inserts": return this.fromComponent(c => c.inserts(args));
            case "setInsertBypass": return this.fromComponent(c => c.setInsertBypass(args));
            case "sends": return this.fromComponent(c => c.sends(args));
            case "setSend": return this.fromComponent(c => c.setSend(args));
            case "setChannelLabel": return this.fromComponent(c => c.setChannelLabel(args));
            case "setChannelColor": return this.fromComponent(c => c.setChannelColor(args));
            case "setAutomation": return this.fromComponent(c => c.setAutomation(args));
            case "pluginParams": return this.fromComponent(c => c.pluginParams(args));
            case "pluginChoices": return this.fromComponent(c => c.pluginChoices(args));
            case "setPluginParam": return this.fromComponent(c => c.setPluginParam(args));
            case "hostProbe": return this.hostProbe(args);
            case "pluginInspect": return this.pluginInspect(args);
            case "pluginEditorOpen": return this.fromComponent(c => c.pluginEditorOpen(args));
            case "pluginState": return this.fromComponent(c => c.pluginState(args));
            case "insertChainState": return this.fromComponent(c => c.insertChainState(args));
            case "pluginParameterDetails": return this.fromComponent(c => c.pluginParameterDetails(args));
            case "pluginParameterFormat": return this.fromComponent(c => c.pluginParameterFormat(args));
            case "pluginParameterDigest": return this.fromComponent(c => c.pluginParameterDigest(args));
            case "pluginModels": return this.fromComponent(c => c.pluginModels(args));
            case "pluginPresets": return this.fromComponent(c => c.pluginPresets(args));
            case "pluginAction": return this.fromComponent(c => c.pluginAction(args));
            default: return fail("unknown op: " + op);
        }
    }

    // Component methods report failure as { error } (see BridgeComponent.js).
    fromComponent(fn) {
        if (!this.component) return fail("mixer not available (no control-surface component)");
        const r = fn(this.component);
        if (r && !Array.isArray(r) && typeof r.error === "string") return fail(r.error);
        return r;
    }

    pluginInspect(args) {
        if (args.include_context === undefined || args.include_context === false)
            return this.fromComponent(c => c.pluginInspect(args));
        const named = value => typeof value === "string" && value.trim() && value.length <= 512 && !/[\x00-\x1f\x7f-\x9f]/.test(value);
        if (args.include_context !== true || ["instrument", "note_effect"].indexOf(args.kind) < 0
            || !named(args.track) || !named(args.plugin) || ["dsp", "controller"].indexOf(args.source) < 0
            || typeof args.expected_instance !== "string"
            || !/^[0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{12}$/.test(args.expected_instance)
            || args.expected_instance === "00000000-0000-0000-0000-000000000000"
            || args.include_nodes === true || args.node_scope !== undefined
            || (args.kind === "instrument" && args.slot !== undefined && args.slot !== 0)
            || (args.kind === "note_effect" && args.slot !== undefined
                && (typeof args.slot !== "number" || Math.floor(args.slot) !== args.slot || args.slot < 0 || args.slot >= 16)))
            return fail("context observations require a bound Music target, explicit provider and no node inventory");
        const before = this.fixtureContext(args);
        if (isFail(before)) return before;
        const result = this.fromComponent(c => c.pluginInspect(args));
        if (isFail(result)) return result;
        const native = result && result.native;
        if (!result || (result.error !== undefined && result.error !== null)
            || (result.native_error !== undefined && result.native_error !== null)
            || !native || native.ok !== true || (native.error !== undefined && native.error !== null) || native.source !== args.source
            || !native.instance || native.instance.object_uid !== args.expected_instance
            || typeof native.instance.class_uid !== "string" || !native.instance.class_uid
            || (native.outcome !== undefined && native.outcome !== "observed")
            || (native.native_outcome !== undefined && native.native_outcome !== "observed"))
            return fail("context inspection requires a known matching native observation");
        const after = this.fixtureContext(args);
        if (isFail(after)) return after;
        if (JSON.stringify(before) !== JSON.stringify(after)) return fail("the Music fixture changed during native inspection");
        result.fixture_context = { version: 1, atomic: false, scope: "music_fixture_before_after",
            target: { kind: args.kind, track: args.track, plugin: args.plugin, slot: args.slot === undefined ? null : args.slot },
            before: before, after: after };
        return result;
    }

    fixtureContext(args) {
        const tp = this.transportPanel();
        if (!tp) return fail("fixture transport is unavailable");
        const switches = {};
        for (const name of ["start", "record"]) {
            const p = tp.findParameter(name);
            if (!p) return fail("fixture transport parameter is unavailable");
            const value = p.value;
            if (value !== true && value !== false && value !== 0 && value !== 1)
                return fail("fixture transport switch is malformed");
            switches[name] = value === true || value === 1;
        }
        const list = trackList();
        const integer = value => typeof value === "number" && isFinite(value) && Math.floor(value) === value && value >= 0 && value <= 1000000;
        if (!list || !has(list, "getSelectedTrack", "function"))
            return fail("complete fixture track selection is unavailable");
        const trackCount = list.numTracks, selectedCount = list.numSelectedTracks;
        if (!integer(trackCount) || !integer(selectedCount)) return fail("complete fixture track selection is unavailable");
        const tracks = [], selected = [];
        for (let i = 0; i < trackCount; i++) {
            const track = list.getTrack(i);
            if (!track) return fail("fixture track enumeration is incomplete");
            if (tracks.indexOf(track) < 0) tracks.push(track);
        }
        const matches = tracks.filter(track => track.name === args.track);
        if (matches.length !== 1 || matches[0].mediaType !== "Music")
            return fail("fixture requires one exact Music track");
        const track = matches[0];
        if (!has(track, "createIterator", "function")) return fail("fixture event enumeration is unavailable");
        const iterator = track.createIterator();
        if (!iterator || !has(iterator, "next", "function")) return fail("fixture event enumeration is incomplete");
        if (iterator.next() !== null) return fail("fixture events are nonempty or their exhaustion is unknown");
        for (let i = 0; i < selectedCount; i++) {
            const current = list.getSelectedTrack(i);
            if (!current || tracks.indexOf(current) < 0) return fail("fixture selected-track enumeration is incomplete");
            if (selected.indexOf(current) < 0) selected.push(current);
        }
        if (list.numTracks !== trackCount || list.numSelectedTracks !== selectedCount)
            return fail("fixture track counts changed during observation");
        if (selected.length !== 1 || selected[0] !== track) return fail("fixture Music track is not exclusively selected");
        const focused = this.fromComponent(c => typeof c.fixtureTrackState === "function"
            ? c.fixtureTrackState() : { error: "strict fixture track state is unavailable" });
        if (isFail(focused)) return focused;
        if (!focused || focused.label !== args.track) return fail("focused fixture identity changed");
        const context = this.context();
        if (!sameFixtureContext(context, this.requestContext)) return fail("fixture document changed during observation");
        return { context: { session: context.session, document: context.document, open: true, title: context.title },
            song: { title: context.title, transport: { playing: switches.start, recording: switches.record } },
            target_track: { exact_match_count: 1, name: args.track, mediaType: "Music", eventCount: 0, events: [], events_complete: true },
            selection: { complete: true, tracks: [{ name: args.track, mediaType: "Music" }] },
            focused_track: focused };
    }

    // An edit command sent to the song's arrangement editor itself. Edit/Copy and
    // Edit/Paste through Host.GUI.Commands go to whichever view has the keyboard
    // focus: after a song was reopened, Paste "ran" and pasted nothing (5.5.2).
    // The editor object takes them whatever is focused.
    editorCommand(args) {
        if (!args.category || !args.name) return fail("category and name are required");
        if (args.checkOnly) return this.command(args);
        if (!args.checkOnly && !this.internalTaskDispatch && (args.name === "Fender MCP Edit" || args.name === "Fender MCP Track Edit")) return fail("internal EditTask commands require a fresh bridge lease");
        const ed = docObject("Editor");
        if (!ed || !has(ed, "interpretCommand", "function")) return this.command(args);
        return { executed: !!ed.interpretCommand(String(args.category), String(args.name)), via: "editor" };
    }

    // checkOnly asks whether the command is currently enabled without running it
    // (the same query Studio One makes to grey out menu items).
    command(args) {
        if (!args.category || !args.name) return fail("category and name are required");
        if (!args.checkOnly && !this.internalTaskDispatch && (args.name === "Fender MCP Edit" || args.name === "Fender MCP Track Edit")) return fail("internal EditTask commands require a fresh bridge lease");
        const cmds = Host.GUI.Commands;
        if (!cmds.findCommand(String(args.category), String(args.name)))
            return fail("unknown command: " + args.category + "/" + args.name + " (see listCommands)");
        if (args.checkOnly) return { enabled: !!cmds.interpretCommand(args.category, args.name, true) };
        let attributes = null;
        if (args.args) {
            attributes = Host.Attributes();
            if (!attributes || !has(attributes, "setAttribute", "function")) return fail("command attributes are not available");
            for (const key of Object.keys(args.args)) attributes.setAttribute(key, args.args[key]);
        }
        const ok = attributes
            ? cmds.interpretCommand(args.category, args.name, false, attributes)
            : cmds.interpretCommand(args.category, args.name);
        return { executed: !!ok };
    }

    listCommands(args) {
        const it = Host.GUI.Commands.newCommandIterator();
        const out = [];
        const filter = args.filter ? String(args.filter).toLowerCase() : null;
        while (it && !it.done()) {
            const c = it.next();
            if (!c) break;
            const entry = { category: String(c.category), name: String(c.name) };
            if (filter && (entry.category + " " + entry.name).toLowerCase().indexOf(filter) < 0) continue;
            if (args.withState) entry.enabled = !!Host.GUI.Commands.interpretCommand(entry.category, entry.name, true);
            out.push(entry);
        }
        return out;
    }

    // ---- song, tracks, transport (document object model) ----------------------

    transportPanel() {
        const tp = docObject("Environment/TransportPanel");
        return tp && has(tp, "findParameter", "function") ? tp : null;
    }

    transportState() {
        const tp = this.transportPanel();
        if (!tp) return fail("no song open");
        const param = n => tp.findParameter(n) || null;
        const val = n => { const p = param(n); return p ? p.value : null; };
        const time = n => { const p = param(n); return p ? { seconds: p.value, display: String(p.string) } : null; };
        return {
            playing: !!val("start"), recording: !!val("record"), loop: !!val("loop"),
            precount: !!val("precount"), preroll: !!val("preroll"),
            tempo: val("tempo"),
            position: time("primaryTime"),
            timeFormat: param("primaryTimeFormat") ? String(param("primaryTimeFormat").string) : null,
            loopRange: { start: time("loopStart"), end: time("loopEnd") },
            // Auto Punch records between the loop locators. "punchIn" is punch-in;
            // "punch" is on whenever any autopunch is (5.5.2), so punch-out alone
            // cannot be told apart once punch-in is on.
            autopunch: param("punchIn") ? { punchIn: !!val("punchIn"), any: !!val("punch") } : null,
        };
    }

    // Track edits through the "Fender MCP Track Edit" task (device/EditTasks): routing,
    // folders, event names. Request in track-edit-request.json, result in
    // track-edit-result.json; the task needs no selection.
    trackTask(args) {
        if (!Array.isArray(args.ops) || !args.ops.length) return fail("ops: one or more track edit operations");
        if (!Host.GUI.Commands.findCommand("Track", "Fender MCP Track Edit")) return fail("the MCP Track Edit task is not installed: reinstall the device, then restart Studio One");
        const id = newSession();
        this.mailbox.write("track-edit-result.json", { id: null });
        const armed = this.armTask("track", "track-edit-request.json", id, args.ops);
        if (isFail(armed)) return armed;
        const r = this.runArmedTask("Track", "Fender MCP Track Edit", "track-edit-request.json");
        if (isFail(r)) return r;
        const res = this.mailbox.read("track-edit-result.json");
        if (!res || res.id !== id) return fail("the track edit task did not report back");
        if (res.error) return fail(res.error);
        return { results: res.results };
    }

    armTask(kind, filename, id, ops) {
        const context = this.context();
        if (!this.requestContext || context.document !== this.requestContext.document || !context.open) return fail("the task document changed before dispatch");
        const lease = { id: id, kind: kind, session: this.session, document: context.document, fileUrl: context.fileUrl, title: context.title, expiresAt: Date.now() + 10000 };
        if (!this.mailbox.write(filename, { id: id, ops: ops, expectedSession: lease.session, expectedDocument: lease.document })) return fail("cannot write the task request");
        if (!this.mailbox.write("task-lease.json", lease)) {
            this.mailbox.write(filename, { id: null, ops: [] });
            return fail("cannot arm the task lease");
        }
        return { armed: true };
    }

    runArmedTask(category, name, filename) {
        this.internalTaskDispatch = true;
        try { return this.run(category, name); }
        finally {
            this.internalTaskDispatch = false;
            this.mailbox.write(filename, { id: null, ops: [] });
            this.mailbox.write("task-lease.json", { id: null });
        }
    }

    song() {
        const transport = this.transportState();
        if (isFail(transport)) return transport;
        const dm = appObject("DocumentManager");
        const doc = dm && has(dm, "activeDocument", "object") ? dm.activeDocument : null;
        const list = trackList();
        const path = doc && has(doc, "path", "object") && has(doc.path, "url", "string") ? doc.path.url : null;
        return {
            title: doc ? String(doc.title) : null,
            fileUrl: path,
            transport: transport,
            trackCount: list ? uniqueTracks(list).length : null,
            selectedTracks: list ? selectedTracks(list).map(t => String(t.name)) : [],
        };
    }

    tracks(args) {
        const list = trackList();
        if (!list) return fail("no song open");
        const withEvents = args.events !== false;
        const maxEvents = args.maxEvents === undefined ? 50 : args.maxEvents;
        const filter = args.name ? String(args.name).toLowerCase() : null;
        const selected = selectedTracks(list);
        const out = [];
        for (const t of uniqueTracks(list)) {
            const name = String(t.name);
            if (filter && name.toLowerCase().indexOf(filter) < 0) continue;
            const entry = {
                index: has(t, "trackIndex", "number") ? t.trackIndex : out.length,
                name: name,
                mediaType: has(t, "mediaType", "string") ? t.mediaType : null,
                color: has(t, "color", "number") ? "#" + ("000000" + (t.color & 0xffffff).toString(16)).slice(-6) : null,
                channel: t.channel && has(t.channel, "label", "string") ? t.channel.label : null,
                takes: t.layers && has(t.layers, "count", "number") ? t.layers.count : null,
                selected: selected.indexOf(t) >= 0,
            };
            if (withEvents) {
                const events = trackEvents(t);
                entry.eventCount = events.length;
                entry.events = events.slice(0, maxEvents);
            }
            out.push(entry);
        }
        return out;
    }

    // Metronome settings: the document's Environment/Metronome parameters
    // (clickOn, precount, preroll, bars = precount length 1..16), read and set.
    metronome(args) {
        const m = docObject("Environment/Metronome");
        if (!m || !has(m, "findParameter", "function")) return fail("metronome not available (no song open?)");
        const fields = { click: "clickOn", precount: "precount", preroll: "preroll", precountBars: "bars" };
        for (const key in fields) {
            if (args[key] === undefined) continue;
            const p = m.findParameter(fields[key]);
            if (!p || !has(p, "setValue", "function")) return fail(key + " not available");
            const v = typeof args[key] === "boolean" ? (args[key] ? 1 : 0) : args[key];
            if (typeof v !== "number" || v < p.min || v > p.max) return fail(key + " must be from " + p.min + " to " + p.max);
            p.setValue(v, true);
        }
        const out = {};
        for (const key in fields) {
            const p = m.findParameter(fields[key]);
            out[key] = p ? (key === "precountBars" ? p.value : !!p.value) : null;
        }
        return out;
    }

    // Notes of an instrument track's parts. A part's createSequenceIterator()
    // walks its notes with done()/next() (its createIterator() gives nothing):
    // pitch, velocity 0..1, startTime/endTime in song seconds, startTime.musical
    // in quarter-note beats. Read-only: editing notes needs an edit task.
    notes(args) {
        const list = trackList();
        if (!list) return fail("no song open");
        const matches = uniqueTracks(list).filter(t => String(t.name) === String(args.track));
        if (matches.length !== 1) return fail(matches.length ? "track name is ambiguous: " + args.track : "no track named " + args.track);
        const t = matches[0];
        const max = args.maxNotes === undefined ? 500 : args.maxNotes;
        const parts = [];
        let total = 0;
        let skipped = 0;
        const it = has(t, "createIterator", "function") ? t.createIterator() : null;
        let ev;
        while (it && (ev = it.next())) {
            if (!has(ev, "createSequenceIterator", "function")) continue;
            const part = { name: has(ev, "name", "string") ? ev.name : "", start: seconds(ev.startTime), end: seconds(ev.endTime), muted: !!ev.isMuted, notes: [], noteCount: 0 };
            const ni = ev.createSequenceIterator();
            while (ni && has(ni, "done", "function") && !ni.done()) {
                const n = ni.next();
                if (!n) break;
                part.noteCount++;
                if (total >= max) { skipped++; continue; }
                total++;
                const start = seconds(n.startTime), end = seconds(n.endTime);
                part.notes.push({
                    pitch: has(n, "pitch", "number") ? n.pitch : null,
                    velocity: has(n, "velocity", "number") ? Math.round(n.velocity * 127) : null,
                    start: start, end: end,
                    length: start !== null && end !== null ? Math.round((end - start) * 1000) / 1000 : null,
                    beat: n.startTime && has(n.startTime, "musical", "number") ? Math.round(n.startTime.musical * 1000) / 1000 : null,
                    muted: !!n.isMuted,
                });
            }
            parts.push(part);
        }
        const mediaType = has(t, "mediaType", "string") ? t.mediaType : null;
        return { track: String(t.name), mediaType: mediaType, parts: parts, truncated: skipped > 0 };
    }

    // ---- plug-ins by name ------------------------------------------------------
    //
    // Studio One's plug-in picker (Host:PlugInMenuParam, as its Add Tracks script
    // uses) lists installed effects: set a category, step through the values, read
    // each name from .string, and getSelectedClass() gives the class id. A channel's
    // Inserts folder takes that id in insertDeviceClass (checked on 5.5.2: it adds
    // the plug-in). Added this way it is not on the undo stack, and scripts have no
    // way to remove a plug-in, so removing one is by hand.

    // category: "AudioEffect" (default) or "AudioSynth" (instruments).
    pluginMenu(category) {
        if (!Host.Classes || !has(Host.Classes, "createInstance", "function")) return null;
        const menu = Host.Classes.createInstance("Host:PlugInMenuParam");
        if (!menu || !has(menu, "setCategory", "function") || !has(menu, "setValue", "function") || !has(menu, "getSelectedClass", "function")) return null;
        menu.setCategory(category || "AudioEffect");
        return menu;
    }

    plugins(args) {
        const menu = this.pluginMenu(args.kind === "instrument" ? "AudioSynth" : (args.kind === "note_effect" ? "MusicEffect" : "AudioEffect"));
        if (!menu) return fail("the plug-in list is not available");
        const names = [];
        for (let i = menu.min; i <= menu.max; i++) {
            menu.setValue(i, true);
            const n = String(menu.string);
            if (names.indexOf(n) < 0) names.push(n);
        }
        const f = args.filter ? String(args.filter).toLowerCase() : null;
        return { plugins: f ? names.filter(n => n.toLowerCase().indexOf(f) >= 0) : names };
    }

    pluginCatalog(args) {
        const categories = ["AudioEffect", "AudioSynth", "MusicEffect", "AudioPanner", "AudioMixEffect"];
        const category = String(args.category || "AudioEffect");
        if (categories.indexOf(category) < 0) return fail("unsupported plugin category");
        const menu = this.pluginMenu(category);
        if (!menu) return fail("plugin class menu is not available");
        const plugins = [];
        const seen = [];
        const upper = Math.min(menu.max, menu.min + 8191);
        for (let i = menu.min; i <= upper; i++) {
            menu.setValue(i, true);
            const cls = menu.getSelectedClass();
            if (!cls) continue;
            const id = String(cls);
            if (seen.indexOf(id) >= 0) continue;
            seen.push(id);
            plugins.push({ name: String(menu.string), classId: id, category: category });
        }
        return { category: category, plugins: plugins, complete: upper === menu.max, source: "live" };
    }

    mixerChannel(label) {
        const con = docObject("Environment/MixerConsole");
        if (!con || !has(con, "getChannelList", "function")) return fail("the mixer console is not available");
        const list = con.getChannelList(1);
        if (!list || !has(list, "getChannel", "function")) return fail("the mixer console is not available");
        // Studio Pro 8 also lists each instrument track's note channel (mediaType
        // "Music") under the same label as its audio channel; the audio one is meant.
        let hits = [];
        for (let i = 0; i < list.numChannels; i++) {
            const c = list.getChannel(i);
            if (c && c.label === label) hits.push(c);
        }
        const audio = hits.filter(c => c.mediaType === "Audio");
        if (hits.length > 1 && audio.length === 1) hits = audio;
        if (hits.length !== 1) return fail(hits.length ? "channel name is ambiguous: " + label : "no channel named " + label);
        return hits[0];
    }

    addPlugin(args) {
        const menu = this.pluginMenu();
        if (!menu) return fail("the plug-in list is not available");
        const want = String(args.plugin || "");
        let exact = -1, loose = -1;
        for (let i = menu.min; i <= menu.max; i++) {
            menu.setValue(i, true);
            const n = String(menu.string);
            if (n === want && exact < 0) exact = i;
            if (n.toLowerCase() === want.toLowerCase() && loose < 0) loose = i;
        }
        const at = exact >= 0 ? exact : loose;
        if (at < 0) return fail("no plug-in named " + want + " (live_plugins lists them)");
        menu.setValue(at, true);
        const name = String(menu.string);
        const cls = menu.getSelectedClass();
        const ch = this.mixerChannel(args.channel);
        if (isFail(ch)) return ch;
        const ins = has(ch, "find", "function") ? ch.find("Inserts") : null;
        if (!ins || !has(ins, "insertDeviceClass", "function")) return fail(args.channel + " has no insert rack to add to");
        if (!ins.insertDeviceClass(cls)) return fail("Studio One did not add " + name);
        const rack = this.fromComponent(c => c.inserts({ channel: args.channel }));
        return { channel: args.channel, added: name, inserts: isFail(rack) || !rack.length ? null : rack[0].inserts };
    }

    // Note edits through the "Fender MCP Edit" task (device/EditTasks): the request goes
    // to edit-request.json, the track's parts are selected, the task's command
    // runs and writes edit-result.json. The selection is restored.
    editNotes(args) {
        if (!Array.isArray(args.ops) || !args.ops.length) return fail("ops: one or more edit operations");
        if (!Host.GUI.Commands.findCommand("Musical Functions", "Fender MCP Edit")) return fail("the MCP Edit task is not installed: reinstall the device, then restart Studio One");
        const initial = this.notes({ track: args.track, maxNotes: 1 });
        if (isFail(initial)) return initial;
        if (initial.mediaType !== "Music") return fail("note edits require an instrument track");
        if (initial.parts.length > 128) return fail("note edits support at most 128 parts in one request");
        const events = this.trackTask({ ops: [{ op: "events", track: args.track }] });
        if (isFail(events)) return events;
        const listed = events.results && events.results[0];
        if (!listed || listed.error || !Array.isArray(listed.events)) return fail(listed && listed.error || "could not enumerate the target parts");
        const ordered = initial.parts.slice().sort((a, b) => a.start - b.start);
        if (listed.events.length !== ordered.length) return fail("the target part list changed");
        if (args.ops.some(op => op.op === "add") && ordered.some(part => !part.noteCount)) return fail("use trackTask addNotes with an explicit part to add notes to empty parts");
        const applied = [], errors = [], results = [];
        let notesBefore = 0;
        const r = this.withTrack(args.track, () => {
            try {
                for (let i = 0; i < ordered.length; i++) {
                    const part = ordered[i], expected = listed.events[i];
                    if (Math.abs(part.start - expected.start) > 0.001 || Math.abs(part.end - expected.end) > 0.001 || part.name !== expected.name) { errors.push("the target part ordering changed"); break; }
                    if (!part.noteCount) { results.push({ part: i + 1, skipped: "empty part" }); continue; }
                    const selected = this.trackTask({ ops: [{ op: "selectEvent", track: args.track, event: i + 1 }] });
                    const selection = !isFail(selected) && selected.results && selected.results[0];
                    const actual = selection && selection.selected;
                    if (!selection || selection.error || !actual) { errors.push(isFail(selected) ? selected.bridgeError : "could not select the target part"); break; }
                    if (Math.abs(actual.start - expected.start) > 0.001 || Math.abs(actual.end - expected.end) > 0.001 || actual.name !== expected.name) { errors.push("the selected part changed before edit dispatch"); break; }
                    const id = newSession();
                    this.mailbox.write("edit-result.json", { id: null });
                    const armed = this.armTask("notes", "edit-request.json", id, args.ops);
                    if (isFail(armed)) { errors.push(armed.bridgeError); break; }
                    const done = this.runArmedTask("Musical Functions", "Fender MCP Edit", "edit-request.json");
                    if (isFail(done)) { errors.push(done.bridgeError); break; }
                    const res = this.mailbox.read("edit-result.json");
                    if (!res || res.id !== id || res.error) { errors.push(res && res.error || "the edit task did not report back; this part's outcome is unknown"); break; }
                    notesBefore += res.notesBefore || 0;
                    for (const item of res.applied || []) applied.push(item);
                    for (const error of res.errors || []) errors.push(error);
                    results.push({ part: i + 1, applied: res.applied, errors: res.errors });
                    if (errors.length) break;
                }
                return { completed: !errors.length };
            } finally { this.command({ category: "Edit", name: "Deselect All" }); }
        });
        if (isFail(r)) return r;
        const after = this.notes({ track: args.track });
        return { track: args.track, applied: applied, errors: errors, results: results, notesBefore: notesBefore, parts: isFail(after) ? null : after.parts };
    }

    selectTrack(args) {
        const list = trackList();
        if (!list) return fail("no song open");
        if (!has(list, "selectTrack", "function")) return fail("track selection not available");
        let matches = uniqueTracks(list).filter(t => String(t.name) === String(args.name));
        // An audio stem and its MIDI render often share a name: mediaType ("Audio" or
        // "Music") picks one of them.
        if (matches.length > 1 && args.mediaType) matches = matches.filter(t => has(t, "mediaType", "string") && t.mediaType === args.mediaType);
        if (matches.length !== 1) return fail(matches.length ? "track name is ambiguous: " + args.name + " (pass kind: audio or instrument)" : "no track named " + args.name);
        if (args.exclusive !== false && has(list, "unselectAll", "function")) list.unselectAll();
        list.selectTrack(matches[0], true, false);
        return { selected: selectedTracks(list).map(t => String(t.name)) };
    }

    // Transport buttons map onto Studio One commands (see listCommands "Transport").
    transport(args) {
        const actions = {
            play: "Start", stop: "Stop", record: "Record", togglePlay: "Toggle Start",
            returnToZero: "Return to Zero", rewind: "Rewind Bar", forward: "Forward Bar",
            loopStart: "Goto Loop Start", loopEnd: "Goto Loop End", toggleLoop: "Toggle Loop",
            toggleClick: "Click", togglePrecount: "Precount", togglePreroll: "Preroll",
            locateSelection: "Locate Selection",
        };
        const name = actions[args.action];
        if (!name) return fail("action must be one of " + Object.keys(actions).join(", "));
        const r = this.command({ category: "Transport", name: name });
        if (isFail(r)) return r;
        const state = this.transportState();
        return { action: args.action, executed: r.executed, transport: isFail(state) ? null : state };
    }

    // Tempo (bpm) and position (seconds) are transport-panel parameters; loop,
    // precount and preroll are toggled through their commands when they differ.
    setTransport(args) {
        const tp = this.transportPanel();
        if (!tp) return fail("no song open");
        if (args.tempo !== undefined) {
            const p = tp.findParameter("tempo");
            if (!p || !has(p, "setValue", "function")) return fail("tempo parameter not available");
            if (typeof args.tempo !== "number" || args.tempo < p.min || args.tempo > p.max) return fail("tempo must be a number from " + p.min + " to " + p.max);
            p.setValue(args.tempo, true);
        }
        if (args.positionSeconds !== undefined) {
            const p = tp.findParameter("primaryTime");
            if (!p || !has(p, "setValue", "function")) return fail("position parameter not available");
            if (typeof args.positionSeconds !== "number" || args.positionSeconds < 0) return fail("positionSeconds must be a number >= 0");
            p.setValue(args.positionSeconds, true);
        }
        if (args.positionBars !== undefined) {
            const r = setTime(tp.findParameter("primaryTime"), args.positionBars);
            if (isFail(r)) return r;
        }
        const toggles = { loop: "Toggle Loop", precount: "Precount", preroll: "Preroll" };
        for (const key in toggles) {
            if (args[key] === undefined) continue;
            const state = this.transportState();
            if (isFail(state)) return state;
            if (!!args[key] !== state[key]) {
                const r = this.command({ category: "Transport", name: toggles[key] });
                if (isFail(r)) return r;
            }
        }
        return this.transportState();
    }

    // ---- markers --------------------------------------------------------------
    //
    // The marker track is not reachable from scripts, but its commands are:
    // "Recall Marker N" is enabled for each existing marker and moves the playhead
    // there. So positions are read by recalling each marker in turn and putting
    // the playhead back. Names are not exposed (the MCP server adds them from the
    // saved .song). Only markers 1-20 have recall commands.

    markerPositions() {
        const tp = this.transportPanel();
        if (!tp) return fail("no song open");
        const pt = tp.findParameter("primaryTime");
        if (!pt || !has(pt, "setValue", "function")) return fail("position parameter not available");
        const state = this.transportState();
        if (state.playing) return fail("stop playback first: reading markers moves the playhead");
        const C = Host.GUI.Commands;
        const home = pt.value;
        const out = [];
        for (let n = 1; n <= 20; n++) {
            const name = "Recall Marker " + n;
            if (!C.findCommand("Marker", name)) break;
            if (!C.interpretCommand("Marker", name, true)) continue;
            C.interpretCommand("Marker", name);
            out.push({ number: n, seconds: pt.value, display: String(pt.string) });
        }
        pt.setValue(home, true);
        return out;
    }

    markers() {
        const list = this.markerPositions();
        return isFail(list) ? list : { markers: list };
    }

    addMarker(args) {
        const tp = this.transportPanel();
        if (!tp) return fail("no song open");
        const pt = tp.findParameter("primaryTime");
        const home = pt.value;
        const at = args.seconds === undefined ? home : args.seconds;
        if (typeof at !== "number" || at < 0) return fail("seconds must be a number >= 0");
        pt.setValue(at, true);
        const r = this.command({ category: "Marker", name: "Insert" });
        pt.setValue(home, true);
        if (isFail(r)) return r;
        const list = this.markerPositions();
        return isFail(list) ? list : { added: r.executed, seconds: at, markers: list };
    }

    deleteMarker(args) {
        const list = this.markerPositions();
        if (isFail(list)) return list;
        const target = args.number !== undefined
            ? list.find(m => m.number === args.number)
            : list.find(m => typeof args.seconds === "number" && Math.abs(m.seconds - args.seconds) < 0.001);
        if (!target) return fail("no marker " + (args.number !== undefined ? "number " + args.number : "at " + args.seconds + "s"));
        const pt = this.transportPanel().findParameter("primaryTime");
        const home = pt.value;
        this.command({ category: "Marker", name: "Recall Marker " + target.number });
        const r = this.command({ category: "Marker", name: "Delete" });
        pt.setValue(home, true);
        if (isFail(r)) return r;
        const after = this.markerPositions();
        return isFail(after) ? after : { deleted: target, markers: after };
    }

    // ---- event selection ------------------------------------------------------
    //
    // Studio One's event commands (Event/Mute Events, Edit/Split at Cursor,
    // Track/Activate Next Layer...) act on the selection. This selects every
    // event on the named tracks (or on all tracks) so live_command can follow.

    selectEvents(args) {
        const list = trackList();
        if (!list) return fail("no song open");
        const C = Host.GUI.Commands;
        this.command({ category: "Edit", name: "Deselect All" });
        if (args.none) return { selectedTracks: selectedTracks(list).map(t => String(t.name)), events: "none" };
        if (args.all) {
            const r = this.command({ category: "Edit", name: "Select All" });
            if (isFail(r)) return r;
        } else {
            const names = Array.isArray(args.tracks) ? args.tracks : [args.track];
            if (!names.length || names.some(n => typeof n !== "string")) return fail("track (or tracks, or all) is required");
            for (let i = 0; i < names.length; i++) {
                const r = this.selectTrack({ name: names[i], exclusive: i === 0 });
                if (isFail(r)) return r;
            }
            const r = this.command({ category: "Edit", name: "Select All on Tracks" });
            if (isFail(r)) return r;
        }
        return {
            selectedTracks: selectedTracks(list).map(t => String(t.name)),
            eventCommandsEnabled: !!C.interpretCommand("Event", "Mute Events", true),
        };
    }

    // ---- more editing tools ---------------------------------------------------

    run(category, name) {
        const r = this.command({ category: category, name: name });
        if (isFail(r)) return r;
        if (!r.executed) return fail(category + "/" + name + " is not available right now");
        return r;
    }

    repeat(category, name, steps) {
        const n = steps === undefined ? 1 : steps;
        if (typeof n !== "number" || n < 1 || n > 50) return fail("steps must be 1 to 50");
        let done = 0;
        for (let i = 0; i < n; i++) {
            const r = this.command({ category: category, name: name });
            if (isFail(r)) return r;
            if (!r.executed) break;
            done++;
        }
        return { done: done };
    }

    // start/end: seconds (number) or a bar position string like "9.1.1.0".
    setLoop(args) {
        const tp = this.transportPanel();
        if (!tp) return fail("no song open");
        if (args.start !== undefined) { const r = setTime(tp.findParameter("loopStart"), args.start); if (isFail(r)) return r; }
        if (args.end !== undefined) { const r = setTime(tp.findParameter("loopEnd"), args.end); if (isFail(r)) return r; }
        if (args.enable !== undefined) {
            const state = this.transportState();
            if (!!args.enable !== state.loop) { const r = this.run("Transport", "Toggle Loop"); if (isFail(r)) return r; }
        }
        return this.transportState();
    }

    // Run fn with only the named track selected, then put the selection back.
    withTrack(name, fn) {
        const list = trackList();
        if (!list) return fail("no song open");
        const before = selectedTracks(list);
        const sel = this.selectTrack({ name: name });
        if (isFail(sel)) return sel;
        try { return fn(list); }
        finally {
            if (has(list, "unselectAll", "function")) list.unselectAll();
            for (const t of before) list.selectTrack(t, true, false);
        }
    }

    trackInfo(name) {
        const found = this.tracks({ name: name, maxEvents: 20 });
        if (isFail(found)) return found;
        return found.find(t => t.name === name) || null;
    }

    // Takes (layers). Scripts see only how many there are, not which is active, so
    // goto walks: "previous" count-1 times reaches take 1 (takes do not wrap), then
    // "next" take-1 times. add (new empty take, made active) and duplicate (copy of
    // the active take) are one undo step each, no dialog (5.5.2).
    takes(args) {
        const actions = { list: null, next: "Activate Next Layer", previous: "Activate Previous Layer", unpack: "Unpack Layers to Tracks", goto: null, add: "Add Layer", duplicate: "Duplicate Layer", retrospective: "Recall Retrospective Recording" };
        const action = args.action || "list";
        if (!(action in actions)) return fail("action must be one of " + Object.keys(actions).join(", "));
        if (action === "goto") {
            const info = this.trackInfo(args.track);
            if (!info) return fail("no track named " + args.track);
            const count = info.takes || 1;
            if (typeof args.take !== "number" || args.take < 1 || args.take > count) return fail("take must be 1 to " + count);
            const r = this.withTrack(args.track, () => {
                for (let i = 1; i < count; i++) {
                    const previous = this.command({ category: "Track", name: actions.previous });
                    if (isFail(previous)) return previous;
                    if (!previous.executed) break;
                }
                for (let i = 1; i < args.take; i++) {
                    const next = this.command({ category: "Track", name: actions.next });
                    if (isFail(next)) return next;
                    if (!next.executed) return fail("take navigation was not accepted; the current take is unknown");
                }
                return { ok: true };
            });
            if (isFail(r)) return r;
        } else if (action !== "list") {
            const r = this.withTrack(args.track, () => this.run("Track", actions[action]));
            if (isFail(r)) return r;
        } else if (!this.trackInfo(args.track)) {
            return fail("no track named " + args.track);
        }
        const t = this.trackInfo(args.track);
        const out = { track: args.track, action: action, takes: t ? t.takes : null, activeEvents: t ? t.events.map(e => e.name) : [] };
        if (action === "goto") { out.requestedTake = args.take; out.active = null; out.verified = false; }
        return out;
    }

    trackState(args) {
        const actions = { arm: "Arm", monitor: "Monitor", mute: "Mute", solo: "Solo", hide: "Hide", duplicate: "Duplicate", showAll: null };
        if (!(args.action in actions)) return fail("action must be one of " + Object.keys(actions).join(", "));
        if (args.action === "showAll") return this.run("Edit", "Show All Tracks");
        const r = this.withTrack(args.track, () => this.run("Track", actions[args.action]));
        if (isFail(r)) return r;
        const ch = this.component && has(this.component, "channels", "function") ? this.component.channels() : null;
        const t = this.trackInfo(args.track);
        const channel = Array.isArray(ch) && t ? ch.find(c => c.label === t.channel) || null : null;
        return { track: args.track, action: args.action, channel: channel };
    }

    // Selection-based clip edits on one track; the playhead is restored.
    editEvents(args) {
        const actions = {
            mute: ["Event", "Mute Events"], unmute: ["Event", "Unmute Events"], toggleMute: ["Event", "Toggle Mute"],
            quantize: ["Event", "Quantize"], transposeUp: ["Event", "Transpose Events Up"], transposeDown: ["Event", "Transpose Events Down"],
            split: ["Edit", "Split at Cursor"], trimStart: ["Event", "Trim Start to Cursor"], trimEnd: ["Event", "Trim End to Cursor"],
            merge: ["Event", "Merge Events"], delete: ["Edit", "Delete"],
        };
        const cmd = actions[args.action];
        if (!cmd) return fail("action must be one of " + Object.keys(actions).join(", "));
        const atCursor = ["split", "trimStart", "trimEnd"].indexOf(args.action) >= 0;
        if (atCursor && args.at === undefined) return fail(args.action + " needs at (seconds or bars)");
        const tp = this.transportPanel();
        if (!tp) return fail("no song open");
        const pt = tp.findParameter("primaryTime");
        const home = pt.value;
        if (atCursor) { const r = setTime(pt, args.at); if (isFail(r)) return r; }
        const r = this.withTrack(args.track, () => {
            const sel = this.command({ category: "Edit", name: "Select All on Tracks" });
            if (isFail(sel)) return sel;
            const done = this.run(cmd[0], cmd[1]);
            this.command({ category: "Edit", name: "Deselect All" });
            return done;
        });
        pt.setValue(home, true);
        if (isFail(r)) return r;
        const t = this.trackInfo(args.track);
        return { track: args.track, action: args.action, events: t ? t.events : [] };
    }

    trackFormats() {
        const formats = Host.Engine && Host.Engine.TrackFormats;
        if (!formats || !has(formats, "at", "function") || typeof formats.count !== "number" || !isFinite(formats.count)
            || formats.count < 0 || Math.floor(formats.count) !== formats.count) return fail("the native track format list is unavailable");
        const out = [];
        for (let i = 0; i < Math.min(formats.count, 128); i++) {
            const format = formats.at(i);
            if (!format || format.mediaType !== "Audio") continue;
            const speaker = format.formatData;
            if (!speaker || typeof speaker.name !== "string" || typeof speaker.type !== "number") return fail("native audio format metadata is incomplete");
            out.push({ format: speaker.name, speakerType: speaker.type, mediaType: "Audio" });
        }
        return { formats: out, truncated: formats.count > 128, scope: "audio_track_formats" };
    }

    addTrack(args) {
        const types = {
            audioMono: "Add Audio Track (mono)", audioStereo: "Add Audio Track (stereo)",
            instrument: "Add Instrument Track", folder: "Add Folder Track", automation: "Add Automation Track",
        };
        const name = types[args.type || "audioMono"];
        if (!name) return fail("type must be one of " + Object.keys(types).join(", "));
        const list = trackList();
        if (!list) return fail("no song open");
        const before = uniqueTracks(list);
        const r = this.run("Track", name);
        if (isFail(r)) return r;
        const added = uniqueTracks(list).filter(t => before.indexOf(t) < 0).map(t => String(t.name));
        return { added: added, trackCount: uniqueTracks(list).length };
    }

    hostProbe(args) {
        const root = docObject("");
        const env = docObject("Environment");
        const synths = env && has(env, "find", "function") ? env.find("Synths") : null;
        const result = { context: this.context(), root: memberTypes(root), environment: memberTypes(env), synths: memberTypes(synths) };
        if (args && args.channel) {
            const target = this.mixerChannel(args.channel);
            if (isFail(target)) return target;
            const channel = target;
            result.channel = { name: args.channel, members: memberTypes(channel), children: {} };
            for (const name of ["Panner", "MixFXRack", "Inserts", "Graph", "FXGraph"]) {
                const child = has(channel, "find", "function") ? channel.find(name) : null;
                const info = { members: memberTypes(child), children: {} };
                for (const nested of ["PanRack", "PanFX", "Graph", "FXGraph", "Device", "FX01"]) {
                    const node = child && has(child, "find", "function") ? child.find(nested) : null;
                    info.children[nested] = memberTypes(node);
                }
                result.channel.children[name] = info;
            }
        }
        return result;
    }
}

function memberTypes(obj) {
    if (!obj) return null;
    const out = {};
    const names = ["self", "name", "title", "uniqueID", "classID", "paramList", "params", "count", "length", "numItems", "numParameters", "getParameterCount", "countParameters", "getParameter", "getParameterAt", "findParameter", "at", "get", "newIterator", "createIterator", "getChild", "countChildren", "getElement", "getElementCount", "insertDeviceClass", "getDevice", "createFunctions", "find", "queryInterface"];
    for (const name of names) {
        try { out[name] = typeof obj[name]; } catch (_) { out[name] = "unavailable"; }
    }
    if (has(obj, "paramList", "object") && obj.paramList) {
        out.paramListMembers = {};
        for (const name of ["params", "count", "length", "getParameter", "at", "get", "newIterator"]) {
            try { out.paramListMembers[name] = typeof obj.paramList[name]; } catch (_) { out.paramListMembers[name] = "unavailable"; }
        }
    }
    if (has(obj, "params", "object") && obj.params) {
        out.paramsMembers = {};
        for (const name of ["count", "length", "getParameter", "at", "get", "newIterator"]) {
            try { out.paramsMembers[name] = typeof obj.params[name]; } catch (_) { out.paramsMembers[name] = "unavailable"; }
        }
    }
    if (has(obj, "self", "object") && obj.self && obj.self !== obj) {
        out.selfMembers = {};
        for (const name of ["name", "params", "paramList", "count", "length", "getParameter", "getParameterCount", "findParameter"]) {
            try { out.selfMembers[name] = typeof obj.self[name]; } catch (_) { out.selfMembers[name] = "unavailable"; }
        }
    }
    return out;
}

// ---- helpers for the document object model --------------------------------------

function fail(message) { return { bridgeError: String(message) }; }
function isFail(value) { return !!value && typeof value === "object" && typeof value.bridgeError === "string"; }

function sameFixtureContext(a, b) {
    return !!a && !!b && a.open === true && b.open === true
        && typeof a.session === "string" && !!a.session && a.session === b.session
        && typeof a.document === "string" && !!a.document && a.document === b.document
        && typeof a.title === "string" && a.title === b.title;
}

function has(obj, key, type) {
    try { return obj !== null && obj !== undefined && typeof obj[key] === type; } catch (_) { return false; }
}

function appObject(name) {
    try { return Host.Objects.getObjectByUrl("://studioapp/" + name) || null; } catch (_) { return null; }
}

function docObject(path) {
    try { return Host.Objects.getObjectByUrl("://hostapp/DocumentManager/ActiveDocument/" + path) || null; } catch (_) { return null; }
}

function trackList() {
    const tl = docObject("TrackList");
    const list = tl && has(tl, "mainTrackList", "object") ? tl.mainTrackList : null;
    return list && has(list, "getTrack", "function") && has(list, "numTracks", "number") ? list : null;
}

// A track with takes shows up once per visible lane; they are the same object.
function uniqueTracks(list) {
    const out = [];
    for (let i = 0; i < list.numTracks; i++) {
        const t = list.getTrack(i);
        if (t && out.indexOf(t) < 0) out.push(t);
    }
    return out;
}

function selectedTracks(list) {
    const out = [];
    if (!has(list, "getSelectedTrack", "function") || !has(list, "numSelectedTracks", "number")) return out;
    for (let i = 0; i < list.numSelectedTracks; i++) {
        const t = list.getSelectedTrack(i);
        if (t && out.indexOf(t) < 0) out.push(t);
    }
    return out;
}

// A time parameter set from seconds (number) or a bar position string ("9.1.1.0").
function setTime(param, value) {
    if (!param || !has(param, "setValue", "function")) return fail("time parameter not available");
    if (typeof value === "number") {
        if (value < 0) return fail("time must be >= 0 seconds");
        param.setValue(value, true);
        return null;
    }
    if (typeof value === "string" && /^\d+(\.\d+){0,3}$/.test(value) && has(param, "fromString", "function")) {
        param.fromString(value);
        return null;
    }
    return fail("time must be seconds (number) or bars like \"9.1.1.0\"");
}

function seconds(time) {
    return time && has(time, "seconds", "number") ? Math.round(time.seconds * 1000) / 1000 : null;
}

function trackEvents(track) {
    const out = [];
    if (!has(track, "createIterator", "function")) return out;
    const it = track.createIterator();
    let ev;
    while (it && (ev = it.next())) {
        const start = seconds(ev.startTime), end = seconds(ev.endTime);
        out.push({
            name: has(ev, "name", "string") ? ev.name : "",
            start: start, end: end,
            length: start !== null && end !== null ? Math.round((end - start) * 1000) / 1000 : null,
            muted: !!ev.isMuted,
        });
    }
    return out;
}

function bridgeConfig() {
    const cfg = typeof BridgeConfig === "object" ? BridgeConfig : null;
    if (!cfg || typeof cfg.mailbox !== "string" || cfg.mailbox.indexOf("file:///") !== 0) {
        Host.Console.writeLine("studio-one-mcp: BridgeConfig.js missing or invalid; bridge disabled");
        return null;
    }
    return cfg;
}
