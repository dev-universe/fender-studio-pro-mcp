"""Synthetic independent parity tests; no DAW, native calls or private fixtures."""
from copy import deepcopy
from dataclasses import dataclass
import hashlib
import json
import math
import struct
import unittest

import parameter_digest as digest


INSTANCE = {"object_uid": "11111111-2222-3333-4444-555555555555",
            "class_uid": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            "native_class": "SyntheticDevice", "object_id": "synthetic:1"}


def typed(kind, *, tag=2, **data):
    return {"variant_tag": tag, "data": {"kind": kind, **data}}


def floating(value):
    return typed("float", value=value, bits_hex=struct.pack(">d", value).hex().upper())


def integer(value):
    return typed("integer", tag=1, **({"value": value} if abs(value) <= digest.MAX_SAFE_INTEGER
                                      else {"exact_decimal": str(value)}))


def scalar(value):
    if value is None:
        return None
    return value["data"].get("value")


def normalized(value):
    if value is None:
        return {"kind": "not_applicable"}, None
    bits = struct.pack(">f", value)
    return {"kind": "float32", "bits_hex": bits.hex().upper()}, struct.unpack(">f", bits)[0]


def record(index=0, *, read_only=False):
    """The native MockReader observation fixture, independently represented."""
    norm, shadow = normalized(0.1)
    return {"index": index, "name": "parameter" + str(index), "title": "parameter" + str(index),
            "tag": index, "native_kind": "Float", "enabled": True, "readOnly": read_only,
            "writable": not read_only, "native_value": floating(1.0), "value": 1.0,
            "native_min": floating(0.0), "min": 0.0, "native_max": floating(10.0), "max": 10.0,
            "native_default": floating(1.0), "default": 1.0, "native_normalized": norm,
            "normalized": shadow, "text": "one", "display_parser": "NativeFormatter { flags: 0 }",
            "native_support": {"raw_number": not read_only, "normalized": not read_only,
                               "display_text": not read_only},
            "support": {"raw": not read_only, "normalized": not read_only, "text": not read_only}}


def snapshot(records=None, source="controller"):
    return {"source": source, "instance": deepcopy(INSTANCE), "records": records if records is not None else [record()]}


def dsp_record(index=0):
    description = record(index)
    del description["index"]
    native = {"index": index, "name": "canonical.parameter" + str(index), "tag": 900 + index,
              "title": "Canonical title", "folder": 7, "raw_flags": 42,
              "native_description": description}
    return {**deepcopy(description), "name": native["name"], "native": native,
            "access_metadata_complete": True}


def replace_value(item, value):
    item["native_value"], item["value"] = value, scalar(value)
    if value["data"]["kind"] in ("opaque_object", "unsupported", "non_finite_float"):
        item["writable"] = False


def no_support(item):
    item["native_support"] = dict.fromkeys(("raw_number", "normalized", "display_text"), False)
    item["support"] = dict.fromkeys(("raw", "normalized", "text"), False)
    item["writable"] = False


def independent_hashes(schemas, values, include, source):
    """Build framed bytes outright, independently of the streaming projector."""
    def encode(value):
        return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False).encode()

    def hashed(name, body):
        parts = [b"fsp-parameter-digest-v1", name.encode(), source.encode(), len(schemas).to_bytes(4, "little"), *body]
        return hashlib.sha256(b"".join(len(part).to_bytes(8, "little") + part for part in parts)).digest()

    schema = hashed("schema", list(map(encode, schemas)))
    all_values = hashed("all_value_records", list(map(encode, values)))
    selected = hashed("non_read_only_value_records", [encode(value) for value, keep in zip(values, include) if keep])
    return {"schema_sha256": schema.hex(), "all_values_sha256": hashed("all_values", [schema, all_values]).hex(),
            "non_read_only_values_sha256": hashed("non_read_only_values", [schema, selected]).hex()}


def native_mock_projection(index, read_only):
    """Explicit native digest schema, without passing through public conversion."""
    def exact(bits):
        return {"variant_tag": 2, "data": {"kind": "float", "bits_hex": bits}}

    return ({"index": index, "canonical_name": "parameter" + str(index), "dsp": None,
             "parameter": {"name": "parameter" + str(index), "tag": index, "kind": "Float",
                           "enabled": True, "read_only": read_only, "writable": not read_only,
                           "min": exact("0000000000000000"), "max": exact("4024000000000000"),
                           "default": exact("3FF0000000000000"),
                           "support": {"raw": not read_only, "normalized": not read_only, "text": not read_only},
                           "native_support": {"raw_number": not read_only, "normalized": not read_only,
                                              "display_text": not read_only},
                           "display_parser": {"kind": "native_formatter", "flags": 0}}},
            {"index": index, "raw": exact("3FF0000000000000"),
             "normalized": {"kind": "float32", "bits_hex": "3DCCCCCD"}, "display": "one"})


