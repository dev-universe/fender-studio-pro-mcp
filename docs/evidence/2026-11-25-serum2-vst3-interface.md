# Serum 2 and Serum 2 FX 2.1.5 VST3 interface

The macOS bundle came from an Apple-notarized installer signed with Xfer's
Developer ID, team `42K3SNWUP7`. The VST3 and AU bundles passed strict signature
verification for both architectures. This runtime evidence applies to the
Apple Silicon VST3 slice, UUID `8A7D619E-C065-34E6-A53F-E7FBE6E21141` and SHA-256
`bba473ac531dd58f8ddef8d6d763fc45534f95f6e82498ab92394209a244c148`.

## Class and parameter discovery

The signed VST3 module manifest and the isolated SDK factory returned the same
four class IDs.

| Variant | Audio module class ID | Edit controller class ID |
| --- | --- | --- |
| Serum 2 | `56534558667350736572756D20320000` | `56534558667343736572756D20320000` |
| Serum 2 FX | `56534558667351736572756D20322066` | `56534558667344736572756D20322066` |

Both controllers returned 2,623 unique IDs, of which 541 were marked automatable.
Three rows differed in normalized defaults. Serum 2 had no audio input and
one stereo output. Serum 2 FX had one stereo input and one stereo output.
Both had one event bus in each direction. The signed AU registered
`aumu/Xf2X/XFER` and `aumf/Xf2Y/XFER` against the same `AUWrapperFactory` export.

## Static analysis

REA with Ghidra 12.1.4 analyzed `GetPluginFactory` at `0x16fc` in the signed VST3
and the signed AU factory at `0x116b8`. The AU constructor resolves an external
module's `bundleEntry` through Core Foundation. Craft Anything 0.6.1 in Deep
mode independently resolved the VST3 factory and the bundle entry and exit
at `0x52d88c` and `0x52da04`. Its 27,725 function candidates included all
26,802 Mach-O function starts. It left 1,590 indirect jumps unresolved.

The SDK probe found the same method offsets for both VST3 classes. These
signed-image offsets differ from the earlier installed-image observations.
MCP code can work through the public interfaces and exact host class identity.
[Ableton MCP study](https://github.com/dev-universe/ableton-mcp/blob/3c83560/docs/serum-2-host-interface.md)

| VST3 method | Apple Silicon image offset |
| --- | --- |
| `IAudioProcessor::process` | `0x73630` |
| `IComponent::setState` | `0x60eb4` |
| `IComponent::getState` | `0x6115c` |
| `IEditController::getParameterInfo` | `0xfcc90` |

## SDK processing and state

The isolated host rendered 96,000 finite, nonzero stereo frames per variant
at 48 kHz. Serum 2 received MIDI and Serum 2 FX received a stereo input
signal. After state restoration, fresh renders and state bytes matched per
variant. The VST3 edit controller independently read and restored three
scalar controls per variant. A Main Vol point sent to the processor changed
both renders. [Ableton VST3 host probe](https://github.com/dev-universe/ableton-mcp/blob/3c83560/tools/serum_host.cpp)

The observed processor states contained 352 bytes for Serum 2 and 356 for
Serum 2 FX. The Ableton MCP decoder verified the XferJson metadata and
compressed frame. Both decoded maps included `demoVersion: true` and product
metadata. The editable oscillator and FX rack fields were absent. These
observations establish default demo-state replay and scalar control. Full
patch editing and Studio Pro integration require separate evidence.
