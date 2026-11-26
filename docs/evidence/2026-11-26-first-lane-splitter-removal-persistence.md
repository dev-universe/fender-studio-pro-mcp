# First-lane nested Splitter removal through Save and reopen

R519 used Studio Pro 8.1.1 build 113103 on macOS arm64 and the
first-lane MCP candidate verified in R518. A named scratch copy
began with eight nodes and ten links. Main and the test channel
were muted and transport was stopped.

After the same bound request rejected the initially empty shape,
one slot move put Mixtool 2 in the first lane. The anchored removal
returned `applied_unverified`. Independent reads showed six nodes,
seven reciprocal links and both Mixtools in series, with the exact
retained slot and device IDs and 144 DSP rows. Studio Pro saved
the scratch once. The file reader independently matched the
six surviving nodes, exact slots and seven links.

A new Studio Pro process reopened the same saved file with writes
blocked. It rebuilt the live folder and graph IDs and retained
the expected persistent IDs, links and 144 DSP rows. An old-document
request and an old-graph request under the new document were both
refused. The graph and DSP rows remained identical after the refusals.
The saved archive retained the same hash.

The Save phase made 159 requests in 118.829 seconds. The reopen
phase made 49 requests in 4.817 seconds. Both apps and owners exited
normally, all 16 capture tokens were released, and the managed
installation and app profile were restored and verified. The
source archive stayed protected.

[First-lane MCP removal and Undo](https://github.com/dev-universe/fender-studio-pro-mcp/pull/292)
