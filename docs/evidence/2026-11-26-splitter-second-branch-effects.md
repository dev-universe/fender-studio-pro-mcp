# Stock Splitter removal with two effects on the second branch

R557 used Studio Pro 8.1.1 build 113103 on macOS arm64. The owned
scratch song had a stopped transport, muted Main at zero, and
Splitter 3 nested inside an existing two-branch rack. Three exact
Mixtool instances were staged with one effect on the first branch and
two serial effects on the second. The original guarded MCP removal
refused the request before dispatch. Independent graph and parameter
reads matched the complete nine-node and eleven-link state.

The stock Splitter 3 menu removed the Splitter and its paired
merger. The graph retained seven nodes and eight ordered links.
The three effects became a single serial chain in order: first
branch head, second branch head, second branch tail. The three
effect instance IDs and 216 typed DSP rows stayed exact. One stock
Undo restored the nine nodes, eleven links, exact effect order,
and all 216 rows. Independent captures and the Routing tab agreed.

The owner issued 223 requests in 92.933 seconds and released all
18 captures. Studio Pro exited normally after 120.060 seconds.
The song was never saved and its hash remained unchanged. The
18 managed installation files and ten host profile files were
restored.
