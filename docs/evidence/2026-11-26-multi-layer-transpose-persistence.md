# Multi Instrument per-layer transpose and persistence

In Studio Pro 8.1.1 Build 113103 on macOS arm64, a fresh owned copy of a
saved Multi Instrument scratch song contained two parallel Mai Tai
layers. With transport stopped and Main muted at zero, the Routing
inspector separately showed Transpose 0 for the original left layer
and its peer on the right.

One confirmed inspector edit changed the left layer to +12 semitones.
The right layer independently showed 0. Native readbacks kept the exact
live graph and slot bindings, existing mixer and track, and all 150
parent and 283 records per child unchanged.

The song was saved once from its own window. An independent archive
read compared the ordered successor graph and normalized contents of
all seven node records. The two route paths and six directed successors
were exact. The sole graph change was the original slot's Transformer
transpose from 0 to 12. The peer Transformer stayed 0 and both key-zone
records matched their original values.

A new Studio Pro process and MCP session reopened the saved copy. Both
values were visible on their respective layers, and the archived
topology and logical bindings matched the native reads. Each child had
a fresh live Device identity. The MCP refused the old document binding.
All 716 typed records remained exact. Audio pitch and a script mutation
were outside this trial.

The live edit used 142 MCP requests in 133.229 seconds. Reopening used
94 in 24.954 seconds. Both processes and owners exited normally. The
source and once-saved copy retained their expected digests after closing
without a second Save. Eighteen managed installation files and nine
app profile files were restored and verified.
