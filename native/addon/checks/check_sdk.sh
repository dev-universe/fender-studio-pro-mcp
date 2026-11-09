#!/bin/sh
# Optional source-only ABI verification; no DAW/library is loaded.
set -eu
if [ "$#" -ne 1 ]; then
    echo "Usage: sh checks/check_sdk.sh /path/to/ccl-framework-5.1.2" >&2
    exit 2
fi
checks_root="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
sdk_root="$(CDPATH= cd -- "$1" && pwd)"
python3 "$checks_root/verify_sdk.py" "$sdk_root"
check_tmp="$(mktemp -d "${TMPDIR:-/tmp}/fender-addon-abi.XXXXXX")"
trap 'rm -rf "$check_tmp"' EXIT HUP INT TERM
clang++ -std=c++20 -arch arm64 -I"$sdk_root" -Werror -Wno-return-type-c-linkage \
    "$checks_root/layout_check.cpp" -o "$check_tmp/layout-check"
"$check_tmp/layout-check"
clang++ -std=c++20 -arch arm64 -I"$sdk_root" -Werror -Wno-return-type-c-linkage \
    -O1 -S "$checks_root/abi_calls.cpp" -o "$check_tmp/abi-calls.s"
python3 "$checks_root/check_abi.py" "$check_tmp/abi-calls.s"
