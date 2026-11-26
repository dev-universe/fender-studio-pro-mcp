# Impact Pad Focus in Studio Pro 8.1.1

Native macOS arm64, Studio Pro 8.1.1 Build 113103. Each trial used an
isolated copy with one selected Impact instrument, one MCP owner, stopped
transport, and a muted Main output at zero. The provider exposed 4,262
DSP records, including 33 Focus aliases and 128 indexed cutoff controls.

## Editor selection with Pad Focus off

R402 opened Impact on bank A, C1. Clicking C♯1 changed the selected pad
in the editor. The typed provider remained unchanged. Writing raw 5,000
through `focus.filter.cutoff` changed only that alias and
`pad[0].filter.cutoff` in two independent full reads. The scratch was
discarded after the expected nonzero-pad binding failed.

## Routed MIDI selection with Pad Focus on

R403 enabled Pad Focus in the editor. The selected Music track used All
Inputs and temporarily monitored the receive-only CoreMIDI source. The
source sent four channel 1 note-on/off pairs at velocity 42. All eight
messages were checked independently in a CoreMIDI loopback before the app
trial. The corresponding sample pads were empty.

Marked cutoffs on pads 1 and 17 gave each selected pad a distinct value.
At each step, two independent full DSP reads agreed. The normalized
Float32 bits of the alias and the corresponding indexed parameter matched.

| MIDI note | Pad | Indexed raw cutoff | Focus raw cutoff | Shared normalized bits |
| --- | --- | ---: | ---: | --- |
| 36 | 0 | 10000 | 10000 | `3F800000` |
| 37 | 1 | 3200 | 3199.9999284744263 | `3EA3D70A` |
| 53 | 17 | 4700 | 4699.999988079071 | `3EF0A3D7` |
| 36 | 0 | 10000 | 10000 | `3F800000` |

For notes 37 and 53, only `focus.filter.cutoff` changed relative to the
marked state. Note 53 visibly selected bank B, F2. The final note 36
returned the editor to bank A, C1. The trial wrote the indexed markers.
The Focus alias was read only during routed selection.

R403 returned monitoring and Pad Focus to their prior states, restored
both indexed values and confirmed all 4,262 records in two final full reads.
The track, mixer and empty timeline matched the prepared scratch baseline.
MCP completed 661 requests in 137.225 seconds and the app ran for 174.944
seconds. R402 completed 261 requests in 49.840 seconds and the app ran for
222.923 seconds. Both scratch files were closed without saving and
verified against their source copy. The normal 18 managed files were
verified after restoration, along with 9 profile files for R402 and 10
for R403. The temporary MIDI source exited.

These observations cover cutoff selection for two nonzero pads.
Writes to a MIDI-selected nonzero pad, other Focus aliases, a loaded
sample's note lifecycle and save/reopen require separate verification.

See the [Fender Impact reference manual](https://fenderstudiopromanual.fender.com/en/Content/Built-In_Instruments_Topics/Impact.htm#Pad_Focus)
for the Pad Focus control and external-controller behavior.
