# Chorder MIDI 60 audition through the MCP parameter in Studio Pro 8.1.1

R375 ran on macOS arm64 with Studio Pro 8.1.1 build 113103. The test used an
owned copy of the R95 song, Chorder Note FX class
`D2D9B002-12FC-4F1D-AFDA-70336D1DC108`, and a fresh binding to the active
document, track, slot and native instance. The Chorder editor was closed,
Learn Mode was off, transport was stopped, the track was neither armed nor
monitored, and the Main fader was zero.

The existing `plugin_parameter_set` API wrote the DSP lane
`chorder.pitch61.guiInput` (raw tag `433`). This lane addresses MIDI note 60.
The exact pressed Float64 bits `40BF400007D00000` came from a prior observation
of the editor's audition key on the same note. Two independent live reads
observed that raw value, the input's `playingLive` flag, and precisely the
stored chord's five output flags for MIDI notes 60, 63, 67, 70 and 74. The
scheduled flags and `playLearnChord` stayed unchanged. The release wrote
zero once after a fresh read confirmed the still-held bound lane.

After release, separate reads returned all 908 DSP rows, all 128 stored
chords, the original 581-byte state fingerprint and the mixer projection
exactly to their baselines. The persisted song bytes also matched the source
fixture after close without saving. The MCP owner completed 91 requests in
8.854 seconds. The host exited cleanly after 159.629 seconds, and 18 managed
installation files and 9 application setting files were restored.

This verifies the live held and released state of the MIDI 60 audition
through the public MCP parameter with the editor closed. Physical audio and
the output effect of the separate Learn Mode preview command remain outside
this test.
