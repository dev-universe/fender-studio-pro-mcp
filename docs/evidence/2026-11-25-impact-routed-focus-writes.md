# Impact cutoff writes on two MIDI-selected pads

R404 used Studio Pro 8.1.1 Build 113103 on macOS arm64. The isolated song
had one Impact instrument, one MCP owner, stopped transport, and Main
muted at zero. A temporary receive-only CoreMIDI source sent channel-1
note-on/off pairs through the monitored Impact track. CoreMIDI loopback
confirmed all eight messages before the live trial.

With Pad Focus enabled, note 36 selected pad 0. Note 37 selected pad 1
and note 53 selected pad 17. Indexed cutoff markers of 2,500 and 7,500
gave these two pads distinct raw values and matching normalized Float32
bits at the Focus alias. The editor displayed bank B, F2 after note 53.

| MIDI note | Indexed cutoff | Exact raw round trip |
| --- | --- | --- |
| 37 | `pad[1].filter.cutoff` | 2,500 → 5,000 → 2,500 |
| 53 | `pad[17].filter.cutoff` | 7,500 → 5,000 → 7,500 |

Each test wrote only `focus.filter.cutoff`. Two independent full DSP reads
showed that the alias and the selected indexed cutoff were the only changed
rows. Both had the exact requested raw value. A fresh binding and provider
read preceded recovery. Two further reads confirmed the selected state
returned before the next note.

The final note 36 returned the editor to bank A, C1. The trial restored
both indexed markers, monitoring and the Pad Focus switch. Two complete
reads matched all 4,262 original DSP records, including the unchanged
tracks, timeline and mixer checks. The app ran for 255.751 seconds. MCP
completed 1,015 requests in 203.668 seconds. The scratch closed without
saving and matched its source bytes. The app and temporary source exited
normally. The normal 18 managed files and 10 restored profile files were
verified.

This covers one Focus alias on two MIDI-selected pads. Other Focus aliases
and arbitrary raw Float64 recovery remain separate, particularly values
whose normalized Float32 representations introduce rounding. The fixture
contained empty pads and exercised no sample playback or song persistence.
