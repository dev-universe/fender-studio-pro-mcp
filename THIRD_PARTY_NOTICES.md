# Third-party notices

The JavaScript control surface and EditTask assets under `assets/` derive from
[b4blue/fender-studio-mcp](https://github.com/b4blue/fender-studio-mcp/tree/7f74f0a150df7bc178ad4cb2ae00c556994758dc),
which derives from [NeanderthalMan/studio-one-mcp](https://github.com/NeanderthalMan/studio-one-mcp).

Copyright (c) 2026 Adam Morgan.
Copyright (c) 2026 Boris Miljkovic.

These assets are MIT licensed. The complete upstream license is preserved in
[licenses/studio-one-mcp-MIT.txt](licenses/studio-one-mcp-MIT.txt).

Local modifications add protocol and document checks, disable arbitrary script
execution, and extend native plugin access. Fender application binaries, SDK
source files, factory presets, and proprietary resource maps are not distributed
by this repository.

## Native interoperability

The optional native crates are independently authored Rust interface bindings and
interop code. Public API/ABI declarations were checked against official CCL
5.1.2, commit `ad5876b1a7b64aa247f4378c86a7862e7c284ba3`. No CCL headers,
implementation, vendor binary, resource, preset or SDK library is vendored.
The compiled add-on resolves services from the already running host. Optional
compiler checks consume a separately provided SDK checkout under its own terms.
Exact-version private DSP interface facts are documented as interoperability
facts, not public SDK guarantees.

