// studio-one-mcp bridge component: owns the mailbox bridge (BridgeCore.js) and
// gives it the mixer through this surface's channel bank.

include_file("resource://com.presonus.musicdevices/sdk/controlsurfacecomponent.js");
include_file("BridgeConfig.js");
include_file("BridgeCore.js");

const AUTOMATION_MODES = ["off", "read", "touch", "latch", "write"];
const PANNER_FIELDS = ["pannerType", "stereoMode", "panX1", "panX2", "panWidth"];

class BridgeComponent extends PreSonus.ControlSurfaceComponent {
    onInit(hostComponent) {
        super.onInit(hostComponent);
        this.tickParam = hostComponent.paramList.addParam("bridgeTick");
        // An alias parameter can be pointed at any element's parameter to read
        // its display text (a send's destination name, a level in dB), the way
        // the FaderPort script fills its scribble strips.
        this.displayAlias = typeof hostComponent.paramList.addAlias === "function"
            ? hostComponent.paramList.addAlias("bridgeDisplay") : null;
        this.bridge = null;
        try {
            const cfg = bridgeConfig();
            if (cfg) this.bridge = new Bridge(cfg, this);
        } catch (e) {
            Host.Console.writeLine("studio-one-mcp: bridge init failed: " + e);
        }
        // The client's note 119 wakes the bridgeTick control.
        // Do NOT use Host.GUI.addIdleTask with a script object here: on Studio One
        // 5.5.2 that crashed the app at launch (EXC_BAD_ACCESS in cclgui's timer).
        if (this.bridge) this.bridge.beat(true); // publish clockErrors even if no clock fires
    }

    onExit() {
        if (this.bridge) this.bridge.close();
        this.bridge = null;
        super.onExit();
    }

    clockTick(source) {
        if (!this.bridge) return;
        try { this.bridge.tick(source); }
        catch (e) { Host.Console.writeLine("studio-one-mcp: tick failed: " + e); }
    }

    onTick_bridgeTimer(time) {
        this.clockTick("timer");
    }

    paramChanged(param) {
        if (param === this.tickParam) return this.clockTick("midi");
        super.paramChanged(param);
    }

    // Never throw out of a component method. Studio One wraps calls into the
    // component and turns any escaping exception into a modal "Scripting Error"
    // dialog, even when the caller has a try/catch. Failures are returned as
    // { error } values and BridgeCore.js rethrows them on its own side.
    channelElements() {
        const model = this.hostComponent && this.hostComponent.model;
        if (!model) return { error: "surface model not available" };
        const bank = model.root.find("mixer").find("channels");
        const out = [];
        for (let i = 0; i < 256; i++) {
            const el = bank.getElement(i);
            if (!el || !el.isConnected()) continue;
            const label = el.getParamValue(PreSonus.ParamID.kLabel);
            if (label !== undefined && label !== null && String(label) !== "") out.push({ index: i, el: el, label: String(label) });
        }
        return out;
    }

    readParam(el, id) {
        try { const v = el.getParamValue(id); return v === undefined ? null : v; } catch (_) { return null; }
    }

    channels() {
        const els = this.channelElements();
        if (els.error) return els;
        return els.map(c => ({
            index: c.index,
            label: c.label,
            type: this.readParam(c.el, PreSonus.ParamID.kChannelType),
            volume: this.readParam(c.el, PreSonus.ParamID.kVolume),
            pan: this.readParam(c.el, PreSonus.ParamID.kPan),
            mute: this.readParam(c.el, "mute"),
            solo: this.readParam(c.el, "solo"),
            recordArmed: this.readParam(c.el, PreSonus.ParamID.kRecord),
            automation: AUTOMATION_MODES[this.readParam(c.el, PreSonus.ParamID.kAutoMode)] || null,
            monitor: this.readParam(c.el, "monitor"),
            // Routing is a list index (-1) whose names are only display text, like
            // sendPort. Read-only: fromString with a name did not change it (5.5.2).
            input: this.readParam(c.el, "recordPort") === null ? null : this.displayOf(c.el, "recordPort"),
            output: this.readParam(c.el, "outputPort") === null ? null : this.displayOf(c.el, "outputPort"),
        }));
    }

