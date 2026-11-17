"""Independently reconstruct native parameter digests from complete public pages.

This module performs no DAW calls. Its input is a complete ProviderSnapshot (or
records/source/instance mapping) already bound to one document by the caller.
Hashes cover sequential exposed observations, never opaque plugin contents.
"""
import hashlib
from copy import deepcopy
import json
import math
import re
import struct


FORMAT = "fsp-parameter-digest-v1"
MAX_COUNT = 8192
MAX_SAFE_INTEGER = 9_007_199_254_740_991
KINDS = frozenset(("Toggle", "Integer", "Float", "String", "List", "Command",
                   "Color", "Image", "Segments", "Scroll", "TextModel"))
HASH_FIELDS = ("schema_sha256", "all_values_sha256", "non_read_only_values_sha256")
FIELDS_COVERED = {
    "schema": ["index", "canonical_name", "dsp.index/name/tag/title/folder/raw_flags",
               "parameter.name/tag/kind/enabled/read_only/writable",
               "parameter.min/max/default.typed_exact", "parameter.support/native_support",
               "parameter.display_parser/formatter_flags"],
    "values": ["index", "raw.variant_tag/data", "normalized.kind/bits_hex", "display"],
    "integer_encoding": "exact_decimal",
    "float_encoding": "IEEE754_uppercase_hex_bits",
    "opaque_values": "variant_tag/type/presence_only",
    "non_read_only_exclusion": "read_only=true AND writable=false",
}


class DigestProjectionError(ValueError):
    pass


def _require(condition, message):
    if not condition:
        raise DigestProjectionError(message)


def _integer(value, lower, upper, field):
    _require(type(value) is int and lower <= value <= upper, "Invalid integer " + field)
    return value


def _text(value, field, *, name=False):
    _require(isinstance(value, str), "Missing text " + field)
    try:
        encoded = value.encode("utf-8")
        units = len(value.encode("utf-16-le")) // 2
    except UnicodeEncodeError as error:
        raise DigestProjectionError("Invalid Unicode " + field) from error
    _require(units <= 4096, "Oversized text " + field)
    if name:
        _require(value and len(encoded) <= 512 and "\0" not in value, "Invalid native name " + field)
    return value


def _keys(value, required, field):
    _require(isinstance(value, dict) and set(value) == set(required), "Invalid typed fields " + field)


def _bits(value, width, field):
    _require(isinstance(value, str) and len(value) == width
             and all(char in "0123456789abcdefABCDEF" for char in value), "Invalid IEEE bits " + field)
    return value.upper()


def _finite_number(value, field):
    finite = type(value) in (int, float)
    if finite:
        try:
            finite = math.isfinite(value)
        except OverflowError:
            finite = False
    _require(finite, "Invalid finite number " + field)
    return value


def _ulp_distance(left, right):
    if left == right:
        return 0

    def ordered(value):
        word = struct.unpack(">Q", struct.pack(">d", float(value)))[0]
        return ~word & ((1 << 64) - 1) if word >> 63 else word | (1 << 63)

    return abs(ordered(left) - ordered(right))


