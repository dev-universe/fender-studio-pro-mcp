# MCP removal with either lane occupied in the verified nested Splitter graph

R518 built the core and native extension from candidate `dbd85f1`
with Rust 1.88.0. Studio Pro 8.1.1 build 113103 on macOS arm64
opened a Save-protected copy of the eight-node Mixtool fixture.
Transport was stopped and the Main and test channel were muted.

The same exact anchored removal request returned `not_applied`
while both lanes were empty, and independent reads confirmed that
the graph and both 72-row DSP projections matched the baseline.
One bound slot move then put Mixtool 2 in the first lane. With that
single changed shape, the same removal request returned
`applied_unverified`.

Independent reads found six nodes, seven reciprocal links and both
Mixtools in series, with the original device IDs and all 144 DSP
rows. One native Undo restored all eight nodes and ten links, the
prepared child enumeration order and the same DSP rows. The
second-lane witness remains the prior R501 trial.

The owner made 181 requests in 148.994 seconds and released all
14 capture tokens. Studio Pro exited after 203.324 seconds with no
Save. All 18 managed files and nine app profile files were restored.
Core Rust tests (273), native Rust tests (511) and the focused
bridge tests (22) passed.

[First-lane stock Remove and Undo](https://github.com/dev-universe/fender-studio-pro-mcp/pull/291)
· [Second-lane public removal](2026-11-26-splitter-single-occupied-mcp.md)
