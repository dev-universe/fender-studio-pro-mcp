# Native middle effect deletion inside a Splitter branch

R566 used candidate `e1afd43` in Studio Pro 8.1.1 build 113103 on
macOS arm64. The owned scratch had a stopped transport and a muted
Main at zero. Splitter 3's left lane held Mixtool 3. Its right lane
held Mixtool, Mixtool 2 and Mixtool 4 in that order.

One stock Remove command on Mixtool 2 kept Splitter 3. The two
neighbors on its right lane connected directly. Independent captures
confirmed nine nodes, eleven ordered links, the same folder and graph
identities, the original left lane, and all 216 typed DSP rows of the
three surviving effects. The removed instance alone was absent.

The native Undo menu identified Mixtool 2 by name. One Undo restored
all original node, slot and effect identities, enumeration, 12 ordered
links and 288 typed DSP rows. This verifies the stock command. A
matching MCP tool for individual routed slot deletion remains open.

The owner issued 296 requests in 90.794 seconds and released all 22
captures. Studio Pro exited normally after 133.362 seconds. The test
song was not saved. Its hash and the 18 managed installation files
and ten changed host profile files were checked after restoration.
