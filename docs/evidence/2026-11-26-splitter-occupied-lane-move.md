# Moving into an occupied nested Splitter lane

R522 used Studio Pro 8.1.1 build 113103 on macOS arm64. An owned,
Save-protected copy of the verified three-Splitter song contained two
same-model Mixtools. The app was stopped and its target and Main
outputs were muted. The stock Routing tab was open for the bound
insert folder.

1. An exact `insert_slot_move` placed the second Mixtool into the first
   empty lane of Splitter 3. Two reads confirmed the scoped staged
   graph and both DSP providers.
2. The same lane then rejected an empty-branch request with
   `not_applied` and the native reason that the branch was no longer
   empty. Independent reads matched the complete staged state.
3. The first Mixtool moved after the second Mixtool by addressing the
   exact current effect-to-merger edge with no `branch` field.
   The `applied_unverified` receipt preceded independent readback.

The first owner stopped at its old slot-order guard after the accepted
move. The stock editor and a fresh MCP owner in the same app and song
both showed the two exact instances swapped their numeric insert indexes.
The second owner issued no move. It compared complete provider rows by
native Device identity. All 144 DSP rows and the two native slot
bindings matched, and three ordered successor lists changed as predicted.
The graph retained eight nodes and ten links.

One stock Edit menu Undo restored the complete staged graph, its child
enumeration order and both numeric indexes. The two DSP providers again
matched all 144 rows. Both owners and the host exited normally. The two
owner phases took 109 requests in 50.299 seconds and 53 requests in
48.500 seconds. Twelve captures were released. The owned host ran
197.070 seconds. There was no Save, and the scratch bytes were unchanged.
The normal installation's 18 managed files and nine profile files were
restored and verified.

This validates moving onto the observed occupied edge and using one host
Undo. It leaves other graph shapes and persistence of this edit open.
