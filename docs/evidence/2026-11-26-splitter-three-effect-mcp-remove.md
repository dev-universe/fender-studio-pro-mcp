# MCP removal of a nested Splitter with three effects

R555 ran the native and bridge candidate `489584d` in Studio Pro 8.1.1
build 113103 on macOS arm64. One owned scratch song had a muted Main bus
and a stopped transport. A third exact Mixtool and three guarded slot
moves prepared Splitter 3 with two effects on branch 0 and one on
branch 1. Each read bound the current document, Inserts graph, and
native effect identities.

The first `insert_splitter_remove` request omitted
`first_branch_tail_node`. The second supplied a node that was not
an effect slot. Both were refused before dispatch. Independent reads
after each refusal matched all nine nodes, eleven ordered links and
216 typed DSP rows.

The third request supplied the exact second slot on branch 0. Its
receipt carried the supplied identities and reported
`applied_unverified`. Two independent graph and DSP reads confirmed
seven nodes and eight ordered links, the three effects serialized in
branch order, and all 216 typed rows unchanged. The rack displayed
the same serial chain. One stock Undo restored both branches, all
native identities and values, and the full nine-node graph. This
matches the separately observed stock-menu removal.

The owner issued 250 requests in 68.831 seconds and released all
20 captures. The host exited normally in 97.254 seconds. The trial
did not save. The scratch song hash remained the same, the 18
managed installation files and ten changed profile files were
restored, and there were no remaining capture files.