def _typed(value, *, extrema=False, field="value"):
    if value is None:
        _require(extrema, "Missing exact raw value")
        return None
    _keys(value, ("variant_tag", "data"), field)
    tag = _integer(value["variant_tag"], -(1 << 15), (1 << 15) - 1, field + ".variant_tag")
    data = value["data"]
    _require(isinstance(data, dict), "Missing native data " + field)
    kind = data.get("kind")
    if kind == "empty":
        _keys(data, ("kind",), field)
        projected = {"kind": kind}
    elif kind == "integer":
        _require(("value" in data) != ("exact_decimal" in data), "Ambiguous native integer " + field)
        if "value" in data:
            _keys(data, ("kind", "value"), field)
            number = _integer(data["value"], -MAX_SAFE_INTEGER, MAX_SAFE_INTEGER, field)
            decimal = str(number)
        else:
            _keys(data, ("kind", "exact_decimal"), field)
            decimal = data["exact_decimal"]
            _require(isinstance(decimal, str) and len(decimal) <= 20
                     and re.fullmatch(r"-?(0|[1-9][0-9]*)", decimal) is not None,
                     "Invalid exact integer " + field)
            number = int(decimal)
            _require(str(number) == decimal and -(1 << 63) <= number < (1 << 63)
                     and abs(number) > MAX_SAFE_INTEGER, "Noncanonical exact integer " + field)
        projected = {"kind": kind, "exact_decimal": decimal}
    elif kind == "float":
        _keys(data, ("kind", "bits_hex", "value"), field)
        bits = _bits(data["bits_hex"], 16, field)
        number = struct.unpack(">d", bytes.fromhex(bits))[0]
        _require(math.isfinite(number), "Nonfinite bits in finite native value " + field)
        shadow = _finite_number(data["value"], field)
        _require(_ulp_distance(number, shadow) <= 1, "Native Float bits disagree with JSON shadow " + field)
        projected = {"kind": kind, "bits_hex": bits}
    elif kind == "non_finite_float":
        _keys(data, ("kind", "bits_hex"), field)
        bits = _bits(data["bits_hex"], 16, field)
        _require(not math.isfinite(struct.unpack(">d", bytes.fromhex(bits))[0]), "Finite bits tagged nonfinite " + field)
        projected = {"kind": kind, "bits_hex": bits}
    elif kind == "text":
        _keys(data, ("kind", "value"), field)
        projected = {"kind": kind, "value": _text(data["value"], field)}
    elif kind == "opaque_object":
        _keys(data, ("kind", "present"), field)
        _require(not extrema and type(data["present"]) is bool, "Incomparable opaque schema field " + field)
        projected = {"kind": kind, "present": data["present"]}
    elif kind == "unsupported":
        _keys(data, ("kind", "tag"), field)
        _require(not extrema, "Incomparable unsupported schema field " + field)
        projected = {"kind": kind, "tag": _integer(data["tag"], -(1 << 15), (1 << 15) - 1, field)}
    else:
        raise DigestProjectionError("Unknown native data kind " + field)
    return {"variant_tag": tag, "data": projected}


def _shadow(typed, shadow, field):
    if typed is None or typed["data"]["kind"] in ("empty", "non_finite_float", "opaque_object", "unsupported"):
        _require(shadow is None, "Native non-scalar and public value disagree " + field)
        return
    data = typed["data"]
    if data["kind"] == "integer":
        number = int(data["exact_decimal"])
        if abs(number) > MAX_SAFE_INTEGER:
            _require(shadow is None, "Unsafe native integer has a numeric public shadow " + field)
        else:
            _require(type(shadow) is int and shadow == number, "Native integer shadow differs " + field)
    elif data["kind"] == "float":
        number = struct.unpack(">d", bytes.fromhex(data["bits_hex"]))[0]
        _require(_ulp_distance(number, _finite_number(shadow, field)) <= 1, "Native Float shadow differs " + field)
    else:
        _require(isinstance(shadow, str) and shadow == data["value"], "Native text shadow differs " + field)


def _normalized(value, shadow):
    _require(isinstance(value, dict), "Missing typed native_normalized metadata")
    kind = value.get("kind")
    if kind == "not_applicable":
        _keys(value, ("kind",), "native_normalized")
        _require(shadow is None, "Non-applicable normalized value has a numeric shadow")
        return {"kind": kind}
    _keys(value, ("kind", "bits_hex"), "native_normalized")
    _require(kind in ("float32", "non_finite_float32"), "Invalid normalized native kind")
    bits = _bits(value["bits_hex"], 8, "native_normalized")
    number = struct.unpack(">f", bytes.fromhex(bits))[0]
    if kind == "float32":
        _require(math.isfinite(number), "Nonfinite normalized bits tagged finite")
        shadow = _finite_number(shadow, "normalized")
        try:
            shadow_bits = struct.pack(">f", shadow).hex().upper()
        except (OverflowError, struct.error) as error:
            raise DigestProjectionError("Normalized shadow is outside Float32") from error
        _require((shadow_bits == bits or shadow == number == 0) and _ulp_distance(number, shadow) <= 1,
                 "Normalized shadow differs from exact Float32 bits")
    else:
        _require(not math.isfinite(number) and shadow is None, "Invalid nonfinite normalized metadata")
    return {"kind": kind, "bits_hex": bits}


def _switches(value, names, field):
    _keys(value, names, field)
    _require(all(type(value[name]) is bool for name in names), "Invalid boolean flags " + field)
    return dict(value)


def _formatter(value):
    simple = {"Unsupported": "unsupported", "LiteralString": "literal_string", "PlainNumeric": "plain_numeric"}
    if isinstance(value, str) and value in simple:
        return {"kind": simple[value]}
    _require(isinstance(value, str), "Missing display parser")
    match = re.fullmatch(r"NativeFormatter \{ flags: (-?[0-9]+) \}", value)
    _require(match is not None and len(match[1]) <= 11, "Unknown display parser")
    return {"kind": "native_formatter", "flags": _integer(int(match[1]), -(1 << 31), (1 << 31) - 1, "formatter flags")}


