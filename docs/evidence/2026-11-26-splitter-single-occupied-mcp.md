# MCP removal of a nested Splitter with one occupied lane

The `insert_splitter_remove_one_lane_nested` command targets the
verified eight-node Mixtool graph on Apple Silicon with Studio Pro
8.1.1 build 113103. The third Splitter's first lane must lead directly
to its merger. Its second lane must contain the exact `occupied_slot_node`
Mixtool, which also leads to that merger. The command binds the preceding
Mixtool in slot 0 and verifies the reciprocal links before requesting
the host's normal Remove action.

R501 supplied the same current document and UUIDs in two graph states.
With both lanes of Splitter 3 empty, the request returned
`not_applied` before removal. All eight nodes, ten links and 144 DSP
rows were identical on a fresh read. After one `insert_slot_move`
placed Mixtool 2 in the second lane, the removal request returned
`applied_unverified`. The editor and independent native reads
confirmed that only Splitter 3 and its merger retired. Six nodes and
seven links remained, with the original two Mixtools in series and
all 144 DSP rows unchanged.

One **Undo Splitter Remove** from Studio Pro's Edit menu restored the
pre-removal eight nodes, ten ordered links, both device and slot
identities, child enumeration order and 144 DSP rows. Mixer, sends and
timeline readbacks stayed unchanged. The trial closed without saving.

The live candidate was `98bbace`. R501 issued 181 requests in 120.214
seconds. The app ran for 154.638 seconds. Fourteen capture tokens were
released. Both processes exited normally, and 18 managed files and nine
profile files were restored and verified.
