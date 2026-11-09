# Native integration

Native integration is an **opt-in macOS component** for Fender Studio Pro
**8.1.1 Build 113103, arm64**, using the public CCL **5.1.2** interface ABI.
It supplies access that the embedded scripting bridge cannot provide directly.
The Rust MCP server and embedded bridge retain their existing process/document
guards. Installing a native add-on is a separate, explicit action; building the
crates under this directory does not install or load anything.

## Parameter library

[`parameter-api`](parameter-api/README.md) is an independently authored, std-only
Rust crate. An owning native integration passes it an already resolved, owned
IParameter reference and validated text service pointers. It describes native
values and supports guarded raw, normalized and display-text data operations.
It has no target resolver, IPC, host loader or arbitrary evaluator. A path
dependency is enough; normal Cargo build/test requires no SDK or DAW installation.

All use and destruction must occur on the verified GUI thread. The wrapper is
neither Send nor Sync. The integrating add-on must retain document/model/target
identities, serialize access across the MCP/bridge/native paths, prevent
reentrant writes and provide context checks immediately around dispatch and
after native temporary cleanup. A lost context after dispatch produces an
unknown outcome, not a verified write or permission to blindly restore state.

The generic helper excludes command execution, opaque image/object writes,
unknown parameter kinds, stateful formatter previews and List label writes.
Its portable mocks verify ABI-carrier ownership and failure paths; they do not
establish that every supported data type works on every native plug-in. The
separate actual 8.1.1 validation covered External Effect's three scalar fields
in Mono and Stereo with exact raw restoration. That result does not imply
generic formatter/string support has been verified in the app.

## ABI and source provenance

The authoritative declarations are official CCL tag
[`ccl-5.1.2`](https://github.com/cclsoftware/ccl-framework/tree/ad5876b1a7b64aa247f4378c86a7862e7c284ba3),
commit `ad5876b1a7b64aa247f4378c86a7862e7c284ba3`. This repository contains
independently authored Rust interface declarations and interoperability facts,
plus compiler-only calls against a separately obtained SDK. It vendors no CCL
headers, implementation, proprietary app resource, raw song, audio or app log.
The SDK has its own licensing terms; the source probes do not link or ship it.
See [the exact parameter ABI map](parameter-api/ABI.md).

Runtime ABI compatibility must be checked **before any CCL virtual call**. The
reviewed 8.1.1 arm64 identities are:

| Image | Mach-O LC_UUID |
| --- | --- |
| Fender Studio Pro 8.1.1 main executable | `6B547378-B3BB-322A-87C9-901493FD2BED` |
| Its cclgui framework | `1A8C66AE-250D-3C7F-801C-6904D5082F57` |

Reject a different image/runtime. CCL5.2 inserted two IParameter methods while
preserving the IID, so a successful QueryInterface or matching product name
does not establish a compatible call layout. New host versions need separate
source/installed-binary validation. The parameter library itself performs no
module discovery or UUID inspection; those checks belong to its owning loader.

## Build and verification

```sh
cargo +1.88.0 fmt --manifest-path native/parameter-api/Cargo.toml -- --check
cargo +1.88.0 test --manifest-path native/parameter-api/Cargo.toml
cargo +1.88.0 clippy --manifest-path native/parameter-api/Cargo.toml --all-targets -- -D warnings
```

These commands run local mocks only. Optional C++ source validation requires a
separate official SDK checkout and an explicit path, as documented in the
crate. There is no implicit download, SDK build script or temporary checkout
path in an ordinary Cargo build. No native loader operation may patch a vendor
bundle, inject into an existing process or bypass host security/signature checks.
