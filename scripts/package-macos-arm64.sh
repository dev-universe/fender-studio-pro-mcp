#!/bin/sh
set -eu

if [ "$#" -gt 1 ]; then
    echo "Usage: sh scripts/package-macos-arm64.sh [output-directory]" >&2
    exit 2
fi
if [ "$(uname -s)" != Darwin ] || [ "$(uname -m)" != arm64 ]; then
    echo "The archive requires macOS arm64" >&2
    exit 2
fi

repo_root="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
commit="$(git -C "$repo_root" rev-parse HEAD)"
dirty=false
if [ -n "$(git -C "$repo_root" status --porcelain)" ]; then
    dirty=true
fi
output="${1:-$repo_root/artifacts/macos-arm64}"
mkdir -p -- "$output"
output="$(CDPATH= cd -- "$output" && pwd)"
work="$(mktemp -d "$output/.fsp-package.XXXXXX")"
trap 'rm -rf "$work"' EXIT

cargo +1.88.0 build --locked --release --manifest-path "$repo_root/Cargo.toml" --target-dir "$repo_root/target"
sh "$repo_root/native/build.sh" "$work/native"

python3 - "$repo_root" "$work" "$output" "$commit" "$dirty" <<'PY'
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tarfile

repo, work, output = map(Path, sys.argv[1:4])
commit, dirty = sys.argv[4], sys.argv[5] == 'true'
cli = repo / 'target/release/fender-studio-pro-mcp'
version_output = subprocess.check_output([cli, '--version'], text=True)
match = re.fullmatch(r'fender-studio-pro-mcp ([0-9A-Za-z.+-]+)\n?', version_output)
if match is None:
    raise SystemExit('The executable did not return its package version')
version = match[1]
name = f'fender-studio-pro-mcp-{version}-macos-arm64-{commit[:12]}' + ('-dirty' if dirty else '')
stage = work / name
stage.mkdir()
native_dir = 'fender-studio-pro-mcp.native.mac'
native_root = work / 'native' / native_dir
native = json.loads((native_root.parent / 'native-artifact-manifest.json').read_text())
uuid = re.fullmatch(r'UUID: ([0-9A-F-]{36}) \(arm64\) .+', native['binary_uuid'])
if uuid is None:
    raise SystemExit('The native build did not report one arm64 UUID')

for path in (cli, repo / 'LICENSE', repo / 'THIRD_PARTY_NOTICES.md'):
    shutil.copy2(path, stage)
shutil.copytree(native_root, stage / native_dir)
manifest = {
    'schema': 1,
    'version': version,
    'source_commit': commit,
    'working_tree_dirty': dirty,
    'platform': 'macos-arm64',
    'cli': {'name': cli.name, 'sha256': hashlib.sha256(cli.read_bytes()).hexdigest()},
    'native': {
        'directory': native_dir,
        'binary_uuid': uuid[1],
        'runtime': native['runtime'],
        'files': native['files'],
    },
}
(stage / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')


def anonymous_owner(info):
    info.uid = info.gid = 0
    info.uname = info.gname = ''
    return info


archive = work / f'{name}.tar.gz'
with tarfile.open(archive, 'w:gz') as tar:
    tar.add(stage, arcname=name, filter=anonymous_owner)
checksum = work / f'{archive.name}.sha256'
checksum.write_text(f'{hashlib.sha256(archive.read_bytes()).hexdigest()}  {archive.name}\n')
published = output / archive.name
os.link(archive, published)
try:
    os.link(checksum, output / checksum.name)
except OSError:
    published.unlink()
    raise
print(published)
PY
