// The host marshals the already resolved device; JSON carries only finite data.
class NativePluginControls {
    constructor(component) { this.component = component; }

    parse(text) {
        if (typeof text !== "string" || text.length > 262144) throw new Error("invalid native response size");
        const value = JSON.parse(text);
        if (!value || typeof value !== "object" || Array.isArray(value) || typeof value.ok !== "boolean") throw new Error("invalid native response object");
        return value;
    }

    context() {
        const bridge = this.component.bridge;
        return bridge && typeof bridge.context === "function" ? bridge.context() : null;
    }

    sameContext(before) {
        if (!before || before.open !== true || typeof before.document !== "string" || !before.document || typeof before.session !== "string" || !before.session) return false;
        const after = this.context();
        return !!after && before.open === true && after.open === true && before.document === after.document && before.session === after.session;
    }

    modelRequest(request) {
        const fields = ["operation", "catalog", "offset", "limit", "expected_instance"];
        const integer = (value, min, max) => typeof value === "number" && isFinite(value) && Math.floor(value) === value && value >= min && value <= max;
        return Object.keys(request).every(key => fields.indexOf(key) >= 0)
            && (request.catalog === "categories" || request.catalog === "effects")
            && integer(request.offset, 0, 4096) && integer(request.limit, 1, 32)
            && typeof request.expected_instance === "string"
            && /^[0-9A-F]{8}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{12}$/.test(request.expected_instance)
            && request.expected_instance !== "00000000-0000-0000-0000-000000000000";
    }

    modelResult(result, request) {
        const fields = ["ok", "outcome", "catalog", "model_name", "count", "offset", "rows", "next_offset", "complete",
            "all_flat_rows_observed", "atomic_snapshot", "mutation_capability", "scope", "instance"];
        const integer = (value, min, max) => typeof value === "number" && isFinite(value) && Math.floor(value) === value && value >= min && value <= max;
        if (Object.keys(result).some(key => fields.indexOf(key) < 0)
            || result.outcome !== "observed" || result.catalog !== request.catalog
            || result.model_name !== (request.catalog === "categories" ? "categoryModel" : "effectListModel")
            || !integer(result.count, 0, 4096) || result.offset !== request.offset || result.offset > result.count
            || !Array.isArray(result.rows) || result.rows.length !== Math.min(request.limit, result.count - result.offset)
            || result.scope !== "current_selector_flat_model" || result.atomic_snapshot !== false || result.mutation_capability !== false
            || !result.instance || result.instance.object_uid !== request.expected_instance
            || ["11B8AA91-9BED-4A7F-90F4-02C6A6C7AF34", "B9D7536C-A010-457D-A182-36E5531164C9"].indexOf(result.instance.class_uid) < 0)
            return false;
        let units = 0;
        for (let i = 0; i < result.rows.length; i++) {
            const row = result.rows[i];
            if (!row || typeof row !== "object" || Array.isArray(row)
                || Object.keys(row).some(key => ["index", "title", "title_available"].indexOf(key) < 0)
                || row.index !== result.offset + i || typeof row.title !== "string" || row.title.length > 512
                || typeof row.title_available !== "boolean") return false;
            units += row.title.length;
        }
        const end = result.offset + result.rows.length;
        const complete = result.rows.every(row => row.title_available);
        return units <= 16384 && result.next_offset === (end < result.count ? end : null)
            && typeof result.complete === "boolean" && result.complete === complete
            && typeof result.all_flat_rows_observed === "boolean"
            && result.all_flat_rows_observed === (complete && result.offset === 0 && end === result.count);
    }

    request(device, request, auxiliary) {
        let dispatched = false;
        let parsed = null;
        try {
            if (request.operation === "model_catalog" && !this.modelRequest(request)) throw new Error("invalid finite native model request");
            const needsAuxiliary = request.operation === "inspect_aux" || (request.operation === "presets" && request.catalog_token === undefined);
            if (needsAuxiliary ? !auxiliary || typeof auxiliary !== "object" : auxiliary !== undefined)
                throw new Error("native auxiliary object is required only for initial presets or fixed auxiliary inspection");
            const before = this.component.bridge && this.component.bridge.requestContext;
            if (!this.sameContext(before)) throw new Error("native request requires the current bridge document lease");
            if (typeof Host === "undefined" || !Host.Objects) return null;
            const objects = Host.Objects;
            let service = typeof objects.getObjectByName === "function" ? objects.getObjectByName("FenderMcpNative") : null;
            if (!service && typeof objects.getObjectByUrl === "function") service = objects.getObjectByUrl("object://FenderMcpNative/", true);
            if (!service) return null;
            if (typeof service.capabilities !== "function" || typeof service.request !== "function") throw new Error("native methods unavailable");
            const capability = this.parse(service.capabilities());
            if (!capability.ok || capability.api !== "fender-mcp-native-v1") throw new Error("unsupported native API");
            if (!this.sameContext(before)) throw new Error("native request requires the current open document");
            const body = JSON.stringify(request);
            if (body.length > 16384) throw new Error("native request exceeds the supported size");
            dispatched = true;
            const result = this.parse(needsAuxiliary ? service.request(device, body, auxiliary) : service.request(device, body));
            parsed = result;
            if (!this.sameContext(before)) throw new Error("document changed during native request");
            if (result.ok && (!result.instance || typeof result.instance.object_uid !== "string" || !result.instance.object_uid || typeof result.instance.class_uid !== "string" || !result.instance.class_uid)) throw new Error("native target identity unavailable");
            if (result.ok && request.expected_instance !== undefined && result.instance.object_uid !== request.expected_instance) throw new Error("native target instance changed");
            if (result.ok && request.operation === "model_catalog" && !this.modelResult(result, request)) throw new Error("invalid native current-selector model list contract");
            if (result.ok && request.operation === "set_parameter" && (typeof result.verified !== "boolean" || !result.before || typeof result.before !== "object" || !result.after || typeof result.after !== "object")) throw new Error("native mutation readback unavailable");
            if (result.ok && request.operation === "parameter_action"
                && (result.action !== request.action || result.source !== "dsp" || result.outcome !== "applied_unverified"
                    || result.verified !== false || result.effects_verified !== false || result.upper_dispatched !== true
                    || result.reset_dispatched !== true || result.reset_verified !== true
                    || !result.before || typeof result.before !== "object" || !result.after || typeof result.after !== "object"
                    || !Array.isArray(result.effect_parameters) || result.effect_parameters.length !== 2
                    || result.effect_parameters.some(name => typeof name !== "string" || !name || name.length > 512)))
                throw new Error("native action phase evidence unavailable");
            return result;
        } catch (error) {
            const failure = { ok: false, outcome: dispatched ? "unknown" : "not_applied", error: { code: "native_request_failed", message: String(error).slice(0, 512) },
                before: parsed && parsed.before, after: parsed && parsed.after, instance: parsed && parsed.instance, source: parsed && parsed.source };
            if (request.operation === "parameter_action") {
                failure.action = request.action;
                failure.effects_verified = false;
                failure.verified = false;
                failure.reset_verified = false;
                for (const phase of ["upper_dispatched", "reset_dispatched"])
                    if (parsed && typeof parsed[phase] === "boolean") failure[phase] = parsed[phase];
            }
            return failure;
        }
    }
}
