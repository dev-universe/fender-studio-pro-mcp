# Move a third Mixtool onto an exact effect-to-effect link

R541 ran Fender Studio Pro 8.1.1 build 113103 on macOS arm64.
The owned stopped scratch had Main and the exact Audio track
muted, no timeline events and three two-way nested Splitters.
Initial readback found eight nodes, ten ordered links and two
Mixtools with 144 DSP rows. Class-qualified insertion added one
Mixtool, one node and one link, preserving the older graph after
contracting the new serial node.

With the exact Routing tab in view, the public
`insert_slot_move` first put the second instance between the
first Mixtool and Splitter 3. The next call identified the third
instance and the precise existing first-to-second effect edge.
The live owned graph then read first effect → third effect →
second effect, followed by the same Splitter 3. The old parent
lost the moved node and the projected splice matched the
complete ordered topology. The graph and folder identities,
all nine nodes and all 216 typed DSP rows stayed associated
with their original native devices. The third device became the
second visible effect in the rack.

The stock Edit menu exposed Undo for that exact moved node. One
Undo restored the step immediately before the second move,
including native child enumeration, all eleven ordered links,
three slot and device identities and all 216 DSP rows. Each of
six phases was independently captured twice. The command's
initial receipt was applied unverified, and independent
ownership and serialized graph reads established the result.

The test used 162 requests in 109.879 seconds. Studio Pro ran
145.459 seconds. The owner and app exited with status zero;
all twelve captures were released. The scratch was closed
without Save, and the source, scratch, 18 managed files and
10 changed profile files matched their original bytes. This
trial covers a move onto an existing effect-to-effect link.
Saved reopen and arbitrary new connection creation remain open.
