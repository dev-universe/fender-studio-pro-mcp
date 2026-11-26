# Live Event Input reads with two Serum 2 instruments

R534 and R534B ran two Xfer-signed Serum 2 instrument VST3 2.1.5
instances in Studio Pro 8.1.1 build 113103 on macOS arm64. The owned
scratch used two uniquely named Music tracks, distinct native instrument
UUIDs, muted Main, and stopped playback. Both instances began at Event
Input 1. The labels were identical and their native port objects were
distinct.

The global selector enumerated 32 distinct objects in two consecutive
sets of Event Inputs 1 through 16. The attached selectors also exposed
overlapping suffixes of the global list. Their enumeration lengths in
the trial were 32, 31, 16, or 1 depending on the current port. This
rules out using the length of the attached list to identify an
instrument's ports in this configuration.

The first trial stopped because the pointer service could not reach
the stock selector. R534B used a fresh scratch. Before each private
switch it matched both current port objects to the two observed groups
and checked the selected track's native instrument identity. The
independent public `track_event_input_get` and per-track selector
readback yielded the following:

| Phase | First track | Second track |
| --- | --- | --- |
| Initial | 1 | 1 |
| First route changed | 2 | 1 |
| Second route changed | 2 | 16 |
| First route restored | 1 | 16 |
| Second route restored | 1 | 1 |

R534B made 218 requests in 41.674 seconds. Both DSP snapshots matched
across all 5,246 rows. Both scratch files stayed byte-identical to their
source. The app and owner exited normally in both trials, and the
usual installation and affected profile files were restored. Neither
trial issued a Save, MIDI, or playback. A generally applicable owner
lookup and a public route setter remain open.

[Public read semantics](https://github.com/dev-universe/fender-studio-pro-mcp/pull/306)
