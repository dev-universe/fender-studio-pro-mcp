# Serum 2 FX in the Studio Pro instrument destination list

R461 used the Apple-notarized Xfer Serum 2 2.1.5 VST3 bundle in Studio
Pro 8.1.1 build 113103 on macOS arm64. The synth and effect had distinct
exact native instance IDs. The scratch song had one Music track
assigned to Serum 2 and one isolated stereo Audio track. Main was muted
at zero. The Audio track was muted and transport, arm, monitor and
Sends were inactive.

The selected Music track's stock Inspector **Out** device menu showed
the following entries in order. The Korean `없음` entry means None.

| Stage | Visible choices | Checked choice |
| --- | --- | --- |
| Before the FX insert | None, Serum 2 | Serum 2 |
| After the FX insert | None, Serum 2, Serum 2 FX | Serum 2 |

The only tested rack change was one `serum2_load` for the exact FX
class at slot 0 on the separate Audio track. The new destination appeared
with the distinct FX product name. Both menus were dismissed with Escape
and the same Music track retained its original synth selection. The
same track and mixer projections and all 2,623 DSP rows per active
instance matched their bound baselines. This establishes stock
destination visibility for the tested case. Assigning the destination
and confirming received MIDI need an independent trial.

The owner made 141 requests over 83.855 seconds. There were no MIDI
messages, destination writes, transport commands, or saves. Studio Pro
and the owner exited normally, the temporary signed bundle was retired,
and the 18 managed files and 10 app profile files were restored and
verified.
[Studio Pro Track and Event Inspectors](https://fenderstudiopromanual.fender.com/en/Content/Editing_Topics/Track_and_Event_Inspectors.htm)
