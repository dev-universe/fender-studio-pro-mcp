"""Check public-header compiler call offsets, without loading a DAW or library."""
from pathlib import Path
import re
import sys

assembly = Path(sys.argv[1]).read_text()
expected = {
    "controller_count": 24, "controller_parameter": 32,
    "controller_provider": 24,
    "parameter_type": 24, "parameter_name": 32, "parameter_tag": 104,
    "parameter_controller": 112, "parameter_normalized": 280,
    "node_uid": 32, "node_class": 40, "node_find": 80,
    "object_named": 72, "object_identified":64, "document_active": 72, "document_model": 48,
    "desktop_count": 48, "desktop_window": 56, "window_controller": 240,
    "view_controller": 256, "view_children": 160, "children_iterator": 88,
    "iterator_done": 24, "iterator_next": 32, "control_parameter": 24,
    "idle_add": 240, "idle_remove": 248, "post": 72,
    "cancel_messages": 88, "thread_id": 24, "node_id": 24,"node_parent":48,
    "node_count": 64, "node_child": 72, "node_lookup": 104,
    "object_type": 24, "type_name": 40, "string_copy": 64,
    "structured_count": 40, "alias_plugin": 24,
    "string_length": 32, "private_count": 24, "private_info": 32,
    "table_register":24,"table_unregister":32,"message_id":24,
    "message_count":32,"message_argument":40,"message_auxiliary":40,"object_invoke":56,
    "string_clone":144,"string_assign":192,"controller_find":40,
    "type_parent":24,"type_methods":64,"type_module":80,
    "scripting_startup":64,"scripting_shutdown":72,
    "storable_save":32,"storable_load":40,
    "stream_read":24,"stream_write":32,"stream_tell":40,
    "stream_seekable":48,"stream_seek":56,
}
functions = dict(re.findall(
    r"^_abi_([^:]+):[^\n]*\n(.*?)(?=^\s*; -- End function)",
    assembly, re.MULTILINE | re.DOTALL,
))
for name, offset in expected.items():
    body = functions[name]
    actual = re.findall(r"ldr\s+x\d+, \[x\d+, #(\d+)\]", body)
    if actual != [str(offset)]:
        raise SystemExit(f"ABI offset mismatch for {name}: {actual}, expected {offset}")
value_body = functions["capture_parameter_value"]
if "mov\tx8, x1" not in value_body:
    raise SystemExit("C++ nontrivial Variant return evidence changed; review required")
for name,registers in {"scripting_startup":["x3","w4"],"scripting_shutdown":["w2"]}.items():
    for register in registers:
        if not re.search(r"mov\s+"+register+r", #0\b",functions[name]):
            raise SystemExit(f"Scripting load/unload=false or null args changed: {name} {register}")
if not re.search(r"mov\s+w1, #2\b", functions["message_auxiliary"]):
    raise SystemExit("Third Message argument must be fetched by reference at fixed index2")
for name,symbol in [("plugin_manager_export","CCLGetPlugInManager"),("file_system_export","CCLGetFileSystem")]:
    if symbol not in functions[name]:
        raise SystemExit(f"Export ABI mismatch: {name}")
print(f"Checked {len(expected)} exact SDK/compiler call offsets and two borrowed service exports; Variant value return remains indirect x8 and absent from runtime")
