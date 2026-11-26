# Reopen an exact effect-to-effect Splitter move

R542 used a new owned copy of the original stock Splitter song
in Studio Pro 8.1.1 build 113103 on macOS arm64. It created
a third exact Mixtool and applied the two bounded
`insert_slot_move` operations. The first moved Mixtool 2
after Mixtool. The second put Mixtool 3 between those same
effects. The entire ordered graph read Mixtool → Mixtool 3 →
Mixtool 2 on the moved branch before Splitter 3. The exact
routing tab showed the same order before Save.

One stock Save wrote only the owned test document. The separate
`song-read` archive decoder matched nine nodes, eleven ordered
links and all three slot and device identities. Each saved
display label was tied to its live native Device ID. Separate
warm reads matched all 216 DSP rows and the saved graph.

A second Studio Pro process opened that same file under a
verified write-deny rule. Its new native document, folder and
graph identifiers were distinct. The persisted slot and device
identifiers, ordered links and all 216 DSP rows were preserved.
The previous document request was refused by its document guard.
The previous graph request, sent with the current exact document
and slot, reached the separate native graph identifier guard and
was refused before applying a change. The first comparator
looked in the wrong response field. A fresh read-only owner
verified both archived refusals and unchanged state on that
same cold process without sending either stale request again.

Warm verification was 162 requests in 70.121 seconds. Cold
readers made 30 in 3.311 seconds and 32 in 3.497 seconds.
All 16 captures were released and the three owners exited with
status zero. The save and reopen hosts ran 107.022 and 71.773
seconds and exited with status zero. Both the source and the
new saved copy retained their verified bytes. The 18 managed
files and 10 changed profile files were restored. The second
host made no saves.
