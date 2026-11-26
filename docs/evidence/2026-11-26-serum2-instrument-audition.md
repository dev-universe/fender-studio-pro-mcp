# Serum 2 editor audition in Studio Pro

R445IAB used the official Xfer-signed Serum 2 VST3 2.1.5 in demo mode,
Studio Pro 8.1.1 build 113103, and macOS arm64. The MCP applied the same
processor-only carrier as [R319](2026-11-25-serum2-mcp-vst3-preset.md).
The independent typed read verified 15 changes across the 2,623 parameter
rows on the selected instrument. Two reads after the audition matched all
2,623 post-apply rows exactly.

One press and release on the editor's C3 key supplied the input. The
three-second hold yielded seven stereo samples wholly inside the
conservative measured press interval. A separate downstream Rumble Native
instance exposed the two read-only input peaks on its full 39-row DSP page.
Each sample was bound to the same current document, track and instance.
The track had no external input, arm or monitor. The transport was stopped,
the timeline was empty, and Main was muted with volume at zero.

| Observation | Readings |
| --- | --- |
| Before the key press | Three stereo samples at exact zero |
| During the key press | Seven active samples on both sides, with raw medians of 3158.7401 left and 3158.7407 right |
| After release | Three consecutive stereo samples at or below raw 0.0001, confirmed 9.179 seconds after the editor input call returned |

The earlier R445IA run detected six active stereo samples and preserved the
same 2,623 post-apply rows. Its six-second observation was too short to
confirm quiet return. R445IAB extended the meter-only observation limit to
20 seconds with the same silence threshold and hold. The measured interval
describes metering behavior. External MIDI routing, physical audio output,
and editable preset persistence are outside this result.

The successful owner made 319 requests in 74.821 seconds. Studio Pro and
the MCP owner exited normally. The scratch song was closed without saving.
All 18 managed files and ten profile files were restored and checked. The
temporary VST3 was retired and the scan destination was verified vacant.
