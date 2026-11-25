# Empty nested Splitter removal in Studio Pro 8.1.1

R428B exercised the public MCP tool `insert_splitter_remove_empty_nested` in Studio Pro 8.1.1 on macOS arm64. The clean candidate source was `d625d070ccda92b12f428ea2bb1863ba7e294bf0`. Playback and recording were stopped, Main was muted at zero, and the exact scratch rack’s Routing tab was open.

| Phase | Observation |
| --- | --- |
| Baseline | Eight nodes and ten ordered links. The inner Splitter had an original Mixtool on each branch. Below the first Mixtool, Splitter 3 had two empty lanes converging on its paired merger. |
| Refusal checks | The wrong document, wrong effect instance, wrong graph, swapped merger roles, and wrong predecessor were refused before dispatch. Fresh graph and typed parameter observations matched the baseline after each request. |
| One valid request | The tool returned `applied_unverified`. Independent captures showed six nodes and seven ordered links. Only Splitter 3 and its paired merger disappeared. The first Mixtool’s slot now led to the existing inner merger. The retained child order, effect instances, mixer, and both 72-row typed DSP snapshots matched. The Routing screen showed the two original Mixtools in parallel. |
| One Save | Independent file inspection matched the six-node graph and the two original insert states. A live capture after Save matched the edited state. |
| Fresh process | A request carrying the old document was refused before dispatch. Independent graph, mixer, saved insert state, both 72-row DSP snapshots, and the visible parallel routing matched the saved result. The file digest matched the once-saved file when the app closed. |

The two MCP owners handled 253 and 75 requests in 63.064 and 23.786 seconds and released 20 and 6 temporary capture tokens. Both owners and hosts exited normally. The original song remained byte-identical. All 18 managed runtime files were restored and rehashed, and nine app profile files were restored. The reopen session displayed a Save prompt on close. The operator declined it and verified the file digest again.

The initial R428 candidate refused the valid request because it treated the retained graph’s `IUnknown` and `INode` interface addresses as the primary object address. Fresh reads in that trial still showed all eight nodes, ten links, and 144 typed DSP rows, and the operator closed without saving. R428B verifies both pinned interface addresses and their tables before recovering the primary object. It rechecks node ownership and topology before opening the edit transaction.

This trial covers the two empty lanes of the supported third-Splitter shape. Studio Pro Undo for this public tool, operation with Routing closed, and other empty-lane topologies remain unverified.
