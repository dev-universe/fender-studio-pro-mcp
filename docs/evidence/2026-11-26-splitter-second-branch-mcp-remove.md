# MCP removal with two effects on the second Splitter branch

R558 used candidate `f37cace` in Studio Pro 8.1.1 build 113103 on
macOS arm64. The owned test song had a stopped transport and a muted
Main at zero. Three bound Mixtool effects occupied Splitter 3, with
one effect on the first branch and two on the second. Each trial
used the current document, graph, and native effect identity.

A request that omitted the tail and a request that supplied the exact
tail on the wrong branch were refused before dispatch. Two independent
captures after each refusal matched the nine nodes, eleven ordered
links and 216 typed DSP rows of the prepared state.

A single `insert_splitter_remove` request with the exact
`second_branch_tail_node` was accepted and returned the matching
binding. Live graph reads and the Routing tab showed seven nodes and
eight links, with the first branch's effect followed by the second
branch's two effects. The three effect identities and all 216 typed
rows were preserved. One stock Undo returned the original complete
node order, links and values.

The owner issued 250 requests in 69.177 seconds, releasing all
20 captures. Studio Pro exited normally after 111.580 seconds.
The scratch archive was unchanged. The 18 managed assets and ten
changed host profile files were restored.
