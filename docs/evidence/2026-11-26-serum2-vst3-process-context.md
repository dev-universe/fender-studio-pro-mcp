# Serum 2 and Serum 2 FX VST3 process context

R545 queried the two components in the Xfer-signed Serum 2 2.1.5
VST3 with the Steinberg SDK. Each query followed component
initialization and 48 kHz realtime setup in a separate process.
The Apple Silicon slice ran natively and the Intel slice under
Rosetta. All four queries returned the same process-context
requirements mask, `0x456`. [Steinberg interface reference](https://steinbergmedia.github.io/vst3_doc/vstinterfaces/classSteinberg_1_1Vst_1_1IProcessContextRequirements.html)

| Requested information | Mask |
| --- | --- |
| Continuous sample time | `0x002` |
| Musical project time | `0x004` |
| Musical cycle information | `0x010` |
| Tempo | `0x040` |
| Transport state | `0x400` |

| Slice | Requirements getter | Process entry |
| --- | --- | --- |
| arm64 | `0xFB9BC` | `0x73630` |
| x86_64 | `0x123DA0` | `0x86390` |

The requirements getter reads a 32-bit member at interface-relative
offset `0x20` on both slices. REA with Ghidra followed the arm64
process entry to `0x6168C`. It checks the playing bit and handles
a change to stopped. The processor reads musical project time
and tempo behind their respective field-validity bits. One
tempo conversion path uses 120 BPM when the context exists
and its tempo is marked invalid. The cycle logic checks cycle
validity and the active-cycle bit. It also compares the valid
musical position against the previous position to detect a
backward jump. [Steinberg ProcessContext layout](https://steinbergmedia.github.io/vst3_doc/vstinterfaces/structSteinberg_1_1Vst_1_1ProcessContext.html)

Craft Anything and an independent read of the shipped Mach-O
images matched 18 arm64 instruction words and five Intel getter
instructions. The analysis establishes which fields the two
variants ask the host to supply and identifies specific reads
in the arm64 processor. Actual context delivery in Studio Pro,
the use of individual cycle boundaries and continuous sample
time, and resulting changes to a rendered preset remain open.
The SDK probe performed setup and queries without starting
audio processing or opening Studio Pro.
