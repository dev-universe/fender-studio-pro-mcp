# Moving a head effect into a different occupied Splitter branch

R563 used candidate `e1afd43` in Studio Pro 8.1.1 build 113103 on
macOS arm64. The stopped, silent scratch held four bound Mixtool
instances across the two lanes of Splitter 3. The left lane had
Mixtool 2 followed by Mixtool 3. The right lane had Mixtool followed
by Mixtool 4.

One `insert_slot_move` request took Mixtool 2 from the left head and
inserted it between the two existing right-lane effects. Independent
captures confirmed the left lane now started with Mixtool 3. The right
lane ran Mixtool, Mixtool 2 and Mixtool 4 in that order. The same
10 nodes and 12 links remained, with all four native effect identities
and 288 typed DSP rows preserved.

An exact request using the superseded Mixtool-to-Mixtool-4 destination
link was rejected before dispatch, even with the current graph and
instance. Independent readback matched the moved state. The Routing tab
showed the new placement. One native Undo of the accepted move restored
the original ordered graph, enumeration, slots and DSP values.

The owner issued 296 requests in 64.899 seconds and released all 22
captures. Studio Pro exited normally after 104.547 seconds. The song
was not saved and its hash matched. The 18 managed installation files
and ten changed profile files were restored.
