# One native Mai Tai addition and Undo in Multi Instrument

A private diagnostic ran in Studio Pro 8.1.1 Build 113103 on macOS arm64
with one Mai Tai in an independently owned Multi Instrument scratch
song. Transport was stopped and Main was muted at zero. Two independent
observations confirmed the exact parent, Instruments folder, Graph, and
Inst01 slot and Device. The Routing editor showed one branch and Edit
had no prior Undo action.

One fixed native invocation requested a stock Mai Tai. The returned
object and `applied_unverified` outcome were recorded before further
checks. Two independent reads found seven graph nodes and two
parallel attachments, including an exact fresh Inst02 slot and Device.
The new child exposed 283 typed records. The original slot, Device and
its 283 records, the parent 150 records, the track and old mixer rows
matched their baseline. The only new mixer row routed to the Multi
Instrument. The Routing editor visibly showed both Mai Tai layers.

The host's top Undo entry named that Mai Tai addition. Applying the
entry once restored the single visible branch and the original
three-node identity records, slot and Device binding, all 433
original typed records, the mixer and track. This comparison matched
the node set by identity. The controller's enumeration order can
differ independently of those bindings.

The run took 158 MCP requests in 124.970 seconds. The app exited normally
after 213.611 seconds. The source and scratch copy retained their
original digests, both processes exited normally, and 18 managed
installation files and 10 app profile files were restored and verified.

The diagnostic establishes one model and one starting topology with the
Routing editor open. Public tool admission, the editor-closed path,
saving that native edit, and physical audio are separate checks.
