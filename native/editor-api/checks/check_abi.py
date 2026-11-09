from pathlib import Path
import re
import sys

if len(sys.argv) != 2:
    raise SystemExit("Usage: check_abi.py /path/to/compiled-arm64.s")
assembly = Path(sys.argv[1]).read_text()
expected = {
    "query": 0, "retain": 8, "release": 16, "object_type": 24,
    "type_name": 40, "node_uid": 32, "node_class": 40,
    "node_find": 80, "node_lookup": 104,
    "desktop_count": 48, "desktop_window": 56, "window_controller": 240,
    "view_controller": 256, "view_children": 160,
    "iterator_create": 88, "direct_child": 96,
    "iterator_done": 24, "iterator_next": 32,
}
functions = dict(re.findall(
    r"^_abi_([^:]+):[^\n]*\n(.*?)(?=^\s*; -- End function)",
    assembly, re.MULTILINE | re.DOTALL,
))
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
print(f"PASS: {len(expected)} exact CCL5.1.2 virtual calls, UID/String layout, reference export")