    // Automation mode is a list parameter on the channel: 0 Off .. 4 Write
    // (checked against its display text on 5.5.2).
    setAutomation(args) {
        const mode = AUTOMATION_MODES.indexOf(args.mode);
        if (mode < 0) return { error: "mode must be one of " + AUTOMATION_MODES.join(", ") };
        const c = this.channelByLabel(args.channel);
        if (c.error) return c;
        const id = PreSonus.ParamID.kAutoMode;
        const before = AUTOMATION_MODES[this.readParam(c.el, id)] || null;
        c.el.setParamValue(id, mode);
        return { channel: args.channel, before: before, after: AUTOMATION_MODES[this.readParam(c.el, id)] || null };
    }

    // Peak meter per channel in dB (-144 is silence), both sides of a stereo strip.
    meters() {
        const els = this.channelElements();
        if (els.error) return els;
        return els.map(c => ({ label: c.label, left: this.readParam(c.el, "level1"), right: this.readParam(c.el, "level2") }));
    }

    pannerOf(channelName) {
        if (typeof channelName !== "string" || !channelName.trim()) return { error: "a channel name is required for standard panner access" };
        if (!this.bridge || typeof this.bridge.mixerChannel !== "function") return { error: "the native mixer is not available" };
        const channel = this.bridge.mixerChannel(channelName);
        if (channel && typeof channel.bridgeError === "string") return { error: channel.bridgeError };
        const panner = channel && typeof channel.find === "function" ? channel.find("Panner") : null;
        if (!panner || typeof panner.findParameter !== "function") return { error: "the standard channel panner is not available" };
        return { panner: panner };
    }

    panner(args) {
        const target = this.pannerOf(args.channel);
        if (target.error) return target;
        const params = [], missing = [];
        for (const name of PANNER_FIELDS) {
            const parameter = target.panner.findParameter(name);
            if (parameter) {
                const info = this.paramInfo(parameter);
                info.writable = name !== "pannerType" && typeof parameter.value === "number" && typeof parameter.setValue === "function";
                params.push(info);
            }
            else missing.push(name);
        }
        return { channel: args.channel, params: params, missing: missing };
    }

    setPanner(args) {
        if (args.param === "pannerType" || PANNER_FIELDS.indexOf(args.param) < 0) return { error: "unsupported standard panner parameter or read-only label" };
        if (typeof args.value !== "number" || !isFinite(args.value)) return { error: "a finite raw panner value is required" };
        const target = this.pannerOf(args.channel);
        if (target.error) return target;
        const parameter = target.panner.findParameter(args.param);
        if (!parameter || typeof parameter.setValue !== "function" || typeof parameter.value !== "number" || !isFinite(parameter.value)) return { error: "the standard panner parameter is not writable" };
        if (typeof parameter.min !== "number" || !isFinite(parameter.min) || typeof parameter.max !== "number" || !isFinite(parameter.max) || parameter.min > parameter.max) return { error: "the standard panner parameter has no supported native range" };
        if (args.value < parameter.min || args.value > parameter.max) return { error: "raw panner value is outside the supported range" };
        const before = this.paramInfo(parameter);
        parameter.setValue(args.value, true);
        return { channel: args.channel, param: args.param, before: before, after: this.paramInfo(parameter) };
    }

    // ---- inserts and sends ----------------------------------------------------
    //
    // Each channel strip carries two sub-banks from the surface file. Names come
    // from the bank element (@owner/deviceName, as Studio One's SDK names it);
    // Bypass is the native slot component's parameter. The parent channel path
    // does not resolve on Studio Pro 8.1.1.

    subBank(el, name) {
        if (!el || typeof el.find !== "function") return null;
        const bank = el.find(name);
        return bank && typeof bank.getElement === "function" ? bank : null;
    }

    channelByLabel(label) {
        const els = this.channelElements();
        if (els.error) return els;
        const matches = els.filter(c => c.label === label);
        if (matches.length !== 1) return { error: matches.length ? "channel name is ambiguous: " + label : "no channel named " + label };
        return matches[0];
    }

