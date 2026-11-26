# Public MCP Event Input read with Serum 2

R532 ran the release MCP binary and its bridge and EditTask on Studio
Pro 8.1.1 build 113103 arm64. The test song had one uniquely named
Music track with the Xfer-signed Serum 2 instrument VST3 2.1.5. Main
was muted at zero. Playback, recording, and monitoring were off.

At Event Input 1, 2, 16 and the restored 1, the stock Inspector menu
independently showed that exact checked item. Two public
`track_event_input_get` calls at each stop returned the matching track
name, one-based port number and host label. A fixture-only selector
changed the route on the owned track. The public endpoint only read it.

The release endpoint refused a missing document, a stale document, and
a track absent from the active document. The existing exact Serum
identity, timeline, mixer, and all 2,623 DSP rows matched after
restoration. The host made 100 requests in 121.2 seconds. The owner and
app exited normally. No Save, MIDI, or playback was issued. Scratch
bytes, 18 managed files, and 10 affected profile files were verified
after restoring the usual installation.

Offline coverage also checks duplicate names, non-Music tracks,
unrecognized ports, and a mismatched task receipt. The operation
reports the currently selected numbered Event Input for a track.
Instance identity, a setter, and live effects require separate
operations and verification.

[Selected-port delivery](2026-11-26-serum2-selected-event-input.md)
