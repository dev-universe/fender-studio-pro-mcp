# Input Filter GUI audition with the track monitored in Studio Pro 8.1.1

R385 used macOS arm64, Studio Pro 8.1.1 build 113103 and independently
bound Input Filter and Chorder instances in serial Note FX slots. The
owned instrument track had no events, transport was stopped and Main
was zero. There were no external MIDI messages in this test. Monitoring
was enabled on only the scratch track and returned to its original value.

The public MCP parameter setter pressed and released seven
`keyboard.pitchN.guiInput` lanes on Input Filter. Its factory range was
MIDI 48 through 72. All seven requests produced the corresponding
Input Filter `playingLive` flag. The separate Chorder independently read
exactly MIDI 48, 60 and 72 during the three in-range holds. MIDI 0, 47,
73 and 127 were absent downstream. For each separate release, two reads
found all 128 downstream live-input flags neutral.

With monitoring off in R383, the same seven GUI presses appeared only in
Input Filter's local flags. With monitoring on in R385, the admitted notes
reached the next effect. This establishes monitored software note delivery
for the sampled lanes and distinguishes it from the local GUI status.

The 388 Input Filter and 908 Chorder DSP rows, mixer and timeline matched
the starting projection. The owner sent 240 requests in 19.527 seconds.
The app closed without saving after 59.616 seconds. The scratch bytes,
18 managed installation files and 9 app settings files matched their
originals.