    insertsOf(el) {
        const bank = this.subBank(el, "inserts");
        const out = [];
        if (!bank) return out;
        for (let i = 0; i < 16; i++) {
            const slot = bank.getElement(i);
            if (!slot || typeof slot.isConnected !== "function" || !slot.isConnected()) continue;
            const name = this.readParam(slot, PreSonus.ParamID.kInsertName);
            if (name === null || String(name) === "") continue;
            const p = slot.component && typeof slot.component.findParameter === "function" ? slot.component.findParameter("bypass") : null;
            out.push({ slot: i, name: String(name), bypassed: p ? !!p.value : null });
        }
        return out;
    }

    inserts(args) {
        const els = this.channelElements();
        if (els.error) return els;
        const want = args && args.channel;
        const out = [];
        for (const c of els) {
            if (want && c.label !== want) continue;
            out.push({ channel: c.label, bypassAll: !!this.readParam(c.el, PreSonus.ParamID.kInsertBypass), inserts: this.insertsOf(c.el) });
        }
        if (want && !out.length) return { error: "no channel named " + want };
        return out;
    }

    // slot: number, or "all" for the channel's bypass-all switch.
    setInsertBypass(args) {
        const c = this.channelByLabel(args.channel);
        if (c.error) return c;
        if (args.slot !== "all") {
            if (typeof args.plugin !== "string" || !args.plugin.trim()) return { error: "an expected plugin class or instance name is required" };
            const plug = this.pluginOf(args);
            if (plug.error) return plug;
            const p = plug.component && typeof plug.component.findParameter === "function" ? plug.component.findParameter("bypass") : null;
            if (!p || typeof p.setValue !== "function" || typeof p.value !== "number") return { error: "slot bypass is not available" };
            const before = !!p.value;
            p.setValue(args.bypassed ? 1 : 0, true);
            return { channel: args.channel, slot: args.slot, plugin: plug.name, before: before, after: !!p.value };
        }
        const param = PreSonus.ParamID.kInsertBypass;
        const before = this.readParam(c.el, param);
        if (before === null) return { error: "rack bypass is not available" };
        c.el.setParamValue(param, args.bypassed ? 1 : 0);
        const after = this.readParam(c.el, param);
        return { channel: args.channel, slot: args.slot, before: !!before, after: after === null ? null : !!after };
    }

    displayOf(el, paramName) {
        const a = this.displayAlias;
        if (!a || !el || typeof el.connectAliasParam !== "function") return null;
        el.connectAliasParam(a, paramName);
        return typeof a.string === "string" ? a.string : null;
    }

    sendsOf(el) {
        const bank = this.subBank(el, "sends");
        const out = [];
        if (!bank) return out;
        for (let i = 0; i < 8; i++) {
            const send = bank.getElement(i);
            if (!send || typeof send.isConnected !== "function" || !send.isConnected()) continue;
            if (this.readParam(send, PreSonus.ParamID.kSendPort) === null) continue;
            out.push({
                index: i,
                to: this.displayOf(send, PreSonus.ParamID.kSendPort),
                level: this.readParam(send, PreSonus.ParamID.kSendLevel),
                levelDb: this.displayOf(send, PreSonus.ParamID.kSendLevel),
                muted: !!this.readParam(send, PreSonus.ParamID.kSendMute),
            });
        }
        return out;
    }

    sends(args) {
        const els = this.channelElements();
        if (els.error) return els;
        const want = args && args.channel;
        const out = [];
        for (const c of els) {
            if (want && c.label !== want) continue;
            const s = this.sendsOf(c.el);
            if (want || s.length) out.push({ channel: c.label, sends: s });
        }
        if (want && !out.length) return { error: "no channel named " + want };
        return out;
    }

    setSend(args) {
        const c = this.channelByLabel(args.channel);
        if (c.error) return c;
        const bank = this.subBank(c.el, "sends");
        const send = bank ? bank.getElement(args.index) : null;
        if (!send || !this.sendsOf(c.el).some(x => x.index === args.index)) return { error: "no send " + args.index + " on " + args.channel };
        const out = { channel: args.channel, index: args.index };
        if (args.level !== undefined) {
            if (typeof args.level !== "number" || args.level < 0 || args.level > 1) return { error: "level must be 0..1 (Studio One's normalised send level)" };
            send.setParamValue(PreSonus.ParamID.kSendLevel, args.level);
        }
        if (args.muted !== undefined) send.setParamValue(PreSonus.ParamID.kSendMute, args.muted ? 1 : 0);
        out.send = this.sendsOf(c.el).find(x => x.index === args.index);
        return out;
    }

