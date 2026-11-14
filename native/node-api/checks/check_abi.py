from pathlib import Path
import re
import sys

if len(sys.argv) != 2:
    raise SystemExit("Usage: check_abi.py /path/to/compiled-arm64.s")
assembly = Path(sys.argv[1]).read_text()
expected = {
    "query": 0, "query_preset_collector": 0, "query_preset_mediator": 0,
    "query_storable": 0, "retain": 8, "release": 16, "object_type": 24,
    "type_parent": 24, "type_flags": 32, "type_name": 40,
    "type_namespace": 48, "type_id": 56, "type_methods": 64,
    "type_properties": 72, "node_id": 24, "node_uid": 32,
    "node_class": 40, "node_parent": 48, "node_count": 64,
    "node_child": 72, "node_find": 80, "string_length": 32,
    "string_copy": 64,
}
functions = dict(re.findall(r"^_abi_([^:]+):[^\n]*\n(.*?)(?=^\s*; -- End function)", assembly, re.MULTILINE | re.DOTALL))
if set(functions) != set(expected) | {"constant_string"}:
    raise SystemExit("Compiler probe function set does not match the reviewed call map")
for name, offset in expected.items():
    body = functions[name]
    offsets = re.findall(r"ldr\s+x\d+, \[x\d+, #(\d+)\]", body)
    if offset == 0:
        if offsets or len(re.findall(r"ldr\s+x\d+, \[x\d+\]", body)) != 2:
            raise SystemExit(f"ABI zero offset mismatch: {name}")
    elif offsets != [str(offset)]:
        raise SystemExit(f"ABI offset mismatch for {name}: {offsets} != {offset}")
if "CCLGetConstantString" not in functions["constant_string"]:
    raise SystemExit("Unexpected public constant String export ABI")
for name, interface in {"query_preset_collector": "IPresetCollector",
                        "query_preset_mediator": "IPresetMediator",
                        "query_storable": "IStorable"}.items():
    if interface not in functions[name]:
        raise SystemExit(f"Compiler query uses an unexpected interface IID: {name}")
print(f"PASS: {len(expected)} exact CCL5.1.2 virtual calls, reflection/UID/String layouts and reference export")
