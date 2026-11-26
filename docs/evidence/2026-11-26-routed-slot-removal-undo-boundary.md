# Exact effect removal and Studio Pro Undo

R568E ran on Studio Pro 8.1.1 Build 113103 for macOS arm64,
using a single MCP owner and an isolated, write-protected copy of a
nested Splitter song. The experimental implementation was pinned at
`17b8e3ed`. Playback and recording were stopped and Main was muted
at zero. The right lane contained Mixtool, Mixtool 2 and Mixtool 4.
The other lane contained Mixtool 3.

| Request | Independent result |
| --- | --- |
| Real but wrong Device UUID | Refused before dispatch. The graph retained 10 nodes, 12 ordered links and 288 DSP rows. |
| Real slot on the other lane as successor | The reciprocal-link check refused the request. The same graph, device identities and DSP rows were unchanged. |
| Exact document, graph, predecessor, target and successor | The routed slot's exact observer accepted synchronous `removeSelf`. Mixtool 2 alone disappeared. The remaining neighbors connected to each other, the other lane and surviving identifiers stayed exact, and the separate graph and DSP reads found nine nodes, 11 links and 216 unchanged rows. The Routing tab showed the same three surviving effects in order. |
| Studio Pro's Edit menu | The next Undo was the preceding node move. The trial stopped at the Undo gate and exercised no Undo. |

An earlier pre-dispatch comparison had matched the base `AudioInsertSlot`
constructor table. A private read-only check on the retained delegate
identified the routed subtype. On the same image, Mach-O fixups bound
the subtype's inherited command check and its distinct observer thunk,
which forwards to the inherited handler for this message. The
subtype-specific observer makes the graph edit visible while leaving the
stock Undo shown above. A stock Routing menu removal of the same
Mixtool 2 did provide a single native Undo in the preceding trial.

R568E issued 327 MCP requests in 81.742 seconds and released all
24 capture tokens. Studio Pro ran for 120.048 seconds and exited with
code zero. The owner closed, the scratch and source song hash matched,
and all 18 managed installation files and 10 changed profile files
were restored. The trial performed no Save.

The public tool requires a journaled action on the same exact target
and independent Undo proof before release.

[Stock Routing menu removal and Undo](https://github.com/dev-universe/fender-studio-pro-mcp/pull/340)
· [Static receiver study](https://github.com/dev-universe/fender-studio-pro-mcp/pull/341)