    // ---- channel name and colour ------------------------------------------------
    //
    // A track's own name and colour are read-only to scripts, but setting its
    // channel's label / colour renames and recolours the track too (5.5.2). Colour
    // is ARGB as a signed 32-bit number. Neither change is on the undo stack.

    setChannelLabel(args) {
        const c = this.channelByLabel(args.channel);
        if (c.error) return c;
        if (typeof args.name !== "string" || args.name === "") return { error: "name must be a non-empty string" };
        c.el.setParamValue(PreSonus.ParamID.kLabel, args.name);
        return { before: args.channel, after: String(this.readParam(c.el, PreSonus.ParamID.kLabel)) };
    }

    setChannelColor(args) {
        const c = this.channelByLabel(args.channel);
        if (c.error) return c;
        if (typeof args.argb !== "number") return { error: "argb must be a number" };
        const id = PreSonus.ParamID.kColor || "color";
        const before = this.readParam(c.el, id);
        c.el.setParamValue(id, args.argb);
        return { channel: args.channel, before: before, after: this.readParam(c.el, id) };
    }

    // ---- plug-in parameters -----------------------------------------------------
    //
    // An insert slot's component ("FX01") has a child component "Device": the
    // plug-in itself. Its parameters are found by name ("comp.threshold" on the
    // Fat Channel, "lffreq" on the Pro EQ) and give value, display text, range and
    // a normalised value. The host cannot list them, so the server sends the names
    // (from the plug-in's presets); unknown names come back as null, never throw.

    pluginOf(args) {
        if (args.kind === "instrument") return this.instrumentOf(args);
        if (args.kind === "note_effect") return this.noteEffectOf(args);
        if (args.kind && args.kind !== "effect" && args.kind !== "mix_fx") return { error: "this plugin kind has no verified live parameter path yet" };
        const c = this.channelByLabel(args.channel);
        if (c.error) return c;
        const bank = this.subBank(c.el, args.kind === "mix_fx" ? "mixfx" : "inserts");
        const el = bank ? bank.getElement(args.slot || 0) : null;
        if (!el || typeof el.isConnected !== "function" || !el.isConnected()) return { error: "no plug-in in slot " + args.slot + " on " + args.channel };
        const label = this.readParam(el, PreSonus.ParamID.kInsertName);
        if (!label && !args.plugin) return { error: "the slot has no plugin identity" };
        const comp = el ? el.component : null;
        const dev = comp && typeof comp.find === "function" ? comp.find("Device") : null;
        if (!dev || typeof dev.findParameter !== "function") return { error: "cannot reach the plug-in in slot " + args.slot + " on " + args.channel };
        const identity = this.pluginIdentity(el, label, args.plugin);
        if (identity.error) return identity;
        return { name: identity.name, instanceName: label, device: dev, component: comp, element: el };
    }

    pluginIdentity(el, label, expected) {
        const className = el && typeof el.getPlugInClassName === "function" ? el.getPlugInClassName() : null;
        const matches = !expected || String(expected) === String(label) || String(expected) === String(className)
            || (el && typeof el.matchesPlugInClass === "function" && el.matchesPlugInClass(String(expected)));
        return matches ? { name: typeof className === "string" && className ? className : String(label) } : { error: "plugin class or instance identity does not match" };
    }

    instrumentOf(args) {
        const wanted = String(args.track || args.channel || "");
        if (!wanted || !this.bridge) return { error: "instrument target requires a selected track" };
        const tracks = this.bridge.tracks({ events: false });
        if (!Array.isArray(tracks)) return { error: "track state is unavailable" };
        const selected = tracks.filter(t => t.selected && t.mediaType === "Music");
        if (selected.length !== 1 || selected[0].name !== wanted) return { error: "select the unique target instrument track first" };
        const model = this.hostComponent && this.hostComponent.model;
        const root = model && model.root;
        const music = root && typeof root.find === "function" ? root.find("music") : null;
        const bank = music && typeof music.find === "function" ? music.find("instrument") : null;
        const el = bank && typeof bank.getElement === "function" ? bank.getElement(0) : null;
        if (!el || typeof el.isConnected !== "function" || !el.isConnected()) return { error: "the instrument mapping is not connected" };
        const comp = el.component;
        const dev = comp && typeof comp.find === "function" ? comp.find("Device") : null;
        const device = dev && typeof dev.findParameter === "function" ? dev : (comp && typeof comp.findParameter === "function" ? comp : null);
        if (!device) return { error: "the focused instrument device is not reachable" };
        const label = this.readParam(el, "deviceName");
        const name = typeof label === "string" && label ? label : (typeof device.name === "string" ? device.name : "focused instrument");
        const identity = this.pluginIdentity(el, name, args.plugin);
        if (identity.error) return identity;
        return { name: identity.name, instanceName: name, device: device, component: comp, element: el, track: wanted };
    }

