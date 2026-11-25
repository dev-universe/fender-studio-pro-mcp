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
The observed host menu uses that VST3 preset path. R319 separately
verified calling the same handler through `serum2_preset_apply`.
[MCP receiver and live evidence](https://github.com/dev-universe/fender-studio-pro-mcp/pull/108)

## Load order and the inactive scale index

R318 loaded FX first, and its inactive `Scale` read back as 51. The
instrument read back as 52. R319 applied the same files through MCP in the
opposite order, and the first and second applies again returned 51 and 52.
The two processor maps carry twelve zero entries and an empty `scaleName`.
The scale selector stayed inactive in both editors.

The signed VST3 state loader at arm64 entry `0x82d1f4` and x86_64 entry
`0x919bb0` compares the input scale and raw name to a process-wide table.
For an unmatched entry it uses `User Scale` as the stored name when the
incoming name is empty. It assigns the next index in the table, whose
capacity check is 64, and passes the index to VST3 parameter 14000001
(Studio Pro tag 14000002). Craft Anything Deep independently confirmed
the arm64 comparisons, fallback name, capacity check, and index write
in the instruction listing. These static observations explain the
load-order result across the two host trials.

The host also exported an instrument `.vstpreset`. Its 352-byte `Comp`
chunk decoded to demo metadata. The imported file's 1,294-byte `Comp`
chunk contained editable oscillator, envelope, modulation and FX maps.
The export therefore did not preserve the applied instrument data in
this demo configuration. The test establishes portable preset input via
the host menu. Full editable state export still requires separate evidence.

The scratch song closed without saving after 292.062 seconds. The two
source presets and the original installation were preserved. Ten Studio
Pro configuration and cache files were restored. The temporary signed
VST3 was removed from the scanner directory, and all 18 managed MCP files
matched their original bytes.
