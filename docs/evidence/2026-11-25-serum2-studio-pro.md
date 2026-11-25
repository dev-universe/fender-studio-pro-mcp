# Serum 2 VST3 in Studio Pro

On Studio Pro 8.1.1 build 113103 and macOS arm64, the R316 scratch song loaded
the Xfer-signed Serum 2 and Serum 2 FX 2.1.5 VST3. A process mapping and a digest
check identified the exact staged binary. The signed bundle ran in demo mode.
The first sandbox launch exposed AU catalog entries only. The second launch
allowed enumeration of the staged VST3 folder and refreshed the VST3 cache.

The public `serum2_load` entry point created a separate Music track for Serum 2
and inserted Serum 2 FX on an existing stereo channel. The receipts agreed
with an independent native scan of both instances. Studio Pro exposed 2,623
DSP parameter rows per instance. For each product, `A Level` changed from
0.75 to the float32 representation of 0.715 and returned to 0.75 on separate
reads. The full set of typed writable values and the opaque component
fingerprints agreed with their corresponding baselines after restoration.
The instruments were addressed by separate instance IDs with transport
stopped and the Main output at zero gain.

The official plug-in editors also opened two synthetic version-11
`.SerumPreset` files reused from the earlier Ableton MCP experiment. The
instrument displayed the loaded preset title. The effect displayed the title
and the Distortion → Equalizer chain. Separate enumerations observed 12
changed DSP parameter values per product. This preset load used the Serum
editor's file picker. The MCP entry point in this change loads plug-in
instances. Native preset authoring, DSP audio in Studio Pro and save/reopen
persistence are outside the measured results.
[Ableton preset experiment](https://github.com/dev-universe/ableton-mcp/blob/3c83560/docs/serum-preset-interoperability.md)

The independent readback and FX insertion used 126 MCP requests in 13.65
seconds. The two separate preset readbacks took 4.27 and 3.72 seconds. The
scratch song closed without saving. Ten original Studio Pro configuration
and cache files were restored. All 18 managed MCP files and the original
scratch file matched their original bytes. The temporary staged plug-in
was removed from the scan directory.

R449C subsequently verified one arrangement note using the same official
instrument, a separate downstream DSP observer and one Play/Stop cycle.
[Arrangement playback verification](2026-11-26-serum2-arrangement-playback.md)