class ProjectionTests(unittest.TestCase):
    def assert_rejected(self, value):
        with self.assertRaises(digest.DigestProjectionError):
            digest.project_snapshot(value)

    def test_native_framing_golden_with_unicode_and_nul(self):
        stream = digest._stream("schema", "dsp", 1)
        digest._frame(stream, digest._canonical({"z": [1, 2], "a": "μ\0"}))
        self.assertEqual(stream.hexdigest(), "3f4a4960cf26fc1f39122116e5afc39b8cdb36114fd707ac4eec36e14ef30cf3")

    def test_native_mock_projection_matches_independently_encoded_records(self):
        first, second = native_mock_projection(0, False), native_mock_projection(1, True)
        expected = independent_hashes([first[0], second[0]], [first[1], second[1]], [True, False], "controller")
        self.assertEqual(expected, {
            "schema_sha256": "2bcc84fb939d394a17d8b8bf0ce78375c5b41469b275fe713eacfee85888e1a3",
            "all_values_sha256": "c024aa60b246495cf18c7b12085ce87f06d7a6ff855a806ad3dd9bba7abd6c94",
            "non_read_only_values_sha256": "fd2402eafb23fd7c3bf1db2a19fd6b288f484baa5334bdc3437ab7fd6f75eefd",
        })
        actual = digest.project_snapshot(snapshot([record(0), record(1, read_only=True)]))
        self.assertEqual({key: actual[key] for key in digest.HASH_FIELDS}, expected)
        self.assertEqual((actual["count"], actual["readable_count"], actual["read_only_count"],
                          actual["readonly_observation_count"], actual["non_read_only_count"]), (2, 2, 1, 1, 1))

    def test_mapping_and_provider_snapshot_shapes_are_equivalent(self):
        @dataclass
        class ProviderSnapshot:
            source: str
            instance: dict
            records: list

        value = snapshot()
        self.assertEqual(digest.project_snapshot(value), digest.project_snapshot(ProviderSnapshot(**value)))

    def test_empty_provider_is_explicitly_bounded_and_hashable(self):
        result = digest.project_snapshot(snapshot([]))
        expected = independent_hashes([], [], [], "controller")
        self.assertEqual({key: result[key] for key in digest.HASH_FIELDS}, expected)
        self.assertEqual(result["count"], 0)

    def test_repeat_and_json_key_order_do_not_change_hashes(self):
        first = digest.project_snapshot(snapshot())
        reordered = json.loads(json.dumps(snapshot(), sort_keys=True))
        self.assertEqual(first, digest.project_snapshot(reordered))
        first["fields_covered"]["schema"].clear()
        self.assertTrue(digest.project_snapshot(snapshot())["fields_covered"]["schema"])

    def test_dsp_keeps_local_identity_separate_from_canonical_metadata(self):
        item = dsp_record()
        schema, values = native_mock_projection(0, False)
        schema["canonical_name"] = "canonical.parameter0"
        schema["dsp"] = {"index": 0, "name": "canonical.parameter0", "tag": 900,
                         "title": "Canonical title", "folder": 7, "raw_flags": 42}
        expected = independent_hashes([schema], [values], [True], "dsp")
        actual = digest.project_snapshot(snapshot([item], "dsp"))
        self.assertEqual({key: actual[key] for key in digest.HASH_FIELDS}, expected)

    def test_source_is_part_of_every_hash(self):
        controller = digest.project_snapshot(snapshot())
        dsp = digest.project_snapshot(snapshot([dsp_record()], "dsp"))
        self.assertTrue(all(controller[key] != dsp[key] for key in digest.HASH_FIELDS))
        controller = digest.project_snapshot(snapshot([], "controller"))
        dsp = digest.project_snapshot(snapshot([], "dsp"))
        self.assertTrue(all(controller[key] != dsp[key] for key in digest.HASH_FIELDS))
        self.assert_rejected(snapshot(source="auto"))

    def test_every_canonical_dsp_field_changes_schema_and_both_value_hashes(self):
        original = snapshot([dsp_record()], "dsp")
        baseline = digest.project_snapshot(original)
        for field, value in (("tag", 901), ("title", "new title"), ("folder", 8), ("raw_flags", 43),
                             ("name", "new.canonical.name")):
            changed = deepcopy(original)
            changed["records"][0]["native"][field] = value
            if field == "name":
                changed["records"][0]["name"] = value
            with self.subTest(field=field):
                result = digest.project_snapshot(changed)
                self.assertTrue(all(result[key] != baseline[key] for key in digest.HASH_FIELDS))

    def test_schema_metadata_is_bound_to_both_value_hashes(self):
        baseline = digest.project_snapshot(snapshot())
        mutations = [("enabled", False), ("name", "new.local.name"), ("tag", 7), ("native_kind", "Integer"),
                     ("display_parser", "NativeFormatter { flags: 2 }"), ("readOnly", True)]
        for field, value in mutations:
            changed = snapshot()
            changed["records"][0][field] = value
            with self.subTest(field=field):
                result = digest.project_snapshot(changed)
                self.assertTrue(all(result[key] != baseline[key] for key in digest.HASH_FIELDS))
        changed = snapshot()
        changed["records"][0]["native_min"] = floating(math.nextafter(0.0, 1.0))
        changed["records"][0]["min"] = math.nextafter(0.0, 1.0)
        self.assertTrue(all(digest.project_snapshot(changed)[key] != baseline[key] for key in digest.HASH_FIELDS))

    def test_only_proven_readonly_observations_are_excluded(self):
        items = [record(0), record(1, read_only=True), record(2), record(3)]
        items[2]["enabled"] = False
        no_support(items[2])
        replace_value(items[3], typed("opaque_object", tag=4, present=True))
        items[3]["native_kind"] = "Image"
        items[3]["native_normalized"], items[3]["normalized"] = normalized(None)
        no_support(items[3])
        previous = digest.project_snapshot(snapshot(items))
        self.assertEqual(previous["non_read_only_count"], 3)
        self.assertEqual(previous["excluded_opaque_count"], 1)
        self.assertFalse(previous["opaque_contents_verified"])
        replace_value(items[1], floating(3.0))
        changed = digest.project_snapshot(snapshot(items))
        self.assertNotEqual(previous["all_values_sha256"], changed["all_values_sha256"])
        self.assertEqual(previous["non_read_only_values_sha256"], changed["non_read_only_values_sha256"])
        previous = changed
        for index, value in ((2, floating(3.0)), (3, typed("opaque_object", tag=4, present=False))):
            replace_value(items[index], value)
            changed = digest.project_snapshot(snapshot(items))
            self.assertNotEqual(previous["non_read_only_values_sha256"], changed["non_read_only_values_sha256"])
            previous = changed

    def test_readonly_flag_with_public_write_support_is_not_excluded(self):
        item = record()
        item["readOnly"] = True
        baseline = digest.project_snapshot(snapshot([item]))
        self.assertEqual(baseline["readonly_observation_count"], 0)
        replace_value(item, floating(2.0))
        changed = digest.project_snapshot(snapshot([item]))
        self.assertNotEqual(baseline["non_read_only_values_sha256"], changed["non_read_only_values_sha256"])

    def test_raw_variant_display_and_normalized_are_independently_hashed(self):
        baseline = digest.project_snapshot(snapshot())
        for field, value in (("variant", 7), ("text", "different"), ("normalized", 0.2)):
            changed = snapshot()
            item = changed["records"][0]
            if field == "variant":
                item["native_value"]["variant_tag"] = value
            elif field == "normalized":
                item["native_normalized"], item["normalized"] = normalized(value)
            else:
                item[field] = value
            result = digest.project_snapshot(changed)
            self.assertEqual(result["schema_sha256"], baseline["schema_sha256"])
            self.assertTrue(all(result[key] != baseline[key] for key in digest.HASH_FIELDS[1:]))

    def test_float_bits_are_authoritative_even_at_one_ulp(self):
        original = snapshot()
        baseline = digest.project_snapshot(original)
        rounded = deepcopy(original)
        rounded["records"][0]["native_value"]["data"]["value"] = math.nextafter(1.0, 2.0)
        rounded["records"][0]["value"] = math.nextafter(1.0, 2.0)
        self.assertEqual(baseline, digest.project_snapshot(rounded))
        changed = deepcopy(original)
        replace_value(changed["records"][0], floating(math.nextafter(1.0, 2.0)))
        self.assertNotEqual(baseline["all_values_sha256"], digest.project_snapshot(changed)["all_values_sha256"])
        rounded["records"][0]["value"] = math.nextafter(math.nextafter(1.0, 2.0), 2.0)
        self.assert_rejected(rounded)

    def test_negative_zero_float_and_normalized_bits_remain_distinct(self):
        positive, negative = snapshot(), snapshot()
        replace_value(positive["records"][0], floating(0.0))
        replace_value(negative["records"][0], floating(-0.0))
        self.assertNotEqual(digest.project_snapshot(positive)["all_values_sha256"],
                            digest.project_snapshot(negative)["all_values_sha256"])
        negative["records"][0]["value"] = 0
        negative["records"][0]["native_value"]["data"]["value"] = 0
        negative["records"][0]["native_normalized"], negative["records"][0]["normalized"] = normalized(-0.0)
        negative["records"][0]["normalized"] = 0
        result = digest.project_snapshot(negative)
        negative["records"][0]["native_normalized"]["bits_hex"] = "00000000"
        self.assertNotEqual(result["all_values_sha256"], digest.project_snapshot(negative)["all_values_sha256"])

    def test_exact_int64_and_safe_integer_metadata(self):
        for number in (-(1 << 63), 1 << 53, (1 << 63) - 1):
            item = record()
            replace_value(item, integer(number))
            no_support(item)
            item["native_support"]["raw_number"] = True
            item["native_normalized"], item["normalized"] = normalized(None)
            result = digest.project_snapshot(snapshot([item]))
            self.assertEqual(result["excluded_opaque_count"], 0)
            next_item = deepcopy(item)
            replace_value(next_item, integer(number + 1 if number < (1 << 63) - 1 else number - 1))
            self.assertNotEqual(result["all_values_sha256"], digest.project_snapshot(snapshot([next_item]))["all_values_sha256"])
        item = record()
        item["native_min"], item["min"] = integer(-(1 << 63)), None
        item["support"] = dict.fromkeys(item["support"], False)
        item["writable"] = False
        self.assertEqual(digest.project_snapshot(snapshot([item]))["count"], 1)

    def test_all_exact_native_raw_types_and_null_extrema(self):
        values = [typed("empty", tag=0), integer(7), typed("text", tag=3, value="한글 μ\0\n\"\\"),
                  typed("non_finite_float", bits_hex="7FF8000000000123"),
                  typed("opaque_object", tag=4, present=False),
                  {"variant_tag": 9, "data": {"kind": "unsupported", "tag": 9}}]
        for value in values:
            with self.subTest(kind=value["data"]["kind"]):
                item = record()
                replace_value(item, value)
                for field in ("min", "max", "default"):
                    item["native_" + field], item[field] = None, None
                item["native_normalized"], item["normalized"] = normalized(None)
                result = digest.project_snapshot(snapshot([item]))
                self.assertEqual(result["count"], 1)
                self.assertFalse(result["opaque_contents_verified"])

    def test_normalized_nonfinite_and_not_applicable_require_tags(self):
        item = record()
        item["native_normalized"] = {"kind": "non_finite_float32", "bits_hex": "7FC00123"}
        item["normalized"] = None
        nonfinite = digest.project_snapshot(snapshot([item]))
        item["native_normalized"] = {"kind": "not_applicable"}
        other = digest.project_snapshot(snapshot([item]))
        self.assertNotEqual(nonfinite["all_values_sha256"], other["all_values_sha256"])
        for malformed in (None, {}, {"kind": "float32", "bits_hex": "7FC00123"},
                          {"kind": "non_finite_float32", "bits_hex": "3F800000"},
                          {"kind": "invalid_float32", "bits_hex": "7FC00123"}):
            item["native_normalized"] = malformed
            self.assert_rejected(snapshot([item]))

    def test_normalized_numeric_shadow_only_allows_one_double_ulp(self):
        original = snapshot()
        baseline = digest.project_snapshot(original)
        shadow = original["records"][0]["normalized"]
        original["records"][0]["normalized"] = math.nextafter(shadow, 1.0)
        self.assertEqual(baseline, digest.project_snapshot(original))
        original["records"][0]["normalized"] = math.nextafter(math.nextafter(shadow, 1.0), 1.0)
        self.assert_rejected(original)
        original["records"][0]["normalized"] = 0.1
        self.assert_rejected(original)

    def test_utf8_unicode_and_json_escapes_are_preserved(self):
        item = record()
        item["name"] = "표시.μ.😀"
        item["text"] = "한글\u2028\u2029\b\t\n\r\f\0\"\\/😀"
        first = digest.project_snapshot(snapshot([item]))
        wire = json.loads(json.dumps(snapshot([item]), ensure_ascii=True))
        self.assertEqual(first, digest.project_snapshot(wire))
        wire["records"][0]["text"] += " "
        self.assertNotEqual(first["all_values_sha256"], digest.project_snapshot(wire)["all_values_sha256"])

    def test_formatter_variants_have_exact_schema(self):
        for name in ("Unsupported", "LiteralString", "PlainNumeric", "NativeFormatter { flags: -2147483648 }",
                     "NativeFormatter { flags: 2147483647 }"):
            item = record()
            item["display_parser"] = name
            self.assertEqual(digest.project_snapshot(snapshot([item]))["count"], 1)

    def test_index_order_duplicates_and_declared_count_fail_closed(self):
        items = [record(0), record(1)]
        self.assert_rejected(snapshot(items[::-1]))
        repeated = deepcopy(items)
        repeated[1]["index"] = 0
        self.assert_rejected(snapshot(repeated))
        repeated = deepcopy(items)
        repeated[1]["name"] = repeated[0]["name"]
        self.assert_rejected(snapshot(repeated))
        for field in ("count", "reported_count"):
            for declared in (2, None, True):
                value = snapshot()
                value[field] = declared
                self.assert_rejected(value)
        self.assert_rejected(snapshot([record()] * (digest.MAX_COUNT + 1)))

    def test_incomplete_dsp_or_contradictory_public_metadata_fails_closed(self):
        for field, value in (("native", None), ("access_metadata_complete", False), ("name", "wrong")):
            item = dsp_record()
            item[field] = value
            self.assert_rejected(snapshot([item], "dsp"))
        item = dsp_record()
        item["native"]["access_error"] = ""
        self.assert_rejected(snapshot([item], "dsp"))
        item = dsp_record()
        item["native"]["native_description"]["tag"] += 1
        self.assert_rejected(snapshot([item], "dsp"))
        item = dsp_record()
        del item["native"]["native_description"]
        self.assert_rejected(snapshot([item], "dsp"))
        item = dsp_record()
        item["native"]["index"] = 1
        self.assert_rejected(snapshot([item], "dsp"))
        items = [dsp_record(0), dsp_record(1)]
        items[1]["native"]["name"] = items[1]["name"] = items[0]["name"]
        self.assert_rejected(snapshot(items, "dsp"))

    def test_typed_fields_access_kind_and_identity_are_mandatory(self):
        for field in ("native_value", "native_min", "native_max", "native_default", "native_support",
                      "native_normalized", "value", "normalized", "min", "text", "enabled", "readOnly", "writable"):
            item = record()
            del item[field]
            with self.subTest(field=field):
                self.assert_rejected(snapshot([item]))
        for field, value in (("native_kind", "Unknown(17)"), ("native_kind", []), ("enabled", 1), ("readOnly", None),
                             ("access_metadata_complete", False), ("readable", False), ("readable", 1),
                             ("name", ""), ("name", "bad\0name"),
                             ("name", "μ" * 257), ("text", "\ud800"), ("text", "x" * 4097), ("display_parser", {})):
            item = record()
            item[field] = value
            with self.subTest(field=field, value=str(value)[:30]):
                self.assert_rejected(snapshot([item]))
        value = snapshot()
        del value["instance"]["class_uid"]
        self.assert_rejected(value)

    def test_explicit_incomplete_snapshot_envelopes_are_rejected(self):
        for field, value in (("access_complete", False), ("access_metadata_complete", None), ("page_complete", 1),
                             ("all_indices_read", False), ("failed_indices", [0]), ("failed_access_indices", [0])):
            original = snapshot()
            original[field] = value
            with self.subTest(field=field):
                self.assert_rejected(original)

    def test_typed_integer_ambiguity_and_wrong_width_are_rejected(self):
        invalid = [{"kind": "integer", "value": 1, "exact_decimal": "1"},
                   {"kind": "integer", "value": 1 << 53}, {"kind": "integer", "value": True},
                   {"kind": "integer", "exact_decimal": "1"}, {"kind": "integer", "exact_decimal": "-0"},
                   {"kind": "integer", "exact_decimal": "09223372036854775807"},
                   {"kind": "integer", "exact_decimal": str(1 << 63)},
                   {"kind": "integer", "exact_decimal": "9" * 5000},
                   {"kind": "float", "value": 1.0, "bits_hex": "3FF000000000000"},
                   {"kind": "float", "value": 1.0, "bits_hex": "7FF0000000000000"},
                   {"kind": "float", "value": 10 ** 1000, "bits_hex": "3FF0000000000000"},
                   {"kind": "non_finite_float", "bits_hex": "3FF0000000000000"}]
        for data in invalid:
            item = record()
            item["native_value"] = {"variant_tag": 2, "data": data}
            with self.subTest(kind=data["kind"]):
                self.assert_rejected(snapshot([item]))

    def test_support_shadow_extrema_and_opaque_schema_contradictions_fail_closed(self):
        for field, value in (("value", 3.0), ("min", 1.0), ("writable", False), ("normalized", 0.2),
                             ("native_min", typed("opaque_object", tag=4, present=True)),
                             ("native_max", {"variant_tag": 8, "data": {"kind": "unsupported", "tag": 8}}),
                             ("native_support", {"raw_number": 1, "normalized": True, "display_text": True})):
            item = record()
            item[field] = value
            with self.subTest(field=field):
                self.assert_rejected(snapshot([item]))
        item = record()
        item["support"]["raw"] = False
        self.assert_rejected(snapshot([item]))

    def test_many_synthetic_parameters_are_not_silently_truncated(self):
        items = [record(index, read_only=index % 5 == 0) for index in range(4262)]
        result = digest.project_snapshot(snapshot(items))
        self.assertEqual(result["count"], 4262)
        self.assertEqual(result["read_only_count"], 853)
        self.assertEqual(result["non_read_only_count"], 3409)
        items[-1]["text"] = "changed final record"
        self.assertNotEqual(result["all_values_sha256"], digest.project_snapshot(snapshot(items))["all_values_sha256"])


