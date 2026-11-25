# Nested Splitter first-lane move after Save and reopen

R434 ran `insert_slot_move` on Studio Pro 8.1.1 Build 113103 for macOS arm64 on the exact clean candidate `92145d5fe4ee546044a0c936224aaee31cc94266` previously used for R421. The starting scratch song was a byte-identical independent copy of the previously verified eight-node graph. Main was muted at zero and transport was stopped.

- The operator confirmed Splitter 3’s two empty lanes in the Routing editor. An independent capture bound the current song, source Mixtool 2, insert graph, destination Splitter, and paired merger. One request carrying `branch: 0` returned `applied_unverified`.
- An independent post-request capture matched the R421 result. Only three ordered successor lists changed. The eight nodes, ten links, original slot and device IDs, and both 72-row typed parameter sets were preserved. Native child enumeration order changed. The Routing screen showed Mixtool 2 on the first lane below the third Splitter, with the original Mixtool above it.
- One normal Save updated the scratch file. An independent file read matched the eight nodes and all ten ordered links in the post-move capture.
- The same saved file opened in a fresh Studio Pro process. Studio Pro issued a fresh document, folder and graph binding. A request carrying the prior document was refused before dispatch. Independent captures on both sides of that request matched the saved routing, slot and device IDs, and all 144 typed rows. The Routing editor still showed Mixtool 2 on the first lane.

The two owners processed 91 requests in 45.061 seconds and 62 requests in 23.953 seconds and released six and four temporary capture tokens. The hosts ran for 75.979 and 59.708 seconds. Both hosts and owners exited normally. The operator declined the reopen session’s Save prompt. The saved file hash and original song hash matched their respective checkpoints. All 18 managed runtime files and nine app profile files were restored.
