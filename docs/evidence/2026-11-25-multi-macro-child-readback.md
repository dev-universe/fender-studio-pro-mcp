# Multi Instrument macro assignments and live child changes

The owned R331 fixture was saved with `Temp - 2 Mai Tai - Controls`.
Its embedded Multi Instrument state declares eight knobs, eight
switches, and four pad axes. All 30 saved target paths resolve to
one of its two embedded Mai Tai devices. Two switch targets declare
an inverted transformation to the synth bypass control.

R391B reopened a byte-identical copy on Studio Pro 8.1.1 build 113103
on macOS arm64. The two saved slot identities matched live slots.
The live Device object IDs differed from the saved device-data IDs.
The first read-only preflight R391 stopped on that difference. The
retry bound each live Device via its matching slot and independently
read its child DSP provider.

| Root macro | Changed child DSP fields |
| --- | --- |
| `float-1` Cutoff | `filter.cutoff` on both children |
| `bool-7` Synth 1 | `bypass` on the first child only |
| `pad-y-2` Amount | `character1.shapemix` on both children |

Each case changed one bound root macro. Two separate reads agreed on
the exact target changes and no other root or child rows changed.
After each restoration all 150 parent rows and both sets of 283 child
rows matched the initial state. The saved map covers all 20 macros.
This live test covers the three table entries. No MIDI event or
audible output was part of the trial.

The retry used 196 requests in 15.375 seconds and the host ran for
62.553 seconds. The scratch was closed without saving and remained
byte-identical. Both processes exited normally. The 18 managed
files and nine profile files were restored.
