# Studio Pro event-port inventory for Serum 2

R505 queried one Xfer-signed Serum 2 instrument VST3 2.1.5 in Studio Pro
8.1.1 build 113103 on macOS arm64. The temporary song contained one
empty instrument track and one verified native Serum instance. Main was
muted at zero, monitoring was off, and the transport was stopped.

The stock Add Tracks task gets a music input-port list from the song
environment and uses `Devices:PortParam` to enumerate destination
ports. Its existing-instrument branch passes a selected port to
`track.channel.connectTo`. A private MCP EditTask probe used the
enumeration path and found 16 distinct port objects in order, labeled
`Event Input 1` through `Event Input 16`. Two separate requests
returned the same inventory. The channel exposed `connectTo` and
`getDestinationChannel`. The port objects exposed labels and a
numeric type, while the probe found no script-facing owner or ID
among the inspected properties. Exact binding in the presence of
same-model instances and live delivery over an alternate port remain
open.

The host made 29 MCP requests in 29.408 seconds. Subsequent reads
matched the original track, mixer and native instrument instance.
The private probe performed zero route changes, event transmissions
and saves. The scratch song was closed without saving and the signed
scanner bundle was retired. All 18 managed files and 11 profile files
were restored. The diagnostic probe was removed with the temporary
installation.

[Default event input delivery](2026-11-26-serum2-default-event-input.md)
