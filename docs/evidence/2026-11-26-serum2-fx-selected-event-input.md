# Serum 2 FX CC 74 on alternate Studio Pro event inputs

R512 used the Xfer-signed Serum 2 FX 2.1.5 VST3 in Studio Pro 8.1.1
build 113103 on macOS arm64. The scratch song contained a Music track
with no synth, routed to the exact Serum 2 FX instance in slot 0 of
a separate stereo Audio track. The Main and receiver were muted.
No notes, audio clips or playback were in use. The owned Core MIDI
source was the only online input.

The private EditTask used the stock track creation and connection
methods. A fresh native lookup of the Music destination matched the
FX insert's instance UUID throughout. On both test paths, independent
Core MIDI loopback observed channel-1 CC 74 value 96 followed by 0.

| Selected destination | DSP row that received CC 74 | Matching held reads | Exact neutral reads |
| --- | --- | --- | --- |
| Event Input 2 | `vstp501000074` | 13 | 3 |
| Event Input 16 | `vstp515000074` | 13 | 3 |

Both selected DSP rows reported 7559.0550899505615 and returned
to 0. The two watched peers stayed at 0. The test restored Event
Input 1, confirmed in the stock inspector, and turned off monitoring.
All 2,623 typed DSP rows, the bound instance, and the track and mixer
readbacks matched their starting state. This qualifies the requested
alternate inputs for one FX insert. The temporary diagnostic remains
private. Saved alternate FX ports and same-model peer selection have
separate verification requirements.

The owner made 171 requests in 69.145 seconds. The app exited after
83.527 seconds without saving. The virtual source and signed scanner
bundle were retired, and 18 managed files and 11 app profile files
were restored and verified.

[Serum 2 FX default event-input delivery](2026-11-26-serum2-fx-live-midi.md)
