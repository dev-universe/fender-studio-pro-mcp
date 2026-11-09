from pathlib import Path
import re
import sys

assembly = Path(sys.argv[1]).read_text()
expected = {
    "query":0,"retain":8,"release":16,"create":24,
    "mediator_meta":24,"mediator_target":32,"presets":56,"iterator":24,
    "list_empty":32,"list_first":40,"list_last":48,"list_contains":56,
    "list_add":64,"list_remove":72,"list_clear":80,"iterator_done":24,"iterator_next":32,
    "read_only":24,"name":40,"meta":48,"url":56,"url_clone":24,"url_equal":40,
    "url_empty":48,"url_kind":56,"url_protocol":80,"url_host":96,"url_native":192,"url_absolute":248,
    "exists":32,"attribute":80,"string_length":32,"string_copy":64,"can_insert":24,"insert":32,
}
functions = dict(re.findall(r"^_abi_([^:]+):[^\n]*\n(.*?)(?=^\s*; -- End function)",assembly,re.MULTILINE|re.DOTALL))
for name,offset in expected.items():
    body=functions[name]
    offsets=re.findall(r"ldr\s+x\d+, \[x\d+, #(\d+)\]",body)
    if offset==0:
        if offsets or len(re.findall(r"ldr\s+x\d+, \[x\d+\]",body))!=2:
            raise SystemExit(f"ABI mismatch: {name}")
    elif offsets!=[str(offset)]:
        raise SystemExit(f"ABI mismatch {name}: {offsets} != {offset}")
for name,symbol in [("allocator_export","CCLGetPlugInManager"),("filesystem_export","CCLGetFileSystem")]:
    if symbol not in functions[name]:
        raise SystemExit(f"Export ABI mismatch: {name}")
print(f"PASS: {len(expected)} CCL5.1.2 virtual calls, two service exports and layout assertions")
