# Exact routed effect removal with native Undo

R570 exercised `insert_slot_remove` on Studio Pro 8.1.1 Build
113103 for macOS arm64 with the clean candidate `19997267`.
One MCP owner bound an isolated scratch song with stopped playback
and recording and Main muted at zero. The right lane contained
Mixtool, Mixtool 2 and Mixtool 4. Mixtool 3 occupied the other lane.

| Request | Independent result |
| --- | --- |
| Real but incorrect removed Device UUID | Refused before dispatch. Ownership, ordered topology and all 288 DSP rows stayed exact. |
| Real sibling slot supplied as the successor | Refused before dispatch by reciprocal link checks. All 10 nodes, 12 links and 288 rows stayed exact. |
| Exact middle effect | The recorded action accepted the bound request and returned `applied_unverified`. Independent ownership, routing and DSP reads agreed with the Routing editor. Mixtool 2 disappeared, its neighbors connected, and the other lane, retained identities and 216 surviving rows stayed exact. The graph contained nine nodes and 11 links. |
| One native Undo | Studio Pro showed Undo Remove Effect "Mixtool 2". One selection restored the original 10 nodes, 12 ordered links, native identities, effect order and all 288 DSP rows. |

The trial issued 355 MCP requests in 77.857 seconds and released
all 26 capture tokens. Studio Pro ran for 112.777 seconds and exited
with code zero. The MCP owner exited, the independent song bytes
still matched the source, and the 18 managed installation files and
10 changed profile files were restored. The trial performed no Save.

The tested capability covers an exact middle slot with two effect-slot
neighbors on this bound routed Inserts graph family. Another document
requires a fresh ownership and routing read before editing.

[Recorded action trace](https://github.com/dev-universe/fender-studio-pro-mcp/pull/343)
