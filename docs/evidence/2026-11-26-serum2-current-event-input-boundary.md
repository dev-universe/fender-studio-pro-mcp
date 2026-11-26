# Studio Pro current event-input inspection

R529 traced the installed macOS arm64 Studio Pro 8.1.1 build 113103.
The extracted main executable had UUID
`6B547378-B3BB-322A-87C9-901493FD2BED` and SHA-256
`c6b326d8dfac4d468f8730508abcf8687997c6cd96a977a560600fda785a2e6e`.
REA with the Ghidra engine identified the script dispatch and device
classes. ARM64 disassembly and dyld rebase records identified the
dispatch slots and concrete implementations. Craft-Anything independently
matched nine 20-byte windows against the extracted executable.

The stock Add Tracks task enumerates music input ports through a
separately created `Devices:PortParam` and passes the chosen port to
`track.channel.connectTo`. The native `PortParam.getCurrentPort` dispatch
at `0x100f10830` calls the selector's own virtual accessor at offset
`0x268`. This describes the selector's current port. R505 observed
16 distinct Event Input objects in the environment and found no port
selector among the three inspected track-channel parameter names.

The scripting `getDestinationChannel` dispatch at `0x100e36360` first
finds the channel's output port through virtual slot `0x218`.
`MusicChannel` and `MusicTrackChannel` both resolve that slot to
`0x100ecdf58`, which returns the port at offset `0x2b8`. In the dispatch,
virtual slot `0x140` must report exactly one connection, then slot `0x148`
reads index zero. Concrete `OutputPort` implementations at `0x100d9c23c`
and `0x100d9c244` read the connection count and bounds-check the index.
The dispatch then reads the connected port's owner at `0x100e36464`,
checks its channel type, and wraps the owner's scripting subobject at
`0x100e36488`. The scripting return value therefore carries a channel
owner rather than the identity of the individual input port.

R529 made no live app calls or route changes. A live observer still needs
to bind the active track and exact Serum instance to the actual connected
port and independently compare that result with the stock Inspector.
Multiple connections, different host builds, and same-model instances
require their own checks.

[Event-port inventory](2026-11-26-serum2-host-event-port-inventory.md)
[Observed delivery on selected inputs](2026-11-26-serum2-selected-event-input.md)
