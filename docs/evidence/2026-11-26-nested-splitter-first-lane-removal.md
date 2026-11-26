# Studio Pro removal and Undo with only the first nested Splitter lane occupied

R517 used Studio Pro 8.1.1 build 113103 on macOS arm64 with a
Save-protected copy of the archived eight-node Splitter fixture.
The Main and exact test channel were muted and transport was stopped.

One document- and instance-bound MCP slot move put the second
Mixtool in lane index 0 of Splitter 3. Independent routed and owned
reads confirmed that only three successor lists changed, the second
lane stayed empty, and the two 72-row DSP projections and exact
device IDs matched. The child enumeration order changed during this
move. The moved graph became the baseline for removal and Undo.

The stock Splitter 3 menu removed the splitter and paired merger.
The observed graph went from eight nodes and ten links to six nodes
and seven links, with both Mixtools connected in series and all
144 DSP rows unchanged. One stock Edit menu Undo restored the
pre-removal graph, exact IDs, reciprocal links, child enumeration
order and all 144 DSP rows.

The owner made 173 requests in 180.955 seconds and released all
14 temporary capture tokens. Studio Pro exited normally after
243.346 seconds without saving. The 18 managed files and nine app
profile files were restored and verified.

[Second-lane stock removal witness](https://github.com/dev-universe/fender-studio-pro-mcp/pull/281)