    noteEffectOf(args) {
        const wanted = String(args.track || args.channel || "");
        const tracks = this.bridge ? this.bridge.tracks({ events: false }) : null;
        if (!Array.isArray(tracks)) return { error: "track state is unavailable" };
        const selected = tracks.filter(t => t.selected && t.mediaType === "Music");
        if (selected.length !== 1 || selected[0].name !== wanted) return { error: "select the unique target instrument track first" };
        const model = this.hostComponent && this.hostComponent.model;
        const root = model && model.root;
        const music = root && typeof root.find === "function" ? root.find("music") : null;
        const focus = music && typeof music.find === "function" ? music.find("noteChannel") : null;
        const channel = focus && typeof focus.getElement === "function" ? focus.getElement(0) : null;
        const bank = this.subBank(channel, "noteInserts");
        const el = bank ? bank.getElement(args.slot === undefined ? 0 : args.slot) : null;
        if (!el || typeof el.isConnected !== "function" || !el.isConnected()) return { error: "the Note FX slot is not connected" };
        const comp = el.component;
        const dev = comp && typeof comp.find === "function" ? comp.find("Device") : null;
        if (!dev || typeof dev.findParameter !== "function") return { error: "the Note FX device is not reachable" };
        const label = this.readParam(el, PreSonus.ParamID.kInsertName);
        const identity = this.pluginIdentity(el, label, args.plugin);
        if (identity.error) return identity;
        return { name: identity.name, instanceName: label, device: dev, component: comp, element: el, track: wanted };
    }

    paramInfo(p) {
        return {
            name: p.name,
            value: p.value,
            text: typeof p.string === "string" ? p.string : null,
            min: p.min,
            max: p.max,
            normalized: typeof p.getNormalized === "function" ? p.getNormalized() : null,
        };
    }

    pluginParameter(plug, name) {
        const parameter = plug.device.findParameter(name);
        if (parameter) return parameter;
        return plug.component && typeof plug.component.findParameter === "function" ? plug.component.findParameter(name) : null;
    }

    pluginParams(args) {
        const plug = this.pluginOf(args);
        if (plug.error) return plug;
        const names = Array.isArray(args.names) ? args.names : [];
        const params = [];
        const missing = [];
        for (const n of names) {
            const p = this.pluginParameter(plug, String(n));
            if (p) params.push(this.paramInfo(p));
            else missing.push(String(n));
        }
        return { channel: args.channel, slot: args.slot, plugin: plug.name, params: params, missing: missing };
    }

    pluginInspect(args) {
        const plug = this.pluginOf(args);
        if (plug.error) return plug;
        const element = {};
        for (const name of ["getParamCount", "getParamTitle", "getParamTitleWithName", "getParamValue", "setParamValue", "getProperty", "getPlugInClassName", "getPlugInInterface"]) {
            element[name] = plug.element ? typeof plug.element[name] : "undefined";
        }
        if (plug.element && typeof plug.element.getProperty === "function") {
            element.identityProperties = {};
            for (const name of ["deviceUID", "slotUID", "uniqueID", "classID", "deviceClassID"]) {
                const value = plug.element.getProperty(name);
                element.identityProperties[name] = typeof value === "string" || typeof value === "number" ? value : null;
            }
        }
        if (plug.element && typeof plug.element.getParamCount === "function") {
            const count = plug.element.getParamCount();
            element.parameterCount = count;
            element.firstParameters = [];
            if (typeof count === "number" && count >= 0 && typeof plug.element.getParamTitle === "function" && typeof plug.element.getParamValue === "function") {
                for (let i = 0; i < Math.min(count, 8); i++) {
                    element.firstParameters.push({ index: i, title: plug.element.getParamTitle(i), value: plug.element.getParamValue(i) });
                }
            }
        }
        element.slotControls = {};
        for (const name of ["bypass", "@bypass", "active", "@active", "@owner/bypass", "@owner/active"]) {
            const p = plug.component && typeof plug.component.findParameter === "function" ? plug.component.findParameter(name) : null;
            element.slotControls[name] = { mapped: this.readParam(plug.element, name), component: p ? this.paramInfo(p) : null };
        }
        return { plugin: plug.name, members: memberTypes(plug.device), component: memberTypes(plug.component), element: element, track: plug.track || null };
    }

