# MCP removal with two effects on each Splitter branch

R560 used candidate `9a2d6e7` in Studio Pro 8.1.1 build 113103 on
macOS arm64. The owned scratch song had a stopped transport and a muted
Main at zero. Splitter 3 held Mixtool 2 and Mixtool 3 on its first
branch, and Mixtool and Mixtool 4 on its second.

A request that omitted the second tail and a request that swapped the
exact tails were refused before dispatch. Two independent captures after
each refusal matched the prepared ten nodes, twelve ordered links and
288 typed DSP rows.

With both exact branch tail UUIDs and the current document, graph and
effect identity, one `insert_splitter_remove` request returned
`applied_unverified`. Independent captures showed eight nodes and nine
links, with Mixtool 2, Mixtool 3, Mixtool and Mixtool 4 connected in
that order. Their instance identities and all 288 rows remained equal.
One native Undo restored both branches, ten nodes, twelve links and all
288 rows. The Routing tab showed the serial chain and the restored
parallel branches.

The owner released all 24 captures and exited normally after 325
requests in 194.812 seconds. Studio Pro exited normally at 266.421
seconds. The scratch song hash stayed equal without saving. All 18
managed installation files and ten changed profile files were restored.
