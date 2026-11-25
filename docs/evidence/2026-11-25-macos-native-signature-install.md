# macOS native bundle signature installation

The arm64 local archive produced from clean `49f7e76` passed the SHA-256 sidecar check. Its native package had the four expected legacy files. Gatekeeper rejected the locally built CLI, and strict verification of the native bundle reported missing signed resources. A copied bundle signed ad hoc as a complete bundle added `Contents/_CodeSignature/CodeResources` and passed strict local signature verification.

The old CLI installed that signed fixture in an isolated profile while dropping `CodeResources`. Strict verification of the installed bundle failed. With the updated installer, an isolated profile could install the legacy package, update to the signed fixture, verify `CodeResources` byte for byte, pass `codesign --verify --strict` on the installed bundle and pass the native installer `doctor` check. Restoring the legacy package returned its exact four files and backed up the old signature resource. A subsequent signed install and uninstall removed all five managed files.

Focused tests cover legacy and signed package updates, modified signature protection and backup, and recovery of the original file bytes and manifest when the signature destination is unwritable. Test signing was local and ad hoc. Developer ID signing, notarization, public download and Gatekeeper acceptance remain unverified.
