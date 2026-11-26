# Move an existing insert into the second lane of a nested Splitter

R496 tested `insert_slot_move` on Studio Pro 8.1.1 build 113103,
macOS arm64, with MCP runtime `59ac9e0`. The owned scratch was a
byte-identical copy of the graph used in the
[first-lane trial](2026-11-26-splitter-nested-first-lane.md).
The two Mixtools started in separate lanes of Splitter 2. The first
Mixtool fed Splitter 3, which had two empty lanes. Main was muted at
zero and the transport stayed stopped.

One request bound the current document, Mixtool 2 slot and native
Device, graph, destination Splitter 3 and its merger. It requested
`branch: 1` and returned `applied_unverified`. The independent routing
capture and editor both confirmed Mixtool 2 in the second lane. The
first lane remained empty. The three changed ordered successor lists
were the following.

| Source | Initial successors | Observed successors |
| --- | --- | --- |
| Splitter 2 | Mixtool, Mixtool 2 | Mixtool, Splitter 2 merger |
| Splitter 3 | Splitter 3 merger, Splitter 3 merger | Splitter 3 merger, Mixtool 2 |
| Mixtool 2 | Splitter 2 merger | Splitter 3 merger |

The eight node identities, ten directed links, both slot and Device
IDs, and all 144 typed parameter rows were preserved. Native child
enumeration order changed. The mixer, sends and timeline stayed the
same. All six temporary capture tokens were released.

The owner made 85 requests in 68.406 seconds. Studio Pro exited normally
after 131.066 seconds. The scratch closed without saving. Its archive
and all 18 managed installation files and nine changed app profile
files were verified. This qualifies the empty second lane of the
tested nested Splitter. Undo and save/reopen for this exact move
remain separate tests.
