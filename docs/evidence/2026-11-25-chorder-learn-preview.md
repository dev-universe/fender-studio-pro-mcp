# Chorder Learn Mode preview reaches a second Note FX in Studio Pro 8.1.1

R376 used an owned scratch song on macOS arm64, Studio Pro 8.1.1 build
113103, with two serial Chorder instances of class
`D2D9B002-12FC-4F1D-AFDA-70336D1DC108`. Both instances had distinct native
UIDs and the same initial stored chord on MIDI input 60. Transport was
stopped, the instrument track had no events, arming or monitoring, and
the Main fader was zero.

The second Chorder acted as a downstream observer. Each observation read
its 128 `playingLive` input flags. As a positive control, the first
instance's normal `chorder.pitch61.guiInput` pressed MIDI 60 and the
downstream instance reported exactly MIDI 60, 63, 67, 70 and 74 in two
successive observations. Release returned all 128 flags to neutral.

With the first editor open, Learn Mode enabled and MIDI 60 selected, the
existing MCP parameter setter held `chorder.playLearnChord` (DSP tag 8) at
1 once and returned it to 0. Separate reads verified both values. The
second Chorder independently reported the same five input notes in each of
two held observations. Two reads after release found all 128 flags neutral.
This is separate from the R371 same-instance observation, which tested the
first Chorder's local display flags.

Learn Mode and its selector, the mixer, and all 908 reported DSP rows and
128 stored chords in each instance returned to their baselines. Two reads
of each stored model also matched the original component state fingerprints.
The owner completed 211 requests in 16.973 seconds. The host exited after
48.951 seconds, the unsaved scratch bytes matched their starting digest,
and 18 managed files and 9 application settings files were restored.

The result establishes MIDI note delivery to a downstream Note FX and release
in this configuration. Physical audio and MIDI hardware were outside the test.
