# Serum 2 FX Mod Wheel and Pitch Bend on two alternate Event Inputs

R527 tested the official Xfer-signed Serum 2 FX VST3 2.1.5 in Studio Pro
8.1.1 build 113103 on macOS arm64. An owned empty Music source was
connected to the exact FX insert on a separate Audio receiver. Two
native inspection paths identified the same FX instance. The receiver
had no audio input and was muted, as was Main at zero. There were no
notes, playback or recording.

| Selected destination | MIDI wire channel | CC74 reference row | Mod Wheel row | Pitch Bend row |
| --- | ---: | --- | --- | --- |
| Event Input 2 | 2 | `vstp501000074` | `vstp516000000` | `vstp516000001` |
| Event Input 16 | 16 | `vstp515000074` | `vstp516000000` | `vstp516000001` |

CoreMIDI loopback checked all sixteen wire messages. The trial sampled
fifty rows, including arithmetic peers for all sixteen channels and
both global rows. Only the corresponding CC74 or global control changed
in each verified phase. The repeated controller reads matched 96/127
for the held CCs and 12288/16383 for the held pitch bend, with exact
Float32 conversion. The center message 8192 produced raw
5000.30517578125. The initial configured value was 5000.

The stock Inspector showed Event Input 1 after restoration. Monitoring
and record arm were off. A fresh read-only owner, after removing the
temporary source, compared all 2,623 DSP rows under the same document
and FX binding. The single Pitch Bend current value and its
representations retained the verified wire center. The other 2,622
rows and immutable metadata matched exactly.

The trial used 279 requests in 129.867 seconds. The fresh read used
40 requests in 4.810 seconds. Both MCP owners and the sender exited
normally. The app shutdown wait expired after 180 seconds and the
subsequent process inventory showed that the app was closed. An exit
code was unavailable. The scratch archive was unchanged and no Save
was issued. The official scan copy was retired, 18 managed files and
seven profile files were restored, and one created cache member was
retired. The final profile matched the original file inventory.

This is host-route verification for the two shown wire and destination
pairs. [VST3 SDK lookup coverage](2026-11-26-serum2-midi-channels.md)