def _record(record, source, index):
    _require(isinstance(record, dict), "Missing parameter record")
    if "readable" in record:
        _require(record["readable"] is True, "Unreadable parameter record")
    if "access_metadata_complete" in record:
        _require(record["access_metadata_complete"] is True, "Incomplete parameter access metadata")
    if source == "dsp":
        _require(record.get("access_metadata_complete") is True, "DSP metadata lacks direct native access proof")
        native = record.get("native")
        _require(isinstance(native, dict) and "access_error" not in native, "Missing DSP native metadata")
        descriptor = native.get("native_description")
        _require(isinstance(descriptor, dict), "Missing DSP native description")
        dsp = {name: native.get(name) for name in ("index", "name", "tag", "title", "folder", "raw_flags")}
        _integer(dsp["index"], index, index, "DSP index")
        _integer(dsp["tag"], 0, (1 << 32) - 1, "DSP tag")
        _integer(dsp["folder"], 0, (1 << 32) - 1, "DSP folder")
        _integer(dsp["raw_flags"], 0, (1 << 32) - 1, "DSP flags")
        _text(dsp["title"], "DSP title")
        canonical = _text(dsp["name"], "DSP name", name=True)
        _require(record.get("name") == canonical, "Canonical DSP name differs from public record")
        for key, value in descriptor.items():
            if key != "name":
                _require(key in record and record[key] == value, "Public/native descriptor disagreement: " + key)
    else:
        descriptor, dsp = record, None
        _integer(record.get("index"), index, index, "controller index")
        canonical = _text(record.get("name"), "controller name", name=True)
    kind = descriptor.get("native_kind")
    _require(isinstance(kind, str) and kind in KINDS, "Unknown native parameter kind")
    name = _text(descriptor.get("name"), "local name", name=True)
    _integer(descriptor.get("tag"), -(1 << 31), (1 << 31) - 1, "local tag")
    for flag in ("enabled", "readOnly", "writable"):
        _require(type(descriptor.get(flag)) is bool, "Missing authoritative " + flag)
    for field in ("native_value", "native_min", "native_max", "native_default", "native_support", "native_normalized",
                  "value", "normalized"):
        _require(field in descriptor, "Missing exact metadata " + field)
    raw = _typed(descriptor["native_value"])
    extrema = {key: _typed(descriptor["native_" + key], extrema=True, field=key) for key in ("min", "max", "default")}
    _shadow(raw, descriptor.get("value"), "value")
    for field, typed in extrema.items():
        _require(field in descriptor, "Missing public metadata " + field)
        _shadow(typed, descriptor[field], field)
    support = _switches(descriptor.get("support"), ("raw", "normalized", "text"), "support")
    native_support = _switches(descriptor["native_support"], ("raw_number", "normalized", "display_text"), "native_support")
    safe_integers = all(value is None or value["data"]["kind"] != "integer"
                        or abs(int(value["data"]["exact_decimal"])) <= MAX_SAFE_INTEGER
                        for value in (raw, *extrema.values()))
    expected_support = {public: safe_integers and native_support[native]
                        for public, native in (("raw", "raw_number"), ("normalized", "normalized"), ("text", "display_text"))}
    opaque = raw["data"]["kind"] in ("opaque_object", "unsupported")
    expected_writable = safe_integers and any(native_support.values()) and not opaque and raw["data"]["kind"] != "non_finite_float"
    _require(support == expected_support and descriptor["writable"] is expected_writable,
             "Public write support disagrees with exact native metadata")
    schema = {"index": index, "canonical_name": canonical, "dsp": dsp, "parameter": {
        "name": name, "tag": descriptor["tag"], "kind": kind, "enabled": descriptor["enabled"],
        "read_only": descriptor["readOnly"], "writable": descriptor["writable"], **extrema,
        "support": support, "native_support": native_support,
        "display_parser": _formatter(descriptor.get("display_parser")),
    }}
    values = {"index": index, "raw": raw,
              "normalized": _normalized(descriptor["native_normalized"], descriptor.get("normalized")),
              "display": _text(descriptor.get("text"), "display")}
    return schema, values, descriptor["readOnly"], descriptor["readOnly"] and not descriptor["writable"], opaque


def _canonical(value):
    try:
        return json.dumps(value, ensure_ascii=False, sort_keys=True, allow_nan=False,
                          separators=(",", ":")).encode("utf-8")
    except (ValueError, UnicodeEncodeError, TypeError) as error:
        raise DigestProjectionError("Projection cannot be encoded as canonical UTF-8 JSON") from error


