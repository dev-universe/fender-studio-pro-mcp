# Multi Instrument channel-mode MIDI observations

R397 used Studio Pro 8.1.1 build 113103 on macOS arm64, one selected
scratch track, and one receive-only virtual Core MIDI endpoint in its
All Inputs route. The fixture held two exact Mai Tai children, no
arrangement events, stopped transport, and zero Main output. A private
loopback matched all 13 transmitted messages on MIDI channel 1. The test
used the defined data values for channel-mode controls CC120–127.
[MIDI Association control and mode table](https://midi.org/midi-1-0-control-change-messages)

| Messages | Observed DSP effect |
| --- | --- |
| CC121 value 0 | Held CC1 remained at 7559.0550899505615 in the parent and both children. A separate CC1 value 0 returned all three to zero. |
| CC122 values 127 and 0 | Parent `control122` changed 0 → 10000 → 0. The two child providers matched. |
| CC126 values 1 and 0 | Parent `control126` changed 0 → 78.74015718698502 → 0. The two child providers matched. |
| CC120, CC123, CC124, CC125 and CC127 with value 0 | None of the 716 exposed DSP rows changed in the empty fixture. |

Two separate complete reads agreed after every message. The first
three rows establish observed facade and wheel behavior on the tested
route. Notes, internal polyphony, and local/omni routing modes require
their own functional witnesses.

All original 150 parent and 566 child rows matched at closure. The
owner used 378 requests in 71.777 seconds. The app ran for 122.253
seconds. The scratch closed without saving and remained byte-identical.
The sender, owner and app exited normally, and 18 managed files and
ten profile files including the original MIDI configuration were restored.
