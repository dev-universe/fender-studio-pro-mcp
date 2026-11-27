# Ampire factory microphone state across an unedited Save and restart

R422 ran on Studio Pro 8.1.1 Build 113103 for macOS arm64. One
exact Ampire insert used MCM 800 and 4x12 M65 on an empty isolated
stereo track. Main was muted at zero and the transport was stopped.
The factory levels matched the R416 baseline. After inserting
Ampire and verifying that baseline, the test made no mic, Link,
preset, or Undo changes.

| Role | Raw value before Save, after Save and after reopen | Raw Float64 bits | Saved normalized Float32 bits |
| --- | ---: | --- | --- |
| Mic A | 7957.139611244202 | `40BF1523BD900000` | `3F4BB3E9` |
| Mic B | 1016.8377310037613 | `408FC6B3AC500000` | `3DD03F95` |
| Mic C | 1026.0231792926788 | `40900817BC500000` | `3DD2212A` |
| Link | 1 | `3FF0000000000000` | `3F800000` |

The app saved the owned song once. The exact insert's
`.dsppreset` in the saved song preserved the microphone and Link
settings, and its bytes matched the independently captured
2,297-byte component. Two fresh live captures at each stage
agreed with each other and with that file. All 90 non-meter rows
and the mixer, selection, track and insert records matched in both
processes. The same stored Device matched the observed live UID.
The fresh process and MCP session refused the previous document.

The control shows that a normal Save and reopen preserve this
untouched factory state. Small bit differences previously observed
after a linked edit followed by Undo or component restore remain
open. [Component restore comparison](2026-11-25-ampire-factory-mic-precision.md) · [Linked edit and Undo](https://github.com/dev-universe/fender-studio-pro-mcp/pull/204)

The two owners made 62 and 45 requests in 21.833 and 4.636
seconds. The two app processes exited normally after 67.637 and
41.136 seconds. All capture tokens were released. The original
copy source and the once-saved scratch bytes were checked along
with 18 managed files and 10 restored app profile files.
