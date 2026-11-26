# Input Filter all 128 GUI key lanes and release in Studio Pro 8.1.1

R386 exercised Studio Pro 8.1.1 build 113103 on macOS arm64. Input Filter
and an independent downstream Chorder were bound to their separate
Note FX slots on an owned instrument track. Main was zero, transport was
stopped, the track held no events, and only its monitor was enabled for
the note test. There were no external MIDI messages.

MCP widened the original pitch bounds from 48–72 to 0–127, with separate
readback of both endpoints. Each of the 128 `keyboard.pitchN.guiInput`
lanes was then pressed with the same raw value 5000 and released to zero.
For each lane, Input Filter showed its own live hold and the downstream
Chorder independently showed exactly MIDI note `N - 1` in two reads of
its entire 128-input set. The scheduled flag stayed neutral. Two downstream
reads after each release had no held notes. MIDI 0 and 127 passed the same
checks as the interior notes.

The original bounds and monitoring state were restored. All 388 Input
Filter and 908 Chorder DSP rows, the timeline, and the mixer matched the
original projections. The owner sent 1,615 requests in 123.855 seconds.
The app closed without saving after 154.160 seconds. The scratch bytes,
18 managed installation files and 9 application settings files matched
their originals. Separate virtual MIDI testing covered the factory range
boundaries. Physical hardware and audio were outside this test.
