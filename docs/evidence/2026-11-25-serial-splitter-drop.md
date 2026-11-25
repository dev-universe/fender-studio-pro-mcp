# Splitter drop between two serial Mixtools

R408 opened an owned copy of the saved R257 fixture on Studio Pro
8.1.1 Build 113103, macOS arm64. The selected track was muted, Main
was muted at zero, and the transport was stopped. Two independent
native reads showed six nodes, seven ordered links, and an exact
Mixtool-to-Mixtool link in the same inner branch. The Routing editor
showed both devices and the Splitter drag control.

One drag from that control ended on the visible link between the
two Mixtools. The editor showed the same layout. Two more native
reads returned the original ownership records and all seven ordered
links, and both Mixtools' 144 total DSP rows remained exact. An
insertion and its Undo were not observed. Capture fingerprints
already differed between consecutive reads at baseline, so those
fingerprints cannot identify a graph change in this fixture.

The run stopped at the independent post-drop check. All six capture
tokens were released. MCP completed 89 requests in 157.743 seconds
and the app ran for 305.765 seconds. The scratch closed without
saving. Its source bytes, 18 managed files and nine restored profile
files matched after cleanup.

This observation covers the single tested gesture on this exact
serial edge. The previously verified slot-to-merger MCP insert has
its own evidence. Other drop affordances and the slot-to-slot
insertion callback remain separate cases.
