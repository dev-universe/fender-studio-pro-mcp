"""Reject any CCL checkout other than the reviewed 5.1.2 API declarations."""
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
    raise SystemExit(f"REFUSED CCL SDK {commit}; expected {pin['tag']} at {pin['commit']}")
for path,expected in pin['headers'].items():
    actual=hashlib.sha256((root/path).read_bytes()).hexdigest()
    if actual!=expected:
        raise SystemExit(f"REFUSED changed public ABI header: {path}")
print(f"Verified exact {pin['tag']} commit and {len(pin['headers'])} API header hashes")

# Verify newly handed-off interfaces against both the pinned declaration and
# our Rust constants. No vendor IID implementation is compiled or linked.
rust=(Path(__file__).parent.parent/'src/request/presets.rs').read_text()
interfaces={
    'MEDIATOR':('ccl/public/app/ipreset.h','IPresetMediator'),
    'DATA_TARGET':('ccl/public/gui/idatatarget.h','IDataTarget'),
    'MANAGER':('ccl/public/app/ipreset.h','IPresetManager'),
    'URL':('ccl/public/storage/iurl.h','IUrl'),
    'FILE_SYSTEM':('ccl/public/system/ifilesystem.h','IFileSystem'),
}
for constant,(path,interface) in interfaces.items():
    declaration=re.search(r'DEFINE_IID\s*\(\s*'+interface+r'\s*,([^)]*)\)',(root/path).read_text())
    implementation=re.search(r'const '+constant+r': Uid = Uid \{(.*?)\};',rust,re.S)
    if not declaration or not implementation:
        raise SystemExit(f'Missing interface declaration: {constant}')
    sdk_values=[int(v.strip(),0) for v in declaration[1].split(',')]
    rust_values=[int(v,0) for v in re.findall(r'0x[0-9a-fA-F]+|\b[0-9]+\b',implementation[1])]
    if sdk_values!=rust_values:
        raise SystemExit(f'Rust interface IID mismatch: {constant}')
print(f'Checked {len(interfaces)} preset bridge interface IID constants')

catalog=(Path(__file__).parent.parent/'src/request/alias_controller/catalog.rs').read_text()
declaration=re.search(r'DEFINE_IID\s*\(\s*IItemModel\s*,([^)]*)\)',(root/'ccl/public/gui/framework/iitemmodel.h').read_text())
implementation=re.search(r'const ITEM_MODEL: Uid = Uid \{(.*?)\};',catalog,re.S)
if not declaration or not implementation:
    raise SystemExit('Missing fixed model catalog IID declaration')
if [int(v.strip(),0) for v in declaration[1].split(',')]!=[int(v,0) for v in re.findall(r'0x[0-9a-fA-F]+|\b[0-9]+\b',implementation[1])]:
    raise SystemExit('Rust IItemModel IID mismatch')
# Fail when a new ABI-bearing transitive SDK include was not reviewed/pinned.
for source in ['abi_calls.cpp','layout_check.cpp']:
    import shlex
    dependencies=subprocess.run(['clang++','-std=c++20','-arch','arm64','-I'+str(root),'-MM',str(Path(__file__).with_name(source))],check=True,capture_output=True,text=True)
    for name in shlex.split(dependencies.stdout.replace('\\\n',' '))[1:]:
        path=Path(name)
        if path.is_relative_to(root) and str(path.relative_to(root)) not in pin['headers']:
            raise SystemExit(f'Unpinned transitive SDK ABI dependency: {path.relative_to(root)}')
print('Checked IItemModel IID and all transitive SDK/compiler ABI dependencies')
