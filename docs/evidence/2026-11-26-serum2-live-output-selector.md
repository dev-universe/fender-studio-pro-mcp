# Live track output selector for Serum 2

R531 followed the installed Studio Pro 8.1.1 build 113103 arm64
channel constructors to the named `outputPort` parameter. The calls
at `0x100e2b484` and `0x100e2f0cc` construct a PortParam with that
name. The track's record-input selector is a distinct `recordPort`.

The live trial used one exact Xfer-signed Serum 2 instrument VST3
2.1.5 in a stopped, muted scratch song with monitoring disabled.
The script read `track.channel.findParameter("outputPort")` and called
`getCurrentPort()` on the existing selector. The stock Inspector
independently showed the same port at each stop.

| Stop | Selector port label | Port type | Separate record input |
| --- | --- | --- | --- |
| Initial | Event Input 1 | 0 | Any |
| First change | Event Input 2 | 0 | Any |
| Second change | Event Input 16 | 0 | Any |
| Restored | Event Input 1 | 0 | Any |

Two script reads agreed at every stop. The scalar selector value was
always `-1`. The live port label tracked the Inspector. Looking for the
selector on the track instead of its channel returned `null`. The
previous `getDestinationChannel()` method returned `null` throughout,
in agreement with the earlier live test.

There were 101 requests in 71.842 seconds and three guarded route
changes. All 2,623 DSP rows, the instrument identity, mixer and timeline
matched after restoring the route. The app and owner exited normally
with no Save, MIDI, or playback. The scratch bytes, all 18 managed
files and 10 affected profile files were verified after restoration.
Tests of a public MCP tool, FX, and same-model cases remain open.

[Selected-port delivery](2026-11-26-serum2-selected-event-input.md)