class NativeComparisonTests(unittest.TestCase):
    def setUp(self):
        self.snapshot = snapshot([record(0), record(1, read_only=True)])
        self.native = {**digest.project_snapshot(self.snapshot), "ok": True, "outcome": "observed"}

    def test_complete_observed_parity_is_accepted_as_projection_only(self):
        checked = digest.validate_native_digest(self.snapshot, self.native)
        self.assertTrue(checked["native_digest_parity_verified"])
        self.assertFalse(checked["opaque_contents_verified"])
        self.assertFalse(checked["atomic_snapshot"])
        self.assertEqual(checked["hash_scope"], "exposed_parameter_projection")

    def test_success_flags_and_native_errors_are_never_hash_acceptance(self):
        for key, value in (("ok", False), ("ok", 1), ("outcome", "accepted"), ("error", "failed owner check"),
                           ("error", ""), ("access_complete", False), ("atomic_snapshot", True),
                           ("opaque_contents_verified", True)):
            changed = deepcopy(self.native)
            changed[key] = value
            with self.subTest(key=key):
                with self.assertRaises(digest.DigestProjectionError):
                    digest.validate_native_digest(self.snapshot, changed)

    def test_all_hashes_and_coverage_counters_and_instance_must_match(self):
        for key in self.native:
            if key in ("ok", "outcome"):
                continue
            changed = deepcopy(self.native)
            del changed[key]
            with self.subTest(missing=key):
                with self.assertRaises(digest.DigestProjectionError):
                    digest.validate_native_digest(self.snapshot, changed)
        for key in digest.HASH_FIELDS:
            for value in ("0" * 64, self.native[key].upper(), "x" * 64):
                changed = deepcopy(self.native)
                changed[key] = value
                with self.assertRaises(digest.DigestProjectionError):
                    digest.validate_native_digest(self.snapshot, changed)
        for key, value in (("source", "dsp"), ("count", 1), ("count", 2.0), ("max_count", 8193),
                           ("hash_format_version", "future-v2"), ("readable_count", True),
                           ("fields_covered", {}), ("instance", {**INSTANCE, "object_uid": "different"})):
            changed = deepcopy(self.native)
            changed[key] = value
            with self.subTest(key=key):
                with self.assertRaises(digest.DigestProjectionError):
                    digest.validate_native_digest(self.snapshot, changed)

    def test_partial_serialized_snapshot_cannot_match_larger_native_count(self):
        partial = deepcopy(self.snapshot)
        partial["records"].pop()
        with self.assertRaises(digest.DigestProjectionError):
            digest.validate_native_digest(partial, self.native)


if __name__ == "__main__":
    unittest.main()
