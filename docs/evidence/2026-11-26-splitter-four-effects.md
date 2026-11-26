# Stock Splitter removal with two effects on each branch

R559 used Studio Pro 8.1.1 build 113103 on macOS arm64. The
owned song had a stopped transport and muted Main at zero. Two
class-qualified additions and four guarded moves prepared Splitter 3
with two Mixtool effects on each branch. Both branch orders were
independently verified in the ten-node and twelve-link graph.

The existing MCP removal refused before dispatch and left the
graph and all 288 typed DSP rows intact. One stock Remove detached
exactly Splitter 3 and its paired merger. It produced eight nodes
and nine links. The four effect instances formed one serial chain,
with both first-branch effects in their original order, followed
by both second-branch effects in their original order. The rest of
the graph and all 288 DSP rows matched the prepared state.

One stock Undo restored all ten nodes, twelve links, identities,
branch and slot order, and DSP rows. The Routing tab and the paired
native reads agreed. The owner issued 295 requests in 115.837
seconds and released all 22 captures. Studio Pro exited normally
after 154.489 seconds. The scratch song was never saved and retained
its hash. The 18 managed assets and ten changed profile files were
restored.