    pluginEditorOpen(args) {
        if (typeof args.plugin !== "string" || !args.plugin.trim()) return { error: "an expected plugin class or instance name is required" };
        const plug = this.pluginOf(args);
        if (plug.error) return plug;
        if (!plug.element || typeof plug.element.invokeMethod !== "function") return { error: "the plugin editor is not available" };
        if (args.kind === "instrument") plug.element.invokeMethod("openEditor", "Instrument", false);
        else if (args.kind === "note_effect") plug.element.invokeMethod("openEditor", "Note FX", false);
        else plug.element.invokeMethod("openEditor");
        return { requested: true, plugin: plug.name, verified: false };
    }

    // One of: text (display text, e.g. "4.0:1" or "-12 dB"), normalized (0..1), value (raw).
    setPluginParam(args) {
        if (typeof args.plugin !== "string" || !args.plugin.trim()) return { error: "an expected plugin class or instance name is required" };
        const plug = this.pluginOf(args);
        if (plug.error) return plug;
        const p = this.pluginParameter(plug, String(args.param));
        if (!p) return { error: "no parameter " + args.param + " on " + plug.name + " (live_plugin_params lists them)" };
        const before = this.paramInfo(p);
        if (args.text !== undefined) {
            if (typeof p.fromString !== "function") return { error: "parameter " + args.param + " does not take text" };
            p.fromString(String(args.text), true);
        } else if (args.normalized !== undefined) {
            if (typeof args.normalized !== "number" || args.normalized < 0 || args.normalized > 1) return { error: "normalized must be 0..1" };
            if (typeof p.setNormalized !== "function") return { error: "parameter " + args.param + " has no normalised value" };
            p.setNormalized(args.normalized, true);
        } else if (args.value !== undefined) {
            if (typeof args.value !== "number" || !isFinite(args.value) || (typeof p.min === "number" && args.value < p.min) || (typeof p.max === "number" && args.value > p.max)) return { error: "raw parameter value is outside the supported range" };
            if (typeof p.setValue !== "function") return { error: "parameter " + args.param + " cannot be set" };
            p.setValue(args.value, true);
        } else {
            return { error: "give one of text, normalized or value" };
        }
        return { channel: args.channel, slot: args.slot, plugin: plug.name, param: String(args.param), before: before, after: this.paramInfo(p) };
    }

    setChannel(args) {
        const fields = { volume: PreSonus.ParamID.kVolume, pan: PreSonus.ParamID.kPan, mute: "mute", solo: "solo", recordArmed: PreSonus.ParamID.kRecord, monitor: "monitor" };
        const param = fields[args.field];
        if (!param) return { error: "field must be one of " + Object.keys(fields).join(", ") };
        const els = this.channelElements();
        if (els.error) return els;
        const matches = els.filter(c => c.label === args.channel);
        if (matches.length !== 1) return { error: matches.length ? "channel name is ambiguous: " + args.channel : "no channel named " + args.channel };
        const el = matches[0].el;
        const before = this.readParam(el, param);
        el.setParamValue(param, args.value);
        const out = { channel: args.channel, field: args.field, before: before, after: this.readParam(el, param) };
        // Volume and pan also as Studio One shows them ("-6.0 dB", "L25"): the
        // normalised fader curve is not documented, so dB are found by reading this.
        if (args.field === "volume" || args.field === "pan") out.text = this.displayOf(el, param);
        return out;
    }
}

function createBridgeComponent() {
    return new BridgeComponent();
}
