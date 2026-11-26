# Removing the third Splitter with two occupied lanes

R523 used Studio Pro 8.1.1 build 113103 on macOS arm64. An owned,
Save-protected copy of the three-Splitter song started stopped, with
the target and Main outputs muted. Two exact MCP moves put one
Mixtool in each lane of Splitter 3. Its parent Splitter's other lane
led directly to the parent merger.

From this state, the stock Routing menu removal and the existing
`insert_splitter_remove` operation each removed only Splitter 3 and
its paired merger. Both produced the same 6-node, 7-link graph from
the original 8 nodes and 10 links. The two Mixtools remained in series
in their observed first-lane then second-lane order. Each read
preserved surviving device and slot identities and all 144 DSP rows.
The MCP call supplied the current document and graph, both ordered
effect nodes, the splitter-merger pair and an exact current effect
instance. Its receipt was `applied_unverified` before independent
readback.

After each removal, one stock Edit menu Undo returned the exact
parallel branches, child enumeration, both native instance bindings
and all typed DSP rows. The Routing editor displayed the serial and
parallel shapes at the corresponding stages.

The MCP owner used 199 requests in 131.798 seconds, including two
staging moves and one MCP removal. Seventeen captures were released
and the host exited after 168.174 seconds. No Save was issued and the
owned scratch remained byte-exact. All 18 managed files and nine
profile files were returned to their verified pre-trial state.

The earlier depth-two removal remains separately documented.
This trial covers the exact deeper, two-occupied-lane shape.
[Earlier removal and Undo](2026-11-20-insert-splitter-remove.json)
