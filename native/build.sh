#!/bin/sh
# Build the normal four-file native extension. Never installs or launches a DAW.
set -eu
native_root="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
repo_root="$(CDPATH= cd -- "$native_root/.." && pwd)"
if [ "$(uname -s)" != Darwin ] || [ "$(uname -m)" != arm64 ]; then
    echo "Native add-on currently supports macOS arm64 only" >&2
    exit 2
fi
output_root="${1:-$repo_root/artifacts/native}"
if [ "$#" -gt 1 ]; then
    echo "Usage: sh native/build.sh [output-directory]" >&2
    exit 2
fi
if [ -n "${CCL_FRAMEWORK_SOURCE:-}" ]; then
    sh "$native_root/addon/checks/check_sdk.sh" "$CCL_FRAMEWORK_SOURCE"
    sh "$native_root/parameter-api/checks/check_sdk.sh" "$CCL_FRAMEWORK_SOURCE"
    sh "$native_root/editor-api/checks/check_sdk.sh" "$CCL_FRAMEWORK_SOURCE"
fi
export CARGO_TARGET_DIR="$repo_root/target/native-addon"
cargo +1.88.0 build --locked --release --manifest-path "$native_root/addon/Cargo.toml"
package="$output_root/fender-studio-pro-mcp.native.mac"
bundle="$package/plugins/mac/fender_mcp_native.bundle"
mkdir -p "$bundle/Contents/MacOS"
cp "$native_root/addon/package/metainfo.xml" "$package/metainfo.xml"
cp "$native_root/addon/package/installdata.xml" "$package/installdata.xml"
cp "$native_root/addon/package/plugins/mac/fender_mcp_native.bundle/Contents/Info.plist" "$bundle/Contents/Info.plist"
cp "$CARGO_TARGET_DIR/release/libfender_mcp_native.dylib" "$bundle/Contents/MacOS/fender_mcp_native"
plutil -lint "$bundle/Contents/Info.plist"
xmllint --noout "$package/metainfo.xml" "$package/installdata.xml"
# Cargo's packed debug info is complete while intermediates still exist.
# Re-running dsymutil on the copied executable would lose removed codegen objects.
ds="$CARGO_TARGET_DIR/release/libfender_mcp_native.dylib.dSYM"
if [ ! -d "$ds" ]; then
    echo "Cargo did not produce the required packed dSYM" >&2
    exit 1
fi
python3 - "$ds" "$output_root/fender_mcp_native.dSYM" <<'PYDSYM'
import pathlib,shutil,sys
source,target=map(pathlib.Path,sys.argv[1:])
if target.is_symlink(): raise SystemExit('Refused symlink dSYM output')
if target.exists(): shutil.rmtree(target)
shutil.copytree(source,target)
PYDSYM
python3 - "$package" "$output_root/native-artifact-manifest.json" "$native_root" <<'PY'
import hashlib,json,pathlib,subprocess,sys
package=pathlib.Path(sys.argv[1]); native_root=pathlib.Path(sys.argv[3]); files={}
for path in sorted(package.rglob('*')):
    if path.is_file(): files[str(path.relative_to(package))]=hashlib.sha256(path.read_bytes()).hexdigest()
if len(files)!=4: raise SystemExit('Refused package with unexpected file count')
binary=package/'plugins/mac/fender_mcp_native.bundle/Contents/MacOS/fender_mcp_native'
uuid=subprocess.check_output(['dwarfdump','--uuid',str(binary)],text=True).strip()
sources={}
for path in sorted(native_root.rglob('*')):
    rel=path.relative_to(native_root)
    if 'target' in rel.parts or '__pycache__' in rel.parts or not path.is_file(): continue
    if path.suffix not in ('.rs','.toml','.lock','.py','.json','.cpp','.sh','.md','.xml','.plist') and path.name not in ('LICENSE','.gitignore'): continue
    sources[str(rel)]=hashlib.sha256(path.read_bytes()).hexdigest()
pathlib.Path(sys.argv[2]).write_text(json.dumps({'source_files':sources,'api':'fender-mcp-native-v1','runtime':'Fender Studio Pro 8.1.1 arm64','sdk_commit':'ad5876b1a7b64aa247f4378c86a7862e7c284ba3','binary_uuid':uuid,'files':files},indent=2)+'\n')
print(package)
print(uuid)
PY
