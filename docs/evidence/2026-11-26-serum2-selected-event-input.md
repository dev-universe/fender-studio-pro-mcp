# Serum 2 CC 74 on selected Studio Pro event inputs

R506 used the Xfer-signed Serum 2 instrument VST3 2.1.5 in Studio Pro
8.1.1 build 113103 on macOS arm64. The named scratch song contained
one instrument track and one exact Serum instance. The transport was
stopped, Main was muted at zero, and the owned Core MIDI source was
the only online MIDI input.

A private EditTask selected each of two host destination ports with
the stock track channel `connectTo` method. Every dispatch used the
same document and verified instrument, with exactly 16 distinct ports
in numeric order. The Core MIDI loopback saw the same wire-channel-1
messages in both cases, CC 74 value 96 followed by 0.

| Selected destination | DSP row that received CC 74 | Matching held reads | Exact neutral reads |
| --- | --- | --- | --- |
| Event Input 2 | `vstp501000074` | 13 | 3 |
| Event Input 16 | `vstp515000074` | 13 | 3 |

Both selected rows read 7559.0550899505615 at value 96 and returned
to 0. In each phase the two watched peer rows stayed at 0. The
programmatic route returned to Event Input 1, confirmed independently
in the stock inspector. Monitoring was turned off, all 2,623 typed DSP
rows returned exactly, and the timeline, mixer and native instance
matched the original state. This qualifies the two observed destination
selections for one instrument. The routing probe remains private and
same-model binding and alternate-port persistence need separate tests.

The owner made 161 requests in 78.753 seconds. The app exited after
98.719 seconds without saving. No notes, playback, or direct DSP
parameter writes were issued. The virtual source and signed scanner
bundle were retired, and 18 managed files and 11 profile files were
restored and verified.

[Default event input delivery](2026-11-26-serum2-default-event-input.md)
