# Studio Pro 8.1.1 VST3 process context delivery

R546 traced the arm64 `vstservice` module from the local Studio Pro
8.1.1 Build 113103 installation. Its UUID is
`20D9ADCB-D623-32A7-9156-A11CB705FF75` and the slice SHA-256 is
`f1269e1c33a27adccb8d4501bd5f4adb16ea99e1f6a16e2fd892dc72e79267a3`.

| Path | Evidence |
| --- | --- |
| Processor binding | The wrapper at `0x660C4` queries the exact Steinberg `IAudioProcessor` interface and stores its pointer at wrapper offset `0x1A8`. |
| Owned context | The wrapper sets `ProcessData::processContext` to the context at wrapper offset `0x290`. `ProcessData` starts at wrapper offset `0x238` and its context pointer is 72 bytes into the struct, at wrapper offset `0x280`. |
| Timing values | The helper at `0x6BA0C` fills the context with sample rate, project and continuous sample positions, system time, musical position and cycle positions, tempo, and time signature. Sample positions and musical positions use the engine's supplied origins. |
| Validity and transport | The helper sets validity bits for continuous samples, system time, musical position, tempo, bar position, the cycle range, and time signature. It maps source state into VST3's playing, cycling, and recording bits. |
| Transfer | At `0x6B8F0` the wrapper calls the context helper. At `0x6B904` it passes the populated `ProcessData` to the `IAudioProcessor::process` slot. |

REA/Ghidra followed the wrapper, its context builder, and the
virtual call. Craft Anything and direct Mach-O reads matched 27
instruction words across four procedures. An independent SDK
build matched 13 struct offsets and the binary contains both
Steinberg interface IDs. [Steinberg ProcessContext layout](https://steinbergmedia.github.io/vst3_doc/vstinterfaces/structSteinberg_1_1Vst_1_1ProcessContext.html)

This establishes the adapter code path for a VST3 processor in
the local build. The exact Serum 2 and Serum 2 FX 2.1.5 arm64
processor reads are documented in [PR #319](https://github.com/dev-universe/fender-studio-pro-mcp/pull/319).
The host's per-plugin mask negotiation and the values during
live Serum playback remain unverified. This check did not
launch Studio Pro, modify the app, or process audio.
