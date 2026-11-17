"""Assert actual arm64 compiler dispatch for every virtual call used by Rust."""
from pathlib import Path
import re
import sys

if len(sys.argv) != 2:
    raise SystemExit("Usage: check_abi.py /path/to/compiled-arm64.s")
assembly = Path(sys.argv[1]).read_text()
expected = {
    "query": 0, "retain": 8, "release": 16,
    "parameter_type": 24, "parameter_name": 32,
    "parameter_enabled": 48, "parameter_state": 64, "parameter_tag": 104,
    "parameter_controller": 112,
    "parameter_begin_edit": 128, "parameter_end_edit": 136,
    "parameter_original": 144, "parameter_precision": 240,
    "parameter_set_raw": 168, "parameter_normalized": 280,
    "parameter_set_normalized": 288, "parameter_formatter": 328,
    "parameter_choice_string": 344,
    "parameter_to_string": 352, "parameter_from_string": 360,
    "object_property": 32, "string_length": 32, "string_copy": 64,
    "string_clone": 144, "string_private": 168, "string_assign": 192,
    "formatter_flags": 24, "formatter_scan": 40,
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
            raise SystemExit(f"ABI zero offset mismatch for {name}: {body}")
    elif offsets != [str(offset)]:
        raise SystemExit(f"ABI offset mismatch for {name}: {offsets}, expected {offset}")
if "mov\tx8, x1" not in functions["forbidden_value_return"]:
    raise SystemExit("Expected nontrivial BY-VALUE Variant x8 ABI evidence missing")
for name, symbol in [("empty_export", "CCLGetEmptyString"),
                     ("cstring_export", "CCLGetConstantCString")]:
    if symbol not in functions[name]:
        raise SystemExit(f"Unexpected text export: {name}")
print(f"Checked {len(expected)} native virtual calls, text export reference ABI, and excluded BY-VALUE return")
