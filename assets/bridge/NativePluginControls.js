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

    numericEditRequest(request) {
        return Object.keys(request).length === 4
            && Object.keys(request).every(key => ["operation", "param", "value", "expected_instance"].indexOf(key) >= 0)
            && typeof request.param === "string" && !!request.param && request.param.length <= 512 && request.param.indexOf("\0") < 0
            && (request.value === 0 || request.value === 1)
            && typeof request.expected_instance === "string"
            && /^[0-9A-F]{8}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{12}$/.test(request.expected_instance)
            && request.expected_instance !== "00000000-0000-0000-0000-000000000000";
    }

    numericEditPhase(phase) {
        return ["prepared", "begin_dispatched", "value_dispatched", "end_dispatched", "closed_checked"].indexOf(phase) >= 0;
    }

    numericEditResult(result) {
        if (result.ok === false)
            return ["not_applied", "unknown"].indexOf(result.outcome) >= 0
                && (result.edit_phase === undefined || this.numericEditPhase(result.edit_phase))
                && (result.outcome !== "not_applied" || result.edit_phase === undefined || result.edit_phase === "prepared")
                && result.error && typeof result.error === "object" && !Array.isArray(result.error)
                && typeof result.error.code === "string" && typeof result.error.message === "string";
        return result.ok === true && result.outcome === "observed" && result.source === "dsp" && result.edit_phase === "closed_checked"
            && typeof result.verified === "boolean"
            && result.before && typeof result.before === "object" && !Array.isArray(result.before)
            && result.after && typeof result.after === "object" && !Array.isArray(result.after);
    }

    guiExact(value, fields) {
        return value && typeof value === "object" && !Array.isArray(value) && Object.keys(value).length === fields.length
            && fields.every(key => Object.prototype.hasOwnProperty.call(value, key));
    }

    guiInteger(value, min, max) {
        return typeof value === "number" && isFinite(value) && Math.floor(value) === value && value >= min && value <= max;
    }

    guiBitsNumber(bits, width) {
        if (typeof bits !== "string" || !(width === 32 ? /^[0-9A-F]{8}$/ : /^[0-9A-F]{16}$/).test(bits)) return NaN;
        const high = parseInt(bits.slice(0, 8), 16);
        const exponent = width === 32 ? (high >>> 23) & 255 : (high >>> 20) & 2047;
        const mantissa = width === 32 ? high & 8388607 : (high & 1048575) * 4294967296 + parseInt(bits.slice(8), 16);
        const maxExponent = width === 32 ? 255 : 2047;
        if (exponent === maxExponent) return mantissa ? NaN : (high >>> 31 ? -Infinity : Infinity);
        const fractionBits = width === 32 ? 23 : 52;
        const bias = width === 32 ? 127 : 1023;
        const magnitude = exponent === 0 ? mantissa * Math.pow(2, 1 - bias - fractionBits)
            : (1 + mantissa / Math.pow(2, fractionBits)) * Math.pow(2, exponent - bias);
        return high >>> 31 ? -magnitude : magnitude;
    }

    guiNumberEqual(left, right) {
        return left === right && (left !== 0 || 1 / left === 1 / right);
    }

    guiTypedScalar(typed, scalar) {
        if (typed === null) return scalar === null;
        if (!this.guiExact(typed, ["variant_tag", "data"]) || !this.guiInteger(typed.variant_tag, -32768, 32767)) return false;
        const data = typed.data;
        if (!data || typeof data !== "object" || Array.isArray(data)) return false;
        const kind = typed.variant_tag & 15;
        if (data.kind === "integer" && kind === 1) {
            if (this.guiExact(data, ["kind", "value"]))
                return this.guiInteger(data.value, -9007199254740991, 9007199254740991) && scalar === data.value;
            if (!this.guiExact(data, ["kind", "exact_decimal"]) || scalar !== null || typeof data.exact_decimal !== "string"
                || !/^-?[1-9][0-9]{0,18}$/.test(data.exact_decimal)) return false;
            const negative = data.exact_decimal.charAt(0) === "-";
            const digits = negative ? data.exact_decimal.slice(1) : data.exact_decimal;
            return Math.abs(Number(data.exact_decimal)) > 9007199254740991
                && (digits.length < 19 || digits <= (negative ? "9223372036854775808" : "9223372036854775807"));
        }
        if (data.kind === "float" && kind === 2)
            return this.guiExact(data, ["kind", "value", "bits_hex"]) && typeof data.value === "number" && isFinite(data.value)
                && this.guiNumberEqual(data.value, this.guiBitsNumber(data.bits_hex, 64)) && this.guiNumberEqual(scalar, data.value);
        if (data.kind === "text" && kind === 3)
            return this.guiExact(data, ["kind", "value"]) && typeof data.value === "string" && data.value.length <= 4096 && scalar === data.value;
        if (scalar !== null) return false;
        if (data.kind === "empty" && kind === 0) return this.guiExact(data, ["kind"]);
        if (data.kind === "opaque_object" && kind === 4)
            return this.guiExact(data, ["kind", "present"]) && typeof data.present === "boolean";
        if (data.kind === "non_finite_float" && kind === 2)
            return this.guiExact(data, ["kind", "bits_hex"]) && typeof data.bits_hex === "string"
                && /^[0-9A-F]{16}$/.test(data.bits_hex) && !isFinite(this.guiBitsNumber(data.bits_hex, 64));
        return data.kind === "unsupported" && kind > 4 && this.guiExact(data, ["kind", "tag"]) && data.tag === kind;
    }

    guiDescription(value) {
        return this.guiFixedDescription(value, "autofill");
    }

    guiFixedDescription(value, name) {
        const fields = ["name", "title", "tag", "value", "text", "min", "max", "default", "normalized", "enabled", "readOnly", "writable",
            "native_kind", "native_value", "native_min", "native_max", "native_default", "native_normalized", "native_support", "support", "display_parser"];
        if (["autofill", "learnMode", "learnInputPitch"].indexOf(name) < 0
            || !this.guiExact(value, fields) || value.name !== name || value.title !== name
            || !this.guiInteger(value.tag, -2147483648, 2147483647)
            || (value.text !== null && (typeof value.text !== "string" || value.text.length > 4096))
            || ["enabled", "readOnly", "writable"].some(key => typeof value[key] !== "boolean")
            || typeof value.native_kind !== "string"
            || !/^(Toggle|Integer|Float|String|List|Command|Color|Image|Segments|Scroll|TextModel|Unknown\(-?[0-9]{1,10}\))$/.test(value.native_kind)
            || typeof value.display_parser !== "string" || value.display_parser.length > 96
            || !/^(Unsupported|LiteralString|PlainNumeric|NativeFormatter \{ flags: -?[0-9]{1,10} \})$/.test(value.display_parser)
            || !this.guiExact(value.native_support, ["raw_number", "normalized", "display_text"])
            || !this.guiExact(value.support, ["raw", "normalized", "text"])
            || Object.keys(value.native_support).some(key => typeof value.native_support[key] !== "boolean")
            || Object.keys(value.support).some(key => typeof value.support[key] !== "boolean")) return false;
        for (const pair of [["native_value", "value"], ["native_min", "min"], ["native_max", "max"], ["native_default", "default"]])
            if (!this.guiTypedScalar(value[pair[0]], value[pair[1]])) return false;
        const normalized = value.native_normalized;
        if (!normalized || typeof normalized !== "object" || Array.isArray(normalized)) return false;
        if (normalized.kind === "not_applicable") {
            if (!this.guiExact(normalized, ["kind"]) || value.normalized !== null) return false;
        } else if (["float32", "invalid_float32", "non_finite_float32"].indexOf(normalized.kind) >= 0) {
            if (!this.guiExact(normalized, ["kind", "bits_hex"]) || typeof normalized.bits_hex !== "string" || !/^[0-9A-F]{8}$/.test(normalized.bits_hex)) return false;
            const number = this.guiBitsNumber(normalized.bits_hex, 32);
            if (normalized.kind === "float32" ? !isFinite(number) || !this.guiNumberEqual(number, value.normalized)
                : isFinite(number) || value.normalized !== null) return false;
        } else return false;
        const safe = [value.native_value, value.native_min, value.native_max, value.native_default]
            .every(typed => !typed || typed.data.exact_decimal === undefined);
        if (value.support.raw !== (safe && value.native_support.raw_number)
            || value.support.normalized !== (safe && value.native_support.normalized)
            || value.support.text !== (safe && value.native_support.display_text)) return false;
        const opaque = value.native_value && ["opaque_object", "unsupported", "non_finite_float"].indexOf(value.native_value.data.kind) >= 0;
        return value.writable === (!!safe && !opaque && (value.native_support.raw_number || value.native_support.normalized || value.native_support.display_text));
    }

    guiToggleProfile(value) {
        return this.guiToggleProfileNamed(value, "autofill");
    }

    guiToggleProfileNamed(value, name) {
        return this.guiFixedDescription(value, name) && value.native_kind === "Toggle" && value.enabled === true && value.readOnly === false
            && value.writable === true && value.support.raw === true && (value.value === 0 || value.value === 1)
            && value.min === 0 && value.max === 1 && [value.native_value, value.native_min, value.native_max].every(typed => typed
                && [1, 2].indexOf(typed.variant_tag) >= 0 && typed.variant_tag === value.native_value.variant_tag
                && (typed.data.kind === "integer" || typed.data.kind === "float"
                    && typed.data.bits_hex === (typed.data.value === 1 ? "3FF0000000000000" : "0000000000000000")));
    }

    guiEditRequest(request) {
        return this.guiExact(request, ["operation", "enabled", "expected_instance"])
            && typeof request.enabled === "boolean" && this.insertChainUid(request.expected_instance);
    }

    guiEditResult(result, request) {
        if (result.ok === false) {
            const fields = ["ok", "outcome", "error"];
            if (result.edit_phase !== undefined) fields.push("edit_phase");
            if (result.instance !== undefined) fields.push("instance");
            return this.guiExact(result, fields) && ["not_applied", "unknown"].indexOf(result.outcome) >= 0
                && (result.edit_phase === undefined || this.numericEditPhase(result.edit_phase))
                && (result.outcome !== "not_applied" || result.edit_phase === undefined || result.edit_phase === "prepared")
                && this.guiExact(result.error, ["code", "message"]) && typeof result.error.code === "string"
                && /^[a-zA-Z0-9_]{1,64}$/.test(result.error.code) && typeof result.error.message === "string" && result.error.message.length <= 512;
        }
        const fields = ["ok", "outcome", "operation", "instance", "binding_source", "logical_parameter", "receiver", "requested_enabled",
            "edit_phase", "gui_parameter", "gui_value_verified", "verification_scope", "persistence_verified", "effects_verified", "dsp_propagation_verified"];
        if (!this.guiExact(result, fields) || result.ok !== true || result.outcome !== "applied_unverified"
            || result.operation !== "edit_chorder_autofill" || result.binding_source !== "dsp"
            || result.logical_parameter !== "chorder.autofill" || result.receiver !== "editor_control"
            || result.requested_enabled !== request.enabled || result.edit_phase !== "closed_checked"
            || !this.guiExact(result.gui_parameter, ["name", "before", "after"]) || result.gui_parameter.name !== "autofill"
            || !this.guiToggleProfile(result.gui_parameter.before) || !this.guiToggleProfile(result.gui_parameter.after)
            || typeof result.gui_value_verified !== "boolean" || result.verification_scope !== "gui_receiver_current_value"
            || result.persistence_verified !== false || result.effects_verified !== false || result.dsp_propagation_verified !== false)
            return false;
        const before = result.gui_parameter.before, after = result.gui_parameter.after;
        const mutable = ["value", "text", "normalized", "native_value", "native_normalized"];
        if (Object.keys(before).some(key => mutable.indexOf(key) < 0 && JSON.stringify(before[key]) !== JSON.stringify(after[key]))) return false;
        if (before.native_value.variant_tag !== after.native_value.variant_tag) return false;
        const desired = request.enabled ? 1 : 0;
        const exact = after.value === desired && (after.native_value.data.kind !== "float"
            || after.native_value.data.bits_hex === (request.enabled ? "3FF0000000000000" : "0000000000000000"));
        return result.gui_value_verified === exact;
    }

    guiLearningChange(change) {
        if (this.guiExact(change, ["kind", "enabled"]) && change.kind === "learn_mode") return typeof change.enabled === "boolean";
        return this.guiExact(change, ["kind", "pitch"]) && change.kind === "input_pitch"
            && (change.pitch === null || this.guiInteger(change.pitch, 0, 127));
    }

    guiLearningRequest(request) {
        return this.insertChainUid(request.expected_instance)
            && (request.operation === "chorder_learning_get" ? this.guiExact(request, ["operation", "expected_instance"])
                : request.operation === "edit_chorder_model" && this.guiExact(request, ["operation", "expected_instance", "change"])
                    && this.guiLearningChange(request.change));
    }

    guiLearningInstance(instance) {
        return this.guiExact(instance, ["object_uid", "class_uid", "native_class", "object_id"])
            && this.insertChainUid(instance.object_uid) && instance.class_uid === "D2D9B002-12FC-4F1D-AFDA-70336D1DC108"
            && ["native_class", "object_id"].every(key => typeof instance[key] === "string" && instance[key].length <= 512);
    }

    guiLearningPitchProfile(value) {
        return this.guiFixedDescription(value, "learnInputPitch") && value.native_kind === "Integer" && value.tag === 6
            && value.enabled === true && value.readOnly === false && value.writable === true && value.support.raw === true
            && this.guiInteger(value.value, -1, 127) && value.min === -1 && value.max === 127 && value.default === -1
            && [value.native_value, value.native_min, value.native_max, value.native_default].every(typed => typed
                && typed.variant_tag === 1 && typed.data.kind === "integer" && this.guiExact(typed.data, ["kind", "value"]));
    }

    guiLearningResult(result, request) {
        const setter = request.operation === "edit_chorder_model";
        if (result.ok === false) {
            const fields = ["ok", "outcome", "error"];
            if (result.instance !== undefined) fields.push("instance");
            if (result.edit_phase !== undefined) fields.push("edit_phase");
            return this.guiExact(result, fields) && (result.outcome === "not_applied" || setter && result.outcome === "unknown")
                && (result.instance === undefined || this.guiLearningInstance(result.instance) && result.instance.object_uid === request.expected_instance)
                && (result.edit_phase === undefined || this.numericEditPhase(result.edit_phase))
                && (result.outcome !== "not_applied" || result.edit_phase === undefined || result.edit_phase === "prepared")
                && this.guiExact(result.error, ["code", "message"]) && typeof result.error.code === "string"
                && /^[a-zA-Z0-9_]{1,64}$/.test(result.error.code) && typeof result.error.message === "string" && result.error.message.length <= 512;
        }
        if (!this.guiLearningInstance(result.instance) || result.ok !== true || result.receiver !== "editor_controller"
            || result.storage_semantics !== "transient_learning_controls") return false;
        if (!setter) return this.guiExact(result, ["ok", "outcome", "operation", "instance", "receiver", "mutation_capability",
            "value_semantics", "binding", "coverage", "parameters", "storage_semantics"])
            && result.outcome === "observed" && result.operation === "chorder_learning_get" && result.mutation_capability === false
            && result.value_semantics === "sequential_snapshot"
            && this.guiExact(result.binding, ["editor_match", "autofill_same_gui", "controller_same_observer"])
            && result.binding.editor_match === "canonical_iunknown" && result.binding.autofill_same_gui === true
            && result.binding.controller_same_observer === true
            && this.guiExact(result.coverage, ["complete", "candidate_count", "visited_views", "max_views", "max_depth"])
            && result.coverage.complete === true && result.coverage.candidate_count === 1 && result.coverage.max_views === 256
            && result.coverage.max_depth === 32 && this.guiInteger(result.coverage.visited_views, 1, 256)
            && this.guiExact(result.parameters, ["learnMode", "learnInputPitch"])
            && this.guiFixedDescription(result.parameters.learnMode, "learnMode")
            && this.guiFixedDescription(result.parameters.learnInputPitch, "learnInputPitch");
        const mode = request.change.kind === "learn_mode", name = mode ? "learnMode" : "learnInputPitch";
        if (!this.guiExact(result, ["ok", "outcome", "operation", "instance", "binding_source", "receiver", "requested_change",
            "edit_phase", "gui_parameter", "gui_value_verified", "verification_scope", "persistence_verified", "effects_verified",
            "dsp_propagation_verified", "storage_semantics"]) || result.outcome !== "applied_unverified"
            || result.operation !== "edit_chorder_model" || result.binding_source !== "dsp" || result.edit_phase !== "closed_checked"
            || !this.guiLearningChange(result.requested_change)
            || result.requested_change.kind !== request.change.kind
            || (mode ? result.requested_change.enabled !== request.change.enabled : result.requested_change.pitch !== request.change.pitch)
            || !this.guiExact(result.gui_parameter, ["name", "before", "after"]) || result.gui_parameter.name !== name
            || typeof result.gui_value_verified !== "boolean" || result.verification_scope !== "gui_receiver_current_value"
            || result.persistence_verified !== false || result.effects_verified !== false || result.dsp_propagation_verified !== false) return false;
        const before = result.gui_parameter.before, after = result.gui_parameter.after;
        if (mode ? !this.guiToggleProfileNamed(before, name) || !this.guiToggleProfileNamed(after, name)
            : !this.guiLearningPitchProfile(before) || !this.guiLearningPitchProfile(after)) return false;
        const mutable = ["value", "text", "normalized", "native_value", "native_normalized"];
        if (Object.keys(before).some(key => mutable.indexOf(key) < 0 && JSON.stringify(before[key]) !== JSON.stringify(after[key]))
            || before.native_value.variant_tag !== after.native_value.variant_tag) return false;
        const desired = mode ? (request.change.enabled ? 1 : 0) : (request.change.pitch === null ? -1 : request.change.pitch);
        return result.gui_value_verified === (after.value === desired);
    }

    guiLearningFailure(error, setter, dispatched, parsed) {
        const result = { ok: false, outcome: setter && dispatched ? "unknown" : "not_applied",
            error: { code: "native_chorder_learning_failed", message: String(error).slice(0, 512) } };
        if (setter && parsed && this.numericEditPhase(parsed.edit_phase)) result.edit_phase = parsed.edit_phase;
        return result;
    }

    guiComparisonRequest(request) {
        return Object.keys(request).every(key => ["operation", "source", "param", "expected_instance", "compare_gui", "include_owner"].indexOf(key) >= 0)
            && request.compare_gui === true && request.source === "dsp" && request.param === "chorder.autofill"
            && this.insertChainUid(request.expected_instance)
            && (request.include_owner === undefined || typeof request.include_owner === "boolean");
    }

    guiObserverMetadata(value) {
        if (this.guiExact(value, ["status"]) && value.status === "absent") return true;
        if (!this.guiExact(value, ["status", "report"]) || value.status !== "observed") return false;
        const report = value.report;
        const text = value => typeof value === "string" && value.length <= 512;
        const optionalText = value => value === null || text(value);
        const optionalUid = value => value === null || this.insertChainUid(value);
        if (!this.guiExact(report, ["coverage", "scope", "mutation_capability", "complete", "limits", "nodes", "edges", "diagnostics"])
            || report.coverage !== "bounded_parameter_controller_metadata" || report.scope !== "parameter_controller_observation"
            || report.mutation_capability !== false || typeof report.complete !== "boolean"
            || !this.guiExact(report.limits, ["nodes", "depth", "ancestors", "children_per_node"])
            || report.limits.nodes !== 1 || report.limits.depth !== 0 || report.limits.ancestors !== 0 || report.limits.children_per_node !== 1
            || !Array.isArray(report.nodes) || report.nodes.length !== 1 || !Array.isArray(report.edges) || report.edges.length !== 0
            || !Array.isArray(report.diagnostics) || report.diagnostics.length > 3) return false;
        const node = report.nodes[0];
        if (!this.guiExact(node, ["index", "identity", "child_count", "declared_interfaces", "reflection"]) || node.index !== 0
            || (node.child_count !== null && !this.guiInteger(node.child_count, 0, 1000000))) return false;
        if (node.identity !== null && (!this.guiExact(node.identity, ["id", "object_uid", "class_uid"])
            || !text(node.identity.id) || !optionalUid(node.identity.object_uid) || !optionalUid(node.identity.class_uid))) return false;
        if ((node.identity === null) !== (node.child_count === null)) return false;
        if (node.declared_interfaces !== null && (!this.guiExact(node.declared_interfaces, ["preset_collector", "preset_mediator", "storable"])
            || Object.keys(node.declared_interfaces).some(key => typeof node.declared_interfaces[key] !== "boolean"))) return false;
        const reflection = node.reflection;
        const issues = ["no_object_interface", "type_cycle", "type_depth_limit", "type_limit", "method_limit", "property_limit"];
        if (!this.guiExact(reflection, ["available", "types", "issues"]) || typeof reflection.available !== "boolean"
            || !Array.isArray(reflection.types) || reflection.types.length > 16 || !Array.isArray(reflection.issues)
            || reflection.issues.length > 34 || reflection.issues.some(issue => issues.indexOf(issue) < 0)
            || (!reflection.available && (reflection.types.length !== 0 || reflection.issues.length !== 1 || reflection.issues[0] !== "no_object_interface"))
            || (reflection.available && (reflection.types.length === 0 || reflection.issues.indexOf("no_object_interface") >= 0))) return false;
        let methods = 0, properties = 0;
        for (const info of reflection.types) {
            if (!this.guiExact(info, ["name", "namespace", "class_uid", "flags", "methods", "properties"])
                || !text(info.name) || !optionalText(info.namespace) || !optionalUid(info.class_uid)
                || !this.guiInteger(info.flags, -2147483648, 2147483647) || !Array.isArray(info.methods) || !Array.isArray(info.properties)) return false;
            methods += info.methods.length;
            properties += info.properties.length;
            if (methods > 256 || properties > 256
                || info.methods.some(method => !this.guiExact(method, ["name", "arguments", "return_type"])
                    || !text(method.name) || !optionalText(method.arguments) || !optionalText(method.return_type))
                || info.properties.some(property => !this.guiExact(property, ["name", "data_type", "type_name", "has_type_info"])
                    || !text(property.name) || !this.guiInteger(property.data_type, -2147483648, 2147483647)
                    || !optionalText(property.type_name) || typeof property.has_type_info !== "boolean")) return false;
        }
        const seen = [];
        for (const diagnostic of report.diagnostics) {
            if (!diagnostic || ["ancestor_limit", "child_limit", "depth_limit"].indexOf(diagnostic.kind) < 0
                || seen.indexOf(diagnostic.kind) >= 0
                || !this.guiExact(diagnostic, diagnostic.kind === "ancestor_limit" ? ["kind"] : ["kind", "node"])
                || (diagnostic.kind !== "ancestor_limit" && diagnostic.node !== 0)) return false;
            seen.push(diagnostic.kind);
        }
        return report.complete === (report.diagnostics.length === 0 && reflection.issues.length === 0);
    }

    guiControllerProbe(probe) {
        const common = ["status", "scope", "mutation_capability", "value_semantics", "interfaces", "anchors", "observer_metadata"];
        const observed = probe && probe.status === "observed";
        if (!this.guiExact(probe, common.concat(observed ? ["binding", "parameters"] : ["reason"]))
            || probe.scope !== "fixed_gui_declared_observer_controller" || probe.mutation_capability !== false
            || probe.value_semantics !== "sequential_snapshot" || !this.guiExact(probe.interfaces, ["controller", "provider"])
            || !this.guiExact(probe.anchors, ["controller", "provider"]) || !this.guiObserverMetadata(probe.observer_metadata)) return false;
        const anchors = ["not_attempted", "controller_absent", "parameter_absent", "identity_mismatch", "same_parameter"];
        const stable = ["controller_absent", "parameter_absent", "identity_mismatch"];
        for (const route of ["controller", "provider"]) {
            const query = probe.interfaces[route], anchor = probe.anchors[route];
            if (!query || anchors.indexOf(anchor) < 0) return false;
            if (query.status === "failed") {
                if (!this.guiExact(query, ["status", "query_status"]) || !this.guiInteger(query.query_status, -2147483648, 2147483647)) return false;
            } else if (!this.guiExact(query, ["status"]) || ["not_attempted", "unsupported", "present"].indexOf(query.status) < 0) return false;
            if ((["not_attempted", "failed"].indexOf(query.status) >= 0 && anchor !== "not_attempted")
                || (query.status === "unsupported" && anchor !== "controller_absent")
                || (query.status === "present" && (anchor === "not_attempted" || route === "controller" && anchor === "controller_absent"))) return false;
        }
        const controller = probe.interfaces.controller.status, provider = probe.interfaces.provider.status;
        const left = probe.anchors.controller, right = probe.anchors.provider;
        if (!observed && probe.status === "unresolved" && probe.reason === "observer_absent")
            return controller === "not_attempted" && provider === "not_attempted" && probe.observer_metadata.status === "absent";
        if (probe.observer_metadata.status !== "observed" || controller === "not_attempted" || provider === "not_attempted") return false;
        const failed = controller === "failed" || provider === "failed";
        if (!observed) {
            if (probe.status !== "unresolved") return false;
            if (probe.reason === "interface_query_failed") return failed;
            if (failed) return false;
            if (probe.reason === "no_anchored_controller") return left !== "same_parameter" && right !== "same_parameter";
            if (probe.reason === "ambiguous_controller_routes") return left === "same_parameter" && right === "same_parameter";
            return probe.reason === "parameter_missing" && (left === "same_parameter" || right === "same_parameter");
        }
        if (failed || !this.guiExact(probe.binding, ["route", "autofill_same_gui", "controller_same_observer", "controller_same_direct_dsp"])
            || probe.binding.autofill_same_gui !== true || typeof probe.binding.controller_same_observer !== "boolean"
            || typeof probe.binding.controller_same_direct_dsp !== "boolean"
            || !this.guiExact(probe.parameters, ["learnMode", "learnInputPitch"])
            || !this.guiFixedDescription(probe.parameters.learnMode, "learnMode")
            || !this.guiFixedDescription(probe.parameters.learnInputPitch, "learnInputPitch")) return false;
        return probe.binding.route === "direct" && left === "same_parameter" && stable.indexOf(right) >= 0
            || probe.binding.route === "provider" && right === "same_parameter" && stable.indexOf(left) >= 0
            || probe.binding.route === "both" && left === "same_parameter" && right === "same_parameter";
    }

    guiComparisonReport(report, includeOwner) {
        const object = value => value && typeof value === "object" && !Array.isArray(value);
        const exact = (value, fields) => object(value) && Object.keys(value).length === fields.length
            && fields.every(key => Object.prototype.hasOwnProperty.call(value, key));
        const integer = (value, min, max) => typeof value === "number" && isFinite(value)
            && Math.floor(value) === value && value >= min && value <= max;
        const common = ["status", "scope", "parameter_name", "semantic_binding", "mutation_capability", "coverage"];
        if (!object(report) || report.scope !== "exact_device_named_control_candidate"
            || report.parameter_name !== "chorder.autofill" || report.semantic_binding !== "named_candidate_only"
            || report.mutation_capability !== false
            || !exact(report.coverage, ["complete", "candidate_count", "visited_views", "max_views", "max_depth"])
            || typeof report.coverage.complete !== "boolean" || report.coverage.max_views !== 128 || report.coverage.max_depth !== 32
            || !integer(report.coverage.visited_views, 0, 128) || !integer(report.coverage.candidate_count, 0, report.coverage.visited_views))
            return false;
        if (report.status === "unresolved") {
            const fields = common.concat(["reason"]);
            if (Object.prototype.hasOwnProperty.call(report, "walk_failure")) {
                fields.push("walk_failure");
                const failure = report.walk_failure;
                const stages = ["root", "control_query", "parameter_metadata", "control_identity", "children", "iterator",
                    "child_query", "view_identity", "edge", "cleanup", "context"];
                if (report.reason !== "incomplete_walk" || report.coverage.complete !== false
                    || !exact(failure, ["kind", "stage", "depth", "query_status"])
                    || stages.indexOf(failure.stage) < 0 || !integer(failure.depth, 0, 32)) return false;
                if (failure.stage === "control_query" && report.coverage.visited_views <= failure.depth) return false;
                const query = ["interface_query_failed", "missing_interface"].indexOf(failure.kind) >= 0;
                if (query) {
                    if (["root", "control_query", "control_identity", "child_query", "view_identity"].indexOf(failure.stage) < 0
                        || !integer(failure.query_status, -2147483648, 2147483647)
                        || (failure.kind === "missing_interface" && failure.query_status !== -2147467262)) return false;
                } else if (failure.query_status !== null) return false;
                const valid = query || (failure.kind === "context_guard_failed" || failure.kind === "inspection_failed")
                    || (failure.kind === "view_limit" && failure.stage === "children" && report.coverage.visited_views === 128)
                    || (failure.kind === "depth_limit" && failure.stage === "children" && failure.depth === 32
                        && report.coverage.visited_views >= 33)
                    || (failure.kind === "repeated_view" && failure.stage === "view_identity" && report.coverage.visited_views >= 2)
                    || (failure.kind === "detached_view" && failure.stage === "edge" && report.coverage.visited_views >= 2)
                    || (failure.kind === "missing_iterator" && failure.stage === "iterator")
                    || (failure.kind === "missing_view" && (failure.stage === "root" || failure.stage === "iterator"))
                    || (failure.kind === "parameter_metadata" && failure.stage === "parameter_metadata");
                if (!valid) return false;
            }
            return exact(report, fields)
                && ["missing_editor", "ambiguous_editor", "missing_root_interface", "incomplete_walk", "no_exact_named_control",
                    "ambiguous_named_controls", "stale_association", "deadline", "inspection_failed"].indexOf(report.reason) >= 0;
        }
        const compared = common.concat(["topParameterSame", "declaredObserverSame", "declaredObserverPresence", "originalSame",
            "originalBasis", "originalUnknownReason", "editorMatch", "gui_parameter"]);
        if (includeOwner === true) compared.push("controller_probe");
        return report.status === "compared" && exact(report, compared)
            && (includeOwner !== true || this.guiControllerProbe(report.controller_probe))
            && this.guiDescription(report.gui_parameter)
            && report.coverage.complete === true && report.coverage.candidate_count === 1
            && typeof report.topParameterSame === "boolean"
            && ["both_present", "both_absent", "one_absent"].indexOf(report.declaredObserverPresence) >= 0
            && (report.declaredObserverPresence === "both_present" ? typeof report.declaredObserverSame === "boolean" : report.declaredObserverSame === null)
            && report.originalSame === null && report.originalBasis === "not_compared" && report.originalUnknownReason === "not_observed"
            && ["canonical_iunknown", "stable_node_uid"].indexOf(report.editorMatch) >= 0;
    }

    guiComparisonFailure(message, report) {
        const gui = report && report.status === "unresolved" && this.guiComparisonReport(report) ? report : {
            status: "unresolved", scope: "exact_device_named_control_candidate", parameter_name: "chorder.autofill",
            semantic_binding: "named_candidate_only", mutation_capability: false, reason: "inspection_failed",
            coverage: { complete: false, candidate_count: 0, visited_views: 0, max_views: 128, max_depth: 32 }
        };
        return { ok: false, outcome: "not_applied", native_outcome: "not_applied", mutation_capability: false,
            error: { code: "native_gui_comparison_failed", message: String(message).slice(0, 512) }, gui_comparison: gui };
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

    insertChainUid(value) {
        return typeof value === "string" && /^[0-9A-F]{8}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{12}$/.test(value)
            && value !== "00000000-0000-0000-0000-000000000000";
    }

    insertChainArguments(args) {
        return args && typeof args === "object" && !Array.isArray(args)
            && Object.keys(args).every(key => ["channel", "action", "token", "expected_folder"].indexOf(key) >= 0)
            && typeof args.channel === "string" && !!args.channel.trim() && args.channel.length <= 256
            && !/[\x00-\x1f\x7f-\x9f]/.test(args.channel)
            && ["capture", "restore", "release"].indexOf(args.action) >= 0
            && (args.action === "capture" ? !Object.prototype.hasOwnProperty.call(args, "token")
                : typeof args.token === "string" && /^[0-9A-F]{64}$/.test(args.token))
            && (args.expected_folder === undefined || this.insertChainUid(args.expected_folder));
    }

    insertChainResult(result, args) {
        const exact = (value, fields) => value && typeof value === "object" && !Array.isArray(value)
            && Object.keys(value).length === fields.length && fields.every(key => Object.prototype.hasOwnProperty.call(value, key));
        const integer = (value, min, max) => typeof value === "number" && isFinite(value)
            && Math.floor(value) === value && value >= min && value <= max;
        if (!result.ok)
            return exact(result, ["ok", "outcome", "error"]) && result.ok === false
                && ["not_applied", "unknown"].indexOf(result.outcome) >= 0
                && exact(result.error, ["code", "message"]) && typeof result.error.code === "string"
                && /^[a-zA-Z0-9_]{1,64}$/.test(result.error.code)
                && typeof result.error.message === "string" && result.error.message.length <= 512;
        const common = ["ok", "outcome", "scope", "format", "version", "folder", "verified", "effects_verified", "atomic_snapshot", "token"];
        const fields = args.action === "release" ? ["released"] : ["byte_count", "sha256", "part_count"];
        if (args.action === "restore") fields.push("load_accepted");
        return exact(result, common.concat(fields)) && result.ok === true
            && result.outcome === (args.action === "capture" ? "observed" : "applied_unverified")
            && result.scope === "insert_chain_state" && result.format === "host_multipreset_v1" && result.version === 1
            && result.verified === false && result.effects_verified === false && result.atomic_snapshot === false
            && exact(result.folder, ["object_uid", "class_uid", "native_class", "object_id"])
            && this.insertChainUid(result.folder.object_uid)
            && (result.folder.class_uid === null || this.insertChainUid(result.folder.class_uid))
            && result.folder.native_class === "AudioInsertFolder" && result.folder.object_id === "Inserts"
            && (args.expected_folder === undefined || result.folder.object_uid === args.expected_folder)
            && typeof result.token === "string" && /^[0-9A-F]{64}$/.test(result.token)
            && (args.action === "capture" || result.token === args.token)
            && (args.action === "release" ? result.released === true
                : integer(result.byte_count, 1, 16777216) && integer(result.part_count, 0, 128)
                    && typeof result.sha256 === "string" && /^[0-9A-Fa-f]{64}$/.test(result.sha256)
                    && (args.action !== "restore" || result.load_accepted === true));
    }

    insertChainRequest(args) {
        let dispatched = false;
        const failure = outcome => ({ ok: false, outcome: outcome, error: {
            code: "native_insert_chain_failed", message: "The guarded native insert-chain request did not complete" } });
        try {
            if (!this.insertChainArguments(args)) return failure("not_applied");
            const bridge = this.component.bridge;
            const before = bridge && bridge.requestContext;
            const finite = value => typeof value === "string" && value.length > 0 && value.length <= 128
                && !/[\x00-\x1f\x7f-\x9f]/.test(value);
            if (!before || !finite(before.session) || !finite(before.document) || !this.sameContext(before)
                || typeof bridge.mixerChannel !== "function") return failure("not_applied");
            const channel = bridge.mixerChannel(args.channel);
            if (!this.sameContext(before) || !channel || channel.bridgeError !== undefined
                || typeof channel.find !== "function") return failure("not_applied");
            const folder = channel.find("Inserts");
            if (!this.sameContext(before) || !folder || typeof folder !== "object") return failure("not_applied");
            if (typeof Host === "undefined" || !Host.Objects) return failure("not_applied");
            const objects = Host.Objects;
            let service = typeof objects.getObjectByName === "function" ? objects.getObjectByName("FenderMcpNative") : null;
            if (!this.sameContext(before)) return failure("not_applied");
            if (!service && typeof objects.getObjectByUrl === "function") service = objects.getObjectByUrl("object://FenderMcpNative/", true);
            if (!this.sameContext(before) || !service || typeof service.capabilities !== "function"
                || typeof service.request !== "function") return failure("not_applied");
            const capability = this.parse(service.capabilities());
            if (!this.sameContext(before) || !capability.ok || capability.api !== "fender-mcp-native-v1"
                || capability.insert_chain_api !== "guarded_folder_collection_v1") return failure("not_applied");
            const request = { operation: "insert_chain_" + args.action, bridge_session: before.session, document_epoch: before.document };
            if (args.expected_folder !== undefined) request.expected_folder = args.expected_folder;
            if (args.action !== "capture") request.token = args.token;
            let auxiliary;
            if (args.action === "capture") {
                if (typeof Host.Url !== "function") return failure("not_applied");
                auxiliary = Host.Url("");
                if (!auxiliary || typeof auxiliary !== "object") return failure("not_applied");
            }
            if (!this.sameContext(before)) return failure("not_applied");
            const body = JSON.stringify(request);
            dispatched = true;
            const result = this.parse(args.action === "capture" ? service.request(folder, body, auxiliary) : service.request(folder, body));
            if (!this.sameContext(before) || !this.insertChainResult(result, args)) return failure("unknown");
            return result.ok ? result : failure(result.outcome);
        } catch (_) {
            return failure(dispatched ? "unknown" : "not_applied");
        }
    }

    request(device, request, auxiliary) {
        let dispatched = false;
        let parsed = null;
        const guiComparison = request.operation === "parameter_details" && request.compare_gui === true;
        const guiEdit = request.operation === "edit_chorder_autofill";
        const guiLearning = request.operation === "chorder_learning_get" || request.operation === "edit_chorder_model";
        const guiLearningSet = request.operation === "edit_chorder_model";
        try {
            if (request.operation === "parameter_details" && request.compare_gui !== undefined
                && (typeof request.compare_gui !== "boolean" || (guiComparison && !this.guiComparisonRequest(request))))
                throw new Error("invalid finite native GUI comparison request");
            if (guiEdit && !this.guiEditRequest(request)) throw new Error("invalid finite native Chorder GUI edit request");
            if (guiLearning && !this.guiLearningRequest(request)) throw new Error("invalid fixed Chorder learning request");
            if (request.operation === "model_catalog" && !this.modelRequest(request)) throw new Error("invalid finite native model request");
            if (request.operation === "edit_numeric_parameter" && !this.numericEditRequest(request)) throw new Error("invalid finite native edit request");
            const needsAuxiliary = request.operation === "inspect_aux" || (request.operation === "presets" && request.catalog_token === undefined);
            if (needsAuxiliary ? !auxiliary || typeof auxiliary !== "object" : auxiliary !== undefined)
                throw new Error("native auxiliary object is required only for initial presets or fixed auxiliary inspection");
            const before = this.component.bridge && this.component.bridge.requestContext;
            if (!this.sameContext(before)) throw new Error("native request requires the current bridge document lease");
            if (typeof Host === "undefined" || !Host.Objects) {
                if (guiLearning) throw new Error("native Chorder learning extension unavailable");
                return guiComparison ? this.guiComparisonFailure("native GUI comparison requires the matching native extension") : null;
            }
            const objects = Host.Objects;
            let service = typeof objects.getObjectByName === "function" ? objects.getObjectByName("FenderMcpNative") : null;
            if (!service && typeof objects.getObjectByUrl === "function") service = objects.getObjectByUrl("object://FenderMcpNative/", true);
            if (!service) {
                if (guiLearning) throw new Error("native Chorder learning extension unavailable");
                return guiComparison ? this.guiComparisonFailure("native GUI comparison requires the matching native extension") : null;
            }
            if (typeof service.capabilities !== "function" || typeof service.request !== "function") throw new Error("native methods unavailable");
            const capability = this.parse(service.capabilities());
            if (!capability.ok || capability.api !== "fender-mcp-native-v1") throw new Error("unsupported native API");
            if (guiEdit && capability.gui_edit_api !== "chorder_autofill_v1") throw new Error("unsupported native Chorder GUI edit API");
            if (guiLearning && capability.gui_model_edit_api !== "chorder_learning_controls_v1") throw new Error("unsupported native Chorder learning API");
            if (request.operation === "edit_numeric_parameter" && capability.numeric_edit_api !== "chorder_autofill_v1") throw new Error("unsupported native numeric edit API");
            if (!this.sameContext(before)) throw new Error("native request requires the current open document");
            const body = JSON.stringify(request);
            if (body.length > 16384) throw new Error("native request exceeds the supported size");
            dispatched = true;
            const result = this.parse(needsAuxiliary ? service.request(device, body, auxiliary) : service.request(device, body));
            parsed = result;
            if (!this.sameContext(before)) throw new Error("document changed during native request");
            if (guiComparison) {
                if (!result.ok) {
                    if (result.outcome !== "not_applied") throw new Error("invalid native GUI read failure outcome");
                    return this.guiComparisonFailure(result.error && result.error.message || "native GUI comparison unavailable", result.gui_comparison);
                }
                if (result.ok !== true || result.outcome !== "observed" || result.source !== "dsp"
                    || (result.error !== undefined && result.error !== null) || (result.native_error !== undefined && result.native_error !== null)
                    || (result.errors !== undefined && (!Array.isArray(result.errors) || result.errors.length !== 0))
                    || (result.native_outcome !== undefined && result.native_outcome !== "observed")
                    || (result.mutation_capability !== undefined && result.mutation_capability !== false)
                    || result.verified !== undefined || result.effects_verified !== undefined || result.token !== undefined
                    || !result.instance || result.instance.class_uid !== "D2D9B002-12FC-4F1D-AFDA-70336D1DC108"
                    || !this.guiComparisonReport(result.gui_comparison, request.include_owner))
                    throw new Error("invalid native GUI comparison result");
            }
            if (guiEdit && (!this.guiEditResult(result, request) || (result.ok === true
                && (!result.instance || result.instance.class_uid !== "D2D9B002-12FC-4F1D-AFDA-70336D1DC108"))))
                throw new Error("invalid native Chorder GUI edit result");
            if (guiLearning && !this.guiLearningResult(result, request)) throw new Error("invalid native Chorder learning result");
            if (request.operation === "edit_numeric_parameter" && !this.numericEditResult(result)) throw new Error("invalid native numeric edit result");
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
            if (guiLearning) return this.guiLearningFailure(error, guiLearningSet, dispatched, parsed);
            if (guiComparison) return this.guiComparisonFailure(error);
            if (guiEdit) {
                const failure = { ok: false, outcome: dispatched ? "unknown" : "not_applied", gui_value_verified: false,
                    persistence_verified: false, effects_verified: false, dsp_propagation_verified: false,
                    error: { code: "native_gui_edit_failed", message: String(error).slice(0, 512) } };
                if (parsed && this.numericEditPhase(parsed.edit_phase)) failure.edit_phase = parsed.edit_phase;
                return failure;
            }
            const failure = { ok: false, outcome: dispatched ? "unknown" : "not_applied", error: { code: "native_request_failed", message: String(error).slice(0, 512) },
                before: parsed && parsed.before, after: parsed && parsed.after, instance: parsed && parsed.instance, source: parsed && parsed.source };
            if (request.operation === "edit_numeric_parameter" && parsed && this.numericEditPhase(parsed.edit_phase)) failure.edit_phase = parsed.edit_phase;
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
