from pathlib import Path
import hashlib
import json
import re
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

folder_source = Path(__file__).parents[1].joinpath('src/folder.rs').read_text()
collection_source = Path(__file__).parents[1].joinpath('src/folder/collection.rs').read_text()
ffi_source = Path(__file__).parents[1].joinpath('src/ffi.rs').read_text()
preset_header = root.joinpath('ccl/public/app/ipreset.h').read_text()
for constant, interface in [('MEDIATOR', 'IPresetMediator'), ('COLLECTOR', 'IPresetCollector'),
                            ('REGISTRY', 'IPresetFileRegistry'), ('HANDLER', 'IPresetFileHandler'),
                            ('COLLECTION', 'IPresetCollection'), ('URL', 'IUrl'), ('PRESET', 'IPreset')]:
    header = root.joinpath('ccl/public/storage/iurl.h').read_text() if interface == 'IUrl' else preset_header
    source = ffi_source if constant == 'PRESET' else collection_source if constant in ('COLLECTION', 'URL') else folder_source
    public = re.search(r'DEFINE_IID\s*\(\s*' + interface + r'\s*,([^)]*)\)', header)
    rust = re.search(r'const\s+' + constant + r':\s*Uid\s*=\s*Uid\s*\{(.*?)\};', source, re.S)
    if not public or not rust:
        raise SystemExit(f'REFUSED missing fixed folder IID {interface}')
    expected = [int(value.strip(), 0) for value in public[1].split(',')]
    actual = [int(value, 0) for value in re.findall(r'0x[0-9a-fA-F]+', rust[1])]
    if actual != expected:
        raise SystemExit(f'REFUSED changed fixed folder IID {interface}')
print('PASS: seven fixed folder/collection preset IIDs match pinned declarations')
