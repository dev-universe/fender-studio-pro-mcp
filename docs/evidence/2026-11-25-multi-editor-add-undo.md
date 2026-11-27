# Multi Instrument editor add and Undo

R407 used Studio Pro 8.1.1 Build 113103 on macOS arm64. The owned
scratch began with one selected Music track, one Multi Instrument
and one Mai Tai. Main stayed muted at zero, transport stopped, and
the empty track remained unarmed and unmonitored. The saved source
identified three event nodes and the original Inst01 slot.

In the Multi Instrument Routing editor, the Add Instrument menu
returned one installed Mai Tai match. Choosing it once created a
second visible parallel branch. Two independent native observations
reported seven graph children and two graph-to-slot associations.
Inst02 and its Mai Tai Device had fresh identities. Its complete
283-row DSP provider was readable. The original Inst01 binding,
its 283 DSP rows, the parent's 150 DSP rows and the existing mixer
channels matched the baseline. A new child mixer channel routed to
the Multi Instrument.

The host Edit menu listed Undo Add Instrument “Mai Tai.” Selecting
that entry once removed the second child and channel. Two fresh
observations confirmed the original Inst01 and DSP providers, the
same three complete graph node records, and the original
graph-to-slot association. The controller enumerated the final two
records in swapped order. Ordered controller projection equality
therefore remains unverified for Undo, and this observation makes
no claim about the serialized event successor order.

MCP completed 124 requests in 167.581 seconds and the app ran for
237.633 seconds. The scratch closed without saving. Its bytes,
the 18 managed installation files and ten restored profile files
matched after cleanup.

MCP supplied the independent reads. The addition and Undo were
host editor actions. A native MCP graph edit and persistence of
the new event edges require separate verification.
