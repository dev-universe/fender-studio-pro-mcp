from pathlib import Path
import hashlib
import json
import subprocess
import sys

root=Path(sys.argv[1])
pin=json.loads(Path(__file__).with_name('sdk-5.1.2.sha256.json').read_text())
commit=subprocess.run(['git','-C',str(root),'rev-parse','HEAD'],check=True,capture_output=True,text=True).stdout.strip()
if commit!=pin['commit']:
    raise SystemExit(f"REFUSED SDK commit {commit}")
for path,expected in pin['headers'].items():
    if hashlib.sha256((root/path).read_bytes()).hexdigest()!=expected:
        raise SystemExit(f"REFUSED changed pinned API source: {path}")
print(f"Verified exact {pin['tag']} and {len(pin['headers'])} public source hashes")
