# Move a Mixtool into the first empty branch of a nested Splitter

R421 used Studio Pro 8.1.1 Build 113103 for macOS arm64. The owned
scratch was a byte-identical copy of the R251 saved fixture. The
offline reader and fresh host agreed with the archived routing.
The original eight nodes included three Splitters and their mergers.
Two Mixtools occupied different branches of Splitter 2. The first
Mixtool fed Splitter 3, which had two empty branches.

With stopped transport and muted, zero-level Main, the public
`insert_slot_move` request bound the current document, native graph,
Mixtool 2 slot and Device, and the destination Splitter 3 with its
paired merger. The request specified branch index 0. The host
returned `applied_unverified`. The following two independent
routing and ownership reads verified the change.

| Source | Original ordered successors | Observed ordered successors |
| --- | --- | --- |
| Splitter 2 | Mixtool, Mixtool 2 | Mixtool, Splitter 2 merger |
| Splitter 3 | Splitter 3 merger, Splitter 3 merger | Mixtool 2, Splitter 3 merger |
| Mixtool 2 | Splitter 2 merger | Splitter 3 merger |

All eight node records and ten directed successor links remained
present. The three lists above were the only edge changes. Folder,
graph, node, slot, and Device identities matched. The opaque native
child enumeration order changed. The UUID-keyed descriptors, both
72-row DSP providers, mixer and timeline matched. Routing showed
the original Mixtool upstream and Mixtool 2 in Splitter 3's first
lane, with the other lane empty.

The owner released all six temporary captures and exited normally.
It made 85 requests in 77.619 seconds. The app closed without saving
after 128.402 seconds. The source and scratch hashes matched and
18 managed files and nine application profile files were verified
after restoration. This trial covers one existing nested branch
move. Undo, reopening the moved graph and physical audio remain
separate verification.
