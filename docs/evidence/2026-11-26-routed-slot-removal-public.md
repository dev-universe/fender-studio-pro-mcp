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

## Saved middle-slot removal and cold reopen

R583 tested the public candidate at `b1b1c7a1d0b0` on Studio Pro 8.1.1
build 113103 arm64. The same supported middle-slot edit left Splitter 3 in
place. Mixtool 3 remained in its own lane. Mixtool and Mixtool 4 became
adjacent, and Mixtool 2 was gone. Independent native and archive reads
agreed on nine nodes and eleven ordered links. The native DSP inventory
contained 216 rows.

One stock Save wrote only the owned scratch. The saved archive passed the
integrity check and matched the live slot IDs and display order. The first
warm checker still expected a different fixture's display names. A fresh
MCP owner compared the two recorded pre-Save observations against complete
post-Save reads on the same document and confirmed exact equality without
another write.

A new process reopened the saved bytes under a file-write denial. A fresh
MCP owner rejected the previous document token. A later check used the
three current slot and device identities with the obsolete graph UUID and
received a pre-dispatch rejection. Its before and after reads matched the
warm slot/device IDs, routing, enumerated order and all 216 DSP rows. The
live graph and folder carried new session-scoped IDs. The saved digest held
through the cold trial, and the protected source digest also matched.

The warm phases made 327 and 32 requests in 110.778 and 3.549 seconds.
The cold phases made 30 and 54 requests in 3.386 and 5.383 seconds. All
32 temporary captures were released. Both hosts exited successfully after
244.754 and 127.910 seconds. The 18 managed files and app profile were
reverified after restoration. The cold phase made zero Saves.
