# Stock removal of a nested Splitter with two effects on its first branch

R554 used Studio Pro 8.1.1 build 113103 on macOS arm64. The owned
scratch song began with three nested Splitters and two Mixtool effects.
Main was muted and at zero. A class-qualified third Mixtool and three
bound slot moves prepared Splitter 3 with two effects on branch 0 and
one on branch 1. Before removal the graph contained nine nodes and
eleven ordered links. The current `insert_splitter_remove` refused the
shape before dispatch and an independent read confirmed the same state.

The stock Splitter 3 menu removed the nested Splitter and its paired
merger once. The first branch's two effects preceded the second
branch's effect in the new serial chain. The rest of the graph
retained its links, with seven nodes and eight links remaining.
All three exact device identities and 216 typed DSP rows were retained.

The first comparison omitted the link from the first branch's tail
to the second branch's first slot. A second owner bound the same
open document, reconstructed the original and removed graphs from
two independently captured snapshots each, and verified the corrected
serial order against a fresh pair of reads. It dispatched no removal.
One stock Undo returned both branches, the complete node and link
order, and all 216 DSP rows to the pre-removal state.

The first owner issued 194 requests in 80.778 seconds. The follow-up
owner issued 53 in 23.632 seconds. They released all 20 captures.
Studio Pro exited normally after 198.693 seconds. The song was never
saved and its archive hash matched the original. All 18 managed files
and ten changed profile files were restored.
