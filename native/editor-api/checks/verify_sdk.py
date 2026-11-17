"""Reject any CCL checkout other than the reviewed 5.1.2 API declarations."""
from pathlib import Path
import hashlib
import json
import re
import subprocess
import sys

if len(sys.argv) != 2:
    raise SystemExit("Usage: verify_sdk.py /path/to/ccl-framework-5.1.2")
root=Path(sys.argv[1])
pin=json.loads(Path(__file__).with_name('sdk-5.1.2.sha256.json').read_text())
commit=subprocess.run(['git','-C',str(root),'rev-parse','HEAD'],check=True,capture_output=True,text=True).stdout.strip()
if commit!=pin['commit']:
    raise SystemExit(f"REFUSED CCL SDK {commit}; expected {pin['tag']} at {pin['commit']}")
for path,expected in pin['headers'].items():
    actual=hashlib.sha256((root/path).read_bytes()).hexdigest()
    if actual!=expected:
        raise SystemExit(f"REFUSED changed public ABI header: {path}")
control_iid = re.search(r"DEFINE_IID\s*\(\s*IControl\s*,([^)]*)\)",
                        (root / 'ccl/public/gui/framework/iview.h').read_text())
if control_iid is None or [int(x.strip(), 0) for x in control_iid[1].split(',')] != [
    0x8186d4bb, 0x76b6, 0x46f6, 0x88, 0x9f, 0x9a, 0xd0, 0xef, 0x50, 0x19, 0x81,
]:
    raise SystemExit("REFUSED changed public IControl IID")
print(f"Verified exact {pin['tag']} commit and {len(pin['headers'])} API header hashes")
