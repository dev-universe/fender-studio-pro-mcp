# Existing Sidechain Send ownership in Studio Pro 8.1.1

R458 examined the exact Studio Pro 8.1.1 build 113103 macOS arm64
executable, the current bridge, and the prior
[Scope Sidechain fixture](https://github.com/dev-universe/fender-studio-pro-mcp/pull/207).
The prior fixture independently bound the source channel and the target
Scope insert, measured signal on the target, and observed empty source
and destination Send lists after restoration. R458 made no DAW calls.

For an existing eligible Send, the stock selection action obtains the
source channel's typed Send slot and separately resolves the effect's
typed sidechain AudioInput. It supplies both to the host edit operation
using the Send's fixed `destination` property. The typed setter uses
the slot's owned output. Its concrete output dispatch connects to the
destination and the concrete sidechain input dispatch appends the
reciprocal incoming connection. The exact new-slot folder dispatch still
requires independent binding. The prior fixture established the stock
add behavior on one initially empty source.

The stock selection view enumerates the chosen channel's typed Send
slots and compares each slot's current destination object with the
separately resolved effect input. The getter first consults the owned
output, with a fallback to the stored destination property when the
output has no destination. This can identify an authored assignment.
Verifying the reciprocal current connection requires additional evidence.

The public `sends` view exposes the Send bank position, a displayed
destination label, level and mute. The translated sidechain label, the
bank position and Scope's source choice do not establish the native input
port index. The native index was found on the typed effect input's
activation path, and no supported retained accessor jointly exposed it
with the exact current Send and effect. No change to the MCP routing
contract was made.
