# Serum 2 FX internal stereo signal in Studio Pro

R445FXC used the official Xfer-signed Serum 2 FX VST3 2.1.5 in demo mode
on Studio Pro 8.1.1 build 113103 and macOS arm64. MCP applied the
[processor-only carrier](2026-11-25-serum2-mcp-vst3-preset.md) to the exact
effect in zero-based slot 2. Independent reads confirmed 13 changed values
on application and all 2,623 expected typed rows after the tone cycle. The
editor showed Distortion followed by Equalizer.

The scratch stereo channel used Tone Generator, Rumble Native, Serum 2 FX,
and a second Rumble Native in that order. The two Rumble instances supplied
the separate read-only output and input stereo peaks immediately before
and after the FX. Each instance exposed a complete 39-row DSP page.
The channel had no hardware input or sends. Monitor and record arm were
off. Main was muted with volume at zero. The transport remained stopped.

| Observation | Result |
| --- | --- |
| Initial Off | Three pairs with both stereo taps at exact zero |
| One 440 Hz sine at −24 dB | Six paired samples within the 350 ms pairing bound, with median raw peaks of 668.0526 upstream and 1512.9276 downstream on each side |
| Output/input during the tone | 2.2634–2.2662 on both sides |
| Independent Tone Off | 3.563 seconds from the single On request |
| Quiet after Off | Three consecutive pairs at or below raw 0.0001, confirmed 8.699 seconds after the Off readback |

The preceding R445FXB run proved the same single On/Off cycle and active
amplitude change. Its shorter read-only observation ended before the meters
settled. R445FXC allowed up to 20 seconds after independently verified Off
and kept the same active and quiet criteria. This was an internal amplitude
measurement with one input and carrier. Bypass response, spectral response,
physical output and editable preset persistence remain separate questions.

The owner made 294 requests in 116.116 seconds. Both processes exited
normally and the scratch song was closed without saving. All 18 managed
files and ten profile files were restored and checked. The temporary VST3
was retired and the scan destination was verified vacant.
