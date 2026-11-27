# Mustang and Rumble model module provenance

R543 examined the currently installed macOS Studio Pro 8.1.1
build 113103. The outer application bundle passed strict local
signature verification. Its signature is ad hoc and has no Team
Identifier. The resource seal contains the exact SHA-256 of
the installed shared Mustang/Rumble model module. That module
has no `LC_CODE_SIGNATURE` in either its arm64 or its x86_64
Mach-O slice. Architecture-specific `codesign` display also
reported both slices unsigned.

The earlier live Mustang trial recorded a model loader failure,
an empty Add Model surface and empty bounded MCP lists on this
installation. The present file evidence matches that module.
It places the catalog observation in the tested local
installation. Vendor provenance and the model catalog from an
independently obtained official build remain unverified.
[Earlier live observation](2026-11-25-mustang-model-selector.md)

The vendor's installer guide directs registered users to
MyFender's **My Gear** download page. The account page is
signed out in the test browser. No independently verified
vendor macOS installer was available during R543, and no
distribution comparison or model-activation test was run.
[Official installation guide](https://support.presonus.com/hc/en-us/articles/41833587461133-How-to-download-and-install-Fender-Studio-Pro)

R543 read the two exact Mach-O slices, outer signature and
resource seal. It ran no plug-in code and changed no installed
files.
