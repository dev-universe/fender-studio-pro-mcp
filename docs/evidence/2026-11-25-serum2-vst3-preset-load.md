# Serum 2 VST3 preset interoperation with Studio Pro

The R318 scratch song used Studio Pro 8.1.1 build 113103 on macOS arm64 with
the Xfer-signed Serum 2 and Serum 2 FX 2.1.5 VST3 in demo mode. The two
synthetic `.vstpreset` files came from the earlier Ableton MCP experiment.
Each was a version-1 VST3 preset container with the exact target's class ID
and one `Comp` chunk. The isolated Steinberg SDK host also accepted those
two files and rendered measurably different audio from the initial patches.
[Ableton preset format and tool](https://github.com/dev-universe/ableton-mcp/blob/3c83560/docs/serum-presets.md)

Studio Pro opened both files through **Preset Menu → Load Preset File**.
The Serum 2 FX editor showed Distortion followed by Equalizer, and a
parameter read reported 13 changed values out of 2,623. The instrument's
Env 1 Release increased from 15 ms to 150 ms on screen. Its parameter
read reported 15 changes and normalized release moved from 0.2150 to 0.34213.
The independent enumerations took 4.14 and 4.90 seconds and were bound to the
two distinct instances. [Studio Pro preset commands](https://fenderstudiopromanual.fender.com/en/Content/Mixing_Topics/Effects_Signal_Routing.htm)

REA with Ghidra located the Studio Pro Apple Silicon `vstservice` class
registration for `VST3 Preset Handler` and the separate registration of
the `.vstpreset` extension with MIME type `application/x-steinberg-vstpreset`.
The observed host menu can therefore use its normal VST3 preset path.
This static result identifies the host handler. A direct MCP receiver
for this handler has yet to be verified.

The host also exported an instrument `.vstpreset`. Its 352-byte `Comp`
chunk decoded to demo metadata. The imported file's 1,294-byte `Comp`
chunk contained editable oscillator, envelope, modulation and FX maps.
The export therefore did not preserve the applied instrument data in
this demo configuration. The test establishes portable preset input via
the host menu. Full editable state export and production MCP file
application still require separate evidence.

The scratch song closed without saving after 292.062 seconds. The two
source presets and the original installation were preserved. Ten Studio
Pro configuration and cache files were restored. The temporary signed
VST3 was removed from the scanner directory, and all 18 managed MCP files
matched their original bytes.
