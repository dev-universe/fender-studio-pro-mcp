// Native controls with verified model paths. No graph creation or deletion.
class NativeControls {
    constructor(owner) { this.owner = owner; }

    graphNodes(args) {
        if (!args || typeof args.channel !== "string" || !args.channel.trim()) return { error: "a channel name is required" };
        const bridge = this.owner && this.owner.bridge;
        if (!bridge || typeof bridge.mixerChannel !== "function") return { error: "native channel lookup is unavailable" };
        try {
            const channel = bridge.mixerChannel(args.channel);
            if (!channel || channel.error || channel.bridgeError) return { error: channel && (channel.error || channel.bridgeError) || "channel is unavailable" };
            if (typeof channel.find !== "function") return { error: "channel graph lookup is unavailable" };
            const graph = channel.find("Inserts/Combinator/Graph");
            if (!graph) return { error: "the channel insert graph is unavailable" };
            const count = graph.numChildren;
            if (typeof count !== "number" || !isFinite(count) || count < 0 || Math.floor(count) !== count) return { error: "the insert graph child count is unavailable" };
            const nodes = [], diagnostics = [];
            for (let i = 0; i < Math.min(count, 128); i++) {
                try {
                    const node = graph["children[" + i + "]"];
                    if (!node) { diagnostics.push({ index: i, error: "graph child is unavailable" }); continue; }
                    const name = node.name, label = node.title;
                    if (typeof name !== "string" || typeof label !== "string") { diagnostics.push({ index: i, error: "graph child identity is unavailable" }); continue; }
                    nodes.push({ index: i, node: node, name: name, label: label });
                } catch (e) { diagnostics.push({ index: i, error: String(e).slice(0, 512) }); }
            }
            return { nodes: nodes, diagnostics: diagnostics, count: count, truncated: count > 128 };
        } catch (e) { return { error: "native graph lookup failed: " + String(e).slice(0, 512) }; }
    }

    parameter(node, name) {
        try { return node && typeof node.findParameter === "function" ? node.findParameter(name) : null; }
        catch (_) { return null; }
    }

    splitterParameters(node) {
        const count = this.parameter(node, "numOutputs");
        const mode = this.parameter(node, "splitMode");
        return count && mode ? { count: count, mode: mode } : null;
    }

    info(param) {
        try { return this.owner.paramInfo(param); }
        catch (e) { return { error: "native parameter read failed: " + String(e).slice(0, 512) }; }
    }

    branchCount(param) {
        try {
            const count = param.value;
            return typeof count === "number" && isFinite(count) && Math.floor(count) === count && count >= 2 && count <= 5 ? count : null;
        } catch (_) { return null; }
    }

    splitters(args) {
        const graph = this.graphNodes(args);
        if (graph.error) return graph;
        const splitters = [], diagnostics = graph.diagnostics.slice();
        for (const entry of graph.nodes) {
            const native = this.splitterParameters(entry.node);
            if (!native) continue;
            const item = { channel: args.channel, node: entry.name, label: entry.label, type: "Splitter", params: [], missing: [] };
            const count = this.branchCount(native.count);
            const names = ["numOutputs", "splitMode"];
            if (count !== null) {
                for (let i = 1; i <= count; i++) names.push("gain[" + i + "]", "mute[" + i + "]");
                for (let i = 1; i < count; i++) names.push("splitFreq[" + i + "]");
            } else item.error = "the native Splitter branch count is outside the verified range 2..5";
            for (const name of names) {
                const param = name === "numOutputs" ? native.count : name === "splitMode" ? native.mode : this.parameter(entry.node, name);
                if (param) item.params.push(this.info(param));
                else item.missing.push(name);
            }
            splitters.push(item);
        }
        return { channel: args.channel, splitters: splitters, scope: "top_level", truncated: graph.truncated, diagnostics: diagnostics };
    }

    setSplitter(args) {
        if (!args || typeof args.node !== "string" || !args.node || typeof args.label !== "string" || !args.label)
            return { error: "the native node name and expected Splitter label are required" };
        const name = args.param;
        const branch = typeof name === "string" ? /^(gain|mute)\[([1-5])\]$/.exec(name) : null;
        const frequency = typeof name === "string" ? /^splitFreq\[([1-4])\]$/.exec(name) : null;
        if (name !== "numOutputs" && name !== "splitMode" && !branch && !frequency) return { error: "unsupported Splitter parameter" };
        if (typeof args.value !== "number" || !isFinite(args.value)) return { error: "a finite numeric value is required" };
        const graph = this.graphNodes(args);
        if (graph.error) return graph;
        if (graph.truncated || graph.diagnostics.length) return { error: "the complete bounded insert graph is required to verify a unique node" };
        const matches = graph.nodes.filter(entry => entry.name === args.node);
        if (matches.length !== 1) return { error: "the native node name is missing or ambiguous" };
        const entry = matches[0];
        if (entry.label !== args.label) return { error: "the expected Splitter label does not match the native node" };
        const native = this.splitterParameters(entry.node);
        if (!native) return { error: "the native node is not a verified Splitter" };
        const count = this.branchCount(native.count);
        if (count === null) return { error: "the native Splitter branch count is outside the verified range 2..5" };
        if ((branch && Number(branch[2]) > count) || (frequency && Number(frequency[1]) >= count)) return { error: "the requested Splitter branch is not active" };
        if ((name === "numOutputs" || name === "splitMode" || (branch && branch[1] === "mute")) && Math.floor(args.value) !== args.value)
            return { error: "this Splitter parameter requires an integer value" };
        if ((name === "numOutputs" && (args.value < 2 || args.value > 5)) || (name === "splitMode" && (args.value < 0 || args.value > 2))
            || (branch && branch[1] === "mute" && args.value !== 0 && args.value !== 1)) return { error: "the value is outside the verified Splitter range" };
        const param = name === "numOutputs" ? native.count : name === "splitMode" ? native.mode : this.parameter(entry.node, name);
        if (!param) return { error: "the native Splitter parameter is unavailable" };
        const before = this.info(param);
        if (before.error) return before;
        if (typeof before.min !== "number" || typeof before.max !== "number" || !isFinite(before.min) || !isFinite(before.max)
            || before.max < before.min || args.value < before.min || args.value > before.max) return { error: "the value is outside the native Splitter parameter range" };
        try {
            if (typeof param.setValue !== "function") return { error: "the native Splitter parameter has no verified setter" };
            if (before.value !== args.value) param.setValue(args.value, true);
        } catch (e) { return { error: "native Splitter setter failed; outcome unknown: " + String(e).slice(0, 512), before: before }; }
        const after = this.info(param);
        if (after.error) return { error: "native Splitter readback failed; outcome unknown", before: before };
        const actual = typeof after.value === "boolean" ? (after.value ? 1 : 0) : after.value;
        const tolerance = 0.000001 * (1 + Math.abs(args.value));
        const verified = typeof actual === "number" && isFinite(actual) && Math.abs(actual - args.value) <= tolerance;
        return { channel: args.channel, node: entry.name, label: entry.label, type: "Splitter", requested: args.value,
            before: before, after: after, verified: verified };
    }
}
