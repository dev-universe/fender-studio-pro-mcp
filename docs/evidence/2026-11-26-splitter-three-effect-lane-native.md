# Native Splitter removal with one effect and three effects

R564 used Studio Pro 8.1.1 build 113103 on macOS arm64 and candidate
`e1afd43`. The silent owned scratch had Mixtool 3 in the first lane
of Splitter 3. The second lane ran Mixtool, Mixtool 2 and Mixtool 4.
The original graph had ten nodes and twelve ordered links.

The Routing tab's stock Remove action was invoked once for Splitter 3.
The resulting eight nodes and nine links serialized the effects as
Mixtool 3, Mixtool, Mixtool 2 and Mixtool 4. Two independent reads
confirmed their identities and all 288 typed DSP rows. One native Undo
restored the original graph, slot order, and all 288 rows. This unit
exercised the native menu for removal. The publicly guarded request
for a three-effect lane remains a separate verification unit.

The owner issued 299 requests in 104.823 seconds and released all
22 captures. Studio Pro exited normally after 157.620 seconds. The
scratch was not saved and its hash matched. All 18 managed files and
ten changed host profile files were restored.
