# Ordered MCP removal with a three-effect Splitter branch

R565 used candidate `39e7e7a` in Studio Pro 8.1.1 build 113103 on
macOS arm64. The owned scratch held four bound Mixtools with playback
and recording stopped, Main muted and the Routing tab open.

With one effect on the first lane and three on the second, one request
supplied the two real continuation nodes in reverse order. It was
refused before dispatch. Independent captures showed the original
10 nodes, 12 ordered links and 288 typed DSP rows unchanged.

A request with both continuation nodes in graph order returned
`applied_unverified`. Independent captures showed eight nodes and nine
links, with Mixtool 3, Mixtool, Mixtool 2 and Mixtool 4 in that order.
The four effect identities and 288 DSP rows were unchanged. One native
Undo restored the exact original graph, enumeration and values. The
same checks passed with three effects on the first lane and one on the
second. Both placements used a single stock removal command.

The owner issued 432 requests in 102.304 seconds and released all 32
captures. Studio Pro exited normally after 139.921 seconds. The owned
song hash matched without saving. The 18 managed installation files
and ten changed host profile files were restored.