def _frame(stream, payload):
    stream.update(struct.pack("<Q", len(payload)))
    stream.update(payload)


def _stream(name, source, count):
    result = hashlib.sha256()
    for payload in (FORMAT.encode(), name.encode(), source.encode(), struct.pack("<I", count)):
        _frame(result, payload)
    return result


def _combined(name, source, count, schema, values):
    stream = _stream(name, source, count)
    _frame(stream, schema)
    _frame(stream, values)
    return stream.hexdigest()


def _snapshot(snapshot):
    def contains(field):
        return field in snapshot if isinstance(snapshot, dict) else hasattr(snapshot, field)

    def get(field):
        return snapshot.get(field) if isinstance(snapshot, dict) else getattr(snapshot, field, None)

    if isinstance(snapshot, dict):
        source, instance, records = (snapshot.get(key) for key in ("source", "instance", "records"))
    else:
        source, instance, records = (getattr(snapshot, key, None) for key in ("source", "instance", "records"))
    _require(source in ("dsp", "controller"), "Use an explicit indexed native source")
    _require(isinstance(instance, dict) and all(isinstance(instance.get(key), str) and instance[key]
                                             for key in ("object_uid", "class_uid")), "Missing native instance binding")
    _require(isinstance(records, list) and len(records) <= MAX_COUNT, "Expected a bounded complete native record list")
    for field in ("count", "reported_count"):
        if contains(field):
            _integer(get(field), len(records), len(records), field)
    for field in ("access_complete", "access_metadata_complete", "page_complete", "all_indices_read"):
        if contains(field):
            _require(get(field) is True, "Incomplete serialized snapshot " + field)
    for field in ("failed_indices", "failed_access_indices"):
        if contains(field):
            _require(get(field) == [], "Failed serialized snapshot " + field)
    return source, deepcopy(instance), records


def project_snapshot(snapshot):
    """Hash one already complete, bound public snapshot without native calls."""
    source, instance, records = _snapshot(snapshot)
    count = len(records)
    schema = _stream("schema", source, count)
    all_values = _stream("all_value_records", source, count)
    non_read_only = _stream("non_read_only_value_records", source, count)
    names = set()
    read_only_count = readonly_observation_count = opaque_count = 0
    for index, record in enumerate(records):
        metadata, values, read_only, readonly_observation, opaque = _record(record, source, index)
        _require(metadata["canonical_name"] not in names, "Duplicate canonical native name")
        names.add(metadata["canonical_name"])
        _frame(schema, _canonical(metadata))
        encoded = _canonical(values)
        _frame(all_values, encoded)
        if not readonly_observation:
            _frame(non_read_only, encoded)
        read_only_count += read_only
        readonly_observation_count += readonly_observation
        opaque_count += opaque
    schema_bytes = schema.digest()
    return {
        "hash_format_version": FORMAT, "hash_algorithm": "sha256", "hash_scope": "exposed_parameter_projection",
        "coverage": "fresh_sequential_indexed_native_parameter_observations", "atomic_snapshot": False,
        "source": source, "instance": instance, "count": count, "readable_count": count,
        "read_only_count": read_only_count, "readonly_observation_count": readonly_observation_count,
        "non_read_only_count": count - readonly_observation_count, "excluded_opaque_count": opaque_count,
        "opaque_contents_verified": False, "opaque_exclusion_scope": "contents_only_records_remain_hashed",
        "access_complete": True, "max_count": MAX_COUNT, "fields_covered": deepcopy(FIELDS_COVERED),
        "schema_sha256": schema_bytes.hex(),
        "all_values_sha256": _combined("all_values", source, count, schema_bytes, all_values.digest()),
        "non_read_only_values_sha256": _combined("non_read_only_values", source, count, schema_bytes, non_read_only.digest()),
    }


def validate_native_digest(snapshot, native):
    """Require complete native parity with the independently serialized snapshot.

    The caller must also validate the outer receipt's document/session lease.
    This comparison does not turn a sequence of observations into atomic state.
    """
    projected = project_snapshot(snapshot)
    _require(isinstance(native, dict) and native.get("ok") is True and native.get("outcome") == "observed"
             and "error" not in native, "Native digest is not a successful observed result")
    for key, expected in projected.items():
        if key in HASH_FIELDS:
            _require(isinstance(native.get(key), str) and re.fullmatch(r"[0-9a-f]{64}", native[key]) is not None,
                     "Missing or malformed native " + key)
        actual = native.get(key)
        _require(type(actual) is type(expected) and actual == expected, "Native digest differs from serialized projection: " + key)
    return {**projected, "native_digest_parity_verified": True}
