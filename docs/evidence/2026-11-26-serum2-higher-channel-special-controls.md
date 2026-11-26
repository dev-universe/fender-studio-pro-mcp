# Serum 2 Mod Wheel and Pitch Bend on two alternate Event Inputs

R526 used the Xfer-signed Serum 2 instrument VST3 2.1.5 in Studio Pro
8.1.1 build 113103 on macOS arm64. The owned scratch contained one
monitored Music track with no notes, no recording and no playback.
Main was muted at zero. One receive-only virtual MIDI source was
independently enumerated and its sixteen messages checked by
CoreMIDI loopback. The route and actual DSP receiver had separate
checks.

| Selected destination | MIDI wire channel | CC74 reference row | Mod Wheel row | Pitch Bend row |
| --- | ---: | --- | --- | --- |
| Event Input 2 | 2 | `vstp501000074` | `vstp516000000` | `vstp516000001` |
| Event Input 16 | 16 | `vstp515000074` | `vstp516000000` | `vstp516000001` |

CC74 and Mod Wheel used MIDI values 96 and 0. Each held value
settled at 7559.0550899505615 on the corresponding row. The bend
used 12288 and wire center 8192. Its exact raw readings were
7500.457763671875 and 5000.30517578125. Each phase included at
least three stable active reads and three stable neutral reads.
Fifty named rows were watched, covering all sixteen ordinary
lanes for CC74, CC1 and pitch bend and the two global rows.
The other watched rows remained unchanged in each phase.

The host returned to Event Input 1, also seen in the stock
Inspector, and monitoring was disabled. The first follow-up
comparison stopped because it treated value aliases inside the
nested response as immutable metadata. A fresh read-only owner
in the same live song checked all 2,623 typed rows against the
original pages. Only Pitch Bend's current scalar, display and
normalized representations changed. Default raw 5000 became
5000.30517578125 after the emitted wire center, with matching
immutable metadata and 2,622 unchanged rows. No MCP setter
wrote the controller back.

The live trial used 270 requests in 123.115 seconds. The fresh
read used 38 requests in 4.843 seconds. The app exited normally
after 190.519 seconds, with both owners and the virtual source
closed. No Save, note, playback or recording was performed.
The original scratch bytes remained exact. The official scan
copy was retired, and 18 managed files and eleven profile files
were restored and verified.

This qualifies the two named host event routes for the signed
instrument. The SDK's channel lookup remains a separate reference.
[SDK mapping coverage](2026-11-26-serum2-midi-channels.md)
