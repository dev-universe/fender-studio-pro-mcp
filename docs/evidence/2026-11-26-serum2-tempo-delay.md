# Serum 2 and Serum 2 FX tempo-synced delay

R548 rendered the Xfer-signed macOS arm64 Serum 2 and Serum 2
FX 2.1.5 VST3 in the independent SDK host used by Ableton MCP.
The host supplied 90, 120 or 147 BPM in the VST3 process context.
Each render was two seconds, stereo, at 48 kHz. The instrument
received one short MIDI note. The FX received a locally generated
10 ms stereo test burst. [SDK probe](https://github.com/dev-universe/ableton-mcp/blob/3c83560/tools/serum_host.cpp)

Both variants used a synthetic Delay with zero feedback and
100% wet output. Each delay is the difference between its first
sample above 0.0001 absolute amplitude and the corresponding
empty-rack reference. Both variants produced the same delays.

| Setting | 90 BPM | 120 BPM | 147 BPM |
| --- | ---: | ---: | ---: |
| Fixed 250 ms | 11,950 | 11,950 | 11,950 |
| Tempo-synced sixteenth note | 7,993 | 5,974 | 4,839 |
| Tempo-synced eighth note | 15,998 | 11,998 | 9,793 |
| Tempo-synced quarter note | 31,998 | 23,998 | 19,590 |

The empty-rack and fixed-delay audio was byte-identical across
tempos within each variant. Six new-process eighth-note repeats
matched their earlier renders byte-for-byte. The eighth- and
quarter-note onset differences were within four frames of
their calculated delays. The sixteenth-note transient moved
by up to 59 frames relative to the calculated delay.

In this tested rack, beat-sync on with both time fields set to
`0.046875` produced eighth-note timing. `0.0625` produced a
quarter note. An initial `0.03125` candidate was checked
against the waveform and classified as a sixteenth note.
The 250 ms comparison sets beat-sync off and both time
fields to `0.25`. This evidence covers those values and
does not define the complete time enumeration.

A minimal demo-state snapshot with only the rack inserted
rendered the same signal as the dry reference. The measured
cases reused the previously verified processor carriers and
changed the rack settings while keeping their other fields.
The demo readback is limited. The audio is independent
evidence of the applied delay behavior.

The 30 qualified renders and six repeats ran with Studio Pro
closed. The same plug-in's transport requirements were checked
in [R545](2026-11-26-serum2-vst3-process-context.md). A Studio Pro
playback test with the signed plug-in and these delay settings
is a separate validation step.
