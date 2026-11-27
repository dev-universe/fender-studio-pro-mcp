# Scope C phase with an active stereo sidechain

In Studio Pro 8.1.1 Build 113103 on macOS arm64, two empty tracks in an
owned scratch routed a software 440 Hz test tone through Mixtool to the
Scope sidechain. Mixtool inverted only the right channel. The sidechain
was enabled in the Scope editor. The source had the sole active pre-fader
send to that Scope, independently visible in the MCP send list. Main
was muted at zero and both track faders were at zero. [Fender sidechain routing](https://fenderstudiopromanual.fender.com/en/Content/Mixing_Topics/Effects_Signal_Routing.htm)

With Tone Generator off, the three traces were flat. With the test tone
on, the dedicated Side Left and Side Right reference traces were
opposite in phase. The test used the same horizontal sample positions
and separated the traces with fixed vertical offsets.

| C DSP readback | C trace | Scope C selector text |
| --- | --- | --- |
| 2, Side Left | In phase with Side Left, opposite Side Right | Side Left |
| 3, Side Right | In phase with Side Right, opposite Side Left | Side Left |

The phase changed with the exact DSP value while the visible C selector
kept the previous label. The test covers an active sidechain, both stereo
inputs and live software traces. It adds no evidence about physical
audio routing, hidden waveform storage, or capture completion time.

The tone was switched off and the same send removed before restoring
all 113 typed records. Two complete readbacks matched the baseline and
the scratch mixer and timeline were unchanged. The trial used 138
requests in 132.649 seconds. The app exited normally after 190.765
seconds. The scratch closed without saving. Its original bytes, 18
managed files and 10 app profile files were verified after cleanup.
