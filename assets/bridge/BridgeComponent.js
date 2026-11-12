// studio-one-mcp bridge component: owns the mailbox bridge (BridgeCore.js) and
// gives it the mixer through this surface's channel bank.

include_file("resource://com.presonus.musicdevices/sdk/controlsurfacecomponent.js");
include_file("BridgeConfig.js");
include_file("McpDocumentContext.js");
include_file("BridgeCore.js");
include_file("NativeControls.js");
include_file("NativePluginControls.js");

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
        if (!el || typeof el.getParamValue !== "function") return null;
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
        return { panner: panner, channel: channel };
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
        const format = typeof target.channel.findParameter === "function" ? target.channel.findParameter("speakerSetup") : null;
        return { channel: args.channel, params: params, missing: missing, speakerFormat: format ? this.paramInfo(format) : null };
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

    splitters(args) { return new NativeControls(this).splitters(args); }
    setSplitter(args) { return new NativeControls(this).setSplitter(args); }

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
            const identityError = this.nativeInstanceError(plug.device, args.expected_instance);
            if (identityError) return { error: identityError };
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
        if (args.kind === "panner") return this.pannerPluginOf(args);
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

    pannerPluginOf(args) {
        if (args.slot !== undefined && args.slot !== 0) return { error: "the channel panner has only slot 0" };
        if (!this.bridge || typeof this.bridge.mixerChannel !== "function") return { error: "the native mixer is not available" };
        const channel = this.bridge.mixerChannel(args.channel);
        if (channel && channel.bridgeError) return { error: channel.bridgeError };
        const comp = channel && typeof channel.find === "function" ? channel.find("Panner/AudioPanFX/FXSlot") : null;
        const dev = comp && typeof comp.find === "function" ? comp.find("Device") : null;
        if (!dev || typeof dev.findParameter !== "function" || typeof comp.findParameter !== "function") return { error: "the channel has no instantiated panner plugin" };
        const labelParam = comp.findParameter("deviceName");
        const label = labelParam && typeof labelParam.value === "string" ? labelParam.value : null;
        // The native PanFX slot is outside the surface's ordinary insert bank.
        // Require its reported identity and its actual Device to agree.
        if (!label || dev.name !== label) return { error: "the panner plugin identity is unavailable or inconsistent" };
        const identity = this.pluginIdentity(null, label, args.plugin);
        if (identity.error) return identity;
        return { name: identity.name, instanceName: label, device: dev, component: comp, element: null };
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
        if (!names.length) {
            const native = this.nativePluginRequest(plug.device, { operation: "parameters", source: args.source || "auto", offset: args.offset === undefined ? 0 : args.offset, limit: args.limit === undefined ? 64 : args.limit, expected_instance: args.expected_instance });
            if (native) {
                if (!native.ok) return { channel: args.channel, slot: args.slot, plugin: plug.name, params: [], missing: [], complete: false, native_error: native.error, parameter_discovery: { source: "native", completeness: "unavailable" } };
                const params = [], missing = [];
                if (!Array.isArray(native.parameters) || native.parameters.length > 128) return { error: "invalid native parameter page" };
                for (const record of native.parameters) {
                    if (!record || typeof record.name !== "string" || record.name.length > 512) return { error: "invalid native parameter name" };
                    if (native.source === "controller") params.push(record);
                    else if (native.source === "dsp") {
                        if (record.native_description && typeof record.native_description === "object") {
                            params.push(Object.assign({}, record.native_description, { name: record.name, native: record, access_metadata_complete: true }));
                            continue;
                        }
                        const p = record.name && typeof plug.device.findParameter === "function" ? plug.device.findParameter(record.name) : null;
                        if (p) params.push(Object.assign(this.paramInfo(p), { name: record.name, native: record, access_metadata_complete: false }));
                        else {
                            params.push({ name: record.name, value: null, text: null, min: null, max: null, normalized: null, readable: false, native: record });
                            missing.push(record.name || "[native index " + record.index + "]");
                        }
                    } else return { error: "unsupported native parameter source" };
                }
                const pageComplete = native.page_complete === true && (!native.failed_indices || native.failed_indices.length === 0);
                return { channel: args.channel, slot: args.slot, plugin: plug.name, params: params, missing: missing, page_complete: pageComplete, complete: pageComplete && native.offset === 0 && native.next_offset === null && native.reported_count === params.length,
                    value_read_complete: missing.length === 0, access_metadata_complete: native.source === "controller" ? pageComplete : native.access_metadata_complete === true,
                    failed_access_indices: native.failed_access_indices || [], failed_indices: native.failed_indices || [], reported_count: native.reported_count, offset: native.offset, next_offset: native.next_offset, instance: native.instance,
                    parameter_discovery: { source: native.source, completeness: "current_provider", all_indices_read: native.all_indices_read === true, note: "This page covers the current provider and selection, not every nested device, model or editor action." } };
            }
            if (args.offset || args.limit !== undefined || args.source && args.source !== "auto") return { error: "native parameter enumeration requires the matching native extension" };
        }
        const params = [];
        const missing = [];
        const identityError = this.nativeInstanceError(plug.device, args.expected_instance);
        if (identityError) return { error: identityError };
        for (const n of names) {
            if (args.source === "dsp") {
                const native = this.nativePluginRequest(plug.device, { operation: "parameter", source: "dsp", param: String(n), expected_instance: args.expected_instance });
                if (!native) return { error: "native DSP parameter descriptions require the matching native extension" };
                if (native.ok && native.parameter) params.push(Object.assign({}, native.parameter, { name: String(n) }));
                else missing.push(String(n));
                continue;
            }
            const p = args.source === "controller" ? null : this.pluginParameter(plug, String(n));
            if (p) params.push(this.paramInfo(p));
            else {
                const native = args.source === "dsp" ? null : this.nativePluginRequest(plug.device, { operation: "parameter", source: "controller", param: String(n), expected_instance: args.expected_instance });
                if (native && native.ok && native.parameter) params.push(native.parameter);
                else missing.push(String(n));
            }
        }
        return { channel: args.channel, slot: args.slot, plugin: plug.name, params: params, missing: missing };
    }

    nativePluginRequest(device, request, auxiliary) {
        return typeof NativePluginControls === "function" ? new NativePluginControls(this).request(device, request, auxiliary) : null;
    }

    pluginChoices(args) {
        const plug = this.pluginOf(args);
        if (plug.error) return plug;
        const native = this.nativePluginRequest(plug.device, { operation: "choices", source: args.source || "controller", param: args.param,
            offset: args.offset === undefined ? 0 : args.offset, limit: args.limit === undefined ? 64 : args.limit, expected_instance: args.expected_instance });
        if (!native) return { error: "native list choices require the matching native extension" };
        if (!native.ok) return { error: native.error && native.error.message || "native list choices unavailable" };
        if (!Array.isArray(native.choices) || native.choices.length > 128) return { error: "invalid native choices page" };
        return Object.assign({}, native, { channel: args.channel, slot: args.slot, plugin: plug.name, param: args.param });
    }

    nativeInstanceError(device, expected) {
        if (expected === undefined) return null;
        const native = this.nativePluginRequest(device, { operation: "inspect", expected_instance: expected });
        return native && native.ok && native.instance && native.instance.object_uid === expected ? null : "the expected native plugin instance is unavailable or changed";
    }

    pluginParameterDetails(args) {
        if (typeof args.plugin !== "string" || !args.plugin.trim() || typeof args.expected_instance !== "string" || !args.expected_instance
            || typeof args.param !== "string" || !args.param.trim() || args.param.length > 512 || args.param.indexOf("\0") >= 0
            || (args.source !== "dsp" && args.source !== "controller")
            || (args.include_owner !== undefined && typeof args.include_owner !== "boolean")
            || (args.compare_gui !== undefined && typeof args.compare_gui !== "boolean"))
            return { error: "an expected plugin, native instance, bounded parameter and explicit source are required" };
        if (args.compare_gui === true && (args.plugin !== "Chorder" || args.param !== "chorder.autofill" || args.source !== "dsp"
            || !/^[0-9A-F]{8}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{12}$/.test(args.expected_instance)
            || args.expected_instance === "00000000-0000-0000-0000-000000000000"))
            return { error: "GUI comparison requires the exact Chorder Auto Fill dsp target and native instance" };
        const plug = this.pluginOf(args);
        if (plug.error) return plug;
        if (args.compare_gui === true && plug.name !== "Chorder") return { error: "the resolved GUI comparison target is not Chorder" };
        const request = { operation: "parameter_details", source: args.source,
            param: args.param, expected_instance: args.expected_instance };
        if (args.include_owner !== undefined) request.include_owner = args.include_owner;
        if (args.compare_gui === true) request.compare_gui = true;
        const native = this.nativePluginRequest(plug.device, request);
        if (!native) return { error: "parameter details require the matching native extension" };
        if (args.compare_gui === true && !native.ok) return Object.assign({}, native, {
            error: native.error && native.error.message || "native GUI comparison unavailable", plugin: plug.name });
        if (!native.ok) return { error: native.error && native.error.message || "native parameter details unavailable" };
        return Object.assign({}, native, { plugin: plug.name });
    }

    pluginParameterFormat(args) {
        if (typeof args.plugin !== "string" || !args.plugin.trim() || typeof args.expected_instance !== "string" || !args.expected_instance
            || typeof args.param !== "string" || !args.param.trim() || args.param.length > 512 || args.param.indexOf("\0") >= 0
            || (args.source !== "dsp" && args.source !== "controller") || !Array.isArray(args.values)
            || args.values.length < 1 || args.values.length > 32
            || args.values.some(value => typeof value !== "number" || !isFinite(value) || Math.abs(value) > 9007199254740991))
            return { error: "an expected plugin, native instance, bounded parameter, explicit source and 1..32 finite numeric values are required" };
        const plug = this.pluginOf(args);
        if (plug.error) return plug;
        const native = this.nativePluginRequest(plug.device, { operation: "format_values", source: args.source,
            param: args.param, values: args.values, expected_instance: args.expected_instance });
        if (!native) return { error: "parameter formatting requires the matching native extension" };
        if (!native.ok) return { error: native.error && native.error.message || "native parameter formatting unavailable" };
        return Object.assign({}, native, { plugin: plug.name });
    }

    pluginParameterDigest(args) {
        if (typeof args.plugin !== "string" || !args.plugin.trim() || typeof args.expected_instance !== "string" || !args.expected_instance
            || (args.source !== "dsp" && args.source !== "controller"))
            return { error: "an expected plugin, native instance and explicit source are required" };
        const plug = this.pluginOf(args);
        if (plug.error) return plug;
        const native = this.nativePluginRequest(plug.device, { operation: "parameter_digest", source: args.source,
            expected_instance: args.expected_instance });
        if (!native) return { error: "parameter digests require the matching native extension" };
        if (!native.ok) return { error: native.error && native.error.message || "native parameter digest unavailable" };
        return Object.assign({}, native, { plugin: plug.name });
    }

    pluginModels(args) {
        const fields = ["channel", "slot", "kind", "track", "plugin", "expected_instance", "catalog", "offset", "limit"];
        const uid = /^[0-9A-F]{8}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{12}$/;
        const integer = (value, min, max) => typeof value === "number" && isFinite(value) && Math.floor(value) === value && value >= min && value <= max;
        if (Object.keys(args).some(key => fields.indexOf(key) < 0)
            || typeof args.plugin !== "string" || !args.plugin.trim()
            || typeof args.expected_instance !== "string" || !uid.test(args.expected_instance)
            || args.expected_instance === "00000000-0000-0000-0000-000000000000"
            || (args.catalog !== "categories" && args.catalog !== "effects")
            || (args.offset !== undefined && !integer(args.offset, 0, 4096))
            || (args.limit !== undefined && !integer(args.limit, 1, 32))
            || (args.channel !== undefined && (typeof args.channel !== "string" || !args.channel))
            || (args.track !== undefined && (typeof args.track !== "string" || !args.track))
            || (args.kind !== undefined && ["effect", "instrument", "note_effect", "mix_fx", "panner"].indexOf(args.kind) < 0)
            || (args.slot !== undefined && !integer(args.slot, 0, 15)))
            return { error: "model lists require a finite catalog, expected plugin, canonical native instance and bounded target/page", native_outcome: "not_applied" };
        if ((args.kind === "instrument" || args.kind === "note_effect") ?
            !args.track || (args.kind === "instrument" && args.slot !== undefined && args.slot !== 0) :
            !args.channel || ((args.kind === "mix_fx" || args.kind === "panner") ? args.slot !== undefined && args.slot !== 0 : args.slot === undefined))
            return { error: "model lists require the standard explicit plugin target", native_outcome: "not_applied" };
        const plug = this.pluginOf(args);
        if (plug.error) return plug;
        const native = this.nativePluginRequest(plug.device, { operation: "model_catalog", catalog: args.catalog,
            offset: args.offset === undefined ? 0 : args.offset, limit: args.limit === undefined ? 32 : args.limit,
            expected_instance: args.expected_instance });
        if (!native) return { error: "model lists require the matching native extension", native_outcome: "not_applied" };
        if (!native.ok) return { error: native.error && native.error.message || "native model list unavailable", native_outcome: native.outcome === "not_applied" ? "not_applied" : "unknown",
            atomic_snapshot: false, mutation_capability: false };
        return Object.assign({}, native, { plugin: plug.name });
    }

    focusedTrackState() {
        const model = this.hostComponent && this.hostComponent.model;
        const music = model && model.root && typeof model.root.find === "function" ? model.root.find("music") : null;
        const bank = music && typeof music.find === "function" ? music.find("noteChannel") : null;
        const channel = bank && typeof bank.getElement === "function" ? bank.getElement(0) : null;
        if (!channel || typeof channel.isConnected !== "function" || !channel.isConnected()) return null;
        return { label: this.readParam(channel, PreSonus.ParamID.kLabel), mute: this.readParam(channel, "mute"), recordArmed: this.readParam(channel, PreSonus.ParamID.kRecord), monitor: this.readParam(channel, "monitor") };
    }

    fixtureTrackState() {
        try {
            const state = this.focusedTrackState();
            if (!state || typeof state.label !== "string" || !state.label
                || ["mute", "recordArmed", "monitor"].some(name => state[name] !== true && state[name] !== false && state[name] !== 0 && state[name] !== 1))
                return { error: "complete focused Music fixture switches are unavailable" };
            return { label: state.label, mute: state.mute, recordArmed: state.recordArmed, monitor: state.monitor };
        } catch (_) { return { error: "focused Music fixture state is unavailable" }; }
    }

    pluginInspect(args) {
        if (args.node_scope === "plugin_controller" && (args.include_nodes !== true
            || typeof args.plugin !== "string" || !args.plugin.trim()
            || typeof args.expected_instance !== "string" || !args.expected_instance))
            return { error: "plugin controller inspection requires an expected plugin, native instance and include_nodes=true" };
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
        const result = { plugin: plug.name, members: memberTypes(plug.device), component: memberTypes(plug.component), element: element, track: plug.track || null };
        if (plug.track) result.focusedTrackState = this.focusedTrackState();
        const native = this.nativePluginRequest(plug.device, { operation: "inspect", source: args.source, expected_instance: args.expected_instance, include_nodes: args.include_nodes, node_scope: args.node_scope });
        if (args.expected_instance !== undefined && (!native || !native.ok || !native.instance || native.instance.object_uid !== args.expected_instance)) return { error: "the expected native plugin instance changed" };
        if (native) result.native = native;
        return result;
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

    editChorderAutofill(args) {
        const fields = ["track", "slot", "enabled", "expected_instance"];
        const failure = message => ({ ok: false, native_outcome: "not_applied", gui_value_verified: false,
            persistence_verified: false, effects_verified: false, dsp_propagation_verified: false,
            errors: [{ code: "invalid_chorder_autofill", message: message }] });
        if (!args || typeof args !== "object" || Array.isArray(args)
            || Object.keys(args).length !== fields.length || Object.keys(args).some(key => fields.indexOf(key) < 0)
            || typeof args.track !== "string" || !args.track.trim() || args.track.length > 256 || /[\x00-\x1f\x7f-\x9f]/.test(args.track)
            || typeof args.slot !== "number" || Math.floor(args.slot) !== args.slot || args.slot < 0 || args.slot >= 16
            || typeof args.enabled !== "boolean" || typeof args.expected_instance !== "string"
            || !/^[0-9A-F]{8}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{12}$/.test(args.expected_instance)
            || args.expected_instance === "00000000-0000-0000-0000-000000000000")
            return failure("A bounded track, slot, Boolean state and canonical native instance are required");
        const plug = this.pluginOf({ kind: "note_effect", track: args.track, slot: args.slot, plugin: "Chorder" });
        if (!plug || plug.error || plug.name !== "Chorder") return failure("The exact Chorder Note FX target is unavailable");
        const native = this.nativePluginRequest(plug.device, { operation: "edit_chorder_autofill",
            enabled: args.enabled, expected_instance: args.expected_instance });
        if (!native) return failure("The matching native Chorder GUI edit extension is required");
        const result = Object.assign({}, native, { plugin: "Chorder", native_outcome: native.outcome });
        if (native.ok === false) {
            result.errors = [native.error];
            delete result.error;
        }
        return result;
    }

    chorderLearningGet(args) { return this.chorderLearning(args, false); }
    chorderLearningSet(args) { return this.chorderLearning(args, true); }

    chorderChordsGet(args) {
        const controls = new NativePluginControls(this);
        const fields = ["track", "slot", "expected_instance", "offset", "limit"];
        const failure = () => controls.chorderChordsFailure(null);
        if (!controls.guiExact(args, fields) || typeof args.track !== "string" || !args.track.trim()
            || args.track.length > 256 || /[\x00-\x1f\x7f-\x9f]/.test(args.track) || !controls.guiInteger(args.slot, 0, 15)) return failure();
        const request = { operation: "chorder_chords_get", expected_instance: args.expected_instance, offset: args.offset, limit: args.limit };
        if (!controls.chorderChordsRequest(request)) return failure();
        const plug = this.pluginOf({ kind: "note_effect", track: args.track, slot: args.slot, plugin: "Chorder" });
        if (!plug || plug.error || plug.name !== "Chorder") return failure();
        const native = this.nativePluginRequest(plug.device, request);
        if (!native) return failure();
        return Object.assign({}, native, { plugin: "Chorder", native_outcome: native.outcome });
    }

    chorderLearning(args, setter) {
        const fields = setter ? ["track", "slot", "expected_instance", "change"] : ["track", "slot", "expected_instance"];
        const failure = message => setter ? {
            ok: false, native_outcome: "not_applied", gui_value_verified: false, persistence_verified: false,
            effects_verified: false, dsp_propagation_verified: false, errors: [{ code: "invalid_chorder_learning", message: message }]
        } : { ok: false, native_outcome: "not_applied", mutation_capability: false,
            error: { code: "invalid_chorder_learning", message: message } };
        if (!args || typeof args !== "object" || Array.isArray(args)
            || Object.keys(args).length !== fields.length || Object.keys(args).some(key => fields.indexOf(key) < 0)
            || typeof args.track !== "string" || !args.track.trim() || args.track.length > 256 || /[\x00-\x1f\x7f-\x9f]/.test(args.track)
            || typeof args.slot !== "number" || Math.floor(args.slot) !== args.slot || args.slot < 0 || args.slot >= 16
            || typeof args.expected_instance !== "string"
            || !/^[0-9A-F]{8}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{12}$/.test(args.expected_instance)
            || args.expected_instance === "00000000-0000-0000-0000-000000000000"
            || (setter && !new NativePluginControls(this).guiLearningChange(args.change)))
            return failure("A bounded track, slot, canonical native instance and fixed learning intent are required");
        const plug = this.pluginOf({ kind: "note_effect", track: args.track, slot: args.slot, plugin: "Chorder" });
        if (!plug || plug.error || plug.name !== "Chorder") return failure("The exact Chorder Note FX target is unavailable");
        const request = { operation: setter ? "edit_chorder_model" : "chorder_learning_get", expected_instance: args.expected_instance };
        if (setter) request.change = args.change;
        const native = this.nativePluginRequest(plug.device, request);
        if (!native) return failure("The matching native Chorder learning extension is required");
        const result = Object.assign({}, native, { plugin: "Chorder", native_outcome: native.outcome });
        if (setter && native.ok === false) {
            result.errors = [native.error];
            delete result.error;
            result.gui_value_verified = false;
            result.persistence_verified = false;
            result.effects_verified = false;
            result.dsp_propagation_verified = false;
        }
        return result;
    }

    pluginAction(args) {
        const fields = ["channel", "slot", "kind", "track", "plugin", "expected_instance", "action"];
        if (Object.keys(args).some(key => fields.indexOf(key) < 0)
            || typeof args.plugin !== "string" || !args.plugin.trim()
            || typeof args.expected_instance !== "string" || !args.expected_instance
            || args.action !== "swap_filter_frequencies")
            return { native_outcome: "not_applied", errors: [{ code: "invalid_action", message: "a fixed action, expected plugin and native instance are required" }] };
        const plug = this.pluginOf(args);
        if (plug.error) return plug;
        const native = this.nativePluginRequest(plug.device, { operation: "parameter_action",
            action: args.action, expected_instance: args.expected_instance });
        if (!native) return { native_outcome: "not_applied", errors: [{ code: "native_required", message: "plugin actions require the matching native extension" }] };
        return Object.assign({}, native, { plugin: plug.name, native_outcome: native.outcome });
    }

    insertChainState(args) {
        const result = typeof NativePluginControls === "function" ? new NativePluginControls(this).insertChainRequest(args)
            : { ok: false, outcome: "not_applied", error: { code: "native_required", message: "The matching native insert-chain extension is required" } };
        if (!result.ok) return { native_outcome: result.outcome, errors: [result.error] };
        return Object.assign({}, result, { native_outcome: result.outcome });
    }

    pluginState(args) {
        if (typeof args.plugin !== "string" || !args.plugin.trim() || typeof args.expected_instance !== "string" || !args.expected_instance)
            return { error: "an expected plugin and native instance are required" };
        const operations = { capture: "state_capture", restore: "state_restore", release: "state_release" };
        const operation = operations[args.action];
        if (typeof operation !== "string") return { error: "unsupported plugin state action" };
        if (args.action === "capture" ? args.token !== undefined : typeof args.token !== "string" || !args.token)
            return { error: "capture takes no token; restore and release require an issued token" };
        const plug = this.pluginOf(args);
        if (plug.error) return plug;
        const native = this.nativePluginRequest(plug.device, { operation: operation, token: args.token, expected_instance: args.expected_instance });
        if (!native) return { error: "plugin state operations require the matching native extension" };
        if (!native.ok) return { plugin: plug.name, native_outcome: native.outcome, errors: [native.error] };
        return Object.assign({}, native, { channel: args.channel, slot: args.slot, plugin: plug.name });
    }

    pluginPresets(args) {
        if (typeof args.plugin !== "string" || !args.plugin.trim() || typeof args.expected_instance !== "string" || !args.expected_instance)
            return { error: "an expected plugin and native instance are required" };
        if (args.catalog_token !== undefined && (typeof args.catalog_token !== "string" || !/^[0-9A-F]{64}$/.test(args.catalog_token)))
            return { error: "an issued uppercase hexadecimal catalog token is required" };
        const operations = { list: "presets", apply: "preset_apply", release: "preset_release" };
        const operation = operations[args.action];
        if (typeof operation !== "string") return { error: "unsupported plugin preset action" };
        const pageValid = (args.offset === undefined || (Number.isInteger(args.offset) && args.offset >= 0 && args.offset <= 4096))
            && (args.limit === undefined || (Number.isInteger(args.limit) && args.limit >= 1 && args.limit <= 64));
        const valid = args.action === "list" ? args.index === undefined && pageValid
            : args.catalog_token !== undefined && args.offset === undefined && args.limit === undefined
                && (args.action === "apply" ? Number.isInteger(args.index) && args.index >= 0 && args.index < 4096 : args.index === undefined);
        if (!valid) return { error: "list takes a bounded page; apply requires a catalog token and index; release takes only the catalog token" };
        const plug = this.pluginOf(args);
        if (plug.error) return plug;
        let auxiliary;
        if (args.action === "list" && args.catalog_token === undefined) {
            if (typeof Host === "undefined" || typeof Host.Url !== "function") return { error: "the native preset catalog requires the host URL factory" };
            try { auxiliary = Host.Url(""); }
            catch (_) { return { error: "the empty host URL prototype is unavailable" }; }
            if (!auxiliary || typeof auxiliary !== "object") return { error: "the empty host URL prototype is unavailable" };
        }
        const request = { operation: operation, expected_instance: args.expected_instance, catalog_token: args.catalog_token };
        if (args.action === "list") {
            request.offset = args.offset === undefined ? 0 : args.offset;
            request.limit = args.limit === undefined ? 32 : args.limit;
        } else if (args.action === "apply") request.index = args.index;
        const native = this.nativePluginRequest(plug.device, request, auxiliary);
        if (!native) return { error: "plugin presets require the matching native extension" };
        if (!native.ok) return { plugin: plug.name, native_outcome: native.outcome, errors: [native.error] };
        return Object.assign({}, native, { channel: args.channel, slot: args.slot, plugin: plug.name, native_outcome: native.outcome });
    }

    // One of: text (display text, e.g. "4.0:1" or "-12 dB"), normalized (0..1), value (raw).
    setPluginParam(args) {
        if (args.edit_lifecycle !== undefined
            && (args.edit_lifecycle !== "native_user" || args.source !== "dsp"
                || typeof args.expected_instance !== "string"
                || !/^[0-9A-F]{8}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{12}$/.test(args.expected_instance)
                || args.expected_instance === "00000000-0000-0000-0000-000000000000"
                || typeof args.param !== "string" || !args.param || args.param.length > 512 || args.param.indexOf("\0") >= 0
                || (args.value !== 0 && args.value !== 1)
                || args.normalized !== undefined || args.text !== undefined || args.float_bits !== undefined))
            return { native_outcome: "not_applied", errors: [{ code: "invalid_native_edit", message: "Native user edit requires explicit dsp, expected_instance and one raw 0 or 1 value" }] };
        if (typeof args.plugin !== "string" || !args.plugin.trim()) return { error: "an expected plugin class or instance name is required" };
        const plug = this.pluginOf(args);
        if (plug.error) return plug;
        const identityError = this.nativeInstanceError(plug.device, args.expected_instance);
        if (identityError) return { plugin: plug.name, native_outcome: "not_applied",
            guard: "native_instance_preflight", errors: [{ code: "instance_guard", message: identityError }] };
        if (args.edit_lifecycle === "native_user") {
            const native = this.nativePluginRequest(plug.device, { operation: "edit_numeric_parameter", param: args.param,
                value: args.value, expected_instance: args.expected_instance });
            if (!native) return { native_outcome: "not_applied", errors: [{ code: "native_edit_unavailable", message: "Native user edit requires the matching native extension" }] };
            if (!native.ok) return { channel: args.channel, slot: args.slot, plugin: plug.name, param: args.param,
                native_outcome: native.outcome, edit_phase: native.edit_phase, errors: [native.error],
                before: native.before || null, after: native.after || null };
            return { channel: args.channel, slot: args.slot, plugin: plug.name, param: args.param,
                before: native.before, after: native.after, instance: native.instance, source: native.source,
                edit_phase: native.edit_phase, native_verified: native.verified === true };
        }
        if (args.float_bits !== undefined && args.source === undefined) return { error: "float_bits requires an explicit native source" };
        if (args.source !== undefined) {
            const mode = args.float_bits !== undefined ? "float_bits" : args.text !== undefined ? "text" : args.normalized !== undefined ? "normalized" : "raw";
            const value = mode === "float_bits" ? args.float_bits : mode === "text" ? args.text : mode === "normalized" ? args.normalized : args.value;
            const native = this.nativePluginRequest(plug.device, { operation: "set_parameter", source: args.source, param: String(args.param), mode: mode, value: value, expected_instance: args.expected_instance });
            if (!native) return { error: "an explicit parameter source requires the matching native extension" };
            if (!native.ok) return { channel: args.channel, slot: args.slot, plugin: plug.name, param: String(args.param), native_outcome: native.outcome, errors: [native.error], before: native.before || null, after: native.after || null };
            return { channel: args.channel, slot: args.slot, plugin: plug.name, param: String(args.param), before: native.before, after: native.after, instance: native.instance, source: native.source, native_verified: native.verified === true };
        }
        const p = this.pluginParameter(plug, String(args.param));
        if (!p) {
            const mode = args.text !== undefined ? "text" : args.normalized !== undefined ? "normalized" : "raw";
            const value = mode === "text" ? args.text : mode === "normalized" ? args.normalized : args.value;
            const native = this.nativePluginRequest(plug.device, { operation: "set_parameter", source: "controller", param: String(args.param), mode: mode, value: value, expected_instance: args.expected_instance });
            if (!native) return { error: "no parameter " + args.param + " on " + plug.name };
            if (!native.ok) return { channel: args.channel, slot: args.slot, plugin: plug.name, param: String(args.param), native_outcome: native.outcome, errors: [native.error], before: native.before || null, after: native.after || null };
            return { channel: args.channel, slot: args.slot, plugin: plug.name, param: String(args.param), before: native.before, after: native.after, instance: native.instance, source: native.source, native_verified: native.verified === true };
        }
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
