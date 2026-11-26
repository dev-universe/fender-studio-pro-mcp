# Serum 2 CC 74 on the default Studio Pro event input

R503 used the Xfer-signed Serum 2 instrument VST3 2.1.5 and Studio Pro
8.1.1 build 113103 on macOS arm64. The test track received one owned
Core MIDI virtual source, routed to the Serum instance with
**Event Input 1** selected. Main was muted at zero and the transport
was stopped. The loopback independently verified the distinct MIDI
status bytes for the two input channels.

| Virtual input | Value sent | First-channel DSP row | Second-channel DSP row |
| --- | --- | --- | --- |
| MIDI channel 1 CC 74 | 96, then 0 | 0 → 7559.0550899505615 → 0 | 0 |
| MIDI channel 2 CC 74 | 96, then 0 | 0 → 7559.0550899505615 → 0 | 0 |

The input from channel 2 reached the first-channel DSP row on this
track route. The peer observation stopped the run before the planned
channel 16 message. Both tested neutral messages returned all three
watched rows to their baseline. A fresh read-only owner rebound the same
document and native instance after the virtual source ended. All 2,623
typed rows matched their initial read and monitoring was off.

The editor's output menu offered Event Input 1–16 and showed Event
Input 1 selected. The observation applies to that routing selection.
[Independent SDK channel mapping](2026-11-26-serum2-midi-reference.md)

The first owner made 111 requests in 75.814 seconds. The read-only
follow-up made 31 requests in 4.239 seconds. No notes, playback, direct
parameter writes or song Save were issued. Both owners, the virtual
source and Studio Pro exited. The temporary signed bundle was retired
and 18 managed files and 11 profile files were restored and verified.
